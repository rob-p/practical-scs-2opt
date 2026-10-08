"""Efficient greedy SCS baseline and shared string utilities.

greedy_scs: the classical maximum-overlap greedy, implemented as in Ukkonen / Tarhio-Ukkonen:
  * Aho-Corasick automaton over the inputs; the failure chain of each string's final state lists
    exactly its suffixes that are prefixes of some input, i.e. all candidate overlaps;
  * candidate overlaps are processed in decreasing length; strings sharing a prefix form a
    contiguous range of the sorted input list, scanned with skip pointers past strings that
    already have a predecessor; a union-find over path fragments forbids cycles.
This is equivalent to repeatedly merging the pair with maximum overlap.

order_merge: given any common superstring T, order the inputs by leftmost occurrence in T and
merge consecutive strings with maximum overlap.  The result is never longer than T.
"""
from bisect import bisect_left
from collections import deque


class AhoCorasick:
    def __init__(self, strings):
        self.goto = [{}]
        self.fail = [0]
        self.depth = [0]
        self.term = [-1]            # index of the string ending here, or -1
        for idx, s in enumerate(strings):
            v = 0
            for ch in s:
                nxt = self.goto[v].get(ch)
                if nxt is None:
                    nxt = len(self.goto)
                    self.goto[v][ch] = nxt
                    self.goto.append({}); self.fail.append(0); self.depth.append(self.depth[v] + 1)
                    self.term.append(-1)
                v = nxt
            self.term[v] = idx
        self.dict_link = [0] * len(self.goto)   # nearest proper failure ancestor that is terminal
        order = deque(self.goto[0].values())
        while order:
            v = order.popleft()
            for ch, w in self.goto[v].items():
                f = self.fail[v]
                while f and ch not in self.goto[f]:
                    f = self.fail[f]
                self.fail[w] = self.goto[f].get(ch, 0)
                fw = self.fail[w]
                self.dict_link[w] = fw if self.term[fw] >= 0 else self.dict_link[fw]
                order.append(w)

    def step(self, v, ch):
        while v and ch not in self.goto[v]:
            v = self.fail[v]
        return self.goto[v].get(ch, 0)


def reduce_instance(strings):
    """Drop empty strings, duplicates, and strings contained in another input (Aho-Corasick)."""
    S = sorted(set(x for x in strings if x))
    ac = AhoCorasick(S)
    dead = [False] * len(S)
    for i, x in enumerate(S):
        v = 0
        for pos, ch in enumerate(x):
            v = ac.step(v, ch)
            w = v if ac.term[v] >= 0 else ac.dict_link[v]
            while w:
                j = ac.term[w]
                if j != i:
                    dead[j] = True
                w = ac.dict_link[w]
    return [x for i, x in enumerate(S) if not dead[i]]


def overlap(a, b):
    """Longest proper suffix of a that is a prefix of b (prefix function on b + sep + a)."""
    s = b + "\x00" + a
    pi = [0] * len(s)
    for i in range(1, len(s)):
        k = pi[i - 1]
        while k and s[i] != s[k]:
            k = pi[k - 1]
        if s[i] == s[k]:
            k += 1
        pi[i] = k
    k, cap = pi[-1], min(len(a), len(b)) - 1
    while k > cap:
        k = pi[k - 1]
    return k


def merge_in_order(seq):
    if not seq:
        return ""
    out = [seq[0]]
    for a, b in zip(seq, seq[1:]):
        out.append(b[overlap(a, b):])
    return "".join(out)


def greedy_scs(strings):
    S = reduce_instance(strings)
    n = len(S)
    if n == 0:
        return ""
    S.sort()
    ac = AhoCorasick(S)
    # candidate overlaps: (k, a, node) for each proper suffix of a that is a prefix of some input
    buckets = {}
    for a, x in enumerate(S):
        v = 0
        for ch in x:
            v = ac.step(v, ch)
        v = ac.fail[v]                       # v was x's own node; move to proper suffixes
        while v:
            buckets.setdefault(ac.depth[v], []).append((a, v))
            v = ac.fail[v]
    # map each node to its [lo, hi) range in sorted S via the first string through it
    first_string = {}
    for idx, x in enumerate(S):
        v = 0
        for ch in x:
            v = ac.goto[v][ch]
            if v not in first_string:
                first_string[v] = idx

    succ = [-1] * n
    has_pred = [False] * n
    nxt_free = list(range(n + 1))            # skip pointers to strings without predecessor
    parent = list(range(n))                  # union-find over path fragments

    def find_free(i):
        root = i
        while nxt_free[root] != root:
            root = nxt_free[root]
        while nxt_free[i] != root:
            nxt_free[i], i = root, nxt_free[i]
        return root

    def find(i):
        root = i
        while parent[root] != root:
            root = parent[root]
        while parent[i] != root:
            parent[i], i = root, parent[i]
        return root

    for k in sorted(buckets, reverse=True):
        for a, v in buckets[k]:
            if succ[a] >= 0:
                continue
            pre = S[first_string[v]][:k]
            lo = bisect_left(S, pre)
            hi = bisect_left(S, pre + "\U0010ffff")
            i = find_free(lo)
            while i < hi:
                if find(i) != find(a):
                    succ[a] = i; has_pred[i] = True
                    nxt_free[i] = i + 1
                    parent[find(a)] = find(i)
                    break
                i = find_free(i + 1)
    out = []
    for h in range(n):
        if has_pred[h]:
            continue
        seq = [S[h]]
        v = h
        while succ[v] >= 0:
            v = succ[v]; seq.append(S[v])
        out.append(merge_in_order(seq))
    return "".join(out)


def order_merge(T, strings):
    """Reorder inputs by leftmost occurrence in superstring T and merge greedily; |result| <= |T|."""
    S = reduce_instance(strings)
    ac = AhoCorasick(S)
    first = {}
    v = 0
    for pos, ch in enumerate(T):
        v = ac.step(v, ch)
        w = v if ac.term[v] >= 0 else ac.dict_link[v]
        while w:
            j = ac.term[w]
            if j not in first:
                first[j] = pos - len(S[j]) + 1
            w = ac.dict_link[w]
    assert len(first) == len(S), "T is not a superstring"
    seq = [S[j] for j in sorted(first, key=first.get)]
    R = merge_in_order(seq)
    assert len(R) <= len(T)
    return R
