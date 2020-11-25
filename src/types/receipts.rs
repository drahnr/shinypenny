use super::*;
use std::path::{Path, PathBuf};

// A set of receipts
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Receipts(pub indexmap::IndexSet<PathBuf>);

impl<A> std::iter::FromIterator<A> for Receipts
where
    A: std::convert::AsRef<Path>,
{
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let bare = iter
            .into_iter()
            .map(|p| p.as_ref().to_owned())
            .collect::<Vec<PathBuf>>();
        Self::from(bare)
    }
}

impl<'p> std::iter::IntoIterator for &'p Receipts {
    type Item = &'p PathBuf;
    type IntoIter = indexmap::set::Iter<'p, PathBuf>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T> std::convert::From<Vec<T>> for Receipts
where
    T: std::convert::AsRef<Path>,
{
    fn from(bare: Vec<T>) -> Self {
        Receipts(bare.into_iter().map(|p| p.as_ref().to_owned()).collect())
    }
}

struct ReceiptsVisitor;

impl<'de> serde::de::Visitor<'de> for ReceiptsVisitor {
    type Value = Receipts;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Euros denotion, with or withou € suffix")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let s = s.trim();
        let bare = s.split(',').try_fold::<Vec<PathBuf>, _, _>(
            Default::default(),
            |mut acc, path_s| {
                let path = PathBuf::from(path_s);
                let path = fs::canonicalize::<PathBuf>(path)
                    .map_err(|e| serde::de::Error::custom(format!(": {}", e)))?;
                acc.push(path);
                Ok(acc)
            },
        )?;
        Ok(Self::Value::from(bare))
    }
}

impl<'de> serde::de::Deserialize<'de> for Receipts {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_str(ReceiptsVisitor)
    }
}
