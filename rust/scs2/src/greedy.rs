//! Maximum-overlap greedy (Ukkonen / Tarhio-Ukkonen formulation) and the order-merge post-pass.
//!
//! Mirrors `scs/greedy.py` exactly, including tie-breaking, so outputs can be compared verbatim.

use crate::strings::{AhoCorasick, NONE, Sym, Word, merge_in_order, reduce_instance};

fn find(p: &mut [u32], i: u32) -> u32 {
    let mut root = i;
    while p[root as usize] != root {
        root = p[root as usize];
    }
    let mut i = i;
    while p[i as usize] != root {
        let next = p[i as usize];
        p[i as usize] = root;
        i = next;
    }
    root
}

/// Greedy shortest common superstring.
pub fn greedy_scs(strings: &[Word]) -> Word {
    let s = reduce_instance(strings); // sorted
    let n = s.len();
    if n == 0 {
        return Vec::new();
    }
    let ac = AhoCorasick::new(&s);
    // candidate overlaps, bucketed by length; within a bucket in order of (a, chain position)
    let maxlen = s.iter().map(|x| x.len()).max().unwrap();
    let mut buckets: Vec<Vec<(u32, u32)>> = vec![Vec::new(); maxlen + 1];
    for a in 0..n {
        let mut v = ac.fail[ac.end_node[a] as usize];
        while v != 0 {
            buckets[ac.depth[v as usize] as usize].push((a as u32, v));
            v = ac.fail[v as usize];
        }
    }
    let mut succ = vec![NONE; n];
    let mut has_pred = vec![false; n];
    let mut next_free: Vec<u32> = (0..=n as u32).collect();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    for k in (1..=maxlen).rev() {
        for &(a, v) in &buckets[k] {
            if succ[a as usize] != NONE {
                continue;
            }
            let (lo, hi) = (ac.lo[v as usize], ac.hi[v as usize]);
            let mut i = find(&mut next_free, lo);
            while i < hi {
                let ra = find(&mut parent, a);
                if find(&mut parent, i) != ra {
                    succ[a as usize] = i;
                    has_pred[i as usize] = true;
                    next_free[i as usize] = i + 1;
                    let ri = find(&mut parent, i);
                    parent[ra as usize] = ri;
                    break;
                }
                i = find(&mut next_free, i + 1);
            }
        }
    }
    let mut out = Vec::new();
    for h in 0..n {
        if has_pred[h] {
            continue;
        }
        let mut seq: Vec<&Word> = vec![&s[h]];
        let mut v = h;
        while succ[v] != NONE {
            v = succ[v] as usize;
            seq.push(&s[v]);
        }
        out.extend(merge_in_order(&seq));
    }
    out
}

/// Reorder the inputs by leftmost occurrence in the superstring `t` and merge consecutive strings
/// with maximum overlap. The result is never longer than `t`.
pub fn order_merge(t: &[Sym], strings: &[Word]) -> Word {
    let s = reduce_instance(strings);
    let ac = AhoCorasick::new(&s);
    let mut first = vec![usize::MAX; s.len()];
    let mut v = 0u32;
    for (pos, &c) in t.iter().enumerate() {
        v = ac.step(v, c);
        let mut w = if ac.term[v as usize] != NONE {
            v
        } else {
            ac.dict_link[v as usize]
        };
        while w != 0 {
            let j = ac.term[w as usize] as usize;
            if first[j] == usize::MAX {
                first[j] = pos + 1 - s[j].len();
            }
            w = ac.dict_link[w as usize];
        }
    }
    assert!(
        first.iter().all(|&f| f != usize::MAX),
        "t is not a superstring"
    );
    let mut order: Vec<usize> = (0..s.len()).collect();
    order.sort_by_key(|&j| first[j]);
    let seq: Vec<&Word> = order.iter().map(|&j| &s[j]).collect();
    let r = merge_in_order(&seq);
    assert!(r.len() <= t.len());
    r
}
