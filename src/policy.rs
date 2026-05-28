use crate::{Building, Person};

pub struct Decision {
    // Wait commands set by the policy
    pub waits: Box<[u64]>,
    // Direction commands set by the policy
    pub dests: Box<[Option<usize>]>,
}

impl Decision {
    pub fn new(elevators: usize) -> Self {
        Self {
            waits: vec![u64::MAX; elevators].into(),
            dests: vec![None; elevators].into(),
        }
    }
}

pub trait Policy {
    fn new(_building: &Building) -> Self
    where
        Self: Sized;
    /// Notifies the policy that a person has requested to go to another floor.
    fn request(&mut self, _building: &Building, decision: &mut Decision, _person: &Person) -> usize;
    /// Notifies that an elevator has reached it's destination.
    /// This is called after passengers have left and entered the elevator.
    fn arrival(&mut self, _building: &Building, _decision: &mut Decision, _elevator: usize) {}
    /// Notifies the policy that some time has passed.
    fn waited(&mut self, _building: &Building, _decision: &mut Decision, _elevator: usize) {}
}
