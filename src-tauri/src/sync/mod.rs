pub mod diff;
pub mod engine;
pub mod error;

pub use diff::{apply_diff, DiffOutcome, DiffPlan};
pub use engine::{Clock, SyncOutcome, Syncer, SystemClock};
pub use error::SyncError;

#[cfg(test)]
mod integration_tests;
