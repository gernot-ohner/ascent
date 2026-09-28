# External provenance readiness

**Goal:** Fix normalization scaling, named-program `Self` references, and inline
rule captures; measure input and witness scaling without changing Ascent.

**Baseline:** `bd8ec04`, registry Ascent 0.8.1, Rust 1.85.0. Existing 49-test
workspace suite, consumer and compile-fail checks passed during this review.

**Execution:** Implement in this session, with one independent review at the end.
User authorized the fixes and benchmarks on September 26. No PR or publication
is requested. Preserve the original checkout and existing experiments.

## Tasks

- [x] Normalization: add an equality-work regression for unique keys and a
  collision/order/zero regression; replace the linear scan with a hash index
  into the output vector. Preserve first-seen order and lattice joining.
- [x] Named `Self`: add real downstream regressions for inherent helpers,
  generic/trait paths, associated types, patterns, and nested impl scopes;
  rewrite user-level `Self` to the public program type before engine renaming.
  Avoid rewriting nested items' own `Self`.
- [x] Inline captures: resolve the API question, then normalize initializers
  before stock `ascent_run!` builds indexes. Cover captured rule expressions,
  initializer side effects/order, source includes and generic outer scopes.
- [x] Benchmarks: add a reproducible release-mode harness for increasing input
  sizes and explanation counts, with independently checked result counts.
  Run the identical harness against the baseline and final implementation.
  Report medians, configuration and limits without claiming general throughput.
- [x] Verification: full workspace tests, inline capture acceptance script,
  compile-fail and independent/relocated consumers, examples and pinned ProvSQL
  comparison when available. Confirm registry dependencies and clean Ascent.
- [x] Update README, verification evidence and the acceptance-gate instructions;
  obtain an independent code review, address concrete findings, and commit.

## Design constraints

Normalization must compare actual keys even when their hashes collide. Use only
the Clone/Eq/Hash bounds already required by stock Ascent; preserve input order.
The second normalization before named execution remains necessary because
callers may assign inputs after Default. Unchanged reruns skip normalization.

`Self` repair must handle user Rust syntax, not generated wrapper internals.
Do not perform blind token replacement across nested trait/impl definitions.

Inline API decision: stock-compatible one-shot results, with reruns and timeout
methods retained for named programs. This recommendation was presented while
independent work continued, then selected as the stated implementation assumption.
The API change and migration path are explicit in README and VERIFICATION.

Keep witness semantics unchanged. Benchmark inherent explanation growth rather
than promising that input normalization solves witness explosion. Boolean
product pruning is a separate potential optimization, outside this change.
