use ascent_provenance::{provenance_run, WhyProvenance as W};
use std::collections::BTreeSet;

// Catches inline rules accidentally being emitted into a noncapturing fn item.
#[test]
fn rules_capture_caller_values_and_return_owned_outputs() {
    let limit = 4;
    let allowed = [1, 3];
    let rows = vec![(1, W::token("a")), (1, W::token("b")), (2, W::default())];
    let p = provenance_run! {
        #[provenance(&'static str)] relation input(i32) = rows;
        #[provenance(&'static str)] relation output(i32);
        relation generated(i32);
        output(x) <-- input(x), if allowed.contains(x);
        generated(x) <-- for x in 0..limit, if allowed.contains(&x);
    };
    assert_eq!(p.input.len(), 1);
    assert_eq!(
        p.output[0].1.witnesses(),
        &BTreeSet::from([BTreeSet::from(["a"]), BTreeSet::from(["b"]),])
    );
    let owned = p.generated;
    assert_eq!(
        owned.into_iter().map(|r| r.0).collect::<BTreeSet<_>>(),
        BTreeSet::from([1, 3])
    );
    assert_eq!(allowed, [1, 3]);
}

struct Factory;
impl Factory {
    const LIMIT: i32 = 3;
    fn values() -> Vec<(i32,)> {
        let offset = 2;
        provenance_run! {
            relation values(i32);
            values(x + offset) <-- for x in 0..Self::LIMIT;
        }
        .values
    }
}

#[test]
fn inline_self_retains_the_callers_impl_scope() {
    assert_eq!(Factory::values(), vec![(2,), (3,), (4,)]);
}

#[test]
fn normalized_initializers_preserve_captures_order_and_single_evaluation() {
    let mut order = Vec::new();
    // Names deliberately coincide with normalizer implementation locals.
    let input = vec![(1, W::token("a")), (1, W::token("b"))];
    let rows = 2;
    let positions = 3;
    let p = provenance_run! {
        #[provenance(&'static str)] relation z(i32) = { order.push("z"); input };
        relation a(i32) = { order.push("a"); vec![(rows + positions,)] };
        #[provenance(&'static str)] relation output(i32);
        output(x) <-- z(x);
    };
    assert_eq!(order, ["a", "z"]);
    assert_eq!(p.a, vec![(5,)]);
    assert_eq!(p.z.len(), 1);
    assert_eq!(p.output.len(), 1);
    assert!(p.summary().contains("output"));
}

#[test]
fn inline_execution_preserves_outer_generic_tokens() {
    fn explain<T: Clone + Ord + std::hash::Hash>(token: T) -> Vec<(i32, W<T>)> {
        let rows = vec![(1, W::token(token))];
        let keep = 1;
        provenance_run! {
            struct Inline<T: Clone + Ord + std::hash::Hash>;
            #[provenance(T)] relation input(i32) = rows;
            #[provenance(T)] relation output(i32);
            output(x) <-- input(x), if *x == keep;
        }
        .output
    }
    assert_eq!(
        explain(String::from("a")),
        vec![(1, W::token(String::from("a")))]
    );
}

#[test]
fn inline_normalization_coexists_with_custom_provider_state() {
    let rows = vec![
        (1, 2, W::token("a")),
        (1, 2, W::token("b")),
        (2, 3, W::token("c")),
    ];
    let p = provenance_run! {
        #[provenance(&'static str)] relation seed(u32, u32) = rows;
        #[ds(ascent_byods_rels::trrel)] relation tr(u32, u32);
        relation materialized(u32, u32);
        tr(x,y) <-- seed(x,y);
        tr(x,y) <-- tr(x,y);
        materialized(x,y) <-- tr(x,y);
    };
    assert_eq!(p.seed.len(), 2);
    assert_eq!(
        p.materialized.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([(1, 2), (1, 3), (2, 3)])
    );
}
