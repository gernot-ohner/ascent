# OpenJDK pointer analysis

This example runs the repository's OpenJDK `java.lang` Steensgaard facts through
three implementations: stock Ascent with compact `eqrel` storage, stock Ascent
with explicit equivalence rules, and those explicit rules with Boolean
provenance. All three use registry Ascent 0.8.1 and the same interned inputs.
The provenance library and macros are unchanged. The [performance report](OPENJDK_RESULTS.md)
contains the measured times, memory, output counts and the cap-128 timeout.

From `extensions/ascent-provenance/`:

```sh
cargo +1.85.0 run -p ascent-provenance --release --locked --example openjdk -- explain
cargo +1.85.0 test -p ascent-provenance --test openjdk --locked
python3 tests/openjdk_runner.py
python3 benchmarks/openjdk.py --output /tmp/openjdk-measurements
# Optional stress case; a failed check returns nonzero and is recorded.
python3 benchmarks/openjdk.py --caps 128 --output /tmp/openjdk-stress
```

The runner requires Python 3 and a POSIX system. It builds the release example,
checks each input, then measures cases serially. Each engine is checked
separately, so a timeout in one cannot hide the other baselines. The output directory must be
new. It contains per-sample CSV, check results, source and data hashes, machine
and compiler details, and process logs. Nonzero exits and timeouts are recorded
as failures; they are never reported as zero time. Each process has a 180-second
limit by default, including its children.

## Input and semantics

The four existing fact files contain 4,261 allocations, 39,607 assignments,
2,692 loads and 509 stores. No dataset copy is added. The files are under
`byods/ascent-byods-rels/examples/steensgaard/openjdk_javalang_steensgaard/`.
The [original artifact](https://zenodo.org/records/3346193) describes their
origin; the runner records hashes of the exact files used here.

The TSV load rows are `(destination, field, base)` and store rows are
`(base, field, source)`. The middle columns are numeric fields; the outer
columns identify variables. The older Ascent example reads loads as
`(destination, base, field)`. This example uses the TSV order explicitly:
`store(x,f,y), load(p,f,q), vpt(x,q)` derives `vpt(y,p)`. It therefore exercises
field propagation and is not a timing reproduction of the older example.
A negative-control run with the older order fails the real-data oracle.

Each source row gets its own token, even when two rows contain the same logical
fact. Tokens are offsets in alloc/assign/load/store file order. The `explain`
command prints the source file and one-based line for each token in an actual
`ProcessBuilder.command` explanation.

A subset keeps complete connected components of the input constraints. Each
fact connects its two variable/object columns; field identifiers are excluded.
The cap counts facts per component. All facts in a selected component remain.
Every rule stays within one such component, including load/store joins, so
outside facts cannot add pairs or witnesses inside it. Caps 8, 32 and 64
retain successively larger components; they are not row-prefix samples.

Full-data here means all 47,069 packaged `java.lang` facts, not the whole JDK.
Full-data `eqrel` runs separately. The runner schedules the full-data case for
`eqrel` only; materializing its pair count is outside this bounded experiment.
Use a positive component cap for those modes.

## Checks and measurement

An imperative union-find fixed point, independent of Ascent and the provenance
operations, supplies the expected equivalence relation. The bounded checks
compare the complete explicit and Boolean pair sets with the oracle. The compact
engine is checked by exact cardinality and every member-to-class-root pair;
together these identify the full equivalence partition. Every Boolean witness is
replayed by itself; removing any one token must destroy its result. This checks
sufficiency and minimality. Exhaustive input subsets establish witness
completeness on a seven-row fixture and a five-row real OpenJDK component.
They do not establish completeness by enumeration on the large subsets.

The full-data check compares compact counts and every member-to-class-root
pair against the independent partition. It never expands the full pair set.
CI runs the 14,763-fact subset, exhaustive small cases and runner failure tests.
There are no wall-clock assertions in CI.

Each measured case has one warmup and five fresh-program samples. Time covers
`run()`, including Boolean input normalization and stock index construction.
Parsing, interning, component selection, input-vector construction, checking,
result inspection and destruction are outside the timed interval. Memory is
macOS process peak RSS, including loading the complete dataset, subset selection,
warmup and all samples. It is not memory allocated solely by the analysis;
other POSIX systems leave RSS blank. All implementations execute serially.

These tests check provenance for the encoded Datalog analysis. They do not
validate the Java fact extractor. The selected components omit the largest
connected region and contain only a small fraction of the stores; their times
cannot predict full-data Boolean performance.

See the [review guide](../reviews/04-openjdk.md) for the code reading path.
