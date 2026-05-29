use crate::Person;

use sketches_ddsketch::DDSketch;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct Stats {
    pub max_len: usize,
    pub start: u64,
    pub window: u64,
    pub latencies: VecDeque<DDSketch>,
    pub served: VecDeque<usize>,
}

impl Stats {
    pub fn new(window: u64, start: u64, max_len: usize) -> Self {
        Self {
            max_len,
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        assert_eq!(self.latencies.len(), self.served.len());
        self.latencies.len()
    }

    pub fn end(&self) -> u64 {
        self.len() as u64 * self.window + self.start
    }

    pub fn tick(&mut self, until: u64) {
        if until < self.start {
            return;
        }
        while self.end() <= until {
            if self.len() >= self.max_len {
                self.pop_front();
            }
            self.latencies.push_back(Default::default());
            self.served.push_back(Default::default());
        }
        assert!(self.latencies.len() <= self.max_len);
        assert!(self.start <= until);
        assert!(self.end() > self.start);
        assert!(self.end() - self.start <= (self.max_len as u64 * self.window));
        assert!(self.len() <= self.max_len);
    }

    pub fn trim(&mut self, to: usize) {
        while self.len() > to {
            self.pop_front();
        }
        assert!(self.len() <= to);
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

    fn last(&self) -> Option<usize> {
        (!self.is_empty()).then_some(self.len().saturating_sub(2))
    }

    pub fn latency(&self, q: f64) -> f64 {
        self.last().map_or(0.0, |i| self.latencies[i].quantile(q).unwrap().unwrap_or(0.0))
    }

    pub fn served(&self) -> usize {
        self.last().map_or(0, |i| self.served[i])
    }

    pub fn latencies(&self, q: f64) -> impl Iterator<Item = f64> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_until_start() {
        let mut s = Stats::new(10, 1, 100);
        s.tick(0);
        s.tick(1);
        s.tick(10000);
    }
}
