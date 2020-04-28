[![Latest Version]][crates.io] [![Documentation]][docs.rs] ![License]

Command-line tools for lookup of an IP address for matching ASN information in the database from https://iptoasn.com/.

# Usage

## asn-update

```
asn-tools 0.2.1
Jakub Pastuszek <jpastuszek@protonmail.com>
Downloads the latest TSV file and caches it for use by the lookup tool.

USAGE:
    asn-update [FLAGS] [OPTIONS]

FLAGS:
        --errors-only     Only log errors
        --force-colors    Force colorizing the logger output
    -h, --help            Prints help information
    -V, --version         Prints version information
    -v, --verbose         Verbose mode (-v for INFO, -vv for DEBUG, -vvv for TRACE, -vvvv TRACE all modules)

OPTIONS:
        --database-cache-path <database_cache_path>
            Path to the database cache file to update [default: OS dependent location]

        --ip2asn-tsv-location <tsv_location>
            File path or HTTP URL to TSV file to build cache from [default: https://iptoasn.com/data/ip2asn-v4.tsv.gz]
```

## asn-lookup

This tool can print out (in different formats) records from ip2asn database for matching IP addressed.

```
asn-tools 0.2.1
Jakub Pastuszek <jpastuszek@protonmail.com>
Lookup an IP address in the ASN database.

USAGE:
    asn-lookup [FLAGS] [OPTIONS] [IP]...

FLAGS:
        --errors-only       Only log errors
        --force-colors      Force colorizing the logger output
    -h, --help              Prints help information
    -n, --no-matched-ips    Don't list matched IP addresses
    -V, --version           Prints version information
    -v, --verbose           Verbose mode (-v for INFO, -vv for DEBUG, -vvv for TRACE, -vvvv TRACE all modules)

OPTIONS:
        --database-cache-path <database_cache_path>    Path to the database cache file [default: OS dependent location]
        --input-csv-delimiter <input_csv_delimiter>    Input CSV delimiter [default: ,]
        --input-csv-ip-column <input_csv_ip_column>    Input CSV separator [default: 1]
    -o, --output <output>                              Output format: table, csv, json, puppet [default: table]

ARGS:
    <IP>...    List of IP addresses to lookup (can also be read from stdin, one per line; may be in CSV format where
               the first column is an IP)
```

### Example

```
./asn-lookup 1.1.1.1 8.8.8.8
Network    Country AS Number Owner                            Matched IPs
1.1.1.0/24 US      13335     CLOUDFLARENET - Cloudflare, Inc. 1.1.1.1
8.8.8.0/24 US      15169     GOOGLE - Google LLC              8.8.8.8
```

[crates.io]: https://crates.io/crates/asn-tools
[Latest Version]: https://img.shields.io/crates/v/asn-tools.svg
[Documentation]: https://docs.rs/asn-tools/badge.svg
[docs.rs]: https://docs.rs/asn-tools
[License]: https://img.shields.io/crates/l/asn-tools.svg
