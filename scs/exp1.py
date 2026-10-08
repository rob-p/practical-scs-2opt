import random, time, itertools, sys
from counts import *

def ov(a, b):
    for k in range(min(len(a), len(b)) - 1, 0, -1):
        if a.endswith(b[:k]): return k
    return 0

def opt(S):
    if len(S) > 8: return None
    best = None
    for perm in itertools.permutations(S):
        L = len(perm[0]) + sum(len(b) - ov(a, b) for a, b in zip(perm, perm[1:]))
        best = L if best is None else min(best, L)
    return best

def classify(s, S):
    pre = any(x.startswith(s) for x in S); suf = any(x.endswith(s) for x in S)
    return ("P" if pre else "") + ("S" if suf else "") or "inner"

# sanity: paper's example
S, V, m, al = compute_counts(["aaaaa"])
print("a^5:", [m["a"*j] for j in range(1,6)], base_graph(S,V,m,al)[2])

random.seed(1)
print(f"{'inst':28}{'|V|':>7}{'supp':>6}{'W':>5}{'W0':>5}{'OPT':>5}{'rules':>7}{'Bevals':>8}{'t(s)':>7}  support classes")
def run(name, strs):
    st = {}
    t = time.time(); S, V, m, al = compute_counts(strs, stats=st); t = time.time() - t
    u, d, W, sup = base_graph(S, V, m, al)
    S0, V0, m0, _ = compute_counts(strs, use_period_rule=False)
    W0 = base_graph(S0, V0, m0, al)[2]
    o = opt(S)
    if o is not None: assert W <= o, (strs, W, o)
    cls = {}
    for x in sup - {""}:
        c = classify(x, S); cls[c] = cls.get(c, 0) + 1
    print(f"{name:28}{len(V):>7}{len(sup):>6}{W:>5}{W0:>5}{str(o):>5}{st['rules']:>7}{st['block_evals']:>8}{t:>7.2f}  {cls}")

for alpha, n, l in [("ab",6,8),("ab",8,10),("acgt",8,12),("acgt",20,15),("acgt",40,20),("ab",30,20)]:
    run(f"rand {alpha} n={n} l={l}", ["".join(random.choice(alpha) for _ in range(l)) for _ in range(n)])
# reads from a genome (realistic SCS use)
for G, n, l in [(60,8,12),(200,40,20),(400,80,25)]:
    g = "".join(random.choice("acgt") for _ in range(G))
    run(f"reads G={G} n={n} l={l}", [g[i:i+l] for i in (random.randrange(G-l) for _ in range(n))])
# periodic-heavy
for n, l in [(6,10),(20,16)]:
    run(f"periodic-ish n={n} l={l}", [("".join(random.choice("ab") for _ in range(random.randint(1,4)))*20)[:l] + random.choice("abc") for _ in range(n)])
