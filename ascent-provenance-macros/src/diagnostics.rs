use crate::tests::program;
use crate::expand::expand_program;
use crate::lower::lower;
fn assert_ascent_error(tokens: proc_macro2::TokenStream, expected: &str) {
 let mut p = program(tokens);
 expand_program(&mut p).unwrap();
 let error = lower(p).err().expect("expected validation error");
 assert!(error.to_string().contains(expected), "{error}");
}

#[test]
fn macro_hidden_negation_is_rejected() {
    assert_ascent_error(quote! {
        #[provenance(&'static str)] relation input(i32);
        #[provenance(&'static str)] relation output(i32);
        macro absent($x: ident) { !input($x) }
        output(x) <-- input(x), absent!(x);
    }, "aggregation or negation");
}

#[test]
fn unsupported_wrapper_attributes_and_mismatched_signatures_are_errors() {
    for tokens in [
        quote! { #[derive(Clone)] struct P; relation input(i32); },
        quote! { #[repr(C)] struct P; relation input(i32); },
        quote! { #[cfg_attr(all(), derive(Clone))] struct P; relation input(i32); },
        quote! { struct P; impl Q; relation input(i32); },
    ] {
        assert!(crate::expand(tokens, false).is_err());
    }
}
#[test]
fn provenance_attribute_validation_errors_are_source_level() {
   assert_ascent_error(
      quote! { #[provenance] relation input(i32); },
      "expected `#[provenance(TokenType)]`",
   );
   assert_ascent_error(
      quote! { #[provenance(u32, u64)] relation input(i32); },
      "expected exactly one token type",
   );
   assert_ascent_error(
      quote! {
         #[provenance(&'static str)]
         #[provenance(&'static str)]
         relation input(i32);
      },
      "multiple `provenance` attributes",
   );
   assert_ascent_error(
      quote! { #[provenance(&'static str)] lattice input(i32); },
      "user-declared `lattice`",
   );
   assert_ascent_error(
      quote! { #[provenance(&'static str)] #[ds(foo)] relation input(i32); },
      "custom data structure providers",
   );

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
            "expected `#![provenance(boolean)]`",
      );
   }
   assert_ascent_error(
      quote! { #![provenance(boolean)] #![provenance(boolean)] relation input(u8); },
      "multiple program-level `provenance` attributes",
   );

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
      "cannot use aggregation or negation",
   );
   assert_ascent_error(
      quote! {
         #[provenance(&'static str)] relation input(i32);
         #[provenance(&'static str)] relation output(i32);
         relation ordinary(i32);
         ordinary(x), output(x) <-- input(x), !input(0);
      },
      "cannot use aggregation or negation",
   );
   assert_ascent_error(
      quote! { #[provenance(&'static str)] relation output(i32); output(1); },
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
      "expected 1, found 2",
   );
   assert_ascent_error(
      quote! {
         #[provenance(u32)] relation output(i32);
         output(x) <-- missing(x);
      },
      "relation `missing` is not defined",
   );
}
