use crate::errors::*;

use serde::Deserialize;

use iban::Iban;

use fs_err as fs;

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct CompanyConfig {
    pub(crate) image: Option<PathBuf>,
    pub(crate) name: String,
    pub(crate) address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Config {
    pub(crate) name: String,

    #[serde(deserialize_with = "deserialize_iban")]
    pub(crate) iban: Iban,

    #[serde(default)]
    pub(crate) company: CompanyConfig,
}

use serde::de;
use serde::de::Visitor;
use serde::Deserializer;

use std::fmt;
use std::str::FromStr;

struct StringyIban;

impl<'de> Visitor<'de> for StringyIban {
    type Value = Iban;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a valid iban")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Iban::from_str(s).map_err(|e| de::Error::custom(format!("Not a valid Iban: {}", e)))
    }
}

fn deserialize_iban<'de, D>(deser: D) -> std::result::Result<Iban, D::Error>
where
    D: Deserializer<'de>,
{
    deser.deserialize_any(StringyIban)
}

use std::convert::AsRef;
use std::path::{Path, PathBuf};

impl Config {
    pub(crate) fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let s = fs::read_to_string(path)?;
        Self::load(&s)
    }

    pub(crate) fn load(s: &str) -> Result<Self> {
        let cfg: Config = toml::from_str(s)?;
        Ok(cfg)
    }

    pub(crate) fn load_user_config() -> Result<Self> {
        let path = Self::user_config_path()?;
        Self::from_file(&path)
    }

    pub(crate) fn user_config_path() -> Result<PathBuf> {
        let dir =
            dirs::config_dir().ok_or_else(|| anyhow!("Missing config dir for current user"))?;
        let path = dir.join("shinypenny.toml");
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        static CFG: &'static str = r#"
name = "Generated Garbage"
iban = "LI2308800847517261798"
"#;
        let _ = Config::load(CFG).unwrap();
    }
}
