use crate::Person;

pub trait Traffic {
    fn peek(&self) -> Person;
    fn pop(&mut self) -> Person;
    fn set_freq(&mut self, _freq: u64) {}
    fn name(&self) -> &'static str {
        ""
    }
}

#[derive(Clone)]
pub struct Until<T: Traffic> {
    inner: T,
    until: u64,
}

impl<T: Traffic> Until<T> {
    pub fn new(until: u64, inner: T) -> Self {
        Self { until, inner }
    }
}

impl<T: Traffic> Traffic for Until<T> {
    fn peek(&self) -> Person {
        let inner_peek = self.inner.peek();
        if inner_peek.req_time < self.until {
            inner_peek
        } else {
            Person {
                dest: 0,
                src: 0,
                req_time: u64::MAX,
            }
        }
    }

    fn pop(&mut self) -> Person {
        let inner_peek = self.inner.peek();
        if inner_peek.req_time < self.until {
            self.inner.pop()
        } else {
            Person {
                dest: 0,
                src: 0,
                req_time: u64::MAX,
            }
        }
    }

    fn set_freq(&mut self, freq: u64) {
        self.inner.set_freq(freq);
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

/// A person at a random floor wants to go to a random floor every `freq`.
#[derive(Clone)]
pub struct Random {
    rng: fastrand::Rng,
    num_floors: usize,
    freq: u64,
    prev_time: u64,
    next: Person,
}

impl Random {
    fn gen_next(&mut self) {
        let req_time = self.prev_time + self.freq;
        let src = self.rng.usize(0..self.num_floors);
        let mut dest = src;
        while dest == src {
            dest = self.rng.usize(0..self.num_floors);
        }
        self.prev_time = req_time;
        self.next = Person { src, dest, req_time };
    }

    pub fn new(num_floors: usize, freq: u64) -> Self {
        let rng = fastrand::Rng::with_seed(8549089435);
        let prev_time = 0;
        let next = Person::default();
        let mut t = Self {
            num_floors,
            rng,
            freq,
            next,
            prev_time,
        };
        t.gen_next();
        t
    }
}

impl Traffic for Random {
    fn peek(&self) -> Person {
        self.next
    }

    fn pop(&mut self) -> Person {
        let n = self.next;
        self.gen_next();
        n
    }

    fn set_freq(&mut self, freq: u64) {
        self.freq = freq;
    }

    fn name(&self) -> &'static str {
        "Random"
    }
}
