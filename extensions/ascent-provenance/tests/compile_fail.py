"""Check real downstream diagnostics without compiler-output snapshots."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
CASES = {
    "expression_shaped_self_macro": ("""
use ascent_provenance::provenance;
macro_rules! tokens { (Self) => { 0..3 }; }
provenance! {
    struct Program;
    relation output(i32);
    output(x) <-- for x in tokens!(Self);
}
fn main() {}
""", "use the explicit program type"),
    "inline_rerun": ("""
use ascent_provenance::provenance_run;
fn main() { let mut p = provenance_run! { relation output(i32); }; p.run(); }
""", "no method named `run`"),
    "inline_timeout": ("""
use ascent_provenance::provenance_run;
fn main() { let _ = provenance_run! { #![generate_run_timeout] relation output(i32); }; }
""", "timeouts require a named"),
    "opaque_self_macro": ("""
use ascent_provenance::provenance;
macro_rules! custom { ($name:ident => $value:expr) => { $value }; }
provenance! {
    struct Program;
    relation output(i32);
    output(x) <-- for x in custom!(value => Self::values());
}
impl Program { fn values() -> std::ops::Range<i32> { 0..3 } }
fn main() {}
""", "use the explicit program type"),
    "token_mismatch": ("""
use ascent_provenance::{provenance, WhyProvenance};
provenance! {
    struct P;
    #[provenance(u32)] relation input(i32);
}
fn main() {
    let mut p = P::default();
    p.input = vec![(1, WhyProvenance::token("wrong"))];
}
""", "mismatched types"),
    "field_move": ("""
use ascent_provenance::provenance;
provenance! { struct P; #[provenance(u32)] relation output(i32); }
fn main() { let p = P::default(); let _rows = p.output; }
""", "cannot move out of dereference"),
    "removed_import": ("""
use ascent_provenance::provenance_par;
fn main() {}
""", "unresolved import"),
}

for name, (source, diagnostic) in CASES.items():
    directory = ROOT / "target" / "compile-fail" / name
    (directory / "src").mkdir(parents=True, exist_ok=True)
    (directory / "Cargo.toml").write_text(f"""
[package]
name = "{name}"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
ascent = {{ version = "=0.8.1", default-features = false }}
ascent-provenance = {{ path = {json.dumps(str(ROOT / "ascent-provenance"))} }}
""")
    (directory / "src" / "main.rs").write_text(source)
    result = subprocess.run(
        ["cargo", "+1.85.0", "check", "--offline", "--quiet",
         "--manifest-path", str(directory / "Cargo.toml"),
         "--target-dir", str(ROOT / "target" / "compile-fail-build")],
        cwd=ROOT, text=True, capture_output=True, timeout=120,
    )
    assert result.returncode != 0, f"{name} unexpectedly compiled"
    assert diagnostic in result.stderr, result.stderr
    assert "proc macro panicked" not in result.stderr, result.stderr
    print(f"PASS {name}: {diagnostic}")
