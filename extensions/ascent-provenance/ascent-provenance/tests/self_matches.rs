use std::collections::BTreeSet;

use ascent_provenance::provenance;

provenance! {
    struct Matching;
    relation output(&'static str);
    output("unqualified") <-- for _ in [()], if matches!(Self::VALUE, 7);
    output("core") <-- for _ in [()], if ::core::matches!(Self::VALUE, 7,);
    output("std") <-- for _ in [()], if std::matches!(Self::VALUE, Self::VALUE);
    output("patterns") <-- for _ in [()], if matches!(8, | Self::VALUE | Self::OTHER,);
    output("guard") <-- for _ in [()],
        if matches!(Self::pair(), (Some(n), _) | (_, Some(n)) if Self::allowed(n),);
    output("bad-guard") <-- for _ in [()],
        if matches!(Self::pair(), (Some(n), _) | (_, Some(n)) if !Self::allowed(n));
    output("bad-pattern") <-- for _ in [()], if matches!(Self::VALUE, Self::OTHER);
}
impl Matching {
   const VALUE: i32 = 7;
   const OTHER: i32 = 8;
   fn pair() -> (Option<i32>, Option<i32>) { (None, Some(7)) }
   fn allowed(value: i32) -> bool { value == Self::VALUE }
}

#[test]
fn standard_matches_preserves_self_in_expressions_patterns_and_guards() {
   let mut program = Matching::default();
   program.run();
   assert_eq!(
      program.output.iter().map(|row| row.0).collect::<BTreeSet<_>>(),
      BTreeSet::from(["unqualified", "core", "std", "patterns", "guard"])
   );
}
