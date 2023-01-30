Web scraper for retrieving transaction and account details from [solana explorer](https://explorer.solana.com).

### Build
* Build the program with cargo:
```
cargo build
```
* Install chromedriver from https://chromedriver.chromium.org/downloads  
or more easily for linux-like systems with:
```
sudo apt install chromium-browser chromium-chromedriver
```

### Usage:
* Start up chromedriver:
```
chromedriver --port=4444 --disable-dev-shm-usage
```
* Make an alias:
```
alias dora = cargo run --
```
* Start a scrape:
```
Usage: dora --parse <PARSE> --id <ID>

Options:
  -p, --parse <PARSE>  account|transaction
  -i, --id <ID>        Id of the account|tx to be parsed
  -h, --help           Print help
  -V, --version        Print version

```


