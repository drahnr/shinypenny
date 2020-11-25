use crate::errors::*;

use float_cmp::ApproxEq;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::str::FromStr;

pub type Date = chrono::Date<chrono::Local>;

#[derive(Serialize, Clone, Copy, Default)]
pub(crate) struct Euro(pub f64);

impl Euro {
    pub(crate) fn floor_whole_cents(self) -> Self {
        Euro((self.0 * 100.).floor() / 100.)
    }
    pub(crate) fn ceil_whole_cents(self) -> Self {
        Euro((self.0 * 100.).ceil() / 100.)
    }
}

impl FromStr for Euro {
    type Err = Error;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        const MSG: &'static str = "Is not an acceptable euro value";
        lazy_static! {
            static ref RE: Regex = Regex::new(r#"^\s*([0-9]+(?:[,.][0-9]+)?)\s*€?\s*$"#).unwrap();
        };
        let cap = if let Some(cap) = RE.captures(s) {
            cap
        } else {
            bail!(MSG)
        };
        if let Some(val) = cap.get(1) {
            let val = f64::from_str(val.as_str())?;
            Ok(Euro(val))
        } else {
            bail!(MSG)
        }
    }
}

struct EuroVisitor;

impl<'de> serde::de::Visitor<'de> for EuroVisitor {
    type Value = Euro;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Euros denotion, with or withou € suffix")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(s).map_err(|e| serde::de::Error::custom(format!(": {}", e)))
    }
}

impl<'de> serde::de::Deserialize<'de> for Euro {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_str(EuroVisitor)
    }
}

/// Contains the percentage in a fraction in the range of `0..1` multiplied by `1e6`
/// yielding sufficient accuracy.
#[derive(Serialize, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub(crate) struct Percentage(pub u64);

impl FromStr for Percentage {
    type Err = Error;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        const MSG: &'static str = "Is not an acceptable percentage value";
        lazy_static! {
            static ref RE: Regex = Regex::new(r#"^\s*([0-9]+(?:[,.][0-9]+)?)\s*(%)?\s*$"#).unwrap();
        };
        let cap = if let Some(cap) = RE.captures(s) {
            cap
        } else {
            bail!(MSG)
        };
        if let Some(val) = cap.get(1) {
            let val = f64::from_str(val.as_str())?;
            let val = if cap.get(2).is_some() {
                val / 100.
            } else if val > 1.0 {
                // if it's greater than one, it must be in percent points
                // taxations over 100% are very uncommon...
                val / 100.
            } else {
                val
            };
            Ok(Percentage((val * 1e6) as u64))
        } else {
            bail!(MSG)
        }
    }
}

impl std::cmp::PartialOrd for Percentage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<'a> std::cmp::Ord for Percentage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

struct PercentageVisitor;

impl<'de> serde::de::Visitor<'de> for PercentageVisitor {
    type Value = Percentage;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "A number")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_str(s).map_err(|e| serde::de::Error::custom(format!(": {}", e)))
    }
}

impl<'de> serde::de::Deserialize<'de> for Percentage {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_str(PercentageVisitor)
    }
}

use core::ops::{Add, AddAssign, Mul, Sub};

impl Mul<Percentage> for Euro {
    type Output = Self;
    fn mul(self, rhs: Percentage) -> Self::Output {
        Euro(self.0 * rhs.0 as f64 / 1e6)
    }
}

impl Add<Euro> for Euro {
    type Output = Self;
    fn add(self, rhs: Euro) -> Self::Output {
        Euro(self.0 + rhs.0)
    }
}

impl AddAssign<Euro> for Euro {
    fn add_assign(&mut self, rhs: Euro) {
        self.0 += rhs.0
    }
}

impl Sub<Euro> for Euro {
    type Output = Self;
    fn sub(self, rhs: Euro) -> Self::Output {
        Euro(self.0 - rhs.0)
    }
}

use core::cmp::{Ordering, PartialOrd};

impl PartialEq<Euro> for Euro {
    fn eq(&self, other: &Self) -> bool {
        f64::abs(self.0 - other.0) < EPSILON
    }
}

impl ApproxEq for Euro {
    type Margin = f64;
    fn approx_eq<M: Into<Self::Margin>>(self, other: Self, margin: M) -> bool {
        f64::abs(self.0 - other.0) <= margin.into()
    }
}

pub const EPSILON: f64 = 0.005; // don't care about less than half a cent up or down

impl PartialOrd<Euro> for Euro {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

use std::fmt;

impl fmt::Display for Euro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>.02}", self.0)
    }
}

impl fmt::Debug for Euro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>.08}", self.0)
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>.02}", self.0 as f64 * (100f64 / 1e6))
    }
}

impl fmt::Debug for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>.08}", self.0 as f64 * (100f64 / 1e6))
    }
}
