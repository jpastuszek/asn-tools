use asn_tools::db_file_path;
use cotton::prelude::*;
use std::path::{Path, PathBuf};
use asn_db::*;

fn cache_db(asn_db: &Db) -> Result<(), Problem> {
    let db_file_path = db_file_path()?;
    info!("Caching DB to: {}", db_file_path.display());
    in_context_of(format!("storing database to file: {}", db_file_path.display()), || {
        Ok(asn_db.store(BufWriter::new(File::create(db_file_path)?))?)
    })
}

fn remove_cache_db() -> Result<(), Problem> {
    let db_file_path = db_file_path()?;
    info!("Removing cached DB file: {}", db_file_path.display());
    std::fs::remove_file(db_file_path)?;
    Ok(())
}

fn download(url: &str, path: impl AsRef<Path>) -> Result<(), Problem> {
    use flate2::write::GzDecoder;
    let path = path.as_ref();
    info!("Downloading ip2asn TSV database from: {} to: {}", url, path.display());
    let tsv_file = File::create(&path).problem_while_with(|| format!("creating ip2asn TSV file at '{}'", path.display()))?;
    let mut tsv_file = GzDecoder::new(tsv_file);
    let mut response = reqwest::get(url)?;
    response.copy_to(&mut tsv_file).problem_while("downloading ip2asn data to TSV file")?;
    Ok(())
}

/// Downloads latest TSV file and caches it for use by other tools
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
}

fn main() {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    download(args.tsv_url.as_str(), &args.tsv_path).or_failed_to(format!("download TSV from: {}", args.tsv_url));

    remove_cache_db().ok();

    info!("Loading DB from TSV: {}", args.tsv_path.display());
    let asn_db = in_context_of(format!("loading database from TSV file: {}", args.tsv_path.display()), || {
        Ok(Db::form_tsv(BufReader::new(File::open(&args.tsv_path)?))?)
    })
    .or_failed_to("load ASN database");

    cache_db(&asn_db).or_failed_to("cache DB file");

    info!("Update done");
}
