use std::collections::{BTreeMap, BTreeSet};

use csv::{ByteRecord, ReaderBuilder};

use crate::CoreError;

/// Single-byte delimiters supported
const DELIMITERS: [u8; 4] = *b",\t;|";

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

    /// Detects the dialect from a potentially truncated sample.
    ///
    /// # Errors
    /// - Returns [`CoreError`] if sampled records cannot be parsed.
    pub(crate) fn detect_sample(data: &[u8], may_be_truncated: bool) -> Result<Self, CoreError> {
        let mut best: Option<CandidateScore> = None;

        for delimiter in DELIMITERS {
            let sample = complete_records(data, delimiter, may_be_truncated)?;

            let Some(candidate) = CandidateScore::sample(sample, delimiter)? else {
                continue;
            };

            if best.is_none_or(|current| candidate.better_than(current)) {
                best = Some(candidate);
            }
        }

        let delimiter = best.map_or(b',', |score| score.delimiter);

        let sample = complete_records(data, delimiter, may_be_truncated)?;

        let has_headers = best.is_some() && detect_headers(sample, delimiter)?;

        Ok(Self { delimiter, has_headers })
    }
}

impl CandidateScore {
    /// Evaluates consistency of the candidate delimiter
    ///
    /// # Errors
    /// - Returns [`CoreError`] if the sampled records cannot be parsed
    fn sample(data: &[u8], delimiter: u8) -> Result<Option<Self>, CoreError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(false)
            .flexible(true)
            .from_reader(data);

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

        if sampled < 2 {
            return Ok(None);
        }

        let Some((columns, consistent)) = frequencies
            .into_iter()
            .filter(|(columns, _)| *columns >= 2)
            .max_by_key(|(columns, count)| (*count, *columns))
        else {
            return Ok(None);
        };

        if consistent * 5 < sampled * 4 {
            return Ok(None);
        }
        Ok(Some(Self { delimiter, columns, consistent, sampled }))
    }

    /// Whether current candidate is stronger than another candidate
    #[must_use]
    const fn better_than(self, other: Self) -> bool {
        let ours = self.consistent * other.sampled;
        let theirs = other.consistent * self.sampled;
        ours > theirs || (ours == theirs && self.columns > other.columns)
    }
}

/// Estimated guess whether first record contains column names. 
///
/// # Errors
/// - Returns [`CoreError`] if sampled records cannot be parsed
fn detect_headers(data: &[u8], delimiter: u8) -> Result<bool, CoreError> {
    let mut reader = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(data);

    let mut records = reader.byte_records();

    let Some(first) = records.next().transpose()? else {
        return Ok(false);
    };

    if records.next().transpose()?.is_none() {
        return Ok(false);
    }
    
    let mut seen = BTreeSet::new();

    for cell in &first {
        let Ok(cell) = std::str::from_utf8(cell) else {
            return Ok(false);
        };
        let cell = cell.trim();
        if cell.is_empty() || cell.parse::<f64>().is_ok() || !seen.insert(cell.to_ascii_lowercase()) {
            return Ok(false)
        }
    }
    Ok(true)
}

/// Recognizes common tabular column labels
fn looks_like_column_name(value: &str) -> bool {
    let name = value.trim().to_ascii_lowercase();

    matches!(name.as_str(), "id" | "name" | "label" | "value")
}

/// Returns complete-record prefix of a potentially truncated sample.
///
/// # Errors
/// - Returns [`CoreError`] if sample cannot be parsed.
fn complete_records(
    data: &[u8],
    delimiter: u8,
    may_be_truncated: bool,
) -> Result<&[u8], CoreError> {
    if !may_be_truncated {
        return Ok(data);
    }
    let mut reader = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(data);

    let mut record = ByteRecord::new();
    let mut complete_end = 0;
    while reader.read_byte_record(&mut record)? {
        let end = usize::try_from(reader.position().byte()).unwrap_or(data.len());
        if end == data.len() {
            break;
        }
        complete_end = end;
    }
    Ok(&data[..complete_end])
}
