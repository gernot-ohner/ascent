# 4. OpenJDK example and performance

[PR #9](https://github.com/gernot-ohner/ascent/pull/9) sits above [#7](https://github.com/gernot-ohner/ascent/pull/7). It adds
an application test and measurements using the existing OpenJDK facts. It changes
no provenance library or macro code and adds no dependencies or dataset copy.
The [overall guide](../REVIEW_GUIDE.md) covers the first three layers.

Read the [performance report](../benchmarks/OPENJDK_RESULTS.md) first. Boolean
provenance passes the completed subsets, but its cost rises sharply and the next
subset times out. The [method](../benchmarks/OPENJDK.md) defines what was timed
and what the checks establish.

## Read 437 lines

These sections cover the new analysis, its independent checks and measurement.
Counts include comments and blank lines. This is a separate reading path from
the 499-line guide to the first three PRs.

| Section | Code | Lines | Check |
| --- | --- | ---: | --- |
| Three programs and timer | [analysis.rs:11–70](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/ascent-provenance/examples/openjdk/analysis.rs#L11-L70) | 60 | Same rows and rules; only storage and annotations differ. |
| TSV parser | [facts.rs:47–82](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/ascent-provenance/examples/openjdk/facts.rs#L47-L82) | 36 | Load field is the middle column; duplicate rows get distinct tokens. |
| Closed subsets | [facts.rs:101–130](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/ascent-provenance/examples/openjdk/facts.rs#L101-L130) | 30 | Whole constraint components; field IDs do not connect them. |
| Independent oracle | [facts.rs:132–209](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/ascent-provenance/examples/openjdk/facts.rs#L132-L209) | 78 | Union-find fixed point; no unseeded reflexive pairs. |
| Engine and witness checks | [analysis.rs:80–141](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/ascent-provenance/examples/openjdk/analysis.rs#L80-L141) | 62 | Exact pairs; replay and one-token deletion for every witness. |
| Semantic tests | [openjdk.rs:17–111](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/ascent-provenance/tests/openjdk.rs#L17-L111) | 95 | Exhaustive small worlds and the 14,763-fact real subset. |
| Timeout handling | [openjdk.py:24–36](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/benchmarks/openjdk.py#L24-L36) | 13 | Kill the process group and preserve the failure. |
| Benchmark scheduling | [openjdk.py:89–126](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/benchmarks/openjdk.py#L89-L126) | 38 | Check before timing; serial runs; failures remain visible. |
| Sampling | [openjdk.rs:94–118](https://github.com/gernot-ohner/ascent/blob/codex/provenance-openjdk/extensions/ascent-provenance/ascent-provenance/examples/openjdk.rs#L94-L118) | 25 | One warmup, fresh programs, result counts checked every time. |
| | **Total** | **437** | |

The TSV load order differs from the older example's declaration. Check it
against an actual row: `(destination, field, base)`. The new rule and parser
use that order. A deliberate run with the older order fails the real-data oracle.
The original example remains unchanged.

For subset closure, follow the two variable columns of each fact. A field join
needs an alias between its two bases; its source and destination therefore stay
in that same constraint component. Sharing a field number alone cannot connect
two components.

The oracle checks the encoded analysis, not the Java fact extractor. Witness
replay proves sufficiency and minimality for every returned witness; only the
tiny exhaustive fixtures check completeness across all input worlds. The full
compact check compares partitions without enumerating 541 million pairs.

The main omissions from this reading path are CLI formatting, file loading,
metadata collection and the union-find tests' setup. The report links every raw
sample and records the Boolean timeout. Check its scope before drawing a claim
about OpenJDK scalability: cap 64 includes only 15 of the 509 stores.
