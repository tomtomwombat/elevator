use crate::Person;

mod random;
pub use random::Random;

pub trait Traffic {
    fn peek(&self) -> Person;
    fn pop(&mut self) -> Person;
    fn scale(&mut self, _x: f64) {}
    fn name(&self) -> &'static str;
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

    fn scale(&mut self, x: f64) {
        self.inner.scale(x);
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}
