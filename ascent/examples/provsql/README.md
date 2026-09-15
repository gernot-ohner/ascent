# Compare why and Boolean provenance with ProvSQL

From the repository root:

```sh
python3 ascent/examples/provsql/compare.py
```

Requires Python 3, Rust 1.85.0 and a running Docker engine. The runner builds
the comparison example and uses unmodified ProvSQL 1.12.0 with PostgreSQL 17.10
(linux/amd64). Every run prints the actual versions and the pinned image:

```text
inriavalda/provsql@sha256:58b7ad6acacfd769d8898a8a27603743ca461c9e9a56da93d3ff049a9c28148c
```

The release source is
[`efe8fe0`](https://github.com/PierreSenellart/provsql/tree/efe8fe0f03ac8b30fb53c93daf2acd7f0e3b8d42).
Docker Desktop on Apple Silicon uses architecture emulation.

## Coverage

One shared [Ascent rule body](../provsql_compare.rs) is instantiated with
`WhyProvenance` and `BooleanProvenance`. Equivalent [SQL queries](queries.sql)
cover all original cases: copy, join, projection, overlapping alternatives,
self-join, non-absorption, product of alternatives and no match. Additional
queries cover recursive reachability, recursive suffixes to a fixed destination,
and the empty identity witness from ordinary background.

The graph datasets are:

- The diamond from [edges.tsv](edges.tsv), the complete fixture (a self-loop
  and duplicate logical edge with distinct labels), its reversed ordering,
  and empty input.
- A two-node cycle with a route to the destination and a disconnected component.
- Duplicate physical rows sharing an application label; labels need not be
  consecutive or start at one.
- Before/after snapshots replacing one route with two alternatives. Each
  snapshot starts a fresh Ascent process and isolated SQL transaction.
- Four generated five-edge graphs, alternating DAGs and unrestricted directed
  graphs, reproducible with Python's random seed `731`.

All original nonrecursive cases run in both modes on every dataset, including
cyclic inputs. Recursive why comparisons run only on directed acyclic graphs
(DAGs); the adapter validates that precondition. On cyclic fixtures the recursive
rules are disabled for why, while the nonrecursive queries still run.
Boolean recursion runs on every graph.

The runner also calls the actual
[`shortest_path_with_why` demo function](../why_provenance_shortest_path.rs).
It compares complete distances and minimal witnesses for ties, a longer route,
a zero-cost self-loop, a two-node zero-cost cycle, a loop at the destination,
disconnected/unreachable sources and empty input. Destination-to-itself produces
the empty suffix. Independent reverse Dijkstra computes nonnegative shortest
distances and tight edges for the SQL reference; it computes no witnesses.
An acyclic tight-edge fixture is additionally compared with direct `sr_why`.

## Reference adapter

For ordinary why, the reference uses provenance class `semiring` and evaluates
saved query roots with `sr_why`. Neither result is minimized.

For Boolean mode, the reference uses class `boolean`. Each query is materialized
once with provenance rewriting active. Rewriting is then disabled for ordinary
mapping updates and evaluation of those same saved roots. For every subset of
the fixture's distinct input labels, the runner evaluates every root with
`sr_boolean` and a complete mapping containing explicit true **and false** values.
All physical input UUIDs sharing an application label receive the same value.
Every reachable circuit input leaf must be mapped, and the mapping must preserve
the physical input-label multiplicities.

The adapter derives inclusion-minimal true subsets from the complete truth
table and compares those sets exactly with Ascent's Boolean witnesses. Fixtures
have at most six labels, hence at most 64 valuations. Valuations are batched into
one SQL session per fixture/mode. The fixed query, data and ordinary background
never change across valuations. No Python provenance evaluator or probability
API is used.

ProvSQL's internal circuit columns are consumed explicitly: saved UUIDs are
projected as text under a separate column name because the extension hides its
`provsql` column in ordinary query output. UUIDs are validated, then excluded
from witness equality. The standalone background identity uses ProvSQL's
`gate_one()`, without inventing an input label.

Every comparison checks the entire tuple/witness map. Ordering alone is
normalized; duplicate tuples, duplicate witnesses/tokens, unknown labels,
malformed output, missing sections and incomplete mappings fail. The startup
checks exercise these parser/mapping failures and DAG rejection. Controlled
wrong/missing answers and a nonminimal Boolean witness must fail the same exact
comparison used for the engines. These controls alter only local test values.

## Isolation and limits

The runner starts a fresh container without ports, host mounts or network
access, creates a disposable database, and rolls back every fixture transaction.
It removes the container and its test data on normal exit or exceptions; the
downloaded image stays cached. A forced process termination can prevent cleanup.
No existing database is used, and ProvSQL Studio is not started.

This is bounded external evidence for these positive queries and fixtures,
not universal equivalence. Non-absorbing cyclic recursion remains outside the
supported contract; the fixture guard is not a runtime Ascent cycle detector.
The comparison does not establish incremental maintenance, tracked negation or
aggregation, probabilities, how-provenance, parallel provenance, arbitrary Rust
expressions or parity with all SQL features.
