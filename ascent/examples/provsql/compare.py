"""Compare actual Ascent and ProvSQL results; no Python provenance evaluator."""

import json
from pathlib import Path
import subprocess
import time
from uuid import UUID, uuid4


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
# Official ProvSQL 1.12.0, pinned rather than following the mutable latest tag.
IMAGE = "inriavalda/provsql@sha256:58b7ad6acacfd769d8898a8a27603743ca461c9e9a56da93d3ff049a9c28148c"
CASES = {
    "copy", "join", "projection", "alternatives", "self_join",
    "non_absorption", "alternative_product", "no_match",
}


def run(*args, input=None, timeout=120):
    return subprocess.run(
        args, input=input, text=True, stdout=subprocess.PIPE,
        check=True, cwd=ROOT, timeout=timeout,
    ).stdout


def parse(output, *, provsql=False):
    results = {}
    current = None
    for line in output.splitlines():
        if line.startswith("@"):
            current = line[1:]
            if current not in CASES or current in results:
                raise ValueError(f"unexpected or repeated case: {line}")
            results[current] = {}
        else:
            fields = line.split("\t")
            if provsql:
                # ProvSQL appends its internal circuit identifier. The sr_why
                # column, not this database-specific UUID, is the explanation.
                source, target, raw, circuit = fields
                UUID(circuit)
            else:
                source, target, raw = fields
            # These fixtures use numeric token labels only. Normalize order,
            # never drop supersets or combine duplicate output rows.
            witnesses = json.loads(raw.replace("{", "[").replace("}", "]"))
            value = frozenset(frozenset(witness) for witness in witnesses)
            key = (int(source), int(target))
            if key in results[current]:
                raise ValueError(f"duplicate output tuple: {current} {key}")
            results[current][key] = value
    if results.keys() != CASES:
        raise ValueError(f"missing cases: {CASES - results.keys()}")
    return results


def main():
    run("cargo", "+1.85.0", "build", "-p", "ascent", "--example", "provsql_compare",
        "--no-default-features", timeout=600)
    metadata = json.loads(run("cargo", "+1.85.0", "metadata", "--format-version", "1", "--no-deps"))
    binary = Path(metadata["target_directory"]) / "debug/examples/provsql_compare"
    container = "ascent-provsql-" + uuid4().hex
    try:
        run("docker", "run", "--name", container, "--detach", "--rm", "--network", "none",
            "--platform", "linux/amd64", IMAGE,
            "pg_ctlcluster", "--foreground", "17", "main", "start", timeout=600)
        for _ in range(60):
            ready = subprocess.run(
                ["docker", "exec", container, "pg_isready", "-U", "test"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10,
            )
            if ready.returncode == 0:
                break
            time.sleep(1)
        else:
            raise RuntimeError("ProvSQL did not become ready")
        run("docker", "exec", container, "createdb", "-U", "test", "ascent_compare")
        psql = ("docker", "exec", "-i", container, "psql", "-X", "-qAt",
                "-F", "\t", "-v", "ON_ERROR_STOP=1", "-U", "test", "-d", "ascent_compare")
        run(*psql, input="CREATE EXTENSION provsql CASCADE;")
        print("Reference image:", IMAGE, flush=True)
        print(run(*psql, input="SELECT version(); SELECT extversion FROM pg_extension WHERE extname='provsql';"), flush=True)
        fixture = (HERE / "edges.tsv").read_text()
        queries = (HERE / "queries.sql").read_text()
        for name, data in [
            ("diamond", "\n".join(fixture.splitlines()[:4]) + "\n"),
            ("example", fixture),
            ("reversed", "\n".join(reversed(fixture.splitlines())) + "\n"),
            ("empty", ""),
        ]:
            sql = (
                "BEGIN; SET search_path TO public, provsql; SET provsql.provenance='semiring';\n"
                "CREATE TABLE edge(source integer, target integer, label text);\n"
                "COPY edge FROM STDIN;\n" + data + "\\.\n"
                "SELECT add_provenance('edge');\n"
                "SELECT create_provenance_mapping('labels', 'edge', 'label');\n"
                + queries + "\nROLLBACK;\n"
            )
            # Setup functions return empty text rows; discard only the setup
            # prefix, not any rows emitted by the actual comparison queries.
            output = run(*psql, input=sql)
            actual = parse(run(str(binary), input=data))
            reference = parse(output[output.index("@copy"):], provsql=True)
            for case in sorted(CASES):
                if actual[case] != reference[case]:
                    raise AssertionError(
                        f"{name}/{case}\nAscent: {actual[case]}\nProvSQL: {reference[case]}"
                    )
                print(f"PASS {name}/{case}: {len(actual[case])} identical result tuples", flush=True)
                if name in {"diamond", "example"} and case in {"join", "non_absorption"}:
                    for key, witnesses in sorted(actual[case].items()):
                        print(f"  {key}: {sorted(sorted(w) for w in witnesses)}", flush=True)
        print("All complete relation/witness comparisons passed.", flush=True)
    finally:
        # A name chosen before launch also permits cleanup if launch times out.
        # Do not mask the original failure if Docker itself is unavailable.
        subprocess.run(["docker", "rm", "--force", "--volumes", container],
                       stdout=subprocess.DEVNULL, check=False, timeout=30)


if __name__ == "__main__":
    main()
