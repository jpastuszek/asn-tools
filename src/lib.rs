use serde_derive::{Serialize, Deserialize};
use error_context::*;
use ipnet::*;
use std::net::Ipv4Addr;
use std::io;
use std::fmt;
use std::error::Error;
use std::io::BufReader;
use std::fs::File;
use std::path::Path;
use superslice::Ext;

#[derive(Serialize, Deserialize, Debug)]
pub struct AnsRecord {
    pub ip: u32,
    pub prefix_len: u8,
    pub country: String,
    pub as_number: u32, pub owner: String,
}

impl AnsRecord {
    pub fn network(&self) -> Ipv4Net {
        Ipv4Net::new(self.ip.into(), self.prefix_len).expect("Bad network")
    }
}

#[derive(Debug)]
pub enum AsnCsvParseError {
    CsvError(csv::Error),
    AddrFieldParseError(std::net::AddrParseError, &'static str),
    IntFieldParseError(std::num::ParseIntError, &'static str),
}

impl fmt::Display for AsnCsvParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AsnCsvParseError::CsvError(err) => write!(f, "CSV format error: {}", err),
            AsnCsvParseError::AddrFieldParseError(err, context) => write!(f, "error parsing IP address while {}: {}", context, err),
            AsnCsvParseError::IntFieldParseError(err, context) => write!(f, "error parsing integer while {}: {}", context, err),
        }
    }
}

impl Error for AsnCsvParseError {}

impl From<csv::Error> for AsnCsvParseError {
    fn from(error: csv::Error) -> AsnCsvParseError {
        AsnCsvParseError::CsvError(error)
    }
}

impl From<ErrorContext<std::net::AddrParseError, &'static str>> for AsnCsvParseError {
    fn from(ec: ErrorContext<std::net::AddrParseError, &'static str>) -> AsnCsvParseError {
        AsnCsvParseError::AddrFieldParseError(ec.error, ec.context)
    }
}


impl From<ErrorContext<std::num::ParseIntError, &'static str>> for AsnCsvParseError {
    fn from(ec: ErrorContext<std::num::ParseIntError, &'static str>) -> AsnCsvParseError {
        AsnCsvParseError::IntFieldParseError(ec.error, ec.context)
    }
}

// TODO: try this https://docs.rs/eytzinger/1.0.1/eytzinger/

/// Reads ASN database CSV file as provided at https://iptoasn.com/
pub fn read_asn_csv<'d, R: io::Read>(data: &'d mut csv::Reader<R>) -> impl Iterator<Item=Result<AnsRecord, AsnCsvParseError>> + 'd {
    data.records()
        .filter(|record| {
            if let Ok(record) = record {
                let owner = &record[4];
                !(owner == "Not routed" || owner == "None")
            } else {
                true
            }
        })
        .map(|record| record.map_err(Into::<AsnCsvParseError>::into))
        .map(|record| {
            record.and_then(|record| {
                let range_start: Ipv4Addr = record[0].parse().wrap_error_while("parsing range_start IP")?;
                let range_end: Ipv4Addr = record[1].parse().wrap_error_while("parsing range_end IP")?;
                let as_number: u32 = record[2].parse().wrap_error_while("parsing as_number")?;
                let country = record[3].to_owned();
                let owner = record[4].to_owned();
                Ok((range_start, range_end, as_number, country, owner))
            })
        })
        .map(|data| {
            data.map(|(range_start, range_end, as_number, country, owner)| {
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
        })
        .flat_map(|data| {
            let mut records = None;
            let mut errors = None;

            match data {
                Ok(data) => records = Some(data),
                Err(err) => errors = Some(AsnCsvParseError::from(err)),
            }

            records.into_iter().flatten().map(Ok).chain(errors.into_iter().map(Err))
        })
}

pub struct AsnDb(Vec<AnsRecord>);

#[derive(Debug)]
pub enum AsnDbError {
    CsvError(AsnCsvParseError),
    FileError(io::Error, &'static str),
}

impl fmt::Display for AsnDbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AsnDbError::CsvError(err) => write!(f, "error opening ASN DB from CSV file: {}", err),
            AsnDbError::FileError(err, context) => write!(f, "error opening ASN DB from file while {}: {}", context, err),
        }
    }
}

impl From<AsnCsvParseError> for AsnDbError {
    fn from(err: AsnCsvParseError) -> AsnDbError {
        AsnDbError::CsvError(err)
    }
}

impl From<ErrorContext<io::Error, &'static str>> for AsnDbError {
    fn from(err: ErrorContext<io::Error, &'static str>) -> AsnDbError {
        AsnDbError::FileError(err.error, err.context)
    }
}

impl AsnDb {
    pub fn form_csv_file(path: impl AsRef<Path>) -> Result<AsnDb, AsnDbError> {
        let mut rdr = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(BufReader::new(File::open(path).wrap_error_while("opending CSV file")?));
        let mut records = read_asn_csv(&mut rdr).collect::<Result<Vec<_>, _>>()?;
        records.sort_by_key(|record| record.ip);
        Ok(AsnDb(records))
    }

    pub fn lookup(&self, ip: Ipv4Addr) -> Option<&AnsRecord> {
        let index = self.0.upper_bound_by_key(&ip.into(), |record| record.ip);
        if index != 0 {
            let record = &self.0[index - 1];
            if record.network().contains(&ip) {
                return Some(record)
            }
        }
        None
    }
}
