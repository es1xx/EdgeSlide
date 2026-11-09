mod actions;
mod audio;
mod input;
use anyhow::Result;
use edgeslide_core::Config;
use std::os::fd::AsRawFd;
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let config = Config::load()?;
    match std::env::args().nth(1).as_deref() {
        Some("--probe-backends") => return actions::probe(),
        Some("--list-devices") => {
            for (name, path) in input::devices()? {
                println!("{}\t{name}", path.display());
            }
            return Ok(());
        }
        Some("--check-config") => {
            println!("Configuration valid");
            return Ok(());
        }
        Some("--print-defaults") => {
            println!("{}", include_str!("../../data/config.toml"));
            return Ok(());
        }
        Some(_) => anyhow::bail!(
            "Usage: edgeslide-daemon [--list-devices|--check-config|--print-defaults|--probe-backends]"
        ),
        None => {}
    }
    run(config)
}
fn open(config: &Config) -> Option<input::Input> {
    match input::Input::open(config) {
        Ok(input) => {
            if input.is_none() {
                tracing::info!("No selected touchpad; waiting for device events");
            }
            input
        }
        Err(error) => {
            tracing::warn!(%error, "Touchpad unavailable; waiting for a udev event or service restart");
            None
        }
    }
}
fn run(config: Config) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| anyhow::anyhow!("XDG_RUNTIME_DIR is required"))?;
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(std::path::PathBuf::from(runtime).join("edgeslide.lock"))?;
    // SAFETY: flock operates on a live descriptor and does not access Rust memory.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        anyhow::bail!("Another EdgeSlide daemon is already running");
    }

    let monitor = udev::MonitorBuilder::new()?
        .match_subsystem("input")?
        .listen()?;
    let actions = actions::Actions::start(config.clone());
    let mut input = open(&config);
    loop {
        let mut descriptors = [
            libc::pollfd {
                fd: monitor.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: input.as_ref().map_or(-1, input::Input::fd),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: descriptors is initialized and lives through poll; -1 means infinite timeout.
        let result = unsafe { libc::poll(descriptors.as_mut_ptr(), descriptors.len() as _, -1) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if descriptors[0].revents != 0 {
            for _ in monitor.iter() {}
            input = open(&config);
            continue;
        }
        if descriptors[1].revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            input = None;
            continue;
        }
        if descriptors[1].revents & libc::POLLIN != 0 {
            if let Some(device) = input.as_mut() {
                if let Err(error) = device.read(&config, &actions) {
                    tracing::warn!(%error,"Input disconnected");
                    input = None;
                }
            }
        }
    }
}
