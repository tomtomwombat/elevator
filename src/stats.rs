use crate::Person;

use sketches_ddsketch::DDSketch;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct Stats {
    pub max_time_len: u64,
    pub start: u64,
    pub window: u64,
    pub latencies: VecDeque<DDSketch>,
    pub served: VecDeque<usize>,
}

impl Stats {
    pub fn new(window: u64, start: u64, max_len: usize) -> Self {
        Self {
            max_time_len: (max_len as u64).saturating_mul(window),
            start,
            window,
            latencies: Default::default(),
            served: Default::default(),
        }
    }

    pub fn reset(&mut self, time: u64, window: u64) {
        self.latencies.clear();
        self.served.clear();
        self.start = time;
        self.window = window;
    }

    /// Records when a person reaches their destination.
    pub fn add(&mut self, time: u64, person: &Person) {
        self.tick(time);
        *self.served.back_mut().unwrap() += 1;
        let latency = time - person.req_time;
        self.latencies.back_mut().unwrap().add(latency as f64);
    }

    pub fn len(&self) -> usize {
        assert_eq!(self.latencies.len(), self.served.len());
        self.latencies.len()
    }

    pub fn end(&self) -> u64 {
        self.len() as u64 * self.window + self.start
    }

    pub fn tick(&mut self, until: u64) {
        if until.saturating_sub(self.start) > self.max_time_len {
            self.trim_start(until - self.max_time_len);
        }
        while self.end() <= until {
            self.latencies.push_back(Default::default());
            self.served.push_back(Default::default());
        }
        assert!(self.latencies.len() <= (self.max_time_len / self.window) as usize + 1);
        assert!(self.start < until);
        assert!(self.end() > self.start);
        assert!(self.end() - self.start <= self.max_time_len + self.window);
    }

    pub fn trim(&mut self, to: usize) {
        while self.len() > to {
            self.pop_front();
        }
    }

    fn trim_start(&mut self, to: u64) {
        while self.start < to {
            self.pop_front();
        }
    }

    #[inline(always)]
    fn pop_front(&mut self) {
        let _ = self.latencies.pop_front();
        let _ = self.served.pop_front();
        self.start += self.window;
    }

    pub fn throughput(&self) -> impl Iterator<Item = &usize> {
        self.served.iter().take(self.len() - 1)
    }

    pub fn latency(&self, q: f64) -> impl Iterator<Item = f64> {
        let mut prev = 0.0;
        self.latencies
            .iter()
            .map(move |w| {
                prev = w.quantile(q).unwrap().unwrap_or(prev);
                prev
            })
            .take(self.len() - 1)
    }
}
