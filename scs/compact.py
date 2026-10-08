"""Compact computation of the forced counts (preprint Section 2) on prefixes ∪ suffixes only.

Never enumerates the O(L^2) substring set V. Uses:
  * Theorem 1 (overlap-support note): m(s) = λ(s) off prefixes, m(s) = ρ(s) off suffixes.
  * Theorem 2: period rules only with v, w overlap words (prefix and suffix).
  * Closed form for blocking sums (step 2):
        B_A(w) = ρ(w) - Σ_{n>|w|} d(A[0:n)) - T2(A, w),
        T2(A, w) = Σ_{n>=|w|} Σ_{c != A(n)} Σ_{λ>=0} u(A[-λ:n) c),
    and the trigger U_A(v) = m(v) - B_A(v) = Σ_{n>=|v|} d(A[0:n)) + T2(A, v).
  * λ(s) = Σ u(y) over longer y ending in s;  ρ(s) = Σ d(y) over longer y starting with s.
"""
from collections import defaultdict


class Trie:
    """Weighted trie: each node stores its own weight and the weight of its subtree."""
    __slots__ = ("root",)

    def __init__(self):
        self.root = [{}, 0, 0]  # children, own, subtree

    def add(self, word, w):
        node = self.root
        node[2] += w
        for ch in word:
            nxt = node[0].get(ch)
            if nxt is None:
                nxt = node[0][ch] = [{}, 0, 0]
            node = nxt
            node[2] += w
        node[1] += w

    def find(self, word):
        node = self.root
        for ch in word:
            node = node[0].get(ch)
            if node is None:
                return None
        return node

    def subtree(self, word):
        node = self.find(word)
        return node[2] if node else 0


def reduce_instance(strings):
    s = sorted(set(x for x in strings if x), key=len, reverse=True)
    out = []
    for x in s:  # longest first, so containment only needs checking against kept strings
        if not any(x in y for y in out):
            out.append(x)
    return sorted(out)


def primitive_periods(v):
    """Periods p < |v| of v whose template v[:p] is primitive (non-primitive templates are dominated)."""
    n = len(v)
    fail = [0] * n
    k = 0
    for i in range(1, n):
        while k and v[i] != v[k]:
            k = fail[k - 1]
        if v[i] == v[k]:
            k += 1
        fail[i] = k
    out = []
    b = fail[-1] if n else 0
    while b:
        p = n - b
        t = v[:p]
        # t primitive iff its least period does not divide p properly
        f = [0] * p; kk = 0
        for i in range(1, p):
            while kk and t[i] != t[kk]:
                kk = f[kk - 1]
            if t[i] == t[kk]:
                kk += 1
            f[i] = kk
        q = p - f[-1]
        if q == p or p % q:
            out.append(p)
        b = fail[b - 1]
    return out


def t1(Fd, tmpl, x_len, strict):
    """Σ d(A[0:n)) over n >= x_len (n > x_len if strict), walking the forward d-trie along A."""
    p = len(tmpl)
    node = Fd.root
    for i in range(x_len):
        node = node[0].get(tmpl[i % p])
        if node is None:
            return 0
    total = 0 if strict else node[1]
    n = x_len
    while True:
        node = node[0].get(tmpl[n % p])
        if node is None:
            return total
        total += node[1]
        n += 1


def t2(Fu, tmpl, x_len):
    """Σ_{n>=x_len} Σ_{c != A(n)} Σ_{λ>=0} u(A[-λ:n) c).

    Words A[r - jp : r - jp + len) c are the same string for all j, so walk once per start residue
    r in [0,p) and multiply by the number of admissible shifts j (start <= 0, end >= x_len)."""
    p = len(tmpl)
    total = 0
    for r in range(p):
        jmin = 0 if r == 0 else 1
        node = Fu.root
        ln = 0
        while True:
            pos = r + ln
            nxt_ch = tmpl[pos % p]
            # admissible j: jmin <= j and r - j*p + ln >= x_len  ->  j <= (r + ln - x_len) // p
            mult = (pos - x_len) // p - jmin + 1
            if mult > 0:
                for ch, child in node[0].items():
                    if ch != nxt_ch and child[1]:
                        total += mult * child[1]
            node = node[0].get(nxt_ch)
            if node is None:
                break
            ln += 1
    return total


def compute_counts_compact(strings, stats=None):
    S = reduce_instance(strings)
    req = set(S)
    P = {x[:i] for x in S for i in range(1, len(x) + 1)}
    Sf = {x[i:] for x in S for i in range(len(x))}
    O = P & Sf
    Ru, Fu, Fd = Trie(), Trie(), Trie()   # reversed u-trie (λ), forward u-trie (T2), forward d-trie (ρ, T1)
    u, d, m = {}, {}, {}
    pending = defaultdict(dict)  # w -> {tmpl: max k}
    n_rules = n_fired = 0
    for s in sorted(P | Sf, key=lambda x: (-len(x), x)):  # deterministic tie-break
        lam = Ru.subtree(s[::-1])
        rho = Fd.subtree(s)
        if s in O:
            val = max(lam, rho, 1 if s in req else 0)
            for tmpl, k in pending.pop(s, {}).items():
                B = rho - t1(Fd, tmpl, len(s), True) - t2(Fu, tmpl, len(s))
                val = max(val, B + k + 1)
        elif s in P:
            val = rho          # s not a suffix
        else:
            val = lam          # s not a prefix
        us, ds = val - lam, val - rho
        assert us >= 0 and ds >= 0, (s, val, lam, rho)
        m[s] = val
        if us:
            u[s] = us; Ru.add(s[::-1], us); Fu.add(s, us)
        if ds:
            d[s] = ds; Fd.add(s, ds)
        if s in O:
            for p in primitive_periods(s):
                tmpl = s[:p]
                n_rules += 1
                if t1(Fd, tmpl, len(s), False) + t2(Fu, tmpl, len(s)) > 0:
                    n_fired += 1
                    k = 1
                    while len(s) - k * p >= 1:
                        w = s[:len(s) - k * p]
                        if w in O:
                            pend = pending[w]
                            if pend.get(tmpl, 0) < k:
                                pend[tmpl] = k
                        k += 1
    W = sum(u.values())
    assert W == sum(d.values())
    if stats is not None:
        stats.update(PS=len(P | Sf), O=len(O), rules=n_rules, fired=n_fired)
    return S, u, d, W
