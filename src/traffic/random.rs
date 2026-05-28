use crate::Traffic;

/// A person at a random floor wants to go to a random floor every `period`.
#[derive(Clone)]
pub struct Random {
    src_distr: Box<[f64]>,
    dest_distr: Box<[f64]>,
    period: u64,
    rng: fastrand::Rng,
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
            period: u64::MAX,
            rng: fastrand::Rng::with_seed(8549089435),
        };
        t.scale(per_sec);
        t
    }

    fn prefix_sum(size: usize, mut data: Vec<f64>) -> Box<[f64]> {
        assert!(data.len() <= size);
        data.resize(size, 1.0);
        assert_eq!(data.len(), size);
        for i in 1..data.len() {
            data[i] += data[i - 1];
        }
        data.into()
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
}

impl Traffic for Random {
    fn when(&self) -> u64 {
        self.period
    }

    fn next(&mut self, _time: u64) -> (usize, usize) {
        if self.rng.bool() {
            let src = Self::sample(&mut self.rng, &self.src_distr, None);
            (src, Self::sample(&mut self.rng, &self.dest_distr, Some(src)))
        } else {
            let dest = Self::sample(&mut self.rng, &self.dest_distr, None);
            (Self::sample(&mut self.rng, &self.src_distr, Some(dest)), dest)
        }
    }

    fn scale(&mut self, per_sec: f64) {
        self.period = (1000.0 / per_sec).round() as u64;
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
