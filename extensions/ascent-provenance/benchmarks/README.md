# Scaling measurements

Run `python3 benchmarks/run.py --samples 5 > benchmarks/current.csv` from the
workspace. Cargo uses release optimization, Rust 1.85.0, the committed lockfile,
and offline registry dependencies. No benchmark dependency is added.

`inputs` measures first execution after loading distinct annotated keys. It
includes normalization and stock index construction, but excludes construction
of input vectors and result validation. Doubling sizes exposes quadratic work.

`why` and `boolean` use a chain with two independently tagged alternatives at
each step. At depth d the final fact has exactly 2^d explanations of d+1 tokens
(including a shared seed token); only d+1 logical path facts exist. Every
explanation is minimal, so Boolean absorption cannot reduce this workload.
These cases measure evaluation, including normalization, with inputs prepared
before timing. They deliberately expose explanation growth on small graphs.

Each case has one untimed warmup and five fresh-instance samples by default.
CSV contains median/minimum/maximum elapsed nanoseconds and result counts.
On macOS the runner also records process peak resident bytes, including input
setup, warmup and allocator retention across samples; it is not live witness
memory or a measure of only the timed region. Other systems leave that column
empty. Runs are serial to avoid benchmark contention. Each case has a 180-second
external timeout; timing out is a failed benchmark, never a zero or a skipped pass.
Use `--no-rss` when macOS sandbox permissions block `/usr/bin/time -l`.

For an identical-harness baseline comparison, copy `scaling.rs` into an archived
baseline's `ascent-provenance/examples/` and pass `--root /path/to/baseline
--label baseline`. Do not change the baseline library. Record commit, hardware,
compiler, sample count and results alongside the comparison.

These are bounded synthetic probes, not a general application-performance
claim. They exclude compile time, multi-relation realistic joins, large tokens,
adversarial key hashes, mutation, and parallel provenance (unsupported).

See [the September 26 comparison](RESULTS.md), [baseline CSV](baseline.csv) and
[updated CSV](current.csv) for the measured results.
