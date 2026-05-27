mod random;
pub use random::Random;

pub trait Traffic {
    /// Returns the duration until the next request.
    fn when(&self) -> u64;
    /// Returns the source and destination floors of the next request.
    /// These floors mut not be the equal and less than the number of floors in
    /// the building.
    fn next(&mut self, _time: u64) -> (usize, usize);
    /// Increase or decrease the volume of traffic.
    /// `_scale` can be interpreted however; `scale`'s implementation is
    /// arbitrary and has no bearing on correctness.
    fn scale(&mut self, _scale: f64);
    /// Returns the name of the traffic pattern.
    fn name(&self) -> &'static str;
}

/// Returns traffic for the duration, then no traffic.
#[derive(Clone)]
pub struct Duration<T: Traffic> {
    inner: T,
    remaining: u64,
}

impl<T: Traffic> Duration<T> {
    pub fn new(remaining: u64, inner: T) -> Self {
        Self { remaining, inner }
    }
}

impl<T: Traffic> Traffic for Duration<T> {
    fn when(&self) -> u64 {
        let inner = self.inner.when();
        if inner < self.remaining { inner } else { u64::MAX }
    }

    fn next(&mut self, time: u64) -> (usize, usize) {
        let res = self.inner.next(time);
        self.remaining = self.remaining.saturating_sub(self.inner.when());
        res
    }

    fn scale(&mut self, x: f64) {
        self.inner.scale(x);
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}
