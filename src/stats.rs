use crate::Person;

use sketches_ddsketch::DDSketch;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct Stats {
    pub start: u64,
    pub window: u64,
    pub latencies: VecDeque<DDSketch>,
    pub served: VecDeque<usize>,
}

impl Stats {
    pub fn new(window: u64, start: u64) -> Self {
        Self {
            start,
            window,
            latencies: Default::default(),
            served: Default::default(),
        }
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
        while self.end() <= until {
            self.latencies.push_back(Default::default());
            self.served.push_back(Default::default());
        }
    }

    pub fn trim(&mut self, to: usize) {
        if to >= self.len() {
            return;
        }
        let removed = self.len() - to;
        let _ = self.latencies.drain(0..removed);
        let _ = self.served.drain(0..removed);
        self.start += removed as u64 * self.window
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
