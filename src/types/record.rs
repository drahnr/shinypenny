use super::*;

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
    #[serde(alias = "path")]
    #[serde(alias = "paths")]
    pub(crate) receipts: Receipts,
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

        for (percent, absolute) in other.tax_total.iter() {
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

    sorted: Vec<Euro>,
}

impl<'a> TotalCellIter<'a> {
    pub(crate) fn new(total: &'a Totals) -> Self {
        let sorted: Vec<Euro> = total
            .tax_total
            .iter()
            .sorted_by(|(p1, _), (p2, _)| p1.cmp(&p2))
            .map(|(_percent, euro)| *euro)
            .collect();
        Self {
            total,
            idx: 0usize,
            sorted,
        }
    }
}

impl<'a> Iterator for TotalCellIter<'a> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        let tax_classes = self.total.tax_total.len();
        let val = match self.idx {
            0 | 1 | 2 => "".to_owned(),
            3 => format!("€ {}", self.total.netto),
            x if x < (4 + tax_classes) => self.sorted[x.saturating_sub(4)].to_string(),
            x if x == 4 + tax_classes => self.total.brutto.to_string(),
            _ => return None,
        };
        self.idx += 1;
        Some(val)
    }
}
