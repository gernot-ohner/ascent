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

The recorded pre-optimization baseline is published at immutable commit
[`bd8ec0498272d2d7383b55646fc08ba5a255b004`](https://github.com/gernot-ohner/ascent/tree/bd8ec0498272d2d7383b55646fc08ba5a255b004)
on the fork's `perf/external-provenance-benchmark-baseline` reference branch.
It is a standalone workspace at the archive root. From this extension workspace,
retrieve that exact source and use the same harness for both versions:

```sh
baseline_dir="$(mktemp -d)"
curl --fail --location \
  https://github.com/gernot-ohner/ascent/archive/bd8ec0498272d2d7383b55646fc08ba5a255b004.tar.gz \
  --output "$baseline_dir/source.tar.gz"
tar -xzf "$baseline_dir/source.tar.gz" --strip-components=1 -C "$baseline_dir"
cp ascent-provenance/examples/scaling.rs "$baseline_dir/ascent-provenance/examples/scaling.rs"
python3 benchmarks/run.py --root "$baseline_dir" --label baseline --samples 5 > "$baseline_dir/baseline.csv"
python3 benchmarks/run.py --label current --samples 5 > "$baseline_dir/current.csv"
```

Do not change the baseline library. Record commit, hardware, compiler, sample
count and results alongside any new comparison. Add `--no-rss` to both runner
commands if macOS sandbox permissions prevent process memory measurement.

These are bounded synthetic probes, not a general application-performance
claim. They exclude compile time, multi-relation realistic joins, large tokens,
adversarial key hashes, mutation, and parallel provenance (unsupported).

See [the September 26 comparison](RESULTS.md), [baseline CSV](baseline.csv) and
[updated CSV](current.csv) for the measured results.
