use std::collections::VecDeque;
use tracing::{debug, trace};

pub mod policies;
pub mod policy;
pub mod stats;
pub mod traffic;
pub mod traffics;
use crate::policy::{Decision, Policy};
use crate::stats::Stats;
use crate::traffic::Traffic;

const ELEVATOR_CAPACITY: usize = 8;
const TIME_PER_FLOOR: u64 = 500;
const STOP_TIME: u64 = 2_000;

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
    people: Box<[VecDeque<Person>]>,
}

impl Floor {
    pub fn new(num_elevators: usize) -> Self {
        let people = (0..num_elevators).map(|_| Default::default()).collect::<Vec<_>>().into();
        Self { people }
    }
}

#[derive(Debug, Clone)]
pub struct Building {
    floors: Box<[Floor]>,
    pub elevators: Box<[Elevator]>,
    /// The time it takes to travel between two adjacent floors
    time_per_floor: u64,
    /// Extra time it takes to stop/start and open/close doors.
    stop_penalty: u64,
    /// Previous event time
    prev_time: u64,
}

impl Building {
    pub fn new(num_floors: usize, num_elevators: usize) -> Self {
        Self {
            floors: (0..num_floors).map(|_| Floor::new(num_elevators)).collect::<Vec<_>>().into(),
            elevators: (0..num_elevators)
                .map(|_| Elevator::new(ELEVATOR_CAPACITY))
                .collect::<Vec<_>>()
                .into(),
            prev_time: 0,
            time_per_floor: TIME_PER_FLOOR,
            stop_penalty: STOP_TIME,
        }
    }

    pub fn time_per_floor(&self) -> u64 {
        self.time_per_floor
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
    // pub fn run<P: Policy, T: Traffic>(&mut self, until: u64, policy: &mut P,
    // decision: &mut Decision, traffic: &mut T, stats: &mut Stats) {
    pub fn run(&mut self, until: u64, policy: &mut dyn Policy, decision: &mut Decision, traffic: &mut dyn Traffic, stats: &mut Stats) {
        while self.prev_time < until {
            // Calculate the next event that will happen, skip simulation to that event
            let (arriving, arrival_time) = self.min_arrival(&decision.dests);
            let waiting = decision.waits.iter().enumerate().min_by_key(|&(_, &v)| v).unwrap().0;
            let next_request = traffic.peek().req_time;
            assert!(next_request >= self.prev_time);
            let events = [arrival_time, decision.waits[waiting], next_request];
            let next_event = events.iter().enumerate().min_by_key(|&(_, &v)| v).unwrap();
            let time = *next_event.1;
            assert!(time >= self.prev_time);
            if time > until {
                break;
            }
            trace!(time = time, event = %"Events", ?events);
            self.move_elevators(time, &decision.dests);
            match next_event.0 {
                // Process an elevator arriving at a floor and opening it's doors.
                // Passengers who's floor this is leave the elevator first,
                // then passengers waiting at the floor enter the elevator FIFO.
                0 => {
                    let e = &mut self.elevators[arriving];
                    let floor_idx = e.floor(self.time_per_floor);
                    e.busy_until = time + self.stop_penalty;

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
                    let new = traffic.pop();
                    assert!(new.src != new.dest);
                    let assignment = policy.request(self, decision, &new);
                    debug!(time = time, event = %"Request", elevator = assignment, person = ?new);
                    self.floors[new.src].people[assignment].push_back(new);
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
    fn min_arrival(&self, dests: &[Option<usize>]) -> (usize, u64) {
        (0..dests.len())
            .map(|i| self.arrival(i, dests))
            .enumerate()
            .min_by_key(|(_, v)| *v)
            .unwrap()
    }

    fn arrival(&self, index: usize, dests: &[Option<usize>]) -> u64 {
        if let Some(dest) = dests[index] {
            let e = &self.elevators[index];
            let target_pos = dest as u64 * self.time_per_floor;
            let travel_time = e.pos.abs_diff(target_pos);
            let start_time = self.prev_time.max(e.busy_until);
            trace!(
                time = self.prev_time,
                event = %"Will Arrive",
                elevator = index,
                pos = e.pos,
                dest = dest,
            );
            start_time + travel_time
        } else {
            u64::MAX
        }
    }
}

#[cfg(test)]
mod tests {
    // $env:RUST_LOG="elevator=debug"

    use super::*;
    use crate::policy::Simple;
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

        let mut building = Building::new(10, 2);
        let mut policy = Simple::default();
        let mut traffic = Random::new(10, 60_000);
        let mut stats = Stats::new(1_000);
        let mut decision = Decision::new(building.elevators.len());

        building.run(1000_000, &mut policy, &mut decision, &mut traffic, &mut stats);
    }
}
