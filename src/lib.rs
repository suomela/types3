//! Type accumulation curves.
//!
//! See [driver] for the main entry point.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod calc_avg;
mod calc_point;
mod calculation;
pub mod categories;
mod counter;
pub mod driver;
pub mod errors;
mod information;
pub mod input;
pub mod output;
mod parallelism;
mod samples;
mod shuffle;
mod subsets;
