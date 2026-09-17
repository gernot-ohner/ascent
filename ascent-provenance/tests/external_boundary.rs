use ascent_provenance::{provenance, provenance_run, WhyProvenance as W};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};

static INITIALIZATIONS: AtomicUsize = AtomicUsize::new(0);
provenance! {
    #![generate_run_timeout]
    /// Public forwarding program.
    #[allow(dead_code)]
    pub struct Lifecycle;
    #[provenance(&'static str)] relation engine(i32) = {
        INITIALIZATIONS.fetch_add(1, Ordering::SeqCst);
        vec![(1, W::token("a")), (1, W::token("b")), (2, W::default())]
    };
    #[provenance(&'static str)] relation path(i32);
    path(x) <-- engine(x);
}

#[test]
fn initialization_once_normalized_default_and_unchanged_runs() {
    let mut p = Lifecycle::default();
    assert_eq!(INITIALIZATIONS.load(Ordering::SeqCst), 1);
    assert_eq!(p.engine.len(), 1);
    p.run();
    let output = p.path.clone();
    p.run();
    assert!(p.run_timeout(std::time::Duration::MAX));
    assert_eq!(p.path, output);
    assert_eq!(INITIALIZATIONS.load(Ordering::SeqCst), 1);
    assert!(Lifecycle::summary().contains("path"));
    assert!(p.relation_sizes_summary().contains("path"));
    let _ = p.scc_times_summary();
    let owned = std::mem::take(&mut p.path);
    assert!(p.path.is_empty());
    let relations = &mut *p;
    let (engine, path) = (&mut relations.engine, &mut relations.path);
    engine.clear();
    path.extend(owned);
    assert_eq!(path, &output);
}

#[test]
fn assigned_duplicate_zero_and_preseeded_head_rows() {
    provenance! {
        struct Assigned;
        #[provenance(&'static str)] relation input();
        #[provenance(&'static str)] relation output();
        output() <-- input();
    }
    let mut p = Assigned::default();
    p.input = vec![(W::token("a"),), (W::token("b"),), (W::default(),)];
    p.output = vec![(W::default(),), (W::token("a"),)];
    p.run();
    assert_eq!(p.input.len(), 1);
    assert_eq!(p.output.len(), 1);
    assert_eq!(p.output[0].0.witnesses(), &BTreeSet::from([BTreeSet::from(["a"]),BTreeSet::from(["b"])]));
}

#[test]
fn inline_captures_and_initializer_order() {
    let mut order = Vec::new();
    let input = vec![(1, W::token("a"))];
    let p = provenance_run! {
        #[provenance(&'static str)] relation z_last(i32) = { order.push("z_last"); input };
        relation a_first(i32) = { order.push("a_first"); vec![(1,)] };
        #[provenance(&'static str)] relation output(i32);
        output(x) <-- z_last(x), a_first(x);
    };
    assert_eq!(order, vec!["a_first", "z_last"]);
    assert_eq!(p.output, vec![(1, W::token("a"))]);
    assert!(p.summary().contains("output"));
}

provenance! {
    pub struct Generic<T: Ord + std::hash::Hash>;
    impl<T: Clone + Eq + std::hash::Hash + Ord> Generic<T>;
    #[provenance(T)] relation input(i32);
    #[provenance(T)] relation output(i32);
    output(x) <-- input(x);
}

// Disabled code must gate the engine and every wrapper impl, not only the struct.
provenance! {
    #[cfg(any())]
    struct Disabled;
    relation absent(UndefinedType);
}
provenance! {
    #[cfg_attr(all(), cfg(any()))]
    struct DisabledIndirect;
    relation absent(UndefinedType);
}

#[test]
fn public_generic_separate_bounds_and_nested_scopes() {
    let mut p = Generic::<&'static str>::default();
    p.input = vec![(1, W::token("a"))];
    p.run();
    assert_eq!(p.input, p.output);
    let q = {
        provenance! { struct Local; relation value(i32) = vec![(1,)]; }
        Local::default()
    };
    let r = {
        provenance! { struct Local; relation value(i32) = vec![(1,)]; }
        Local::default()
    };
    assert_eq!(q.value, r.value);
}

#[test]
fn actual_nested_head_parameter_and_shadowing_macros_preserve_token() {
    let p = provenance_run! {
        macro read($r: ident, $x: ident) { $r($x) }
        macro nested($x: ident) { (read!(left, $x) | right($x)) }
        macro write($x: ident) { output($x) }
        macro shadow($x: ident) { left($x), if { let x = 0; x == 0 } }
        #[provenance(&'static str)] relation left(i32) = vec![(1, W::token("a"))];
        #[provenance(&'static str)] relation right(i32);
        #[provenance(&'static str)] relation output(i32);
        write!(x) <-- nested!(x);
        output(x) <-- shadow!(x);
    };
    assert_eq!(p.output, vec![(1, W::token("a"))]);
}

#[test]
fn timeout_false_is_inspectable_and_fresh_restart_completes() {
    provenance! {
        #![generate_run_timeout]
        struct Bounded;
        #[provenance(&'static str)] relation input(u32);
        #[provenance(&'static str)] relation path(u32);
        path(x) <-- input(x);
        path(x+1) <-- path(x), if *x < 32;
    }
    let mut partial = Bounded::default();
    partial.input = vec![(0, W::token("a"))];
    let completed = partial.run_timeout(std::time::Duration::ZERO);
    assert!(partial.path.iter().all(|r| r.1 == W::token("a")));
    // Zero duration may complete on an exceptionally fast clock; no exact timing assumption.
    if !completed {
        drop(partial);
        let mut fresh = Bounded::default();
        fresh.input = vec![(0, W::token("a"))];
        assert!(fresh.run_timeout(std::time::Duration::MAX));
        assert!(fresh.path.iter().any(|r| r.0 == 32));
    }
}

#[test]
fn custom_provider_private_state_survives_unchanged_rerun() {
    provenance! {
        struct Provider;
        #[provenance(&'static str)] relation seed(u32, u32);
        #[ds(ascent_byods_rels::trrel)] relation tr(u32, u32);
        relation materialized(u32, u32);
        tr(x,y) <-- seed(x,y);
        tr(x,y) <-- tr(x,y);
        materialized(x,y) <-- tr(x,y);
    }
    let mut p = Provider::default();
    p.seed = vec![(1,2,W::token("a")), (2,3,W::token("b"))];
    p.run();
    let expected = BTreeSet::from([(1,2), (1,3), (2,3)]);
    assert_eq!(p.materialized.iter().copied().collect::<BTreeSet<_>>(), expected);
    p.run();
    assert_eq!(p.materialized.iter().copied().collect::<BTreeSet<_>>(), expected);
}
