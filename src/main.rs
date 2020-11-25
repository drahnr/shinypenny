use std::io::BufRead;
use std::path::PathBuf;

use docopt::Docopt;
use serde::Deserialize;

use fs_err as fs;

mod errors;
pub use errors::*;

mod types;
pub use types::*;

mod pdf;

mod config;
use config::Config;

const USAGE: &'static str = r#"
shinypenny

Usage:
  shinypenny [(-q|-v...)] [-c <config>] [--learning] [--date=<date>] --company=<company> --desc=<desc> --brutto=<brutto> --tax-percent=<tax_percent> --netto=<netto> [--dest=<dest>] <receipts>..
  shinypenny [(-q|-v...)] [-c <config>] [--learning] --csv=<csv> [--dest=<dest>]
  shinypenny config
  shinypenny --version

Options:
  --version                     Show version.
  -v --verbose                  Verbosity level.
  -q --quiet                    Silence all messages, dominates `-v`.
  -h --help                     Show this screen.
  --learning                    Deduct from learning budget.
  -c --config                   An alternative configuration file.
  --desc=<desc>                 What was purchased.
  --brutto=<brutto>             Amount of € to be re-imbursed (includes tax).
  --tax-percent=<tax_percent>   The tax percentage used.
  --netto=<netto>               Value of the service goods without added tax.
  --date=<date>                 The date of receipt creation, defaults to today.
  --dest=<dest>                 Write the receipt to the given dest file
"#;

#[derive(Debug, Deserialize)]
struct Args {
    arg_dest: Option<PathBuf>,
    arg_receipts: Receipts,
    cmd_config: bool,
    flag_date: Option<chrono::NaiveDate>,
    flag_company: Option<String>,
    flag_brutto: Option<Euro>,
    flag_tax_percent: Option<Percentage>,
    flag_netto: Option<Euro>,
    flag_desc: Option<String>,
    flag_version: bool,
    flag_verbose: Option<usize>,
    flag_quiet: bool,
    flag_learning: bool,
    flag_csv: Option<PathBuf>,
    flag_config: Option<PathBuf>,
}

use float_cmp::ApproxEq;
use lopdf::Document;

/// Create the pdf from all records
fn create_pdf(
    records: &[Record],
    bankinfo: BankInfo,
    companyinfo: CompanyInfo,
    learning_budget: bool,
) -> Result<Document> {
    let mut documents = Vec::with_capacity(records.len() + 1);

    let mut rows = Vec::with_capacity(records.len());
    let mut totals = Totals::default();

    // we want to create a column for each tax value
    let mut tax_percentage_set = indexmap::IndexSet::<Percentage>::default();

    // transform the csv `Record`s into table `Row` types
    let mut receipts = Vec::with_capacity(32);

    for record in records.into_iter() {
        receipts.push((record.description.as_str(), &record.receipts));

        let brutto = record.brutto;
        let netto = record.netto;
        let percentage = record.tax;
        if brutto < netto {
            bail!("For expenses, `netto` must be less than `brutto`.");
        }
        let delta: Euro = brutto - netto;

        let vat = netto * percentage;
        if !&delta.approx_eq(vat, EPSILON) {
            bail!(
                "The percentage {} derived delta {} does not match the provided delta {} between brutto {} and netto {} with a max epsilon error of {}",
                percentage,
                vat,
                delta,
                brutto,
                netto, EPSILON
            );
        }

        // track all tax percentage values
        // commonly 0; 5; 7; 16; 19
        tax_percentage_set.insert(percentage);

        let row = Row {
            date: Date::from_utc(record.date, chrono::FixedOffset::west(0)), // TODO assume
            description: record.description.clone(),
            company: record.company.clone(),
            brutto,
            netto,
            tax_total: indexmap::indexmap! { percentage => delta },
        };

        totals.add(&row);
        rows.push(row);
    }

    // fill up all rows to the same number
    for row in rows.iter_mut() {
        use itertools::Itertools;

        for percentage in tax_percentage_set.iter() {
            row.tax_total.entry(*percentage).or_default();
        }
        row.tax_total = row
            .tax_total
            .clone()
            .into_iter()
            .sorted_by(|(p1, _), (p2, _)| p1.cmp(&p2))
            .collect();
    }

    log::info!("Number integrity checks and folding complete");

    for (desc, receipt_paths) in receipts {
        documents.push(pdf::separation_page(desc)?);
        for path in receipt_paths {
            let document = pdf::load_receipt(path)?;
            documents.push(document);
        }
    }

    log::info!("Receipt document loading complete");

    let tabular = pdf::tabular(bankinfo, companyinfo, &rows, totals, learning_budget)?;

    documents.insert(0, tabular);

    let x = pdf::combine(&mut documents)?;

    log::info!("Document creation complete");

    Ok(x)
}

fn run() -> Result<()> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let level = if args.flag_quiet {
        log::LevelFilter::Warn
    } else if let Some(verbosity) = args.flag_verbose {
        match verbosity {
            x if x >= 4 => log::LevelFilter::Trace,
            3 => log::LevelFilter::Debug,
            2 => log::LevelFilter::Info,
            1 => log::LevelFilter::Warn,
            0 => log::LevelFilter::Error,
            _ => log::LevelFilter::Warn,
        }
    } else {
        log::LevelFilter::Warn
    };

    pretty_env_logger::formatted_builder()
        .filter_level(level)
        .init();

    if args.flag_version {
        println!("shinypenny {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config = if let Some(config) = args.flag_config {
        Config::from_file(&config)
    } else {
        log::info!(
            "Using default user config path {}",
            Config::user_config_path()?.display()
        );
        Config::load_user_config()
    }?;

    if args.cmd_config {
        println!("{:?}", config);
        return Ok(());
    }

    let dest = if let Some(dest) = args.arg_dest {
        log::debug!("Using provides destination path: {}", dest.display());
        dest
    } else {
        let today = chrono::Local::today();
        let file_name = today
            .format("reimbursement_request_%Y_%m_%d.pdf")
            .to_string();
        let dest = std::env::current_dir()
            .expect("CWD must exists")
            .join(file_name);
        log::info!("Using default destination path {}", dest.display());
        dest
    };

    // collect csv `Record`s
    let data = if let Some(path) = args.flag_csv.as_ref() {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(false)
            .truncate(false)
            .open(path)
            .map_err(|_e| eyre!("Failed to open passed --csv <{}>", path.display()))?;
        let mut buffered = std::io::BufReader::with_capacity(4096, &mut file);

        // attempt once with each separator
        const SEP: &[u8] = &[b'|', b';', b','];
        let mut r = Err(eyre!("unreachable"));
        for sep in SEP.into_iter().copied() {
            let buffered = std::io::BufReader::with_capacity(4096, &mut buffered);
            r = data_plumbing(buffered, sep);
            if r.is_ok() {
                break;
            }
            log::warn!(
                "Splitting with separator '{}' failed, trying next",
                sep as char
            );
        }
        let data = r.map_err(|_e| eyre!("No separator could read the provided data stream"))?;
        data
    } else {
        // create a single record from the provided commandline flags
        vec![Record {
            date: args.flag_date.unwrap_or_else(|| {
                let today = chrono::Local::today();
                today.naive_local()
            }),
            description: args
                .flag_desc
                .expect("docopt assured description has a value. qed"),
            company: args
                .flag_company
                .unwrap_or_else(|| config.company.name.clone()),
            netto: args
                .flag_netto
                .expect("docopt assured netto has a value. qed"),
            tax: args
                .flag_tax_percent
                .expect("docopt assured tax has a value. qed"),
            brutto: args
                .flag_brutto
                .expect("docopt assured brutto has a value. qed"),
            receipts: args.arg_receipts,
        }]
    };

    let bankinfo = BankInfo::new(&config.name, config.iban)?;

    log::info!("BankInfo: {:?}", &bankinfo);
    log::info!("Institute: {}", bankinfo.institute().unwrap());

    let company = &config.company;
    let companyinfo = CompanyInfo::new(&company.name, &company.address, company.image.clone())?;

    let mut document = create_pdf(&data, bankinfo, companyinfo, args.flag_learning)?;

    // size would be way too large, but this does not do too much
    document.compress();
    document.prune_objects();

    document.save(dest)?;

    Ok(())
}

fn data_plumbing(mut buffered: impl BufRead, separator: u8) -> Result<Vec<Record>> {
    let mut data = Vec::<Record>::with_capacity(256);

    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .delimiter(separator)
        .has_headers(false)
        .from_reader(&mut buffered);

    const FIELDS: &[&'static str] = &["date", "description", "netto", "tax", "brutto", "path"];

    let mut records = rdr.records();

    // manually parse the first row, and determine if it is a header
    // or just starts with plain dataset
    let header = if let Some(rec) = records.next() {
        let rec = rec.map_err(|e| eyre!("Failed to parse csv line: {:?}", e))?;
        let mut fields = FIELDS
            .into_iter()
            .map(|x| -> String { (*x).to_owned() })
            .enumerate()
            .map(|(idx, field)| (field, idx))
            .collect::<indexmap::IndexMap<String, usize>>();

        // crafting this mapping is a bit over the top
        // technically it's a confusion mapping.
        // But the `Option<StringRecord>` header passed to the deserialize
        // has the same purpose.
        let mapping = rec
            .iter()
            .enumerate()
            .filter_map(|(idx, field)| {
                let s = field.to_lowercase();
                fields.remove(&s).map(|maps2| (idx, maps2))
            })
            .collect::<indexmap::IndexMap<usize, usize>>();

        if FIELDS.len() == mapping.len() {
            assert!(fields.is_empty());
            log::info!("Found header");
            Some(rec)
        } else {
            log::info!("No header, assume default order and attempt to consume");
            // we don't need a mapping here, it's the default sequence
            let rec = rec
                .deserialize::<Record>(None)
                .map_err(|_e| eyre!("Failed to parse record <{:?}>", rec))?;
            data.push(rec);
            None
        }
    } else {
        return Err(eyre!("Provided CSV file is empty"));
    };

    for rec in records {
        let rec = rec.map_err(|_e| eyre!("Failed to parse csv line"))?;

        let rec = rec
            .deserialize::<Record>(header.as_ref())
            .map_err(|_e| eyre!("Failed to parse record <{:?}>", rec))?;
        data.push(rec);
    }

    Ok(data)
}

fn main() -> Result<()> {
    color_eyre::install()?;
    run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static DATA: &[(&'static str, usize /*, &[Record]*/)] = &[
        (
            r#"date      |company|description                    |netto |tax |brutto|path
2020-09-20|SoloDudeSeller|Device: Superblaster 2k21      |95|0.05|100.00|assets/spensiv.pdf
"#,
            1usize,
        ),
        (
            r#"2020-09-20|Big$Corp|FFF   |95|0.05| 100.00|assets/spensiv.pdf"#,
            1usize,
        ),
        (
            r#"2021-09-20|SingleDev|FFF   |95|0.05| 100.00|assets/spensiv.pdf
2020-09-20|CorpInc|TTT|95   |0.05| 100.00|assets/funny.pdf
"#,
            2usize,
        ),
        (
            r#"2021-09-20|SingleDev|FFF   |95|0.05| 100.00|assets/spensiv.pdf,assets/funny.pdf
2020-09-20|CorpInc|TTT|95   |0.05| 100.00|assets/funny.pdf
"#,
            2usize,
        ),
        (
            r#"description|company|date                   |path |netto |tax |brutto
Device: Superblaster 2k21|abc| 2020-09-20   |assets/spensiv.pdf |95.00|0.05| 100.00
"#,
            1usize,
        ),
    ];

    #[test]
    fn data() {
        for (idx, data) in DATA.iter().enumerate().skip(0) {
            println!("Processing test sample #{}", idx);
            println!("{}", data.0);
            let cursor = std::io::Cursor::new(&data.0);
            let buffered = std::io::BufReader::with_capacity(4096, cursor);

            let rows = dbg!(data_plumbing(buffered, b'|').expect("Data plumbing works. qed"));
            assert_eq!(data.1, rows.len());
        }
    }
}
