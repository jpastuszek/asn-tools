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

use cotton::prelude::*;
use std::io;
use std::fs::File;
use ipnet::*;
use std::net::Ipv4Addr;
use std::str::FromStr;
use superslice::Ext;
use bincode::{serialize_into, deserialize_from};

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

fn load_db<'d, R: io::Read>(data: &'d mut csv::Reader<R>) -> impl Iterator<Item=AnsRecord> + 'd {
    data.records().or_failed_to("read IP2ANS record")
        .filter(|record| {
            let owner = &record[4];
            !(owner == "Not routed" || owner == "None")
        })
        .flat_map(|record| {
            //println!("{:?}", record);
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

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    #[structopt(name = "IP")]
    ips: Vec<String>,
}

// https://iptoasn.com/
fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);
    info!("Loading DB... ");
    let records = match File::open("db.bincode") {
        Ok(db_file) => deserialize_from(BufReader::new(db_file)).or_failed_to("read bincode DB file"),
        Err(_) => {
            let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(BufReader::new(File::open("ip2asn-v4.tsv").or_failed_to("open DB file")));
            let mut records = load_db(&mut rdr).collect::<Vec<_>>();
            info!("CSV DB loaded; sorting...");
            records.sort_by_key(|record| record.ip);
            info!("writing bincode DB...");
            let db_file = File::create("db.bincode").or_failed_to("open bincode DB for writing");
            serialize_into(db_file, &records).or_failed_to("write bincode DB to file");
            records
        }
    };
    info!("DB ready");

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
    
    for lookup_ip in ips 
        .map(|ip| Ipv4Addr::from_str(&ip)).or_failed_to("parse lookup IP") {

        let index = records.upper_bound_by_key(&lookup_ip.into(), |record| record.ip);
        if index == 0 {
            continue;
        }
        let record = &records[index - 1];
        if record.network().contains(&lookup_ip) {
            println!("{:?}: {:?} > {} {} {}", lookup_ip, record.network(), record.country, record.as_number, record.owner);
        }
    }
}
