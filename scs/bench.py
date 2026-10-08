import random, time, sys
from counts import compute_counts, base_graph
from compact import compute_counts_compact

def reads(rng, G, n, l, genome=None):
    g = genome or "".join(rng.choice("acgt") for _ in range(G))
    return [g[i:i+l] for i in (rng.randrange(len(g)-l) for _ in range(n))]

def repgenome(rng, G):
    parts = []
    while sum(map(len, parts)) < G:
        if rng.random() < .3:
            unit = "".join(rng.choice("acgt") for _ in range(rng.randint(1, 12)))
            parts.append(unit * rng.randint(5, 40))
        else:
            parts.append("".join(rng.choice("acgt") for _ in range(rng.randint(50, 400))))
    return "".join(parts)

if __name__ == "__main__":
    rng = random.Random(1)
    print(f"{'instance':40}{'L':>8}{'ref (s)':>9}{'compact (s)':>12}{'W':>7}")
    for G, n, l in [(400, 40, 20), (1000, 60, 30), (2000, 80, 40)]:
        for kind in ["random", "repeats"]:
            s = reads(rng, G, n, l, None if kind == "random" else repgenome(rng, G))
            t = time.time(); W = base_graph(*compute_counts(s))[2]; tr = time.time() - t
            t = time.time(); W2 = compute_counts_compact(s)[3]; tc = time.time() - t
            assert W == W2
            print(f"{kind+f' G={G} n={n} l={l}':40}{n*l:>8}{tr:>9.2f}{tc:>12.3f}{W:>7}")
    print("--- compact only ---")
    for G, n, l in [(20000, 1000, 100), (100000, 5000, 100), (200000, 20000, 100)]:
        for kind in ["random", "repeats"]:
            s = reads(rng, G, n, l, None if kind == "random" else repgenome(rng, G))
            st = {}; t = time.time(); W2 = compute_counts_compact(s, stats=st)[3]; tc = time.time() - t
            print(f"{kind+f' G={G} n={n} l={l}':40}{n*l:>8}{'':>9}{tc:>12.2f}{W2:>7}  {st}")
