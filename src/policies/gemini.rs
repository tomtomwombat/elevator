use crate::policy::{Decision, Policy};
use crate::{Building, Person};
use std::collections::BTreeSet;

/// Gemini is a LOOK-based elevator policy.
/// It prioritizes targets in the current direction of travel and only reverses
/// when no further requests exist in that direction.
pub struct Gemini {
    targets: Vec<BTreeSet<usize>>,
    moving_up: Vec<bool>,
}

impl Policy for Gemini {
    fn new(building: &Building) -> Self {
        Self {
            targets: vec![BTreeSet::new(); building.elevators.len()],
            moving_up: vec![true; building.elevators.len()],
        }
    }

    fn arrival(&mut self, building: &Building, decision: &mut Decision, elevator: usize) {
        let e = &building.elevators[elevator];
        let floor = e.floor(building.time_per_floor);

        // 1. Clear current floor and sync with current passengers
        self.targets[elevator].remove(&floor);
        for p in &e.passengers {
            self.targets[elevator].insert(p.dest);
        }

        let next_above = self.targets[elevator].range((floor + 1)..).next().copied();
        let next_below = self.targets[elevator].range(..floor).next_back().copied();

        // 2. LOOK Algorithm: Find the actual next target floor, skip intermediate
        //    floors
        if self.moving_up[elevator] {
            if let Some(target) = next_above {
                decision.dests[elevator] = Some(target);
            } else if let Some(target) = next_below {
                self.moving_up[elevator] = false;
                decision.dests[elevator] = Some(target);
            } else {
                decision.dests[elevator] = None;
            }
        } else {
            if let Some(target) = next_below {
                decision.dests[elevator] = Some(target);
            } else if let Some(target) = next_above {
                self.moving_up[elevator] = true;
                decision.dests[elevator] = Some(target);
            } else {
                decision.dests[elevator] = None;
            }
        }
    }

    fn request(&mut self, building: &Building, decision: &mut Decision, person: &Person) -> usize {
        let elevator_idx = (0..building.elevators.len())
            .min_by_key(|&i| {
                let e = &building.elevators[i];
                let floor = e.floor(building.time_per_floor);
                let dist = floor.abs_diff(person.src) as u64;

                // Penalty for elevators moving away from the request floor.
                let penalty = match decision.dests[i] {
                    Some(target) if (target > floor && person.src < floor) || (target < floor && person.src > floor) => {
                        building.floors.len() as u64
                    }
                    _ => 0,
                };
                dist + penalty
            })
            .unwrap();

        self.targets[elevator_idx].insert(person.src);

        // Update destination if the new request is "on the way" or elevator is idle
        let e = &building.elevators[elevator_idx];
        let floor = e.floor(building.time_per_floor);
        match decision.dests[elevator_idx] {
            None => {
                self.moving_up[elevator_idx] = person.src >= floor;
                decision.dests[elevator_idx] = Some(person.src);
            }
            Some(curr_target) => {
                // If new request is between us and current target, stop there first.
                if (self.moving_up[elevator_idx] && person.src > floor && person.src < curr_target)
                    || (!self.moving_up[elevator_idx] && person.src < floor && person.src > curr_target)
                {
                    decision.dests[elevator_idx] = Some(person.src);
                }
            }
        }
        elevator_idx
    }
}
