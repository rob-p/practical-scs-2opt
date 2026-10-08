import random, sys
from multiprocessing import Pool
from counts import compute_counts, base_graph
from compact import compute_counts_compact
from search import gen
import exp2

def work(seed):
    rng = random.Random(seed); random.seed(seed); bad = 0
    for i in range(200):
        strs = gen(rng) if i % 2 else exp2.gen(random.choice(["periodic", "fib", "rand"]))
        u, d, W, _ = base_graph(*compute_counts(strs))
        _, u2, d2, W2 = compute_counts_compact(strs)
        bad += (u, d) != (u2, d2)
    return bad

if __name__ == "__main__":
    with Pool() as p: res = p.map(work, range(1000, 1000 + int(sys.argv[1])))
    print("instances:", 200 * int(sys.argv[1]), "mismatches:", sum(res))
