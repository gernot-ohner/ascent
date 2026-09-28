# 1. Provenance values

This PR adds the two lattice values and their tests. It has no provenance
macros. The crate is a separate workspace and uses registry Ascent 0.8.1.

Read these **192 lines**, the values portion of the overall 499-line reading
list, including comments and blanks. Links stay on this
layer's branch even when you open this guide from a later PR.

| Read | Lines | Check |
| --- | ---: | --- |
| [why_provenance.rs:1-127](https://github.com/gernot-ohner/ascent/blob/feature/provenance-values/extensions/ascent-provenance/ascent-provenance/src/why_provenance.rs#L1-L127) | 127 | Join, product, order and Boolean absorption. |
| [why_provenance_absorption.rs:1-65](https://github.com/gernot-ohner/ascent/blob/feature/provenance-values/extensions/ascent-provenance/ascent-provenance/tests/why_provenance_absorption.rs#L1-L65) | 65 | An independent truth-table model checks all three-token Boolean values. |
| **Total** | **192** | |

Why mode keeps all distinct supporting token sets, including supersets.
Boolean mode keeps only minimal sets. Repeated uses of a token and repeated
derivations disappear in both. An expression backend for how-provenance will
need a different value representation.

Check that zero differs from one, that Boolean products are already minimal
before any lattice join, and that each join's change flag agrees with equality.
Why's meet is intersection of alternatives; it is not the rule-body product.

The other algebra test enumerates every value over two tokens and checks
operations against a separate set model, then checks lattice and product laws
over every triple. It is useful backup if an operation looks questionable.

Run from `extensions/ascent-provenance/`:

```sh
cargo +1.85.0 test --workspace --locked
```

Five tests should pass. Explicit sets can be expensive; the third PR contains
measurements. The question here is whether these values express the intended
why and positive Boolean semantics.

[Overall stack guide](https://github.com/gernot-ohner/ascent/blob/perf/provenance-performance/extensions/ascent-provenance/REVIEW_GUIDE.md)
