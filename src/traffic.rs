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

pub struct Nothing;

impl Traffic for Nothing {
    fn when(&self) -> u64 {
        u64::MAX
    }

    fn next(&mut self, _: u64) -> (usize, usize) {
        unreachable!();
    }

    fn scale(&mut self, _: f64) {}

    fn name(&self) -> &'static str {
        "Nothing"
    }
}

/// Cycles between traffics for duration.
///
/// `Cycle` waits to generate at least one request from between[cur], even if
/// between[cur].when() > durations[cur].
pub struct Cycle {
    between: Box<[Box<dyn Traffic>]>,
    durations: Box<[u64]>,
    cur: usize,
    prev_time: u64,
}

impl Cycle {
    pub fn new(between: Vec<Box<dyn Traffic>>, durations: Vec<u64>) -> Self {
        assert_eq!(between.len(), durations.len());
        Self {
            between: between.into(),
            durations: durations.into(),
            cur: 0,
            prev_time: 0,
        }
    }
}

impl Traffic for Cycle {
    fn when(&self) -> u64 {
        self.between[self.cur].when()
    }

    fn next(&mut self, time: u64) -> (usize, usize) {
        let res = self.between[self.cur].next(time);
        let since_last_cycle = time - self.prev_time;
        if since_last_cycle > self.durations[self.cur] {
            self.prev_time = time;
            self.cur = (self.cur + 1) % self.durations.len();
        }
        // TODO, can also cycle if self.between[self.cur].when() >
        // self.durations[self.cur]
        res
    }

    fn scale(&mut self, x: f64) {
        for t in self.between.iter_mut() {
            t.scale(x);
        }
    }

    fn name(&self) -> &'static str {
        "Cycle"
    }
}

/// `pre` traffic for `remaining`, then `post` traffic.
pub struct Then {
    inner: Cycle,
}

impl Then {
    pub fn new(remaining: u64, pre: Box<dyn Traffic>, post: Box<dyn Traffic>) -> Self {
        Self {
            inner: Cycle::new(vec![pre, post], vec![remaining, u64::MAX]),
        }
    }
}

/// Returns traffic for the duration, then no traffic.
pub struct Duration {
    inner: Then,
}

impl Duration {
    pub fn new(remaining: u64, inner: impl Traffic + 'static) -> Self {
        Self {
            inner: Then::new(remaining, Box::new(inner), Box::new(Nothing)),
        }
    }
}

pub struct Delay {
    inner: Then,
}

impl Delay {
    pub fn new(remaining: u64, inner: impl Traffic + 'static) -> Self {
        Self {
            inner: Then::new(remaining, Box::new(Nothing), Box::new(inner)),
        }
    }
}

macro_rules! impl_traffic_from_inner {
    ($type:ty, $inner:ident) => {
        impl Traffic for $type {
            fn when(&self) -> u64 {
                self.$inner.when()
            }

            fn next(&mut self, time: u64) -> (usize, usize) {
                self.$inner.next(time)
            }

            fn scale(&mut self, x: f64) {
                self.$inner.scale(x)
            }

            fn name(&self) -> &'static str {
                stringify!($type)
            }
        }
    };
}

impl_traffic_from_inner!(Then, inner);
impl_traffic_from_inner!(Duration, inner);
impl_traffic_from_inner!(Delay, inner);
