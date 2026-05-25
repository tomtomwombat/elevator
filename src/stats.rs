use crate::Person;

use sketches_ddsketch::DDSketch;
use std::collections::VecDeque;

#[derive(Clone, Default)]
pub struct Window {
    pub latencies: DDSketch,
    pub served: usize,
}

#[derive(Clone)]
pub struct Stats {
    pub start: u64,
    pub window: u64,
    pub windows: VecDeque<Window>,
}

impl Stats {
    pub fn new(window: u64) -> Self {
        Self {
            start: 0,
            window,
            windows: Default::default(),
        }
    }

    /// Records when a person reaches their destination.
    pub fn add(&mut self, time: u64, person: &Person) {
        self.tick(time);
        self.windows.back_mut().unwrap().served += 1;
        let latency = time - person.req_time;
        self.windows.back_mut().unwrap().latencies.add(latency as f64);
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn end(&self) -> u64 {
        self.len() as u64 * self.window + self.start
    }

    pub fn tick(&mut self, until: u64) {
        while self.end() <= until {
            self.windows.push_back(Default::default());
        }
    }

    pub fn trim(&mut self, to: usize) {
        if to >= self.len() {
            return;
        }
        let removed = self.len() - to;
        let _ = self.windows.drain(0..removed);
        self.start += removed as u64 * self.window
    }

    pub fn throughput_history(&self) -> Vec<usize> {
        self.windows.iter().map(|w| w.served).collect()
    }

    pub fn latency_history(&self, q: f64) -> Vec<f64> {
        let mut prev = 0.0;
        self.windows
            .iter()
            .map(|w| {
                prev = w.latencies.quantile(q).unwrap().unwrap_or(prev);
                prev
            })
            .collect()
    }
}
