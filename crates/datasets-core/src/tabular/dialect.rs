
/// Metadata about the structure of the tabular table
pub struct TabularDialect {
    /// The delimiter character used for separating columns
    delimiter: u8,
    /// Whether the table includes a header row 
    has_headers: bool,
}