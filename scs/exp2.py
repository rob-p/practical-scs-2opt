import random
from counts import *
import itertools


def ov(a, b):
    for k in range(min(len(a), len(b)) - 1, 0, -1):
        if a.endswith(b[:k]): return k
    return 0


def cyc(S):  # min-weight cycle cover of the distance graph (brute force)
    n = len(S)
    C = [[len(a) - ov(a, b) for b in S] for a in S]
    for i, a in enumerate(S):
        C[i][i] = len(a) - max((k for k in range(1, len(a)) if a.endswith(a[:k])), default=0)
    return min(sum(C[i][p[i]] for i in range(n)) for p in itertools.permutations(range(n)))


def gen(kind):
    if kind == "periodic":
        out = []
        for _ in range(random.randint(2, 7)):
            base = "".join(random.choice("ab") for _ in range(random.randint(1, 4)))
            x = (base * 30)[random.randint(0, 3):][:random.randint(3, 14)]
            if random.random() < .5: x = random.choice("abc") + x
            if random.random() < .5: x = x + random.choice("abc")
            out.append(x)
        return out
    if kind == "fib":
        f = ["b", "a"]
        while len(f[-1]) < 60: f.append(f[-1] + f[-2])
        F = f[-1]
        return [F[i:i + random.randint(4, 12)] for i in random.sample(range(40), random.randint(2, 7))]
    a = random.choice(["ab", "abc"])
    return ["".join(random.choice(a) for _ in range(random.randint(2, 10))) for _ in range(random.randint(2, 7))]


if __name__ == "__main__":
    random.seed(7)
    viol = eq = tot = 0
    gaps = []
    for it in range(1500):
        strs = gen(random.choice(["periodic", "fib", "rand"]))
        S, V, m, al = compute_counts(strs)
        u, d, W, sup = base_graph(S, V, m, al)
        for s in u:
            if not any(x.startswith(s) for x in S): viol += 1; print("UP non-prefix", S, s)
        for s in d:
            if not any(x.endswith(s) for x in S): viol += 1; print("DOWN non-suffix", S, s)
        S0, V0, m0, _ = compute_counts(strs, use_period_rule=False)
        W0 = base_graph(S0, V0, m0, al)[2]
        c = cyc(S); tot += 1; eq += (W0 == c)
        if W0 != c and len(gaps) < 5: gaps.append((S, W0, c))
    print("violations:", viol, " W0==CYC:", eq, "/", tot)
    for g in gaps: print("  W0!=CYC", g)
