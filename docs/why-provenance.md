# Why-Provenance

Why-provenance records which tagged input facts can explain each derived tuple. It is opt-in per relation and is currently available in the serial `ascent!` and `ascent_run!` macros.

## API

Annotate a relation with the input-token type:

```rust
use ascent::{WhyProvenance, ascent};

type FactId = &'static str;

ascent! {
   struct Reachability;

   #[provenance(FactId)] relation edge(&'static str, &'static str);
   #[provenance(FactId)] relation path(&'static str, &'static str);
   relation allowed(&'static str);

   path(x, y) <-- edge(x, y), allowed(y);
   path(x, z) <-- edge(x, y), path(y, z), allowed(z);
}

let mut program = Reachability::default();
program.edge = vec![("A", "B", WhyProvenance::token("edge:A-B"))];
program.allowed = vec![("B",)];
program.run();

let explanations = program.path[0].2.witnesses();
```

The declared logical arity remains two, but public storage for `edge` and `path` contains a final `WhyProvenance<FactId>` field. Construct a tagged input with `WhyProvenance::token(token)`. The token type must implement `Clone + Ord + Hash`. `witnesses()` gives read-only access to the explanations as a deterministic `BTreeSet<BTreeSet<T>>`.

## Semantics

The outer set contains alternative explanations. Each inner set contains the input tokens jointly used by one derivation. Rules multiply annotations by taking pairwise token-set unions, while alternative derivations are retained by outer-set union.

- Zero is an empty outer set: there is no explanation.
- One is an outer set containing one empty inner set: the derivation used no tracked input.
- Reusing a token does not add multiplicity, and witnesses do not record traversal order.
- No Boolean simplification is performed. Both `{a}` and `{a, b}` remain when both are derived.

### Boolean mode

Add `#![provenance(boolean)]` at the beginning of an `ascent!` or
`ascent_run!` program to retain only inclusion-minimal witness sets:

```rust
use ascent::{BooleanProvenance, ascent};

ascent! {
    #![provenance(boolean)]
    struct Paths;
    #[provenance(String)] relation edge(i32, i32);
    #[provenance(String)] relation path(i32, i32);
    path(x, y) <-- edge(x, y);
    path(x, z) <-- edge(x, y), path(y, z);
}

let mut paths = Paths::default();
paths.edge = vec![(1, 2, BooleanProvenance::token("e12".into()))];
paths.run();
```

The setting applies to every provenance-annotated relation in that program;
ordinary relations and user-declared lattices are unaffected. Public annotations
use `BooleanProvenance<T>` instead of `WhyProvenance<T>`, with the same
`token` and `witnesses` interface. Omitting the setting preserves the default.

Boolean absorption merges `{a}` and `{a, b}` into just `{a}`. It can therefore remove
zero-cost-cycle witnesses while retaining tied shortest-route witnesses that
do not contain one another. Multiplication also normalizes its candidates,
including when a tuple is first inserted. The specialized lattice reports
changes by content, not witness count; Ascent's evaluator remains unchanged.

The Boolean lattice's join is disjunction (minimal alternatives), its meet
is conjunction (normalized witness multiplication), and its ordering is logical
implication. This differs from the non-absorbing type's ordinary set-inclusion
ordering and intersection-based meet. These types are not implicitly converted.

In ProvSQL terms, the default mode records the `why` witness alternatives as
sets of application-supplied labels. Boolean mode applies Boolean absorption to
those same labels and retains the inclusion-minimal alternatives (the prime
implicants). The modes do not change or synthesize labels.

### Untracked background

Ordinary relations are untracked background. An ordinary positive clause contributes no tokens to an annotated head, so a rule using only ordinary inputs produces the identity witness. Passing data through an ordinary relation deliberately discards upstream provenance. The supported provenance scope is positive, fixed background: tracked negation and aggregation are not expanded into witness explanations.

Embedded Rust conditions, bindings, patterns, generators, and called functions are supported but untracked. Ascent does not inspect those expressions for external dependencies or purity; their behavior must remain stable during a fixed-point run.

## Inputs and execution lifecycle

Before a run, annotated inputs with the same logical tuple are coalesced by witness union, and zero-annotated rows are removed. Calling `run()` again with unchanged inputs and stable embedded Rust behavior is supported and does not change the result.

Changing or deleting inputs, or changing untracked background data, is not an incremental recomputation or retraction mechanism. Create a fresh program instance for a changed dataset. This fresh-instance lifecycle applies after arbitrary input changes. If `run_timeout` returns `false`, the relations contain a partial computation; discard that instance and start a fresh one rather than resuming it.

Mixed recursion between annotated relations and ordinary application lattices is permitted. Application-lattice values and their merge history remain untracked. Ascent only accumulates consequences: if a lattice value changes so that an earlier condition becomes false, previously derived tuples and witnesses are not retracted. Programs using such mixed recursion must ensure their rules are monotone for the intended interpretation.

## Unsupported constructs

The compiler rejects:

- a `provenance` attribute with a missing or malformed type argument, or duplicate `provenance` attributes;
- `#[provenance(...)]` on a user-declared `lattice`;
- provenance annotations in `ascent_par!` and `ascent_run_par!`;
- a custom `#[ds(...)]` provider on an annotated relation;
- aggregation or negation in a rule with any annotated head, including clauses introduced by an Ascent rule macro; and
- an empty-body fact rule with an annotated head. Initialize that relation with explicitly tagged rows instead.

There is no generic provenance-semiring interface, how-provenance, probability provenance, retraction API, or separate provenance macro. There is no expansion for tracked aggregation or negation.

The runnable [`why_provenance` example](../ascent/examples/why_provenance.rs) shows a small diamond graph with two alternative reachability explanations.

The [ProvSQL comparison](../ascent/examples/provsql/README.md) checks complete witness sets for both modes, including acyclic why recursion, cyclic Boolean recursion, fresh dataset snapshots and the actual shortest-path demo against pinned ProvSQL results.

## Shortest paths

The [`why_provenance_shortest_path` example](../ascent/examples/why_provenance_shortest_path.rs) first computes final `Dual<u32>` distances to a destination and then runs a separate Boolean-provenance computation over distance-tight edges. It uses checked addition, seeds the destination with distance zero and the identity witness, retains tied minimal routes while absorbing zero-cost-cycle witnesses, and reports unreachable sources explicitly.

The resulting token sets explain walks in the distance-tight subgraph; they are not standalone certificates of optimality because they omit order and multiplicity. The finalized distance from the first computation establishes optimality.

## Prototype performance limits

The number of witness alternatives can be exponential in the number of input tokens, even with Boolean absorption. No witness cap is applied. Boolean absorption requires subset checks and is not a performance guarantee. Input coalescing is currently quadratic in the number of annotated input rows because it uses a linear search for each row. The supported fixed-point scope assumes a finite active domain and terminating positive derivations; this prototype favors direct, deterministic semantics over large-instance performance.

Non-Boolean recursive provenance is supported only for acyclic derivations. This restriction is on recursive derivation dependencies, not on nonrecursive queries over cyclic graph data, which remain supported. Boolean recursion includes cyclic inputs because repeated or supersets of existing witnesses are absorbed. Ascent does not automatically reject unsupported cyclic non-Boolean inputs; callers must keep them out of scope.
