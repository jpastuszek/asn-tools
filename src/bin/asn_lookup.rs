use cotton::prelude::*;
use std::io;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use itertools::Itertools;
use app_dirs::*;

use asn_tools::*;

const APP_INFO: AppInfo = AppInfo{name: "asn_tools", author: "Jakub Pastuszek"};

fn db_file_path() -> Result<PathBuf, Problem> {
    let mut db_file_path = app_dir(AppDataType::UserCache, &APP_INFO, "asn_records")?;
    db_file_path.push("db.bincode");
    Ok(db_file_path)
}

fn cache_db(asn_db: &AsnDb) -> Result<(), Problem> {
    let db_file_path = db_file_path()?;
    debug!("Caching DB to: {}", db_file_path.display());
    asn_db.store(db_file_path)?;
    Ok(())
}

fn load_cached_db() -> Result<Option<AsnDb>, Problem> {
    let db_file_path = db_file_path()?;
    Ok(if db_file_path.exists() {
        debug!("Loading cached DB from: {}", db_file_path.display());
        Some(AsnDb::from_stored_file(db_file_path)?)
    } else {
        None
    })
}

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    #[structopt(long = "ip2asn-tsv", default_value = "ip2asn-v4.tsv")]
    tsv_path: PathBuf,

    #[structopt(name = "IP")]
    ips: Vec<String>,
}

fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    let asn_db = match load_cached_db().or_failed_to("load cached DB") {
        Some(records) => records,
        None => {
            debug!("Loading DB from CSV: {}", args.tsv_path.display());
            AsnDb::form_csv_file(args.tsv_path).tap_ok(|asn_db| cache_db(asn_db).or_failed_to("cache DB file")).or_failed_to("open DB file")
        }
    };

    let mut stdin_csv = if args.ips.is_empty() {
        Some(csv::ReaderBuilder::new()
            .from_reader(io::stdin()))
    } else {
        None
    };

    let ips = args.ips.into_iter()
        .chain(stdin_csv.iter_mut().flat_map(|csv|
            csv.records().or_failed_to("read lookup IP from stdin")
            .map(|record| record[0].to_owned())));
    
    let mut matches = ips.map(|ip| Ipv4Addr::from_str(&ip)).or_failed_to("parse lookup IP").map(|lookup_ip| {
        (lookup_ip, asn_db.lookup(lookup_ip))
    }).collect::<Vec<_>>();

    matches.sort_by_key(|(lookup_ip, _)| lookup_ip.clone());

    for (lookup_ip, record) in matches.iter().unique_by(|(lookup_ip, record)| record.map(|r| r.ip).unwrap_or(lookup_ip.clone().into())) {
        if let Some(record) = record {
            println!("'{:?}', # {} {} {} ({})", record.network(), record.country, record.as_number, record.owner, lookup_ip);
        } else {
            println!("'{}', # IP '{}' was not found in the ASN DB", lookup_ip, lookup_ip);
        }
    }
}
