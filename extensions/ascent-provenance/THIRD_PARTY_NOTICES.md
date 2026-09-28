# Source attribution

This project includes MIT-licensed code from Ascent, copyright its individual
contributors (see LICENSE), [Ascent upstream](https://github.com/s-arash/ascent).

Transferred from the provenance experiment at commit
`ad05d27c00fb0237a00dc58a8f82c9e35d5ec479`; stock comparison commit:
`e52c84b4419c56976413de1cbf15d56c8c136a5c`.

- `ascent-provenance/src/why_provenance.rs`: transferred from
  `ascent_base/src/why_provenance.rs`, with public Ascent imports.
- `syntax.rs`: declaration, rule, clause, signature and source callback parsers
  selected from `ascent_macro/src/ascent_syntax.rs`; token emission is new.
  Source callback boundaries use parser cursors rather than span equality.
- `expand.rs`: local rule-macro substitution/hygiene, bound-variable visitors
  and disjunction expansion selected from that same syntax file.
- `syntax_utils.rs`: required Rust expression/pattern visitors from
  `ascent_macro/src/syn_utils.rs`, and punctuated/token/span helpers from
  `ascent_macro/src/utils.rs`. The local `update` helper uses `mem::take`.
  No dependency analysis, index planner, or evaluator was extracted.
- `lower.rs`: transferred `ascent_macro/src/why_provenance.rs`, with
  public external runtime paths, local logical-arity lookup, and private
  annotation metadata instead of HIR membership.
- The five `tests/why_provenance*.rs` files and diamond, shortest-path and
  ProvSQL examples/harness are transferred from `ascent/tests` and
  `ascent/examples` at the same commit. Changes concern imports, macro entry
  points, output ownership and the Cargo package argument, not expected results
  or oracle semantics. The historical absorption test filename is retained.

The OpenJDK example adapts the stock Steensgaard example at the stock comparison
commit above, with explicit mapping of the checked-in load TSV columns. It uses
the existing repository fact files from the [PACT 2019 artifact](https://zenodo.org/records/3346193),
whose dataset record specifies the Universal Permissive License 1.0. No copy
of those fact files is added to the extension.
