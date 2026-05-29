use crate::policy::{Decision, Policy};
use crate::{Building, Person};

pub struct Simple {
    rng: fastrand::Rng,
}

impl Default for Simple {
    fn default() -> Self {
        Self {
            rng: fastrand::Rng::with_seed(2453),
        }
    }
}

impl Policy for Simple {
    fn new(_: &Building) -> Self {
        Self::default()
    }

    fn arrival(&mut self, building: &Building, decision: &mut Decision, elevator: usize) {
        let floor = building.elevators[elevator].floor(building.time_per_floor);
        decision.dests[elevator] = Some((floor + 1) % building.floors.len());
    }

    fn request(&mut self, _: &Building, decision: &mut Decision, _: &Person) -> usize {
        let elevator = self.rng.usize(0..decision.dests.len());
        if decision.dests[elevator].is_none() {
            decision.dests[elevator] = Some(0);
        }
        elevator
    }
}
