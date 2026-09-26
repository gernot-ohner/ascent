use ascent_provenance::{provenance, WhyProvenance as W};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};

static EQUALITIES: AtomicUsize = AtomicUsize::new(0);
const ROWS: usize = 4096;

#[derive(Clone, Debug, Eq, Hash)]
struct Counted(usize);
impl PartialEq for Counted {
    fn eq(&self, other: &Self) -> bool {
        EQUALITIES.fetch_add(1, Ordering::Relaxed);
        self.0 == other.0
    }
}
fn unique_rows() -> Vec<(Counted, W<usize>)> {
    (0..ROWS).map(|i| (Counted(i), W::token(i))).collect()
}
provenance! {
    struct Unique;
    #[provenance(usize)] relation input(Counted) = unique_rows();
}

// Catches a return to all-pairs key scans without relying on clock timings.
#[test]
fn unique_inputs_do_not_require_quadratic_equality_work() {
    EQUALITIES.store(0, Ordering::Relaxed);
    let mut program = Unique::default();
    let construction = EQUALITIES.swap(0, Ordering::Relaxed);
    assert!(
        construction < ROWS * 32,
        "construction compared keys {construction} times"
    );
    program.run();
    let execution = EQUALITIES.swap(0, Ordering::Relaxed);
    assert!(
        execution < ROWS * 32,
        "first execution compared keys {execution} times"
    );
    assert_eq!(program.input.len(), ROWS);
    for (i, (key, annotation)) in program.input.iter().enumerate() {
        assert_eq!(key.0, i);
        assert_eq!(annotation, &W::token(i));
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Collision(u32);
impl Hash for Collision {
    fn hash<H: Hasher>(&self, state: &mut H) {
        0_u8.hash(state);
    }
}

// Catches hash-only merging, nondeterministic output ordering and retained zeros.
#[test]
fn collisions_preserve_distinct_keys_first_seen_order_and_witnesses() {
    provenance! {
        struct Collisions;
        #[provenance(&'static str)] relation input(Collision);
    }
    let mut p = Collisions::default();
    p.input = vec![
        (Collision(2), W::token("b")),
        (Collision(1), W::token("a")),
        (Collision(2), W::token("c")),
        (Collision(3), W::default()),
    ];
    p.run();
    assert_eq!(
        p.input.iter().map(|r| r.0 .0).collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert_eq!(
        p.input[0].1.witnesses(),
        &BTreeSet::from([BTreeSet::from(["b"]), BTreeSet::from(["c"]),])
    );
    assert_eq!(p.input[1].1, W::token("a"));
    let before = p.input.clone();
    p.run();
    assert_eq!(p.input, before);
}
