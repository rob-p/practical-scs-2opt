import random, sys
from multiprocessing import Pool
from counts import compute_counts, base_graph
from compact import compute_counts_compact
from search import gen

def work(seed):
    rng = random.Random(seed); bad = []
    for _ in range(100):
        strs = gen(rng)
        u, d, W, _ = base_graph(*compute_counts(strs))
        _, u2, d2, W2 = compute_counts_compact(strs)
        if (u, d) != (u2, d2): bad.append((strs, W, W2))
    return bad

if __name__ == "__main__":
    for strs in [["aabaa", "baaab"], ["aaaaa"], ["abab", "babb"]]:
        print(strs, base_graph(*compute_counts(strs))[2], compute_counts_compact(strs)[3])
    with Pool() as p: res = p.map(work, range(int(sys.argv[1])))
    bad = [b for r in res for b in r]
    print("instances:", 100 * int(sys.argv[1]), "mismatches:", len(bad))
    for b in bad[:5]: print(" ", b)
