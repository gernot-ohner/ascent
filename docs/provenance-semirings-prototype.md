# Provenance semirings prototype

## Scope for review

This branch is a research prototype for evaluating that design. It is not an
upstream-ready proposal. The target is a small, correct implementation with an
isolated API, executable examples, explicit semantic boundaries, and no
regressions in existing Ascent behavior.

## User-facing semantics

`ascent_provenance!` accepts ordinary Ascent relation declarations and positive
rules after one semiring declaration:

```rust
ascent_provenance! {
   semiring HowProvenance<String>;

   struct Reachability;

   relation edge(i32, i32);
   relation path(i32, i32);

   path(x, y) <-- edge(x, y);
}
```

Rules retain their logical arity. Provenance is an implicit value during rule
evaluation, but it is the final field of each public relation row:

```rust
program.edge = vec![
   (1, 2, HowProvenance::token("e12".to_owned())),
];
```

Every rule derivation starts with `one`. Matching a relational body clause
multiplies its annotation into the derivation. Reaching a head adds that
derivation to the annotation already stored for the same logical tuple. A
projection therefore adds source alternatives, a join multiplies jointly used
facts, and distinct rules add alternative derivations.

The public algebra contract is:

```rust
trait ProvenanceSemiring: Clone + Eq {
   fn zero() -> Self;
   fn one() -> Self;
   fn add_assign(&mut self, other: &Self) -> bool;
   fn multiply(&self, other: &Self) -> Self;
}

trait IdempotentConvergentProvenanceSemiring: ProvenanceSemiring {}
```

`add_assign` reports whether an annotation genuinely changed. The evaluator
uses that result to decide whether a tuple must be scheduled again. Although
Rust cannot encode the algebraic laws in a trait bound, implementations used by
the macro must behave as commutative semirings. Implementations of
`IdempotentConvergentProvenanceSemiring` must additionally have idempotent addition and
terminating ascending chains for finite inputs. Idempotence is necessary
because the recursive scheduler reprocesses the tuple's full accumulated
annotation after a change.

At the beginning of `run()`, every relation is normalized. Duplicate logical
tuples are combined with semiring addition and zero-annotated rows are removed.
The generated program rejects a second call to `run()` or `run_timeout()`.
This is deliberately stricter than ordinary Ascent because a second batch run
would double-count how-provenance without a provenance-aware incremental
maintenance model. It omits the deprecated public `update_indices()`
method because indexing unnormalized provenance rows before `run()` would leave
stale row references after coalescing.

## Included domains

`HowProvenance<T>` implements provenance polynomials in `N[X]`. It stores a
canonical ordered map from sorted monomials to arbitrary-precision `BigUint`
coefficients. Coefficients distinguish duplicate derivations, and repeated
tokens retain exponents. For the diamond graph, the two paths produce exactly:

```text
x1*x3 + x2*x4
```

`WhyProvenance<T>` stores a set of witness sets. Addition unions alternative
witnesses. Multiplication takes every pair of witnesses and unions the tokens
required jointly by that pair. It intentionally performs no absorption or
minimal-witness simplification, so both `{x1}` and `{x1,x2}` remain visible if
both are derivable. Coefficient and exponent distinctions disappear because
both outer and inner containers are sets.

Only `WhyProvenance<T>` implements `IdempotentConvergentProvenanceSemiring`. With a finite
input token universe there are finitely many witness sets, so recursive
evaluation reaches a fixed point even for cyclic graphs.
This assumes the rules generate finitely many logical tuples. Rust expressions
and generators can still create unbounded relations; the marker constrains
annotation growth only.

## Compiler and evaluator architecture

The macro shares the existing parse, high-level intermediate representation,
middle-level intermediate representation, and code-generation pipeline.
`ascent_provenance!` parses `semiring Type;`, attaches the type to the program,
and invokes the normal compiler in serial mode.

The former relation/lattice Boolean distinction is represented by a relation
kind with normal, lattice, and provenance variants. A provenance relation keeps
the original field types as its logical key while code generation appends the
semiring type to the physical row. Provenance indexes return row references so
body-clause code can read the hidden annotation without exposing it to rule
syntax.

Head insertion indexes only the logical tuple fields. A new logical tuple gets
a new physical row. An existing tuple receives semiring addition in place. A
changed annotation marks the relation as changed; an unchanged annotation does
not. In a recursive strongly connected component, an annotation change also
places the existing row index in the next-iteration indexes. This reuses
Ascent's changed-row scheduling while keeping one physical row per logical
tuple.

The compiler emits a `ProvenanceSemiring` type check for all provenance
programs. If any strongly connected component is recursive, it additionally
emits an `IdempotentConvergentProvenanceSemiring` check. Consequently, a recursive program
using `HowProvenance` fails at compile time instead of running with incorrect
finite-polynomial semantics.

## Supported and rejected surface

The prototype supports serial positive relational clauses, joins, projection,
alternative rules, disjunction, Rust conditions, bindings, patterns,
generators, rule macros, and multi-head rules. Existing rule bodies do not need
provenance variables or rewritten relation arities.

The macro produces focused compile errors for negation, aggregation, lattice
relations, Bring Your Own Data Structures (BYODS) attributes, empty-body fact
rules, and recursive use of a nonconvergent semiring. Provenance evaluation is
serial; no parallel provenance macro is provided. Base facts must be supplied
as annotated relation rows.

## Why recursive how-provenance stops here

Green, Karvounarakis, and Tannen derive positive relational semantics from
commutative semirings: addition combines alternative derivations and
multiplication combines jointly used facts. They use `N[X]` provenance
polynomials for positive relational algebra. For recursive Datalog, however,
cycles may generate infinitely many distinct monomials or infinitely many
copies of one monomial. Their semantics therefore move to formal power series
over `N` extended with infinity and describe finite algebraic systems of fixed
point equations for representing them. See the
[canonical paper](https://www.cs.ucdavis.edu/~green/papers/pods07.pdf).

Treating a repeatedly updated finite polynomial as if it were that formal
power series would be mathematically wrong. The prototype instead rejects
recursive how-provenance. A serious next version needs a representation for
algebraic systems, a useful query interface for coefficients or finite cases,
and a clear relationship between Ascent's evaluation schedule and least-fixed-
point semantics.

## Verification and current results

The focused suite covers semiring identities and laws, copy, join, projection,
alternative rules, duplicate derivations, why-provenance collapsing behavior,
input normalization, the diamond polynomial, supported Rust clauses, recursive
acyclic and cyclic why-provenance, and the one-run guard. Six compile-fail
fixtures pin every explicit rejection category.

Two reference-based tests additionally check nonlinear recursion
(`path(x,z) <-- path(x,y), path(y,z)`) and mutually recursive relations.
The reference enumerates nonempty walks and tracks their token sets, without
using the production semiring operations or join scheduler. Each test compares
every tuple and witness set on 83 graphs in both input orders: all directed
two-node graphs including self-loops, all directed three-node graphs without
self-loops, and diamond, cyclic-diamond, and duplicate-edge cases. Downstream
relations are also checked, and evaluation has a timeout to catch nontermination.
These are finite exhaustive families and targeted examples, not a proof for
all recursive programs.

The runnable examples are:

```text
cargo +1.85.0 run -p ascent --example provenance_diamond
cargo +1.85.0 run -p ascent --example provenance_recursive_why
```

The final clean verification passed:

- `cargo +1.85.0 test --workspace`, including 15 provenance runtime tests,
  three algebra tests, and all six compile-fail fixtures
- the focused provenance suites with `--no-default-features`
- all 64 tests in the separately excluded `ascent_tests` crate in serial mode
  and all 64 again with its `par` feature
- both runnable examples, scoped Clippy checks, targeted formatting checks,
  and Markdown linting

Parallel regression testing protects the existing `ascent_par!` path; it does
not imply parallel support for provenance. The focused runtime suite also
checks that reusable Ascent rule sources run unchanged in provenance mode and
that a user-defined semiring works through the public trait.

## Next research directions

The lead research question is a usable recursive how-provenance representation
based on formal power series or their finite equation systems. The next
engineering question is provenance-aware incremental maintenance. Existing
[Ascent issue #30](https://github.com/s-arash/ascent/issues/30) shows that
repeated `run()` calls can sometimes reuse work but can also perform more work
than a full rerun, while removals are not maintained. Provenance makes this
harder because additions and retractions must update annotations, not only set
membership.

After those semantics are settled, the remaining branches are negation and
aggregation, parallel scheduling and concurrent annotation updates, BYODS
integration, and performance work. None should be added merely by threading an
extra tuple field through existing code; each changes the provenance algebra or
the evaluator's update discipline.
