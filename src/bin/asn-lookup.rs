use asn_tools::db_file_path;
use cotton::prelude::*;
use std::io;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use itertools::Itertools;
use asn_db::*;

fn load_cached_db() -> Result<Option<Db>, Problem> {
    let db_file_path = db_file_path()?;
    db_file_path.exists().as_some_from(|| {
        debug!("Loading cached DB from: {}", db_file_path.display());
        in_context_of(format!("loading database from file: {}", db_file_path.display()), || {
            Ok(Db::load(BufReader::new(File::open(db_file_path)?))?)
        })
    }).transpose()
}

/// Lookup IP in ASN database
#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    /// Path to TSV file to build cache from or store downloaded database
    #[structopt(long = "ip2asn-tsv-path", default_value = "ip2asn-v4.tsv")]
    tsv_path: PathBuf,

    /// URL to TSV file containing ip2asn database for update
    #[structopt(long = "ip2asn-tsv-url", default_value = "https://iptoasn.com/data/ip2asn-v4.tsv.gz")]
    tsv_url: String,

    /// Fetch new TSV database file form ip2asn-tsv-url and rebuild database
    #[structopt(long = "update")]
    update: bool,

    /// List of IP addresses to lookup (can also be read from stdin, one per line)
    #[structopt(name = "IP")]
    ips: Vec<String>,
}

fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    let tsv_path = args.tsv_path;
    let asn_db = load_cached_db().or_failed_to("load cached DB")
        .unwrap_or_else(|| {
            info!("Loading DB from TSV: {}", tsv_path.display());
            in_context_of(format!("loading database from TSV file: {}", tsv_path.display()), || {
                Ok(Db::form_tsv(BufReader::new(File::open(tsv_path)?))?)
            })
            .tap_ok(|_| warn!("Consider running asn-update to get TSV file cached for fast loading"))
            .or_failed_to("load ASN database")
        });

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
