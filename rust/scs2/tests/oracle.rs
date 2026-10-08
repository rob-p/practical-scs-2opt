//! Differential tests against the Python oracle (`scs/make_fixtures.py`).
//!
//! The counts u, d (and hence W) are uniquely determined by the input, so they must match exactly.
//! Greedy replicates the Python tie-breaking, so its output string must match verbatim.

use rustc_hash::FxHashMap;
use scs2::counts::compute_counts;
use scs2::greedy::{greedy_scs, order_merge};
use scs2::strings::{Word, reduce_instance, to_string, to_word};
use serde_json::Value;

fn load_file(name: &str) -> Vec<Value> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing {path}; see scs/make_fixtures*.py"));
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

fn load() -> Vec<Value> {
    load_file("oracle.json")
}

fn strings_of(inst: &Value) -> Vec<Word> {
    inst["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| to_word(s.as_str().unwrap()))
        .collect()
}

fn map_of(v: &Value) -> FxHashMap<String, i64> {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, x)| (k.clone(), x.as_i64().unwrap()))
        .collect()
}

#[test]
fn counts_match_oracle() {
    check_counts(&load());
}

/// Medium repeat-rich instances; generate with
/// `python3 scs/make_fixtures_medium.py rust/scs2/tests/fixtures/oracle_medium.json 64`.
#[test]
#[ignore]
fn counts_match_oracle_medium() {
    check_counts(&load_file("oracle_medium.json"));
}

fn check_counts(insts: &[Value]) {
    let mut failures = 0;
    for (i, inst) in insts.iter().enumerate() {
        let c = compute_counts(&strings_of(inst));
        let u: FxHashMap<String, i64> =
            c.u.iter()
                .map(|(o, k)| (to_string(c.word(o)), *k))
                .collect();
        let d: FxHashMap<String, i64> =
            c.d.iter()
                .map(|(o, k)| (to_string(c.word(o)), *k))
                .collect();
        let ok = c.w == inst["W"].as_i64().unwrap()
            && u == map_of(&inst["u"])
            && d == map_of(&inst["d"]);
        if !ok {
            failures += 1;
            if failures <= 3 {
                eprintln!(
                    "instance {i}: W rust={} oracle={} strings={}",
                    c.w, inst["W"], inst["strings"]
                );
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "{failures} of {} instances differ",
        insts.len()
    );
}

#[test]
fn greedy_matches_oracle() {
    let insts = load();
    let mut failures = 0;
    for (i, inst) in insts.iter().enumerate() {
        let strs = strings_of(inst);
        let g = greedy_scs(&strs);
        if to_string(&g) != inst["greedy"].as_str().unwrap() {
            failures += 1;
            if failures <= 3 {
                eprintln!(
                    "instance {i}: rust={} oracle={}",
                    to_string(&g),
                    inst["greedy"]
                );
            }
        }
        for x in reduce_instance(&strs) {
            assert!(
                g.windows(x.len()).any(|w| w == &x[..]),
                "greedy output misses an input"
            );
        }
        let r = order_merge(&g, &strs);
        assert!(r.len() <= g.len());
    }
    assert_eq!(
        failures,
        0,
        "{failures} of {} instances differ",
        insts.len()
    );
}

fn check_superstrings(insts: &[Value]) {
    let mut failures = 0;
    for (i, inst) in insts.iter().enumerate() {
        let strs = strings_of(inst);
        let c = compute_counts(&strs);
        let (t, _) = scs2::connect::superstring(&c);
        if to_string(&t) != inst["superstring"].as_str().unwrap() {
            failures += 1;
            if failures <= 3 {
                eprintln!(
                    "instance {i}: rust len {} oracle len {} strings={}",
                    t.len(),
                    inst["superstring"].as_str().unwrap().chars().count(),
                    inst["strings"]
                );
            }
        }
        assert!(t.len() as i64 <= 2 * c.w, "longer than 2W");
        for x in &c.strings {
            assert!(
                t.windows(x.len()).any(|w| w == &x[..]),
                "superstring misses an input"
            );
        }
    }
    assert_eq!(
        failures,
        0,
        "{failures} of {} superstrings differ",
        insts.len()
    );
}

#[test]
fn superstring_matches_oracle() {
    check_superstrings(&load());
}

#[test]
#[ignore]
fn superstring_matches_oracle_medium() {
    check_superstrings(&load_file("oracle_medium.json"));
}
