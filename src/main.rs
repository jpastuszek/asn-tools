extern crate csv;
extern crate problem;
extern crate ipnet;

use std::io;
use std::fs::File;
use problem::*;
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

// https://iptoasn.com/
fn main() {
    print!("Loading DB... ");
    let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(File::open("ip2asn-v4.tsv").or_failed_to("open DB file"));
    let records = load_db(&mut rdr).collect::<Vec<_>>();
    println!("done");
    
    for lookup_ip in csv::ReaderBuilder::new().from_reader(io::stdin())
        .records().or_failed_to("read lookup IP from stdin")
        .map(|record| Ipv4Addr::from_str(&record[0])).or_failed_to("parse lookup IP") {

        for record in &records {
            if record.net.contains(&lookup_ip) {
                println!("{:?}: {:?} > {} {} {}", lookup_ip, record.net, record.country, record.as_number, record.owner);
            }
        }
    }
}
