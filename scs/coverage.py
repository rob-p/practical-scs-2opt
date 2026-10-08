import random, sys
from collections import Counter
from multiprocessing import Pool
from connect import superstring
from search import gen
import exp2, bench

def work(seed):
    rng = random.Random(seed); random.seed(seed); c = Counter()
    for i in range(50):
        if i % 3 == 0: strs = gen(rng)
        elif i % 3 == 1: strs = exp2.gen(random.choice(["periodic", "fib", "rand"]))
        else: strs = bench.reads(rng, 0, rng.randint(5, 40), rng.randint(6, 20), bench.repgenome(rng, 300))
        st = {}; superstring(strs, stats=st)
        for k, v in st["by_op"].items(): c[k] += 1
    return c

if __name__ == "__main__":
    with Pool() as p: res = p.map(work, range(int(sys.argv[1])))
    tot = sum(res, Counter())
    print("instances:", 50 * int(sys.argv[1]))
    for k, v in tot.most_common(): print(f"{v:8}  {k}")
