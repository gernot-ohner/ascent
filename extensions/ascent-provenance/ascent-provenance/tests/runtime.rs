use ascent::Lattice;
use ascent_provenance::{BooleanProvenance, WhyProvenance};
use std::collections::BTreeSet;

#[test]
fn alternatives_product_and_absorption_remain_distinct() {
    let a = WhyProvenance::token("a");
    let b = WhyProvenance::token("b");
    assert_eq!(a.clone().join(b.clone()).witnesses(),
        &BTreeSet::from([BTreeSet::from(["a"]), BTreeSet::from(["b"])]));
    assert_eq!(a.__product(&b).witnesses(),
        &BTreeSet::from([BTreeSet::from(["a", "b"])]));
    let a = BooleanProvenance::token("a");
    let ab = a.__product(&BooleanProvenance::token("b"));
    assert_eq!(a.clone().join(ab), a);
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TokenWithoutDefault;

#[test]
fn public_stock_lattice_supports_tokens_without_default() {
    ascent::ascent! {
        struct CopyProgram;
        lattice input(i32, WhyProvenance<TokenWithoutDefault>);
        lattice output(i32, WhyProvenance<TokenWithoutDefault>);
        output(x, p.clone()) <-- input(x, p);
    }
    let mut program = CopyProgram::default();
    program.input = vec![(1, WhyProvenance::token(TokenWithoutDefault))];
    program.run();
    assert_eq!(program.input, program.output);
    let _: BooleanProvenance<TokenWithoutDefault> = Default::default();
}
