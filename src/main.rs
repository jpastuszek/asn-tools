extern crate cotton;
#[macro_use]
extern crate structopt;
#[macro_use]
extern crate log;
extern crate csv;
extern crate problem;
extern crate ipnet;

use cotton::prelude::*;
use std::io;
use std::fs::File;
use ipnet::*;
use std::net::Ipv4Addr;
use std::str::FromStr;

#[derive(Debug)]
struct AnsRecord {
    net: Ipv4Net,
    country: String,
    as_number: u32,
    owner: String,
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
                    net,
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

    let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(File::open("ip2asn-v4.tsv").or_failed_to("open DB file"));
    let records = load_db(&mut rdr).collect::<Vec<_>>();
    info!("DB loaded");

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

        for record in &records {
            if record.net.contains(&lookup_ip) {
                println!("{:?}: {:?} > {} {} {}", lookup_ip, record.net, record.country, record.as_number, record.owner);
            }
        }
    }
}
