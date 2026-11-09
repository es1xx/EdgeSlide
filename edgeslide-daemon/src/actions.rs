use crate::audio::Audio;
use anyhow::{bail, Context, Result};
use edgeslide_core::{Action, Adjustment, Config};
use glib::{prelude::*, variant::ObjectPath};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

struct Batch {
    values: [f64; 2],
    updated: Instant,
}
type Pending = Arc<(Mutex<Batch>, Condvar)>;
pub struct Actions {
    pending: Pending,
}
impl Actions {
    pub fn start(config: Config) -> Self {
        let pending = Arc::new((
            Mutex::new(Batch {
                values: [0.0; 2],
                updated: Instant::now(),
            }),
            Condvar::new(),
        ));
        let queue = pending.clone();
        std::thread::spawn(move || {
            let mut backend = Backend::default();
            let mut last_error = None;
            loop {
                let (lock, wake) = &*queue;
                let mut values = lock.lock().unwrap();
                while values.values == [0.0; 2] {
                    values = wake.wait(values).unwrap();
                }
                let batch = values.values;
                let stale = values.updated.elapsed() > Duration::from_millis(200);
                values.values = [0.0; 2];
                drop(values);
                if stale {
                    continue;
                }
                for (index, delta) in batch.into_iter().enumerate() {
                    if delta == 0.0 {
                        continue;
                    }
                    if let Err(error) = backend.adjust(index, delta, &config) {
                        if last_error
                            .is_none_or(|time: Instant| time.elapsed() > Duration::from_secs(5))
                        {
                            tracing::warn!(%error, "Action failed");
                            last_error = Some(Instant::now());
                        }
                    }
                }
                // A one-shot coalescing delay exists only after actual gesture work.
                std::thread::sleep(Duration::from_millis(25));
            }
        });
        Self { pending }
    }
    pub fn submit(&self, adjustment: Adjustment) {
        let index = match adjustment.action {
            Action::Volume => 0,
            Action::Brightness => 1,
            Action::Disabled => return,
        };
        let (lock, wake) = &*self.pending;
        let mut pending = lock.lock().unwrap();
        pending.values[index] = (pending.values[index] + adjustment.delta).clamp(-0.15, 0.15);
        pending.updated = Instant::now();
        wake.notify_one();
    }
}
#[derive(Default)]
struct Backend {
    audio: Option<Audio>,
    bus: Option<gio::DBusConnection>,
    backlight: Option<(PathBuf, u32)>,
    indicator_socket: Option<std::os::unix::net::UnixDatagram>,
    last_indicator_spawn: Option<Instant>,
}
fn call(
    bus: &gio::DBusConnection,
    destination: &str,
    path: &str,
    interface: &str,
    method: &str,
    args: glib::Variant,
) -> Result<glib::Variant> {
    Ok(bus.call_sync(
        Some(destination),
        path,
        interface,
        method,
        Some(&args),
        None,
        gio::DBusCallFlags::NONE,
        1000,
        gio::Cancellable::NONE,
    )?)
}
impl Backend {
    fn session(&mut self) -> Result<String> {
        if self.bus.as_ref().is_none_or(|bus| bus.is_closed()) {
            self.bus = Some(gio::bus_get_sync(
                gio::BusType::System,
                gio::Cancellable::NONE,
            )?);
        }
        let bus = self.bus.as_ref().unwrap();
        let reply = call(
            bus,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "ListSessions",
            ().to_variant(),
        )?;
        type Sessions = (Vec<(String, u32, String, String, ObjectPath)>,);
        let (sessions,) = reply
            .get::<Sessions>()
            .context("Invalid logind session list")?;
        for (_, uid, _, seat, path) in sessions {
            // SAFETY: getuid has no preconditions or memory arguments.
            if uid != unsafe { libc::getuid() } || seat.is_empty() {
                continue;
            }
            let props = call(
                bus,
                "org.freedesktop.login1",
                path.as_str(),
                "org.freedesktop.DBus.Properties",
                "GetAll",
                ("org.freedesktop.login1.Session",).to_variant(),
            )?;
            let (props,) = props
                .get::<(HashMap<String, glib::Variant>,)>()
                .context("Invalid session properties")?;
            let active = props
                .get("Active")
                .and_then(|v| v.get::<bool>())
                .unwrap_or(false);
            let locked = props
                .get("LockedHint")
                .and_then(|v| v.get::<bool>())
                .unwrap_or(true);
            if active && !locked {
                return Ok(path.to_string());
            }
        }
        bail!("No active, unlocked local session for this user")
    }
    fn adjust(&mut self, index: usize, delta: f64, config: &Config) -> Result<()> {
        let session = self.session()?;
        let value = if index == 0 {
            if self.audio.is_none() {
                self.audio = Some(Audio::connect()?);
            }
            match self.audio.as_mut().unwrap().adjust(
                delta * config.volume_speed,
                config.volume_min,
                config.volume_max,
            ) {
                Ok(value) => value,
                Err(error) => {
                    self.audio = None;
                    return Err(error);
                }
            }
        } else {
            match self.brightness(
                &session,
                delta * config.brightness_speed,
                config.brightness_min,
            ) {
                Ok(value) => value,
                Err(error) => {
                    self.backlight = None;
                    return Err(error);
                }
            }
        };
        if config.indicator {
            self.notify(index, value);
        }
        Ok(())
    }
    fn brightness(&mut self, session: &str, delta: f64, minimum: f64) -> Result<f64> {
        if self.backlight.is_none() {
            let mut paths: Vec<_> = std::fs::read_dir("/sys/class/backlight")?
                .filter_map(Result::ok)
                .map(|v| v.path())
                .collect();
            paths.sort();
            let path = paths.into_iter().next().context("No backlight device")?;
            let max = std::fs::read_to_string(path.join("max_brightness"))?
                .trim()
                .parse::<u32>()?;
            if max == 0 {
                bail!("Invalid backlight maximum");
            }
            self.backlight = Some((path, max));
        }
        let (path, max) = self.backlight.as_ref().unwrap();
        let current = std::fs::read_to_string(path.join("brightness"))?
            .trim()
            .parse::<u32>()?;
        let value = (f64::from(current) / f64::from(*max) + delta).clamp(minimum, 1.0);
        let raw = ((value * f64::from(*max)).round() as u32)
            .max((minimum * f64::from(*max)).ceil() as u32)
            .min(*max);
        let device = path.file_name().unwrap().to_string_lossy();
        call(
            self.bus.as_ref().unwrap(),
            "org.freedesktop.login1",
            session,
            "org.freedesktop.login1.Session",
            "SetBrightness",
            ("backlight", device.as_ref(), raw).to_variant(),
        )?;
        Ok(value)
    }
    fn notify(&mut self, index: usize, value: f64) {
        let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
            return;
        };
        let path = PathBuf::from(runtime).join("edgeslide-indicator.sock");
        if self.indicator_socket.is_none() {
            self.indicator_socket = std::os::unix::net::UnixDatagram::unbound().ok();
            if let Some(socket) = self.indicator_socket.as_ref() {
                let _ = socket.set_nonblocking(true);
            }
        }
        let message = format!("{index}:{value:.6}");
        if self
            .indicator_socket
            .as_ref()
            .is_some_and(|socket| socket.send_to(message.as_bytes(), &path).is_ok())
        {
            return;
        }
        if self
            .last_indicator_spawn
            .is_some_and(|time| time.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        self.last_indicator_spawn = Some(Instant::now());
        let Ok(binary) = std::env::current_exe() else {
            return;
        };
        match std::process::Command::new(binary.with_file_name("edgeslide-indicator"))
            .arg(message)
            .stdin(std::process::Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(error) => tracing::warn!(%error,"Cannot launch optional indicator"),
        }
    }
}

/// Explicit diagnostic: reapply existing levels, with no intended level change.
pub fn probe() -> Result<()> {
    let mut backend = Backend::default();
    let session = backend.session()?;
    println!("Active unlocked session: {session}");
    let mut audio = Audio::connect()?;
    let volume = audio.adjust(0.0, 0.0, 65535.0)?;
    println!(
        "Native audio same-value write acknowledged: {:.3}%",
        volume * 100.0
    );
    let brightness = backend.brightness(&session, 0.0, 0.0)?;
    println!(
        "Native logind same-value brightness write acknowledged: {:.3}%",
        brightness * 100.0
    );
    Ok(())
}
