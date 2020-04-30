use asn_db::*;
use asn_tools::default_database_cache_path;
use cotton::prelude::*;
use itertools::Itertools;
use std::io;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn load_cached_db(db_file_path: &Path) -> Result<Db, Problem> {
    in_context_of(
        &format!("loading database from file: {}", db_file_path.display()),
        || Ok(Db::load(BufReader::new(File::open(db_file_path)?))?),
    )
}

#[derive(Debug)]
enum Output {
    Table,
    Csv,
    Json,
    Puppet,
}

impl FromStr for Output {
    type Err = Problem;
    fn from_str(output: &str) -> Result<Self, Self::Err> {
        match output {
            "table" => Ok(Output::Table),
            "csv" => Ok(Output::Csv),
            "json" => Ok(Output::Json),
            "puppet" => Ok(Output::Puppet),
            _ => Err("options are: table, csv, json, puppet".into()),
        }
    }
}

/// Lookup an IP address in the ASN database.
#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    /// Path to the database cache file [default: OS dependent location]
    #[structopt(long = "database-cache-path")]
    database_cache_path: Option<PathBuf>,

    /// Input CSV delimiter
    #[structopt(long = "input-csv-delimiter", default_value = ",")]
    input_csv_delimiter: String,

    /// Input CSV separator
    #[structopt(long = "input-csv-ip-column", default_value = "1")]
    input_csv_ip_column: usize,

    /// Output format: table, csv, json, puppet
    #[structopt(short = "o", long = "output", default_value = "table")]
    output: Output,

    /// Don't list matched IP addresses
    #[structopt(short = "n", long = "no-matched-ips")]
    no_matched_ips: bool,

    /// List of IP addresses to lookup (can also be read from stdin, one per line; may be in CSV format where the first column is an IP)
    #[structopt(name = "IP")]
    ips: Vec<String>,
}

fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    if args.input_csv_delimiter.len() != 1 {
        panic!("input-csv-delimiter needs to be exactly one character")
    }

    if args.input_csv_ip_column < 1 {
        panic!("input-csv-ip-column needs to be greater than 0")
    }

    let db_file_path = args.database_cache_path.unwrap_or_else(|| {
        default_database_cache_path().or_failed_to("get default database cache file path")
    });

    debug!(
        "Loading database cache file from: {}",
        db_file_path.display()
    );
    if !db_file_path.exists() {
        error!(
            "No database cache file found in '{}', please use asn-update to create one.",
            db_file_path.display()
        );
        std::process::exit(2)
    }
    let asn_db = load_cached_db(&db_file_path).or_failed_to("load database cache file");

    let mut stdin_csv = if args.ips.is_empty() {
        Some(
            csv::ReaderBuilder::new()
                .delimiter(args.input_csv_delimiter.as_bytes()[0] as u8)
                .from_reader(io::stdin()),
        )
    } else {
        None
    };

    let column = args.input_csv_ip_column - 1;

    let ips = args
        .ips
        .into_iter()
        .chain(stdin_csv.iter_mut().flat_map(|csv| {
            csv.records()
                .or_failed_to("read lookup IP from stdin")
                .map(|record| {
                    record
                        .get(column)
                        .or_failed_to("error accessing CSV column")
                        .to_owned()
                })
        }));

    let mut ips = ips
        .map(|ip| Ipv4Addr::from_str(&ip))
        .or_failed_to("parse lookup IP")
        .collect::<Vec<_>>();

    // Prepare input so we can group results later on
    ips.sort();
    ips.dedup();

    // Resolve and group by record
    let groups = ips
        .into_iter()
        .map(|lookup_ip| (lookup_ip, asn_db.lookup(lookup_ip)))
        .group_by(|(_lookup_ip, record)| record.clone());

    // If true the records will provide empty list of IPs
    let no_matched_ips = args.no_matched_ips;

    // Map out only lookup_ip from each group since key is the record
    let records = groups.into_iter().map(|(lookup_ip, group)| {
        (
            lookup_ip,
            (!no_matched_ips).as_some(group.map(|(lookup_ip, _)| lookup_ip)),
        )
    });

    fn print_puppet<'g>(
        records: impl Iterator<
            Item = (
                Option<&'g asn_db::Record>,
                Option<impl Iterator<Item = Ipv4Addr>>,
            ),
        >,
    ) {
        for (record, lookup_ips) in records.into_iter() {
            if let Some(record) = record {
                print!(
                    "'{:?}', # {} {} {}",
                    record.network(),
                    record.country,
                    record.as_number,
                    record.owner
                );
                if let Some(mut matched_ips) = lookup_ips {
                    println!(" ({})", matched_ips.join(", "));
                } else {
                    println!();
                }
            } else {
                if let Some(matched_ips) = lookup_ips {
                    for matched_ips in matched_ips {
                        println!("'{}', # Not found in the ASN DB", matched_ips);
                    }
                }
            }
        }
    }

    fn print_table<'g>(
        records: impl Iterator<
            Item = (
                Option<&'g asn_db::Record>,
                Option<impl Iterator<Item = Ipv4Addr>>,
            ),
        >,
    ) {
        use tabular::{row, Row, Table};

        let mut table = Table::new("{:<} {:<} {:<} {:<} {:<} ");
        table.add_row(row![
            "Network",
            "Country",
            "AS Number",
            "Owner",
            "Matched IPs"
        ]);

        for (record, lookup_ips) in records.into_iter() {
            let row = Row::new()
                .with_cell(
                    record
                        .map(|r| r.network().to_string())
                        .unwrap_or("-".to_owned()),
                )
                .with_cell(record.as_ref().map(|r| r.country.as_str()).unwrap_or("-"))
                .with_cell(
                    record
                        .map(|r| r.as_number.to_string())
                        .unwrap_or("-".to_owned()),
                )
                .with_cell(record.as_ref().map(|r| r.owner.as_str()).unwrap_or("-"))
                .with_cell(
                    lookup_ips
                        .map(|mut ips| ips.join(", "))
                        .unwrap_or("-".to_owned()),
                );

            table.add_row(row);
        }
        print!("{}", table);
    }

    fn print_csv<'g>(
        records: impl Iterator<
            Item = (
                Option<&'g asn_db::Record>,
                Option<impl Iterator<Item = Ipv4Addr>>,
            ),
        >,
    ) -> Result<(), Problem> {
        use csv::WriterBuilder;

        let mut csv = WriterBuilder::new().from_writer(std::io::stdout());
        csv.write_record(&["Network", "Country", "AS Number", "Owner", "Matched IPs"])?;

        for (record, lookup_ips) in records.into_iter() {
            csv.write_field(
                record
                    .map(|r| r.network().to_string())
                    .unwrap_or("-".to_owned()),
            )?;
            csv.write_field(record.as_ref().map(|r| r.country.as_str()).unwrap_or("-"))?;
            csv.write_field(
                record
                    .map(|r| r.as_number.to_string())
                    .unwrap_or("-".to_owned()),
            )?;
            csv.write_field(record.as_ref().map(|r| r.owner.as_str()).unwrap_or("-"))?;
            csv.write_field(
                lookup_ips
                    .map(|mut ips| ips.join(", "))
                    .unwrap_or("-".to_owned()),
            )?;
            csv.write_record(None::<&[u8]>)?;
        }
        Ok(())
    }

    fn print_json<'g>(
        records: impl Iterator<
            Item = (
                Option<&'g asn_db::Record>,
                Option<impl Iterator<Item = Ipv4Addr>>,
            ),
        >,
    ) {
        use json_in_type::{inlined_json_object, json_object, JSONValue};
        use std::cell::RefCell;

        for (record, lookup_ips) in records.into_iter() {
            println!("{}", json_object!{
                network: record.map(|record| record.network().to_string()),
                country: record.map(|record| &record.country),
                as_number: record.map(|record| record.as_number.to_string()),
                owner: record.map(|record| &record.owner),
                matched_ips: lookup_ips.map(|lookup_ips| RefCell::new(lookup_ips.map(|i| i.to_string()))),
            }.to_json_string());
        }
    }

    match args.output {
        Output::Table => print_table(records),
        Output::Csv => print_csv(records).or_failed_to("print CSV"),
        Output::Json => print_json(records),
        Output::Puppet => print_puppet(records),
    }
}
