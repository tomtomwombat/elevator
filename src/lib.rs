use std::cmp;
use std::collections::VecDeque;
use tracing::{debug, trace};

pub mod args;
pub mod controls;
pub mod policies;
pub mod policy;
pub mod simulation;
pub mod stats;
pub mod traffic;
use crate::policy::{Decision, Policy};
use crate::stats::Stats;
use crate::traffic::Traffic;

/// Maximum number of people on a floor waiting for an elevator.
/// Prevents potentially overwhelming memory when speeding up the simulator
/// massively.
const MAX_WAITING: usize = 100_000;

#[derive(Default, Debug, Copy, Clone)]
pub struct Person {
    /// Floor index that person requests elevator at
    src: usize,
    /// Floor index that person requests to go
    dest: usize,
    /// Simulation time of the request
    req_time: u64,
}

#[derive(Debug, Clone)]
pub struct Elevator {
    /// Absolute position of the elevator in time units (0 is ground floor).
    pub pos: u64,
    /// The current passengers inside the elevator
    pub passengers: Vec<Person>,
    /// Maximum number of passengers inside the elevator
    capacity: usize,
    /// Time until which the elevator is occupied with door operations
    busy_until: u64,
}

impl Elevator {
    pub fn new(capacity: usize) -> Self {
        Self {
            pos: 0,
            passengers: Default::default(),
            capacity,
            busy_until: 0,
        }
    }

    /// Returns the index of the floor the elevator is currently at or last
    /// passed.
    pub fn floor(&self, time_per_floor: u64) -> usize {
        (self.pos / time_per_floor) as usize
    }
}

#[derive(Debug, Clone)]
pub struct Floor {
    /// people[n] = list of people waiting for elevator n
    /// They enter the elevator in FIFO order (they are polite).
    pub people: Box<[VecDeque<Person>]>,
}

impl Floor {
    pub fn new(elevators: usize) -> Self {
        let people = (0..elevators).map(|_| Default::default()).collect::<Vec<_>>().into();
        Self { people }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.people.len()
    }
}

#[derive(Debug, Clone)]
pub struct Building {
    pub floors: Box<[Floor]>,
    pub elevators: Box<[Elevator]>,
    /// The time it takes to travel between two adjacent floors
    time_per_floor: u64,
    /// Extra time it takes to stop/start and open/close doors.
    time_per_stop: u64,
    /// Previous event time
    prev_time: u64,
    /// Previous request time
    prev_req_time: u64,
}

impl Default for Building {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Default, Debug, Clone)]
pub struct BuildingBuilder {
    time_per_floor: Option<u64>,
    time_per_stop: Option<u64>,
    elevators: Option<usize>,
    floors: Option<usize>,
    elevator_capacity: Option<usize>,
}

macro_rules! impl_builder {
    ($($field:ident: $ty:ty),* $(,)?) => {
        $(
            pub fn $field(mut self, $field: $ty) -> Self {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl BuildingBuilder {
    pub(crate) const DEFAULT_FLOORS: usize = 20;
    pub(crate) const DEFAULT_ELEVATORS: usize = 4;
    pub(crate) const DEFAULT_ELEVATOR_CAPACITY: usize = 8;
    pub(crate) const DEFAULT_TIME_PER_FLOOR: u64 = 500;
    pub(crate) const DEFAULT_TIME_PER_STOP: u64 = 2_000;

    impl_builder! {
        time_per_floor: u64,
        time_per_stop: u64,
        elevators: usize,
        floors: usize,
        elevator_capacity: usize,
    }

    pub fn build(&self) -> Building {
        let elevators = self.elevators.unwrap_or(Self::DEFAULT_FLOORS);
        let floors = self.floors.unwrap_or(Self::DEFAULT_ELEVATORS);
        let time_per_floor = self.time_per_floor.unwrap_or(Self::DEFAULT_TIME_PER_FLOOR);
        let time_per_stop = self.time_per_stop.unwrap_or(Self::DEFAULT_TIME_PER_STOP);
        let elevator_capacity = self.elevator_capacity.unwrap_or(Self::DEFAULT_ELEVATOR_CAPACITY);
        assert!(elevators <= 1_000);
        assert!(floors <= 100_000);
        assert!(time_per_floor <= 1_000_000);
        assert!(time_per_stop <= 1_000_000);
        assert!(elevator_capacity <= 100_000);
        Building {
            floors: (0..floors).map(|_| Floor::new(elevators)).collect::<Vec<_>>().into(),
            elevators: (0..elevators).map(|_| Elevator::new(elevator_capacity)).collect::<Vec<_>>().into(),
            prev_time: 0,
            prev_req_time: 0,
            time_per_floor,
            time_per_stop,
        }
    }
}

impl Building {
    pub fn builder() -> BuildingBuilder {
        Default::default()
    }

    pub fn time_per_floor(&self) -> u64 {
        self.time_per_floor
    }

    pub fn num_floors(&self) -> usize {
        self.floors.len()
    }

    pub fn waiting_at(&self, floor: usize) -> usize {
        self.floors[floor].people.iter().map(|q| q.len()).sum()
    }

    pub fn waiting_for_elevator(&self, floor: usize, elevator: usize) -> usize {
        self.floors[floor].people[elevator].len()
    }

    pub fn prev_time(&self) -> u64 {
        self.prev_time
    }
}

impl Building {
    pub fn run<P: Policy + ?Sized, T: Traffic + ?Sized>(
        &mut self,
        until: u64,
        policy: &mut P,
        decision: &mut Decision,
        traffic: &mut T,
        stats: &mut Stats,
    ) {
        while self.prev_time < until {
            // Calculate the next event that will happen, skip simulation to that event
            let (arriving, arrival_time) = self.next_arrival(&decision.dests);
            let waiting = decision.waits.iter().enumerate().min_by_key(|&(_, &v)| v).unwrap().0;
            let next_request = cmp::max(self.prev_time, traffic.when().saturating_add(self.prev_req_time));
            let events = [arrival_time, decision.waits[waiting], next_request];
            let (next_event, &time) = events.iter().enumerate().min_by_key(|&(_, &v)| v).unwrap();
            assert!(time >= self.prev_time);
            if time > until {
                break;
            }
            trace!(time = time, event = %"Events", ?events);
            self.move_elevators(time, &decision.dests);
            match next_event {
                // Process an elevator arriving at a floor and opening it's doors.
                // Passengers who's floor this is leave the elevator first,
                // then passengers waiting at the floor enter the elevator FIFO.
                0 => {
                    let e = &mut self.elevators[arriving];
                    let floor_idx = e.floor(self.time_per_floor);
                    e.busy_until = time + self.time_per_stop;

                    assert_eq!(e.pos, decision.dests[arriving].unwrap() as u64 * self.time_per_floor);
                    debug!(time = time, event = %"Arrival", elevator = arriving, floor = floor_idx, waiting = ?self.floors[floor_idx].people[arriving]);

                    decision.dests[arriving] = None;
                    e.passengers.retain(|&x| {
                        if x.dest == floor_idx {
                            stats.add(time, &x);
                            debug!(time = time, event = %"Exit", person = ?x);
                            false
                        } else {
                            true
                        }
                    });
                    let space = e.capacity - e.passengers.len();
                    let entering = std::cmp::min(space, self.floors[floor_idx].people[arriving].len());
                    let new = self.floors[floor_idx].people[arriving].drain(0..entering);
                    e.passengers.extend(new.into_iter());
                    debug!(
                        time = time,
                        event = %"Entry",
                        elevator = arriving,
                        count = entering,
                        passengers = ?&e.passengers[(e.passengers.len() - entering)..],
                    );
                    policy.arrival(self, decision, arriving);
                }
                1 => {
                    decision.waits[waiting] = u64::MAX;
                    policy.waited(self, decision, waiting);
                }
                // Process a new person spawning at a source floor and requesting to go to destination floor.
                2 => {
                    let (src, dest) = traffic.next(time);
                    assert!(src != dest);
                    assert!(src < self.floors.len());
                    assert!(dest < self.floors.len());
                    if self.floors[src].len() < MAX_WAITING {
                        let new = Person { src, dest, req_time: time };
                        let assignment = policy.request(self, decision, &new);
                        debug!(time = time, event = %"Request", elevator = assignment, person = ?new);
                        self.floors[src].people[assignment].push_back(new);
                    }
                    self.prev_req_time = time;
                }
                _ => unreachable!(),
            }
            self.prev_time = time;
        }
        self.move_elevators(until, &decision.dests);
        self.prev_time = until;
        stats.tick(until);
    }

    fn move_elevators(&mut self, until: u64, dests: &[Option<usize>]) {
        for (i, e) in self.elevators.iter_mut().enumerate() {
            if let Some(dest) = dests[i] {
                let start_time = self.prev_time.max(e.busy_until);
                if until >= start_time {
                    let delta = until - start_time;
                    let target_pos = dest as u64 * self.time_per_floor;
                    if e.pos < target_pos {
                        e.pos = (e.pos + delta).min(target_pos);
                    } else if e.pos > target_pos {
                        e.pos = e.pos.saturating_sub(delta).max(target_pos);
                    }
                }
            }
        }
    }

    /// Returns the next time an elevator arrival will happen.
    fn next_arrival(&self, dests: &[Option<usize>]) -> (usize, u64) {
        (0..dests.len())
            .map(|e| dests[e].map_or(u64::MAX, |d| self.arrival_time(e, d)))
            .enumerate()
            .min_by_key(|(_, v)| *v)
            .unwrap()
    }

    pub fn arrival_time(&self, elevator: usize, dest_floor: usize) -> u64 {
        let e = &self.elevators[elevator];
        let travel_time = self.travel_time(elevator, dest_floor);
        let start_time = self.prev_time.max(e.busy_until);
        start_time + travel_time
    }

    pub fn travel_time(&self, elevator: usize, dest_floor: usize) -> u64 {
        let e = &self.elevators[elevator];
        let target_pos = dest_floor as u64 * self.time_per_floor;
        e.pos.abs_diff(target_pos)
    }
}

#[cfg(test)]
mod tests {
    // $env:RUST_LOG="elevator=debug"

    use super::*;
    use crate::traffic::Random;
    use std::sync::Once;

    static TRACING: Once = Once::new();

    fn init_tracing() {
        TRACING.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .without_time()
                .with_test_writer()
                .try_init();
        });
    }

    #[test]
    fn simple() {
        init_tracing();

        let mut building = Building::builder().floors(10).elevators(2).build();
        let mut policy = crate::policies::Simple::default();
        let mut traffic = Random::new(10, vec![5.0], vec![5.0], 0.1);
        let mut stats = Stats::new(1_000, 0, 1024);
        let mut decision = Decision::new(building.elevators.len());

        building.run(10_000, &mut policy, &mut decision, &mut traffic, &mut stats);
    }
}
