Command line tools for looking up IP address for matching ASN information in database from https://iptoasn.com/.

h1. Usage

h2. asn-update
```
asn-tools 0.1.0
Jakub Pastuszek <jpastuszek@protonmail.com>
Downloads latest TSV file and caches it for use by other tools

USAGE:
    asn-update [FLAGS] [OPTIONS]

FLAGS:
        --force-colors    Force colorizing the logger output
    -h, --help            Prints help information
    -V, --version         Prints version information
    -v, --verbose         Verbose mode (-v for Debug, -vv for Trace, -vvv Trace all modules)

OPTIONS:
        --database-cache-path <database_cache_path>
            Path to database cache file to update [default: OS dependent location]

        --ip2asn-tsv-location <tsv_location>
            File path or HTTP URL to TSV file to build cache from [default: https://iptoasn.com/data/ip2asn-v4.tsv.gz]
```

h2. asn-lookup

This tool can print out (in different formats) records from ip2asn database for matching IP addressed.

```
asn-tools 0.1.0
Jakub Pastuszek <jpastuszek@protonmail.com>
Lookup IP in ASN database

USAGE:
    asn-lookup [FLAGS] [OPTIONS] [IP]...

FLAGS:
        --force-colors    Force colorizing the logger output
    -h, --help            Prints help information
    -V, --version         Prints version information
    -v, --verbose         Verbose mode (-v for Debug, -vv for Trace, -vvv Trace all modules)

OPTIONS:
        --database-cache-path <database_cache_path>    Path to database cache file [default: OS dependent location]
    -o, --output <output>                              Output format: table, csv, json, puppet [default: table]

ARGS:
    <IP>...    List of IP addresses to lookup (can also be read from stdin, one per line; may be in CSV format where
               first column is the IP)
```

h3. Example

```
./asn-lookup 1.1.1.1 8.8.8.8
Network    Country AS Number Owner                            Matched IPs 
1.1.1.0/24 US      13335     CLOUDFLARENET - Cloudflare, Inc. 1.1.1.1     
8.8.8.0/24 US      15169     GOOGLE - Google LLC              8.8.8.8     
```