use crate::errors::*;
use crate::types::*;

use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// A value followed by a 3 digit ISO 4217 character code or a unicode currency symbol.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct Expense(pub f64, pub Currency, pub Option<ExchangeRate>);

impl Default for Expense {
    fn default() -> Self {
        Self(f64::default(), Currency::EUR, None)
    }
}

impl Expense {
    pub fn amount(&self) -> f64 {
        self.0
    }
    pub fn currency(&self) -> Currency {
        self.1
    }
    pub fn exchange_rate(&self) -> Option<ExchangeRate> {
        self.2
    }

    pub fn set_exchange_rate(&mut self, rate: ExchangeRate) {
        self.2 = Some(rate);
    }

    pub fn as_euro(&self) -> Euro {
        if self.currency() == Currency::EUR {
            return Euro(self.amount())
        }
        self.exchange_rate().map(|rate| {
            Euro(rate * self.amount())
        }).expect("Assumes an exchange rate was set.")
    }
}

fn silly_decimals(fragment: &str) -> Result<f64> {
    let val = fragment
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
    Ok(f64::from_str(val.as_str())?)
}

impl FromStr for Expense {
    type Err = Error;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        const MSG: &'static str = "Is not an acceptable euro value";
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r#"^\s*([0-9]+(?:[,.][0-9]*)?)\s*([¥£€$]|[A-Z]{3})?\s*(?:@\s*([0-9]+(?:[,.][0-9]*)?)\s*)?$"#).unwrap();
        };
        let captures = if let Some(captures) = RE.captures(dbg!(s)) {
            captures
        } else {
            bail!("Regex is not a match")
        };
        let amount: f64 = if let Some(amount) = captures.get(1) {
            silly_decimals(amount.as_str())?
        } else {
            bail!("First capture is a decimal number")
        };
        log::trace!("Parsed amount: {}", amount);
        let currency = if let Some(currency) = captures.get(2) {
            match currency.as_str() {
                "$" => Currency::USD,
                "€" => Currency::EUR,
                "¥" => Currency::JPY,
                "£" => Currency::GBP,
                three_letter_code => {
                    if let Some(currency) = Currency::from_code(three_letter_code) {
                        currency
                    } else {
                        bail!("Unknown currency code: {}", three_letter_code)
                    }
                }
            }
        } else {
            Currency::EUR
        };
        log::trace!("Parsed currency: {}", &currency);

        let rate = if let Some(rate) = captures.get(3) {
            if currency == Currency::EUR {
                bail!("Can't have eur AND a rate for converting to euro");
            }
            let rate = silly_decimals(rate.as_str())?;
            log::trace!("Parsed rate: {}", rate);
            Some(rate)
        } else {
            log::trace!("Parsed rate: w/o");
            None
        };
        Ok(Expense(amount, currency, rate))
    }
}

struct CurrencyVisitor;

impl<'de> serde::de::Visitor<'de> for CurrencyVisitor {
    type Value = Expense;

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

impl<'de> serde::de::Deserialize<'de> for Expense {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_str(CurrencyVisitor)
    }
}

impl fmt::Display for Expense {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>.02}", self.0)?;
        if self.currency() != Currency::EUR {
            write!(f, " {}", self.1.code())?;
            if let Some(rate) = self.2 {
                writeln!(f, " @ {} :  {}", rate, self.as_euro())?;
                return Ok(())
            }
        }
        writeln!(f, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn symbol() {
        assert_matches!(
            Expense::from_str("7.50 €"),
            Ok(Expense(amount, Currency::EUR, _)) => {
                assert_eq!((amount * 100.) as i32, 750);
            }
        );
        assert_matches!(Expense::from_str("7,0 £"),
        Ok(Expense(amount, Currency::GBP, _)) => {
            assert_eq!((amount * 100.) as i32, 700);
        });
        assert_matches!(
            Expense::from_str("0,50 $"),
            Ok(Expense(amount, Currency::USD, _)) => {
                assert_eq!((amount * 100.) as i32, 50);
            }
        );
        assert_matches!(
            Expense::from_str("11.22 ¥"),
            Ok(Expense(amount, Currency::JPY, _)) => {
                assert_eq!((amount * 100.) as i32, 1122);
            }
        );
    }

    #[test]
    fn iso4217_code() {
        assert_matches!(
            Expense::from_str("7.50 EUR"),
            Ok(Expense(amount, Currency::EUR, _)) => {
                assert_eq!((amount * 100.) as i32, 750);
            }
        );
        assert_matches!(
            Expense::from_str("200,01 USD"),
            Ok(Expense(amount, Currency::USD, _)) => {
                assert_eq!((amount * 100.) as i32, 20001);
            }
        );
        assert_matches!(
            Expense::from_str("500 GBP"),
            Ok(Expense(amount, Currency::GBP, _)) => {
                assert_eq!((amount * 100.) as i32, 50000);
            }
        );
        assert_matches!(
            Expense::from_str("0,50 JPY"),
            Ok(Expense(amount, Currency::JPY, _)) => {
                assert_eq!((amount * 100.) as i32, 50);
            }
        );
    }

    #[test]
    fn euro_is_base() {
        assert_matches!(
            Expense::from_str("999.99"),
            Ok(Expense(amount, Currency::EUR, _)) => {
                assert_eq!((amount * 100.) as i32, 99999);
            }
        );
    }
}
