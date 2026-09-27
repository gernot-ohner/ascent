# 4. OpenJDK example and performance

This PR adds a three-way comparison on the existing OpenJDK facts. The library,
macros and previous PRs are unchanged. The [methods and report](../benchmarks/OPENJDK.md)
explain the selected subsets, validation and timed region.

Start with the three programs in
[analysis.rs](../ascent-provenance/examples/openjdk/analysis.rs). The compact
version relies on `eqrel`; the other two spell out reflexivity, symmetry and
transitivity. The Boolean version tags all four input relations, including
load and store. Check the load column order against an actual input row.

Then read `reference` and the component selector in
[facts.rs](../ascent-provenance/examples/openjdk/facts.rs), and `check` in
[analysis.rs](../ascent-provenance/examples/openjdk/analysis.rs). The oracle uses
union-find, not the provenance operations. Check that replay cannot accept an
unseeded reflexive pair, and that a field number does not connect components.

[openjdk.rs tests](../ascent-provenance/tests/openjdk.rs) enumerate all worlds
of the small fixtures and compare exact witness sets. The larger test compares
all logical pairs and replays every witness. Large-case replay establishes
sufficiency and minimality; it is not exhaustive witness-completeness testing.

Finally, inspect the timing calls in the
[example](../ascent-provenance/examples/openjdk.rs) and the serial
[runner](../benchmarks/openjdk.py). Parsing and correctness checks are outside
the measured interval. Whole-process peak RSS includes both. Full-data `eqrel`
is checked without enumerating its pair set.

The main questions are whether the selected facts remain closed under the
rules, whether the oracle could share a mistake with the example, and whether
the report draws only conclusions supported by the measured cases.
