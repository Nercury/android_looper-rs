//! Looper Error and Result.
//!
//!

use std::result;

#[derive(Copy, Clone, Debug)]
pub enum Error {
    PrepareLooperFailed,
}

pub type Result<T> = result::Result<T, Error>;
