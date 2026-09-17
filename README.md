# External provenance for Ascent

Why and positive Boolean provenance implemented as a standalone Rust package.
The macros emit ordinary lattice programs for **unmodified registry Ascent
0.8.1**. Ascent owns storage, indexing, joins, scheduling and evaluation.

**Implementation review remains open:** caller-local values in inline rule
bodies do not yet work through the forwarding wrapper. Captured relation
initializers work. See [verification and the reproducer](VERIFICATION.md).

## Dependencies and entry points

Rust 1.85 or later, edition 2021. Use the canonical dependency names:

```toml
[dependencies]
ascent = { version = "=0.8.1", default-features = false }
ascent-provenance = { path = "../ascent-provenance-extension/ascent-provenance" }
```

Keep `ascent` as a direct dependency: stock generated code refers to that
name. The external macros likewise refer to `ascent_provenance`. Renamed
dependencies are outside the prototype's contract.

```rust
use ascent_provenance::{provenance, WhyProvenance};

provenance! {
    struct Paths;
    #[provenance(&'static str)] relation edge(char, char);
    #[provenance(&'static str)] relation path(char, char);
    path(x, y) <-- edge(x, y);
    path(x, z) <-- edge(x, y), path(y, z);
}

let mut program = Paths::default();
program.edge = vec![
    ('A', 'B', WhyProvenance::token("ab")),
    ('B', 'D', WhyProvenance::token("bd")),
    ('A', 'C', WhyProvenance::token("ac")),
    ('C', 'D', WhyProvenance::token("cd")),
];
program.run();
let explanation = program.path.iter().find(|r| (r.0, r.1) == ('A', 'D')).unwrap();
assert_eq!(explanation.2.witnesses().len(), 2);
let owned_rows = std::mem::take(&mut program.path);
```

Inline execution returns a wrapper after evaluation. Initializers run in the
caller's block and may capture or consume local values:

```rust
use ascent_provenance::{provenance_run, BooleanProvenance};
let rows = vec![(1, BooleanProvenance::token("input"))];
let program = provenance_run! {
    #![provenance(boolean)]
    #[provenance(&'static str)] relation input(i32) = rows;
    #[provenance(&'static str)] relation output(i32);
    output(x) <-- input(x);
};
assert_eq!(program.output[0].1, BooleanProvenance::token("input"));
```

Public exports are `provenance!`, `provenance_run!`, `WhyProvenance<T>`
and `BooleanProvenance<T>`. There is no parallel provenance macro. Stock
`ascent!` and `ascent_par!` can be used alongside these macros.

## Witness meaning

- A witness is a set of input tokens jointly sufficient for a derivation.
  The annotation contains alternative witnesses.
- Default why mode retains every distinct witness, including supersets.
  Its addition is union of alternatives; multiplication unions each pair of
  witnesses. Its lattice meet is intersection of alternatives.
- `#![provenance(boolean)]` selects positive Boolean provenance throughout
  a program. Smaller witnesses absorb supersets; meet is conjunction/product.
- Zero has no witnesses. One contains the empty witness. Ordinary-only rule
  bodies contribute one. Ordinary premises and Rust conditions are fixed
  background. Crossing an ordinary relation loses upstream witnesses.
- Applications supply tokens; token types need not implement `Default`.
  Disconnected rules may use different token types.

Default why's recursive reference contract requires acyclic derivations.
It is not a syntactic ban on recursion, and unsupported cycles are not
guaranteed to produce a diagnostic. Boolean recursion supports cycles when
reachable logical data is finite and ordinary monotonicity conditions hold.

An annotated-head rule cannot contain explicit aggregation or negation,
including through local macros. Ordinary heads may aggregate or negate
annotated relations while ignoring annotations. Witnesses do not explain
absence or aggregate values.

Mixed recursion with ordinary user lattices remains allowed. Consequences
of superseded, nonmonotone background values may remain; arbitrary
nonmonotone Rust/custom-lattice programs are outside the reference contract.

## Initialization, execution and ownership

Named `Default` evaluates initializers once (in stock relation-name order),
merges duplicate logical keys and removes zero annotations, without running
rules. Before the first `run`, assigned annotated vectors are normalized
again. Ordinary/custom-provider storage is owned by the stock engine.

An unchanged rerun retains the complete engine, including private provider
state and accumulated statistics. **After any relation-storage mutation,
construct a fresh wrapper and reload intended inputs.** Mutation includes
sorting or taking output rows. Sort a copy if you intend to rerun.
Do not reload materialized derived outputs as base inputs unless intended.
This is not incremental maintenance.

With `#![generate_run_timeout]`, `run_timeout(Duration)` returns stock
Ascent's completion Boolean. On false, partial rows may be inspected, but
discard the instance and restart from fresh inputs; resumption is not
supported. Discard after a panic too.

The wrapper uses `Deref`/`DerefMut` to forward relation fields:

- Reading, assignment, indexing, iteration and mutation use `program.path`.
- Moving fields directly or destructuring them out does not work. Use
  `std::mem::take(&mut program.path)` for ownership.
- To split mutable borrows, first borrow the engine:

```rust,ignore
let relations = &mut *program;
let (edge, path) = (&mut relations.edge, &mut relations.path);
```

Named programs offer `Paths::summary()`; inline programs offer
`program.summary()`. Borrowed `relation_sizes_summary()` and
`scc_times_summary()` forward to stock Ascent.

A public wrapper necessarily exposes a documentation-hidden engine type as
`Deref::Target`. Explicitly dereferencing and invoking engine methods can
bypass normalization; this is a supported-usage boundary, not access control.
Direct engine construction is not part of the supported API.

Documentation, conditional compilation and lint attributes are supported on
wrapper declarations. Layout-sensitive attributes and arbitrary derives are
rejected. Visibility, generics and separate implementation bounds are retained.
Custom storage providers are supported for ordinary relations, including
program-wide provider selection; annotated relations use stock lattice vectors.

## Language and version coupling

The package extracts selected parsing and local-rule-macro hygiene from
Ascent's frontend, expands disjunctions and lowers annotations before emitting
ordinary Ascent syntax. It does not copy Ascent's index planner or evaluator.
The grammar and `include_source!` callback protocol are coupled to 0.8.1.
Use stock `ascent::ascent_source!` to define reusable sources, including
cross-crate sources. Stock restrictions on includes inside source definitions
remain in force.

Both lockfiles record tested resolution of `ascent`, `ascent_base` and
`ascent_macro`. A downstream application resolves independently; upgrades
require rerunning the compatibility suite.

No how-provenance, probabilities, negative/aggregate provenance, parallel
provenance or new general semiring framework is provided.

## Examples and verification

```sh
cargo +1.85.0 run -p ascent-provenance --example why_provenance
cargo +1.85.0 run -p ascent-provenance --example why_provenance_shortest_path
cargo +1.85.0 test --workspace --locked
cargo +1.85.0 run --manifest-path tests/consumer/Cargo.toml --locked
cargo +1.85.0 run --manifest-path tests/consumer/Cargo.toml --features parallel-stock --locked
python3 tests/compile_fail.py
python3 ascent-provenance/examples/provsql/compare.py
```

The shortest-path demo first computes final distances using stock lattices,
then explains routes following tight edges. Boolean mode preserves minimal tied
witnesses and absorbs redundant zero-cost cycle detours. These witnesses explain
attainment of fixed distances, not the absence of a shorter route.

The [ProvSQL harness](ascent-provenance/examples/provsql/README.md) uses the
pinned independent oracle, complete relation comparisons, complete Boolean
valuations and deliberately wrong-witness controls.

See [VERIFICATION.md](VERIFICATION.md) for executed evidence and outstanding
compatibility work, and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for
source attribution and the MIT notice.
