//! String utilities shared by greedy and the 2-approximation: symbols, the Aho-Corasick
//! automaton, instance reduction, suffix-prefix overlaps, and polynomial hashing.

/// A symbol is a Unicode code point, matching the Python reference implementation's `str`.
pub type Sym = u32;
pub type Word = Vec<Sym>;

pub fn to_word(s: &str) -> Word {
    s.chars().map(|c| c as Sym).collect()
}

pub fn to_string(w: &[Sym]) -> String {
    w.iter()
        .map(|&c| char::from_u32(c).expect("valid code point"))
        .collect()
}

/// Aho-Corasick automaton over a sorted, duplicate-free pattern list.
///
/// The trie is built from consecutive longest common prefixes, so nodes are numbered in DFS
/// preorder. Children are stored contiguously per node (CSR), sorted by symbol. No hashing.
pub struct AhoCorasick {
    pub sym: Vec<Sym>,
    pub parent: Vec<u32>,
    /// children of v: child_sym/child_node[child_off[v]..child_off[v+1]], sorted by symbol
    pub child_off: Vec<u32>,
    pub child_sym: Vec<Sym>,
    pub child_node: Vec<u32>,
    pub fail: Vec<u32>,
    pub depth: Vec<u32>,
    /// Index of the pattern ending at this node, or NONE.
    pub term: Vec<u32>,
    /// Nearest proper failure ancestor that is terminal (0 if none).
    pub dict_link: Vec<u32>,
    /// Patterns passing through each node form the index range [lo, hi) of the sorted patterns.
    pub lo: Vec<u32>,
    pub hi: Vec<u32>,
    /// Node reached by each full pattern.
    pub end_node: Vec<u32>,
    /// All non-root nodes in BFS order (nondecreasing depth).
    pub bfs: Vec<u32>,
    /// Dense child table for small alphabets: dense[v * sigma + code[c]] (empty if unused).
    dense: Vec<u32>,
    code: [u8; 256],
    sigma: usize,
}

/// Dense child tables are used when all symbols are < 256 and there are at most this many.
const DENSE_MAX_SIGMA: usize = 16;

pub const NONE: u32 = u32::MAX;

impl AhoCorasick {
    /// `patterns` must be sorted and free of duplicates.
    pub fn new(patterns: &[Word]) -> Self {
        let mut tt = std::time::Instant::now();
        debug_assert!(
            patterns.windows(2).all(|w| w[0] < w[1]),
            "patterns must be sorted, unique"
        );
        let total: usize = patterns.iter().map(|p| p.len()).sum();
        let cap = total + 1;
        let mut first_child: Vec<u32> = Vec::with_capacity(cap);
        let mut next_sib: Vec<u32> = Vec::with_capacity(cap);
        let mut t = AhoCorasick {
            sym: Vec::with_capacity(cap),
            parent: Vec::with_capacity(cap),
            child_off: Vec::new(),
            child_sym: Vec::new(),
            child_node: Vec::new(),
            fail: Vec::new(),
            depth: Vec::with_capacity(cap),
            term: Vec::with_capacity(cap),
            dict_link: Vec::new(),
            lo: Vec::with_capacity(cap),
            hi: Vec::with_capacity(cap),
            end_node: Vec::with_capacity(patterns.len()),
            bfs: Vec::new(),
            dense: Vec::new(),
            code: [u8::MAX; 256],
            sigma: 0,
        };
        let mut last_child: Vec<u32> = Vec::with_capacity(cap);
        let push = |t: &mut AhoCorasick,
                    last_child: &mut Vec<u32>,
                    first_child: &mut Vec<u32>,
                    next_sib: &mut Vec<u32>,
                    par: u32,
                    c: Sym,
                    d: u32,
                    idx: u32|
         -> u32 {
            let v = t.sym.len() as u32;
            t.sym.push(c);
            t.parent.push(par);
            first_child.push(NONE);
            next_sib.push(NONE);
            t.depth.push(d);
            t.term.push(NONE);
            t.lo.push(idx);
            t.hi.push(idx + 1);
            last_child.push(NONE);
            if par != NONE {
                let lc = last_child[par as usize];
                if lc == NONE {
                    first_child[par as usize] = v;
                } else {
                    next_sib[lc as usize] = v;
                }
                last_child[par as usize] = v;
            }
            v
        };
        push(
            &mut t,
            &mut last_child,
            &mut first_child,
            &mut next_sib,
            NONE,
            0,
            0,
            0,
        );
        t.hi[0] = patterns.len() as u32;
        let mut path: Vec<u32> = vec![0]; // path[d] = node at depth d on the previous pattern
        let mut prev: &[Sym] = &[];
        for (idx, p) in patterns.iter().enumerate() {
            let l = prev
                .iter()
                .zip(p.iter())
                .take_while(|(a, b)| a == b)
                .count();
            path.truncate(l + 1);
            for &v in &path[1..] {
                t.hi[v as usize] = idx as u32 + 1;
            }
            for d in l..p.len() {
                let par = *path.last().unwrap();
                let v = push(
                    &mut t,
                    &mut last_child,
                    &mut first_child,
                    &mut next_sib,
                    par,
                    p[d],
                    d as u32 + 1,
                    idx as u32,
                );
                path.push(v);
            }
            let end = *path.last().unwrap();
            t.term[end as usize] = idx as u32;
            t.end_node.push(end);
            prev = p;
        }
        crate::counts::trace("  ac: trie", &mut tt);
        // children in CSR form (sibling lists are already sorted by symbol)
        let n = t.sym.len();
        drop(last_child);
        t.child_off = Vec::with_capacity(n + 1);
        t.child_sym = Vec::with_capacity(n);
        t.child_node = Vec::with_capacity(n);
        for v in 0..n {
            t.child_off.push(t.child_node.len() as u32);
            let mut w = first_child[v];
            while w != NONE {
                t.child_sym.push(t.sym[w as usize]);
                t.child_node.push(w);
                w = next_sib[w as usize];
            }
        }
        t.child_off.push(t.child_node.len() as u32);
        drop(first_child);
        drop(next_sib);
        crate::counts::trace("  ac: csr", &mut tt);
        // dense table for small alphabets
        let mut present = [false; 256];
        let mut small = true;
        for &c in &t.child_sym {
            if c < 256 {
                present[c as usize] = true;
            } else {
                small = false;
                break;
            }
        }
        let sigma = present.iter().filter(|&&x| x).count();
        if small && sigma <= DENSE_MAX_SIGMA {
            let mut k = 0u8;
            for (c, &pr) in present.iter().enumerate() {
                if pr {
                    t.code[c] = k;
                    k += 1;
                }
            }
            t.sigma = sigma.max(1);
            t.dense = vec![NONE; n * t.sigma];
            for v in 0..n {
                for ci in t.child_off[v]..t.child_off[v + 1] {
                    let c = t.child_sym[ci as usize] as usize;
                    t.dense[v * t.sigma + t.code[c] as usize] = t.child_node[ci as usize];
                }
            }
        }
        crate::counts::trace("  ac: dense", &mut tt);
        // failure and dictionary links, one depth level at a time: a node's links depend only on
        // shallower nodes, so each level is computed in parallel and then written back
        use rayon::prelude::*;
        t.fail = vec![0; n];
        t.dict_link = vec![0; n];
        let maxd = t.depth.iter().copied().max().unwrap_or(0) as usize;
        let mut level_start = vec![0usize; maxd + 2];
        for &d in &t.depth[1..] {
            level_start[d as usize + 1] += 1;
        }
        for d in 1..=maxd + 1 {
            level_start[d] += level_start[d - 1];
        }
        let mut queue: Vec<u32> = vec![0; n - 1];
        {
            let mut fill = level_start.clone();
            // level_start[d] = number of non-root nodes shallower than d
            for v in 1..n {
                let d = t.depth[v] as usize;
                queue[fill[d]] = v as u32;
                fill[d] += 1;
            }
        }
        for d in 1..=maxd {
            let level = &queue[level_start[d]..level_start[d + 1]];
            let links: Vec<(u32, u32)> = if d == 1 {
                level.iter().map(|_| (0, 0)).collect()
            } else {
                let t_ref = &t;
                level
                    .par_iter()
                    .with_min_len(1024)
                    .map(|&w| {
                        let cw = t_ref.sym[w as usize];
                        let mut f = t_ref.fail[t_ref.parent[w as usize] as usize];
                        let fw = loop {
                            let x = t_ref.child(f, cw);
                            if x != NONE {
                                break x;
                            }
                            if f == 0 {
                                break 0;
                            }
                            f = t_ref.fail[f as usize];
                        };
                        let dl = if t_ref.term[fw as usize] != NONE {
                            fw
                        } else {
                            t_ref.dict_link[fw as usize]
                        };
                        (fw, dl)
                    })
                    .collect()
            };
            for (&w, (fw, dl)) in level.iter().zip(links) {
                t.fail[w as usize] = fw;
                t.dict_link[w as usize] = dl;
            }
        }
        t.bfs = queue;
        crate::counts::trace("  ac: fail", &mut tt);
        t
    }

    pub fn num_nodes(&self) -> usize {
        self.sym.len()
    }

    /// Child of v labelled c, or NONE.
    #[inline]
    pub fn child(&self, v: u32, c: Sym) -> u32 {
        if !self.dense.is_empty() {
            if c >= 256 {
                return NONE;
            }
            let k = self.code[c as usize];
            if k == u8::MAX {
                return NONE;
            }
            return self.dense[v as usize * self.sigma + k as usize];
        }
        let (lo, hi) = (
            self.child_off[v as usize] as usize,
            self.child_off[v as usize + 1] as usize,
        );
        let syms = &self.child_sym[lo..hi];
        if syms.len() <= 8 {
            for (i, &s) in syms.iter().enumerate() {
                if s == c {
                    return self.child_node[lo + i];
                }
            }
            NONE
        } else {
            match syms.binary_search(&c) {
                Ok(i) => self.child_node[lo + i],
                Err(_) => NONE,
            }
        }
    }

    #[inline]
    pub fn step(&self, mut v: u32, c: Sym) -> u32 {
        loop {
            let w = self.child(v, c);
            if w != NONE {
                return w;
            }
            if v == 0 {
                return 0;
            }
            v = self.fail[v as usize];
        }
    }
}

/// Drop empty strings, duplicates, and strings contained in another input. Returns sorted output.
pub fn reduce_instance(strings: &[Word]) -> Vec<Word> {
    reduce_instance_keep_ac(strings).0
}

/// As `reduce_instance`, also returning the automaton of the result when it could be reused
/// (nothing was removed after deduplication).
pub fn reduce_instance_keep_ac(strings: &[Word]) -> (Vec<Word>, Option<AhoCorasick>) {
    use rayon::prelude::*;
    let mut tt = std::time::Instant::now();
    let mut s: Vec<Word> = strings.iter().filter(|x| !x.is_empty()).cloned().collect();
    s.par_sort_unstable();
    s.dedup();
    crate::counts::trace(" reduce: sort", &mut tt);
    // after deduplication a string can only occur inside a strictly longer one
    let minlen = s.iter().map(|x| x.len()).min().unwrap_or(0);
    let maxlen = s.iter().map(|x| x.len()).max().unwrap_or(0);
    if minlen == maxlen {
        return (s, None);
    }
    let ac = AhoCorasick::new(&s);
    let mut tt = std::time::Instant::now();
    // every pattern occurring inside another pattern (scan in parallel, mark sequentially)
    let hits: Vec<Vec<u32>> = s
        .par_iter()
        .enumerate()
        .map(|(i, x)| {
            let mut out = Vec::new();
            let mut v = 0u32;
            for &c in x {
                v = ac.step(v, c);
                let mut w = if ac.term[v as usize] != NONE {
                    v
                } else {
                    ac.dict_link[v as usize]
                };
                while w != 0 {
                    let j = ac.term[w as usize];
                    if j as usize != i {
                        out.push(j);
                    }
                    w = ac.dict_link[w as usize];
                }
            }
            out
        })
        .collect();
    crate::counts::trace(" reduce: scan", &mut tt);
    let mut dead = vec![false; s.len()];
    let mut any = false;
    for h in hits {
        for j in h {
            dead[j as usize] = true;
            any = true;
        }
    }
    if !any {
        return (s, Some(ac));
    }
    let kept = s
        .into_iter()
        .zip(dead)
        .filter(|(_, d)| !d)
        .map(|(x, _)| x)
        .collect();
    (kept, None)
}

/// Longest proper suffix of `a` that is a proper prefix of `b`.
pub fn overlap(a: &[Sym], b: &[Sym]) -> usize {
    let cap = a.len().min(b.len()).saturating_sub(1);
    if cap == 0 {
        return 0;
    }
    // prefix function of b, then run a through it (KMP)
    let mut pi = vec![0usize; b.len()];
    let mut k = 0;
    for i in 1..b.len() {
        while k > 0 && b[i] != b[k] {
            k = pi[k - 1];
        }
        if b[i] == b[k] {
            k += 1;
        }
        pi[i] = k;
    }
    let mut k = 0;
    for &c in a {
        while k > 0 && (k == b.len() || c != b[k]) {
            k = pi[k - 1];
        }
        if k < b.len() && c == b[k] {
            k += 1;
        }
    }
    while k > cap {
        k = pi[k - 1];
    }
    k
}

/// Concatenate a sequence, merging consecutive strings by their maximum overlap.
pub fn merge_in_order(seq: &[&Word]) -> Word {
    let mut out: Word = Vec::new();
    if let Some(first) = seq.first() {
        out.extend_from_slice(first);
    }
    for w in seq.windows(2) {
        let k = overlap(w[0], w[1]);
        out.extend_from_slice(&w[1][k..]);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Polynomial hashing modulo the Mersenne prime 2^61 - 1.
// ---------------------------------------------------------------------------------------------

pub const MOD: u64 = (1 << 61) - 1;
pub const BASE: u64 = 0x1d4e_9a3f_77c2_b51; // fixed odd base < MOD, deterministic across runs

#[inline]
pub fn mulmod(a: u64, b: u64) -> u64 {
    let p = (a as u128) * (b as u128);
    let lo = (p as u64) & MOD;
    let hi = (p >> 61) as u64;
    let s = lo + hi;
    if s >= MOD { s - MOD } else { s }
}

#[inline]
pub fn addmod(a: u64, b: u64) -> u64 {
    let s = a + b;
    if s >= MOD { s - MOD } else { s }
}

#[inline]
pub fn submod(a: u64, b: u64) -> u64 {
    if a >= b { a - b } else { a + MOD - b }
}

#[inline]
pub fn sym_val(c: Sym) -> u64 {
    c as u64 + 1
}

/// Hash of a word extended by one symbol on the right.
#[inline]
pub fn extend(h: u64, c: Sym) -> u64 {
    addmod(mulmod(h, BASE), sym_val(c))
}

/// Key identifying a word: (hash, length).
pub type Key = (u64, u32);

/// Prefix-hash tables for O(1) hashing of any slice of the inputs.
pub struct SliceHasher {
    pre: Vec<Vec<u64>>,
    pow: Vec<u64>,
}

impl SliceHasher {
    pub fn new(strings: &[Word]) -> Self {
        let maxlen = strings.iter().map(|s| s.len()).max().unwrap_or(0);
        let mut pow = vec![1u64; maxlen + 1];
        for i in 1..=maxlen {
            pow[i] = mulmod(pow[i - 1], BASE);
        }
        let pre = strings
            .iter()
            .map(|s| {
                let mut v = Vec::with_capacity(s.len() + 1);
                let mut h = 0u64;
                v.push(0);
                for &c in s {
                    h = extend(h, c);
                    v.push(h);
                }
                v
            })
            .collect();
        SliceHasher { pre, pow }
    }

    #[inline]
    pub fn hash(&self, si: usize, a: usize, b: usize) -> u64 {
        let p = &self.pre[si];
        submod(p[b], mulmod(p[a], self.pow[b - a]))
    }

    #[inline]
    pub fn key(&self, si: usize, a: usize, b: usize) -> Key {
        (self.hash(si, a, b), (b - a) as u32)
    }
}
