# Verification: external provenance readiness

Verified September 26, 2026 in the standalone package at `bda3860`, based on
`bd8ec04`. That package is imported under `extensions/ascent-provenance/` on
`codex/external-provenance-readiness`, based on fork master `e52c84b`, for review
within `gernot-ohner/ascent`. The former inline-capture acceptance gate is
resolved. Ascent source and its parent workspace remain unmodified; the external
workspace has its own CI workflow. No crate publication is part of this change.

The initial readiness hash `bda3860` identifies the standalone source history.
The pre-optimization baseline `bd8ec04` is now published on the fork's separate
`codex/external-provenance-benchmark-baseline` branch; it is not an ancestor of
this PR. See benchmarks/README.md for immutable source retrieval and replay.
Source attribution and measurement provenance remain in THIRD_PARTY_NOTICES.md
and benchmarks/RESULTS.md.

## Result and API decision

- Named `provenance!` programs retain normalized `Default`, field forwarding,
  unchanged reruns, and optional timeout execution.
- Inline `provenance_run!` now emits stock `ascent_run!`, preserving local rule
  captures and caller-scope `Self`. It returns owned one-shot results. Its old
  wrapper rerun/timeout methods are removed; the timeout-generation attribute
  receives an explicit diagnostic. This is an intentional prototype API change.
- Named `Self` in rule expressions and relation types retains the public program
  type, including generic bounds, trait-associated types and patterns. Nested Rust
  items preserve their own scope. Standard expression macros are supported;
  custom/shadowed macros require explicit program types. `stringify!` keeps
  literal tokens. See README for the macro-opacity boundary.
- Hash-index normalization preserves equality, first-seen order, witness unions
  and zero removal. It replaces quadratic key scanning with expected-linear
  grouping at the cost of storing cloned keys. Adversarial hashes and witness
  union work are not covered by the expected-linear claim.

## Executed checks

| Check | Result |
| --- | --- |
| `cargo +1.85.0 test --workspace --locked --offline` | 62 tests passed, zero warnings. |
| `python3 tests/inline_capture.py` | Both stock and external caller-local rule captures pass. |
| `python3 tests/compile_fail.py` | Seven expected diagnostics pass: custom Self macro forms, inline rerun/timeout API, token mismatch, named field move, removed parallel import. |
| Independent consumer with `--features parallel-stock --locked --offline` | Passes; cross-crate source callbacks and stock parallel coexistence work. |
| `python3 tests/relocated_consumer.py` | Source-only relocated consumer passes independently. |
| Diamond and shortest-path examples | Pass with original expected witnesses. |
| `cargo +1.85.0 doc --workspace --no-deps --locked --offline` | Passes. |
| Fresh pinned ProvSQL comparison | All 264 graph-relation comparisons, four actual shortest-path fixtures, extra acyclic tight-edge why comparison and oracle controls pass. |
| Published baseline replay | Downloaded the immutable GitHub archive; all 16 benchmark smoke cases pass for baseline and current code, with identical tuple/witness counts. Historical timing CSVs unchanged. |
| Workspace and independent-consumer metadata | All three Ascent packages resolve to registry 0.8.1. |
| Original integrated experiment and stock checkout | Clean and unchanged at `ad05d27` and `e52c84b`, respectively. |

The new tests cover initializer and assigned-input normalization, collision
safety and stable order, equality-work scaling, generic named helpers and trait
paths, nested `Self` scopes, literal macro tokens, inline local/generic captures,
owned outputs, initializer capture/order and inline custom-provider state.

Observed regression failures before the respective fixes:

- 4,096 initialized keys caused 8,386,560 equality checks, failing the linear-work
  budget. The optimized implementation passes that budget and the collision test.
- Named helper/trait expressions failed on the generated engine type.
- Inline captures failed with E0434; direct inline output movement also failed.
- Independent review found `stringify!(Self)` changed to the program name.
  Its added regression failed before the macro boundary fix and now passes.

Independent review rechecked the macro fix and found no remaining implementation
blocker within its bounded review. It explicitly recorded the one-shot inline
API, opaque-macro boundary, hash-index memory and explicit witness growth limits.

## PR review corrections

The multi-angle review of PR #4 identified two supported-language compatibility
failures. New regressions reproduced both before their fixes:

- `Self` in declaration and separate implementation bounds referred to the
  hidden engine. The engine copy now rewrites those bounds to the public type;
  the wrapper retains its original signature.
- Qualified standard `core::matches!` with `Self` was rejected as custom syntax.
  A dedicated parser now visits its expression, pattern alternatives and
  optional guard. Tests cover qualified/unqualified forms, trailing commas,
  positive and negative patterns/guards.

An independent reviewer reran both original failing examples after the fixes;
both compile, run and satisfy their output assertions. The full workspace,
downstream diagnostics/consumers and documentation checks also pass.

Macro hygiene now collects statement bindings without computing discarded
immutable free-variable results. The mutable visitor preserves initializer,
shadowing and let-else scopes, covered by an additional regression. A throwaway
instrumented parse/expansion probe measured 25 expression visits at nested-block
depth 20, compared with 5,242,880 before. No instrumentation is shipped. This
removes redundant work in the external frontend; stock Ascent is unchanged.
The external macro crate no longer directly depends on `duplicate`.

The extension now passes `cargo +nightly fmt --all --check` under the inherited
repository configuration. The oracle README names the correct working
directory, and the exact benchmark baseline is publicly retrievable as described
in benchmarks/README.md. The historical timing CSVs are unchanged.

## Performance evidence

The identical release harness was run against `bd8ec04` and the optimized code,
with one warmup and five measured samples per case. At 32,000 distinct input
rows, median first execution fell from 173.816 ms to 7.503 ms (23.17× faster).
Witness-heavy evaluation remains essentially unchanged: 4,096 minimal witnesses
take 2.131 ms in why mode and 417.753 ms in Boolean mode on the measured machine.

See [benchmark methods](benchmarks/README.md), [results](benchmarks/RESULTS.md),
and the raw CSV files. This is bounded synthetic evidence, not a scalability
guarantee for arbitrary graphs or Rust expressions. Boolean product generation
and minimization were not changed in this task.

## Dependencies and reference versions

Both dependency graphs resolve `ascent`, `ascent_base` and `ascent_macro` 0.8.1
from `registry+https://github.com/rust-lang/crates.io-index`. No Ascent path/git
override is used. Test-only `ascent-byods-rels` remains registry 0.8.1.

- rustc 1.85.0, commit `4d91de4e4`.
- ProvSQL 1.12.0, source pin `efe8fe0f03ac8b30fb53c93daf2acd7f0e3b8d42`.
- PostgreSQL 17.10, Debian 17.10-1.pgdg12+1, x86_64.
- Docker image `inriavalda/provsql@sha256:58b7ad6acacfd769d8898a8a27603743ca461c9e9a56da93d3ff049a9c28148c`.

The comparison uses a disposable container with no host mounts, ports or network
and removes it on completion. The historical September 17 verification record
and extraction line counts remain available at `bd8ec04:VERIFICATION.md`.
