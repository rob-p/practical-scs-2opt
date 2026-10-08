"""Connection phase of the SCS 2-approximation (preprint Sections 3-7), on top of compact counts.

Pipeline:  counts (compact.py) -> base graph -> closed walks -> ordered layers (Lemma 3.3)
           -> group processing (Section 5) -> cycle opening (Section 6) -> Euler tour (Section 7).

Coordinates follow the paper: a vertex on a layer is a window [x, e) of a periodic text.
Every planned operation is materialized as an explicit closed walk of hierarchical-graph edges,
and invariants claimed by the paper are asserted along the way.
"""
from collections import defaultdict, Counter
from compact import compute_counts_compact


# ----------------------------------------------------------------------------------------------
# Periodic texts and ordered window lists (Lemma 4.1)
# ----------------------------------------------------------------------------------------------

class Text:
    """Periodic text ch(i) = tmpl[(i + off) mod p]."""
    __slots__ = ("tmpl", "off", "p")

    def __init__(self, tmpl, off=0):
        self.tmpl, self.off, self.p = tmpl, off % len(tmpl), len(tmpl)

    def ch(self, i):
        return self.tmpl[(i + self.off) % self.p]

    def word(self, x, e):
        return "".join(self.tmpl[(i + self.off) % self.p] for i in range(x, e))

    def shifted(self, s):
        """The same text in coordinates i' = i + s."""
        return Text(self.tmpl, self.off - s)

    def equals(self, other):
        if self.p != other.p:
            return False
        return all(self.ch(i) == other.ch(i) for i in range(self.p))


def path_edges(text, a, b, c, d):
    """Lemma 4.1: walk from window [a,b) to [c,d) (a<=c, b<=d) of cost d-b."""
    assert a <= c and b <= d, (a, b, c, d)
    edges = []
    if c <= b:
        for x in range(a, c):
            edges.append(("D", text.word(x, b)))
        for y in range(b, d):
            edges.append(("U", text.word(c, y + 1)))
    else:
        for x in range(a, b):
            edges.append(("D", text.word(x, b)))
        for y in range(b, c):
            edges.append(("U", text.word(y, y + 1)))
            edges.append(("D", text.word(y, y + 1)))
        for y in range(c, d):
            edges.append(("U", text.word(c, y + 1)))
    return edges


def closed_list_walk(text, windows):
    """Follow an ordered list of windows; first and last must spell the same word."""
    assert text.word(*windows[0]) == text.word(*windows[-1]), (windows, text.tmpl)
    edges = []
    for (a, b), (c, d) in zip(windows, windows[1:]):
        edges += path_edges(text, a, b, c, d)
    return edges


def round_trip(word):
    """Closed walk word -> eps -> word, cost |word|."""
    return [("D", word[i:]) for i in range(len(word))] + [("U", word[:i]) for i in range(1, len(word) + 1)]


def up_cost(edges):
    return sum(1 for k, _ in edges if k == "U")


# ----------------------------------------------------------------------------------------------
# Base graph, closed walks, and ordered layers (Section 3)
# ----------------------------------------------------------------------------------------------

def closed_walks(u, d):
    """Decompose the balanced base multigraph into closed walks (Hierholzer per component)."""
    out_up = defaultdict(list)
    for s, k in u.items():
        out_up[s[:-1]].extend([s] * k)
    out_down = {s: k for s, k in d.items()}
    verts = set(out_up) | set(out_down)

    def next_edge(v):
        if out_down.get(v, 0):
            out_down[v] -= 1
            return v[1:]
        lst = out_up.get(v)
        if lst:
            return lst.pop()
        return None

    walks = []
    for v0 in sorted(verts, key=lambda s: (len(s), s)):
        while out_down.get(v0, 0) or out_up.get(v0):
            # Hierholzer: build an Eulerian circuit of the remaining edges reachable from v0
            stack, circuit = [v0], []
            while stack:
                v = stack[-1]
                w = next_edge(v)
                if w is None:
                    circuit.append(stack.pop())
                else:
                    stack.append(w)
            circuit.reverse()
            walks.append(circuit)
    return walks


def least_rotation(s):
    return min(s[i:] + s[:i] for i in range(len(s)))


class Layer:
    __slots__ = ("id", "group", "p", "text", "zs", "budget", "rec_requests")

    def z(self, x):
        q, r = divmod(x, self.p)
        return self.zs[r] + q * self.p

    def f(self, x):
        return self.z(x - 1)

    def l(self, x):
        return self.z(x)

    def windows(self):
        """One period of actual windows [x, e), f(x) <= e <= l(x), x in [0, p)."""
        for x in range(self.p):
            for e in range(self.f(x), self.l(x) + 1):
                yield x, e

    def min_vertex(self):
        """Shortest vertex (start, end) over a period."""
        return min(((x, self.f(x)) for x in range(self.p)), key=lambda w: w[1] - w[0])


class Group:
    __slots__ = ("id", "tmpl", "p", "text", "layers", "t")


def build_layers(u, d):
    walks = closed_walks(u, d)
    by_root = defaultdict(list)
    for circ in walks:
        # Lift to windows: start at v0 = [0, |v0|), up step appends at e, down step advances x.
        v0 = circ[0]
        x, e = 0, len(v0)
        letters, Z = [], {}
        for a, b in zip(circ, circ[1:]):
            if len(b) == len(a) + 1:
                letters.append(b[-1]); e += 1
            else:
                Z[x] = e; x += 1
        P = len(letters)
        assert x == P and e == len(v0) + P
        tmpl = "".join(letters)
        wtext = Text(tmpl, -len(v0))
        # Check that every vertex is the window it should be (paper's lifting claim).
        x, e = 0, len(v0)
        assert wtext.word(0, len(v0)) == v0
        for a, b in zip(circ, circ[1:]):
            if len(b) > len(a):
                e += 1
            else:
                x += 1
            assert wtext.word(x, e) == b
        p = next(q for q in range(1, P + 1) if P % q == 0 and tmpl == tmpl[q:] + tmpl[:q])
        canon = least_rotation(tmpl[:p])
        ctext = Text(canon)
        s = next(s for s in range(p) if all(wtext.ch(i) == ctext.ch(i + s) for i in range(P)))
        by_root[canon].append((P, Z, s))

    groups, layers = [], []
    for canon in sorted(by_root, key=lambda c: (len(c), c)):
        g = Group(); g.id = len(groups); g.tmpl = canon; g.p = p = len(canon); g.text = Text(canon)
        g.t = max(range(p), key=lambda t: canon[t:] + canon[:t])  # distinguished position
        g.layers = []

        def Zg(P, Z, s, xg):
            y = xg - s
            q, r = divmod(y, P)
            return Z[r] + q * P + s

        per_x = []
        for xg in range(p):
            E = sorted(Zg(P, Z, s, xg + r * p) - r * p for (P, Z, s) in by_root[canon] for r in range(P // p))
            per_x.append(E)
        h = len(per_x[0])
        for i in range(h):
            L = Layer(); L.id = len(layers); L.group = g; L.p = p; L.text = g.text
            L.zs = [per_x[x][i] for x in range(p)]; L.budget = p; L.rec_requests = {}
            for x in range(p):
                assert L.z(x - 1) <= L.z(x) and L.z(x - 1) >= x
            layers.append(L); g.layers.append(L)   # index 0 = lowest layer
        groups.append(g)
    return groups, layers


def layer_edge_multiset(layers):
    U, D = defaultdict(int), defaultdict(int)
    for L in layers:
        for x in range(L.p):
            for e in range(L.f(x) + 1, L.l(x) + 1):
                U[L.text.word(x, e)] += 1
            D[L.text.word(x, L.l(x))] += 1
    return dict(U), dict(D)


# ----------------------------------------------------------------------------------------------
# Connection operations (Section 4)
# ----------------------------------------------------------------------------------------------

def band_join(g, Ls, t, h):
    """Lemma 4.1 (band): join ordered layers Ls (low..high) of one group at start t, cost <= h."""
    E, H = Ls[0].l(t), Ls[-1].f(t)
    assert H - E <= h, (H, E, h)
    if E >= H:
        return []
    return closed_list_walk(g.text, [(t, E), (t, H), (t + h, E + h)])


def collective_link(g, Ls, a, b, K):
    """Lemma 4.2: cost-K walk based at R=[a,b) touching all layers Ls (low..high)."""
    rH, rL = Ls[-1].f(a), Ls[0].l(a)
    assert rH <= b + K and rH - rL <= K and rL + K >= b, (rH, rL, a, b, K)
    Y = max(b, rH)
    wins = [(a, b)]
    for L in reversed(Ls):  # top to bottom
        x = a
        while L.z(x) < Y:
            x += 1
        assert x <= a + K and L.f(x) <= Y <= L.l(x)
        wins.append((x, Y))
    wins.append((a + K, b + K))
    return closed_list_walk(g.text, wins)


def individual_link(L, a, b):
    """Section 5 case (ii): cost-p walk [a,b),[a+p,e'),[a+p,b+p); returns edges and short contact."""
    p = L.p
    ep = max(L.f(a + p), b)
    assert ep <= min(L.l(a + p), b + p)
    edges = closed_list_walk(L.text, [(a, b), (a + p, ep), (a + p, b + p)])
    contact = L.text.word(a + p, b) if a + p <= b else ""
    return edges, contact


# ----------------------------------------------------------------------------------------------
# The algorithm
# ----------------------------------------------------------------------------------------------

class Connector:
    def __init__(self, groups, layers, W):
        self.groups, self.layers, self.W = groups, layers, W
        self.walks = []          # (label, edges)
        self.rooted = set()      # layer ids
        self.blocks = []         # outgoing blocks
        self.block_of = {}
        self.requests = defaultdict(dict)   # host layer id -> child group id -> entry
        self.events = Counter()

    def add(self, label, edges):
        if edges:
            self.walks.append((label, edges))

    # --- record search (Section 5.3) -------------------------------------------------------
    def find_record(self, g, w_lo, w_hi):
        A = g.text
        w = A.word(w_lo, w_hi)
        for D in self.layers:
            for x, e in D.windows():
                win = D.text.word(x, e)
                o = win.find(w)
                while o >= 0:
                    delta = w_lo - (x + o)            # A-coord = D-coord + delta
                    if all(D.text.ch(y) == A.ch(y + delta) for y in range(x, e)):
                        aligned = D.text.shifted(delta)   # D's text in A-coordinates
                        if not aligned.equals(A):
                            return D, delta, (x + delta, e + delta)
                    o = win.find(w, o + 1)
        raise AssertionError("count rule guarantees a record; none found")

    # --- host requests (Lemma 4.3) ---------------------------------------------------------
    def fulfill_host(self, host, entries):
        q = host.p
        es = sorted(entries.values(), key=lambda c: (c["p"], c["gid"]))
        self.events["host with %s child groups" % ("1" if len(es) == 1 else ">1")] += 1
        j = es[0]
        aj, bj = j["rec"]
        others = []
        for c in es[1:]:
            a, b = c["rec"]
            m = 0
            while a + m * q < aj or b + m * q < bj:
                m += 1
            while a + (m - 1) * q >= aj and b + (m - 1) * q >= bj:
                m -= 1
            sh = m * q
            others.append(dict(c, rec=(a + sh, b + sh), ends=[e + sh for e in c["ends"]],
                               text=c["text"].shifted(sh)))
        others.sort(key=lambda c: c["rec"])
        T = aj + q
        wins = [(aj, e) for e in sorted(j["ends"])] + [(aj, bj)]
        targets = []
        for c in others:
            a, b = c["rec"]
            assert aj <= a <= T and bj <= b <= bj + q
            for e in sorted(c["ends"]):
                Q = (min(a + c["p"], T), max(e, bj))
                wins.append(Q); targets.append((c, e, Q))
        e0 = min(j["ends"])
        last_b = wins[-1][1]
        wins.append((T, e0 + q))
        edges = closed_list_walk(host.text, wins)
        assert up_cost(edges) == q
        self.add("host excursion", edges)
        # requesters of other children attach at their targets, in their own child text
        for c, e, (s, hh) in targets:
            pi = c["p"]
            assert c["rec"][0] <= s <= c["rec"][0] + pi and e <= hh <= e + pi
            ed = closed_list_walk(c["text"], [(s, hh), (c["rec"][0] + pi, e + pi), (s + pi, hh + pi)])
            assert up_cost(ed) == pi
            self.add("host requester loop", ed)
        # one requester budget at j roots the short contact
        contact = host.text.word(T, last_b) if T <= last_b else ""
        assert len(contact) < j["p"]
        self.add("host contact to eps", round_trip(contact))
        self.rooted.add(host.id)
        for c in es:
            self.rooted.update(c["layers"])

    # --- Section 5 -------------------------------------------------------------------------
    def process_group(self, g):
        p = g.p
        Ls = g.layers
        recipients = [L for L in Ls if self.requests.get(L.id)]
        for L in recipients:
            self.fulfill_host(L, self.requests[L.id])
        base_idx = max((i for i, L in enumerate(Ls) if self.requests.get(L.id)), default=None)
        if base_idx is not None:
            base = Ls[base_idx]
            a, h = next(iter(self.requests[base.id].values()))["rec"]
            assert h - a < 2 * p
            for L in Ls[:base_idx]:
                if L.id in self.rooted:
                    continue
                x, e = L.min_vertex()
                if e - x <= p:
                    self.add("lower round trip", round_trip(L.text.word(x, e)))
                else:
                    e = min(L.l(a), h)
                    assert L.f(a) <= e and p < e - a <= h - a < 2 * p
                    self.add("lower to baseline", closed_list_walk(g.text, [(a, h), (a + p, e + p), (a + p, h + p)]))
                self.rooted.add(L.id)
            upper = Ls[base_idx + 1:]
        else:
            base = None
            upper = Ls
        n = len(upper)
        if n == 0:
            return
        t = g.t
        H = upper[-1].f(t)
        if H - t <= n * p:
            self.add("easy round trip", round_trip(g.text.word(t, H)))
            self.rooted.update(L.id for L in upper)
            return
        if base is not None and base.l(t) >= H - n * p:
            self.add("easy band", band_join(g, [base] + upper, t, n * p))
            self.rooted.update(L.id for L in upper)
            return
        # hard case: threshold search
        k = 1
        while sum(1 for L in Ls if L.l(t) >= H - k * p) > k:
            k += 1
        assert 1 <= k <= n
        K = k * p
        top = Ls[-k:]
        Lstar = top[0]
        assert Lstar.l(t) >= H - (k - 1) * p
        self.events["hard case"] += 1
        D, delta, (a, b) = self.find_record(g, t, H - K)
        self.events["record same group" if D.group is g else "record other group"] += 1
        assert a <= t and b >= H - K
        rH, rL = top[-1].f(a), Lstar.l(a)
        assert rH - rL <= K and rH <= b + K
        in_block = set()
        if rL + K >= b:
            blk = dict(kind="collective", group=g, layers=top, K=K, k=k, rec=(a, b), target=D, delta=delta)
            self.new_block(blk)
            in_block = {L.id for L in top}
        for L in upper:
            if L.id in in_block:
                continue
            assert L.f(a) <= b
            x, e = L.min_vertex()
            if e - x <= p:
                self.add("individual round trip", round_trip(L.text.word(x, e)))
                self.rooted.add(L.id)
            elif L.l(a) + p >= b:
                self.new_block(dict(kind="individual", group=g, layers=[L], K=p, k=1, rec=(a, b), target=D, delta=delta))
            else:
                e = L.f(a)
                assert a + p < e <= L.l(a) < b - p and D.p > p
                ent = self.requests[D.id].setdefault(g.id, dict(
                    gid=g.id, p=p, rec=(a - delta, b - delta), ends=[], layers=[],
                    text=g.text.shifted(-delta)))
                assert ent["rec"] == (a - delta, b - delta)
                self.events["request sent"] += 1
                ent["ends"].append(e - delta); ent["layers"].append(L.id)

    def new_block(self, blk):
        self.events[blk["kind"] + " block"] += 1
        blk["id"] = len(self.blocks)
        self.blocks.append(blk)
        for L in blk["layers"]:
            assert L.id not in self.block_of
            self.block_of[L.id] = blk["id"]

    # --- Section 6 -------------------------------------------------------------------------
    def internal_join(self, blk):
        if blk["kind"] == "individual" or blk["k"] == 1:
            return []
        g = blk["group"]
        return band_join(g, blk["layers"], g.t, (blk["k"] - 1) * g.p)

    def link(self, blk):
        g = blk["group"]
        a, b = blk["rec"]
        if blk["kind"] == "collective":
            return collective_link(g, blk["layers"], a, b, blk["K"])
        return individual_link(blk["layers"][0], a, b)[0]

    def contact_link(self, blk, q):
        """Lemma 6.2: realize blk's link with a contact of length <= q (q = target period)."""
        g = blk["group"]
        a, b = blk["rec"]
        if blk["kind"] == "individual":
            edges, contact = individual_link(blk["layers"][0], a, b)
            assert len(contact) <= q
            return edges, contact
        for L in blk["layers"]:
            x, e = L.min_vertex()
            if e - x <= q:
                return self.link(blk), L.text.word(x, e)
        K, t = blk["K"], g.t
        H = blk["layers"][-1].f(t)
        assert a <= t < H - K <= b and b - t < q
        x = min(t, a + K)
        assert b - x < q
        F = blk["layers"][-1].f(x)
        assert b < F <= H <= b + K
        edges = closed_list_walk(g.text, [(a, b), (x, b), (x, F), (a + K, b + K)])
        return edges, g.text.word(x, b)

    def open_cycles(self):
        nb = len(self.blocks)
        succ = []
        for blk in self.blocks:
            D = blk["target"]
            if D.id in self.rooted:
                succ.append(None)
            else:
                succ.append(self.block_of[D.id])
        color = [0] * nb
        cycles = []
        for s in range(nb):
            path = []
            v = s
            while v is not None and color[v] == 0:
                color[v] = 1; path.append(v); v = succ[v]
            if v is not None and color[v] == 1:
                cycles.append(path[path.index(v):])
            for x in path:
                color[x] = 2
        skip = set()
        for cyc in cycles:
            self.events["cycle len 1" if len(cyc) == 1 else "cycle len >1"] += 1
            if len(cyc) == 1:
                blk = self.blocks[cyc[0]]
                self.add("self-cycle join", self.internal_join(blk))
                a, b = blk["rec"]
                assert b - a < blk["group"].p
                self.add("self-cycle contact", round_trip(blk["target"].text.word(a - blk["delta"], b - blk["delta"])))
                skip.add(cyc[0])
                continue

            def rot_key(blk, length):
                g = blk["group"]
                return "".join(g.text.ch(g.t + i) for i in range(length))
            length = 2 * max(self.blocks[i]["group"].p for i in cyc)
            star = max(cyc, key=lambda i: rot_key(self.blocks[i], length))
            sblk = self.blocks[star]
            tgt_blk = self.blocks[succ[star]]
            q = sblk["target"].p
            assert q == tgt_blk["group"].p
            edges, contact = self.contact_link(sblk, q)
            self.add("cycle selected link", edges)
            self.add("cycle target internal join", self.internal_join(tgt_blk))
            self.add("cycle contact to eps", round_trip(contact))
            skip.update([star, succ[star]])
        for i, blk in enumerate(self.blocks):
            if i not in skip:
                self.add("planned link", self.link(blk))

    def run(self):
        for g in self.groups:   # increasing period, ties by least rotation
            self.process_group(g)
        for L in self.layers:
            assert (L.id in self.rooted) != (L.id in self.block_of), L.id
        self.open_cycles()


# ----------------------------------------------------------------------------------------------
# Euler tour and driver
# ----------------------------------------------------------------------------------------------

def euler_superstring(U, D):
    out_up = defaultdict(list)
    for s, k in U.items():
        out_up[s[:-1]].extend([s] * k)
    out_down = dict(D)

    def nxt(v):
        if out_down.get(v, 0):
            out_down[v] -= 1
            return v[1:]
        lst = out_up.get(v)
        return lst.pop() if lst else None
    stack, circuit = [""], []
    while stack:
        v = stack[-1]
        w = nxt(v)
        if w is None:
            circuit.append(stack.pop())
        else:
            stack.append(w)
    circuit.reverse()
    assert not any(out_down.values()) and not any(out_up.values()), "graph not connected"
    return "".join(b[-1] for a, b in zip(circuit, circuit[1:]) if len(b) > len(a))


def superstring(strings, check=True, stats=None):
    S, u, d, W = compute_counts_compact(strings)
    if not S:
        return ""
    groups, layers = build_layers(u, d)
    if check:
        assert layer_edge_multiset(layers) == (u, d), "layer sorting changed the edge multiset"
        assert sum(L.p for L in layers) == W
    C = Connector(groups, layers, W)
    C.run()
    U, D = defaultdict(int, u), defaultdict(int, d)
    added = 0
    for label, edges in C.walks:
        bal = defaultdict(int)
        for kind, s in edges:
            if kind == "U":
                U[s] += 1; bal[s[:-1]] -= 1; bal[s] += 1; added += 1
            else:
                D[s] += 1; bal[s] -= 1; bal[s[1:]] += 1
        assert not any(bal.values()), label
    assert added <= W, (added, W)
    T = euler_superstring(U, D)
    assert len(T) == W + added
    if check:
        assert all(x in T for x in S)
    if stats is not None:
        cnt = defaultdict(int)
        for label, edges in C.walks:
            cnt[label] += up_cost(edges)
        stats.update(events=dict(C.events), W=W, added=added, groups=len(groups), layers=len(layers),
                     blocks=len(C.blocks), by_op=dict(cnt))
    return T
