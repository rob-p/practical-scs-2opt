"""Coverage-guided mutation fuzzing for rare connection-phase branches."""
import random, sys, time
from collections import Counter
from multiprocessing import Pool
from connect import superstring
from fuzz_cov import gen

TARGETS = ["lower round trip", "lower to baseline", "easy band", "self-cycle join", "self-cycle contact",
           "host with >1 child groups", "host requester loop", "host contact to eps", "individual block",
           "cycle target internal join", "record same group"]

def feats(strs):
    st = {}
    superstring(strs, stats=st)
    return set(st["by_op"]) | set(st["events"])

def mutate(rng, strs):
    s = list(strs)
    op = rng.random()
    al = sorted(set("".join(s))) or ["a"]
    if op < .25 and len(s) > 1: s.pop(rng.randrange(len(s)))
    elif op < .5:
        x = rng.choice(s); i = rng.randrange(len(x) + 1); u = x[max(0, i - rng.randint(1, 5)):i] or "a"
        s.append((u * 30)[:rng.randint(len(u) + 1, 40)])
    elif op < .75:
        i = rng.randrange(len(s)); x = s[i]; j = rng.randrange(len(x))
        s[i] = x[:j] + rng.choice(al) + x[j + 1:]
    else:
        i = rng.randrange(len(s)); s[i] = s[i] + (s[i][-rng.randint(1, min(6, len(s[i]))):]) * rng.randint(1, 4)
    return [x for x in s if x]

def work(seed):
    rng = random.Random(seed); found = {}; corpus = []; fails = []
    t0 = time.time()
    while time.time() - t0 < float(sys.argv[2]):
        base = rng.choice(corpus) if corpus and rng.random() < .8 else gen(rng)
        strs = mutate(rng, base) if corpus else base
        try:
            f = feats(strs)
        except Exception as e:
            fails.append((strs, repr(e)[:300])); continue
        new = [t for t in TARGETS if t in f and t not in found]
        for t in new: found[t] = strs
        if new or "request sent" in f or "host excursion" in f or ("hard case" in f and rng.random() < .3):
            corpus.append(strs); corpus = corpus[-200:]
    return found, fails

if __name__ == "__main__":
    with Pool() as p: res = p.map(work, range(int(sys.argv[1])))
    found = {}
    for r in res:
        for k, v in r[0].items(): found.setdefault(k, []).append(v)
    fails = [f for r in res for f in r[1]]
    for t in TARGETS: print(f"{len(found.get(t, [])):4} workers hit  {t}", "  e.g." if t in found else "", min(found[t], key=lambda s: sum(map(len, s))) if t in found else "")
    print("FAILURES:", len(fails))
    for f in fails[:5]: print("  ", f)
