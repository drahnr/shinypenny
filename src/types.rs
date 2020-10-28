use crate::errors::*;

use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
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

use float_cmp::ApproxEq;

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

/// A record in the input csv data.
#[derive(Debug, Deserialize)]
pub(crate) struct Record {
    pub(crate) date: chrono::NaiveDate,
    pub(crate) description: String,
    pub(crate) company: String,
    pub(crate) netto: Euro,
    pub(crate) tax: Percentage,
    pub(crate) brutto: Euro,
    #[serde(alias = "receipt")]
    pub(crate) path: PathBuf,
}

/// A table row to be displayed in the pdf table.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub(crate) date: Date,
    pub(crate) company: String,
    pub(crate) description: String,
    pub(crate) brutto: Euro,
    pub(crate) netto: Euro,
    pub(crate) tax_total: indexmap::IndexMap<Percentage, Euro>,
}

impl Row {
    #[allow(unused)]
    pub(crate) fn iter(&self) -> RowCellIter {
        RowCellIter::new(&self)
    }
}

impl<'a> IntoIterator for &'a Row {
    type Item = String;
    type IntoIter = RowCellIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RowCellIter::<'a>::new(self)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowCellIter<'a> {
    idx: usize,
    row: &'a Row,
}

impl<'a> RowCellIter<'a> {
    pub(crate) fn new(row: &'a Row) -> Self {
        Self { row, idx: 0usize }
    }
}

impl<'a> Iterator for RowCellIter<'a> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        let tax_classes = self.row.tax_total.len();
        let val = match self.idx {
            0 => self.row.date.format("%Y-%m-%d").to_string(),
            1 => self.row.description.clone(),
            2 => self.row.company.clone(),
            3 => self.row.netto.to_string(),
            x if x < (4 + tax_classes) => self
                .row
                .tax_total
                .get_index(x.saturating_sub(4))
                .expect("Bounds are evaled outside. qed")
                .1
                .to_string(),
            x if x == (4 + tax_classes) => self.row.brutto.to_string(),
            _ => return None,
        };
        self.idx += 1;
        Some(val)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Totals {
    pub(crate) brutto: Euro,
    pub(crate) netto: Euro,
    pub(crate) tax_total: indexmap::IndexMap<Percentage, Euro>,
}

use itertools::Itertools;

impl Totals {
    pub fn add(&mut self, other: &Row) {
        self.brutto += other.brutto;
        self.netto += other.netto;

        for (percent, absolute) in other.tax_total.iter().sorted_by(|x, y| x.0.cmp(&y.0)) {
            let val = self.tax_total.entry(*percent).or_default();
            *val += *absolute;
        }
    }
}

impl<'a> IntoIterator for &'a Totals {
    type Item = String;
    type IntoIter = TotalCellIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TotalCellIter::<'a>::new(self)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TotalCellIter<'a> {
    idx: usize,
    total: &'a Totals,
}

impl<'a> TotalCellIter<'a> {
    pub(crate) fn new(total: &'a Totals) -> Self {
        Self { total, idx: 0usize }
    }
}

impl<'a> Iterator for TotalCellIter<'a> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        let tax_classes = self.total.tax_total.len();
        let val = match self.idx {
            0 | 1 | 2 => "".to_owned(),
            3 => format!("€ {}", self.total.netto),
            x if x < (4 + tax_classes) => self.total.tax_total[x.saturating_sub(4)].to_string(),
            x if x == 4 + tax_classes => self.total.brutto.to_string(),
            _ => return None,
        };
        self.idx += 1;
        Some(val)
    }
}

use fints_institute_db::get_bank_by_bank_code;
use fints_institute_db::Bank;
use fs_err as fs;
use iban::Iban;

#[derive(Clone)]
pub(crate) struct CompanyInfo {
    pub(crate) image: Option<printpdf::image::DynamicImage>,
    pub(crate) name: String,
    pub(crate) address: String,
}

impl CompanyInfo {
    pub(crate) fn new(name: &str, address: &str, image_path: Option<PathBuf>) -> Result<Self> {
        let image = if let Some(image_path) = image_path {
            log::trace!("Loading company image from {}", image_path.display());
            let file = fs::OpenOptions::new().read(true).open(&image_path)?;
            let reader = std::io::BufReader::with_capacity(2048, file);
            let reader = printpdf::image::io::Reader::new(reader).with_guessed_format()?;
            log::trace!("Determined company image format: {:?}", reader.format());
            let image = reader.decode()?;
            Some(image)
        } else {
            None
        };
        let name = name.to_owned();
        let address = address.to_owned();
        Ok(Self {
            image,
            name,
            address,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct BankInfo {
    /// Full name of the bank account owner.
    pub(crate) name: String,
    /// IBAN contains all info about the bank, so that's all needed
    pub(crate) iban: Iban,
    /// Your bank institute information.
    bank: Option<Bank>,
}

impl BankInfo {
    pub(crate) fn new(name: impl AsRef<str>, iban: Iban) -> Result<Self> {
        // let iban = Iban::from_str(iban.as_ref())?;
        let name = name.as_ref().to_owned();
        let bank_indentifier = iban
            .bank_identifier()
            .ok_or_else(|| eyre!("Failed to extract bank identifier from IBAN"))?;
        let bank = get_bank_by_bank_code(bank_indentifier);
        Ok(Self { name, iban, bank })
    }

    /// If the institute is not in the db, should return `None`.
    pub fn institute(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.institute.clone())
    }

    #[allow(unused)]
    pub fn bank_code(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.bank_code.clone())
    }

    #[allow(unused)]
    pub fn location(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.location.clone())
    }
    pub fn bic(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.bic.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpy() {
        let netto = Euro(32.99f64);
        let tax = (netto * Percentage::from_str("5%").unwrap());
        assert!(dbg!(tax).approx_eq(Euro(1.65), EPSILON));
        let brutto = netto + tax;
        assert!((Euro(32.99f64) + Euro(1.65)).approx_eq(brutto, EPSILON));
    }
}
