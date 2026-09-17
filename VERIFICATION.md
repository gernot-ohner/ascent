# Verification and open acceptance gate

Branch: `feature/external-provenance`, independent local repository.
The migration is **not yet accepted as complete** because the inline
rule-body capture case below requires a design decision.

## Executed passing checks

| Check | Result |
| --- | --- |
| `cargo +1.85.0 test --workspace --locked` | 49 tests passed; all five migrated provenance test files included. |
| Independent `tests/consumer` application | Passed with its own lockfile and direct registry Ascent dependency. |
| `python3 tests/relocated_consumer.py` | Passed from a temporary source-only copy, independent of original checkout and probes. |
| Consumer with `--features parallel-stock --locked` | Passed; stock parallel and external serial programs coexist. |
| `python3 tests/compile_fail.py` | Token mismatch, direct field move, nonexistent parallel import rejected with expected diagnostics, no macro panic. |
| Diamond and shortest-path examples | Passed; original fixtures and expected witnesses retained. |
| Pinned ProvSQL runner | All 264 graph-relation comparisons, four shortest-path fixtures, extra acyclic tight-edge why comparison, and oracle self-checks passed. |
| `cargo +1.85.0 doc --workspace --no-deps --locked` | Passed. |
| `cargo package --list`, both packages | Inspected: source/tests/examples/license/readme, no build outputs or scratch probes. |
| Original experiment checkout | Clean; HEAD remains `ad05d27c00fb0237a00dc58a8f82c9e35d5ec479`. |

The ProvSQL runner was launched from `/private/tmp`, verifying working-directory
independence. Its existing `ROOT = HERE.parents[2]` still locates the workspace
after migration; only the Cargo package argument changed. SQL, fixtures,
complete false/true Boolean valuations, saved roots, mapping checks,
acyclicity guards and negative controls are preserved.

## Exact dependencies and tools

Both workspace and independent-consumer metadata resolve:

| Package | Version | Source |
| --- | --- | --- |
| ascent | 0.8.1 | `registry+https://github.com/rust-lang/crates.io-index` |
| ascent_base | 0.8.1 | `registry+https://github.com/rust-lang/crates.io-index` |
| ascent_macro | 0.8.1 | `registry+https://github.com/rust-lang/crates.io-index` |
| ascent-byods-rels (workspace tests only) | 0.8.1 | `registry+https://github.com/rust-lang/crates.io-index` |

No path/git override of an Ascent package is used.

- rustc 1.85.0, commit `4d91de4e4`.
- cargo 1.85.0, commit `d73d2caf9`.
- Python 3.11.5; Docker server 24.0.7.
- ProvSQL 1.12.0, source pin `efe8fe0f03ac8b30fb53c93daf2acd7f0e3b8d42`.
- PostgreSQL 17.10, Debian 17.10-1.pgdg12+1, x86_64.
- Image: `inriavalda/provsql@sha256:58b7ad6acacfd769d8898a8a27603743ca461c9e9a56da93d3ff049a9c28148c`.

## Open gate: inline rule captures

```rust,ignore
let limit = 3;
let p = ascent_provenance::provenance_run! {
    relation output(i32);
    output(x) <-- for x in 0..limit;
};
```

Stock `ascent::ascent_run!` compiles and runs this program. The external
wrapper produces E0434, "can't capture dynamic environment in a fn item",
because the rule is inside the generated engine's `run()` method.
Relation **initializers** capturing local values do work and are covered
by tests.

Reproduce with `python3 tests/inline_capture.py`: it first runs the stock
case, then requires the external case to pass. It currently fails on the
external case; it is deliberately not counted among the passing tests.

Task 4's stop condition explicitly requires review when inline captures need
a restriction beyond the written specification. The choice is to revise
inline execution to preserve rule-body captures or explicitly approve
initializer-only capture support. No restriction has been silently accepted.

## Architectural accounting

Source line counts at this checkpoint, kept separate by responsibility:

- Transferred runtime plus adapted lowering: 335 lines.
- Selected syntax/parser: 568 lines; macro expansion and required Rust/token
  helpers: 965 lines (includes retained helper unit tests).
- New wrapper: 152 lines; syntax emission/callback adapter: 90 lines, plus the
  small macro entry-point pipeline.
- Behavioral tests, examples and oracle tooling remain separate from production
  machinery. Additional consumer/diagnostic tests exercise the package boundary.

No evaluator, index planning, dependency graph, pattern/wildcard/negation
desugaring, or compiler HIR/MIR was extracted. Ordinary programs are handed
to stock Ascent after provenance lowering.

## Review findings addressed

- Source inclusion now identifies token boundaries by parser cursor, preserving
  prefixes even when generated tokens share spans.
- Hygiene unit fixtures use distinct source spans; actual downstream macro
  invocations also test nested, head and parameterized macros.
- The parallel consumer fixture uses stock `boxcar::Vec` input storage.
- The one moved-field assertion in the transferred property tests uses
  `mem::take`; expected semantics were not changed.

No remote, publication, upstream PR, or changes to historical experiment PRs.
