# Compare why-provenance with ProvSQL

From the repository root:

```sh
python3 ascent/examples/provsql/compare.py
```

Requires Python 3, the Rust 1.85.0 toolchain, and a running Docker engine.
The first run downloads the official ProvSQL image (about 1 GB). Its digest
is pinned in `compare.py`: ProvSQL 1.12.0 with PostgreSQL 17.10, linux/amd64.
Docker Desktop on Apple Silicon runs it through architecture emulation.

The runner builds the Ascent example and starts PostgreSQL in a fresh container,
without publishing ports, mounting host directories, or enabling container
network access. It creates a disposable database and removes the container and
its test data on exit. The downloaded image remains cached for future runs.
No existing database is used or modified. ProvSQL Studio is not started.

## What is compared

[`edges.tsv`](edges.tsv) supplies the same input rows to both systems. The token
labels are integers; `0`, `1`, `2`, and `3` identify graph nodes. Each dataset
runs these corresponding [Ascent rules](../provsql_compare.rs) and
[SQL queries](queries.sql):

| Example | Behavior checked |
| --- | --- |
| Copy | Preserve annotations; combine duplicate logical input tuples. |
| Join | Multiply evidence, including alternative paths through a diamond. |
| Projection | Combine evidence from distinct rows projected to one tuple. |
| Alternatives | Union two rules, including overlapping matches. |
| Self-join | Collapse repeated tokens but retain different combined witnesses. |
| Non-absorption | Retain a witness and its proper superset. |
| Alternative product | Multiply accumulated alternatives from an intermediate relation. |
| No match | Return an empty relation. |

The four datasets are the plain diamond, the full fixture with a loop and a
duplicate logical edge, that full fixture in reverse order, and empty input.
Every comparison checks the **entire relation and every witness set**, ignoring
only row/set ordering and ProvSQL's internal circuit identifiers. Missing or
extra tuples/witnesses fail the run; duplicate output tuples are rejected.
The runner performs no provenance evaluation of its own.

Two useful results printed by the runner:

- Plain diamond, join from node 0 to node 3: `[[1, 3], [2, 4]]`.
- Full fixture, non-absorption from node 1 to node 3: `[[3], [3, 5]]`.
  Token 5 is the loop: the larger witness is intentionally retained.

## Reference and limits

The reference is ProvSQL's built-in `sr_why`, with its provenance class explicitly
set to `semiring`, not `boolean` or `absorptive`. The release's source revision is
[`efe8fe0`](https://github.com/PierreSenellart/provsql/tree/efe8fe0f03ac8b30fb53c93daf2acd7f0e3b8d42).
See its [semiring documentation](https://provsql.org/docs/user/semirings.html).

These are nonrecursive queries, even when the input contains a loop: the rules
traverse only the explicitly specified number of edges. This comparison does
**not** establish equivalence for cyclic recursion, shortest-path evaluation,
arbitrary Rust expressions, or all possible programs. The existing recursive
tests continue to use an independent graph-walk reference. Agreement on these
examples is reproducible external evidence, not a universal correctness proof.
