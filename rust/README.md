# scs2 (Rust)

Rust implementations of the greedy baseline and of the forced-count step of the SCS
2-approximation. The Python code in `../scs/` is the reference oracle: it is validated against the
brute-force construction of the preprint, and the Rust code must reproduce it exactly.

| Module | Contents |
|---|---|
| `strings.rs` | Aho–Corasick automaton (trie built by LCP from sorted patterns, CSR children), instance reduction, overlaps, polynomial hashing |
| `greedy.rs` | Max-overlap greedy (same tie-breaking as `scs/greedy.py`) and order-merge |
| `counts.rs` | Forced counts u, d on prefixes/suffixes (`scs/compact.py`), with words as trie nodes |
| `io.rs`, `main.rs` | FASTA / one-string-per-line input; `scs2 greedy|counts <file>` prints JSON stats |

The connection phase (`scs/connect.py`) is not ported yet.

## Testing

```bash
cd scs && python3 make_fixtures.py ../rust/scs2/tests/fixtures/oracle.json      # committed
python3 make_fixtures_medium.py ../rust/scs2/tests/fixtures/oracle_medium.json 64  # not committed
cd ../rust && cargo test --release -- --include-ignored
```

- `counts_match_oracle`: u, d and W equal the oracle on 3,712 small instances.
- `greedy_matches_oracle`: greedy output is identical to the oracle string.
- `counts_match_oracle_medium`: 64 medium repeat-rich read sets (52k reads); ignored by default
  because the fixture is generated on demand.

## Benchmarks

Error-free 100 bp reads; `python3 scs/make_bench_data.py` writes the inputs to `rust/bench_data/`.
Single thread, release build, peak RSS from `/usr/bin/time`.

| Input | L | greedy | greedy RSS | counts | counts RSS | W |
|---|---|---|---|---|---|---|
| `random_G20000_n1000_l100` | 100,000 | 0.01 s | 7 MB | 0.03 s | 22 MB | 19,793 |
| `repeats_G20000_n1000_l100` | 100,000 | 0.01 s | 6 MB | 0.07 s | 21 MB | 17,299 |
| `random_G200000_n20000_l100` | 2,000,000 | 0.17 s | 101 MB | 1.56 s | 372 MB | 199,986 |
| `repeats_G200000_n20000_l100` | 2,000,000 | 0.18 s | 92 MB | 2.18 s | 356 MB | 183,692 |
| `random_G1000000_n100000_l100` | 10,000,000 | 2.11 s | 489 MB | 13.03 s | 1823 MB | 999,937 |
| `repeats_G1000000_n100000_l100` | 10,000,000 | 1.85 s | 459 MB | 17.59 s | 1718 MB | 910,809 |

For comparison, the Python `compact.py` takes about 80–100 s at L = 2,000,000.
