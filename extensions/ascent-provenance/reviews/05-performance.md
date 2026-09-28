# 5. OpenJDK and Boolean product performance

[PR #16](https://github.com/gernot-ohner/ascent/pull/16) sits above [PR #15](https://github.com/gernot-ohner/ascent/pull/15).
It makes two small changes in separate commits. Ascent, the external macro and
public APIs are unchanged. The first commit changes the example and adds its
tests; the second changes Boolean product normalization. Supporting reports
and CI configuration follow separately.

Read the [optimization report](../benchmarks/OPENJDK_INVESTIGATION.md) for the
before/after measurements, operation counts and remaining timeout. The older
[report](../benchmarks/OPENJDK_RESULTS.md) remains as the PR #15 baseline.

## Read 128 lines

| Code | Lines | Review question |
| --- | ---: | --- |
| [Explicit and Boolean rules](https://github.com/gernot-ohner/ascent/blob/perf/provenance-performance/extensions/ascent-provenance/ascent-provenance/examples/openjdk/analysis.rs#L25-L53) | 29 | Does extending paths through direct constraints preserve the equivalence closure and its Boolean witnesses? |
| [Boolean product](https://github.com/gernot-ohner/ascent/blob/perf/provenance-performance/extensions/ascent-provenance/ascent-provenance/src/why_provenance.rs#L87-L103) | 17 | Can a later candidate absorb an earlier survivor after sorting by size? |
| [New exhaustive fixtures](https://github.com/gernot-ohner/ascent/blob/perf/provenance-performance/extensions/ascent-provenance/ascent-provenance/tests/openjdk.rs#L113-L147) | 35 | Do chained field constraints and all worlds of the generated inputs agree with the independent oracle? |
| [Existing Boolean truth-table test](https://github.com/gernot-ohner/ascent/blob/perf/provenance-performance/extensions/ascent-provenance/ascent-provenance/tests/why_provenance_absorption.rs#L19-L65) | 47 | Does the product still represent conjunction and retain exactly the minimal witnesses? |
| **Total** | **128** | |

The rule rewrite is specific to Boolean provenance. Allocation, assignment and
field-derived edges generate the same equivalence closure in every input world.
Their minimal supports therefore agree. This does not preserve derivation
multiplicities for a future how-provenance backend. Both explicit and Boolean
baselines use the rewritten rules; compact `eqrel` is unchanged.

Sorting product candidates by cardinality means no later candidate can be a
strict subset of an earlier survivor. Equal sets and supersets are rejected by
the existing subset relation. The generic lattice join is unchanged and still
handles arbitrary arrival order. Zero, one and all three-token Boolean functions
are covered by the existing algebra test.

The fifth layer adds 36 lines of tests. It does not add wall-clock assertions to
CI. The benchmark keeps fresh programs and separates checks from timed runs.
Historical source patches, raw samples and timeout logs make the earlier
measurements reviewable; they are supporting evidence, not additional runtime.

Remaining limits matter: the 16,514-fact case has 1.94 million witnesses, and a
17,598-fact probe still times out after 60 seconds. These changes do not provide
a shared expression representation or establish full-input scalability.

[Overall stack guide](https://github.com/gernot-ohner/ascent/blob/perf/provenance-performance/extensions/ascent-provenance/REVIEW_GUIDE.md)
