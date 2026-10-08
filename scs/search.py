"""Large randomized search: (B) PS-restricted rules vs full rules; also finer variants."""
import random, sys
from multiprocessing import Pool
from exp4 import counts_restricted
from counts import base_graph, reduce_instance, substrings, periods, block_functional
from collections import defaultdict

def gen(rng):
    kind = rng.random()
    if kind < .4:   # shared periodic units, fragments with noisy ends
        units = ["".join(rng.choice("ab") for _ in range(rng.randint(1, 4))) for _ in range(rng.randint(1, 2))]
        out = []
        for _ in range(rng.randint(2, 6)):
            u = rng.choice(units); x = (u * 40)[rng.randint(0, 5):][:rng.randint(2, 13)]
            if rng.random() < .6: x = rng.choice("abc") + x
            if rng.random() < .6: x = x + rng.choice("abc")
            out.append(x)
        return out
    if kind < .6:   # fragments of a word with nested periodicity
        u = "".join(rng.choice("ab") for _ in range(rng.randint(1, 3)))
        big = (u * rng.randint(2, 4) + rng.choice("ab")) * 10
        return [big[i:i + rng.randint(3, 14)] for i in (rng.randrange(30) for _ in range(rng.randint(2, 6)))]
    return ["".join(rng.choice("ab") for _ in range(rng.randint(2, 12))) for _ in range(rng.randint(2, 6))]

def work(seed):
    rng = random.Random(seed); bad = []
    for _ in range(200):
        strs = gen(rng)
        ref = base_graph(*counts_restricted(strs))[:2]
        got = base_graph(*counts_restricted(strs, ps_only=True))[:2]
        if got != ref: bad.append(strs)
    return bad

if __name__ == "__main__":
    with Pool() as p:
        res = p.map(work, range(int(sys.argv[1])))
    bad = [b for r in res for b in r]
    print("instances:", 200 * int(sys.argv[1]), "PS-restriction failures:", len(bad))
    for b in bad[:5]: print(" ", b)
