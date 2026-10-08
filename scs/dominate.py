"""For each word w in PS: compare bounds from rules with v not in PS against max(ext-left, ext-right, PS-v rules)."""
import random, sys
from collections import defaultdict, Counter
from multiprocessing import Pool
from counts import reduce_instance, substrings, periods, block_functional
from search import gen

def analyse(strs):
    S = reduce_instance(strs); V = substrings(S); al = sorted(set("".join(S)))
    P = {x[:i] for x in S for i in range(1, len(x)+1)}; Sf = {x[i:] for x in S for i in range(len(x))}
    m = {}; pending = defaultdict(list); out = Counter(); ex = []
    for s in sorted((x for x in V if x), key=len, reverse=True):
        lam = sum(m.get(c+s,0) for c in al); rho = sum(m.get(s+c,0) for c in al)
        bPS = bNon = 0; nonsrc = None
        for tmpl, k, v in pending.pop(s, []):
            b = block_functional(s, tmpl, V, m, al) + k + 1
            if v in P and v in Sf: bPS = max(bPS, b)
            elif b > bNon: bNon, nonsrc = b, (v, tmpl, k)
        req = 1 if s in S else 0
        m[s] = max(lam, rho, req, bPS, bNon)
        if s in P and s in Sf and bNon:
            if bNon > max(lam, rho, req, bPS): out["NON-PS RULE STRICTLY NEEDED"] += 1; ex.append((S, s, nonsrc))
            elif bNon == max(lam, rho, req, bPS):
                out["tight; also from " + ",".join(n for n, x in [("lam", lam), ("rho", rho), ("PSrule", bPS)] if x == bNon)] += 1
            else: out["dominated"] += 1
        for p in periods(s):
            if m[s] > block_functional(s, s[:p], V, m, al):
                for k in range(1, (len(s)-1)//p + 1):
                    pending[s[:len(s)-k*p]].append((s[:p], k, s))
    return out, ex

def work(seed):
    rng = random.Random(seed); tot = Counter(); exs = []
    for _ in range(200):
        o, e = analyse(gen(rng)); tot += o; exs += e
    return tot, exs[:2]

if __name__ == "__main__":
    with Pool() as p: res = p.map(work, range(int(sys.argv[1])))
    tot = sum((r[0] for r in res), Counter())
    for k, v in tot.most_common(): print(f"{v:8}  {k}")
    for r in res:
        for e in r[1][:1]: print(" ex:", e)
