"""Which period rules bind? Instrumented copy of compute_counts."""
import random
from collections import defaultdict, Counter
from counts import reduce_instance, substrings, periods, block_functional
from exp2 import gen


def least_period(v):
    return next((p for p in range(1, len(v)) if all(v[i] == v[i + p] for i in range(len(v) - p))), len(v))


def instrumented(strings):
    S = reduce_instance(strings)
    V = substrings(S)
    al = sorted(set("".join(S)))
    m = {}
    pending = defaultdict(list)
    events = []
    isP = lambda x: any(y.startswith(x) for y in S)
    isS = lambda x: any(y.endswith(x) for y in S)
    for s in sorted((x for x in V if x), key=len, reverse=True):
        ext = max(sum(m.get(c + s, 0) for c in al), sum(m.get(s + c, 0) for c in al), 1 if s in S else 0)
        best, src = ext, None
        for tmpl, k, v in pending.pop(s, []):
            b = block_functional(s, tmpl, V, m, al)
            if b + k + 1 > best:
                best, src = b + k + 1, (tmpl, k, v, b)
        m[s] = best
        if src:
            tmpl, k, v, b = src
            events.append(dict(
                w_type=("P" if isP(s) else "") + ("S" if isS(s) else "") or "inner",
                v_type=("P" if isP(v) else "") + ("S" if isS(v) else "") or "inner",
                p_least=len(tmpl) == least_period(v),
                v_maximal_in_run=not any((c + v) in V and c == tmpl[(-1) % len(tmpl)] for c in al)
                and not any((v + c) in V and c == tmpl[len(v) % len(tmpl)] for c in al),
                B_nonzero=b > 0,
                k=k, gain=best - ext))
        for p in periods(s):
            if best > block_functional(s, s[:p], V, m, al):
                for k in range(1, (len(s) - 1) // p + 1):
                    pending[s[:len(s) - k * p]].append((s[:p], k, s))
    return events


if __name__ == "__main__":
    random.seed(11)
    agg = defaultdict(Counter)
    n = 0
    for it in range(1500):
        for e in instrumented(gen(random.choice(["periodic", "fib", "rand"]))):
            n += 1
            for key in ["w_type", "v_type", "p_least", "v_maximal_in_run", "B_nonzero", "k"]:
                agg[key][e[key]] += 1
    print("binding rule applications:", n)
    for k, c in agg.items():
        print(f"  {k:18}", dict(c.most_common()))
