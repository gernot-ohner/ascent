# External provenance: notes for review

[PR #11](https://github.com/gernot-ohner/ascent/pull/11) adds why and Boolean
provenance through two crates on top of Ascent 0.8.1. The macros add an annotation
column to each tracked relation and translate its rules into ordinary lattice
rules. Stock Ascent handles the joins, indexes and execution. Ascent itself does
not change.

Why mode keeps every distinct set of input tokens used by a derivation. Boolean
mode drops supersets: `{{a}, {a,b}}` becomes `{{a}}`. Neither preserves repeated
use of an input or the number of derivations. For example, `ab`, `a²b` and `2ab`
all give `{{a,b}}`. How-provenance will need a different representation.

The following sections cover **491 lines**, including comments and blank lines.
The links point to commit `67fd01d`, so the line numbers will stay fixed.

| Read in this order | Lines | What it does |
| --- | ---: | --- |
| 1. [tests.rs:92-115](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/tests.rs#L92-L115) | 24 | Runs a diamond graph through stock Ascent and checks both route witnesses. |
| 2. [why_provenance.rs:1-127](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance/src/why_provenance.rs#L1-L127) | 127 | Defines the two annotation types and their operations. |
| 3. [lib.rs:23-36](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/lib.rs#L23-L36) | 14 | Parses the program, expands macros and disjunctions, then translates the rules. |
| 4. [lower.rs:13-155](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L13-L155) and [lower.rs:173-184](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L173-L184) | 155 | Adds annotation columns and multiplies premise annotations to form each head annotation. |
| 5. [normalize.rs:5-32](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/normalize.rs#L5-L32) | 28 | Merges duplicate input keys and removes zero annotations before indexing. |
| 6. [wrapper.rs:103-180](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/wrapper.rs#L103-L180) | 78 | Emits the wrapper and calls stock `ascent!` or `ascent_run!`. |
| 7. [why_provenance_absorption.rs:1-65](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance/tests/why_provenance_absorption.rs#L1-L65) | 65 | Checks Boolean operations against three-token truth tables. The alias `Why` here means `BooleanProvenance`. |
| **Total** | **491** | |

In the diamond test, `run_stock` writes the translated program into a separate
consumer crate, compiles it and runs the assertion. The helper lies outside the
selected range.

For each logical tuple, Ascent joins the annotations from alternative
derivations. Within a rule, the product combines tokens from its tracked
premises. A body with no tracked premises contributes one: a single empty witness.
Why's lattice meet is intersection of alternatives; it is distinct from this
product. Boolean meet and product both mean conjunction.

Two details are easy to miss. First, inserting a new tuple can bypass lattice
join, so Boolean products must already have their supersets removed. Second,
input vectors can contain duplicate keys. The normalizer joins their annotations
before Ascent builds indexes. Its hash map stores cloned keys and uses equality
to resolve collisions.

The wrapper normalizes named-program inputs during `Default` and before the
first run, which also catches inputs assigned by the caller. An unchanged rerun
keeps the same engine. Inline programs run once in the caller's scope and return
owned results. Earlier wrapper code, outside the selected range, handles fresh
names, public `Self` references and initializer order.

Much of the remaining diff is parser and macro-hygiene code adapted from Ascent,
plus tests, examples, lockfiles and documentation. The [attribution note](THIRD_PARTY_NOTICES.md)
lists the source files. Keeping this frontend in sync with Ascent is a maintenance
cost; the extension is currently tied to 0.8.1.

The [verification record](VERIFICATION.md) reports 62 passing workspace tests.
They check the algebra against separate set and truth-table models, and Boolean
reachability against a walk enumerator on all 512 directed three-node graphs.
CI also builds independent consumers. A manual [ProvSQL comparison](ascent-provenance/examples/provsql/README.md)
passed 264 complete relation comparisons and four shortest-path cases. For
Boolean mode, it evaluates every input-label assignment with `sr_boolean` and
compares the minimal true sets. Deliberately wrong or missing witnesses must
fail the comparison.

The main restrictions are:

- Why mode supports recursion only with acyclic derivations. The implementation
  does not reliably detect unsupported cycles. Boolean recursion allows cycles with
  finite reachable data and monotone rules.
- Ordinary relations are fixed background. Passing through one loses upstream
  annotations. Tracked negation and aggregation are unsupported. The shortest-path
  example explains routes attaining a fixed distance, not the absence of a shorter route.
- After changing relation storage following a run, or after a timeout or panic,
  use a fresh instance. There is no incremental maintenance or parallel provenance.

Performance still limits the use of explicit witnesses. In the [recorded benchmarks](benchmarks/RESULTS.md),
normalization and indexing of 32,000 inputs fell from 173.816 ms to 7.503 ms.
But 4,096 minimal witnesses took 2.131 ms in why mode and 417.753 ms in Boolean
mode. Enumerating witnesses can require exponential output; subset checks add
further cost. These measurements use small synthetic workloads.

To run the checks from the repository root:

```sh
cd extensions/ascent-provenance
cargo +1.85.0 test --workspace --locked
python3 ascent-provenance/examples/provsql/compare.py
python3 benchmarks/run.py --samples 5 --no-rss > /tmp/ascent-provenance-perf.csv
```

The ProvSQL command needs Docker. It and the timing benchmarks run manually.

For how-provenance, a later backend would record grounded rule matches and
premise occurrences before their multiplicities disappear. It could reuse the
frontend and stock-Ascent integration. Capturing those matches and building an
expression graph remain future work.

The two questions for review are:

1. Is there a counterexample to the annotation propagation, especially when
   several rules contribute to one tuple or recursion revisits it?
2. Is this a useful external extension, given the frontend coupling and API
   restrictions? Does any part make later capture of rule matches harder?
