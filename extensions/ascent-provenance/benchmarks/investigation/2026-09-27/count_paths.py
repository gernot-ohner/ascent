"""Count the isolated 115-fact component; optionally write its four fact files.

Usage: python3 count_paths.py FACTS_DIRECTORY [NEW_COMPONENT_DIRECTORY]
This diagnostic enumerates simple paths and is intended only for this component.
"""
import json
from pathlib import Path
import sys

refs = json.loads(Path(__file__).with_name("isolated-component.json").read_text())
facts = Path(sys.argv[1])
selected = {rel: [(facts / (rel + ".facts")).read_text().splitlines()[line - 1]
                  for line in lines] for rel, lines in refs.items()}
rows = [line.split("\t") for lines in selected.values() for line in lines]
assert len(rows) == 115 and all(len(row) == 2 for row in rows)
names = sorted({v for row in rows for v in row})
assert len(names) == 99
index = {name: i for i, name in enumerate(names)}
edges = [tuple(sorted((index[a], index[b]))) for a, b in rows]
assert len(set(edges)) == 115 and all(a != b for a, b in edges)
adj = [[] for _ in names]
for a, b in edges:
    adj[a].append(b)
    adj[b].append(a)
total = maximum = 0
for source in range(len(names)):
    counts = [0] * len(names)
    def visit(v, seen):
        for u in adj[v]:
            if not seen & (1 << u):
                counts[u] += 1
                visit(u, seen | (1 << u))
    visit(source, 1 << source)
    # Reflexive supports are the individual incident seed edges.
    counts[source] = len(adj[source])
    total += sum(counts)
    maximum = max(maximum, max(counts))
assert (total, maximum) == (135460, 58)
print(json.dumps(dict(facts=len(rows), nodes=len(names), pairs=len(names)**2,
                      witnesses=total, max_witnesses=maximum)))
if len(sys.argv) == 3:
    destination = Path(sys.argv[2])
    destination.mkdir(parents=True, exist_ok=False)
    for rel in ("alloc", "assign", "load", "store"):
        lines = selected.get(rel, [])
        (destination / (rel + ".facts")).write_text("".join(line + "\n" for line in lines))
