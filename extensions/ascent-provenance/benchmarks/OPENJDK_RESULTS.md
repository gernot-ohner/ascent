# OpenJDK performance report

These are the original PR #9 measurements. The next stack layer changes the
rules and product normalization; see the [optimization report](OPENJDK_INVESTIGATION.md)
for the comparison and the remaining 17,598-fact timeout.

Boolean provenance passes the real-data checks on three closed subsets, up to
15,645 facts. It is already expensive there: 2.32 seconds against 16.8 ms for
stock Ascent with the same explicit pairs, and 3.33 ms with compact `eqrel`.
Adding one more component makes Boolean evaluation exceed the 180-second
process limit. This example exposes a practical limit; it does not establish
that this implementation can handle the full input with provenance.

## What was measured

The input is the repository's 47,069 OpenJDK `java.lang` facts. A cap selects
whole constraint components containing at most that many facts. All three
engines receive the same selected rows. See the [method and commands](OPENJDK.md)
for the rules, column mapping, oracle and selection argument.

Run on 27 September 2026: Apple M1 Pro, 8 cores, 32 GiB RAM, macOS 26.6.2,
Rust 1.85.0, release build. The measured checkout was clean at
[`85fc164`](https://github.com/gernot-ohner/ascent/commit/85fc164565233480fe85b7995860f2943866d720).
Later changes add tests, CI paths, this report and review documentation; the
measured example and runner are unchanged. The metadata records their hashes,
the lockfile hash and all four input hashes.

Each row below contains five fresh-program samples after one warmup. Times
cover `run()`, including input normalization and index construction. Loading,
tagging, checking, inspecting and dropping the result are outside that interval.
The range is the smallest and largest sample, not a confidence interval.
Peak RSS covers the whole process: full-input loading, selection, warmup and
all samples. It is not engine-only memory, and the same peak is repeated on
all five CSV rows for that case. Cases ran serially on one machine.

## Results

| Component cap | Engine | Median ms | Range ms | Process peak MiB |
| ---: | --- | ---: | ---: | ---: |
| 8 | `eqrel` | 3.150 | 3.076–3.420 | 51.8 |
| 8 | Explicit pairs | 7.703 | 7.550–8.060 | 55.8 |
| 8 | Boolean | 159.257 | 154.216–165.823 | 99.9 |
| 32 | `eqrel` | 3.522 | 3.308–3.633 | 54.7 |
| 32 | Explicit pairs | 12.379 | 11.412–13.985 | 63.2 |
| 32 | Boolean | 524.466 | 484.383–576.825 | 108.6 |
| 64 | `eqrel` | 3.332 | 3.233–3.595 | 50.8 |
| 64 | Explicit pairs | 16.772 | 15.999–19.368 | 60.5 |
| 64 | Boolean | 2,317.779 | 2,272.602–2,475.423 | 114.6 |
| 128 | `eqrel` | 3.303 | 3.288–3.320 | 50.8 |
| 128 | Explicit pairs | 25.162 | 24.974–30.456 | 61.4 |
| 128 | Boolean | Check timed out | No samples | Not recorded |
| Full input | `eqrel` | 19.581 | 18.507–20.203 | 57.4 |

The normal run contains caps 8, 32, 64 and the full compact case. The separate
stress run contains cap 128 and another full compact case; that repeat's median
was 17.506 ms. Full-input explicit and Boolean runs were not attempted.

| Cap | Facts | Logical pairs | Minimal witnesses | Most witnesses for one pair | Pairs added by field rules |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 8 | 14,763 | 54,608 | 55,530 | 6 | 14 |
| 32 | 15,318 | 65,130 | 68,468 | 16 | 102 |
| 64 | 15,645 | 72,523 | 81,164 | 20 | 102 |
| 128 | 15,760 | 82,324 | Unknown | Unknown | 102 |
| Full | 47,069 | 541,214,413 | Not measured | Not measured | 100,775,707 |

Witness counts are totals across result pairs. Compact storage represents the
full pair count without allocating a row for each pair. The last column compares
the oracle's final pair count with its allocation/assignment-only count.

## What this tells us

Replacing compact equivalence classes with explicit pairs costs 2.4–5.0 times
as much on the three completed subsets. Boolean provenance adds another
20.7, 42.4 and 138.2 times over explicit pairs. Both changes matter.

From cap 32 to 64, Boolean time grows 4.4 times while the final witness count
grows 19%. Cap 64 has only 1.12 witnesses per pair on average. Final output size
alone does not explain the cost. These measurements do not separate intermediate
products, repeated joins, cloning and witness minimization; that needs profiling.

Cap 128 adds one 115-fact component with 99 variables: 36 allocations and
79 assignments. The resulting equivalence class contributes another 9,801 pairs.
The Boolean check reached evaluation but did not finish before the 180-second
process deadline. The [phase log](openjdk/2026-09-27/stress/check-128-boolean.log)
contains the start message and no evaluation-complete message. Witness replay
had not begun. This is a timeout observation, not a completed runtime sample;
its final witness count and peak memory are unknown. The runner records the
failure, skips timing that engine and exits nonzero. Both baselines still pass.

The subsets are biased toward small, weakly connected constraints. Cap 64 keeps
about a third of all facts but only 15 of 509 stores. It excludes the largest
connected region. The results justify using the example as a regression test
and a workload for profiling. They do not justify extrapolating Boolean time
or memory to the full input, nor do they measure a future expression backend.

## Evidence and reproduction

All completed engines match an independent union-find fixed point. Every emitted
Boolean witness is sufficient under replay and fails when any one token is
removed. Exhaustive worlds check completeness on a seven-row fixture and a
five-row real `ProcessBuilder.command` component. Large-subset completeness is
not checked by enumerating all possible input worlds.

Raw files are committed with this report:

- Normal run: [samples](openjdk/2026-09-27/normal/samples.csv),
  [checked counts](openjdk/2026-09-27/normal/checks.csv),
  [metadata](openjdk/2026-09-27/normal/metadata.json),
  [status](openjdk/2026-09-27/normal/status.json).
- Stress run: [samples](openjdk/2026-09-27/stress/samples.csv),
  [checked counts](openjdk/2026-09-27/stress/checks.csv),
  [metadata](openjdk/2026-09-27/stress/metadata.json),
  [status](openjdk/2026-09-27/stress/status.json).

Run the commands in [OPENJDK.md](OPENJDK.md) to reproduce the checks and collect
new measurements. CI checks semantics and timeout handling without time thresholds.
