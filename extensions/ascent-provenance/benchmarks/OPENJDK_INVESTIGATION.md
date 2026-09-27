# OpenJDK: why the Boolean run timed out

The main problem is the closure rule. It joins two relations that already
contain every known path and all its witnesses. The same support is constructed
through many intermediate vertices and rediscovered after annotation updates.
Product minimization adds another large cost because it visits witnesses in
lexicographic order, often inserting supersets before smaller witnesses replace
them.

Two small changes address those costs. Extending paths through direct edges
gets the formerly failing 15,760-fact case below a second without changing the
provenance library. Processing smaller product candidates first reduces it to
0.407 seconds. The fifth stack layer adds these changes and their tests above PR #9.
The earlier PRs and their measurements remain unchanged.

## Measured changes

Same Apple M1 Pro, Rust 1.85.0 release build, serial runs, one warmup, five fresh
programs per row; cap 512 uses three samples. Time covers `run()`, including
normalization and index construction, but excludes parsing, tagging, checking,
inspection and destruction. The [metadata](investigation/2026-09-27/experiment-metadata.json)
and CSVs below identify the experiments. The baseline is PR #9 at `31d7710`.

| Input | Expected/result witnesses | Original rules and library | Edge extension only | Both changes |
| --- | ---: | ---: | ---: | ---: |
| 115-fact isolated component | 135,460 | 180-second check timeout | 336 ms | Not measured separately |
| 15,645 facts, cap 64 | 81,164 | 2,309 ms | 146 ms | Not measured separately |
| 15,760 facts, cap 128 | 216,624 | 180-second check timeout | 508 ms | 407 ms |
| 16,514 facts, cap 512 | 1,944,910 | Not measured | 12,576 ms | 9,018 ms |

Timeouts are process deadlines, not completed runtime samples. The candidate-order
change alone reduced cap 64 from 2,309 to 1,890 ms, but the isolated component
still failed to finish within a 60-second check deadline. This separates a useful
local improvement from the much larger effect of changing the rule.

A separate run gave the stock explicit baseline the same edge-extension rules.
At cap 128, its median was 9.16 ms, versus 3.23 ms for compact `eqrel` and
476 ms for Boolean edge extension. Thus the rule rewrite removes the dramatic
cliff but leaves substantial provenance overhead. These separate batches differ
slightly in timing; their raw samples remain separate.

The original benchmark runner's 60-second budget was too short for the cap-512
batch of a warmup plus five 12-second evaluations. Its correctness check passed;
the *benchmark batch* timed out. That status remains in the raw record. The
subsequent three-sample batches used a 120-second process limit and completed.

## Counted work

The instrumented original run did not finish. Its last complete checkpoint before
the 45-second deadline can still be compared with the completed rewritten run on
the same 115 facts:

| Operation | Original, partial checkpoint | Edge extension, completed |
| --- | ---: | ---: |
| Witness products | 344,064 | 35,522 |
| Candidate witness combinations | 47,127,551 | 333,266 |
| Subset checks | 1,292,165,140 | 10,551,816 |
| Removed transient witnesses | 20,533,359 | 33,340 |
| Largest retained witness family | 398 | 58 |

The independent final answer contains at most 58 witnesses per pair. The original
checkpoint had spent 15.26 seconds constructing products, 26.27 seconds minimizing
them and 1.85 seconds joining candidates into stored values. These times include
instrumentation overhead and are not a complete profile of the eventual run.
Counters also saw 358 thousand joins but only 40 thousand changes to their target
values: most candidates at that checkpoint added nothing.

There are two levels of repeated work. First, the rule combines all witnesses of
both premise pairs. Second, Ascent's lattice delta identifies an updated row;
our annotation is the entire witness family stored in that row. When one new
witness arrives, later evaluations can multiply the old witnesses again too.
The current `Lattice` interface exposes `join_mut` and a changed flag, without a
witness-level delta operation. This is a mismatch between the granularity of the
stored value and the work needed for provenance.

## Change 1: extend through direct edges

The expensive rule is:

```text
vpt(x,z) <-- vpt(x,y), vpt(y,z);
```

The example retains a separate annotated relation of direct constraints:

```text
edge(x,y) <-- alloc(x,y);
edge(x,y) <-- assign(x,y);
edge(y,p) <-- store(x,f,y), load(p,f,q), vpt(x,q);
edge(y,x) <-- edge(x,y);
vpt(x,y), vpt(x,x) <-- edge(x,y);
vpt(x,z) <-- vpt(x,y), edge(y,z);
```

The last rule extends a path through a direct constraint. For the problematic
component, each direct edge has one input token, so it avoids multiplying two
large families of path explanations. Field-derived edges remain annotated with
their premises. Because fields can depend on `vpt`, the whole program is not
necessarily linear recursion in the formal Datalog sense.

For each possible input world, both programs compute the equivalence closure of
allocation, assignment and enabled field constraints. Every such constraint is
in the original `vpt`; conversely, composing symmetric direct constraints gives
its entire equivalence relation. Applying this argument through the least fixed
point covers field feedback too. Equal results in every input world imply the
same positive Boolean function and the same inclusion-minimal witness sets.

This argument is specific to Boolean provenance. The rewrite changes proof
structure and multiplicity; it is not a promise of identical how-provenance
polynomials. It is an application-level optimization, not a valid automatic
rewrite of every recursive Datalog program.

The stock explicit baseline uses the same rewritten rules, so comparisons of
storage and provenance costs remain meaningful. Compact `eqrel` is unchanged.

## Change 2: minimize smaller candidates first

The original product builds all candidate unions, sorts them lexicographically in
a `BTreeSet`, and inserts them into a minimal family. Each insertion searches
for an absorbing subset and, if it survives, removes its supersets.

Lexicographic order has no useful relationship to subset size. For example,
`{0,220}`, `{1,220}`, ..., `{219,220}` all sort before `{220}`. The algorithm can
spend time retaining and comparing hundreds of candidates that the last one
removes.

The product sorts candidates by cardinality. A later candidate is at least as
large as every existing survivor. It cannot be a strict subset of one, so no
survivor needs removing. Only subset rejection remains; equality also counts
as absorption. The normal lattice join retains its existing algorithm.

This preserves the public API and Boolean semantics. It still constructs the
whole Cartesian product and does not fix witness-level delta propagation. Its
measured gain is 18% on the original cap-64 program and about 28% on the rewritten
cap-512 program. It is useful, but insufficient on its own for the original cliff.

## Validation and limits

All 69 workspace tests pass for both changes. New tests exhaust every input world
of a chained-field fixture and of 100 generated seven-row inputs. Existing tests
cover duplicate source facts, reflexivity, mismatched fields and the real
`ProcessBuilder` component. Every witness in the larger completed runs was replayed
and checked by deleting each token in turn. The isolated component's 135,460
witnesses match the independent simple-path count. Adding that count to the
81,164 witnesses observed at cap 64 gives the cap-128 result of 216,624.
Large-case witness completeness is not generally proved by exhaustive worlds.

The cap-512 run demonstrates a remaining limit: 1.94 million explicit witnesses
still take about nine seconds. The subsets include only 15 of the input's 509
stores. Neither change establishes full-input scalability. Full compact
`eqrel` represents 541 million logical pairs; these changes do not remove the
cost of explicitly storing pairs or enumerating all their witnesses.

These changes leave two broader questions open: whether compact token arrays
and subset indexes can make explicit enumeration practical on larger cases,
and when a shared dependency graph should replace enumeration. Those are
separate projects. A nested expression value alone can keep unrolling recursive
dependencies; a graph backend needs defined fixed-point semantics.

## The next subset still times out

A later probe added a 1,084-fact component, bringing the total to 17,598 facts.
The component has 1,057 assignments and 27 loads, with no stores. The ordinary
engines passed their oracle checks: compact `eqrel` took 4.35 ms and explicit
storage took 91.12 ms for 833,051 pairs. Boolean evaluation did not finish
before the 60-second process deadline. It had reached 3.12 GiB of resident
memory in samples taken approximately every 250 ms. A 4 GiB guard did not
trigger. This is sampled RSS, not an exact peak measurement.

The Boolean log contains the evaluation-start marker and no completion marker;
witness replay had not begun. There is no final witness count. The deadline
covers the entire check process, so it is not a completed evaluation-time sample.
These are single checks, not medians. A separate three-sample cap-64 run of the
same binary had a median of 144.39 ms, with the same pairs and witnesses.

| Component cap | Facts | Independent components | Largest component, facts | Stores |
| --- | ---: | ---: | ---: | ---: |
| 64 | 15,645 | 12,364 | 50 | 15 |
| 128 | 15,760 | 12,365 | 115 | 15 |
| 512 | 16,514 | 12,368 | 272 | 15 |
| 1,200 | 17,598 | 12,369 | 1,084 | 15 |
| Full input | 47,069 | 12,370 | 29,471 | 509 |

The small subsets contain many independent problems. Even cap 1,200 omits
494 of 509 stores. They establish a useful improvement and a remaining limit,
not broad OpenJDK scalability. Ordinary Ascent handles the timed-out subset
easily; explicit provenance computation is the bottleneck. The timeout alone
does not establish how much of that cost is required by the final witness count.

The [probe results](investigation/2026-09-27/larger-subset-1200/results.json),
[metadata](investigation/2026-09-27/larger-subset-1200/metadata.json) and
[phase log](investigation/2026-09-27/larger-subset-1200/boolean.err) preserve the
original observation. Its source was local prototype `3d4851e`; the saved patches
below reconstruct its example and library from PR #9. The published example and library differ only in the library's explanatory
comment relative to that prototype.

## Verification of the published changes

The final code was checked again on the same machine with one warmup and three
fresh-program samples per engine and subset. Each correctness check completed
before any timing samples ran. The metadata records commit `f9a2718` and a dirty
tree: the runtime and example match that commit, while documentation, evidence,
CI configuration and the runner's source-hash list were being prepared. All five
measured source hashes, including the provenance library, are recorded.

| Facts | Compact ms | Explicit ms | Boolean ms | Witnesses | Boolean process peak MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| 15,645 | 3.23 | 9.57 | 131.88 | 81,164 | 128.8 |
| 15,760 | 3.22 | 9.06 | 400.03 | 216,624 | 148.0 |
| 16,514 | 3.46 | 20.61 | 8940.35 | 1,944,910 | 486.7 |

All ten engine checks and all 30 samples passed. Full-input compact `eqrel`
also passed; it represents 541,214,413 pairs without allocating each pair.
Time covers evaluation only. RSS is the whole benchmark process peak, including
loading, warmup and all samples; it is different from the sampled RSS of the
17,598-fact timeout. See [samples](investigation/2026-09-27/final/samples.csv),
[checks](investigation/2026-09-27/final/checks.csv),
[metadata](investigation/2026-09-27/final/metadata.json) and
[status](investigation/2026-09-27/final/status.json).

## Raw evidence

- [First comparison](investigation/2026-09-27/measurements.json): baseline,
  product-order-only and edge-extension runs, five samples each.
- [Larger comparisons](investigation/2026-09-27/larger-measurements.json):
  edge extension versus both changes.
- [Operation counters](investigation/2026-09-27/operation-counts.json): original
  partial run and completed instrumented controls.
- [Common-baseline samples](investigation/2026-09-27/edge-extension-all/samples.csv),
  [checked results](investigation/2026-09-27/edge-extension-all/checks.csv),
  [machine/data metadata](investigation/2026-09-27/edge-extension-all/metadata.json),
  [statuses](investigation/2026-09-27/edge-extension-all/status.json).
- Reconstruct the prototypes with the [rule patch](investigation/2026-09-27/original-to-edge.patch)
  and [product patch](investigation/2026-09-27/product-order.patch) against `31d7710`
  using `git apply --unidiff-zero`.
  The first comparison changed only the Boolean program; its
  [exact rule patch](investigation/2026-09-27/boolean-only-edge.patch) is also saved.
  The common-baseline run rewrote both programs. CSV filenames match the variant
  names. `benchmarks/openjdk.py --caps 64 128 512 --timeout 180 --output NEW_DIR`
  repeats the complete comparison against compact and explicit stock baselines.

### Reproduce the isolated diagnostic

The [path counter](investigation/2026-09-27/count_paths.py) reads the isolated
component from its recorded source-row numbers and independently enumerates
simple paths. It checks that the input has 99 nodes, 115 distinct undirected
edges and no self-loops. For this component, simple paths are exactly the
minimal off-diagonal supports; each incident seed edge gives one reflexive
support. It reproduces 135,460 witnesses and a maximum of 58 per pair.

```sh
python3 benchmarks/investigation/2026-09-27/count_paths.py FACTS_DIRECTORY /tmp/openjdk-component-115
```

The [instrumentation patch](investigation/2026-09-27/instrumentation.patch)
records the actual counter definitions used in the historical probe. In a
throwaway checkout of `31d7710`, apply it with `git apply --unidiff-zero`, then
build the release `openjdk` example and check the isolated fact directory with
cap 128 in Boolean mode. Counter checkpoints are printed every 16,384 products;
a final dump is printed after evaluation. Use an external process deadline.
Apply `boolean-only-edge.patch` as well to reproduce the rewritten control.
Do not apply the product-order patch to these instrumented comparisons: they
measure the rule rewrite with the original product algorithm. Counts at a
wall-clock cutoff can vary; the retained partial snapshot is not a completed run.
Instrumentation adds overhead and is not included in any normal timing sample.
