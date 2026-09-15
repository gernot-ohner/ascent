#![cfg(test)]
use crate::{AscentMacroKind, ascent_impl};

fn assert_ascent_error(input: proc_macro2::TokenStream, kind: AscentMacroKind, expected: &str) {
   let error = ascent_impl(input, kind).expect_err("expected macro expansion to fail");
   assert!(error.to_string().contains(expected), "unexpected error: {error}");
}

#[test]
fn test_agg_not_stratifiable() {
   let inp = quote! {
      relation foo(i32, i32, i32);
      relation bar(i32, i32);
      relation baz(i32);

      baz(x) <--
         foo(x, _, _),
         !bar(_, x);

      bar(x, x + 1) <-- baz(x);
   };
   let res = ascent_impl(inp, AscentMacroKind::default());
   println!("res: {:?}", res);
   assert!(res.is_err());
   assert!(res.unwrap_err().to_string().contains("bar"));
}

#[test]
fn provenance_attribute_validation_errors_are_source_level() {
   assert_ascent_error(
      quote! { #[provenance] relation input(i32); },
      AscentMacroKind::default(),
      "expected `#[provenance(TokenType)]`",
   );
   assert_ascent_error(
      quote! { #[provenance(u32, u64)] relation input(i32); },
      AscentMacroKind::default(),
      "expected exactly one token type",
   );
   assert_ascent_error(
      quote! {
         #[provenance(&'static str)]
         #[provenance(&'static str)]
         relation input(i32);
      },
      AscentMacroKind::default(),
      "multiple `provenance` attributes",
   );
   assert_ascent_error(
      quote! { #[provenance(&'static str)] lattice input(i32); },
      AscentMacroKind::default(),
      "user-declared `lattice`",
   );
   assert_ascent_error(
      quote! { #[provenance(&'static str)] #[ds(foo)] relation input(i32); },
      AscentMacroKind::default(),
      "custom data structure providers",
   );
   for is_ascent_run in [false, true] {
      assert_ascent_error(
         quote! { #[provenance(&'static str)] relation input(i32); },
         AscentMacroKind { is_ascent_run, is_parallel: true },
         "only supported by serial Ascent",
      );
   }
}

#[test]
fn provenance_program_setting_is_validated() {
   for attribute in [
      quote! { #![provenance] },
      quote! { #![provenance()] },
      quote! { #![provenance(unknown)] },
      quote! { #![provenance(absorption)] },
      quote! { #![provenance(boolean, extra)] },
   ] {
      assert_ascent_error(
         quote! { #attribute #[provenance(u8)] relation input(u8); },
         AscentMacroKind::default(),
         "expected `#![provenance(boolean)]`",
      );
   }
   assert_ascent_error(
      quote! { #![provenance(boolean)] #![provenance(boolean)] relation input(u8); },
      AscentMacroKind::default(),
      "multiple program-level `provenance` attributes",
   );
   for is_ascent_run in [false, true] {
      assert_ascent_error(
         quote! { #![provenance(boolean)] #[provenance(u8)] relation input(u8); },
         AscentMacroKind { is_ascent_run, is_parallel: true },
         "only supported by serial Ascent",
      );
   }
}

#[test]
fn provenance_restrictions_are_checked_after_macro_and_disjunction_expansion() {
   assert_ascent_error(
      quote! {
         #[provenance(&'static str)] relation input(i32);
         #[provenance(&'static str)] relation output(i32);
         relation ordinary(i32);
         macro aggregate($x: ident) { agg $x = ascent::aggregators::min(v) in input(v) }
         ordinary(x), output(x) <-- (aggregate!(x) | input(x));
      },
      AscentMacroKind::default(),
      "cannot use aggregation or negation",
   );
   assert_ascent_error(
      quote! {
         #[provenance(&'static str)] relation input(i32);
         #[provenance(&'static str)] relation output(i32);
         relation ordinary(i32);
         ordinary(x), output(x) <-- input(x), !input(0);
      },
      AscentMacroKind::default(),
      "cannot use aggregation or negation",
   );
   assert_ascent_error(
      quote! { #[provenance(&'static str)] relation output(i32); output(1); },
      AscentMacroKind::default(),
      "empty-body fact rules",
   );
}

#[test]
fn provenance_arity_diagnostics_keep_logical_arity() {
   assert_ascent_error(
      quote! {
         #[provenance(&'static str)] relation input(i32);
         relation output(i32);
         output(x) <-- input(x, y);
      },
      AscentMacroKind::default(),
      "expected 1, found 2",
   );
   assert_ascent_error(
      quote! {
         #[provenance(u32)] relation output(i32);
         output(x) <-- missing(x);
      },
      AscentMacroKind::default(),
      "relation `missing` is not defined",
   );
}
