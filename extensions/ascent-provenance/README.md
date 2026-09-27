# Provenance values for Ascent

`WhyProvenance<T>` and `BooleanProvenance<T>` implement Ascent's `Lattice`
trait. This crate uses registry Ascent 0.8.1; the parent Ascent workspace is
unchanged. Rust 1.85 or later is required.

Why values retain every distinct set of input tokens supporting a derivation.
Boolean values retain only inclusion-minimal sets: `{{a}, {a,b}}` becomes `{{a}}`.
Both discard repeated token use and derivation counts, so neither represents
how-provenance. Zero has no witnesses; one has a single empty witness.

Join combines alternatives. Product combines the tokens from two premises.
Why's lattice meet intersects alternatives; Boolean meet equals product.
Explicit witnesses can grow exponentially, and Boolean minimization adds
subset checks.

The [stock-Ascent test](ascent-provenance/tests/runtime.rs) shows how to use
these values in a plain `ascent!` lattice program. This layer supplies values
only. The next PR adds macros that propagate them through annotated rules.

From this directory:

```sh
cargo +1.85.0 test --workspace --locked
cargo +1.85.0 doc --workspace --no-deps --locked
```

The five tests cover the two-token why algebra, three-token Boolean truth
tables, and use with stock Ascent. See the [values review guide](reviews/01-values.md)
and the [overall stack guide](https://github.com/gernot-ohner/ascent/blob/codex/provenance-validation/extensions/ascent-provenance/REVIEW_GUIDE.md).
