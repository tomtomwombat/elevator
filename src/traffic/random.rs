use crate::{Person, Traffic};

/// A person at a random floor wants to go to a random floor every `freq`.
#[derive(Clone)]
pub struct Random {
    src_distr: Vec<f64>,
    dest_distr: Vec<f64>,
    freq: u64,
    rng: fastrand::Rng,
    prev_time: u64,
    next: Person,
}

impl Random {
    /// # Example
    /// Traffic that spawns 0.1 people per second randomly in a 20 story
    /// building, with src and destination floors being 5 times more likely to
    /// be the ground floor.
    /// ```
    /// use elevator::traffic::Random;
    ///
    /// let r = Random::new(20, vec![5.0], vec![5.0], 0.1);
    /// ```
    pub fn new(floors: usize, src_distr: Vec<f64>, dest_distr: Vec<f64>, per_sec: f64) -> Self {
        let mut t = Self {
            src_distr: Self::prefix_sum(floors, src_distr),
            dest_distr: Self::prefix_sum(floors, dest_distr),
            freq: u64::MAX,
            rng: fastrand::Rng::with_seed(8549089435),
            next: Person::default(),
            prev_time: 0,
        };
        t.scale(per_sec);
        t.gen_next();
        t
    }

    fn prefix_sum(size: usize, mut data: Vec<f64>) -> Vec<f64> {
        assert!(data.len() <= size);
        data.resize(size, 1.0);
        assert_eq!(data.len(), size);
        for i in 1..data.len() {
            data[i] += data[i - 1];
        }
        data
    }

    fn sample(rng: &mut fastrand::Rng, prefix_sum: &[f64], exclude: Option<usize>) -> usize {
        let l = prefix_sum.len();
        let excluded_value = match exclude {
            Some(e) if e > 0 => prefix_sum[e] - prefix_sum[e - 1],
            Some(e) => prefix_sum[e],
            None => 0.0,
        };
        let total = prefix_sum[l - 1] - excluded_value;
        let mut s = rng.f64() * total;
        for i in 0..l {
            if Some(i) == exclude {
                s += excluded_value;
            }
            if s <= prefix_sum[i] {
                return i;
            }
        }
        unreachable!();
    }

    fn gen_next(&mut self) {
        let req_time = self.prev_time.saturating_add(self.freq);
        let (src, dest) = match self.rng.bool() {
            true => {
                let src = Self::sample(&mut self.rng, &self.src_distr, None);
                (src, Self::sample(&mut self.rng, &self.dest_distr, Some(src)))
            }
            false => {
                let dest = Self::sample(&mut self.rng, &self.dest_distr, None);
                (Self::sample(&mut self.rng, &self.src_distr, Some(dest)), dest)
            }
        };
        self.prev_time = req_time;
        self.next = Person { src, dest, req_time };
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

    fn scale(&mut self, per_sec: f64) {
        assert!(per_sec > 0.0f64, "todo, might requiring changing traffic API");
        let freq = (1000.0 / per_sec).round() as u64;
        self.freq = freq;
    }

    fn name(&self) -> &'static str {
        "Random"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_distribution_no_exclude() {
        const N: usize = 100_000;
        const EPS: f64 = 0.02;
        let mut rng = fastrand::Rng::with_seed(12345);
        let mut weights = vec![5.0, 15.0, 3.0, 1.0];
        const NUM: usize = 5;
        let prefix_sum = Random::prefix_sum(NUM, weights.clone());
        let mut counts = [0usize; NUM];
        for _ in 0..N {
            let floor = Random::sample(&mut rng, &prefix_sum, None);
            counts[floor] += 1;
        }

        weights.resize(NUM, 1.0);
        let total: f64 = weights.iter().sum();
        let expected: Vec<f64> = (0..NUM).map(|i| weights[i] / total).collect();

        for i in 0..NUM {
            let observed = counts[i] as f64 / N as f64;

            assert!(
                (observed - expected[i]).abs() < EPS,
                "floor={i}: got {observed}, expected {}",
                expected[i]
            );
        }
    }

    #[test]
    fn test_sample_with_exclude() {
        sample_with_exclude(5, vec![5.0, 15.0, 3.0, 1.0], 1);
        sample_with_exclude(5, vec![5.0], 1);
    }

    fn sample_with_exclude(num: usize, mut weights: Vec<f64>, exclude: usize) {
        const N: usize = 50_000;
        const EPS: f64 = 0.02;
        let mut rng = fastrand::Rng::with_seed(12345);
        let prefix_sum = Random::prefix_sum(num, weights.clone());
        let mut counts = vec![0usize; num];
        for _ in 0..N {
            let floor = Random::sample(&mut rng, &prefix_sum, Some(exclude));
            counts[floor] += 1;
        }

        weights.resize(num, 1.0);
        let total: f64 = weights.iter().sum::<f64>() - weights[exclude];
        let expected: Vec<f64> = (0..num).map(|i| if i == exclude { 0.0 } else { weights[i] / total }).collect();
        println!("counts = {:?}", counts);
        for i in 0..num {
            let observed = counts[i] as f64 / N as f64;

            assert!(
                (observed - expected[i]).abs() < EPS,
                "floor={i}: got {observed}, expected {}",
                expected[i]
            );
        }
    }
}
