use crate::policy::{Decision, Policy};
use crate::{Building, Person};

/// OpenAi is a high-performance bitmask-based LOOK policy.
/// It uses bitwise operations to find targets, making it extremely fast
/// for buildings with up to 64 floors.
pub struct OpenAi {
    targets: Vec<u64>,
    moving_up: Vec<bool>,
}

impl Policy for OpenAi {
    fn new(building: &Building) -> Self {
        Self {
            targets: vec![0; building.elevators.len()],
            moving_up: vec![true; building.elevators.len()],
        }
    }

    fn arrival(&mut self, building: &Building, decision: &mut Decision, elevator: usize) {
        let e = &building.elevators[elevator];
        let floor = e.floor(building.time_per_floor);

        // Clear current stop and sync destinations
        self.targets[elevator] &= !(1 << floor);
        for p in &e.passengers {
            self.targets[elevator] |= 1 << p.dest;
        }

        let mask = self.targets[elevator];
        if mask == 0 {
            decision.dests[elevator] = None;
            return;
        }

        let floors_above = mask & (!0 << (floor + 1));
        let floors_below = mask & ((1 << floor) - 1);

        if self.moving_up[elevator] {
            if floors_above != 0 {
                decision.dests[elevator] = Some(floors_above.trailing_zeros() as usize);
            } else {
                self.moving_up[elevator] = false;
                decision.dests[elevator] = Some(63 - floors_below.leading_zeros() as usize);
            }
        } else {
            if floors_below != 0 {
                decision.dests[elevator] = Some(63 - floors_below.leading_zeros() as usize);
            } else {
                self.moving_up[elevator] = true;
                decision.dests[elevator] = Some(floors_above.trailing_zeros() as usize);
            }
        }
    }

    fn request(&mut self, building: &Building, decision: &mut Decision, person: &Person) -> usize {
        let elevator_idx = (0..building.elevators.len())
            .min_by_key(|&i| {
                let e = &building.elevators[i];
                let floor = e.floor(building.time_per_floor);
                let dist = floor.abs_diff(person.src) as u64;

                // Directional Penalty: Heavy penalty if moving away from the request
                let dir_penalty = match decision.dests[i] {
                    Some(t) if (t > floor && person.src < floor) || (t < floor && person.src > floor) => building.floors.len() as u64,
                    _ => 0,
                };

                // Capacity Penalty: Each passenger adds a virtual "stop delay" to the cost.
                // If the elevator is at full capacity, we apply a massive penalty to favor
                // other elevators.
                let load_penalty = (e.passengers.len() * 2) as u64;
                let full_penalty = if e.passengers.len() >= e.capacity { 1000 } else { 0 };

                dist + dir_penalty + load_penalty + full_penalty
            })
            .unwrap();

        self.targets[elevator_idx] |= 1 << person.src;

        let e = &building.elevators[elevator_idx];
        let floor = e.floor(building.time_per_floor);
        match decision.dests[elevator_idx] {
            None => {
                self.moving_up[elevator_idx] = person.src >= floor;
                decision.dests[elevator_idx] = Some(person.src);
            }
            Some(curr_target) => {
                if (self.moving_up[elevator_idx] && person.src > floor && person.src < curr_target)
                    || (!self.moving_up[elevator_idx] && person.src < floor && person.src > curr_target)
                {
                    decision.dests[elevator_idx] = Some(person.src);
                }
            }
        }
        elevator_idx
    }

    fn name(&self) -> &'static str {
        "OpenAI"
    }
}
