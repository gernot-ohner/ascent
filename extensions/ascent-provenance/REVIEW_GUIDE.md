# External provenance: review the stack

This stack splits [PR #11](https://github.com/gernot-ohner/ascent/pull/11) into
three buildable layers. PR #11 remains open and unchanged. Layer 3 contains the
same runtime, macros, tests, examples and recorded benchmark data. The split
changes documentation, commit boundaries and CI branch filters. A fourth layer
adds the OpenJDK example and performance measurements.

| Order | PR | Base | Review guide |
| --- | --- | --- | --- |
| 1 | [Provenance values](https://github.com/gernot-ohner/ascent/pull/12) | `master` | [Values](reviews/01-values.md) |
| 2 | [External macros](https://github.com/gernot-ohner/ascent/pull/13) | `feature/provenance-values` | [Macros](reviews/02-macros.md) |
| 3 | [Validation and benchmarks](https://github.com/gernot-ohner/ascent/pull/14) | `feature/provenance-macros` | [Validation](reviews/03-validation.md) |
| 4 | [OpenJDK test and performance](https://github.com/gernot-ohner/ascent/pull/15) | `feature/provenance-validation` | [OpenJDK](reviews/04-openjdk.md) |

Review each PR against its listed base. The second diff contains only what it
adds to the first; the third contains only what it adds to the second. Each tip
has its own passing checks. If both review routes remain open, merge either
this stack or PR #11, not both.

The macros add a witness column to tracked relations and emit ordinary lattice
rules. Registry Ascent 0.8.1 supplies the storage, indexes and evaluator. Why
mode keeps distinct supporting token sets; Boolean mode removes supersets.
Both lose repeated token use and derivation counts. How-provenance needs a
separate representation and capture backend; the frontend may be reusable.

## Read 499 lines

The first three per-PR guides divide this same reading list. You do not need to read another
500 lines for each PR. Counts include comments and blank lines; the source
links point to each layer's branch.

| Layer | Code | Lines | Look for |
| --- | --- | ---: | --- |
| Values | [why_provenance.rs:1-127](https://github.com/gernot-ohner/ascent/blob/feature/provenance-values/extensions/ascent-provenance/ascent-provenance/src/why_provenance.rs#L1-L127) | 127 | Alternatives, products and absorption. |
| Values | [why_provenance_absorption.rs:1-65](https://github.com/gernot-ohner/ascent/blob/feature/provenance-values/extensions/ascent-provenance/ascent-provenance/tests/why_provenance_absorption.rs#L1-L65) | 65 | Independent Boolean truth tables. |
| Macros | [lower.rs:13-155](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L13-L155) and [173-184](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L173-L184) | 155 | Products of premise annotations become head annotations. |
| Macros | [normalize.rs:5-32](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/normalize.rs#L5-L32) | 28 | Duplicate keys join before indexing; zero rows disappear. |
| Macros | [wrapper.rs:103-180](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/wrapper.rs#L103-L180) | 78 | Stock execution, named reruns and one-shot inline results. |
| Validation | [compare.py:206-251](https://github.com/gernot-ohner/ascent/blob/feature/provenance-validation/extensions/ascent-provenance/ascent-provenance/examples/provsql/compare.py#L206-L251) | 46 | ProvSQL evaluation and extraction of minimal true sets. |
| | **Total** | **499** | |

Why's meet intersects alternatives; it differs from the rule-body product.
Boolean meet and product both mean conjunction. A rule with no tracked premises
contributes one, a single empty witness. Ordinary relations are fixed background;
passing through one loses upstream annotations.

The short list omits most parser and hygiene code, helper routines and fixtures.
[Source attribution](THIRD_PARTY_NOTICES.md) identifies code adapted from Ascent.
Keeping that frontend aligned with Ascent is a maintenance cost. Compatibility
tests cover source inclusion, local captures, public `Self` paths and ordinary
custom storage.

Why mode requires acyclic derivations; unsupported cycles need not receive a
diagnostic. Boolean recursion permits cycles with finite reachable data and
monotone rules. Tracked negation and aggregation are unsupported. After changing
stored relations following a run, or after a timeout or panic, use a fresh
program. Inline results run once. See the [README](README.md) for the API limits.

## Evidence and limits

The layers have 5, 58 and 62 workspace tests respectively. Feature tests stay
with their implementation. The independent consumers and diagnostic checks
arrive with the macros. The final layer adds Docker comparisons with pinned
ProvSQL and the benchmark tools. [VERIFICATION.md](VERIFICATION.md) records the
checks and distinguishes fresh runs from historical measurements.

The [recorded benchmark](benchmarks/RESULTS.md) reduced normalization and
indexing of 32,000 inputs from 173.816 ms to 7.503 ms. But 4,096 minimal witnesses
took 2.131 ms in why mode and 417.753 ms in Boolean mode. Explicit output can
grow exponentially; subset checks add cost. These are synthetic measurements,
not evidence of large-application scalability.

The main review questions are whether annotation propagation has a
counterexample, whether the external frontend is worth maintaining, and
whether any of its choices would obstruct a later expression backend.

The [OpenJDK follow-up](reviews/04-openjdk.md) has its own 437-line reading path
and [performance report](benchmarks/OPENJDK_RESULTS.md). It compares compact
`eqrel`, explicit pairs and Boolean provenance on closed real-data subsets.
All completed cases pass independent checks; the next Boolean subset times out.
