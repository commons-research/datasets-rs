# datasets-rs

Repository for exploring potential scientific dataset sources, evaluating datasets for suitability and cataloguing these datasets. `datasets-rs` has the goal of discovering relevant scientific datasets with scientific content that may not necessarily be labeled correctly, have incorrect metadata, or be from outside of a specific scientific domain and therefore be overlooked through conventional searching. The repository is separated into sub-crates, where each crate handles a different step in the search and evaluation of potential datasets.

## `datasets-core` 

The core crate for evaluating each dataset, by file and content type. This is where the primary evaluation of a dataset occurs. Additional scientific data types are welcome for addition. 

## `datasets-crawler`

The layer responsible for traversing dataset sources and providing datasets for the `datasets-core` to evaluate. Results of crawls are stored to an sqlite database. Application is accessible via CLI and provides basic logging information that may be piped to log files for additional crawl information.

### Default usage

The crawler is currently configured to default to crawling hugging face:

```bash
cargo run -p datasets-crawler -- --default
```
