"""Do restricted period-rule sets give identical base graphs?"""
import random
from collections import defaultdict
from counts import reduce_instance, substrings, periods, block_functional, base_graph
from exp2 import gen
from exp3 import least_period

def counts_restricted(strings, least_only=False, ps_only=False):
    S = reduce_instance(strings); V = substrings(S); al = sorted(set("".join(S)))
    P = {x[:i] for x in S for i in range(len(x)+1)}; Sf = {x[i:] for x in S for i in range(len(x)+1)}
    m = {}; pending = defaultdict(list)
    for s in sorted((x for x in V if x), key=len, reverse=True):
        val = max(sum(m.get(c+s,0) for c in al), sum(m.get(s+c,0) for c in al), 1 if s in S else 0)
        for tmpl, k in pending.pop(s, []):
            val = max(val, block_functional(s, tmpl, V, m, al) + k + 1)
        m[s] = val
        if ps_only and not (s in P and s in Sf): continue
        ps = [least_period(s)] if least_only else periods(s)
        for p in ps:
            if p >= len(s): continue
            if val > block_functional(s, s[:p], V, m, al):
                for k in range(1, (len(s)-1)//p + 1):
                    w = s[:len(s)-k*p]
                    if ps_only and not (w in P and w in Sf): continue
                    pending[w].append((s[:p], k))
    return S, V, m, al

if __name__ == "__main__":
    random.seed(5)
    res = {k: 0 for k in ["least", "ps", "both"]}; N = 1500
    for it in range(N):
        strs = gen(random.choice(["periodic", "fib", "rand"]))
        ref = base_graph(*counts_restricted(strs))[:2]
        for k, kw in [("least", dict(least_only=True)), ("ps", dict(ps_only=True)), ("both", dict(least_only=True, ps_only=True))]:
            got = base_graph(*counts_restricted(strs, **kw))[:2]
            res[k] += got == ref
            if got != ref and res[k] > it - 3: print(k, "DIFF", strs)
    print({k: f"{v}/{N} identical" for k, v in res.items()})
