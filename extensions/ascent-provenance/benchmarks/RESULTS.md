# September 26, 2026: normalization and explanation scaling

Compared [baseline `bd8ec04`](https://github.com/gernot-ohner/ascent/tree/bd8ec0498272d2d7383b55646fc08ba5a255b004)
with the initial readiness changes using the identical
`scaling.rs` harness. Machine: Apple M1 Pro, 8 CPU cores, 32 GiB memory,
macOS 26.6.2. Compiler: rustc 1.85.0 (4d91de4e4), release profile, locked registry
Ascent 0.8.1. Each row is the median of five fresh-instance samples after one
warmup. Baseline and updated processes ran sequentially, with no other task
build or test running during measurement. The earlier sandbox attempt failed
in the macOS memory-reporting utility; it produced no retained measurement.

## Distinct annotated inputs

Time includes normalization and stock index construction, excluding preparation
of input vectors and output checks. There are no derivation rules in this case.

| Input rows | Baseline ms | Updated ms | Speedup |
| ---: | ---: | ---: | ---: |
| 1,000 | 0.288 | 0.140 | 2.06× |
| 2,000 | 1.655 | 0.303 | 5.46× |
| 4,000 | 3.901 | 0.962 | 4.06× |
| 8,000 | 11.989 | 1.611 | 7.44× |
| 16,000 | 44.444 | 2.759 | 16.11× |
| 32,000 | 173.816 | 7.503 | 23.17× |

The equality-work regression independently rejects the old quadratic scan:
4,096 distinct initialized keys required 8,386,560 comparisons before the fix.
The new check allows a generous linear comparison budget, avoiding wall-clock
assertions in the test suite. Hash collisions still use actual key equality;
the implementation is expected-linear, not a worst-case-linear guarantee.

The updated 32,000-input process peaked at 46.23 MiB RSS. That number includes
setup, warmup and allocator retention across samples, not just the hash index.

## Increasing explanation counts

A chain has two tagged alternatives per step. At depth 12 there are only 13
logical path facts, but the last fact has 4,096 minimal explanations. All these
witnesses are necessary for the requested explicit output, so Boolean absorption
does not shrink this fixture.

| Mode | Depth | Final witnesses | Baseline ms | Updated ms |
| --- | ---: | ---: | ---: | ---: |
| Why | 8 | 256 | 0.095 | 0.098 |
| Why | 10 | 1,024 | 0.405 | 0.409 |
| Why | 12 | 4,096 | 2.155 | 2.131 |
| Boolean | 8 | 256 | 1.865 | 1.883 |
| Boolean | 10 | 1,024 | 26.967 | 27.026 |
| Boolean | 12 | 4,096 | 422.953 | 417.753 |

The normalization change does not materially improve witness-heavy evaluation.
Boolean subset checking is especially costly on many incomparable witnesses;
four times as many final explanations here costs roughly fifteen times as much.
Both explicit witness enumeration and Boolean minimization remain scalability
limits. At depth 12 updated process peak RSS was 6.95 MiB for why and 7.33 MiB
for Boolean, subject to the process-level qualification above.

Raw timings, extrema and all smaller cases are in [baseline.csv](baseline.csv)
and [current.csv](current.csv). Results are descriptive synthetic measurements,
not throughput guarantees. Larger tokens, joins, duplicate distributions,
adversarial hashes, and deeper derivations need their own measurements.
