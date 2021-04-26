use super::*;
use std::path::{Path, PathBuf};

// A set of receipts
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub(crate) struct Receipts(indexmap::IndexSet<PathBuf>);

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

impl Receipts {
    pub fn insert(&mut self, x: impl Into<PathBuf>) {
        self.0.insert(x.into());
    }
}

struct ReceiptsVisitor;

impl<'de> serde::de::Visitor<'de> for ReceiptsVisitor {
    type Value = Receipts;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Receipts must a comma delimited list of paths relative to the current work dir or absulte")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if s.is_empty() {
            Ok(Self::Value::default())
        } else {
            let bare = s
                .split(',')
                .try_fold(Vec::<PathBuf>::default(), |mut acc, path_s| {
                    let path = path_s.trim();
                    if path.is_empty() {
                        return Err(serde::de::Error::custom(format!(
                            "Must contain at least one (1) receipt"
                        )));
                    } else {
                        acc.push(PathBuf::from(path));
                    }
                    Ok(acc)
                })?;
            Ok(Self::Value::from(bare))
        }
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
