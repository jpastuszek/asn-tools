use asn_db::*;
use asn_tools::default_database_cache_path;
use cotton::prelude::*;
use flate2::read::GzDecoder;
use reqwest::Url;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn cache_db(asn_db: &Db, db_file_path: &Path) -> Result<(), Problem> {
    in_context_of(
        &format!("storing database to file: {}", db_file_path.display()),
        || Ok(asn_db.store(BufWriter::new(File::create(db_file_path)?))?),
    )
}

fn remove_cache_db(db_file_path: &Path) -> Result<(), Problem> {
    if db_file_path.exists() {
        if db_file_path.is_file() {
            std::fs::remove_file(db_file_path)?;
        } else {
            Err(Problem::from_error(format!(
                "{} is not a file",
                db_file_path.display()
            )))?;
        }
    }
    Ok(())
}

fn load_tsv(path: &Path) -> Result<Box<dyn Read>, Problem> {
    let file = BufReader::new(File::open(path)?);
    if let Some(_) = path
        .extension()
        .filter(|ext| ext == &OsString::from("gz").as_os_str())
    {
        Ok(Box::new(GzDecoder::new(file)))
    } else {
        Ok(Box::new(file))
    }
}

fn fetch(url: Url) -> Result<impl Read, Problem> {
    let response = BufReader::new(reqwest::get(url)?);
    Ok(GzDecoder::new(response))
}

#[derive(Debug)]
enum UrlOrFile {
    Url(Url),
    File(PathBuf),
}

impl FromStr for UrlOrFile {
    type Err = Problem;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Url::parse(value)
            .map(UrlOrFile::Url)
            .or_else(|_| PathBuf::from_str(value).map(UrlOrFile::File))
            .problem_while("parsing as URL or file path")
    }
}

/// Downloads the latest TSV file and caches it for use by the lookup tool.
#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    /// Path to the database cache file to update [default: OS dependent location]
    #[structopt(long)]
    database_cache_path: Option<PathBuf>,

    /// File path or HTTP URL to TSV file to build cache from
    #[structopt(
        long = "ip2asn-tsv-location",
        default_value = "https://iptoasn.com/data/ip2asn-v4.tsv.gz"
    )]
    tsv_location: UrlOrFile,
}

fn main() -> FinalResult {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    let db_file_path = args.database_cache_path.map_or_else(
        || default_database_cache_path().problem_while("getting default database cache file path"),
        Ok,
    )?;

    let tsv: Box<dyn Read> = match args.tsv_location {
        UrlOrFile::File(tsv_path) => {
            info!(
                "Loading ip2asn database from TSV file: {}",
                tsv_path.display()
            );
            load_tsv(&tsv_path)
                .problem_while_with(|| format!("loading TSV from: {}", tsv_path.display()))?
        }
        UrlOrFile::Url(tsv_url) => {
            info!("Loading ip2asn database from TSV located at: {}", tsv_url);
            Box::new(
                fetch(tsv_url.clone())
                    .problem_while_with(|| format!("fetching TSV from: {}", tsv_url))?,
            )
        }
    };
    let asn_db = Db::form_tsv(tsv).problem_while("loading ASN database")?;

    info!("Updating cached database file: {}", db_file_path.display());
    remove_cache_db(&db_file_path).problem_while("removing database cache file")?;
    cache_db(&asn_db, &db_file_path).problem_while("creating database cache file")?;

    info!("Update done");
    Ok(())
}
