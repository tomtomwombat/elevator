pub mod bogo;
pub mod chatgpt;
pub mod gemini;
pub mod gemini2;
pub mod greedy_utilitarian;
pub mod scan;
pub mod simple;

pub use bogo::Bogo;
pub use chatgpt::OpenAi;
pub use gemini::Gemini;
pub use gemini2::Gemini2;
pub use greedy_utilitarian::GreedyUtilitarian;
pub use scan::Scan;
pub use simple::Simple;
