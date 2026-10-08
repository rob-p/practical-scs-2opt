"""Faithful reference implementation of Section 2 (forced occurrence counts + base graph)
of 'A Polynomial-Time 2-Approximation for SCS' (OpenAI, Sept 2026). Small instances only."""
from collections import defaultdict


def reduce_instance(strings):
    s = sorted(set(x for x in strings if x))
    return [x for x in s if not any(x != y and x in y for y in s)]


def substrings(strings):
    V = {""}
    for x in strings:
        for i in range(len(x)):
            for j in range(i + 1, len(x) + 1):
                V.add(x[i:j])
    return V


def periods(v):
    """All p < |v| such that v has period p."""
    return [p for p in range(1, len(v)) if all(v[i] == v[i + p] for i in range(len(v) - p))]


def block_functional(s, tmpl, V, m, alphabet):
    """B_A(s;m) for periodic text A with A(i)=tmpl[i mod p], A[0:|s|)=s."""
    p = len(tmpl)
    A = lambda i: tmpl[i % p]
    total = 0
    lam = 0
    while True:  # left extension length lam: middle starts at -lam
        left = "".join(A(i) for i in range(-lam, 0)) + s
        if left not in V:
            break
        rho = 0
        while True:
            mid = left + "".join(A(len(s) + i) for i in range(rho))
            if mid not in V:
                break
            cl, cr = A(-lam - 1), A(len(s) + rho)
            for c in alphabet:
                if c == cl:
                    continue
                for d in alphabet:
                    if d == cr:
                        continue
                    r = c + mid + d
                    if r in V:
                        total += m.get(r, 0)
            rho += 1
        lam += 1
    return total


def compute_counts(strings, use_period_rule=True, stats=None):
    S = reduce_instance(strings)
    V = substrings(S)
    alphabet = sorted(set("".join(S)))
    req = set(S)
    m = {}
    pending = defaultdict(list)  # w -> list of (tmpl, k)
    by_len = sorted((x for x in V if x), key=len, reverse=True)
    n_rules = n_fired = n_bf = 0
    for s in by_len:
        val = max(sum(m.get(c + s, 0) for c in alphabet),
                  sum(m.get(s + c, 0) for c in alphabet),
                  1 if s in req else 0)
        for tmpl, k in pending.pop(s, []):
            n_bf += 1
            val = max(val, block_functional(s, tmpl, V, m, alphabet) + k + 1)
        m[s] = val
        if use_period_rule:
            for p in periods(s):
                tmpl = s[:p]
                n_rules += 1
                n_bf += 1
                if val > block_functional(s, tmpl, V, m, alphabet):
                    n_fired += 1
                    k = 1
                    while len(s) - k * p >= 1:
                        pending[s[:len(s) - k * p]].append((tmpl, k))
                        k += 1
    if stats is not None:
        stats.update(V=len(V), rules=n_rules, fired=n_fired, block_evals=n_bf)
    return S, V, m, alphabet


def base_graph(S, V, m, alphabet):
    u = {}
    d = {}
    for s in V:
        if not s:
            continue
        us = m[s] - sum(m.get(c + s, 0) for c in alphabet)
        ds = m[s] - sum(m.get(s + c, 0) for c in alphabet)
        assert us >= 0 and ds >= 0
        if us:
            u[s] = us  # up edge pref(s) -> s
        if ds:
            d[s] = ds  # down edge s -> suf(s)
    W = sum(m.get(c, 0) for c in alphabet)
    assert sum(u.values()) == W == sum(d.values())
    # balance check
    deg = defaultdict(int)
    for s, k in u.items():
        deg[s[:-1]] -= k; deg[s] += k
    for s, k in d.items():
        deg[s] -= k; deg[s[1:]] += k
    assert all(v == 0 for v in deg.values()), "unbalanced"
    support = set()
    for s in u: support |= {s, s[:-1]}
    for s in d: support |= {s, s[1:]}
    return u, d, W, support
