"""Practical comparison: 2-approximation (connection phase) vs efficient greedy.

For every instance we report:
  W        the forced-count lower bound (W <= OPT), so len/W upper-bounds the true ratio
  ours     raw Euler-tour output of the 2-approximation (guaranteed <= 2W)
  ours+om  ours followed by order-merge (never longer than ours, so still <= 2 OPT)
  greedy   maximum-overlap greedy
  OPT      exact optimum, only for instances with at most 8 strings
Usage: python3 experiments.py [quick]
"""
import itertools, json, random, sys, time
from multiprocessing import Pool
from connect import superstring
from greedy import greedy_scs, order_merge, reduce_instance, overlap
from bench import repgenome


def exact_opt(S):
    best = None
    ov = {(a, b): overlap(a, b) for a in S for b in S if a != b}
    for perm in itertools.permutations(S):
        L = len(perm[0]) + sum(len(b) - ov[a, b] for a, b in zip(perm, perm[1:]))
        best = L if best is None else min(best, L)
    return best


def run_instance(args):
    family, label, strs = args
    t = time.time(); st = {}
    T = superstring(strs, check=False, stats=st); t_ours = time.time() - t
    t = time.time(); R = order_merge(T, strs); t_om = time.time() - t
    t = time.time(); G = greedy_scs(strs); t_g = time.time() - t
    S = reduce_instance(strs)
    assert all(x in T for x in S) and all(x in R for x in S) and all(x in G for x in S)
    W = st["W"]
    assert len(T) <= 2 * W
    return dict(family=family, label=label, n=len(S), L=sum(map(len, S)), W=W,
                ours=len(T), ours_om=len(R), greedy=len(G),
                opt=exact_opt(S) if len(S) <= 8 else None,
                t_ours=t_ours, t_om=t_om, t_greedy=t_g, events=st["events"])


def reads(rng, genome, n, l):
    return [genome[i:i + l] for i in (rng.randrange(len(genome) - l + 1) for _ in range(n))]


def instances(quick):
    rng = random.Random(2026)
    out = []
    # 1. Error-free reads from a random genome, varying coverage
    g = "".join(rng.choice("acgt") for _ in range(20000))
    for cov in [1, 3, 10]:
        for l in [50, 100]:
            n = cov * len(g) // l
            out.append(("random genome reads", f"cov={cov} l={l}", reads(rng, g, n, l)))
    # 2. Reads from a repeat-rich genome (tandem repeats of random units)
    for seed in range(3):
        rg = repgenome(random.Random(seed), 20000)
        for cov in [3, 10]:
            out.append(("repeat-rich genome reads", f"seed={seed} cov={cov} l=100", reads(rng, rg, cov * len(rg) // 100, 100)))
    # 3. Microsatellite-heavy: short-unit tandem repeats with point mutations
    for unit_len in [2, 3, 5]:
        parts = []
        for _ in range(60):
            u = "".join(rng.choice("acgt") for _ in range(unit_len))
            rep = list(u * rng.randint(10, 40))
            for _ in range(rng.randint(0, 3)):
                rep[rng.randrange(len(rep))] = rng.choice("acgt")
            parts.append("".join(rep) + "".join(rng.choice("acgt") for _ in range(rng.randint(5, 30))))
        mg = "".join(parts)
        out.append(("microsatellite reads", f"unit={unit_len} cov=5 l=40", reads(rng, mg, 5 * len(mg) // 40, 40)))
    # 4. Random strings (unrelated, short overlaps only)
    for al, n, l in [("ab", 300, 12), ("abcd", 300, 12), ("ab", 1000, 20)]:
        out.append(("random strings", f"|Σ|={len(al)} n={n} l={l}",
                    ["".join(rng.choice(al) for _ in range(l)) for _ in range(n)]))
    # 5. Greedy's classical bad family {c(ab)^k, (ba)^k, (ab)^k c}, in independent copies
    for k in [5, 20, 50]:
        strs = []
        for copy in range(20):
            a, b, c, e = (chr(0x100 + 4 * copy + i) for i in range(4))   # private alphabet per copy
            strs += [c + (a + b) * k, (b + a) * k, (a + b) * k + e]
        out.append(("greedy-adversarial", f"k={k} x20", strs))
    # 6. Small instances with exact optimum
    for kind in ["periodic", "binary"]:
        for i in range(200 if not quick else 40):
            if kind == "binary":
                strs = ["".join(rng.choice("ab") for _ in range(rng.randint(3, 10))) for _ in range(rng.randint(3, 7))]
            else:
                units = ["".join(rng.choice("ab") for _ in range(rng.randint(1, 4))) for _ in range(2)]
                strs = []
                for _ in range(rng.randint(3, 7)):
                    u = rng.choice(units); x = (u * 20)[rng.randint(0, 3):][:rng.randint(3, 12)]
                    if rng.random() < .5: x += rng.choice("abc")
                    strs.append(x)
            out.append((f"small {kind} (exact OPT)", str(i), strs))
    return out


if __name__ == "__main__":
    quick = len(sys.argv) > 1 and sys.argv[1] == "quick"
    insts = instances(quick)
    with Pool() as p:
        res = p.map(run_instance, insts, chunksize=1)
    with open("experiments_results.json", "w") as f:
        json.dump(res, f, indent=1)
    print(f"{len(res)} instances; results in experiments_results.json")
