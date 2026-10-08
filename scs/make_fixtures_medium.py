"""Medium-size oracle instances (repeat-rich and microsatellite read sets), for the ignored-by-default
Rust test `counts_match_oracle_medium`. Output is large, so it is generated on demand, not committed."""
import json, random, sys
from multiprocessing import Pool
from compact import compute_counts_compact
from bench import repgenome, reads

def micro(rng, unit_len, G):
    parts = []
    while sum(map(len, parts)) < G:
        u = "".join(rng.choice("acgt") for _ in range(unit_len))
        rep = list(u * rng.randint(10, 40))
        for _ in range(rng.randint(0, 3)):
            rep[rng.randrange(len(rep))] = rng.choice("acgt")
        parts.append("".join(rep) + "".join(rng.choice("acgt") for _ in range(rng.randint(5, 30))))
    return "".join(parts)

def make(seed):
    rng = random.Random(seed)
    g = repgenome(rng, rng.choice([5000, 10000, 20000])) if seed % 2 else micro(rng, rng.randint(1, 7), 8000)
    strs = reads(rng, 0, rng.choice([300, 800, 1500]), rng.choice([40, 70, 100]), g)
    S, u, d, W = compute_counts_compact(strs)
    return {"strings": strs, "W": W, "u": u, "d": d}

if __name__ == "__main__":
    with Pool() as p:
        out = p.map(make, range(int(sys.argv[2])))
    json.dump(out, open(sys.argv[1], "w"), separators=(",", ":"))
    print(len(out), "medium instances, total reads", sum(len(o["strings"]) for o in out))
