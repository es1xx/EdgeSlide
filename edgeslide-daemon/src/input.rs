use crate::actions::Actions;
use anyhow::{bail, Context, Result};
use edgeslide_core::{Config, Decoder, Recognizer};
use evdev::{raw_stream::RawDevice, AbsoluteAxisCode as Abs, KeyCode};
use std::{
    os::fd::AsRawFd,
    time::{Duration, SystemTime},
};

pub fn devices() -> Result<Vec<(String, std::path::PathBuf)>> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("input")?;
    enumerator.match_property("ID_INPUT_TOUCHPAD", "1")?;
    let mut result = Vec::new();
    for device in enumerator.scan_devices()? {
        let Some(path) = device.devnode() else {
            continue;
        };
        if !path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .starts_with("event")
        {
            continue;
        }
        let name = device
            .parent()
            .and_then(|parent| {
                parent
                    .attribute_value("name")
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        result.push((name, path.to_path_buf()));
    }
    result.sort();
    Ok(result)
}
pub struct Input {
    pub device: RawDevice,
    range: [(i32, i32); 2],
    recognizer: Recognizer,
    last_frame: Option<SystemTime>,
    decoder: Decoder,
    was_touching: bool,
}
impl Input {
    pub fn open(config: &Config) -> Result<Option<Self>> {
        if !config.enabled {
            return Ok(None);
        }
        for (name, path) in devices()? {
            if !config.touchpad.is_empty() && config.touchpad != name {
                continue;
            }
            let file = std::fs::File::open(&path).with_context(|| {
                format!(
                    "Cannot read {} ({name}); see docs/PERMISSIONS.md",
                    path.display()
                )
            })?;
            let device = RawDevice::from_fd(file.into())?;
            let axes = device.get_abs_state()?;
            let range = [Abs::ABS_X, Abs::ABS_Y].map(|code| {
                let a = axes[code.0 as usize];
                (a.minimum, a.maximum)
            });
            if range.iter().any(|(min, max)| max <= min)
                || !device.supported_keys().is_some_and(|keys| {
                    keys.contains(KeyCode::BTN_TOOL_FINGER) && keys.contains(KeyCode::BTN_TOUCH)
                })
            {
                bail!("Touchpad {name} lacks required absolute axes or finger state");
            }
            // No EVIOCGRAB: this descriptor observes only; compositor input is untouched.
            tracing::info!(%name, path=%path.display(), "Observing touchpad nonexclusively");
            let mut recognizer = Recognizer::default();
            if device.get_key_state()?.contains(KeyCode::BTN_TOUCH) {
                recognizer.cancel();
            }
            let decoder = snapshot(&device, range)?;
            return Ok(Some(Self {
                device,
                range,
                recognizer,
                last_frame: None,
                decoder,
                was_touching: false,
            }));
        }
        Ok(None)
    }
    pub fn fd(&self) -> i32 {
        self.device.as_raw_fd()
    }
    pub fn read(&mut self, config: &Config, actions: &Actions) -> Result<()> {
        for event in self.device.fetch_events()? {
            let Some(frame) = self
                .decoder
                .push(event.event_type().0, event.code(), event.value())
            else {
                continue;
            };
            let now = event.timestamp();
            let stale = SystemTime::now()
                .duration_since(now)
                .unwrap_or(Duration::MAX)
                > Duration::from_millis(150);
            let gap = self.was_touching
                && self.last_frame.is_some_and(|last| {
                    now.duration_since(last).unwrap_or(Duration::MAX) > Duration::from_millis(750)
                });
            if stale || gap {
                self.recognizer.cancel();
            }
            self.last_frame = Some(now);
            self.was_touching = frame.fingers > 0;
            if let Some(adjustment) = self.recognizer.update(frame, config) {
                if !stale {
                    actions.submit(adjustment);
                }
            }
        }
        if self.decoder.resync_ready {
            self.recognizer.cancel();
            self.decoder = snapshot(&self.device, self.range)?;
            if let Some(frame) = self.decoder.push(0, 0, 0) {
                self.recognizer.update(frame, config);
                self.was_touching = frame.fingers > 0;
            }
        }
        Ok(())
    }
}
fn snapshot(device: &RawDevice, range: [(i32, i32); 2]) -> Result<Decoder> {
    let mut decoder = Decoder::new(range);
    let axes = device.get_abs_state()?;
    for code in [Abs::ABS_X, Abs::ABS_Y, Abs::ABS_MT_TOOL_TYPE] {
        decoder.push(3, code.0, axes[code.0 as usize].value);
    }
    for key in device.get_key_state()?.iter() {
        decoder.push(1, key.0, 1);
    }
    Ok(decoder)
}
