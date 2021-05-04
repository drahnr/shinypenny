use crate::types::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type ExchangeRate = f64;

/// Exchangerate from currency to EUROs.
#[derive(Default)]
struct ExchangeBuro {
    cache: Arc<Mutex<HashMap<Date, HashMap<Currency, f64>>>>,
}

impl ExchangeBuro {
    /// Obtain a exchange rate for a currency at a specific date.
    fn exchange_rate_by_date(&self, when: Date, currency: Currency) -> Option<f64> {
        self.cache
            .lock()
            .unwrap()
            .entry(when)
            .or_insert_with(|| {
                let when_str = when.format("%Y-%m-%d");
                log::debug!("Querying exchange rate for {}", &when_str);
                #[derive(Deserialize, Debug)]
                struct Helper {
                    base: String,
                    rates: HashMap<Currency, ExchangeRate>,
                }

                let per_data: Helper = reqwest::blocking::get(format!(
                    "https://api.openrates.io/{}?base=EUR",
                    when_str
                ))
                .unwrap()
                .json()
                .unwrap();
                log::debug!("Received exchange rates: {:?}", &per_data.rates);
                per_data.rates
            })
            .get(&currency)
            .map(|rate| *rate)
    }

    pub fn convert(when: Date, expense: Expense) -> (ExchangeRate, Euro) {
        lazy_static::lazy_static! {
            static ref EXCHANGE_BURO: ExchangeBuro = ExchangeBuro::default();
        };
        if expense.currency() == Currency::EUR {
            (1.0_f64, Euro(expense.amount()))
        } else {
            let rate = EXCHANGE_BURO
                .exchange_rate_by_date(when, expense.currency())
                .expect("Currency is not supported.");
            (rate, Euro(expense.amount() * rate))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn query() {
        let _ = pretty_env_logger::formatted_timed_builder()
            .is_test(true)
            .try_init();
        let exb = ExchangeBuro::default();
        let when = chrono::Local::today();
        assert_matches!(exb.exchange_rate_by_date(when, Currency::USD), Some(rate) => {
            dbg!(rate)
        });
        assert_matches!(ExchangeBuro::convert(when, Expense(10., Currency::EUR)), (rate, ten) => {
            dbg!(rate);
            assert_eq!(10_i32, ten.0 as i32);
        });
    }
}
