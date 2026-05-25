pub mod chatgpt;
pub mod gemini;
pub mod gemini2;
pub mod scan;
pub mod simple;
pub mod bogo;

pub use chatgpt::OpenAi;
pub use gemini::Gemini;
pub use gemini2::Gemini2;
pub use scan::Scan;
pub use simple::Simple;
pub use bogo::Bogo;
