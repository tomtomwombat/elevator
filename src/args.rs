use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// number of floors
    #[arg(short, long, default_value_t = 20)]
    pub floors: usize,
    /// number of elevators
    #[arg(short, long, default_value_t = 4)]
    pub elevators: usize,
}