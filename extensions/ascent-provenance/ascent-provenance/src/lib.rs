//! Provenance values and macros for stock Ascent.
mod why_provenance;
pub use why_provenance::{BooleanProvenance, WhyProvenance};
pub use ascent_provenance_macros::{provenance, provenance_run};
