"""Write the oracle corpus for the Rust tests: u, d, W from compact.py (validated against the
brute-force reference) and the greedy output string from greedy.py (tie-breaking replicated)."""
import json, random, sys
from compact import compute_counts_compact
from greedy import greedy_scs
import search, exp2, fuzz_cov, bench

def entry(strs):
    S, u, d, W = compute_counts_compact(strs)
    return {"strings": strs, "W": W, "u": u, "d": d, "greedy": greedy_scs(strs)}

rng = random.Random(7); random.seed(7)
insts = []
for i in range(1500): insts.append(search.gen(rng))
for i in range(1000): insts.append(exp2.gen(random.choice(["periodic", "fib", "rand"])))
for i in range(1000): insts.append(fuzz_cov.gen(rng))
for i in range(200):  # non-ASCII alphabets (greedy's bad family, small copies)
    k = rng.randint(1, 8); strs = []
    for copy in range(rng.randint(1, 4)):
        a, b, c, e = (chr(0x100 + 4 * copy + j) for j in range(4))
        strs += [c + (a + b) * k, (b + a) * k, (a + b) * k + e]
    insts.append(strs)
for i in range(12):  # medium read sets
    g = bench.repgenome(rng, 3000) if i % 2 else "".join(rng.choice("acgt") for _ in range(3000))
    insts.append(bench.reads(rng, 0, rng.choice([100, 200, 300]), rng.choice([30, 50]), g))
out = [entry(s) for s in insts]
path = sys.argv[1]
with open(path, "w") as f:
    json.dump(out, f, separators=(",", ":"), ensure_ascii=False)
print(len(out), "instances written to", path)
