use crate::syntax::{parse_program, emit_program, Parsed};
use crate::expand::expand_program;

fn program(tokens: proc_macro2::TokenStream) -> crate::syntax::AscentProgram {
    match parse_program(tokens, syn::parse_quote!(::ascent_provenance::provenance)).unwrap() {
        Parsed::Program(p) => p,
        Parsed::Include(_) => panic!("unexpected inclusion"),
    }
}

#[test]
fn syntax_round_trip() {
    let p = program(quote! {
        #![generate_run_timeout]
        /// A generic program.
        pub struct Test<T: Clone + Eq + std::hash::Hash>;
        impl<T: Clone + Eq + std::hash::Hash> Test<T>;
        relation input(T);
        relation numbers(Option<i32>) = vec![(Some(1),)];
        lattice best(i32, ascent::lattice::Min<i32>);
        relation output(i32);
        macro read($x: ident) { numbers($x) }
        output(x), output(x + 1) <-- numbers(?Some(x)) if *x > 0, for y in [1], if y > 0;
        output(n) <-- agg n = ascent::aggregators::count() in numbers(_), !output(0);
        output(x) <-- (numbers(?Some(x)) | numbers(?Some(x))), let z = { let x = 1; x }, if z > 0;
    });
    let emitted = emit_program(&p);
    assert_eq!(emitted.to_string(), emit_program(&program(emitted.clone())).to_string());
}

#[test]
fn nested_head_and_parameterized_macros_expand() {
    let mut p = program(quote! {
        macro read($r: ident, $x: ident) { $r($x) }
        macro nested($x: ident) { (read!(left, $x) | right($x)) }
        macro write($x: ident) { output($x) }
        relation left(i32); relation right(i32); relation output(i32);
        write!(x) <-- nested!(x);
    });
    expand_program(&mut p).unwrap();
    let emitted = emit_program(&p).to_string();
    assert!(!emitted.contains("nested !"));
    assert!(!emitted.contains("write !"));
    assert!(p.macros.is_empty());
    program(emit_program(&p));
    compile_stock("expanded", emit_program(&p));
}

fn compile_stock(name: &str, tokens: proc_macro2::TokenStream) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let dir = root.join("target/frontend-probes").join(name);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("Cargo.toml"), format!(r#"
[package]
name = "frontend-{name}"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
ascent = {{ version = "=0.8.1", default-features = false }}
ascent-provenance = {{ path = "{}" }}
"#, root.join("ascent-provenance").display())).unwrap();
    std::fs::write(dir.join("src/main.rs"), quote! {
        ascent::ascent! { #tokens }
        fn main() {}
    }.to_string()).unwrap();
    let output = std::process::Command::new("cargo")
        .args(["+1.85.0", "check", "--offline", "--quiet", "--manifest-path"])
        .arg(dir.join("Cargo.toml")).env("CARGO_TARGET_DIR", root.join("target/frontend-probe-build"))
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn malformed_and_recursive_macros_return_errors() {
    for tokens in [
        quote! { macro m($x: ident) { input($x) } relation input(i32); input(x) <-- m!(); },
        quote! { macro m($x: ident) { m!($x) } relation input(i32); input(x) <-- m!(x); },
        quote! { relation input(i32); input(x) <-- missing!(x); },
    ] {
        assert!(expand_program(&mut program(tokens)).is_err());
    }
}

#[test]
fn inclusion_keeps_external_callback_and_surrounding_tokens() {
    let Parsed::Include(tokens) = parse_program(quote! {
        relation before(i32); include_source!(fixture::rules); relation after(i32);
    }, syn::parse_quote!(::ascent_provenance::provenance_run)).unwrap() else { panic!() };
    let text = tokens.to_string();
    for expected in ["before", "after", "fixture :: rules", "ascent_provenance :: provenance_run"] {
        assert!(text.contains(expected));
    }
}
