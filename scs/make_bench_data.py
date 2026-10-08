"""Write benchmark read sets (one read per line) to rust/bench_data/."""
import os, random
from bench import repgenome, reads
out = os.path.join(os.path.dirname(__file__), "..", "rust", "bench_data")
os.makedirs(out, exist_ok=True)
rng = random.Random(11)
for G, n, l in [(20000, 1000, 100), (200000, 20000, 100), (1000000, 100000, 100)]:
    for kind in ["random", "repeats"]:
        g = "".join(rng.choice("acgt") for _ in range(G)) if kind == "random" else repgenome(rng, G)
        path = os.path.join(out, f"{kind}_G{G}_n{n}_l{l}.txt")
        with open(path, "w") as f:
            f.write("\n".join(reads(rng, 0, n, l, g)) + "\n")
        print(path)
