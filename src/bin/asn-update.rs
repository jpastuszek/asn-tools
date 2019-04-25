use asn_tools::default_database_cache_path;
use cotton::prelude::*;
use std::path::{Path, PathBuf};
use flate2::read::GzDecoder;
use asn_db::*;
use std::ffi::OsString;

fn cache_db(asn_db: &Db, db_file_path: &Path) -> Result<(), Problem> {
    in_context_of(&format!("storing database to file: {}", db_file_path.display()), || {
        Ok(asn_db.store(BufWriter::new(File::create(db_file_path)?))?)
    })
}

fn remove_cache_db(db_file_path: &Path) -> Result<(), Problem> {
    if db_file_path.exists() {
        if db_file_path.is_file() {
            std::fs::remove_file(db_file_path)?;
        } else {
            Err(Problem::from_error(format!("{} is not a file", db_file_path.display())))?;
        }
    }
    Ok(())
}

fn load(path: &Path) -> Result<Box<dyn Read>, Problem> {
    let file = BufReader::new(File::open(path)?);
    if let Some(_) = path.extension().filter(|ext| ext == &OsString::from("gz").as_os_str()) {
        Ok(Box::new(GzDecoder::new(file)))
    } else {
        Ok(Box::new(file))
    }
}

fn request(url: &str) -> Result<impl Read, Problem> {
    let response = BufReader::new(reqwest::get(url)?);
    Ok(GzDecoder::new(response))
}

/// Downloads latest TSV file and caches it for use by other tools
#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    /// Path to database cache file to update; if not given default OS dependent location will be used
    #[structopt(long = "database-cache-path")]
    database_cache_path: Option<PathBuf>,

    /// Path to TSV file to build cache from; if not given file will be downloaded from https://iptoasn.com
    #[structopt(long = "ip2asn-tsv-path")]
    tsv_path: Option<PathBuf>,

    /// URL to TSV file containing ip2asn database for update
    #[structopt(long = "ip2asn-tsv-url", default_value = "https://iptoasn.com/data/ip2asn-v4.tsv.gz")]
    tsv_url: String,
}

fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    let db_file_path = args.database_cache_path.unwrap_or_else(|| default_database_cache_path().or_failed_to("get default database cache file path"));

    let tsv: Box<dyn Read> = if let Some(tsv_path) = args.tsv_path {
        info!("Loading ip2asn database from TSV file: {}", tsv_path.display());
        load(&tsv_path).or_failed_to(format!("load TSV from: {}", tsv_path.display()))
    } else {
        info!("Loading ip2asn database from TSV located at: {}", args.tsv_url);
        Box::new(request(&args.tsv_url).or_failed_to(format!("request TSV from: {}", args.tsv_url)))
    };
    let asn_db = Db::form_tsv(tsv).or_failed_to("load ASN database");

    info!("Updating cached database file: {}", db_file_path.display());
    remove_cache_db(&db_file_path).or_failed_to("remove database cache file");
    cache_db(&asn_db, &db_file_path).or_failed_to("create database cache file");

    info!("Update done");
}
