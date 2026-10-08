import random
from counts import reduce_instance, periods
def sizes(name, strs):
    S = reduce_instance(strs); L = sum(map(len, S))
    V = {x[i:j] for x in S for i in range(len(x)) for j in range(i+1, len(x)+1)}
    P = {x[:i] for x in S for i in range(1, len(x)+1)}; Sf = {x[i:] for x in S for i in range(len(x))}
    PS = P & Sf
    nper = sum(len(periods(v)) for v in PS)
    nper_V = sum(len(periods(v)) for v in random.sample(sorted(V), min(len(V), 20000))) * len(V) / min(len(V), 20000)
    print(f"{name:34} n={len(S):5} L={L:7} |V|={len(V):9} |P∪S|={len(P|Sf):7} |PS|={len(PS):6}  (v,p) pairs: all V≈{nper_V:10.0f}  PS={nper:7}")
random.seed(3)
g = "".join(random.choice("acgt") for _ in range(20000))
sizes("random genome, 100bp reads", [g[i:i+100] for i in (random.randrange(len(g)-100) for _ in range(600))])
unit = "".join(random.choice("acgt") for _ in range(7))
rep = "".join(random.choice("acgt") for _ in range(3000)) + unit*120 + "".join(random.choice("acgt") for _ in range(3000)) + "".join(random.choice("acgt") for _ in range(30))*40 + "".join(random.choice("acgt") for _ in range(3000))
sizes("genome w/ tandem repeats, 100bp", [rep[i:i+100] for i in (random.randrange(len(rep)-100) for _ in range(600))])
sizes("random binary strings l=60", ["".join(random.choice("ab") for _ in range(60)) for _ in range(600)])
