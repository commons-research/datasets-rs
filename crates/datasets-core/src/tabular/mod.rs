/// The Tabular Dataset's dialect
mod dialect;
/// The visitor implementation for tabular data
mod visitor;

pub use dialect::TabularDialect;
pub use visitor::TabularVisitor;
