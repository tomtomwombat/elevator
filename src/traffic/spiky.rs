use crate::Traffic;

/// # Example
/// ```
/// use elevator::traffic::{Random, Spiky};
///
/// let inner = Random::new(6, vec![], vec![], 0.1);
/// let traffic = Box::new(Spiky::new(inner, 10_000, 250, 1_000, 60_000, 10_000));
/// ```
#[derive(Clone)]
pub struct Spiky<T: Traffic> {
    // TODO: set the intensity of the inner instead
    inner: T,
    rise: u64,
    cur_period: u64,

    lull_period: u64,
    lull_duration: u64,

    spike_period: u64,
    spike_duration: u64,

    phase_start: u64,
    phase: Phase,
}

#[derive(Clone, Copy)]
enum Phase {
    Spike,
    Falling,
    Lull,
    Rising,
}

impl<T: Traffic> Spiky<T> {
    pub fn new(inner: T, lull_period: u64, spike_period: u64, rise: u64, lull_duration: u64, spike_duration: u64) -> Self {
        assert!(lull_period >= spike_period);
        Self {
            inner,
            lull_period,
            spike_period,
            rise,
            lull_duration,
            spike_duration,
            cur_period: lull_period,
            phase_start: 0,
            phase: Phase::Lull,
        }
    }
}

impl<T: Traffic> Traffic for Spiky<T> {
    fn when(&self) -> u64 {
        self.cur_period
    }

    fn next(&mut self, time: u64) -> (usize, usize) {
        match self.phase {
            Phase::Lull if time - self.phase_start > self.lull_duration => {
                self.phase = Phase::Rising;
                self.cur_period = self.rise;
            }
            Phase::Spike if time - self.phase_start > self.spike_duration => {
                self.phase = Phase::Falling;
                self.cur_period = self.cur_period.saturating_sub(self.rise);
            }
            Phase::Rising => {
                self.cur_period = self.cur_period.saturating_sub(self.rise);
                if self.cur_period <= self.spike_period {
                    self.cur_period = self.spike_period;
                    self.phase = Phase::Spike;
                    self.phase_start = time;
                }
            }
            Phase::Falling => {
                self.cur_period += self.rise;
                if self.cur_period >= self.lull_period {
                    self.cur_period = self.lull_period;
                    self.phase = Phase::Lull;
                    self.phase_start = time;
                }
            }
            _ => (),
        }
        self.inner.next(time)
    }

    fn scale(&mut self, _: f64) {
        // TODO
    }

    fn name(&self) -> &'static str {
        "Spiky"
    }
}
