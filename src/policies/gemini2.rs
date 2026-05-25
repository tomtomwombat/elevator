use crate::policy::{Decision, Policy};
use crate::{Building, Person};
use std::collections::BTreeSet;

/// Gemini2: Advanced Dispatching Policy
/// Focuses on ETA (Estimated Time of Arrival) rather than just distance.
/// Accounts for intermediate stops and directional momentum.
pub struct Gemini2 {
    targets: Vec<BTreeSet<usize>>,
    moving_up: Vec<bool>,
}

impl Policy for Gemini2 {
    fn new(building: &Building) -> Self {
        Self {
            targets: vec![BTreeSet::new(); building.elevators.len()],
            moving_up: vec![true; building.elevators.len()],
        }
    }

    fn arrival(&mut self, building: &Building, decision: &mut Decision, elevator: usize) {
        let e = &building.elevators[elevator];
        let floor = e.floor(building.time_per_floor);

        // 1. Update targets based on current passengers and current floor exit
        self.targets[elevator].remove(&floor);
        for p in &e.passengers {
            self.targets[elevator].insert(p.dest);
        }

        let next_above = self.targets[elevator].range((floor + 1)..).next().copied();
        let next_below = self.targets[elevator].range(..floor).next_back().copied();

        // 2. LOOK logic for high-throughput movement
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
                let current_floor = e.floor(building.time_per_floor);
                let target_floor = person.src;

                // Calculate basic distance
                let dist = current_floor.abs_diff(target_floor) as u64;

                // Calculate "Inertia Cost"
                let cost = match decision.dests[i] {
                    None => dist, // Idle elevators are cheap
                    Some(d) => {
                        let heading_up = d > current_floor;
                        let person_above = target_floor >= current_floor;

                        if heading_up == person_above {
                            // On the way
                            dist
                        } else {
                            // Requires a full trip to current destination and back
                            let dist_to_dest = current_floor.abs_diff(d) as u64;
                            let dist_from_dest_to_person = d.abs_diff(target_floor) as u64;
                            dist_to_dest + dist_from_dest_to_person
                        }
                    }
                };

                // Throughput optimization: Add cost for every stop already committed.
                // Each stop adds a "time penalty" for the new passenger.
                let stop_penalty = (e.passengers.len() + self.targets[i].len()) as u64 * 4;

                // Capacity Hard-cap
                let capacity_penalty = if e.passengers.len() >= e.capacity { 1000 } else { 0 };

                cost + stop_penalty + capacity_penalty
            })
            .unwrap();

        self.targets[elevator_idx].insert(person.src);

        // Re-evaluate immediate destination
        let e = &building.elevators[elevator_idx];
        let floor = e.floor(building.time_per_floor);
        if let Some(curr) = decision.dests[elevator_idx] {
            // If the new person is strictly between the elevator and its current target,
            // stop early.
            if (self.moving_up[elevator_idx] && person.src > floor && person.src < curr)
                || (!self.moving_up[elevator_idx] && person.src < floor && person.src > curr)
            {
                decision.dests[elevator_idx] = Some(person.src);
            }
        } else {
            self.moving_up[elevator_idx] = person.src >= floor;
            decision.dests[elevator_idx] = Some(person.src);
        }

        elevator_idx
    }

    fn name(&self) -> &'static str {
        "Gemini2"
    }
}
