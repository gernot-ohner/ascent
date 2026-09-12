#[test]
fn why_provenance_rejects_unsupported_programs_with_source_level_diagnostics() {
   let tests = trybuild::TestCases::new();
   tests.compile_fail("tests/ui/why_provenance/*.rs");
   #[cfg(feature = "par")]
   tests.compile_fail("tests/ui/why_provenance/parallel/*.rs");
}
