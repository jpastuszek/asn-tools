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

#[derive(Debug)]
enum Output {
    Table,
    Puppet,
}

impl FromStr for Output {
    type Err = Problem;
    fn from_str(output: &str) -> Result<Self, Self::Err> {
        match output {
            "table" => Ok(Output::Table),
            "puppet" => Ok(Output::Puppet),
            _ => Err("options are: table, puppet".into()),
        }
    }
}

/// Lookup IP in ASN database
#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    /// Path to database cache file to update; if not given default OS dependent location will be used
    #[structopt(long = "database-cache-path")]
    database_cache_path: Option<PathBuf>,

    /// Output format: table, puppet
    #[structopt(short = "o", long = "output", default_value = "table")]
    output: Output,

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

    let mut ips = ips
        .map(|ip| Ipv4Addr::from_str(&ip)).or_failed_to("parse lookup IP")
        .collect::<Vec<_>>();

    // prepare input so we can group results later on
    ips.sort();
    ips.dedup();

    // resolve and group by record
    let groups = ips
        .into_iter()
        .map(|lookup_ip| (lookup_ip, asn_db.lookup(lookup_ip)))
        .group_by(|(_lookup_ip, record)| record.clone());

    // map out only lookup_ip from each group since key is the record
    let records = groups.into_iter().map(|(lookup_ip, group)| (lookup_ip, group.map(|(lookup_ip, _)| lookup_ip)));

    fn print_puppet<'g>(records: impl Iterator<Item = (Option<&'g asn_db::Record>, impl Iterator<Item = Ipv4Addr>)>) {
        for (record, mut lookup_ips) in records.into_iter() {
            if let Some(record) = record {
                println!("'{:?}', # {} {} {} ({})", record.network(), record.country, record.as_number, record.owner, lookup_ips.join(", "));
            } else {
                for lookup_ip in lookup_ips {
                    println!("'{}', # Not found in the ASN DB", lookup_ip);
                }
            }
        }
    }

    fn print_table<'g>(records: impl Iterator<Item = (Option<&'g asn_db::Record>, impl Iterator<Item = Ipv4Addr>)>) {
        use tabular::{Table, Row, row};

        let mut table = Table::new("{:<} {:<} {:<} {:<} {:<} ");
        table.add_row(row!["Network", "Country", "AS Number", "Owner", "Matched IPs"]);

        for (record, mut lookup_ips) in records.into_iter() {
            let row = if let Some(record) = record {
                Row::new()
                    .with_cell(record.network())
                    .with_cell(&record.country)
                    .with_cell(record.as_number)
                    .with_cell(&record.owner)
                    .with_cell(lookup_ips.join(", "))
            } else {
                Row::new()
                    .with_cell("-")
                    .with_cell("-")
                    .with_cell("-")
                    .with_cell("-")
                    .with_cell(lookup_ips.join(", "))
            };
            table.add_row(row);
        }
        print!("{}", table);
    }

    match args.output {
        Output::Table => print_table(records),
        Output::Puppet => print_puppet(records),
    }
}
