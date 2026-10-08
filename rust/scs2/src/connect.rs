//! Connection phase of the SCS 2-approximation (preprint Sections 3-7) and Euler-tour output.
//!
//! Port of `scs/connect.py`, reproducing every deterministic choice of the Python reference so the
//! superstrings are identical. Vertices of the hierarchical graph are identified by polynomial
//! hashes of their words (hash, length), so no word is ever materialized as a string; each edge
//! carries the symbols needed to continue the walk (first symbol for down steps, appended symbol
//! for up steps).
//!
//! Two parts differ in method but not in result from the Python version:
//!   * the record search uses an index of layer windows by word and returns the minimum admissible
//!     candidate under Python's scan order (layer id, start, end, offset);
//!   * primitive roots, extremal rotations and alignments use linear-time string algorithms.

use crate::counts::{Counts, Occ, trace};
use crate::strings::{BASE, Sym, Word, extend, mulmod, submod, sym_val};
use rustc_hash::FxHashMap;

// ---------------------------------------------------------------------------------------------
// Hashing helpers
// ---------------------------------------------------------------------------------------------

/// Vertex key: hash combined with length (the empty word has key 0).
pub type VKey = u64;

#[inline]
fn vkey(h: u64, len: usize) -> VKey {
    h ^ (len as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

struct Pow(Vec<u64>);

impl Pow {
    fn new(n: usize) -> Self {
        let mut v = vec![1u64; n + 1];
        for i in 1..=n {
            v[i] = mulmod(v[i - 1], BASE);
        }
        Pow(v)
    }
    #[inline]
    fn get(&self, k: usize) -> u64 {
        self.0[k]
    }
}

// ---------------------------------------------------------------------------------------------
// Periodic texts
// ---------------------------------------------------------------------------------------------

/// Periodic text ch(i) = tmpl[(i + off) mod p].
#[derive(Clone)]
pub struct Text {
    pub tmpl: std::rc::Rc<Vec<Sym>>,
    pub off: i64,
}

impl Text {
    fn new(tmpl: std::rc::Rc<Vec<Sym>>, off: i64) -> Self {
        let p = tmpl.len() as i64;
        Text {
            tmpl,
            off: off.rem_euclid(p),
        }
    }
    #[inline]
    pub fn p(&self) -> i64 {
        self.tmpl.len() as i64
    }
    #[inline]
    pub fn ch(&self, i: i64) -> Sym {
        self.tmpl[(i + self.off).rem_euclid(self.p()) as usize]
    }
    /// The same text in coordinates i' = i + s.
    fn shifted(&self, s: i64) -> Text {
        Text::new(self.tmpl.clone(), self.off - s)
    }
    fn equals(&self, o: &Text) -> bool {
        self.p() == o.p() && (0..self.p()).all(|i| self.ch(i) == o.ch(i))
    }
    fn hash(&self, x: i64, e: i64) -> u64 {
        let mut h = 0;
        for i in x..e {
            h = extend(h, self.ch(i));
        }
        h
    }
}

/// Start of the least rotation of s (or the greatest, if `greater`); the smallest such index.
/// Two-pointer algorithm, O(|s|).
fn extremal_rotation(s: &[Sym], greater: bool) -> usize {
    let n = s.len();
    let (mut i, mut j, mut k) = (0usize, 1usize, 0usize);
    while i < n && j < n && k < n {
        let (x, y) = (s[(i + k) % n], s[(j + k) % n]);
        if x == y {
            k += 1;
            continue;
        }
        // advance the pointer whose rotation is worse
        if (x > y) != greater {
            i += k + 1;
        } else {
            j += k + 1;
        }
        if i == j {
            j += 1;
        }
        k = 0;
    }
    i.min(j)
}

fn rotate(s: &[Sym], k: usize) -> Vec<Sym> {
    s[k..].iter().chain(s[..k].iter()).copied().collect()
}

/// Smallest q dividing |s| such that s is invariant under rotation by q.
fn cyclic_root_len(s: &[Sym]) -> usize {
    let n = s.len();
    let mut pi = vec![0usize; n];
    let mut k = 0;
    for i in 1..n {
        while k > 0 && s[i] != s[k] {
            k = pi[k - 1];
        }
        if s[i] == s[k] {
            k += 1;
        }
        pi[i] = k;
    }
    let q = n - pi[n - 1];
    if n % q == 0 { q } else { n }
}

// ---------------------------------------------------------------------------------------------
// Edges and walks
// ---------------------------------------------------------------------------------------------

/// An edge of the hierarchical graph, by vertex keys.
#[derive(Clone, Copy, Debug)]
pub enum Edge {
    /// up edge parent -> child = parent·c; `first` is the child's first symbol
    Up {
        child: VKey,
        parent: VKey,
        c: Sym,
        first: Sym,
        len: u32,
    },
    /// down edge v -> suf(v); `first` is v's first symbol
    Down {
        v: VKey,
        first: Sym,
        len: u32,
        h: u64,
    },
}

struct Walker<'a> {
    pow: &'a Pow,
}

impl Walker<'_> {
    /// Edges of the path from window [a,b) to [c,d) in `text` (Lemma 4.1), appended to `out`.
    fn path(&self, text: &Text, a: i64, b: i64, c: i64, d: i64, out: &mut Vec<Edge>) {
        assert!(a <= c && b <= d, "unordered windows {a} {b} {c} {d}");
        let mut h = text.hash(a, b);
        let mut x = a;
        let down = |h: &mut u64, x: &mut i64, end: i64, out: &mut Vec<Edge>| {
            let len = (end - *x) as usize;
            let first = text.ch(*x);
            out.push(Edge::Down {
                v: vkey(*h, len),
                first,
                len: len as u32,
                h: *h,
            });
            *h = submod(*h, mulmod(sym_val(first), self.pow.get(len - 1)));
            *x += 1;
        };
        if c <= b {
            while x < c {
                down(&mut h, &mut x, b, out);
            }
            let first = if c < b { text.ch(c) } else { 0 };
            for y in b..d {
                let ch = text.ch(y);
                let nh = extend(h, ch);
                let len = (y + 1 - c) as usize;
                let f = if len == 1 { ch } else { first };
                out.push(Edge::Up {
                    child: vkey(nh, len),
                    parent: vkey(h, len - 1),
                    c: ch,
                    first: f,
                    len: len as u32,
                });
                h = nh;
            }
        } else {
            while x < b {
                down(&mut h, &mut x, b, out);
            }
            for y in b..c {
                let ch = text.ch(y);
                let nh = extend(0, ch);
                out.push(Edge::Up {
                    child: vkey(nh, 1),
                    parent: 0,
                    c: ch,
                    first: ch,
                    len: 1,
                });
                out.push(Edge::Down {
                    v: vkey(nh, 1),
                    first: ch,
                    len: 1,
                    h: nh,
                });
            }
            let first = text.ch(c);
            let mut h = 0u64;
            for y in c..d {
                let ch = text.ch(y);
                let nh = extend(h, ch);
                let len = (y + 1 - c) as usize;
                out.push(Edge::Up {
                    child: vkey(nh, len),
                    parent: vkey(h, len - 1),
                    c: ch,
                    first,
                    len: len as u32,
                });
                h = nh;
            }
        }
    }

    /// Follow an ordered list of windows whose first and last spell the same word.
    fn closed_list(&self, text: &Text, wins: &[(i64, i64)]) -> Vec<Edge> {
        debug_assert_eq!(
            vkey(
                text.hash(wins[0].0, wins[0].1),
                (wins[0].1 - wins[0].0) as usize
            ),
            vkey(
                text.hash(wins[wins.len() - 1].0, wins[wins.len() - 1].1),
                (wins[wins.len() - 1].1 - wins[wins.len() - 1].0) as usize
            )
        );
        let mut out = Vec::new();
        for w in wins.windows(2) {
            self.path(text, w[0].0, w[0].1, w[1].0, w[1].1, &mut out);
        }
        out
    }

    /// Closed walk word -> eps -> word for the window [x, e) of `text`.
    fn round_trip(&self, text: &Text, x: i64, e: i64) -> Vec<Edge> {
        let mut out = Vec::new();
        if e > x {
            self.path(text, x, e, e, e, &mut out); // down to eps
            self.path(text, x, x, x, e, &mut out); // up again
        }
        out
    }
}

fn up_cost(edges: &[Edge]) -> i64 {
    edges
        .iter()
        .filter(|e| matches!(e, Edge::Up { .. }))
        .count() as i64
}

// ---------------------------------------------------------------------------------------------
// Layers (Section 3)
// ---------------------------------------------------------------------------------------------

pub struct Layer {
    pub id: usize,
    pub group: usize,
    pub p: i64,
    pub zs: Vec<i64>,
}

impl Layer {
    #[inline]
    pub fn z(&self, x: i64) -> i64 {
        let (q, r) = (x.div_euclid(self.p), x.rem_euclid(self.p));
        self.zs[r as usize] + q * self.p
    }
    #[inline]
    pub fn f(&self, x: i64) -> i64 {
        self.z(x - 1)
    }
    #[inline]
    pub fn l(&self, x: i64) -> i64 {
        self.z(x)
    }
    fn min_vertex(&self) -> (i64, i64) {
        let mut best = (0, self.f(0));
        for x in 1..self.p {
            if self.f(x) - x < best.1 - best.0 {
                best = (x, self.f(x));
            }
        }
        best
    }
}

pub struct Group {
    pub id: usize,
    pub p: i64,
    pub text: Text,
    pub layers: Vec<usize>,
    pub t: i64,
}

/// A base-graph vertex: its word as an occurrence in the inputs.
#[derive(Clone, Copy)]
struct BVert {
    occ: Occ,
    key: VKey,
}

struct Base {
    verts: Vec<BVert>,
    /// up edges in Python insertion order: (child vertex, multiplicity)
    u: Vec<(usize, i64)>,
    d: Vec<(usize, i64)>,
    /// parent vertex of each u edge's child, suffix vertex of each d edge's source (parallel to u, d)
    u_par: Vec<usize>,
    d_suf: Vec<usize>,
}

fn build_base(c: &Counts, hs: &crate::strings::SliceHasher) -> Base {
    let word = |o: &Occ| &c.strings[o.si as usize][o.a as usize..o.b as usize];
    // Python order: decreasing length, then lexicographic
    use rayon::prelude::*;
    let mut u: Vec<(Occ, i64)> = c.u.clone();
    u.par_sort_by(|x, y| {
        y.0.len()
            .cmp(&x.0.len())
            .then_with(|| word(&x.0).cmp(word(&y.0)))
    });
    let mut d: Vec<(Occ, i64)> = c.d.clone();
    d.par_sort_by(|x, y| {
        y.0.len()
            .cmp(&x.0.len())
            .then_with(|| word(&x.0).cmp(word(&y.0)))
    });
    let mut ids: FxHashMap<VKey, usize> = FxHashMap::default();
    let mut verts: Vec<BVert> = Vec::new();
    let mut vid = |o: Occ| -> usize {
        let key = vkey(hs.hash(o.si as usize, o.a as usize, o.b as usize), o.len());
        *ids.entry(key).or_insert_with(|| {
            verts.push(BVert { occ: o, key });
            verts.len() - 1
        })
    };
    let mut bu = Vec::with_capacity(u.len());
    let mut u_par = Vec::with_capacity(u.len());
    for (o, k) in &u {
        let child = vid(*o);
        u_par.push(vid(Occ { b: o.b - 1, ..*o }));
        bu.push((child, *k));
    }
    let mut bd = Vec::with_capacity(d.len());
    let mut d_suf = Vec::with_capacity(d.len());
    for (o, k) in &d {
        let v = vid(*o);
        d_suf.push(vid(Occ { a: o.a + 1, ..*o }));
        bd.push((v, *k));
    }
    Base {
        verts,
        u: bu,
        d: bd,
        u_par,
        d_suf,
    }
}

/// Closed-walk decomposition of the base graph exactly as `closed_walks` in connect.py.
fn closed_walks(c: &Counts, base: &Base) -> Vec<Vec<usize>> {
    let nv = base.verts.len();
    let mut out_up: Vec<Vec<usize>> = vec![Vec::new(); nv];
    for (&(child, k), &par) in base.u.iter().zip(&base.u_par) {
        for _ in 0..k {
            out_up[par].push(child);
        }
    }
    let mut out_down: Vec<i64> = vec![0; nv];
    let mut suf: Vec<usize> = vec![usize::MAX; nv];
    for (&(v, k), &sv) in base.d.iter().zip(&base.d_suf) {
        out_down[v] += k;
        suf[v] = sv;
    }
    let word = |i: usize| {
        let o = base.verts[i].occ;
        &c.strings[o.si as usize][o.a as usize..o.b as usize]
    };
    // Python: verts = keys of out_up (parents) | keys of out_down
    let mut verts: Vec<usize> = (0..nv)
        .filter(|&i| !out_up[i].is_empty() || out_down[i] > 0)
        .collect();
    {
        use rayon::prelude::*;
        verts.par_sort_by(|&x, &y| {
            word(x)
                .len()
                .cmp(&word(y).len())
                .then_with(|| word(x).cmp(word(y)))
        });
    }
    let mut walks = Vec::new();
    for &v0 in &verts {
        while out_down[v0] > 0 || !out_up[v0].is_empty() {
            let mut stack = vec![v0];
            let mut circuit = Vec::new();
            while let Some(&v) = stack.last() {
                let w = if out_down[v] > 0 {
                    out_down[v] -= 1;
                    Some(suf[v])
                } else {
                    out_up[v].pop()
                };
                match w {
                    Some(w) => stack.push(w),
                    None => circuit.push(stack.pop().unwrap()),
                }
            }
            circuit.reverse();
            walks.push(circuit);
        }
    }
    walks
}

fn build_layers(c: &Counts, base: &Base, walks: &[Vec<usize>]) -> (Vec<Group>, Vec<Layer>) {
    let word = |i: usize| {
        let o = base.verts[i].occ;
        &c.strings[o.si as usize][o.a as usize..o.b as usize]
    };
    // canon -> list of (P, Z, s), in walk order; groups keyed by canonical primitive rotation
    let mut by_root: FxHashMap<Vec<Sym>, Vec<(i64, Vec<i64>, i64)>> = FxHashMap::default();
    let mut root_order: Vec<Vec<Sym>> = Vec::new();
    for circ in walks {
        let len0 = word(circ[0]).len() as i64;
        let mut x = 0i64;
        let mut letters: Vec<Sym> = Vec::new();
        let mut z: Vec<i64> = Vec::new();
        let mut e = len0;
        for pair in circ.windows(2) {
            let (wa, wb) = (word(pair[0]), word(pair[1]));
            if wb.len() == wa.len() + 1 {
                letters.push(*wb.last().unwrap());
                e += 1;
            } else {
                z.push(e);
                x += 1;
            }
        }
        let pp = letters.len() as i64;
        assert!(x == pp && e == len0 + pp && pp > 0);
        let p = cyclic_root_len(&letters);
        let root = &letters[..p];
        let c0 = extremal_rotation(root, false);
        let canon = rotate(root, c0);
        // walk text ch(i) = root[(i - len0) mod p] = canon[(i - len0 - c0) mod p]; shift s
        let s = (-len0 - c0 as i64).rem_euclid(p as i64);
        let entry = by_root.entry(canon.clone()).or_insert_with(|| {
            root_order.push(canon.clone());
            Vec::new()
        });
        entry.push((pp, z, s));
    }
    root_order.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    let mut groups = Vec::new();
    let mut layers: Vec<Layer> = Vec::new();
    for canon in root_order {
        let walks_g = &by_root[&canon];
        let p = canon.len() as i64;
        let t = extremal_rotation(&canon, true) as i64;
        let gid = groups.len();
        let text = Text::new(std::rc::Rc::new(canon), 0);
        let zg = |pp: i64, z: &Vec<i64>, s: i64, xg: i64| -> i64 {
            let y = xg - s;
            z[y.rem_euclid(pp) as usize] + y.div_euclid(pp) * pp + s
        };
        let mut per_x: Vec<Vec<i64>> = Vec::with_capacity(p as usize);
        for xg in 0..p {
            let mut ev: Vec<i64> = Vec::new();
            for (pp, z, s) in walks_g {
                for r in 0..pp / p {
                    ev.push(zg(*pp, z, *s, xg + r * p) - r * p);
                }
            }
            ev.sort_unstable();
            per_x.push(ev);
        }
        let h = per_x[0].len();
        let mut ids = Vec::with_capacity(h);
        for i in 0..h {
            let zs: Vec<i64> = (0..p as usize).map(|x| per_x[x][i]).collect();
            let l = Layer {
                id: layers.len(),
                group: gid,
                p,
                zs,
            };
            for x in 0..p {
                assert!(l.z(x - 1) <= l.z(x) && l.z(x - 1) >= x);
            }
            ids.push(l.id);
            layers.push(l);
        }
        groups.push(Group {
            id: gid,
            p,
            text,
            layers: ids,
            t,
        });
    }
    (groups, layers)
}

// ---------------------------------------------------------------------------------------------
// The algorithm (Sections 4-6)
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Request {
    gid: usize,
    p: i64,
    rec: (i64, i64),
    ends: Vec<i64>,
    layers: Vec<usize>,
    text: Text,
}

#[derive(Clone)]
struct Block {
    collective: bool,
    group: usize,
    layers: Vec<usize>, // low..high
    k: i64,
    big_k: i64,
    rec: (i64, i64),
    target: usize,
    delta: i64,
}

#[derive(Default, Debug, Clone)]
pub struct ConnectStats {
    pub groups: usize,
    pub layers: usize,
    pub blocks: usize,
    pub hard_cases: usize,
    pub requests: usize,
    pub cycles: usize,
    pub added: i64,
}

struct Connector<'a> {
    groups: &'a [Group],
    layers: &'a [Layer],
    w: Walker<'a>,
    walks: Vec<Vec<Edge>>,
    rooted: Vec<bool>,
    blocks: Vec<Block>,
    block_of: Vec<usize>,
    /// host layer -> child requests in insertion order of child group
    requests: Vec<Vec<Request>>,
    /// layer windows by word key: (layer id, x, e), in Python scan order per key
    win_index: FxHashMap<VKey, Vec<(u32, i64, i64)>>,
    /// per group: prefix hashes of its text over [0, p + maxlen]
    gph: Vec<Vec<u64>>,
    pow: &'a Pow,
    maxlen: i64,
    stats: ConnectStats,
}

impl Connector<'_> {
    fn add(&mut self, edges: Vec<Edge>) {
        if !edges.is_empty() {
            self.walks.push(edges);
        }
    }

    fn text_of(&self, l: usize) -> &Text {
        &self.groups[self.layers[l].group].text
    }

    /// Hash of A[a:b) for the text of group g (any integer a).
    fn ghash(&self, g: usize, a: i64, b: i64) -> u64 {
        let p = self.groups[g].p;
        let a0 = a.rem_euclid(p);
        let b0 = a0 + (b - a);
        let ph = &self.gph[g];
        submod(
            ph[b0 as usize],
            mulmod(ph[a0 as usize], self.pow.get((b - a) as usize)),
        )
    }

    /// Record search (Section 5.3): minimum admissible (layer, x, e, offset) as in Python.
    fn find_record(&self, g: usize, w_lo: i64, w_hi: i64) -> (usize, i64, (i64, i64)) {
        let a_text = &self.groups[g].text;
        let mut best: Option<(u32, i64, i64, i64, i64)> = None; // (D, x, e, o, delta)
        let maxlen = self.maxlen;
        for a in (w_hi - maxlen).max(w_lo - maxlen)..=w_lo {
            let mut b = w_hi;
            while b - a <= maxlen {
                let key = vkey(self.ghash(g, a, b), (b - a) as usize);
                if let Some(list) = self.win_index.get(&key) {
                    for &(d, x, e) in list {
                        if e - x != b - a {
                            continue;
                        }
                        let delta = a - x;
                        let o = w_lo - a;
                        let cand = (d, x, e, o, delta);
                        if let Some(bst) = best {
                            if (cand.0, cand.1, cand.2, cand.3) >= (bst.0, bst.1, bst.2, bst.3) {
                                continue;
                            }
                        }
                        let dl = &self.layers[d as usize];
                        let aligned = self.groups[dl.group].text.shifted(delta);
                        if !aligned.equals(a_text) {
                            best = Some(cand);
                        }
                    }
                }
                b += 1;
            }
        }
        let (d, x, e, _, delta) = best.expect("count rule guarantees a record; none found");
        (d as usize, delta, (x + delta, e + delta))
    }

    fn band_join(&self, g: usize, ls: &[usize], t: i64, h: i64) -> Vec<Edge> {
        let (e, hh) = (
            self.layers[ls[0]].l(t),
            self.layers[*ls.last().unwrap()].f(t),
        );
        assert!(hh - e <= h);
        if e >= hh {
            return Vec::new();
        }
        self.w
            .closed_list(&self.groups[g].text, &[(t, e), (t, hh), (t + h, e + h)])
    }

    fn collective_link(&self, g: usize, ls: &[usize], a: i64, b: i64, big_k: i64) -> Vec<Edge> {
        let top = &self.layers[*ls.last().unwrap()];
        let (rh, rl) = (top.f(a), self.layers[ls[0]].l(a));
        assert!(rh <= b + big_k && rh - rl <= big_k && rl + big_k >= b);
        let y = b.max(rh);
        let mut wins = vec![(a, b)];
        for &li in ls.iter().rev() {
            let l = &self.layers[li];
            let mut x = a;
            while l.z(x) < y {
                x += 1;
            }
            assert!(x <= a + big_k && l.f(x) <= y && y <= l.l(x));
            wins.push((x, y));
        }
        wins.push((a + big_k, b + big_k));
        self.w.closed_list(&self.groups[g].text, &wins)
    }

    /// Returns edges and the contact window (empty if it passes through eps).
    fn individual_link(&self, li: usize, a: i64, b: i64) -> (Vec<Edge>, (i64, i64)) {
        let l = &self.layers[li];
        let p = l.p;
        let ep = l.f(a + p).max(b);
        assert!(ep <= l.l(a + p).min(b + p));
        let edges = self
            .w
            .closed_list(self.text_of(li), &[(a, b), (a + p, ep), (a + p, b + p)]);
        let contact = if a + p <= b { (a + p, b) } else { (0, 0) };
        (edges, contact)
    }

    fn fulfill_host(&mut self, host: usize, entries: Vec<Request>) {
        let q = self.layers[host].p;
        let mut es = entries;
        es.sort_by_key(|c| (c.p, c.gid));
        let j = es[0].clone();
        let (aj, bj) = j.rec;
        let mut others: Vec<Request> = Vec::new();
        for c in &es[1..] {
            let (a, b) = c.rec;
            let mut m = 0;
            while a + m * q < aj || b + m * q < bj {
                m += 1;
            }
            while a + (m - 1) * q >= aj && b + (m - 1) * q >= bj {
                m -= 1;
            }
            let sh = m * q;
            others.push(Request {
                rec: (a + sh, b + sh),
                ends: c.ends.iter().map(|e| e + sh).collect(),
                text: c.text.shifted(sh),
                ..c.clone()
            });
        }
        others.sort_by_key(|c| c.rec);
        let tt = aj + q;
        let mut jends = j.ends.clone();
        jends.sort_unstable();
        let mut wins: Vec<(i64, i64)> = jends.iter().map(|&e| (aj, e)).collect();
        wins.push((aj, bj));
        let mut targets = Vec::new();
        for (ci, c) in others.iter().enumerate() {
            let (a, b) = c.rec;
            assert!(aj <= a && a <= tt && bj <= b && b <= bj + q);
            let mut ends = c.ends.clone();
            ends.sort_unstable();
            for e in ends {
                let qq = ((a + c.p).min(tt), e.max(bj));
                wins.push(qq);
                targets.push((ci, e, qq));
            }
        }
        let e0 = jends[0];
        let last_b = wins.last().unwrap().1;
        wins.push((tt, e0 + q));
        let host_text = self.text_of(host).clone();
        let edges = self.w.closed_list(&host_text, &wins);
        assert_eq!(up_cost(&edges), q);
        self.add(edges);
        for (ci, e, (s, hh)) in targets {
            let c = &others[ci];
            let pi = c.p;
            assert!(c.rec.0 <= s && s <= c.rec.0 + pi && e <= hh && hh <= e + pi);
            let ed = self.w.closed_list(
                &c.text,
                &[(s, hh), (c.rec.0 + pi, e + pi), (s + pi, hh + pi)],
            );
            assert_eq!(up_cost(&ed), pi);
            self.add(ed);
        }
        if tt <= last_b {
            assert!(last_b - tt < j.p);
            let rt = self.w.round_trip(&host_text, tt, last_b);
            self.add(rt);
        }
        self.rooted[host] = true;
        for c in &es {
            for &l in &c.layers {
                self.rooted[l] = true;
            }
        }
    }

    fn new_block(&mut self, blk: Block) {
        let id = self.blocks.len();
        for &l in &blk.layers {
            assert_eq!(self.block_of[l], usize::MAX);
            self.block_of[l] = id;
        }
        self.blocks.push(blk);
    }

    fn process_group(&mut self, gi: usize) {
        let p = self.groups[gi].p;
        let ls: Vec<usize> = self.groups[gi].layers.clone();
        let text = self.groups[gi].text.clone();
        for &l in &ls {
            if !self.requests[l].is_empty() {
                let ent = self.requests[l].clone();
                self.fulfill_host(l, ent);
            }
        }
        let base_idx = ls.iter().rposition(|&l| !self.requests[l].is_empty());
        let (base, upper): (Option<usize>, Vec<usize>) = match base_idx {
            Some(bi) => {
                let base = ls[bi];
                let (a, h) = self.requests[base][0].rec;
                assert!(h - a < 2 * p);
                for &l in &ls[..bi] {
                    if self.rooted[l] {
                        continue;
                    }
                    let (x, e) = self.layers[l].min_vertex();
                    let edges = if e - x <= p {
                        self.w.round_trip(&text, x, e)
                    } else {
                        let e = self.layers[l].l(a).min(h);
                        assert!(
                            self.layers[l].f(a) <= e
                                && p < e - a
                                && e - a <= h - a
                                && h - a < 2 * p
                        );
                        self.w
                            .closed_list(&text, &[(a, h), (a + p, e + p), (a + p, h + p)])
                    };
                    self.add(edges);
                    self.rooted[l] = true;
                }
                (Some(base), ls[bi + 1..].to_vec())
            }
            None => (None, ls.clone()),
        };
        let n = upper.len() as i64;
        if n == 0 {
            return;
        }
        let t = self.groups[gi].t;
        let hh = self.layers[*upper.last().unwrap()].f(t);
        if hh - t <= n * p {
            let rt = self.w.round_trip(&text, t, hh);
            self.add(rt);
            for &l in &upper {
                self.rooted[l] = true;
            }
            return;
        }
        if let Some(b) = base {
            if self.layers[b].l(t) >= hh - n * p {
                let mut band = vec![b];
                band.extend(&upper);
                let e = self.band_join(gi, &band, t, n * p);
                self.add(e);
                for &l in &upper {
                    self.rooted[l] = true;
                }
                return;
            }
        }
        // hard case: threshold search
        let mut k = 1i64;
        while ls
            .iter()
            .filter(|&&l| self.layers[l].l(t) >= hh - k * p)
            .count() as i64
            > k
        {
            k += 1;
        }
        assert!(1 <= k && k <= n);
        let big_k = k * p;
        let top: Vec<usize> = ls[ls.len() - k as usize..].to_vec();
        let lstar = top[0];
        assert!(self.layers[lstar].l(t) >= hh - (k - 1) * p);
        self.stats.hard_cases += 1;
        let (d, delta, (a, b)) = self.find_record(gi, t, hh - big_k);
        assert!(a <= t && b >= hh - big_k);
        let (rh, rl) = (
            self.layers[*top.last().unwrap()].f(a),
            self.layers[lstar].l(a),
        );
        assert!(rh - rl <= big_k && rh <= b + big_k);
        let mut in_block = vec![];
        if rl + big_k >= b {
            self.new_block(Block {
                collective: true,
                group: gi,
                layers: top.clone(),
                k,
                big_k,
                rec: (a, b),
                target: d,
                delta,
            });
            in_block = top.clone();
        }
        for &l in &upper {
            if in_block.contains(&l) {
                continue;
            }
            let lay = &self.layers[l];
            assert!(lay.f(a) <= b);
            let (x, e) = lay.min_vertex();
            if e - x <= p {
                let rt = self.w.round_trip(&text, x, e);
                self.add(rt);
                self.rooted[l] = true;
            } else if lay.l(a) + p >= b {
                self.new_block(Block {
                    collective: false,
                    group: gi,
                    layers: vec![l],
                    k: 1,
                    big_k: p,
                    rec: (a, b),
                    target: d,
                    delta,
                });
            } else {
                let e = lay.f(a);
                assert!(a + p < e && e <= lay.l(a) && lay.l(a) < b - p && self.layers[d].p > p);
                self.stats.requests += 1;
                let reqs = &mut self.requests[d];
                let pos = reqs.iter().position(|r| r.gid == gi);
                let ent = match pos {
                    Some(i) => &mut reqs[i],
                    None => {
                        reqs.push(Request {
                            gid: gi,
                            p,
                            rec: (a - delta, b - delta),
                            ends: vec![],
                            layers: vec![],
                            text: text.shifted(-delta),
                        });
                        reqs.last_mut().unwrap()
                    }
                };
                assert_eq!(ent.rec, (a - delta, b - delta));
                ent.ends.push(e - delta);
                ent.layers.push(l);
            }
        }
    }

    fn internal_join(&self, blk: &Block) -> Vec<Edge> {
        if !blk.collective || blk.k == 1 {
            return Vec::new();
        }
        let g = &self.groups[blk.group];
        self.band_join(blk.group, &blk.layers, g.t, (blk.k - 1) * g.p)
    }

    fn link(&self, blk: &Block) -> Vec<Edge> {
        let (a, b) = blk.rec;
        if blk.collective {
            self.collective_link(blk.group, &blk.layers, a, b, blk.big_k)
        } else {
            self.individual_link(blk.layers[0], a, b).0
        }
    }

    /// Lemma 6.2: the link with a contact window of length <= q, in the given text.
    fn contact_link(&self, blk: &Block, q: i64) -> (Vec<Edge>, Text, (i64, i64)) {
        let g = &self.groups[blk.group];
        let (a, b) = blk.rec;
        if !blk.collective {
            let (edges, contact) = self.individual_link(blk.layers[0], a, b);
            assert!(contact.1 - contact.0 <= q);
            return (edges, g.text.clone(), contact);
        }
        for &l in &blk.layers {
            let (x, e) = self.layers[l].min_vertex();
            if e - x <= q {
                return (self.link(blk), g.text.clone(), (x, e));
            }
        }
        let (big_k, t) = (blk.big_k, g.t);
        let hh = self.layers[*blk.layers.last().unwrap()].f(t);
        assert!(a <= t && t < hh - big_k && hh - big_k <= b && b - t < q);
        let x = t.min(a + big_k);
        assert!(b - x < q);
        let ff = self.layers[*blk.layers.last().unwrap()].f(x);
        assert!(b < ff && ff <= hh && hh <= b + big_k);
        let edges = self
            .w
            .closed_list(&g.text, &[(a, b), (x, b), (x, ff), (a + big_k, b + big_k)]);
        (edges, g.text.clone(), (x, b))
    }

    fn open_cycles(&mut self) {
        let nb = self.blocks.len();
        let succ: Vec<Option<usize>> = self
            .blocks
            .iter()
            .map(|b| {
                if self.rooted[b.target] {
                    None
                } else {
                    Some(self.block_of[b.target])
                }
            })
            .collect();
        let mut color = vec![0u8; nb];
        let mut cycles: Vec<Vec<usize>> = Vec::new();
        for s in 0..nb {
            let mut path = Vec::new();
            let mut v = Some(s);
            while let Some(x) = v {
                if color[x] != 0 {
                    break;
                }
                color[x] = 1;
                path.push(x);
                v = succ[x];
            }
            if let Some(x) = v {
                if color[x] == 1 {
                    let i = path.iter().position(|&y| y == x).unwrap();
                    cycles.push(path[i..].to_vec());
                }
            }
            for &x in &path {
                color[x] = 2;
            }
        }
        let mut skip = vec![false; nb];
        for cyc in cycles {
            self.stats.cycles += 1;
            if cyc.len() == 1 {
                let blk = self.blocks[cyc[0]].clone();
                let e = self.internal_join(&blk);
                self.add(e);
                let (a, b) = blk.rec;
                assert!(b - a < self.groups[blk.group].p);
                let tt = self.text_of(blk.target).clone();
                let rt = self.w.round_trip(&tt, a - blk.delta, b - blk.delta);
                self.add(rt);
                skip[cyc[0]] = true;
                continue;
            }
            let length = 2 * cyc
                .iter()
                .map(|&i| self.groups[self.blocks[i].group].p)
                .max()
                .unwrap();
            let rot = |i: usize| -> Vec<Sym> {
                let g = &self.groups[self.blocks[i].group];
                (0..length).map(|k| g.text.ch(g.t + k)).collect()
            };
            let mut star = cyc[0];
            let mut best = rot(star);
            for &i in &cyc[1..] {
                let r = rot(i);
                if r > best {
                    best = r;
                    star = i;
                }
            }
            let sblk = self.blocks[star].clone();
            let tgt = succ[star].unwrap();
            let tblk = self.blocks[tgt].clone();
            let q = self.layers[sblk.target].p;
            assert_eq!(q, self.groups[tblk.group].p);
            let (edges, ctext, (cx, ce)) = self.contact_link(&sblk, q);
            self.add(edges);
            let e = self.internal_join(&tblk);
            self.add(e);
            let rt = self.w.round_trip(&ctext, cx, ce);
            self.add(rt);
            skip[star] = true;
            skip[tgt] = true;
        }
        for i in 0..nb {
            if !skip[i] {
                let blk = self.blocks[i].clone();
                let e = self.link(&blk);
                self.add(e);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Driver and Euler tour (Section 7)
// ---------------------------------------------------------------------------------------------

/// Ordered multiset of up edges (insertion order of first occurrence) and down-edge counts.
struct Graph {
    up_index: FxHashMap<VKey, usize>,
    /// (child, parent, appended symbol, count)
    up: Vec<(VKey, VKey, Sym, i64)>,
    down: FxHashMap<VKey, (i64, Sym, u32, u64)>,
}

impl Graph {
    fn add_up(&mut self, child: VKey, parent: VKey, c: Sym, k: i64) {
        match self.up_index.get(&child) {
            Some(&i) => self.up[i].3 += k,
            None => {
                self.up_index.insert(child, self.up.len());
                self.up.push((child, parent, c, k));
            }
        }
    }
    fn add_down(&mut self, v: VKey, first: Sym, len: u32, h: u64, k: i64) {
        self.down.entry(v).or_insert((0, first, len, h)).0 += k;
    }
}

pub fn superstring(c: &Counts) -> (Word, ConnectStats) {
    if c.strings.is_empty() {
        return (Vec::new(), ConnectStats::default());
    }
    let maxlen = c.strings.iter().map(|s| s.len()).max().unwrap() as i64;
    let mut tt = std::time::Instant::now();
    let hs = crate::strings::SliceHasher::new(&c.strings);
    let base = build_base(c, &hs);
    trace("base graph", &mut tt);
    let walks = closed_walks(c, &base);
    trace("closed walks", &mut tt);
    let (groups, layers) = build_layers(c, &base, &walks);
    trace("layers", &mut tt);
    let pmax = groups.iter().map(|g| g.p).max().unwrap_or(1);
    let pow = Pow::new((2 * (pmax + maxlen) + 4) as usize);
    // prefix hashes of each group text over [0, p + maxlen]
    let gph: Vec<Vec<u64>> = groups
        .iter()
        .map(|g| {
            let mut v = vec![0u64];
            let mut h = 0;
            for i in 0..g.p + maxlen + 1 {
                h = extend(h, g.text.ch(i));
                v.push(h);
            }
            v
        })
        .collect();
    // index of layer windows (one period per layer), in Python scan order
    let mut win_index: FxHashMap<VKey, Vec<(u32, i64, i64)>> = FxHashMap::default();
    for l in &layers {
        let ph = &gph[l.group];
        for x in 0..l.p {
            for e in l.f(x)..=l.l(x) {
                let h = submod(
                    ph[e as usize],
                    mulmod(ph[x as usize], pow.get((e - x) as usize)),
                );
                win_index
                    .entry(vkey(h, (e - x) as usize))
                    .or_default()
                    .push((l.id as u32, x, e));
            }
        }
    }
    let nl = layers.len();
    trace("window index", &mut tt);
    let w_total: i64 = c.w;
    let mut conn = Connector {
        groups: &groups,
        layers: &layers,
        w: Walker { pow: &pow },
        walks: Vec::new(),
        rooted: vec![false; nl],
        blocks: Vec::new(),
        block_of: vec![usize::MAX; nl],
        requests: vec![Vec::new(); nl],
        win_index,
        gph,
        pow: &pow,
        maxlen,
        stats: ConnectStats {
            groups: groups.len(),
            layers: nl,
            ..Default::default()
        },
    };
    for gi in 0..groups.len() {
        conn.process_group(gi);
    }
    for l in 0..nl {
        assert!(
            conn.rooted[l] != (conn.block_of[l] != usize::MAX),
            "layer {l} neither rooted nor blocked"
        );
    }
    conn.open_cycles();
    trace("groups + cycles", &mut tt);
    conn.stats.blocks = conn.blocks.len();

    // ---- final graph: base edges (Python order) then added walks ----
    let mut g = Graph {
        up_index: FxHashMap::default(),
        up: Vec::new(),
        down: FxHashMap::default(),
    };
    for &(v, k) in &base.u {
        let o = base.verts[v].occ;
        let par = vkey(
            hs.hash(o.si as usize, o.a as usize, o.b as usize - 1),
            o.len() - 1,
        );
        let last = c.strings[o.si as usize][o.b as usize - 1];
        g.add_up(base.verts[v].key, par, last, k);
    }
    for &(v, k) in &base.d {
        let o = base.verts[v].occ;
        let first = c.strings[o.si as usize][o.a as usize];
        let h = hs.hash(o.si as usize, o.a as usize, o.b as usize);
        g.add_down(base.verts[v].key, first, o.len() as u32, h, k);
    }
    let mut added = 0i64;
    for walk in &conn.walks {
        let mut bal: FxHashMap<VKey, i64> = FxHashMap::default();
        for e in walk {
            match *e {
                Edge::Up {
                    child, parent, c, ..
                } => {
                    g.add_up(child, parent, c, 1);
                    *bal.entry(parent).or_insert(0) -= 1;
                    *bal.entry(child).or_insert(0) += 1;
                    added += 1;
                }
                Edge::Down { v, first, len, h } => {
                    g.add_down(v, first, len, h, 1);
                    let sh = submod(h, mulmod(sym_val(first), pow.get(len as usize - 1)));
                    *bal.entry(v).or_insert(0) -= 1;
                    *bal.entry(vkey(sh, len as usize - 1)).or_insert(0) += 1;
                }
            }
        }
        assert!(bal.values().all(|&x| x == 0), "added walk is not closed");
    }
    assert!(added <= w_total, "added cost {added} exceeds W = {w_total}");
    conn.stats.added = added;
    trace("final graph", &mut tt);

    // ---- Euler tour from eps (Hierholzer, as euler_superstring in connect.py) ----
    // integer vertex ids; up lists in CSR form preserving insertion order (popped from the end)
    let mut vid: FxHashMap<VKey, u32> = FxHashMap::default();
    vid.insert(0, 0);
    let id_of = |k: VKey, vid: &mut FxHashMap<VKey, u32>| -> u32 {
        let n = vid.len() as u32;
        *vid.entry(k).or_insert(n)
    };
    let ups: Vec<(u32, u32, Sym, i64)> =
        g.up.iter()
            .map(|&(child, parent, ch, k)| (id_of(parent, &mut vid), id_of(child, &mut vid), ch, k))
            .collect();
    let downs: Vec<(u32, u32, i64)> = g
        .down
        .iter()
        .map(|(&v, &(k, first, len, h))| {
            let sh = submod(h, mulmod(sym_val(first), pow.get(len as usize - 1)));
            (
                id_of(v, &mut vid),
                id_of(vkey(sh, len as usize - 1), &mut vid),
                k,
            )
        })
        .collect();
    let nv = vid.len();
    drop(vid);
    let mut up_off = vec![0usize; nv + 1];
    for &(par, _, _, k) in &ups {
        up_off[par as usize + 1] += k as usize;
    }
    for i in 0..nv {
        up_off[i + 1] += up_off[i];
    }
    let mut up_end = up_off.clone(); // fill pointer, then stack top (exclusive)
    let mut up_dst: Vec<(u32, Sym)> = vec![(0, 0); up_off[nv]];
    for &(par, child, ch, k) in &ups {
        for _ in 0..k {
            up_dst[up_end[par as usize]] = (child, ch);
            up_end[par as usize] += 1;
        }
    }
    let mut down_cnt = vec![0i64; nv];
    let mut down_dst = vec![u32::MAX; nv];
    for &(v, sv, k) in &downs {
        down_cnt[v as usize] += k;
        down_dst[v as usize] = sv;
    }
    let mut stack: Vec<(u32, Sym, bool)> = vec![(0, 0, false)];
    let mut circuit: Vec<(Sym, bool)> = Vec::with_capacity(up_off[nv] * 2 + 1);
    while let Some(&(v, _, _)) = stack.last() {
        let vu = v as usize;
        if down_cnt[vu] > 0 {
            down_cnt[vu] -= 1;
            stack.push((down_dst[vu], 0, false));
        } else if up_end[vu] > up_off[vu] {
            up_end[vu] -= 1;
            let (child, ch) = up_dst[up_end[vu]];
            stack.push((child, ch, true));
        } else {
            let (_, s, up) = stack.pop().unwrap();
            circuit.push((s, up));
        }
    }
    assert!(
        down_cnt.iter().all(|&c| c == 0) && (0..nv).all(|i| up_end[i] == up_off[i]),
        "graph not connected"
    );
    circuit.reverse();
    let t: Word = circuit.iter().filter(|x| x.1).map(|x| x.0).collect();
    assert_eq!(t.len() as i64, w_total + added);
    trace("euler tour", &mut tt);
    (t, conn.stats)
}
