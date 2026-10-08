"""Hunt for instances exercising rare connection-phase branches."""
import random, sys
from collections import Counter
from multiprocessing import Pool
from connect import superstring

def gen(rng):
    k = rng.random()
    al = rng.choice(["ab", "abc", "acgt"])
    units = ["".join(rng.choice(al) for _ in range(rng.randint(1, 6))) for _ in range(rng.randint(1, 4))]
    out = []
    for _ in range(rng.randint(2, 12)):
        u = rng.choice(units)
        x = (u * 60)[rng.randint(0, len(u)):][:rng.randint(2, 30)]
        if rng.random() < .5: x = "".join(rng.choice(al) for _ in range(rng.randint(0, 3))) + x
        if rng.random() < .5: x = x + "".join(rng.choice(al) for _ in range(rng.randint(0, 3)))
        if rng.random() < .3:  # splice two periodic pieces
            v = rng.choice(units); x = x + (v * 20)[:rng.randint(1, 15)]
        out.append(x)
    return out

def work(seed):
    rng = random.Random(seed); c = Counter(); ex = {}
    for _ in range(100):
        strs = gen(rng); st = {}
        try:
            superstring(strs, stats=st)
        except Exception as e:
            c["FAIL " + type(e).__name__] += 1; ex.setdefault("FAIL", (strs, repr(e)[:200])); continue
        for k in list(st["by_op"]) + list(st["events"]):
            c[k] += 1; ex.setdefault(k, strs)
    return c, ex

if __name__ == "__main__":
    with Pool() as p: res = p.map(work, range(int(sys.argv[1])))
    tot = sum((r[0] for r in res), Counter())
    print("instances:", 100 * int(sys.argv[1]))
    for k, v in sorted(tot.items()): print(f"{v:8}  {k}")
    for r in res:
        if "FAIL" in r[1]: print("FAIL example:", r[1]["FAIL"]); break
