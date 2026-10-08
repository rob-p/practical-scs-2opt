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

use crate::strings::{AhoCorasick, NONE, SliceHasher, Sym, Word, extend, reduce_instance};
use rustc_hash::FxHashMap;

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
}

impl Counts {
    pub fn word(&self, o: &Occ) -> &[Sym] {
        &self.strings[o.si as usize][o.a as usize..o.b as usize]
    }
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
    u_own: Vec<i64>,
    u_pre: Vec<i64>,
    u_child: Vec<i64>,
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

/// T_A(x) = sum_{n>=|x|} sum_{c != A(n)} sum_{mu>=0} u(A[-mu:n) c), via one F walk per residue.
/// All terms are nonnegative, so the walk stops early once the total reaches `cap`; the return
/// value is exact when below `cap` and otherwise only known to be >= `cap`.
fn t2(st: &State, a: &Tmpl, x_len: usize, cap: i64) -> i64 {
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
                    st.u_own[next as usize]
                };
                total += mult * (st.u_child[cur as usize] - own);
                if total >= cap {
                    return total;
                }
            }
            if next == NONE || st.u_pre[next as usize] == 0 {
                break;
            }
            cur = next;
            ln += 1;
        }
    }
    total
}

pub fn compute_counts(input: &[Word]) -> Counts {
    let t0 = std::time::Instant::now();
    let strings = reduce_instance(input); // sorted, unique, substring-free
    let ns = strings.len();
    let maxlen = strings.iter().map(|s| s.len()).max().unwrap_or(0);
    let hs = SliceHasher::new(&strings);

    // ---- tries: F over the inputs, R over the reversed inputs ----
    let f = AhoCorasick::new(&strings);
    let mut rorder: Vec<u32> = (0..ns as u32).collect();
    rorder.sort_by(|&x, &y| {
        strings[x as usize]
            .iter()
            .rev()
            .cmp(strings[y as usize].iter().rev())
    });
    let rstrings: Vec<Word> = rorder
        .iter()
        .map(|&i| strings[i as usize].iter().rev().copied().collect())
        .collect();
    let r = AhoCorasick::new(&rstrings);
    drop(rstrings);
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
        let n = t.num_nodes();
        let mut by_depth: Vec<u32> = (0..n as u32).collect();
        by_depth.sort_by_key(|&v| t.depth[v as usize]);
        let mut sk = vec![0u32; n];
        for &v in &by_depth[1..] {
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
    let mut lam = vec![0i64; nids];
    let mut rho = vec![0i64; nids];
    let mut d_own = vec![0i64; nids];
    let mut st = State {
        f: &f,
        u_own: vec![0; nf],
        u_pre: vec![0; nf],
        u_child: vec![0; nf],
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
    stats.t_setup = t0.elapsed().as_secs_f64();
    let mut anc: Vec<u32> = Vec::with_capacity(maxlen + 1);

    for &id32 in &order {
        let id = id32 as usize;
        let o = occ_of(id);
        let (si, a, b) = (o.si as usize, o.a as usize, o.b as usize);
        let s = &strings[si][a..b];
        let is_p = id < nf;
        let is_o = is_p && f2r[id] != NONE;
        let (l_s, r_s) = (lam[id], rho[id]);
        let mut h = 0u64;
        let val = if is_o {
            stats.overlap_words += 1;
            h = hs.hash(si, a, b);
            let mut val = l_s.max(r_s).max(if f.term[id] != NONE { 1 } else { 0 });
            if let Some(mut rules) = pending.remove(&id32) {
                // larger k first, so that val grows early and prunes later rules
                rules.sort_by(|x, y| y.2.cmp(&x.2));
                for (t, _, k) in rules {
                    let tm = Tmpl {
                        t: &strings[t.si as usize][t.a as usize..t.b as usize],
                    };
                    let x1 = t1(&st, &tm, h, b - a, true, 0);
                    // bound = rho + k + 1 - x1 - T beats val only if T < rho + k + 1 - x1 - val
                    let slack = r_s + k + 1 - x1 - val;
                    if slack <= 0 {
                        continue;
                    }
                    let x2 = t2(&st, &tm, b - a, slack);
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
        let (us, ds) = (val - l_s, val - r_s);
        assert!(us >= 0 && ds >= 0, "negative multiplicity");
        if us > 0 {
            u_out.push((o, us));
            st.u_own[id] += us;
            // mark the prefix closure; stop at the first ancestor already marked
            let mut v = id as u32;
            while v != 0 && st.u_pre[v as usize] == 0 {
                st.u_pre[v as usize] = 1;
                v = f.parent[v as usize];
            }
            st.u_child[f.parent[id] as usize] += us;
            // suffixes of s in Q: walk R along s reversed
            let mut rn = 0u32;
            for &c in s.iter().rev() {
                rn = r.child(rn, c);
                if rn == NONE {
                    break;
                }
                lam[rid(rn)] += us;
            }
            // suffixes of s in P \ Q: F failure chain, unpaired nodes only
            let mut g = f_skip[id];
            while g != 0 {
                lam[g as usize] += us;
                g = f_skip[g as usize];
            }
        }
        if ds > 0 {
            d_out.push((o, ds));
            d_own[id] += ds;
            if h == 0 {
                h = hs.hash(si, a, b);
            }
            *st.d_map.entry(wkey(h, b - a)).or_insert(0) += ds;
            // prefixes of s in P: walk F along s
            let mut fv = 0u32;
            for &c in s {
                fv = f.child(fv, c);
                if fv == NONE {
                    break;
                }
                rho[fv as usize] += ds;
            }
            // prefixes of s in Q \ P: R failure chain of s's R node
            let rn = if is_p { f2r[id] } else { (id - nf) as u32 };
            let mut g = r_skip[rn as usize];
            while g != 0 {
                rho[nf + g as usize] += ds;
                g = r_skip[g as usize];
            }
        }
        if is_o {
            anc.clear();
            for p in primitive_periods(s) {
                stats.rules += 1;
                let tm = Tmpl { t: &s[..p] };
                let fired =
                    t1(&st, &tm, h, b - a, false, d_own[id]) > 0 || t2(&st, &tm, b - a, 1) > 0;
                if fired {
                    stats.fired += 1;
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
                        if f2r[wn as usize] != NONE {
                            let list = pending.entry(wn).or_default();
                            match list.iter_mut().find(|e| e.1 == tkey) {
                                Some(e) => e.2 = e.2.max(k as i64),
                                None => list.push((tocc, tkey, k as i64)),
                            }
                        }
                        k += 1;
                    }
                }
            }
        }
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
