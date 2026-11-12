use std::io::{self, Read};

use super::Classification;

/// Bounds on how much of one artifact is inspected. 
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InspectionLimits {
    /// Maximum number of records to inspect.
    pub max_records: Option<usize>,
    /// Maximum number of bytes to read from the artifact.
    pub max_bytes: Option<u64>,
}

impl InspectionLimits {
    /// Inspects whole artifacts, no matter artifact size. 
    pub const UNLIMITED: Self = Self { max_records: None, max_bytes: None};
}

/// Reason the artifact's inspection ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Entire artifact was successfully read.
    EndOfInput,
    /// [`InspectLimits::max_records`] was reached.
    RecordLimit,
    /// [`InspectionLimits::max_bytes`] was reached.
    ByteLimit,
    /// Reading or parsing failed while running.
    Error(String),
}

/// Result of inspecting one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    /// Evidence collected per content type and location.
    pub classifications: Vec<Classification>,
    /// Number of records inspected.
    pub records: usize,
    /// Number of bytes read from the artifact.
    pub bytes: u64,
    /// Reason inspection ended.
    pub stop: StopReason,
}

impl Inspection {
    /// Returns status of whole artifact inspection. 
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.stop == StopReason::EndOfInput
    }
}

/// Report visitor returns to the classifier.
#[derive(Debug)]
pub struct Visit {
    /// Evidence collected per content type and location.
    pub classifications: Vec<Classification>,
    /// Number of records that were inspected.
    pub records: usize,
    /// Reason the visitor stopped.
    pub stop: StopReason,
}

/// Structure for tracking budget of bytes to use with reader.
pub struct BudgetReader<R> {
    /// The reader
    inner: R,
    /// Bytes currently read out.
    bytes_read: u64,
    /// Maximum number of bytes to hand out (can be unlimited).
    max_bytes: Option<u64>,
    /// Reader budget limit reached.
    limit_hit: bool,
}

impl<R> BudgetReader<R> {
    /// Wraps the reader with the optional budget.
    pub const fn new(inner: R, max_bytes: Option<u64>) -> Self {
        Self {
            inner,
            bytes_read: 0,
            max_bytes,
            limit_hit: false
        }
    }
    /// Returns number of bytes read currently.
    pub const fn bytes_read(&self) -> u64 {
        self.bytes_read
    }
    /// Returns whether budget has been reached.
    pub const fn limit_hit(&self) -> bool {
        self.limit_hit
    }
}

impl<R: Read> Read for BudgetReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let allowed = match self.max_bytes {
            Some(max_bytes) => {
                let remaining = max_bytes.saturating_sub(self.bytes_read);
                if remaining == 0 {
                    self.limit_hit = true;
                    return Ok(0);
                }
                usize::try_from(remaining).unwrap_or(usize::MAX).min(buf.len())
            }
            None => buf.len()
        };
        let read = self.inner.read(&mut buf[..allowed])?;
        self.bytes_read += u64::try_from(read).unwrap_or(u64::MAX);
        Ok(read)
    }
}