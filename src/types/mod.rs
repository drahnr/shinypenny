use crate::errors::*;

use float_cmp::ApproxEq;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::str::FromStr;

pub type Date = chrono::Date<chrono::Local>;

use fs_err as fs;

use std::fmt;

mod bankinfo;
mod companyinfo;
mod euro;
mod receipts;
mod record;

pub(crate) use self::bankinfo::*;
pub(crate) use self::companyinfo::*;
pub(crate) use self::euro::*;
pub(crate) use self::receipts::*;
pub(crate) use self::record::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpy() {
        let netto = Euro(32.99f64);
        let tax = netto * Percentage::from_str("5%").unwrap();
        assert!(dbg!(tax).approx_eq(Euro(1.65), EPSILON));
        let brutto = netto + tax;
        assert!((Euro(32.99f64) + Euro(1.65)).approx_eq(brutto, EPSILON));
    }

    #[test]
    fn total_acc() {
        let date = chrono::Local::today();
        let r1 = Row {
            date,
            company: "Dodo GmbH".to_owned(),
            description: "Birdy".to_owned(),
            brutto: Euro::from_str("5 €").unwrap(),
            netto: Euro::from_str("4 €").unwrap(),
            tax_total: indexmap::indexmap! {
                Percentage::from_str("5%").unwrap() => Euro::from_str("1").unwrap(),
            },
        };
        let r2 = Row {
            date,
            company: "Cuba Corp".to_owned(),
            description: "Crops".to_owned(),
            brutto: Euro::from_str("12.50 €").unwrap(),
            netto: Euro::from_str("10 €").unwrap(),
            tax_total: indexmap::indexmap! {
                Percentage::from_str("25%").unwrap() => Euro::from_str("2.50").unwrap(),
            },
        };
        let r3 = Row {
            date,
            company: "Octopus Inc".to_owned(),
            description: "Ink".to_owned(),
            brutto: Euro::from_str("7 €").unwrap(),
            netto: Euro::from_str("7 €").unwrap(),
            tax_total: indexmap::indexmap! {
                Percentage::from_str("0%").unwrap() => Euro::from_str("0").unwrap(),
            },
        };
        let r4 = Row {
            date,
            company: "Cuba Corp".to_owned(),
            description: "Crops (Moar)".to_owned(),
            brutto: Euro::from_str("25.00 €").unwrap(),
            netto: Euro::from_str("20 €").unwrap(),
            tax_total: indexmap::indexmap! {
                Percentage::from_str("25%").unwrap() => Euro::from_str("5.0").unwrap(),
            },
        };

        let mut total = Totals::default();

        total.add(&r1);
        total.add(&r2);
        total.add(&r3);
        total.add(&r4);

        let mut iter = total.into_iter();

        // assert_eq!(iter.next().as_ref(), Some(&date.format("%Y-%m-%d").to_string()));
        assert_eq!(iter.next(), Some("".to_owned()));
        assert_eq!(iter.next(), Some("".to_owned()));
        assert_eq!(iter.next(), Some("".to_owned()));
        assert_eq!(iter.next(), Some("€ 41.00".to_owned()));
        assert_eq!(iter.next(), Some("0.00".to_owned()));
        assert_eq!(iter.next(), Some("1.00".to_owned()));
        assert_eq!(iter.next(), Some("7.50".to_owned()));
        assert_eq!(iter.next(), Some("49.50".to_owned()));
        assert_eq!(iter.next(), None);
    }
}
