use crate::{Building, BuildingBuilder};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// number of floors
    #[arg(short, long, default_value_t = BuildingBuilder::DEFAULT_FLOORS)]
    pub floors: usize,
    /// number of elevators
    #[arg(short, long, default_value_t = BuildingBuilder::DEFAULT_ELEVATORS)]
    pub elevators: usize,
    /// max number of people in an elevator
    #[arg(long, default_value_t = BuildingBuilder::DEFAULT_ELEVATOR_CAPACITY)]
    pub elevator_capacity: usize,
    /// time it takes to travel between floors (ms)
    #[arg(long, default_value_t = BuildingBuilder::DEFAULT_TIME_PER_FLOOR)]
    pub time_per_floor: u64,
    /// time it takes to stop at a floor (ms)
    #[arg(long, default_value_t = BuildingBuilder::DEFAULT_TIME_PER_STOP)]
    pub time_per_stop: u64,
}

impl Args {
    pub fn building(&self) -> Building {
        Building::builder()
            .floors(self.floors)
            .elevators(self.elevators)
            .elevator_capacity(self.elevator_capacity)
            .time_per_floor(self.time_per_floor)
            .time_per_stop(self.time_per_stop)
            .build()
    }
}
