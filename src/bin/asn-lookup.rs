use asn_tools::default_database_cache_path;
use cotton::prelude::*;
use std::io;
use std::net::Ipv4Addr;
use std::path::{PathBuf, Path};
use std::str::FromStr;
use itertools::Itertools;
use asn_db::*;

fn load_cached_db(db_file_path: &Path) -> Result<Db, Problem> {
    in_context_of(&format!("loading database from file: {}", db_file_path.display()), || {
        Ok(Db::load(BufReader::new(File::open(db_file_path)?))?)
    })
}

/// Lookup IP in ASN database
#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    /// Path to database cache file to update; if not given default OS dependent location will be used
    #[structopt(long = "database-cache-path")]
    database_cache_path: Option<PathBuf>,

    /// List of IP addresses to lookup (can also be read from stdin, one per line)
    #[structopt(name = "IP")]
    ips: Vec<String>,
}

fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    let db_file_path = args.database_cache_path.unwrap_or_else(|| default_database_cache_path().or_failed_to("get default database cache file path"));

    debug!("Loading database cache file from: {}", db_file_path.display());
    if !db_file_path.exists() {
        error!("No database cache file found in '{}', please use asn-update to create one.", db_file_path.display());
        std::process::exit(2)
    }
    let asn_db = load_cached_db(&db_file_path).or_failed_to("load database cache file");

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
