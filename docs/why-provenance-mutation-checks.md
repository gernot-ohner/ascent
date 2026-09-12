# Why-Provenance Property and Mutation Checks

This record describes the bounded property coverage and manual semantic
mutations used to harden why-provenance. Each mutant was applied alone, was
compiled by the listed `cargo` command, failed a behavior assertion, and was
then reversed before the next mutation. No mutant or mutation framework remains
in the tree.

## Property Scope

- The independent Rust walk oracle explores states keyed by
  `(endpoint, used-token-set)` and never calls production witness
  multiplication. Linear and mutual recursion cover all 512 directed graphs on
  three nodes, including self-loops. Nonlinear recursion covers all 466 such
  graphs with at most six edges. Every graph runs in original and reversed input
  order, and every logical row and witness alternative is compared with the
  oracle, including downstream relations.
- The nonlinear bound is a runtime bound, not a witness cap. A combined
  full-512 run of all three formulations exceeded 150 seconds before it was
  interrupted, while the separate full linear and mutual sweeps took 2.94 and
  5.54 seconds. The dense nonlinear cases were the bottleneck; the retained
  466-graph nonlinear sweep took 16.84 seconds. No production semantics are
  capped.
- The finite algebra sweep models the complete 16-value
  `P(P({0, 1}))` domain independently. It checks 256 operand pairs and 4,096
  triples for set-model results, change flags, partial order, lattice laws,
  product associativity, and distributivity.
- The lifecycle sweep checks all 1,331 three-position input batches formed from
  absence or five annotations at either of two keys. It independently models
  zero removal and witness union, compares normalized inputs and derived
  outputs, and checks unchanged reruns.

The independent Python calculation produced 512 graphs, 3,072 logical rows,
31,251 witness alternatives, graph digest
`4c7d2d1617fd8ab99e755e3ccfbd97ed8464599bfc9cc78ed821a335d3d66b4b`,
16 algebra values, 256 pairs, 4,096 triples, 1,331 lifecycle batches, and
lifecycle digest
`57e55d08a19587d4cde72abc3c54b01fb3cae90aedb2716d4d51ec89e553839a`.

## Mutation Results

All four mutations were killed. There were no survivors.

### 1. Omit a Joined Body Witness

Target: `provenance_product` in `ascent_macro/src/why_provenance.rs`.
The product fold

```rust
let mut product: Expr = parse_quote_spanned! {span=> (*#first).clone()};
for annotation in rest {
   product = parse_quote_spanned! {span=>
      ::ascent::internal::why_provenance_product(&(#product), #annotation)
   };
}
```

was temporarily replaced with:

```rust
let product: Expr = parse_quote_spanned! {span=> (*#first).clone()};
for _annotation in rest {}
```

Command:

```text
cargo +1.85.0 test -p ascent \
  --test why_provenance_recursion \
  every_three_node_graph_matches_the_linear_walk_oracle_in_both_input_orders \
  -- --exact
```

Result: compiled, then failed at graph mask `0x003`. For row `(0, 1)`,
the program produced `{{0}, {1}}` instead of oracle result
`{{0, 1}, {1}}`.

### 2. Lose Tied Witness Alternatives

Target: `WhyProvenance::join_mut` in
`ascent_base/src/why_provenance.rs`. The union

```rust
self.witnesses.extend(other.witnesses);
```

was temporarily replaced with:

```rust
if self.witnesses.is_empty() {
   self.witnesses.extend(other.witnesses.into_iter().take(1));
}
```

Command:

```text
cargo +1.85.0 test -p ascent \
  --test why_provenance_properties \
  finite_domain_operations_match_the_independent_set_model \
  -- --exact
```

Result: compiled, then failed for the join of domain values 0 and 3. The
mutant retained `{{}}` instead of the modeled tied alternatives
`{{}, {0}}`.

### 3. Suppress Recursive Annotation-Change Rescheduling

Target: the serial existing-lattice-row branch in
`head_update_code` in `ascent_macro/src/ascent_codegen.rs`. The line

```rust
#set_changed_true_code
```

was removed from the `if __lat_changed` branch while leaving the merge and
index updates intact.

Command:

```text
cargo +1.85.0 test -p ascent \
  --test why_provenance_recursion \
  nonlinear_recursion_propagates_late_alternatives_to_downstream_consumers \
  -- --exact
```

Result: compiled, then failed for row `(0, 2)`. The mutant omitted witness
`{0, 1, 3}`, producing `{{0, 1}, {2}}` instead of
`{{0, 1}, {0, 1, 3}, {2}}`.

### 4. Bypass Annotated Input Normalization

Target: the serial index-setup branch in
`ascent_macro/src/ascent_codegen.rs`. The generated normalization insertion

```rust
#provenance_normalization
```

was temporarily made unreachable while retaining type checking:

```rust
if false {
   #provenance_normalization
}
```

Command:

```text
cargo +1.85.0 test -p ascent \
  --test why_provenance_properties \
  every_bounded_input_batch_normalizes_and_reruns_idempotently \
  -- --exact
```

Result: compiled, then failed for input choices `[0, 0, 1]`. The mutant
retained one zero-annotated input row where the independent model expected no
row.

The earlier named-program `Default` initializer mutation check belongs to the
pass-1 implementation review and was not repeated here.

## Restoration and Green Verification

Production files matched their pre-mutation SHA-256 values after restoration:

```text
181b0cd20e4d294e38ea5e14a66c46fbdf8618137b46c69f817cbf17e8c1b799  ascent_macro/src/why_provenance.rs
a506c1cb23853b787400edf1ec0cfb69b9ee5b1514fcb92f44dc1c93a65f0c8d  ascent_macro/src/ascent_codegen.rs
8841f8345c318de5e250b7bb2eae8ab3bd5eb68703dfca26786176db99ca0eda  ascent_base/src/why_provenance.rs
```

Restored command:

```text
cargo +1.85.0 test -p ascent --test why_provenance_properties --test why_provenance_recursion
```

Result: exit 0; property tests 3/3 and recursion tests 6/6 passed. The
recursion integration finished in 15.90 seconds.
