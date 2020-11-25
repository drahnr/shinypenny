use super::*;

use fints_institute_db::get_bank_by_bank_code;
use fints_institute_db::Bank;

use iban::Iban;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct BankInfo {
    /// Full name of the bank account owner.
    pub(crate) name: String,
    /// IBAN contains all info about the bank, so that's all needed
    pub(crate) iban: Iban,
    /// Your bank institute information.
    bank: Option<Bank>,
}

impl BankInfo {
    pub(crate) fn new(name: impl AsRef<str>, iban: Iban) -> Result<Self> {
        // let iban = Iban::from_str(iban.as_ref())?;
        let name = name.as_ref().to_owned();
        let bank_indentifier = iban
            .bank_identifier()
            .ok_or_else(|| eyre!("Failed to extract bank identifier from IBAN"))?;
        let bank = get_bank_by_bank_code(bank_indentifier);
        Ok(Self { name, iban, bank })
    }

    /// If the institute is not in the db, should return `None`.
    pub fn institute(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.institute.clone())
    }

    #[allow(unused)]
    pub fn bank_code(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.bank_code.clone())
    }

    #[allow(unused)]
    pub fn location(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.location.clone())
    }
    pub fn bic(&self) -> Option<String> {
        self.bank.as_ref().map(|ref bank| bank.bic.clone())
    }
}
