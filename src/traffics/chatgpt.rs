use crate::Person;
use crate::traffic::Traffic;

#[derive(Clone)]
pub struct ChatGpt {
    rng: fastrand::Rng,
    floors: usize,
    load: f64,
    prev_time: u64,
    next: Person,
    morning_until: u64,
    evening_start: u64,
}

impl ChatGpt {
    pub fn new(floors: usize, load: f64) -> Self {
        assert!(floors >= 2);
        assert!(load.is_finite() && load > 0.0);
        let rng = fastrand::Rng::with_seed(0xC0DE_2026);
        let mut traffic = Self {
            rng,
            floors,
            load,
            prev_time: 0,
            next: Person::default(),
            morning_until: 2 * 60 * 60 * 1_000,
            evening_start: 6 * 60 * 60 * 1_000,
        };
        traffic.gen_next();
        traffic
    }

    fn scaled_interval(base: u64, load: f64) -> u64 {
        ((base as f64 / load).round() as u64).max(1)
    }

    fn gen_next(&mut self) {
        let delta = self.next_delta();
        let req_time = self.prev_time + delta;
        let (src, dest) = self.trip(req_time);
        self.prev_time = req_time;
        self.next = Person { src, dest, req_time };
    }

    fn next_delta(&mut self) -> u64 {
        let base = if self.prev_time < self.morning_until {
            6_000
        } else if self.prev_time >= self.evening_start {
            8_000
        } else {
            20_000
        };
        let base = Self::scaled_interval(base, self.load);
        let low = (base / 2).max(1);
        let high = (base * 3 / 2).max(low + 1);
        self.rng.u64(low..high)
    }

    fn trip(&mut self, time: u64) -> (usize, usize) {
        if time < self.morning_until && self.rng.u32(0..100) < 70 {
            (0, self.rng.usize(1..self.floors))
        } else if time >= self.evening_start && self.rng.u32(0..100) < 70 {
            (self.rng.usize(1..self.floors), 0)
        } else {
            let src = self.rng.usize(0..self.floors);
            let mut dest = self.rng.usize(0..self.floors);
            while dest == src {
                dest = self.rng.usize(0..self.floors);
            }
            (src, dest)
        }
    }
}

impl Traffic for ChatGpt {
    fn peek(&self) -> Person {
        self.next
    }

    fn pop(&mut self) -> Person {
        let next = self.next;
        self.gen_next();
        next
    }
}
