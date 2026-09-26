# Review guide: external why/Boolean provenance

For [PR #4](https://github.com/gernot-ohner/ascent/pull/4), targeting `master` in
the same fork. This guide asks for a focused review of the semantics and the
external implementation strategy. It selects **491 source lines**, counting
comments, imports and blank lines. The four command lines below bring the total
code reading budget to **495 lines**. All source ranges are pinned to implementation
commit `67fd01d`; this guide adds documentation without changing that code.

**What the change is meant to do**

Two standalone crates turn annotated Ascent rules into ordinary lattice rules
for **unmodified registry Ascent 0.8.1**. The external macro handles syntax and
annotation propagation. Stock Ascent owns storage, indexes, joins, scheduling
and execution. The extension is a nested, independent Cargo workspace; the
parent compiler, runtime and workspace are unchanged.

An explanation is a set of application-supplied input tokens. Joint premises
combine their token sets; alternative derivations produce alternative sets.

| Mode | Meaning | Example |
| --- | --- | --- |
| Why, the default | All distinct derivation-support sets, including supersets | `{{a}, {a,b}}` retains both witnesses. |
| Positive Boolean | Only inclusion-minimal supports, representing a monotone Boolean function | `{{a}, {a,b}}` becomes `{{a}}`, equivalent to `a OR (a AND b) = a`. |

Zero has no witnesses; one contains the empty witness. Ordinary premises are
fixed background and contribute one. Both modes discard derivation multiplicity
and repeated token use. **This is not how-provenance:** `2a²b` and `ab` both become
the single witness `{{a,b}}`.

**Read these sections in order, then stop**

The links select exact ranges; the rest of each file is outside this reading
assignment. Items 1 and 7 are tests, so the path includes observable behavior as
well as implementation.

| Step | Source section | Lines | What to check |
| --- | --- | ---: | --- |
| 1. One complete example | [tests.rs:92-115](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/tests.rs#L92-L115) | 24 | A diamond graph lowers to stock Ascent and yields exactly the two expected route witnesses. |
| 2. Annotation algebra | [why_provenance.rs:1-127](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance/src/why_provenance.rs#L1-L127) | 127 | Alternative union, pairwise token-set product, zero/one, Boolean absorption, ordering and change reporting. |
| 3. Macro entry | [lib.rs:23-36](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/lib.rs#L23-L36) | 14 | Parse/includes, expand local rule macros and disjunctions, then lower and emit the wrapper. |
| 4. Rule translation | [lower.rs:13-155](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L13-L155) and [lower.rs:173-184](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/lower.rs#L173-L184) | 155 | Tracked relations become lattices; premises acquire annotations; each tracked head receives their product. |
| 5. Input normalization | [normalize.rs:5-32](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/normalize.rs#L5-L32) | 28 | Drop zero rows and join duplicate logical keys before stock indexing, preserving first-seen order. |
| 6. Execution boundary | [wrapper.rs:103-180](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance-macros/src/wrapper.rs#L103-L180) | 78 | Generated named programs invoke stock `ascent!`; inline programs invoke stock `ascent_run!`. Normalization surrounds that boundary. |
| 7. A separate Boolean model | [why_provenance_absorption.rs:1-65](https://github.com/gernot-ohner/ascent/blob/67fd01d0f6ff6bdd18d070b661fb496bccb2ab74/extensions/ascent-provenance/ascent-provenance/tests/why_provenance_absorption.rs#L1-L65) | 65 | All 256 witness families over three tokens reduce to 20 Boolean values; operations and ordering agree with truth tables. Here the alias `Why` means `BooleanProvenance`. |
| **Total source** | | **491** | |

**The invariants behind those sections**

- **Translation:** the intended invariant is one logical tuple paired with its
  accumulated annotation. A rule match multiplies the annotations of tracked
  premises; stock lattice insertion joins alternative contributions to the same
  head tuple. A body with no tracked premises contributes one. Logical arity is checked
  before adding annotation columns. Negation and aggregation are rejected for
  annotated heads after local macro expansion.
- **Algebra:** why join is union of witness alternatives; its lattice meet is
  intersection, which is distinct from rule-body multiplication. Boolean join
  removes absorbed supersets and its meet is conjunction/product. A new row can
  enter stock storage without calling join, so Boolean products must already be
  minimized. The `changed` result matters to fixed-point scheduling.
- **Initialization:** user vectors may contain several annotations for the same
  key or zero annotations. Normalization establishes valid tracked input rows
  before indexing. The hash map locates output positions; it still checks key
  equality. Expected-linear key grouping excludes witness union work and
  adversarial collisions, and stores cloned keys.
- **Lifecycle:** earlier wrapper setup, omitted from the path, generates fresh
  names, preserves public `Self` paths and arranges initializer evaluation once
  in relation-name order. The selected section normalizes named defaults and
  caller-assigned inputs before first execution. Unchanged reruns retain the
  engine. Inline execution remains in the caller's scope and returns owned
  one-shot results. It has no rerun or timeout API.

The diamond test's omitted `run_stock` helper writes the lowered program into
a separate consumer, invokes registry Ascent through `ascent::ascent!`, and
executes the assertion. It does not compare generated text alone. These
invariants are the reasoning to scrutinize, not a claimed correctness proof.

**What the remaining diff contains**

The parser, local-rule-macro expansion and Rust expression visitors are
extracted/adapted frontend code. That is a maintenance dependency on Ascent
0.8.1, not a second evaluator. The omitted `self_paths.rs` handles named-program
Rust scope compatibility. Tests, examples, the ProvSQL adapter, two lockfiles,
CI and documentation account for much of the remaining diff. This focused path
does not constitute an audit of the omitted parser/hygiene code. The
[source attribution](THIRD_PARTY_NOTICES.md) identifies its origins.

**Evidence already obtained, and its limits**

The [verification record](VERIFICATION.md) reports 62 passing workspace tests
after the review fixes. CI includes exhaustive bounded algebra tests, Boolean
reachability on all 512 directed three-node graphs in both input orders, small
acyclic why cases, and independent consumer/compatibility checks.

A separate, manually run [ProvSQL comparison](ascent-provenance/examples/provsql/README.md)
passed 264 complete graph-relation comparisons and four shortest-path fixtures.
Why results are compared with `sr_why`; Boolean results are checked against all
input-label truth assignments through `sr_boolean`, then reduced to minimal true
supports. Deliberately wrong/missing witnesses must fail the same comparator.
This is bounded independent-engine evidence, with an adapter we must still trust.

[Recorded release measurements](benchmarks/RESULTS.md) show the normalization
change reducing the 32,000-input case from 173.816 ms to 7.503 ms, including stock
index construction. Explicit explanation counts can grow exponentially. Boolean
minimization adds substantial overhead: the 4,096-minimal-witness fixture takes
2.131 ms in why mode and 417.753 ms in Boolean mode. These are synthetic timings,
not a plain-Ascent overhead comparison or a production scalability claim.

To reproduce selected checks, starting at the repository root:

```sh
cd extensions/ascent-provenance
cargo +1.85.0 test --workspace --locked
python3 ascent-provenance/examples/provsql/compare.py
python3 benchmarks/run.py --samples 5 --no-rss > /tmp/ascent-provenance-perf.csv
```

The ProvSQL command requires Docker; its README records the pinned reference
image. ProvSQL comparisons and timing benchmarks are manual, not current CI gates.

**Scope that affects the review**

Why recursion has an acyclic-derivation reference contract; unsupported cycles
need not produce a diagnostic. Boolean cycles require finite reachable data and
the documented monotonicity assumptions. Crossing an ordinary relation discards
upstream annotations. Arbitrary nonmonotone Rust/custom-lattice behavior is outside
the reference contract. The shortest-path demo explains attainment of fixed
distances, not the absence of a shorter route.

After any relation-storage mutation following execution, construct a fresh named
instance. A timed-out or panicked instance must also be discarded. This is not
incremental maintenance. Parallel provenance, tracked negation/aggregation and
how-provenance are outside this PR.

An expression-based how backend would need to capture distinct grounded rule
matches and premise occurrences before multiplicities disappear. The frontend,
stock integration and tests can be reused; the current witness representation
cannot recover that information. Such a backend is future work, not validated
by the passing why/Boolean tests.

**Most useful feedback**

1. Does the annotation algebra plus this lowering support the stated semantics,
   particularly first insertion, multiple contributions and recursive scheduling?
2. Is the external lattice-based route a reasonable boundary for these modes,
   given the frontend/version coupling and lifecycle restrictions?
3. Is there a small counterexample or missing independent check that would most
   efficiently challenge the claimed contract, or a frontend decision that would
   obstruct later capture of rule matches for how-provenance?
