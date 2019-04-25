use app_dirs::*;
use std::path::PathBuf;

const APP_INFO: AppInfo = AppInfo{name: "asn_tools", author: "Jakub Pastuszek"};
const DEFAULT_DATA_FILE: &'static str = "asn-db.dat";

pub fn default_database_cache_path() -> Result<PathBuf, AppDirsError> {
    let mut db_file_path = app_dir(AppDataType::UserCache, &APP_INFO, "asn_records")?;
    db_file_path.push(DEFAULT_DATA_FILE);
    Ok(db_file_path)
}