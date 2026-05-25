use crate::policy::{Decision, Policy};
use crate::{Building, Person};

use std::collections::BinaryHeap;

pub struct GreedyUtilitarian {
    /// count, src, dest
    counts: Vec<(usize, usize, usize)>,
    refresh_time: u64,
    last_refresh: u64,
    floors: usize,
    elevators: usize,
    elevator_assignments: Vec<usize>,
}

impl GreedyUtilitarian {
    fn assign(&mut self) {
        self.counts.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        let mut loads = vec![0usize; self.elevators];

        for &(count, src, dest) in &self.counts {
            let e = loads.iter().enumerate().min_by_key(|(_, load)| *load).unwrap().0;
            let i = self.index(src, dest);
            self.elevator_assignments[i] = e;
            loads[e] += count;
        }
        self.new_counts();
    }

    fn new_counts(&mut self) {
        self.counts = Vec::with_capacity(self.floors * self.floors);
        for src in 0..self.floors {
            for dest in 0..self.floors {
                self.counts.push((0, src, dest))
            }
        }
    }

    fn index(&self, src: usize, dest: usize) -> usize {
        src * self.floors + dest
    }

    fn maybe_refresh(&mut self, now: u64) {
        if now - self.last_refresh > self.refresh_time {
            self.assign();
            self.last_refresh = now;
            self.refresh_time *= 2;
        }
    }

    fn set_dir(&self, b: &Building, out: &mut Decision) {
        assert_eq!(out.dests.len(), self.elevators);
        assert_eq!(self.floors, b.floors.len());
        for e in 0..self.elevators {
            let mut scores = BinaryHeap::<(u64, i64)>::new();
            for f in 0..self.floors {
                let cap = b.elevators[e].capacity;
                let space_left = cap - b.elevators[e].passengers.len();
                let incoming = b.floors[f].people[e].iter().count().min(space_left);
                let outgoing = b.elevators[e].passengers.iter().filter(|p| p.dest == f).count();
                let exchange = incoming + outgoing;
                if exchange == 0 {
                    continue;
                }
                let travel_time = (b.arrival_time(e, f) - b.prev_time) as f64;
                let passengers_per_ms = exchange as f64 / travel_time;
                let score = (passengers_per_ms * 1000.0).round() as u64;
                scores.push((score, f as i64 * -1));
            }
            if let Some((_, floor)) = scores.pop() {
                out.dests[e] = Some((-1 * floor) as usize);
            }
        }
    }
}

impl Policy for GreedyUtilitarian {
    fn new(b: &Building) -> Self {
        let floors = b.floors.len();
        let mut res = Self {
            counts: Default::default(),
            elevator_assignments: vec![0; floors * floors],
            floors,
            elevators: b.elevators.len(),
            refresh_time: 10_000,
            last_refresh: 0,
        };
        res.new_counts();
        for c in &mut res.counts {
            c.0 += 1;
        }
        res.assign();
        res
    }

    fn arrival(&mut self, b: &Building, decision: &mut Decision, _: usize) {
        self.set_dir(b, decision);
        self.maybe_refresh(b.prev_time);
    }

    fn request(&mut self, b: &Building, decision: &mut Decision, p: &Person) -> usize {
        let i = self.index(p.src, p.dest);
        self.counts[i].0 += 1;
        self.set_dir(b, decision);
        self.maybe_refresh(b.prev_time);
        self.elevator_assignments[i]
    }

    fn name(&self) -> &'static str {
        "GreedyUtilitarian"
    }
}
