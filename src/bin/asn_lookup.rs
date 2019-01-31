use cotton::prelude::*;
use log::*;
use std::io;
use std::fs::File;
use std::path::Path;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use bincode::{serialize_into, deserialize_from};
use itertools::Itertools;
use app_dirs::*;

use asn_tools::*;

const APP_INFO: AppInfo = AppInfo{name: "asn_tools", author: "Jakub Pastuszek"};

fn db_file_path() -> Result<PathBuf, Problem> {
    let mut db_file_path = app_dir(AppDataType::UserCache, &APP_INFO, "asn_records")?;
    db_file_path.push("db.bincode");
    Ok(db_file_path)
}

fn cache_db(records: &[AnsRecord]) -> Result<(), Problem> {
    let db_file_path = db_file_path()?;
    debug!("Caching DB to: {}", db_file_path.display());

    let db_file = File::create(&db_file_path).problem_while_with(|| format!("creating DB file: {}", db_file_path.display()))?;
    serialize_into(BufWriter::new(db_file), records).problem_while("serilizing DB to bincode")?;

    Ok(())
}

fn load_cached_db() -> Result<Option<Vec<AnsRecord>>, Problem> {
    let db_file_path = db_file_path()?;

    if db_file_path.exists() {
        debug!("Loading cached DB from: {}", db_file_path.display());
        let db_file = File::open(&db_file_path)?;
        Ok(Some(deserialize_from(BufReader::new(db_file)).problem_while("read bincode DB file")?))
    } else {
        Ok(None)
    }
}

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    #[structopt(name = "IP")]
    ips: Vec<String>,
}

fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    // let records = match load_cached_db().or_failed_to("load cached DB") {
    //     Some(records) => records,
    //     None => {
    //         debug!("Loading DB from CSV... ");
    //         AsnDb::form_csv_file(Path::new("ip2asn-v4.tsv")).or_failed_to("open DB file")
    //     }
    // };
    let asn_db = AsnDb::form_csv_file(Path::new("ip2asn-v4.tsv")).or_failed_to("open DB file");

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
