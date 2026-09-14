pub mod lattice;
mod why_provenance;
#[doc(hidden)]
pub mod util;
pub use lattice::{Dual, Lattice};
pub use why_provenance::{AbsorbingWhyProvenance, WhyProvenance};
