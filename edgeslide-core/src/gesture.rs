use crate::{Action, Config};
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub fingers: usize,
    pub button: bool,
    pub palm: bool,
}
#[derive(Debug, Clone, Copy)]
pub struct Adjustment {
    pub action: Action,
    pub delta: f64,
}
#[derive(Default)]
enum State {
    #[default]
    Idle,
    Rejected,
    Tracking {
        right: bool,
        start_x: f64,
        start_y: f64,
        last_y: f64,
        active: bool,
    },
}
#[derive(Default)]
pub struct Recognizer {
    state: State,
}
impl Recognizer {
    /// Ignore the current contact until all fingers lift after discontinuities.
    pub fn cancel(&mut self) {
        self.state = State::Rejected;
    }
    pub fn update(&mut self, frame: Frame, config: &Config) -> Option<Adjustment> {
        if frame.fingers == 0 {
            self.state = State::Idle;
            return None;
        }
        if !config.enabled
            || frame.fingers != 1
            || frame.button
            || frame.palm
            || !frame.x.is_finite()
            || !frame.y.is_finite()
            || !(0.0..=1.0).contains(&frame.x)
            || !(0.0..=1.0).contains(&frame.y)
        {
            self.cancel();
            return None;
        }
        if matches!(self.state, State::Idle) {
            let right = frame.x >= 1.0 - config.right.width;
            let edge = if right { &config.right } else { &config.left };
            if (!right && frame.x > edge.width) || !edge.enabled || edge.action == Action::Disabled
            {
                self.cancel();
                return None;
            }
            self.state = State::Tracking {
                right,
                start_x: frame.x,
                start_y: frame.y,
                last_y: frame.y,
                active: false,
            };
            return None;
        }
        let State::Tracking {
            right,
            start_x,
            start_y,
            last_y,
            active,
        } = &mut self.state
        else {
            return None;
        };
        let edge = if *right { &config.right } else { &config.left };
        let outside = if *right {
            frame.x < 1.0 - edge.width * 1.5
        } else {
            frame.x > edge.width * 1.5
        };
        if outside || (frame.x - *start_x).abs() > edge.width {
            self.cancel();
            return None;
        }
        if !*active {
            if (frame.y - *start_y).abs() < edge.threshold
                || (frame.y - *start_y).abs() < (frame.x - *start_x).abs() * 1.5
            {
                return None;
            }
            // Consume activation distance, avoiding a sudden initial jump.
            *active = true;
            *last_y = frame.y;
            return None;
        }
        let movement = *last_y - frame.y;
        *last_y = frame.y;
        if movement.abs() > 0.25 {
            self.cancel();
            return None;
        }
        let delta = movement * edge.sensitivity * if edge.invert { -1.0 } else { 1.0 };
        (delta.abs() > f64::EPSILON).then_some(Adjustment {
            action: edge.action,
            delta,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn f(x: f64, y: f64, fingers: usize) -> Frame {
        Frame {
            x,
            y,
            fingers,
            button: false,
            palm: false,
        }
    }
    #[test]
    fn upward_and_downward() {
        let c = Config::default();
        let mut r = Recognizer::default();
        assert!(r.update(f(0.02, 0.8, 1), &c).is_none());
        assert!(r.update(f(0.02, 0.7, 1), &c).is_none());
        let a = r.update(f(0.02, 0.6, 1), &c).unwrap();
        assert_eq!(a.action, Action::Volume);
        assert!((a.delta - 0.1).abs() < 1e-8);
        assert!(r.update(f(0.02, 0.65, 1), &c).unwrap().delta < 0.0);
    }
    #[test]
    fn center_cannot_enter_edge() {
        let c = Config::default();
        let mut r = Recognizer::default();
        for (x, y) in [(0.5, 0.8), (0.01, 0.7), (0.01, 0.5)] {
            assert!(r.update(f(x, y, 1), &c).is_none());
        }
    }
    #[test]
    fn multitouch_cancels_until_release() {
        let c = Config::default();
        let mut r = Recognizer::default();
        for (y, n) in [(0.8, 1), (0.7, 2), (0.6, 1), (0.5, 1)] {
            assert!(r.update(f(0.01, y, n), &c).is_none());
        }
        r.update(f(0.01, 0.5, 0), &c);
        r.update(f(0.01, 0.8, 1), &c);
        r.update(f(0.01, 0.7, 1), &c);
        assert!(r.update(f(0.01, 0.6, 1), &c).is_some());
    }
    #[test]
    fn right_inversion_and_disabled() {
        let mut c = Config::default();
        c.right.invert = true;
        let mut r = Recognizer::default();
        r.update(f(0.99, 0.8, 1), &c);
        r.update(f(0.99, 0.7, 1), &c);
        let a = r.update(f(0.99, 0.6, 1), &c).unwrap();
        assert_eq!(a.action, Action::Brightness);
        assert!(a.delta < 0.0);
        c.enabled = false;
        assert!(r.update(f(0.99, 0.5, 1), &c).is_none());
    }
    #[test]
    fn clicks_palms_and_discontinuities() {
        for kind in 0..3 {
            let c = Config::default();
            let mut r = Recognizer::default();
            r.update(f(0.01, 0.8, 1), &c);
            let mut frame = f(0.01, 0.7, 1);
            frame.button = kind == 0;
            frame.palm = kind == 1;
            if kind == 2 {
                r.cancel();
            }
            assert!(r.update(frame, &c).is_none());
            assert!(r.update(f(0.01, 0.6, 1), &c).is_none());
        }
    }
    #[test]
    fn sensitivity_does_not_trigger_jump_rejection() {
        let mut config = Config::default();
        config.left.sensitivity = 5.0;
        let mut recognizer = Recognizer::default();
        recognizer.update(f(0.02, 0.8, 1), &config);
        recognizer.update(f(0.02, 0.7, 1), &config);
        assert!((recognizer.update(f(0.02, 0.6, 1), &config).unwrap().delta - 0.5).abs() < 1e-8);
    }
    #[test]
    fn leaving_edge_requires_lift() {
        let config = Config::default();
        let mut recognizer = Recognizer::default();
        for (x, y) in [(0.02, 0.8), (0.5, 0.7), (0.02, 0.6), (0.02, 0.5)] {
            assert!(recognizer.update(f(x, y, 1), &config).is_none());
        }
    }
    #[test]
    fn short_motion_does_not_activate() {
        let config = Config::default();
        let mut recognizer = Recognizer::default();
        for y in [0.8, 0.79, 0.78, 0.81, 0.8] {
            assert!(recognizer.update(f(0.02, y, 1), &config).is_none());
        }
    }
}
