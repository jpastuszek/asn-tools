extern crate cotton;
#[macro_use]
extern crate structopt;
#[macro_use]
extern crate log;
extern crate csv;
extern crate problem;
extern crate ipnet;
extern crate superslice;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;
extern crate itertools;
extern crate app_dirs;

use cotton::prelude::*;
use std::io;
use std::fs::File;
use ipnet::*;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;
use superslice::Ext;
use bincode::{serialize_into, deserialize_from};
use itertools::Itertools;
use app_dirs::*;

const APP_INFO: AppInfo = AppInfo{name: "asn_tools", author: "Jakub Pastuszek"};

#[derive(Serialize, Deserialize, Debug)]
struct AnsRecord {
    ip: u32,
    prefix_len: u8,
    country: String,
    as_number: u32,
    owner: String,
}

impl AnsRecord {
    fn network(&self) -> Ipv4Net {
        Ipv4Net::new(self.ip.into(), self.prefix_len).expect("Bad network")
    }
}

// https://iptoasn.com/
fn load_ip_to_asn_csv<'d, R: io::Read>(data: &'d mut csv::Reader<R>) -> impl Iterator<Item=AnsRecord> + 'd {
    data.records().or_failed_to("read IP2ANS record")
        .filter(|record| {
            let owner = &record[4];
            !(owner == "Not routed" || owner == "None")
        })
        .flat_map(|record| {
            let range_start: Ipv4Addr = record[0].parse().or_failed_to("parse IP");
            let range_end: Ipv4Addr = record[1].parse().or_failed_to("parse IP");
            let as_number: u32 = record[2].parse().or_failed_to("parse AS number");
            let country = record[3].to_owned();
            let owner = record[4].to_owned();

            Ipv4Subnets::new(range_start, range_end, 8).map(move |net| {
                AnsRecord {
                    ip: net.network().into(),
                    prefix_len: net.prefix_len(),
                    country: country.clone(),
                    as_number,
                    owner: owner.clone(),
                }
            })
        })
}

fn db_file_path() -> Result<PathBuf, Problem> {
    let mut db_file_path = app_dir(AppDataType::UserCache, &APP_INFO, "asn_records")?;
    db_file_path.push("db.bincode");
    Ok(db_file_path)
}

fn cache_db(records: &[AnsRecord]) -> Result<(), Problem> {
    let db_file_path = db_file_path()?;
    info!("Caching DB to: {}", db_file_path.display());

    let db_file = File::create(&db_file_path).problem_while_with(|| format!("creating DB file: {}", db_file_path.display()))?;
    serialize_into(BufWriter::new(db_file), records).problem_while("serilizing DB to bincode")?;

    Ok(())
}

fn load_cached_db() -> Result<Option<Vec<AnsRecord>>, Problem> {
    let db_file_path = db_file_path()?;

    if db_file_path.exists() {
        info!("Loading cached DB from: {}", db_file_path.display());
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

    let records = match load_cached_db().or_failed_to("load cached DB") {
        Some(records) => records,
        None => {
            info!("Loading DB from CSV... ");
            let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(BufReader::new(File::open("ip2asn-v4.tsv").or_failed_to("open DB file")));
            let mut records = load_ip_to_asn_csv(&mut rdr).collect::<Vec<_>>();
            records.sort_by_key(|record| record.ip);
            cache_db(&records).or_failed_to("cache records DB");
            records
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
    
    let mut matches = ips.map(|ip| Ipv4Addr::from_str(&ip)).or_failed_to("parse lookup IP").filter_map(|lookup_ip| {
        let index = records.upper_bound_by_key(&lookup_ip.into(), |record| record.ip);
        if index != 0 {
            let record = &records[index - 1];
            if record.network().contains(&lookup_ip) {
                return Some((lookup_ip, record))
            }
        }
        None
    }).collect::<Vec<_>>();

    matches.sort_by_key(|(_, record)| record.ip);

    for (lookup_ip, record) in matches.iter().unique_by(|(_, record)| record.ip) {
        println!("'{:?}', # {} {} {} ({})", record.network(), record.country, record.as_number, record.owner, lookup_ip);
    }
}
