use crate::policies;
use crate::{Building, BuildingBuilder};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Number of floors
    #[arg(short, long, default_value_t = BuildingBuilder::DEFAULT_FLOORS)]
    pub floors: usize,
    /// Number of elevators
    #[arg(short, long, default_value_t = BuildingBuilder::DEFAULT_ELEVATORS)]
    pub elevators: usize,
    /// Max number of people in an elevator
    #[arg(long, default_value_t = BuildingBuilder::DEFAULT_ELEVATOR_CAPACITY)]
    pub elevator_capacity: usize,
    /// Time it takes to travel between floors (ms)
    #[arg(long, default_value_t = BuildingBuilder::DEFAULT_TIME_PER_FLOOR)]
    pub time_per_floor: u64,
    /// Time it takes to stop at a floor (ms)
    #[arg(long, default_value_t = BuildingBuilder::DEFAULT_TIME_PER_STOP)]
    pub time_per_stop: u64,
    /// List of polices that control the building's elevators. Each policy gets
    /// its own building.
    #[arg(
        short, long, num_args = 1..,
        value_parser = clap::builder::PossibleValuesParser::new(policies::ALL),
        default_values_t = policies::DEFAULT.iter().map(|s| String::from(*s)),
    )]
    pub policies: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        // Automatically dissects entire Clap configuration
        Args::command().debug_assert();
    }
}
