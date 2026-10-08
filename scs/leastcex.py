import itertools, random
from exp4 import counts_restricted
from counts import base_graph, reduce_instance
from exp3 import least_period
best = None
rng = random.Random(0)
words = ["".join(t) for n in range(2, 9) for t in itertools.product("ab", repeat=n)]
for _ in range(300000):
    strs = reduce_instance(rng.sample(words, rng.randint(2, 3)))
    L = sum(map(len, strs))
    if best and L >= best[0]: continue
    if base_graph(*counts_restricted(strs))[2] != base_graph(*counts_restricted(strs, least_only=True))[2]:
        best = (L, strs)
print(best)
S = best[1]
for flag in (False, True):
    S_, V, m, al = counts_restricted(S, least_only=flag)
    print("least_only" if flag else "full      ", "W =", base_graph(S_, V, m, al)[2],
          {s: m[s] for s in sorted(V, key=lambda x: (-len(x), x)) if s and m[s] > 1})
