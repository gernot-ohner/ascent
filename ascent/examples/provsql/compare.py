"""Compare actual Ascent and pinned ProvSQL results; no Python provenance evaluator."""

from collections import Counter
import heapq
import json
from pathlib import Path
import random
import subprocess
import time
from uuid import UUID, uuid4


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
IMAGE = "inriavalda/provsql@sha256:58b7ad6acacfd769d8898a8a27603743ca461c9e9a56da93d3ff049a9c28148c"
CASES = {
    "copy", "join", "projection", "alternatives", "self_join", "non_absorption",
    "alternative_product", "no_match", "identity", "reach", "suffix",
}


def run(*args, input=None, timeout=120):
    return subprocess.run(
        args, input=input, text=True, stdout=subprocess.PIPE,
        check=True, cwd=ROOT, timeout=timeout,
    ).stdout


def sections(output, names):
    result, current = {}, None
    for line in output.splitlines():
        if line.startswith("@"):
            current = line[1:]
            if current not in names or current in result:
                raise ValueError(f"unexpected or repeated section: {line}")
            result[current] = []
        elif current is None or not line:
            raise ValueError(f"unexpected output: {line!r}")
        else:
            result[current].append(line.split("\t"))
    if result.keys() != names:
        raise ValueError(f"missing sections: {names - result.keys()}")
    return result


def witnesses(raw, labels):
    value = json.loads(raw.replace("{", "[").replace("}", "]"))
    if not isinstance(value, list) or not value:
        raise ValueError(f"expected nonzero witness collection: {raw}")
    result = []
    for witness in value:
        if (not isinstance(witness, list)
                or any(type(token) is not int or token not in labels for token in witness)
                or len(set(witness)) != len(witness)):
            raise ValueError(f"invalid witness: {raw}")
        result.append(frozenset(witness))
    if len(set(result)) != len(result):
        raise ValueError(f"duplicate witnesses: {raw}")
    return frozenset(result)


def parse(output, *, labels, names=CASES, provsql=False, boolean=False):
    result = {}
    for name, rows in sections(output, names).items():
        result[name] = {}
        for fields in rows:
            if len(fields) != (4 if provsql else 3):
                raise ValueError(f"unexpected columns in {name}: {fields}")
            if provsql:
                # Explicitly consume and validate the saved internal circuit ID.
                source, target, raw, circuit = fields
                UUID(circuit)
            else:
                source, target, raw = fields
            key = (int(source), int(target))
            if key in result[name]:
                raise ValueError(f"duplicate output tuple: {name} {key}")
            if boolean:
                if raw not in {"t", "f"}:
                    raise ValueError(f"invalid Boolean: {raw}")
                value = raw == "t"
            else:
                value = witnesses(raw, labels)
            result[name][key] = value
    return result


def validate_mapping(rows, leaves, edges):
    mapping = {}
    for raw_id, raw_label in rows:
        uid, label = UUID(raw_id), int(raw_label)
        if uid in mapping:
            raise ValueError("duplicate mapped input UUID")
        mapping[uid] = label
    if Counter(mapping.values()) != Counter(token for _, _, token in edges):
        raise ValueError("mapping does not preserve physical input labels")
    leaf_ids = [UUID(row[0]) for row in leaves if len(row) == 1]
    if len(leaf_ids) != len(leaves) or len(set(leaf_ids)) != len(leaf_ids):
        raise ValueError("invalid or repeated input leaves")
    if not set(leaf_ids) <= mapping.keys():
        raise ValueError("unmapped circuit input leaf")


def require_dag(edges):
    nodes = {node for a, b, _ in edges for node in (a, b)}
    pending = {node: set() for node in nodes}
    for a, b, _ in edges:
        pending[b].add(a)
    while pending:
        roots = {node for node, parents in pending.items() if not parents}
        if not roots:
            raise ValueError("non-absorbing recursive fixture must be a DAG")
        pending = {node: parents - roots for node, parents in pending.items() if node not in roots}


def equal(actual, expected, name):
    if actual != expected:
        raise AssertionError(f"{name}\nAscent: {actual}\nProvSQL: {expected}")


def rejects(error, action):
    try:
        action()
    except error:
        return
    raise AssertionError(f"negative control did not raise {error.__name__}")


def self_checks():
    # These controls catch lossy parsing, incomplete maps and weak comparisons.
    valid = "@copy\n0\t1\t[[1, 3], [2, 4]]\n"
    expected = {"copy": {(0, 1): frozenset({frozenset({1, 3}), frozenset({2, 4})})}}
    equal(parse(valid, labels={1, 2, 3, 4}, names={"copy"}), expected, "parser")
    for raw in ("[[1], [1]]", "[[1, 1]]", "[[true]]", "[[9]]", "[]", "[1]", "null"):
        rejects(ValueError, lambda: parse(f"@copy\n0\t1\t{raw}\n", labels={1}, names={"copy"}))
    for output in (valid + valid, valid + "0\t1\t[[1]]\n", "@unknown\n", "", "noise\n" + valid):
        rejects(ValueError, lambda: parse(output, labels={1, 2, 3, 4}, names={"copy"}))
    rejects(ValueError, lambda: parse("@copy\n0\t1\t[[1]]\tbad-uuid\n",
                                     labels={1}, names={"copy"}, provsql=True))
    uid, other = str(UUID(int=1)), str(UUID(int=2))
    rejects(ValueError, lambda: parse(f"@copy\n0\t1\ttrue\t{uid}\n",
                                     labels={1}, names={"copy"}, provsql=True, boolean=True))
    validate_mapping([(uid, "7")], [(uid,)], [(0, 1, 7)])
    rejects(ValueError, lambda: validate_mapping([], [(uid,)], [(0, 1, 7)]))
    rejects(ValueError, lambda: validate_mapping([(uid, "7")], [(other,)], [(0, 1, 7)]))
    rejects(ValueError, lambda: validate_mapping([(uid, "8")], [(uid,)], [(0, 1, 7)]))
    rejects(ValueError, lambda: validate_mapping([(uid, "7"), (uid, "7")], [(uid,)],
                                                [(0, 1, 7), (0, 1, 7)]))
    require_dag([(0, 1, 1), (1, 2, 2)])
    rejects(ValueError, lambda: require_dag([(0, 0, 1)]))
    rejects(ValueError, lambda: require_dag([(0, 1, 1), (1, 0, 2)]))
    for bad in ({}, {"copy": {}}, {"copy": {(0, 1): frozenset({frozenset({9})})}},
                {"copy": {(0, 1): frozenset({frozenset({1, 3})})}},
                {"copy": {**expected["copy"], (9, 9): frozenset({frozenset({1})})}},
                {"copy": {(0, 1): expected["copy"][(0, 1)] | {frozenset({1, 3, 5})}}}):
        rejects(AssertionError, lambda: equal(bad, expected, "controlled wrong answer"))
    print("PASS parser, mapping, DAG guard and negative witness controls", flush=True)


def query_bodies():
    result = {}
    for part in (HERE / "queries.sql").read_text().split("-- @")[1:]:
        name, body = part.split("\n", 1)
        result[name] = body.strip()
    if result.keys() != CASES:
        raise ValueError("SQL case inventory differs from Ascent")
    return result


def tsv(rows):
    return "".join("\t".join(map(str, row)) + "\n" for row in rows)


def reference(psql, edges, mode, recursive, destination=3):
    if mode == "why" and recursive:
        require_dag(edges)
    labels = sorted({token for _, _, token in edges})
    if len(labels) > 6:
        raise ValueError("reference fixtures are bounded to six labels")
    queries = query_bodies()
    sql = [
        "BEGIN; SET search_path TO public, provsql;",
        f"SET provsql.provenance='{'semiring' if mode == 'why' else 'boolean'}';",
        "CREATE TEMP TABLE edge(source integer, target integer, label text);",
        "COPY edge FROM STDIN;\n" + tsv(edges) + "\\.",
        "SELECT add_provenance('edge');",
        "SELECT create_provenance_mapping('labels', 'edge', 'label');",
        f"CREATE TEMP TABLE destination AS SELECT {destination} AS node;",
        "CREATE TEMP TABLE recursive_enabled(enabled boolean);",
    ]
    if recursive:
        sql.append("INSERT INTO recursive_enabled VALUES (true);")
    for name, query in queries.items():
        sql.append(f"CREATE TEMP TABLE result_{name} AS {query};")
    # Rewriting is off for inspection/evaluation only. All saved roots are fixed.
    sql += [
        "SET provsql.active=off;",
        "\\echo @mapping",
        "SELECT provenance,value FROM labels;",
        "\\echo @leaves",
        "WITH RECURSIVE gates(id) AS ("
        + " UNION ".join(f"SELECT provsql FROM result_{name}" for name in queries)
        + " UNION SELECT child FROM gates, unnest(get_children(id)) child"
        + ") SELECT id FROM gates WHERE get_gate_type(id)='input';",
    ]
    masks = range(1 << len(labels)) if mode == "boolean" else [None]
    if mode == "boolean":
        sql.append("CREATE TEMP TABLE valuation AS SELECT provenance,false AS value FROM labels;")
    names = set()
    for mask in masks:
        if mask is not None:
            # Every row gets true or false; absent inputs are never omitted.
            true_labels = [label for i, label in enumerate(labels) if mask & (1 << i)]
            condition = "value::integer IN (" + ",".join(map(str, true_labels)) + ")" if true_labels else "false"
            sql.append("UPDATE valuation SET value=chosen.present FROM "
                       f"(SELECT provenance, {condition} AS present FROM labels) chosen "
                       "WHERE valuation.provenance=chosen.provenance;")
        for name in queries:
            section = name if mask is None else f"{name}/{mask}"
            names.add(section)
            evaluator = "sr_why(provsql,'labels')" if mask is None else "sr_boolean(provsql,'valuation')"
            sql += [f"\\echo @{section}", f"SELECT x,y,{evaluator},provsql::text AS circuit FROM result_{name};"]
    sql.append("ROLLBACK;")
    output = run(*psql, input="\n".join(sql) + "\n")
    # Only setup functions emit empty rows, before the first explicit marker.
    prefix, marker, payload = output.partition("@mapping\n")
    if not marker or prefix.strip():
        raise ValueError(f"unexpected reference setup output: {prefix!r}")
    parts = sections(marker + payload, names | {"mapping", "leaves"})
    validate_mapping(parts.pop("mapping"), parts.pop("leaves"), edges)
    values = parse(
        "".join("@" + name + "\n" + "".join("\t".join(row) + "\n" for row in rows)
                for name, rows in parts.items()),
        labels=set(labels), names=names, provsql=True, boolean=mode == "boolean",
    )
    if mode == "why":
        return values
    result = {name: {} for name in queries}
    for name in queries:
        keys = values[f"{name}/0"].keys()
        for mask in masks:
            if values[f"{name}/{mask}"].keys() != keys:
                raise ValueError("saved query tuples changed across valuations")
        for key in keys:
            supports = [
                frozenset(label for i, label in enumerate(labels) if mask & (1 << i))
                for mask in masks if values[f"{name}/{mask}"][key]
            ]
            if not supports:
                raise ValueError("query tuple is false under every complete valuation")
            result[name][key] = frozenset(s for s in supports if not any(t < s for t in supports))
    return result


def shortest_distances(edges, destination):
    # Independent reverse Dijkstra; no witness or provenance evaluation here.
    distances, queue = {destination: 0}, [(0, destination)]
    while queue:
        distance, node = heapq.heappop(queue)
        if distance != distances[node]:
            continue
        for source, target, weight, _ in edges:
            total = distance + weight
            if target == node and total <= 2**32 - 1 and total < distances.get(source, float("inf")):
                distances[source] = total
                heapq.heappush(queue, (total, source))
    tight = [(a, b, label) for a, b, weight, label in edges
             if a in distances and b in distances and weight + distances[b] == distances[a]]
    return distances, tight


def compare_all(binary, psql):
    fixture = [tuple(map(int, row.split())) for row in (HERE / "edges.tsv").read_text().splitlines()]
    datasets = [
        ("diamond", fixture[:4], True), ("example", fixture, False),
        ("reversed", list(reversed(fixture)), False), ("empty", [], True),
        ("cycle-disconnected", [(0, 1, 10), (1, 0, 20), (1, 3, 30), (7, 8, 40)], False),
        ("shared-label-duplicates", [(0, 1, 7), (0, 1, 7), (1, 3, 11)], True),
        ("snapshot-before", [(0, 1, 1), (1, 3, 2)], True),
        ("snapshot-after", [(0, 2, 3), (2, 3, 4), (0, 3, 5)], True),
    ]
    rng = random.Random(731)
    for index in range(4):
        dag = index % 2 == 0
        candidates = [(a, b) for a in range(4) for b in range(4) if not dag or a < b]
        selected = rng.sample(candidates, 5)
        datasets.append((f"generated-{index}", [(a, b, i + 1) for i, (a, b) in enumerate(selected)], dag))
    for name, edges, dag in datasets:
        for mode in ("why", "boolean"):
            recursive = dag or mode == "boolean"
            if mode == "why" and recursive:
                require_dag(edges)
            actual = parse(run(str(binary), mode, str(recursive).lower(), "3", input=tsv(edges)),
                           labels={token for _, _, token in edges})
            expected = reference(psql, edges, mode, recursive)
            equal(actual, expected, f"{name}/{mode}")
            print(f"PASS {name}/{mode}: {len(CASES)} complete relations", flush=True)
    weighted = [
        ("ties-longer-loop", [(0, 1, 2, 1), (1, 3, 3, 2), (0, 2, 1, 3),
                              (2, 3, 4, 4), (0, 3, 9, 5), (1, 1, 0, 6)]),
        ("zero-cycle", [(0, 1, 2, 1), (1, 2, 0, 2), (2, 1, 0, 3),
                        (1, 3, 3, 4), (3, 3, 0, 5), (7, 8, 1, 6)]),
        ("acyclic-tight", [(0, 1, 2, 1), (1, 3, 3, 2), (0, 3, 5, 3)]),
        ("empty", []),
    ]
    for name, edges in weighted:
        distances, tight = shortest_distances(edges, 3)
        expected = reference(psql, tight, "boolean", True)["suffix"]
        expected = {"shortest": {(node, distances[node]): value for (node, _), value in expected.items()}}
        actual = parse(run(str(binary), "shortest", "true", "3", input=tsv(edges)),
                       labels={row[3] for row in edges}, names={"shortest"})
        equal(actual, expected, f"actual shortest demo/{name}")
        if name == "acyclic-tight":
            actual_why = parse(run(str(binary), "why", "true", "3", input=tsv(tight)),
                               labels={row[2] for row in tight})
            equal(actual_why, reference(psql, tight, "why", True), "acyclic tight-edge why")
        print(f"PASS actual shortest demo/{name}: complete distances and witnesses", flush=True)


def main():
    self_checks()
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
        print(run(*psql, input="SELECT version(); SELECT extversion FROM pg_extension WHERE extname='provsql';"),
              flush=True)
        compare_all(binary, psql)
        print("All complete relation/witness comparisons passed.", flush=True)
    finally:
        subprocess.run(["docker", "rm", "--force", "--volumes", container],
                       stdout=subprocess.DEVNULL, check=False, timeout=30)


if __name__ == "__main__":
    main()
