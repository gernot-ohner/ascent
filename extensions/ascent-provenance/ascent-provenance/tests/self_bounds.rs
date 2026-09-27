use std::hash::Hash;

use ascent_provenance::provenance;

trait Values<T> {
   fn adjust(value: &T) -> T;
}

provenance! {
    struct Separate<T: Clone + Eq + Hash>;
    impl<T: Clone + Eq + Hash> Separate<T> where Self: Values<T>;
    relation input(T);
    relation output(T);
    output(Self::adjust(x)) <-- input(x);
}
impl<T: Clone + Eq + Hash> Values<T> for Separate<T> {
   fn adjust(value: &T) -> T { value.clone() }
}

provenance! {
    struct Declaration<T: Clone + Eq + Hash> where Self: Values<T>;
    relation input(T);
    relation output(T);
    output(Self::adjust(x)) <-- input(x);
}
impl<T: Clone + Eq + Hash> Values<T> for Declaration<T> {
   fn adjust(value: &T) -> T { value.clone() }
}

#[test]
fn separate_impl_bounds_keep_public_self() {
   let mut program = Separate::<i32>::default();
   program.input = vec![(42,)];
   program.run();
   assert_eq!(program.output, vec![(42,)]);
}

#[test]
fn declaration_bounds_keep_public_self() {
   let mut program = Declaration::<String>::default();
   program.input = vec![("hello".to_owned(),)];
   program.run();
   assert_eq!(program.output, vec![("hello".to_owned(),)]);
}
