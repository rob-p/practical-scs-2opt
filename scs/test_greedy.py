import random, itertools
from greedy import greedy_scs, reduce_instance, overlap, order_merge
from compact import reduce_instance as reduce_naive
from connect import superstring

def naive_ov(a, b):
    return max([k for k in range(1, min(len(a), len(b))) if a.endswith(b[:k])], default=0)

def naive_greedy(strs):
    S = reduce_naive(strs)
    while len(S) > 1:
        best = max(((naive_ov(a, b), i, j) for i, a in enumerate(S) for j, b in enumerate(S) if i != j))
        k, i, j = best
        m = S[i] + S[j][k:]
        S = [x for t, x in enumerate(S) if t not in (i, j)] + [m]
        S = reduce_naive(S)
    return S[0] if S else ""

rng = random.Random(0)
same = diff = 0; deltas = []
for it in range(20000):
    al = rng.choice(["ab", "abc", "acgt"])
    strs = ["".join(rng.choice(al) for _ in range(rng.randint(1, 10))) for _ in range(rng.randint(1, 8))]
    assert sorted(reduce_instance(strs)) == sorted(reduce_naive(strs)), strs
    a, b = rng.choice(strs), rng.choice(strs)
    assert overlap(a, b) == naive_ov(a, b), (a, b)
    G = greedy_scs(strs); N = naive_greedy(strs)
    assert all(x in G for x in strs)
    if len(G) == len(N): same += 1
    else: diff += 1; deltas.append(len(G) - len(N))
    T = superstring(strs, check=False); R = order_merge(T, strs)
    assert all(x in R for x in strs) and len(R) <= len(T)
print("greedy length == naive greedy:", same, " differ (tie-breaking):", diff, " mean delta:", sum(deltas) / max(1, len(deltas)))
