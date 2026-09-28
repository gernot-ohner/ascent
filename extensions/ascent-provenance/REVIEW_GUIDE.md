# External provenance: review guide

This stack adds a standalone `ascent-provenance` package for Ascent 0.8.1.

- Why provenance keeps all distinct supporting input sets produced by derivations.
- Positive Boolean provenance keeps only inclusion-minimal supporting sets.
- The extension translates annotations into ordinary Ascent lattice programs.
  Ascent still does the indexing, joins and evaluation.
- Existing Ascent code is unchanged. The extension copies and adapts some parsing
  and macro-expansion code; see [source attribution](THIRD_PARTY_NOTICES.md).
- Tests compare supported cases with ProvSQL in Docker. Performance tests include
  synthetic workloads and the existing OpenJDK `java.lang` pointer analysis.

## Limitations

- No How provenance, shared provenance expressions or probability evaluation.
  Both modes lose repeated input use and derivation counts.
- No tracked negation or aggregation. No parallel provenance.
- Why mode requires acyclic derivations. Unsupported cycles may go undiagnosed.
  Boolean mode supports cycles with finite reachable data and monotone rules.
- No incremental updates. Use a fresh program after changing stored relations or
  after a timeout or panic. Inline programs run once. See the [API limits](README.md).
- Severe performance problems on interconnected inputs. Explicit witness sets can
  grow exponentially. Intermediate candidates, subset checks and duplicated
  storage add avoidable costs.

How provenance needs a different representation and capture backend. The parser
and Ascent integration may be reusable; the witness sets cannot recover the
information they discard.

## Review order

| Order | PR | Base | Review guide |
| --- | --- | --- | --- |
| 1 | [#12: Provenance values](https://github.com/gernot-ohner/ascent/pull/12) | `master` | [Values](reviews/01-values.md) |
| 2 | [#13: External macros](https://github.com/gernot-ohner/ascent/pull/13) | `feature/provenance-values` | [Macros](reviews/02-macros.md) |
| 3 | [#14: Validation and benchmarks](https://github.com/gernot-ohner/ascent/pull/14) | `feature/provenance-macros` | [Validation](reviews/03-validation.md) |
| 4 | [#15: OpenJDK test and performance](https://github.com/gernot-ohner/ascent/pull/15) | `feature/provenance-validation` | [OpenJDK](reviews/04-openjdk.md) |
| 5 | [#16: Performance improvements](https://github.com/gernot-ohner/ascent/pull/16) | `feature/provenance-openjdk` | [Performance](reviews/05-performance.md) |

Each PR builds on the previous one. Review it against the listed base.
The first three split [PR #11](https://github.com/gernot-ohner/ascent/pull/11)
without changing its runtime, macros or tests. PR #11 remains open and unchanged;
it does not include the OpenJDK example or the later optimizations. It overlaps
the first three PRs, so do not merge both routes.

## Read 499 lines for the core

This is the combined reading list for PRs #12, #13 and #14. Counts include comments
and blank lines. Links point to the branch where each section enters the stack.
The later guides add [437 lines for OpenJDK](reviews/04-openjdk.md) and
[128 lines for the optimizations](reviews/05-performance.md), including the
revised Boolean product.

| Layer | Code | Lines | Look for |
| --- | --- | ---: | --- |
| Values | [why_provenance.rs:1-127](https://github.com/gernot-ohner/ascent/blob/feature/provenance-values/extensions/ascent-provenance/ascent-provenance/src/why_provenance.rs#L1-L127) | 127 | Combine alternatives, multiply witnesses and remove supersets. |
| Values | [why_provenance_absorption.rs:1-65](https://github.com/gernot-ohner/ascent/blob/feature/provenance-values/extensions/ascent-provenance/ascent-provenance/tests/why_provenance_absorption.rs#L1-L65) | 65 | Compare with independent Boolean truth tables. |
| Macros | [lower.rs:13-155](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L13-L155) and [173-184](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L173-L184) | 155 | Multiply premise annotations to form head annotations. |
| Macros | [normalize.rs:5-32](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/normalize.rs#L5-L32) | 28 | Merge duplicate keys and remove zero rows before indexing. |
| Macros | [wrapper.rs:103-180](https://github.com/gernot-ohner/ascent/blob/feature/provenance-macros/extensions/ascent-provenance/ascent-provenance-macros/src/wrapper.rs#L103-L180) | 78 | Run stock Ascent; expose named reruns and one-shot inline results. |
| Validation | [compare.py:206-251](https://github.com/gernot-ohner/ascent/blob/feature/provenance-validation/extensions/ascent-provenance/ascent-provenance/examples/provsql/compare.py#L206-L251) | 46 | Evaluate ProvSQL results and extract minimal true sets. |
| | **Total** | **499** | |

Three details to check while reading:

- Why's lattice meet intersects alternatives; rule-body multiplication combines
  them. Boolean meet and multiplication both mean conjunction.
- A rule with no tracked premises contributes one: a single empty witness.
  Ordinary relations are fixed background. Passing through one loses upstream
  annotations.
- Most parser and hygiene code is outside this reading list. Keeping that copied
  frontend aligned with Ascent is a maintenance cost. Compatibility tests cover
  source inclusion, local captures, public `Self` paths and ordinary custom storage.

## What the tests establish

- All 69 workspace tests passed for PR #16's implementation. They include exhaustive
  small-input tests and Boolean truth-table checks. Consumer and diagnostic checks
  cover the external macro's compatibility with stock Ascent.
- PR #14's pinned ProvSQL run passed 264 complete graph-relation comparisons,
  shortest-path fixtures and negative controls. These checks test agreement on
  the fixtures; they do not prove general equivalence. ProvSQL was not rerun for
  PR #16 because the pinned Docker image was unavailable locally.
- The OpenJDK tests compare result pairs with an independent union-find oracle.
  Every returned Boolean witness in the completed benchmark checks was replayed
  and checked for minimality. Exhaustive small fixtures also check completeness;
  the larger runs do not establish witness completeness.

[VERIFICATION.md](VERIFICATION.md) records the checks and when they ran.

## OpenJDK performance

The example adapts Ascent's existing Steensgaard analysis and uses its checked-in
OpenJDK `java.lang` facts. Subsets contain whole constraint components.
The benchmark compares compact `eqrel`, explicit pairs and Boolean provenance.
PR #15 adds the example; PR #16 changes path extension and Boolean product
normalization. The [current report](benchmarks/OPENJDK_INVESTIGATION.md) includes
the original measurements, optimizations and remaining timeout.

With the optimizations, Boolean evaluation takes about 0.13 seconds for a subset
yielding 81,000 minimal supporting sets, and 9 seconds for one yielding 1.94
million. These are release medians on an M1 Pro. The larger subset includes
larger components with many alternative derivation paths. Evaluation with another,
particularly interconnected component did not finish before a 60-second process
deadline. Ordinary Ascent with explicit pair storage computed the same result
pairs in about 0.09 seconds. That last comparison was a separate single check;
there is no final Boolean witness count for the timed-out run.

The cost lies in computing and storing explanations, including intermediate
candidates that are later discarded. The timeout does not tell us how much work
the final witness count would require. The tested subsets include only 15 of
509 stores; they do not establish full-input scalability.
