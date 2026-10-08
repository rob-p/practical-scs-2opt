"""Numerically check every lemma used in the proofs, on the full (unrestricted) counts."""
import random, sys
from collections import defaultdict, Counter
from multiprocessing import Pool
from counts import reduce_instance, substrings, periods, block_functional, compute_counts
from search import gen

def check(strs):
    S, V, m, al = compute_counts(strs)
    P = {x[:i] for x in S for i in range(1, len(x)+1)}; Sf = {x[i:] for x in S for i in range(len(x))}
    lam = lambda s: sum(m.get(c+s,0) for c in al); rho = lambda s: sum(m.get(s+c,0) for c in al)
    bad = Counter(); n = Counter()
    for v in V:
        if not v: continue
        # Theorem A
        if v not in P: n["A"] += 1; bad["A: m=lam off P"] += m[v] != lam(v)
        if v not in Sf: n["A"] += 1; bad["A: m=rho off Sf"] += m[v] != rho(v)
        for p in periods(v):
            t = v[:p]; U = m[v] - block_functional(v, t, V, m, al)
            n["U"] += 1; bad["U>=0"] += U < 0
            e = t[len(v) % p]
            if v + e in V:  # monotonicity along A
                bad["U monotone"] += U < m[v+e] - block_functional(v+e, t, V, m, al)
            if U > 0:
                for k in range(1, (len(v)-1)//p + 1):
                    w = v[:len(v)-k*p]; bound = block_functional(w, t, V, m, al) + k + 1
                    n["shift"] += 1
                    if m[v] == lam(v): bad["shift(a): bound<=lam(w)"] += bound > lam(w)
                    if m[v] == rho(v): bad["shift(b): bound<=rho(w)"] += bound > rho(w)
    return n, bad

def work(seed):
    rng = random.Random(seed); n = Counter(); bad = Counter()
    for _ in range(100):
        a, b = check(gen(rng)); n += a; bad.update(b)
    return n, bad

if __name__ == "__main__":
    with Pool() as p: res = p.map(work, range(int(sys.argv[1])))
    n = sum((r[0] for r in res), Counter()); bad = Counter()
    for r in res: bad.update(r[1])
    print("checks:", dict(n)); print("violations:", dict(bad))
