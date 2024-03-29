use crate::errors::*;

use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

pub type Date = chrono::Date<chrono::Local>;

pub use iso_currency::Currency;

use fs_err as fs;

use std::fmt;

mod bankinfo;
mod companyinfo;
mod euro;
mod exchange;
mod expense;
mod percentage;
mod receipts;
mod record;

pub use self::bankinfo::*;
pub use self::companyinfo::*;
pub use self::euro::*;
pub use self::exchange::*;
pub use self::expense::*;
pub use self::percentage::*;
pub use self::receipts::*;
pub use self::record::*;

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
            brutto: Expense::from_str("5 €").unwrap(),
            netto: Expense::from_str("4 €").unwrap(),
            tax_total: indexmap::indexmap! {
                Percentage::from_str("5%").unwrap() => Euro::from_str("1").unwrap(),
            },
        };
        let r2 = Row {
            date,
            company: "Cuba Corp".to_owned(),
            description: "Crops".to_owned(),
            brutto: Expense::from_str("12.50 €").unwrap(),
            netto: Expense::from_str("10 €").unwrap(),
            tax_total: indexmap::indexmap! {
                Percentage::from_str("25%").unwrap() => Euro::from_str("2.50").unwrap(),
            },
        };
        let r3 = Row {
            date,
            company: "Octopus Inc".to_owned(),
            description: "Ink".to_owned(),
            brutto: Expense::from_str("7 €").unwrap(),
            netto: Expense::from_str("7 €").unwrap(),
            tax_total: indexmap::indexmap! {
                Percentage::from_str("0%").unwrap() => Euro::from_str("0").unwrap(),
            },
        };
        let r4 = Row {
            date,
            company: "Cuba Corp".to_owned(),
            description: "Crops (Moar)".to_owned(),
            brutto: Expense::from_str("25.00 €").unwrap(),
            netto: Expense::from_str("20 €").unwrap(),
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
