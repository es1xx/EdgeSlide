use anyhow::{bail, Context as _, Result};
use libpulse_binding as pulse;
use pulse::{
    context::{Context, State},
    mainloop::standard::Mainloop,
    operation::Operation,
    time::MicroSeconds,
    volume::{ChannelVolumes, Volume},
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

pub struct Audio {
    context: Context,
    mainloop: Mainloop,
}
impl Audio {
    pub fn connect() -> Result<Self> {
        let mainloop = Mainloop::new().context("Cannot allocate PulseAudio mainloop")?;
        let mut context =
            Context::new(&mainloop, "EdgeSlide").context("Cannot allocate audio context")?;
        context.connect(None, pulse::context::FlagSet::NOAUTOSPAWN, None)?;
        let mut audio = Self { context, mainloop };
        let deadline = Instant::now() + Duration::from_secs(2);
        while audio.context.get_state() != State::Ready {
            audio.step(deadline)?;
        }
        Ok(audio)
    }
    fn step(&mut self, deadline: Instant) -> Result<()> {
        if Instant::now() >= deadline {
            bail!("Audio server timed out");
        }
        if matches!(self.context.get_state(), State::Failed | State::Terminated) {
            bail!("Audio disconnected");
        }
        // libpulse blocks on its descriptors, with a deadline only while an operation is pending.
        self.mainloop.prepare(Some(MicroSeconds(200_000)))?;
        self.mainloop.poll()?;
        self.mainloop.dispatch()?;
        Ok(())
    }
    fn wait<T: ?Sized>(&mut self, mut operation: Operation<T>) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(2);
        while operation.get_state() == pulse::operation::State::Running {
            if let Err(error) = self.step(deadline) {
                operation.cancel();
                return Err(error);
            }
        }
        if operation.get_state() != pulse::operation::State::Done {
            bail!("Audio operation cancelled");
        }
        Ok(())
    }
    pub fn adjust(&mut self, delta: f64, min: f64, max: f64) -> Result<f64> {
        let name = Rc::new(RefCell::new(None));
        let output = name.clone();
        let operation = self.context.introspect().get_server_info(move |info| {
            *output.borrow_mut() = info.default_sink_name.as_ref().map(|v| v.to_string());
        });
        self.wait(operation)?;
        let name = name.borrow().clone().context("No default audio sink")?;
        let sink = Rc::new(RefCell::new(None));
        let output = sink.clone();
        let operation = self
            .context
            .introspect()
            .get_sink_info_by_name(&name, move |result| {
                if let pulse::callbacks::ListResult::Item(info) = result {
                    *output.borrow_mut() = Some((info.index, info.volume));
                }
            });
        self.wait(operation)?;
        let (index, mut volumes): (u32, ChannelVolumes) =
            sink.borrow_mut().take().context("Audio sink disappeared")?;
        let value =
            (f64::from(volumes.max().0) / f64::from(Volume::NORMAL.0) + delta).clamp(min, max);
        // Scaling preserves the existing channel balance. Mute state is never changed.
        volumes.scale(Volume((value * f64::from(Volume::NORMAL.0)).round() as u32));
        let success = Rc::new(RefCell::new(false));
        let output = success.clone();
        let operation = self.context.introspect().set_sink_volume_by_index(
            index,
            &volumes,
            Some(Box::new(move |ok| *output.borrow_mut() = ok)),
        );
        self.wait(operation)?;
        if !*success.borrow() {
            bail!("Audio server rejected volume change");
        }
        Ok(value)
    }
}
impl Drop for Audio {
    fn drop(&mut self) {
        self.context.disconnect();
    }
}
