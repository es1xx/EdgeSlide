//! Short-lived layer-shell OSD. No keyboard/pointer input, animation timer, or GTK loop.
use anyhow::{bail, Context, Result};
use edgeslide_core::{Config, IndicatorAccent, IndicatorPosition};
use gtk::cairo;
use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    os::{
        fd::{AsFd, AsRawFd, FromRawFd},
        unix::{fs::OpenOptionsExt, net::UnixDatagram},
    },
    path::PathBuf,
    time::{Duration, Instant},
};
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_buffer, wl_compositor, wl_region, wl_registry, wl_shm, wl_shm_pool, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, Layer},
    zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity},
};
const WIDTH: i32 = 300;
const HEIGHT: i32 = 86;
const SCALE: i32 = 2;
struct Buffer {
    proxy: wl_buffer::WlBuffer,
    file: File,
    busy: bool,
}
struct State {
    surface: wl_surface::WlSurface,
    buffers: Vec<Buffer>,
    configured: bool,
    closed: bool,
    dirty: bool,
    index: usize,
    value: f64,
    accent: IndicatorAccent,
}
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                surface.ack_configure(serial);
                state.configured = true;
                state.dirty = true;
            }
            zwlr_layer_surface_v1::Event::Closed => state.closed = true,
            _ => {}
        }
    }
}
impl Dispatch<wl_buffer::WlBuffer, usize> for State {
    fn event(
        state: &mut Self,
        _: &wl_buffer::WlBuffer,
        event: wl_buffer::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            state.buffers[*index].busy = false;
        }
    }
}
delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_region::WlRegion);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);

fn buffer(shm: &wl_shm::WlShm, qh: &QueueHandle<State>, index: usize) -> Result<Buffer> {
    // SAFETY: the name is NUL-terminated, flags are valid; ownership is transferred once.
    let fd = unsafe { libc::memfd_create(c"edgeslide-osd".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let size = WIDTH * HEIGHT * SCALE * SCALE * 4;
    file.set_len(size as u64)?;
    let pool = shm.create_pool(file.as_fd(), size, qh, ());
    let proxy = pool.create_buffer(
        0,
        WIDTH * SCALE,
        HEIGHT * SCALE,
        WIDTH * SCALE * 4,
        wl_shm::Format::Argb8888,
        qh,
        index,
    );
    pool.destroy();
    Ok(Buffer {
        proxy,
        file,
        busy: false,
    })
}
fn rounded(context: &cairo::Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let pi = std::f64::consts::PI;
    context.new_sub_path();
    context.arc(x + width - radius, y + radius, radius, -pi / 2.0, 0.0);
    context.arc(
        x + width - radius,
        y + height - radius,
        radius,
        0.0,
        pi / 2.0,
    );
    context.arc(x + radius, y + height - radius, radius, pi / 2.0, pi);
    context.arc(x + radius, y + radius, radius, pi, 1.5 * pi);
    context.close_path();
}
impl State {
    fn paint(&mut self) -> Result<()> {
        if !self.configured || !self.dirty {
            return Ok(());
        }
        let Some(buffer) = self.buffers.iter_mut().find(|buffer| !buffer.busy) else {
            return Ok(());
        };
        let mut image =
            cairo::ImageSurface::create(cairo::Format::ARgb32, WIDTH * SCALE, HEIGHT * SCALE)?;
        let context = cairo::Context::new(&image)?;
        context.scale(f64::from(SCALE), f64::from(SCALE));
        rounded(&context, 1.0, 1.0, 298.0, 84.0, 18.0);
        context.set_source_rgba(0.10, 0.12, 0.17, 0.96);
        context.fill()?;
        context.set_source_rgb(0.92, 0.94, 0.98);
        context.select_font_face(
            "sans-serif",
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        context.set_font_size(15.0);
        context.move_to(22.0, 32.0);
        context.show_text(if self.index == 0 {
            "Volume"
        } else {
            "Brightness"
        })?;
        let value = format!("{:.0}%", self.value * 100.0);
        let extent = context.text_extents(&value)?;
        context.move_to(278.0 - extent.width(), 32.0);
        context.show_text(&value)?;
        rounded(&context, 22.0, 50.0, 256.0, 10.0, 5.0);
        context.set_source_rgb(0.23, 0.26, 0.32);
        context.fill()?;
        let (red, green, blue) = match self.accent {
            IndicatorAccent::Blue => (0.35, 0.72, 1.0),
            IndicatorAccent::Amber => (1.0, 0.7, 0.3),
            IndicatorAccent::Green => (0.4, 0.85, 0.62),
        };
        let width = 256.0 * self.value.clamp(0.0, 1.0);
        if width > 0.0 {
            rounded(&context, 22.0, 50.0, width, 10.0, 5.0_f64.min(width / 2.0));
            context.set_source_rgb(red, green, blue);
            context.fill()?;
        }
        drop(context);
        image.flush();
        buffer.file.seek(SeekFrom::Start(0))?;
        buffer.file.write_all(&image.data()?)?;
        self.surface.attach(Some(&buffer.proxy), 0, 0);
        self.surface
            .damage_buffer(0, 0, WIDTH * SCALE, HEIGHT * SCALE);
        self.surface.commit();
        buffer.busy = true;
        self.dirty = false;
        Ok(())
    }
}
struct SocketPath(PathBuf);
impl Drop for SocketPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn parse(text: &str) -> Result<(usize, f64)> {
    let (index, value) = text.split_once(':').context("Expected action:value")?;
    let index = index.parse::<usize>()?;
    let value = value.parse::<f64>()?;
    if index > 1 || !value.is_finite() || !(0.0..=1.5).contains(&value) {
        bail!("Invalid indicator value");
    }
    Ok((index, value))
}
fn main() -> Result<()> {
    let (index, value) = parse(
        &std::env::args()
            .nth(1)
            .context("Usage: edgeslide-indicator ACTION:VALUE")?,
    )?;
    let config = Config::load()?;
    let runtime =
        PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").context("No runtime directory")?);
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(runtime.join("edgeslide-indicator.lock"))?;
    // SAFETY: flock takes a live file descriptor; a failed nonblocking lock means another OSD owns it.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Ok(());
    }
    let path = SocketPath(runtime.join("edgeslide-indicator.sock"));
    let _ = std::fs::remove_file(&path.0);
    let socket = UnixDatagram::bind(&path.0)?;
    socket.set_nonblocking(true)?;
    let connection = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)?;
    let qh = queue.handle();
    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=4, ())?;
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ())?;
    let layer: zwlr_layer_shell_v1::ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ())?;
    let surface = compositor.create_surface(&qh, ());
    surface.set_buffer_scale(SCALE);
    let region = compositor.create_region(&qh, ());
    surface.set_input_region(Some(&region));
    region.destroy();
    let overlay =
        layer.get_layer_surface(&surface, None, Layer::Overlay, "edgeslide".into(), &qh, ());
    overlay.set_keyboard_interactivity(KeyboardInteractivity::None);
    overlay.set_exclusive_zone(-1);
    overlay.set_size(WIDTH as u32, HEIGHT as u32);
    overlay.set_margin(32, 32, 32, 32);
    overlay.set_anchor(match config.indicator_position {
        IndicatorPosition::Top => Anchor::Top,
        IndicatorPosition::Bottom => Anchor::Bottom,
        IndicatorPosition::Left => Anchor::Left,
        IndicatorPosition::Right => Anchor::Right,
    });
    surface.commit();
    let mut state = State {
        surface,
        buffers: vec![buffer(&shm, &qh, 0)?, buffer(&shm, &qh, 1)?],
        configured: false,
        closed: false,
        dirty: true,
        index,
        value,
        accent: config.indicator_accent,
    };
    let mut deadline = Instant::now() + Duration::from_millis(900);
    while !state.closed && Instant::now() < deadline {
        queue.dispatch_pending(&mut state)?;
        state.paint()?;
        connection.flush()?;
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let mut descriptors = [
            libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: socket.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        let timeout = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .max(1) as i32;
        // SAFETY: both descriptors and their owners remain alive throughout poll.
        let result = unsafe { libc::poll(descriptors.as_mut_ptr(), 2, timeout) };
        if result < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if descriptors[0].revents & libc::POLLIN != 0 {
            guard.read()?;
        } else {
            drop(guard);
        }
        if descriptors[0].revents & (libc::POLLERR | libc::POLLHUP) != 0 {
            break;
        }
        if descriptors[1].revents & libc::POLLIN != 0 {
            let mut bytes = [0_u8; 64];
            while let Ok(size) = socket.recv(&mut bytes) {
                if let Ok(text) = std::str::from_utf8(&bytes[..size]) {
                    if let Ok((index, value)) = parse(text) {
                        state.index = index;
                        state.value = value;
                        state.dirty = true;
                        deadline = Instant::now() + Duration::from_millis(900);
                    }
                }
            }
        }
    }
    overlay.destroy();
    state.surface.destroy();
    connection.flush()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_invalid_messages() {
        for input in ["garbage", "0:NaN", "1:-1", "2:0.5", "0:99"] {
            assert!(parse(input).is_err());
        }
        assert_eq!(parse("1:0.5").unwrap(), (1, 0.5));
    }
}
