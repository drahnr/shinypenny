use docopt::Docopt;
use serde::Deserialize;
use serde::Serialize;
use log::{warn,error,debug,trace};

use lazy_static::lazy_static;
use regex::Regex;

mod errors;

pub use errors::*;

mod pdf;


const USAGE: &'static str = r#"
shinypenny

Usage:
  shinypenny [--title=<title>] [--learning-budget] <file>..
  shinypenny --csv <csv>
  shinypenny --version

Options:
  --version            Show version.
  -h --help            Show this screen.
  --learning-budget    Deduct from learning budget.
"#;

#[derive(Debug, Serialize, Deserialize)]
struct Args {
    flag_version: bool,
    arg_file: Vec<std::path::PathBuf>,
    flag_label: String,
    flag_title: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct Euro(f64);


use std::str::FromStr;
use std::convert::AsRef;

#[derive(Debug, Serialize, Deserialize)]
struct Percentage(f64);

impl FromStr for Percentage {
    type Err = ::anyhow::Error;
    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        lazy_static! {
            static ref W_PERECENT: Regex = Regex::new(r#"^\s*([0-9]+(?:[,.][0-9]+))?\s*%\s*$"#);
        };
        lazy_static! {
            static ref WO_PERECENT: Regex = Regex::new(r#"^\s*([0-9]+(?:[,.][0-9]+))?\s*$"#);
        };
        let w = W_PERECENT.captures(s);
        let wo = WO_PERECENT.captures(s);
        if w.is_some() && !wo.is_some() {
            let val = f64::from_str(s)?;
            Ok(Percentage(val))
        } else if !w.is_some() && wo.is_some() {
            let val = f64::from_str(s)?;
            // heuristic!
            if val > 1.0 {
                Ok(Percentage(val))
            } else {
                Ok(Percentage(val))
            }
        } else {
            bail!("Is not an acceptable percentage value")
        }
    }
}

use core::ops::{Mul, Add, AddAssign, Sub};

impl Mul<Percentage> for Euro {
    type Output = Self;
    fn mul(self, rhs: Percentage) -> Self::Output {
        Euro(self.0 * rhs.0)
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

use core::cmp::{PartialOrd, Ordering};

impl PartialEq<Euro> for Euro {
    fn eq(&self, other: &Self) -> bool {
        f64::abs(self.0 - other.0) < EPSILON
    }
}

impl PartialOrd<Euro> for Euro {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

#[derive(Debug, Deserialize)]
struct Record {
    brutto: Euro,
    percentage: Percentage,
    netto: Euro,
}

/// DIN A4 in mm dimensions (HxW)
const DIN_A4: (f32, f32) = (297_f32, 210_f32);

use printpdf::*;


use lopdf::Document;


use iban::Iban;

#[derive(Debug)]
pub(crate) struct BankInfo {
    /// Full name of the bank account owner.
    name: String,
    /// IBAN contains all info about the bank, so that's all needed
    iban: Iban,
}

#[derive(Debug, Clone)]
pub(crate) struct Row {
    brutto: f64,
    netto: f64,
    tax_total: f64,
    tax_percentage: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct Totals {
    brutto: f64,
    netto: f64,
    tax_total: f64,
}

impl Totals {
    fn add(&mut self, other: Totals) {
        self.brutto += other.brutto;
        self.netto += other.netto;
        self.tax_total += other.tax_total;
    }
}

const EPSILON: f64 = 0.004; // don't care about less than half a cent up or down

fn create_pdf(records: &[Record]) -> Result<Document> {

    let mut documents = Vec::with_capacity(records.len() + 1);

    let mut rows = Vec::with_capacity(records.len());
    let mut totals = Row {
        brutto: 0.0f64,
        netto: 0.0f64,
        tax_total: 0.0f64,
        tax_percentage: 0.0f64,
    };

    for record in records {
        let document = Document::load_from(record.path)?;
        documents.push(document);

        let brutto = record.brutto;
        let netto = record.netto;
        let percentage = record.percentage;
        if brutto < netto {
            bail!("For expenses, brutto must be larger than netto. Netto includes taxes addition/deduction, brutto does not.");

        }
        let delta = netto - brutto;

        if delta.percentage.approx_cmp(netto * percentage, EPSILON) {
            bail!("The percentage does not match the delta between brutton and netto");
        }

        let row = Row {
            brutto,
            netto,
            tax_total: delta,
            tax_percentage: percentage,
        };

        totals.add(&row);
        rows.push(row);
    }


    let bankinfo = BankInfo {
        name: "Bernhard Schuster".to_owned(),
        iban: "DE24....32606283476239".parse().unwrap(),
    };

    documents.insert(0, pdf::tabular(bankinfo, &rows, totals)?);
    
    let x = pdf::combine(&documents)?;

    Ok(x)
}

fn run() -> Result<()> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());


    if args.flag_version {
        println!("shinypenny {}", env!("CARGO_PKG_VERSION"));
        return Ok(())
    }

    let mut data = Vec::with_capacity(256);
    let path = unimplemented!("...");
    let file = std::fs::OpenOptions::new().read(true).write(false).truncate(false).open(path);
    let mut buffered = std::io::BufReader::with_capacity(4096, file);
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .flexible(true)
        .from_reader(buffered);

    let mut first_valid_record = false;
    for rec in rdr.records() {
        let rec = rec.map_err(|_e| anyhow!("Failed to parse csv line"))?;

        rec.deserialize::<Record>(None)
            .map_err(|_e| anyhow!("Failed to parse record"))
            .unwrap_or_else(|e| {
                warn!("Failed to convert {:?}", e);
                ()
            });

    }
    Ok(())
}

fn main() {
    run().unwrap();
}
