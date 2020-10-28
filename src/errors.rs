pub use ::color_eyre::eyre::{bail, eyre, Error, Report, WrapErr};

pub type Result<T> = ::color_eyre::eyre::Result<T, ::color_eyre::eyre::Error>;
