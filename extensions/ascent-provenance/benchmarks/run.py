"""Run bounded release benchmarks; optionally use the same harness on a baseline."""
import argparse
import csv
import json
from pathlib import Path
import platform
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--label", default="current")
    parser.add_argument("--no-rss", action="store_true",
                        help="omit macOS memory measurement (useful inside a sandbox)")
    args = parser.parse_args()
    root = args.root.resolve()
    subprocess.run(["cargo", "+1.85.0", "build", "--release", "--locked", "--offline",
                    "-p", "ascent-provenance", "--example", "scaling"], cwd=root, check=True)
    metadata = json.loads(subprocess.check_output(
        ["cargo", "+1.85.0", "metadata", "--no-deps", "--format-version", "1", "--offline"], cwd=root))
    executable = Path(metadata["target_directory"]) / "release/examples/scaling"
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(["revision", "case", "size", "samples", "output_tuples", "output_witnesses",
                     "median_ns", "min_ns", "max_ns", "process_peak_rss_bytes"])
    for case, sizes in [("inputs", [1000, 2000, 4000, 8000, 16000, 32000]),
                        ("why", [4, 6, 8, 10, 12]),
                        ("boolean", [4, 6, 8, 10, 12])]:
        for size in sizes:
            command = [str(executable), case, str(size), str(args.samples)]
            if platform.system() == "Darwin" and not args.no_rss:
                command = ["/usr/bin/time", "-l", *command]
            result = subprocess.run(command, cwd=root, capture_output=True, text=True,
                                    timeout=180)
            if result.returncode:
                raise RuntimeError(f"benchmark failed: {command}\n{result.stderr}")
            peak = re.search(r"(\d+)\s+maximum resident set size", result.stderr)
            writer.writerow([args.label, *next(csv.reader([result.stdout.strip()])),
                             peak.group(1) if peak else ""])
            sys.stdout.flush()


if __name__ == "__main__":
    main()
