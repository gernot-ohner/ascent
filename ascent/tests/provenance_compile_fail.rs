#[test]
fn provenance_rejects_unsupported_programs() {
   let tests = trybuild::TestCases::new();
   tests.compile_fail("tests/ui/provenance/*.rs");
}
