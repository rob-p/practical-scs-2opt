import random, sys, itertools, traceback
from multiprocessing import Pool
from connect import superstring
from compact import reduce_instance
from search import gen
import exp2

def ov(a, b):
    for k in range(min(len(a), len(b)) - 1, 0, -1):
        if a.endswith(b[:k]): return k
    return 0

def opt(S):
    best = None
    for perm in itertools.permutations(S):
        L = len(perm[0]) + sum(len(b) - ov(a, b) for a, b in zip(perm, perm[1:]))
        best = L if best is None else min(best, L)
    return best

def work(seed):
    rng = random.Random(seed); random.seed(seed)
    fails, worst = [], 0.0
    for i in range(100):
        strs = gen(rng) if i % 2 else exp2.gen(random.choice(["periodic", "fib", "rand"]))
        try:
            T = superstring(strs)
            S = reduce_instance(strs)
            if len(S) <= 7:
                o = opt(S); r = len(T) / o
                if r > 2: fails.append(("ratio>2", strs, T, o))
                worst = max(worst, r)
        except Exception as ex:
            fails.append((type(ex).__name__ + ": " + str(ex)[:120], strs, traceback.format_exc().splitlines()[-3]))
    return fails, worst

if __name__ == "__main__":
    with Pool() as p: res = p.map(work, range(int(sys.argv[1])))
    fails = [f for r in res for f in r[0]]
    print("instances:", 100 * int(sys.argv[1]), "failures:", len(fails), "worst ratio vs OPT:", max(r[1] for r in res))
    from collections import Counter
    print(Counter(f[0].split(":")[0] + ":" + f[0].split(":")[1][:40] if ":" in f[0] else f[0] for f in fails).most_common(8))
    for f in fails[:4]: print(" ", f)
