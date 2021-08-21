use crate::types::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type ExchangeRate = f64;

/// Exchangerate from currency to EUROs.
#[derive(Default)]
pub struct ExchangeBuro {
    cache: Arc<Mutex<HashMap<chrono::Date<chrono::Utc>, HashMap<Currency, f64>>>>,
}

impl ExchangeBuro {
    /// Obtain a exchange rate for a currency at a specific date.
    fn exchange_rate_by_date(
        &self,
        when: chrono::Date<chrono::Utc>,
        currency: Currency,
    ) -> Option<f64> {
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

    pub fn query(when: chrono::Date<chrono::Utc>, currency: Currency) -> ExchangeRate {
        lazy_static::lazy_static! {
            static ref EXCHANGE_BURO: ExchangeBuro = ExchangeBuro::default();
        };
        if currency == Currency::EUR {
            return 1.0;
        }
        let rate = EXCHANGE_BURO
            .exchange_rate_by_date(when, currency)
            .expect(format!("Currency {} is not supported.", currency).as_str());
        rate
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
        let when = chrono::Local::today().with_timezone(&chrono::Utc);
        assert_matches!(exb.exchange_rate_by_date(when, Currency::USD), Some(rate) => {
            dbg!(rate)
        });
    }
}
