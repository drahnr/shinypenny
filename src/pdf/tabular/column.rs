use super::super::types::Pt;

/// Width of a column.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColumnWidth(pub(crate) Pt);

#[derive(Debug, Clone)]
pub(crate) struct ColumnWidthSet(pub(crate) Vec<ColumnWidth>);

impl ColumnWidthSet {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    /// Calculate the total width of the set.
    pub(crate) fn total_width(&self) -> Pt {
        self.0.iter().fold(Pt(0.), |acc, cw| acc + cw.0)
    }
}

impl<'a> IntoIterator for &'a ColumnWidthSet {
    type IntoIter = ColumnWidthSetIter<'a>;
    type Item = Pt;
    fn into_iter(self) -> Self::IntoIter {
        ColumnWidthSetIter {
            idx: 0usize,
            set: self,
        }
    }
}

pub(crate) struct ColumnWidthSetIter<'a> {
    idx: usize,
    set: &'a ColumnWidthSet,
}

impl<'a> Iterator for ColumnWidthSetIter<'a> {
    type Item = Pt;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(x) = self.set.0.get(self.idx).map(|w| w.0) {
            self.idx += 1;
            return Some(x);
        }
        None
    }
}
