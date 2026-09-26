use ascent_provenance::provenance;
use std::collections::BTreeSet;
use std::hash::Hash;

trait Values {
    type Value: Clone + Eq + Hash;
    const BASE: i32;
    fn numbers() -> std::ops::Range<i32>;
}

provenance! {
    struct Helpers<T: Clone + Eq + Hash>;
    relation marker(T);
    relation values(<Self as Values>::Value);
    relation picked(i32);
    relation output(i32);
    output(Self::adjust(x)) <-- for x in <Self as Values>::numbers(), if Self::allowed(x);
    values(x) <-- for x in vec![Self::BASE];
    values(x) <-- for x in vec![Self::BASE; 2];
    picked(Self::BASE) <-- values(?&Self::BASE);
    picked(Self::adjust(*x)) <-- values(x) if Self::allowed(*x);
    output(x) <-- for x in Self::numbers(), if {
        struct Local;
        impl Local {
            fn yes() -> bool { Self::answer() }
            fn answer() -> bool { true }
        }
        Local::yes()
    };
}
impl<T: Clone + Eq + Hash> Helpers<T> {
    fn adjust(value: i32) -> i32 {
        value + 10
    }
    fn allowed(value: i32) -> bool {
        value > 0
    }
}
impl<T: Clone + Eq + Hash> Values for Helpers<T> {
    type Value = i32;
    const BASE: i32 = 7;
    fn numbers() -> std::ops::Range<i32> {
        0..3
    }
}

// Catches engine renaming redirecting user Self, including generic trait paths;
// the nested impl checks that its unrelated Self is left alone.
#[test]
fn named_rules_keep_the_public_program_self() {
    let mut p = Helpers::<String>::default();
    p.run();
    assert_eq!(
        p.output.iter().map(|r| r.0).collect::<BTreeSet<_>>(),
        BTreeSet::from([0, 1, 2, 11, 12])
    );
    assert_eq!(p.values, vec![(7,)]);
    assert_eq!(
        p.picked.iter().map(|r| r.0).collect::<BTreeSet<_>>(),
        BTreeSet::from([7, 17])
    );
}

#[test]
fn stringify_preserves_self_as_literal_tokens() {
    provenance! {
        struct Text;
        relation output(&'static str);
        output(stringify!(Self)) <-- for _ in [()];
        output(::core::stringify!(Self)) <-- for _ in [()];
    }
    let mut p = Text::default();
    p.run();
    assert_eq!(p.output, vec![("Self",)]);
}
