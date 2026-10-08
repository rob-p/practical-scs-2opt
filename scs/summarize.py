import json, statistics as stt
from collections import defaultdict
res = json.load(open("experiments_results.json"))
fam = defaultdict(list)
for r in res: fam[r["family"]].append(r)
print("Large instances (ratio to lower bound W; OPT unknown, true ratios are at most these):")
print(f"{'family':26}{'instance':24}{'n':>6}{'W':>7}{'ours':>8}{'ours+om':>8}{'greedy':>8}  {'ours/W':>7}{'om/W':>7}{'gr/W':>7}  t_ours t_gr")
for f, rs in fam.items():
    if f.startswith("small"): continue
    for r in rs:
        print(f"{f:26}{r['label']:24}{r['n']:>6}{r['W']:>7}{r['ours']:>8}{r['ours_om']:>8}{r['greedy']:>8}  {r['ours']/r['W']:7.4f}{r['ours_om']/r['W']:7.4f}{r['greedy']/r['W']:7.4f}  {r['t_ours']:5.1f}s {r['t_greedy']:4.2f}s")
print("\nSmall instances vs exact OPT:")
for f, rs in fam.items():
    if not f.startswith("small"): continue
    for key in ["ours", "ours_om", "greedy"]:
        rat = [r[key] / r["opt"] for r in rs]
        print(f"  {f:28}{key:8} mean {stt.mean(rat):.4f}  max {max(rat):.4f}  optimal in {sum(x == 1 for x in rat)}/{len(rat)}")
    om_better = sum(r["ours_om"] < r["greedy"] for r in rs); gr_better = sum(r["greedy"] < r["ours_om"] for r in rs)
    print(f"  {'':28}ours+om shorter than greedy: {om_better}, greedy shorter: {gr_better}")
