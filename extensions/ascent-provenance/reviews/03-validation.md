# 3. Validation and benchmarks

This PR adds the shortest-path example and its four tests, the ProvSQL
comparison, the scaling harness and recorded results. The runtime and macro
code are unchanged from the second PR. Their regression tests stay there.

Read [compare.py:206-251](https://github.com/gernot-ohner/ascent/blob/codex/provenance-validation/extensions/ascent-provenance/ascent-provenance/examples/provsql/compare.py#L206-L251):
**46 lines**, completing the overall 499-line reading list. For each saved
ProvSQL result, this code evaluates every assignment of input labels and keeps
the minimal true sets. Why mode instead calls `sr_why`.

The setup above this section records the query circuits and checks their input
mapping. The code below runs the fixtures and compares complete relations,
including missing or extra tuples. The short list does not cover every line of
the harness; the [oracle README](../ascent-provenance/examples/provsql/README.md)
describes the cases and deliberately wrong-witness controls.

Check two claims. First, the oracle compares the intended semantics: all
distinct token sets for why, and minimal true sets for Boolean. Second, the
performance results separate input normalization from witness growth. The
[results](../benchmarks/RESULTS.md) and [raw CSVs](../benchmarks/current.csv)
are measurements from September 26, not new measurements made while splitting
the PR. They use small synthetic workloads. They do not establish that a large
pointer analysis will fit in memory.

The shortest-path example first fixes distances, then explains routes through
tight edges. It explains attainment of a distance, not why no shorter route
exists. Boolean absorption removes redundant zero-cost cycle detours.

Run from `extensions/ascent-provenance/`:

```sh
cargo +1.85.0 test --workspace --locked
python3 ascent-provenance/examples/provsql/compare.py
python3 benchmarks/run.py --samples 1 --no-rss > /tmp/provenance-smoke.csv
```

Expect 62 workspace tests, 264 complete graph-relation comparisons, four
shortest-path fixtures, an additional acyclic why comparison, and 16 benchmark
rows. The oracle requires Docker and uses a pinned image. The one-sample run
checks that the benchmark still works; use the [benchmark instructions](../benchmarks/README.md)
for timing comparisons. CI runs the Rust tests and consumer checks. Docker
comparisons and timing measurements remain manual.

[Overall stack guide](../REVIEW_GUIDE.md)
