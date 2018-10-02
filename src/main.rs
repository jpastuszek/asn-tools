extern crate csv;
extern crate problem;
extern crate ipnet;

use std::io;
use problem::*;
use ipnet::*;
use std::net::{IpAddr, Ipv4Addr};

// https://iptoasn.com/
fn main() {
    println!("Hello, world!");

    let target: Ipv4Addr = "104.149.157.100".parse().or_failed_to("parse target IP");

    let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(io::stdin());
    for record in rdr.records().or_failed_to("read IP2ANS record") {
        //println!("{:?}", record);
        let range_start: Ipv4Addr = record[0].parse().or_failed_to("parse IP");
        let range_end: Ipv4Addr = record[1].parse().or_failed_to("parse IP");
        let as_number = &record[2];
        let country = &record[3];
        let autonomous_system = &record[4];

        if autonomous_system == "Not routed" || autonomous_system == "None" {
            continue;
        }

        for subnet in Ipv4Subnets::new(range_start, range_end, 8) {
            if subnet.contains(&target) {
                println!("{:?}: {:?} - {:?} -> {:?} > {} {} {}", target, range_start, range_end, subnet, country, as_number, autonomous_system);
            }
        }
    }
}
