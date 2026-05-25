use crate::policy::{Decision, Policy};
use crate::{Building, Person};

pub struct Scan {
    moving_up: Box<[bool]>,
}

impl Scan {
    fn scan_distance_and_direction(num_floors: usize, from: usize, to: usize, moving_up: bool) -> (u64, bool) {
        let top = num_floors - 1;
        if moving_up {
            if to >= from {
                (to.abs_diff(from) as u64, moving_up)
            } else {
                ((top - from + top - to) as u64, !moving_up)
            }
        } else if to <= from {
            (from.abs_diff(to) as u64, moving_up)
        } else {
            ((from + to) as u64, !moving_up)
        }
    }

    fn next_floor(floor: usize, target: usize, moving_up: bool) -> usize {
        if target == floor {
            floor
        } else if moving_up {
            floor + 1
        } else {
            floor - 1
        }
    }

    fn estimated_latency(building: &Building, person: &Person, pos: u64, moving_up: bool) -> u64 {
        let num_floors = building.floors.len();
        let current_floor = (pos / building.time_per_floor) as usize;

        let (pickup_floors, pickup_direction) = Self::scan_distance_and_direction(num_floors, current_floor, person.src, moving_up);
        let pickup_time = (pickup_floors * building.time_per_floor).saturating_sub(pos % building.time_per_floor);

        let (dropoff_floors, _) = Self::scan_distance_and_direction(num_floors, person.src, person.dest, pickup_direction);

        pickup_time + dropoff_floors * building.time_per_floor
    }
}

impl Policy for Scan {
    fn new(building: &Building) -> Self {
        Self {
            moving_up: vec![true; building.elevators.len()].into(),
        }
    }

    fn arrival(&mut self, building: &Building, decision: &mut Decision, elevator: usize) {
        let floor = building.elevators[elevator].floor(building.time_per_floor);
        if floor == building.floors.len() - 1 {
            self.moving_up[elevator] = false;
        } else if floor == 0 {
            self.moving_up[elevator] = true;
        }
        decision.dests[elevator] = Some(if self.moving_up[elevator] { floor + 1 } else { floor - 1 });
    }

    fn request(&mut self, building: &Building, decision: &mut Decision, person: &Person) -> usize {
        let (elevator, _) = (0..decision.dests.len())
            .map(|i| {
                let e = &building.elevators[i];
                Self::estimated_latency(building, person, e.pos, self.moving_up[i])
            })
            .enumerate()
            .min_by_key(|(_, v)| *v)
            .unwrap();

        if decision.dests[elevator].is_none() {
            let floor = building.elevators[elevator].floor(building.time_per_floor);
            self.moving_up[elevator] = person.src >= floor;
            decision.dests[elevator] = Some(Self::next_floor(floor, person.src, self.moving_up[elevator]));
        }

        elevator
    }

    fn name(&self) -> &'static str {
        "Scan"
    }
}
