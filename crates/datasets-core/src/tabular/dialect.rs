use std::collections::BTreeMap;

use csv::ReaderBuilder;

use crate::CoreError;


/// Single-byte delimiters supported
const DELIMITERS: [u8;4] = [b',',b'\t',b';',b'|'];

/// Number of records to use for dialect inference
const SAMPLE_RECORDS: usize = 20;

/// Metadata about the structure of the tabular table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabularDialect {
    /// The delimiter character used for separating columns
    delimiter: u8,
    /// Whether the table includes a header row 
    has_headers: bool,
}

/// Evidence collected for one delimiter
#[derive(Debug, Clone, Copy)]
struct CandidateScore {
    /// candidate delimiter
    delimiter: u8,
    /// Most common number of fields per record
    columns: usize,
    /// Number of records with column/field count
    consistent: usize,
    /// Total records sampled
    sampled: usize,
}

impl TabularDialect {
        /// Standard CSV configuration.
    #[must_use]
    pub(crate) const fn csv() -> Self {
        Self { delimiter: b',', has_headers: true }
    }
    /// Returns selected delimiter
    #[must_use]
    pub(crate) const fn delimiter(self) -> u8 {
        self.delimiter
    }

    /// Whether first record is a header
    #[must_use]
    pub(crate) const fn has_headers(self) -> bool {
        self.has_headers
    }

    /// Detects dialect of a delimited text artifact
    /// 
    /// # Errors
    /// - Returns [`CoreError`] if records cannot be parsed
    pub(crate) fn detect(data: &[u8]) -> Result<Self, CoreError> {
        let mut best: Option<CandidateScore> = None;

        for delimiter in DELIMITERS {
            let Some(candidate) = CandidateScore::sample(data, delimiter)? else { continue; };
            if best.is_none_or(|current| candidate.better_than(current)) {
                best = Some(candidate);
            }
        }

        let delimiter = best.map_or(b',', |score| score.delimiter);

        let has_headers = if best.is_some() {
            detect_headers(data, delimiter)?
        } else {
            false
        };

        Ok(Self {delimiter, has_headers})
    }
}

impl CandidateScore {
    /// Evaluates consistency of the candidate delimiter
    /// 
    /// # Errors
    /// - Returns [`CoreError`] if the sampled records cannot be parsed
    fn sample(data: &[u8], delimiter: u8) -> Result<Option<Self>, CoreError> {
        let mut reader = ReaderBuilder::new().delimiter(delimiter).has_headers(false).flexible(true).from_reader(data);

        let mut frequencies = BTreeMap::<usize, usize>::new();
        let mut sampled = 0usize;

        for result in reader.records().take(SAMPLE_RECORDS) {
            let record = result?;
            if record.is_empty() {
                continue;
            }
            *frequencies.entry(record.len()).or_default() += 1;
            sampled += 1;
        }

        if sampled < 2{ 
            return Ok(None);
        }

        let Some((columns, consistent)) = frequencies.into_iter().filter(|(columns, _)| *columns >= 2).max_by_key(|(columns, count)| (*count, *columns)) else {
            return Ok(None);
        };

        if consistent * 5 < sampled * 4 {
            return Ok(None);
        }
        Ok(Some(Self {
            delimiter,
            columns,
            consistent,
            sampled
        }))
    }

    /// Whether current candidate is stronger than another candidate
    #[must_use]
    fn better_than(self, other: Self) -> bool {
        let ours = self.consistent * other.sampled;
        let theirs = other.consistent * self.sampled;
        ours > theirs || (ours == theirs && self.columns > other.columns)
    }
}

    /// Estimated guess whether first record contains column names
    /// 
    /// # Errors
    /// - Returns [`CoreError`] if sampled records cannot be parsed
    fn detect_headers(data: &[u8], delimiter: u8) -> Result<bool, CoreError> {
        let mut reader = ReaderBuilder::new().delimiter(delimiter).has_headers(false).flexible(true).from_reader(data);

        let rows = reader.records().take(SAMPLE_RECORDS).collect::<Result<Vec<_>,_>>()?;

        let Some((first, remaining)) = rows.split_first() else {
            return Ok(false);
        };

        if remaining.is_empty() {
            return Ok(false);
        }

        if first.iter().any(looks_like_column_name) {
            return Ok(true);
        }

        for (index, value) in first.iter().enumerate() {
            if value.trim().parse::<f64>().is_ok() {
                continue;
            }
            let values = remaining.iter().filter_map(|row| row.get(index)).filter(|value| !value.trim().is_empty());

            let mut total = 0usize;
            let mut numeric = 0usize;

            for value in values {
                total += 1;
                if value.trim().parse::<f64>().is_ok() {
                    numeric += 1;
                }
            }

            if total >= 2 && numeric * 4 >= total *3 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Recognizes common tabular column labels
    fn looks_like_column_name(value: &str) -> bool {
        let name = value.trim().to_ascii_lowercase();

            matches!(
        name.as_str(),
        "id"
            | "name"
            | "label"
            | "value"
    )
    }