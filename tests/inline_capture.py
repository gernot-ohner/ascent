"""Compatibility gate: stock and external inline rules must capture caller locals."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
directory = ROOT / "target" / "inline-capture"
(directory / "src").mkdir(parents=True, exist_ok=True)
(directory / "Cargo.toml").write_text(f"""
[package]
name = "inline-capture"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
ascent = {{ version = "=0.8.1", default-features = false }}
ascent-provenance = {{ path = {json.dumps(str(ROOT / "ascent-provenance"))} }}
""")
for macro in ["ascent::ascent_run", "ascent_provenance::provenance_run"]:
    (directory / "src" / "main.rs").write_text(f"""
fn main() {{
    let limit = 3;
    let p = {macro}! {{
        relation output(i32);
        output(x) <-- for x in 0..limit;
    }};
    assert_eq!(p.output.iter().map(|r| r.0).collect::<std::collections::BTreeSet<_>>(),
               (0..limit).collect());
}}
""")
    result = subprocess.run(
        ["cargo", "+1.85.0", "run", "--offline", "--quiet",
         "--manifest-path", str(directory / "Cargo.toml"),
         "--target-dir", str(ROOT / "target" / "capture-check")],
        cwd=ROOT, text=True, capture_output=True, timeout=120,
    )
    assert result.returncode == 0, f"{macro}:\n{result.stderr}"
    print(f"PASS {macro} caller-local rule capture")
