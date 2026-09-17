"""Build a relocated source-only consumer: no old checkout or probe dependency."""
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="ascent-relocated-") as temporary:
    destination = Path(temporary)
    for filename in ["Cargo.toml", "Cargo.lock", "LICENSE", "README.md"]:
        shutil.copy2(ROOT / filename, destination / filename)
    for package in ["ascent-provenance", "ascent-provenance-macros"]:
        folder = destination / package
        folder.mkdir()
        for filename in ["Cargo.toml", "LICENSE"]:
            shutil.copy2(ROOT / package / filename, folder / filename)
        shutil.copytree(ROOT / package / "src", folder / "src")
    shutil.copytree(ROOT / "tests" / "consumer", destination / "tests" / "consumer",
                    ignore=shutil.ignore_patterns("target"))
    subprocess.run(
        ["cargo", "+1.85.0", "run", "--offline", "--locked", "--manifest-path",
         str(destination / "tests" / "consumer" / "Cargo.toml")],
        cwd=destination, check=True, timeout=120,
    )
print("PASS relocated source-only consumer")
