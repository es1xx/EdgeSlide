use crate::Frame;
/// Linux evdev single-contact compatibility axes, with tool-count safeguards.
/// This never interprets relative pointer motion or injects input.
pub struct Decoder {
    range: [(i32, i32); 2],
    xy: [i32; 2],
    touching: bool,
    finger: bool,
    multiple: [bool; 4],
    buttons: [bool; 3],
    palm: bool,
    pub dropped: bool,
    pub resync_ready: bool,
}
impl Decoder {
    pub fn new(range: [(i32, i32); 2]) -> Self {
        Self {
            range,
            xy: [0; 2],
            touching: false,
            finger: false,
            multiple: [false; 4],
            buttons: [false; 3],
            palm: false,
            dropped: false,
            resync_ready: false,
        }
    }
    pub fn push(&mut self, kind: u16, code: u16, value: i32) -> Option<Frame> {
        if kind == 0 && code == 3 {
            self.dropped = true;
        }
        if self.dropped {
            if kind == 0 && code == 0 {
                self.resync_ready = true;
            }
            return None;
        }
        match (kind, code) {
            (3, 0..=1) => self.xy[code as usize] = value,
            (3, 55) => self.palm = value == 2, // ABS_MT_TOOL_TYPE / MT_TOOL_PALM
            (1, 330) => self.touching = value != 0, // BTN_TOUCH
            (1, 325) => self.finger = value != 0, // BTN_TOOL_FINGER
            (1, 333..=335) => self.multiple[(code - 333) as usize] = value != 0,
            (1, 328) => self.multiple[3] = value != 0, // BTN_TOOL_QUINTTAP
            (1, 272..=274) => self.buttons[(code - 272) as usize] = value != 0,
            (0, 0) => return Some(self.frame()), // SYN_REPORT commits the entire contact frame
            _ => {}
        }
        None
    }
    fn frame(&self) -> Frame {
        let normalized = |index: usize| {
            f64::from(self.xy[index] - self.range[index].0)
                / f64::from(self.range[index].1 - self.range[index].0)
        };
        Frame {
            x: normalized(0),
            y: normalized(1),
            fingers: if !self.touching {
                0
            } else if !self.finger || self.multiple.iter().any(|v| *v) {
                2
            } else {
                1
            },
            button: self.buttons.iter().any(|v| *v),
            palm: self.palm,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_boundary_and_tool_counts() {
        let mut d = Decoder::new([(100, 1100), (0, 1000)]);
        for (kind, code, value) in [(3, 0, 150), (3, 1, 800), (1, 330, 1), (1, 325, 1)] {
            assert!(d.push(kind, code, value).is_none());
        }
        let f = d.push(0, 0, 0).unwrap();
        assert_eq!(f.fingers, 1);
        assert_eq!(f.x, 0.05);
        d.push(1, 333, 1);
        assert_eq!(d.push(0, 0, 0).unwrap().fingers, 2);
        d.push(1, 330, 0);
        assert_eq!(d.push(0, 0, 0).unwrap().fingers, 0);
    }
    #[test]
    fn dropped_stream_emits_nothing() {
        let mut d = Decoder::new([(0, 1000), (0, 1000)]);
        d.push(0, 3, 0);
        for (kind, code, value) in [(1, 330, 1), (3, 1, 500), (0, 0, 0)] {
            assert!(d.push(kind, code, value).is_none());
        }
        assert!(d.dropped);
    }
}
