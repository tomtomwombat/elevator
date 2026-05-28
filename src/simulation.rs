use crate::{Building, Decision, Policy, Stats, Traffic, controls::Controls};

pub struct Simulation {
    pub building: Building,
    decision: Decision,
    policy: Box<dyn Policy>,
    pub traffic: Box<dyn Traffic>,
    pub stats: Stats,
}

impl Simulation {
    pub fn new(building: Building, policy: Box<dyn Policy>, traffic: Box<dyn Traffic>, c: &Controls) -> Self {
        let decision = Decision::new(building.elevators.len());
        let stats = Stats::new(c.stats_window(), 0);
        Self {
            building,
            decision,
            policy,
            traffic,
            stats,
        }
    }

    pub fn tick(&mut self, until: u64) {
        self.building.run(
            until,
            self.policy.as_mut(),
            &mut self.decision,
            self.traffic.as_mut(),
            &mut self.stats,
        );
    }
}
