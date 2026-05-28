use crate::policy::{Decision, Policy};
use crate::{Building, Person};

/// bogo sort but for elevator scheduling
pub struct Bogo {
    rng: fastrand::Rng,
}

impl Default for Bogo {
    fn default() -> Self {
        Self {
            rng: fastrand::Rng::with_seed(69),
        }
    }
}

impl Policy for Bogo {
    fn new(_: &Building) -> Self {
        Self::default()
    }

    fn request(&mut self, _: &Building, decision: &mut Decision, _: &Person) -> usize {
        let elevator = self.rng.usize(0..decision.dests.len());
        if decision.dests[elevator].is_none() {
            decision.dests[elevator] = Some(elevator);
        }
        elevator
    }

    fn arrival(&mut self, building: &Building, decision: &mut Decision, elevator: usize) {
        let current = building.elevators[elevator].floor(building.time_per_floor);
        let num_floors = building.floors.len();
        let mut next = self.rng.usize(0..num_floors - 1);
        if next >= current {
            next += 1;
        }
        decision.dests[elevator] = Some(next);
    }
}
