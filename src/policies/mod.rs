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

use crate::policy::Policy;

macro_rules! register_policies {
    ($($struct:ty),* $(,)?) => {
        pub fn new(name: &str, b: &crate::Building) -> Box<dyn crate::Policy + 'static> {
            match name {
                $(stringify!($struct) => Box::new(<$struct>::new(b)),)*
                _ => panic!("Unknown policy: {}", name),
            }
        }

        pub const ALL: &[&str] = &[$(stringify!($struct)),*];
    };
}

register_policies! {
    Bogo, OpenAi, Gemini, Gemini2, GreedyUtilitarian, Scan, Simple
}

pub const DEFAULT: &[&str] = &["GreedyUtilitarian", "Gemini2", "OpenAi"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_from_str() {
        let b = Default::default();
        for p in crate::policies::ALL.iter() {
            let _ = new(p, &b);
        }
    }
}
