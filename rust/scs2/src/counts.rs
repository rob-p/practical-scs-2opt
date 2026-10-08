//! Forced occurrence counts of the SCS 2-approximation, computed on prefixes and suffixes only.
//!
//! Port of `scs/compact.py` (see Sections 4 and 7 of the note): words outside P ∪ Q are never
//! touched, period rules are applied only to overlap words with primitive templates, and blocking
//! sums use the closed form
//!     B_A(w) = rho(w) - sum_{n>|w|} d(A[0:n)) - T_A(w),
//!     U_A(v) = sum_{n>=|v|} d(A[0:n)) + T_A(v).
//!
//! Words are trie nodes: P is the node set of the forward trie F of the inputs, Q the node set
//! of the trie R of the reversed inputs, and overlap words (P ∩ Q) are paired across the two tries
//! along failure chains. Running sums live in flat arrays over word ids:
//!   lambda(s) = sum of u(y) over support words y with suffix s
//!     (suffixes of y in Q: walk R along y reversed; suffixes in P \ Q: F failure chain of y),
//!   rho(s) = sum of d(y) over support words y with prefix s
//!     (prefixes of y in P: walk F along y; prefixes in Q \ P: R failure chain of y).
//! T_A walks follow F (the prefix closure of the u support lies in P). Only the D_A walks hash,
//! against a map of the d support.

use crate::strings::{AhoCorasick, NONE, SliceHasher, Sym, Word, extend, reduce_instance_keep_ac};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicI64, AtomicU8, Ordering::Relaxed};

#[derive(Clone, Copy, Debug)]
pub struct Occ {
    pub si: u32,
    pub a: u32,
    pub b: u32,
}

impl Occ {
    #[inline]
    pub fn len(&self) -> usize {
        (self.b - self.a) as usize
    }
}

/// Result of the count computation: the support of u and d (base-graph edge multiplicities).
pub struct Counts {
    pub strings: Vec<Word>,
    /// (word occurrence, multiplicity) for up edges pref(s) -> s with u(s) > 0.
    pub u: Vec<(Occ, i64)>,
    /// (word occurrence, multiplicity) for down edges s -> suf(s) with d(s) > 0.
    pub d: Vec<(Occ, i64)>,
    pub w: i64,
    pub stats: CountStats,
}

#[derive(Default, Debug, Clone)]
pub struct CountStats {
    pub words: usize,
    pub overlap_words: usize,
    pub rules: usize,
    pub fired: usize,
    /// seconds spent reducing the instance, building the tries and pairing overlap words
    pub t_setup: f64,
    /// seconds in the parallel per-level phase and the sequential update phase
    pub t_phase_a: f64,
    pub t_phase_b: f64,
}

impl Counts {
    pub fn word(&self, o: &Occ) -> &[Sym] {
        &self.strings[o.si as usize][o.a as usize..o.b as usize]
    }
}

fn tracing() -> bool {
    static T: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *T.get_or_init(|| std::env::var_os("SCS2_TRACE").is_some())
}

fn trace(name: &str, t: &mut std::time::Instant) {
    if tracing() {
        eprintln!("  [{:>20}] {:.2}s", name, t.elapsed().as_secs_f64());
    }
    *t = std::time::Instant::now();
}

/// Combine a word hash (< 2^61) and its length into one map key.
#[inline]
fn wkey(h: u64, len: usize) -> u64 {
    h ^ (len as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Periods p < |v| of v whose template v[..p] is primitive.
fn primitive_periods(v: &[Sym]) -> Vec<usize> {
    let n = v.len();
    if n == 0 {
        return Vec::new();
    }
    let fail = prefix_function(v);
    let mut out = Vec::new();
    let mut b = fail[n - 1];
    while b > 0 {
        let p = n - b;
        // the prefix function of v[..p] agrees with fail[..p]
        let lp = p - fail[p - 1];
        if lp == p || p % lp != 0 {
            out.push(p);
        }
        b = fail[b - 1];
    }
    out
}

fn prefix_function(v: &[Sym]) -> Vec<usize> {
    let mut pi = vec![0usize; v.len()];
    let mut k = 0;
    for i in 1..v.len() {
        while k > 0 && v[i] != v[k] {
            k = pi[k - 1];
        }
        if v[i] == v[k] {
            k += 1;
        }
        pi[i] = k;
    }
    pi
}

/// A periodic text given by a template slice of the input: A(i) = tmpl[i mod p].
struct Tmpl<'a> {
    t: &'a [Sym],
}

impl Tmpl<'_> {
    #[inline]
    fn at(&self, i: i64) -> Sym {
        let p = self.t.len() as i64;
        self.t[i.rem_euclid(p) as usize]
    }
}

struct State<'a> {
    f: &'a AhoCorasick,
    /// u values and u-sums per F node (index 0 = empty word)
    u_own: Vec<AtomicI64>,
    /// 1 iff the F node is a prefix of some u-support word (prefix closure of the support)
    u_pre: Vec<AtomicU8>,
    u_child: Vec<AtomicI64>,
    /// d support keyed by word hash, for D_A walks
    d_map: FxHashMap<u64, i64>,
    maxlen: usize,
}

/// Sum of d(A[0:n)) over n >= x_len (n > x_len if strict); `hx` is the hash of A[0:x_len) and
/// `dx` its d value. The prefix closure of the d support is not indexed, so walk to the maximum
/// length.
fn t1(st: &State, a: &Tmpl, hx: u64, x_len: usize, strict: bool, dx: i64) -> i64 {
    let mut total = if strict { 0 } else { dx };
    if st.d_map.is_empty() {
        return total;
    }
    let mut h = hx;
    for n in x_len..st.maxlen {
        h = extend(h, a.at(n as i64));
        if let Some(&v) = st.d_map.get(&wkey(h, n + 1)) {
            total += v;
        }
    }
    total
}

/// T_A(x) = sum_{n>=|x|} sum_{c != A(n)} sum_{mu>=0} u(A[-mu:n) c).
///
/// Equivalently, sum over words A[r:e) in P with r in [0,p) of mult(r, e) * (u_child - u_own of
/// the mismatching extensions), where mult(r, e) = floor((e - |x|)/p) - [r > 0] + 1. One
/// Aho-Corasick scan of A finds them: the state before position e is the longest suffix of
/// A[0:e) in P, and its failure chain lists every A[s:e) in P. Contributions need e >= |x|, and
/// the scan ends once the state is too shallow for any start s < p (its depth grows by at most 1
/// per step). All terms are nonnegative, so the scan stops once the total reaches `cap`; the
/// return value is exact when below `cap` and otherwise only known to be >= `cap`.
fn t2(st: &State, a: &Tmpl, x_len: usize, cap: i64) -> i64 {
    let f = st.f;
    let p = a.t.len() as i64;
    let x = x_len as i64;
    let end = p + st.maxlen as i64;
    let mut total = 0i64;
    let mut v = 0u32; // longest suffix of A[0:e) in P
    let mut e = 0i64;
    while e < end {
        let depth = f.depth[v as usize] as i64;
        if e > 0 && depth <= e - p {
            break; // no word A[s:e) with s < p now or later
        }
        let ch = a.at(e);
        if e >= x {
            let q = (e - x).div_euclid(p);
            let mut node = v;
            while node != 0 {
                let s = e - f.depth[node as usize] as i64;
                if s >= p {
                    break;
                }
                let mult = q - (s > 0) as i64 + 1;
                if mult > 0 {
                    let next = f.child(node, ch);
                    let own = if next == NONE {
                        0
                    } else {
                        st.u_own[next as usize].load(Relaxed)
                    };
                    total += mult * (st.u_child[node as usize].load(Relaxed) - own);
                    if total >= cap {
                        return total;
                    }
                }
                node = f.fail[node as usize];
            }
        }
        v = f.step(v, ch);
        e += 1;
    }
    if check_t2() {
        let w = t2_walk(st, a, x_len, i64::MAX);
        assert_eq!(
            total, w,
            "t2 scan disagrees with walk (p={}, |x|={})",
            p, x_len
        );
    }
    total
}

/// Whether to cross-check every uncapped `t2` result against `t2_walk` (env SCS2_CHECK_T2).
fn check_t2() -> bool {
    static CHECK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CHECK.get_or_init(|| std::env::var_os("SCS2_CHECK_T2").is_some())
}

/// Reference version of `t2`: one walk of F per residue (used only for checking).
fn t2_walk(st: &State, a: &Tmpl, x_len: usize, cap: i64) -> i64 {
    let p = a.t.len() as i64;
    let x_len = x_len as i64;
    let mut total = 0i64;
    for r in 0..p {
        let jmin = if r == 0 { 0 } else { 1 };
        let mut cur = 0u32; // F root = empty word
        let mut ln = 0i64;
        loop {
            let pos = r + ln;
            let next = st.f.child(cur, a.at(pos));
            let mult = (pos - x_len).div_euclid(p) - jmin + 1;
            if mult > 0 {
                let own = if next == NONE {
                    0
                } else {
                    st.u_own[next as usize].load(Relaxed)
                };
                total += mult * (st.u_child[cur as usize].load(Relaxed) - own);
                if total >= cap {
                    return total;
                }
            }
            if next == NONE || st.u_pre[next as usize].load(Relaxed) == 0 {
                break;
            }
            cur = next;
            ln += 1;
        }
    }
    total
}

/// Outcome of phase A for one word.
struct WordResult {
    id: u32,
    occ: Occ,
    h: u64,
    us: i64,
    ds: i64,
    is_o: bool,
    rules: usize,
    fired: usize,
    /// (target F node, template occurrence, template key, k)
    new_rules: Vec<(u32, Occ, u64, i64)>,
    secs: f64,
}

/// Read-only view of the state during phase A.
struct Ctx<'a> {
    strings: &'a [Word],
    hs: &'a SliceHasher,
    f: &'a AhoCorasick,
    f2r: &'a [u32],
    st: &'a State<'a>,
    lam: &'a [AtomicI64],
    rho: &'a [AtomicI64],
}

impl Ctx<'_> {
    fn process(
        &self,
        id32: u32,
        nf: usize,
        occ_of: &(dyn Fn(usize) -> Occ + Sync),
        rules: Option<Vec<(Occ, u64, i64)>>,
    ) -> WordResult {
        let (strings, hs, f, st) = (self.strings, self.hs, self.f, self.st);
        let id = id32 as usize;
        let o = occ_of(id);
        let (si, a, b) = (o.si as usize, o.a as usize, o.b as usize);
        let s = &strings[si][a..b];
        let is_p = id < nf;
        let is_o = is_p && self.f2r[id] != NONE;
        let (l_s, r_s) = (self.lam[id].load(Relaxed), self.rho[id].load(Relaxed));
        let mut res = WordResult {
            id: id32,
            occ: o,
            h: 0,
            us: 0,
            ds: 0,
            is_o,
            rules: 0,
            fired: 0,
            new_rules: Vec::new(),
            secs: 0.0,
        };
        let val = if is_o {
            res.h = hs.hash(si, a, b);
            let mut val = l_s.max(r_s).max(if f.term[id] != NONE { 1 } else { 0 });
            if let Some(mut rules) = rules {
                // larger k first, so that val grows early and prunes later rules
                rules.sort_by(|x, y| y.2.cmp(&x.2));
                for (t, _, k) in rules {
                    let tm = Tmpl {
                        t: &strings[t.si as usize][t.a as usize..t.b as usize],
                    };
                    let x1 = t1(st, &tm, res.h, b - a, true, 0);
                    // bound = rho + k + 1 - x1 - T beats val only if T < rho + k + 1 - x1 - val
                    let slack = r_s + k + 1 - x1 - val;
                    if slack <= 0 {
                        continue;
                    }
                    let x2 = t2(st, &tm, b - a, slack);
                    if x2 < slack {
                        val = r_s + k + 1 - x1 - x2;
                    }
                }
            }
            val
        } else if is_p {
            r_s // not a suffix: m = rho
        } else {
            l_s // not a prefix: m = lambda
        };
        res.us = val - l_s;
        res.ds = val - r_s;
        assert!(res.us >= 0 && res.ds >= 0, "negative multiplicity");
        if is_o {
            let mut anc: Vec<u32> = Vec::new();
            for p in primitive_periods(s) {
                res.rules += 1;
                let tm = Tmpl { t: &s[..p] };
                let fired =
                    t1(st, &tm, res.h, b - a, false, res.ds) > 0 || t2(st, &tm, b - a, 1) > 0;
                if !fired {
                    continue;
                }
                res.fired += 1;
                if anc.is_empty() {
                    // ancestors of the F node: anc[j] = node of s[..j]
                    let mut v = id as u32;
                    while v != NONE {
                        anc.push(v);
                        v = f.parent[v as usize];
                    }
                    anc.reverse();
                }
                let tocc = Occ {
                    si: o.si,
                    a: o.a,
                    b: (a + p) as u32,
                };
                let tkey = wkey(hs.hash(si, a, a + p), p);
                let mut k = 1usize;
                while (b - a) > k * p {
                    let wn = anc[(b - a) - k * p];
                    if self.f2r[wn as usize] != NONE {
                        res.new_rules.push((wn, tocc, tkey, k as i64));
                    }
                    k += 1;
                }
            }
        }
        res
    }
}

pub fn compute_counts(input: &[Word]) -> Counts {
    let t0 = std::time::Instant::now();
    let mut tt = t0;
    let (strings, f_reused) = reduce_instance_keep_ac(input); // sorted, unique, substring-free
    trace("reduce", &mut tt);
    let ns = strings.len();
    let maxlen = strings.iter().map(|s| s.len()).max().unwrap_or(0);
    let hs = SliceHasher::new(&strings);

    // ---- tries: F over the inputs, R over the reversed inputs ----
    let (f, (r, rorder)) = rayon::join(
        || f_reused.unwrap_or_else(|| AhoCorasick::new(&strings)),
        || {
            let mut rorder: Vec<u32> = (0..ns as u32).collect();
            rorder.par_sort_unstable_by(|&x, &y| {
                strings[x as usize]
                    .iter()
                    .rev()
                    .cmp(strings[y as usize].iter().rev())
            });
            let rstrings: Vec<Word> = rorder
                .iter()
                .map(|&i| strings[i as usize].iter().rev().copied().collect())
                .collect();
            (AhoCorasick::new(&rstrings), rorder)
        },
    );
    trace("F and R tries", &mut tt);
    let (nf, nr) = (f.num_nodes(), r.num_nodes());
    let mut rpos = vec![0u32; ns]; // position of each input in the reversed order
    for (k, &i) in rorder.iter().enumerate() {
        rpos[i as usize] = k as u32;
    }

    // ---- pair overlap words: suffixes of x that are in P lie on the F failure chain of x ----
    let mut f2r = vec![NONE; nf];
    let mut r2f = vec![NONE; nr];
    let mut rpath: Vec<u32> = Vec::with_capacity(maxlen + 1);
    for si in 0..ns {
        // rpath[t] = R node of the suffix of length t
        rpath.clear();
        let mut v = r.end_node[rpos[si] as usize];
        while v != NONE {
            rpath.push(v);
            v = r.parent[v as usize];
        }
        rpath.reverse();
        let mut fv = f.end_node[si];
        while fv != 0 {
            let rn = rpath[f.depth[fv as usize] as usize];
            f2r[fv as usize] = rn;
            r2f[rn as usize] = fv;
            fv = f.fail[fv as usize];
        }
    }
    // skip pointers: nearest proper failure ancestor that is not an overlap word (0 if none);
    // nodes are processed by increasing depth so that the failure target is already done
    let skip_of = |t: &AhoCorasick, paired: &Vec<u32>| -> Vec<u32> {
        let mut sk = vec![0u32; t.num_nodes()];
        for &v in &t.bfs {
            let g = t.fail[v as usize];
            sk[v as usize] = if g == 0 || paired[g as usize] == NONE {
                g
            } else {
                sk[g as usize]
            };
        }
        sk
    };
    let f_skip = skip_of(&f, &f2r);
    let r_skip = skip_of(&r, &r2f);
    trace("pairing + skip", &mut tt);
    // word ids: F node v -> v; R node rn -> r2f[rn] if paired, else nf + rn
    let rid = |rn: u32| -> usize {
        if r2f[rn as usize] != NONE {
            r2f[rn as usize] as usize
        } else {
            nf + rn as usize
        }
    };
    // representative occurrence of a word id
    let occ_of = |id: usize| -> Occ {
        if id < nf {
            Occ {
                si: f.lo[id],
                a: 0,
                b: f.depth[id],
            }
        } else {
            let rn = id - nf;
            let si = rorder[r.lo[rn] as usize];
            let n = strings[si as usize].len() as u32;
            Occ {
                si,
                a: n - r.depth[rn],
                b: n,
            }
        }
    };

    let word_len = |id: usize| -> usize {
        if id < nf {
            f.depth[id] as usize
        } else {
            r.depth[id - nf] as usize
        }
    };
    // ---- processing order: decreasing length (F nodes, then unpaired R nodes, per length) ----
    let mut order: Vec<u32>;
    {
        let mut cnt = vec![0usize; maxlen + 2];
        for v in 1..nf {
            cnt[f.depth[v] as usize] += 1;
        }
        for rn in 1..nr {
            if r2f[rn] == NONE {
                cnt[r.depth[rn] as usize] += 1;
            }
        }
        let mut start = vec![0usize; maxlen + 2];
        let mut acc = 0;
        for l in (1..=maxlen).rev() {
            start[l] = acc;
            acc += cnt[l];
        }
        order = vec![0; acc];
        for v in 1..nf {
            let l = f.depth[v] as usize;
            order[start[l]] = v as u32;
            start[l] += 1;
        }
        for rn in 1..nr {
            if r2f[rn] == NONE {
                let l = r.depth[rn] as usize;
                order[start[l]] = (nf + rn) as u32;
                start[l] += 1;
            }
        }
    }

    let nids = nf + nr;
    let zeros = |n: usize| -> Vec<AtomicI64> { (0..n).map(|_| AtomicI64::new(0)).collect() };
    let lam = zeros(nids);
    let rho = zeros(nids);
    let mut st = State {
        f: &f,
        u_own: zeros(nf),
        u_pre: (0..nf).map(|_| AtomicU8::new(0)).collect(),
        u_child: zeros(nf),
        d_map: FxHashMap::default(),
        maxlen,
    };
    // pending rules: target F node -> list of (template occurrence, template hash, max k)
    let mut pending: FxHashMap<u32, Vec<(Occ, u64, i64)>> = FxHashMap::default();
    let mut u_out = Vec::new();
    let mut d_out = Vec::new();
    let mut stats = CountStats {
        words: order.len(),
        ..Default::default()
    };
    trace("order + arrays", &mut tt);
    stats.t_setup = t0.elapsed().as_secs_f64();
    // Words of equal length depend only on longer words, so each length level is processed in
    // two phases: (A) in parallel, read-only, compute u, d and the fired rules of every word;
    // (B) sequentially, in level order, apply the updates. Results do not depend on threading.
    let mut level_trace: Vec<(f64, usize, usize, f64, f64, usize, usize)> = Vec::new();
    let mut lo = 0;
    while lo < order.len() {
        let len = word_len(order[lo] as usize);
        let mut hi = lo;
        while hi < order.len() && word_len(order[hi] as usize) == len {
            hi += 1;
        }
        let level = &order[lo..hi];
        let rules_in: Vec<Option<Vec<(Occ, u64, i64)>>> =
            level.iter().map(|id| pending.remove(id)).collect();
        let ta = std::time::Instant::now();
        let ctx = Ctx {
            strings: &strings,
            hs: &hs,
            f: &f,
            f2r: &f2r,
            st: &st,
            lam: &lam,
            rho: &rho,
        };
        let results: Vec<WordResult> = level
            .par_iter()
            .zip(rules_in.into_par_iter())
            .map(|(&id32, rules)| {
                if tracing() {
                    let c0 = std::time::Instant::now();
                    let mut r = ctx.process(id32, nf, &occ_of, rules);
                    r.secs = c0.elapsed().as_secs_f64();
                    r
                } else {
                    ctx.process(id32, nf, &occ_of, rules)
                }
            })
            .collect();
        if tracing() {
            let tot: f64 = results.iter().map(|r| r.secs).sum();
            let mx = results
                .iter()
                .max_by(|x, y| x.secs.total_cmp(&y.secs))
                .unwrap();
            level_trace.push((
                ta.elapsed().as_secs_f64(),
                len,
                level.len(),
                tot,
                mx.secs,
                mx.rules,
                mx.fired,
            ));
        }
        let tb = std::time::Instant::now();
        stats.t_phase_a += (tb - ta).as_secs_f64();
        // ---- phase B: bookkeeping in level order, then commutative updates in parallel ----
        for res in &results {
            stats.overlap_words += res.is_o as usize;
            stats.rules += res.rules;
            stats.fired += res.fired;
            let o = res.occ;
            if res.us > 0 {
                u_out.push((o, res.us));
            }
            if res.ds > 0 {
                d_out.push((o, res.ds));
                let h = if res.h != 0 {
                    res.h
                } else {
                    hs.hash(o.si as usize, o.a as usize, o.b as usize)
                };
                *st.d_map.entry(wkey(h, o.len())).or_insert(0) += res.ds;
            }
            for &(wn, tocc, tkey, k) in &res.new_rules {
                let list = pending.entry(wn).or_default();
                match list.iter_mut().find(|e| e.1 == tkey) {
                    Some(e) => e.2 = e.2.max(k),
                    None => list.push((tocc, tkey, k)),
                }
            }
        }
        let st_ref = &st;
        results.par_iter().for_each(|res| {
            let st = st_ref;
            let id = res.id as usize;
            let o = res.occ;
            let s = &strings[o.si as usize][o.a as usize..o.b as usize];
            let is_p = id < nf;
            let (us, ds) = (res.us, res.ds);
            if us > 0 {
                st.u_own[id].fetch_add(us, Relaxed);
                // mark the prefix closure; stop at the first ancestor already marked
                // (whoever marked it continues upward, so the closure is complete after the phase)
                let mut v = id as u32;
                while v != 0 && st.u_pre[v as usize].swap(1, Relaxed) == 0 {
                    v = f.parent[v as usize];
                }
                st.u_child[f.parent[id] as usize].fetch_add(us, Relaxed);
                // suffixes of s in Q: walk R along s reversed
                let mut rn = 0u32;
                for &c in s.iter().rev() {
                    rn = r.child(rn, c);
                    if rn == NONE {
                        break;
                    }
                    lam[rid(rn)].fetch_add(us, Relaxed);
                }
                // suffixes of s in P \ Q: F failure chain, unpaired nodes only
                let mut g = f_skip[id];
                while g != 0 {
                    lam[g as usize].fetch_add(us, Relaxed);
                    g = f_skip[g as usize];
                }
            }
            if ds > 0 {
                // prefixes of s in P: walk F along s
                let mut fv = 0u32;
                for &c in s {
                    fv = f.child(fv, c);
                    if fv == NONE {
                        break;
                    }
                    rho[fv as usize].fetch_add(ds, Relaxed);
                }
                // prefixes of s in Q \ P: R failure chain of s's R node
                let rn = if is_p { f2r[id] } else { (id - nf) as u32 };
                let mut g = r_skip[rn as usize];
                while g != 0 {
                    rho[nf + g as usize].fetch_add(ds, Relaxed);
                    g = r_skip[g as usize];
                }
            }
        });
        stats.t_phase_b += tb.elapsed().as_secs_f64();
        lo = hi;
    }
    if tracing() {
        level_trace.sort_by(|x, y| y.0.total_cmp(&x.0));
        eprintln!("  slowest levels: (wall, len, words, cpu sum, max word, its rules, fired)");
        for t in level_trace.iter().take(8) {
            eprintln!(
                "    {:.3}s len={} words={} cpu={:.3}s max={:.4}s rules={} fired={}",
                t.0, t.1, t.2, t.3, t.4, t.5, t.6
            );
        }
        let wall: f64 = level_trace.iter().map(|t| t.0).sum();
        let cpu: f64 = level_trace.iter().map(|t| t.3).sum();
        let maxsum: f64 = level_trace.iter().map(|t| t.4).sum();
        eprintln!(
            "  phase A wall {:.2}s, cpu {:.2}s, sum of per-level max word {:.2}s, levels {}",
            wall,
            cpu,
            maxsum,
            level_trace.len()
        );
    }
    let w: i64 = u_out.iter().map(|x| x.1).sum();
    assert_eq!(
        w,
        d_out.iter().map(|x| x.1).sum::<i64>(),
        "unbalanced base graph"
    );
    Counts {
        strings,
        u: u_out,
        d: d_out,
        w,
        stats,
    }
}
