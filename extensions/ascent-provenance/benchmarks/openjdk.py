"""Check and measure closed subsets of the repository's OpenJDK facts."""
import argparse
import csv
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FILES = ("alloc", "assign", "load", "store")


def output(command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def run_process(command, timeout):
    with subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          text=True, start_new_session=True) as process:
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            stdout, stderr = process.communicate()
            raise subprocess.TimeoutExpired(command, timeout, output=stdout, stderr=stderr)
        return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--facts", type=Path, default=ROOT.parents[1] / "byods/ascent-byods-rels/examples/steensgaard/openjdk_javalang_steensgaard")
    parser.add_argument("--output", type=Path, required=True, help="new directory for raw samples, checks and metadata")
    parser.add_argument("--caps", type=int, nargs="+", default=[8, 32, 64])
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--timeout", type=int, default=180, help="seconds per check or benchmark process")
    args = parser.parse_args()
    if not 1 <= args.samples <= 25 or args.timeout <= 0 or any(n <= 0 for n in args.caps) or len(set(args.caps)) != len(args.caps):
        parser.error("samples must be 1..25; timeout and distinct subset caps must be positive")
    args.output.mkdir(parents=True, exist_ok=False)
    facts = args.facts.resolve()
    subprocess.run(["cargo", "+1.85.0", "build", "--release", "--locked", "--offline", "-p", "ascent-provenance", "--example", "openjdk"], cwd=ROOT, check=True)
    target = Path(json.loads(output(["cargo", "+1.85.0", "metadata", "--no-deps", "--format-version", "1", "--offline"]))["target_directory"])
    binary = target / "release/examples/openjdk"
    source_paths = ["ascent-provenance/src/why_provenance.rs", "ascent-provenance/examples/openjdk.rs", "ascent-provenance/examples/openjdk/analysis.rs", "ascent-provenance/examples/openjdk/facts.rs", "benchmarks/openjdk.py"]
    metadata = {
        "date_utc": datetime.now(timezone.utc).isoformat(),
        "commit": output(["git", "rev-parse", "HEAD"]),
        "dirty": bool(output(["git", "status", "--porcelain", "--untracked-files=normal"])),
        "cargo_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "source_sha256": {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in source_paths},
        "platform": platform.platform(), "machine": platform.machine(), "cpus": os.cpu_count(),
        "compiler": output(["rustc", "+1.85.0", "-Vv"]),
        "samples": args.samples, "warmups": 1, "caps": args.caps, "timeout_seconds": args.timeout,
        "dataset": {name: {"sha256": hashlib.sha256((facts / (name + ".facts")).read_bytes()).hexdigest(), "rows": len((facts / (name + ".facts")).read_bytes().splitlines())} for name in FILES},
        "rss": "macOS process peak bytes, including loading, warmup and all samples" if platform.system() == "Darwin" else "unavailable",
    }
    if platform.system() == "Darwin":
        metadata["cpu"] = output(["sysctl", "-n", "machdep.cpu.brand_string"])
        metadata["memory_bytes"] = int(output(["sysctl", "-n", "hw.memsize"]))
    (args.output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    statuses, checks, raw = [], [], []

    def run(command, cap, mode, stage):
        try:
            result = run_process(command, args.timeout)
            (args.output / f"{stage}-{cap}-{mode}.log").write_text(result.stderr)
            if result.returncode:
                statuses.append(dict(cap=cap, mode=mode, stage=stage, status="failed", returncode=result.returncode))
                return None
            return result
        except subprocess.TimeoutExpired as error:
            stderr = error.stderr or b""
            if isinstance(stderr, bytes):
                stderr = stderr.decode(errors="replace")
            (args.output / f"{stage}-{cap}-{mode}.log").write_text(stderr)
            statuses.append(dict(cap=cap, mode=mode, stage=stage, status="timeout", timeout_seconds=args.timeout))
            return None

    # Each engine is checked separately: one timeout must not hide the baselines.
    for cap in [*args.caps, 0]:
        for mode in (["eqrel", "explicit", "boolean"] if cap else ["eqrel"]):
            result = run([str(binary), "check", str(facts), str(cap), mode], cap, mode, "check")
            if result:
                row = next(csv.DictReader(io.StringIO(result.stdout)))
                checks.append(row)
                statuses.append(dict(cap=cap, mode=mode, stage="check", status="passed"))
                print(f"PASS cap={cap} {mode}: {row['facts']} facts, {row['pairs']} pairs", flush=True)
            (args.output / "status.json").write_text(json.dumps(statuses, indent=2) + "\n")
    # No oracle runs beside a sample. Only successfully checked engines are timed.
    for check in checks:
        cap, mode = int(check["cap"]), check["mode"]
        command = [str(binary), "bench", str(facts), str(cap), mode, str(args.samples)]
        if platform.system() == "Darwin":
            command = ["/usr/bin/time", "-l", *command]
        result = run(command, cap, mode, "bench")
        if result is None:
            continue
        rows = list(csv.DictReader(io.StringIO(result.stdout)))
        assert len(rows) == args.samples
        rss = re.search(r"(\d+)\s+maximum resident set size", result.stderr)
        for row in rows:
            for name in ("facts", "alloc", "assign", "load", "store", "pairs", "witnesses", "max_witnesses"):
                assert row[name] == check[name], (cap, mode, name, row, check)
            row["process_peak_rss_bytes"] = rss.group(1) if rss else ""
        raw.extend(rows)
        statuses.append(dict(cap=cap, mode=mode, stage="bench", status="passed"))
        print(f"PASS cap={cap} {mode}: {len(rows)} samples", flush=True)
    for name, rows in [("checks.csv", checks), ("samples.csv", raw)]:
        if rows:
            with (args.output / name).open("w", newline="") as stream:
                writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
                writer.writeheader()
                writer.writerows(rows)
    (args.output / "status.json").write_text(json.dumps(statuses, indent=2) + "\n")
    if any(row["status"] != "passed" for row in statuses):
        sys.exit("Some cases failed or timed out; see status.json and logs. They are not measurements.")


if __name__ == "__main__":
    main()
