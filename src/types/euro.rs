//! Base and target currency.
use crate::errors::*;
use crate::types::*;
use float_cmp::ApproxEq;
use lazy_static::lazy_static;
use regex::Regex;

use serde::Serialize;

use std::str::FromStr;

pub use iso_currency::Currency;

#[derive(Serialize, Clone, Copy, Default)]
pub struct Euro(pub f64);

impl FromStr for Euro {
    type Err = Error;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        const MSG: &'static str = "Is not an acceptable euro value";
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"^\s*([0-9]+(?:[,.][0-9]*)?)\s*(?:€|(?:EUR))?\s*$"#).unwrap();
        };
        let captures = if let Some(captures) = RE.captures(s) {
            captures
        } else {
            bail!(MSG)
        };
        let amount: f64 = if let Some(amount) = captures.get(1) {
            let amount = amount
                .as_str()
                .chars()
                .scan(true, |first: &mut bool, c: char| {
                    if c == ',' && *first {
                        *first = false;
                        Some('.')
                    } else {
                        Some(c)
                    }
                })
                .collect::<String>();
            f64::from_str(amount.as_str())?
        } else {
            bail!(MSG)
        };
        Ok(Euro(amount))
    }
}

struct EuroVisitor;

impl<'de> serde::de::Visitor<'de> for EuroVisitor {
    type Value = Euro;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Expense denotion, amount with or without currency 3 letter or symbol suffix"
        )
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let s = s.trim();
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
