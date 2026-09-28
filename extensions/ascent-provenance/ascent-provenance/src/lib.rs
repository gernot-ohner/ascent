//! Provenance values and macros for stock Ascent.
mod why_provenance;
pub use ascent_provenance_macros::{provenance, provenance_run};
pub use why_provenance::{BooleanProvenance, WhyProvenance};
