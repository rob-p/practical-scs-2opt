# scs2 (Rust)

Rust implementation of the full SCS 2-approximation (forced counts, connection phase, Euler
tour) and of the greedy baseline. The Python code in `../scs/` is the reference oracle: it is validated against the
brute-force construction of the preprint, and the Rust code must reproduce it exactly.

| Module | Contents |
|---|---|
| `strings.rs` | Aho–Corasick automaton (trie built by LCP from sorted patterns, CSR children), instance reduction, overlaps, polynomial hashing |
| `greedy.rs` | Max-overlap greedy (same tie-breaking as `scs/greedy.py`) and order-merge |
| `counts.rs` | Forced counts u, d on prefixes/suffixes (`scs/compact.py`), with words as trie nodes; parallel by length level |
| `connect.rs` | Connection phase and Euler tour (`scs/connect.py`); vertices are word hashes, record search is indexed |
| `io.rs`, `main.rs` | FASTA / one-string-per-line input; `scs2 greedy|counts|solve <file> [-o out]` prints JSON stats |

`scs2 solve` runs the 2-approximation followed by order-merge and writes the latter with `-o`.
Thread count follows `RAYON_NUM_THREADS`.

## Testing

```bash
cd scs && python3 make_fixtures.py ../rust/scs2/tests/fixtures/oracle.json      # committed
python3 make_fixtures_medium.py ../rust/scs2/tests/fixtures/oracle_medium.json 64  # not committed
cd ../rust && cargo test --release -- --include-ignored
```

- `counts_match_oracle`: u, d and W equal the oracle on 3,712 small instances.
- `greedy_matches_oracle`: greedy output is identical to the oracle string.
- `superstring_matches_oracle`: the 2-approximation's superstring is identical to the oracle's.
- `*_medium`: the same checks on 64 medium repeat-rich read sets (52k reads); ignored by default
  because the fixture is generated on demand.

## Benchmarks

Error-free 100 bp reads; `python3 scs/make_bench_data.py` writes the inputs to `rust/bench_data/`.
Release build; greedy is single-threaded, `solve` ran with 16 threads (counts parallel, connection
phase and order-merge sequential). Lengths are relative to the lower bound W <= OPT.

| Input | L | greedy | greedy time | ours | ours + order-merge | ours, 1 thread | ours, 16 threads | ours RSS |
|---|---|---|---|---|---|---|---|---|
| `random_G20000_n1000_l100` | 10^5 | 1.0000 | 0.01 s | 1.0017 | 1.0000 | 0.05 s | 0.04 s | 24 MB |
| `repeats_G20000_n1000_l100` | 10^5 | 1.0080 | 0.01 s | 1.0208 | 1.0042 | 0.08 s | 0.08 s | 21 MB |
| `random_G200000_n20000_l100` | 2·10^6 | 1.0000 | 0.22 s | 1.0005 | 1.0000 | 1.66 s | 0.74 s | 407 MB |
| `repeats_G200000_n20000_l100` | 2·10^6 | 1.0091 | 0.22 s | 1.0075 | 1.0000 | 2.08 s | 1.14 s | 387 MB |
| `random_G1000000_n100000_l100` | 10^7 | 1.0000 | 2.03 s | 1.0001 | 1.0000 | 11.63 s | 5.74 s | 2025 MB |
| `repeats_G1000000_n100000_l100` | 10^7 | 1.0075 | 1.99 s | 1.0068 | 1.0002 | 15.72 s | 7.69 s | 1880 MB |

With 16 threads at 10^7 the time splits into counts (3.4-5.6 s, of which about 2.2 s of
sequential setup), connection phase and Euler tour (about 2 s, sequential) and order-merge
(0.1 s). `SCS2_TRACE=1` prints the breakdown. The Python reference needs 80-100 s for the
counts at 2·10^6.

## Real data

`scs/make_real_data.sh` extracts inputs from local copies of GENCODE v49 protein-coding transcripts
(human; real sequence, isoforms share exons), reads simulated from those transcripts, and SEQC
RNA-seq reads (real Illumina reads with sequencing errors and `N`). Same settings as above.

| Input | n (reduced) | L | W | greedy | greedy time | ours | ours + order-merge | ours time (16 threads) | ours RSS | hard cases |
|---|---|---|---|---|---|---|---|---|---|---|
| `gencode_tx_2000000` | 802 | 2,000,333 | 1,985,792 | 1.00000 | 0.20 s | 1.00137 | 1.00000 | 1.46 s | 640 MB | 0 |
| `gencode_tx_10000000` | 3,728 | 10,001,916 | 9,885,614 | 1.00000 | 1.17 s | 1.00058 | 1.00000 | 7.54 s | 2991 MB | 0 |
| `gencode_simreads_20000` | 17,345 | 2,000,000 | 597,614 | 1.00000 | 0.25 s | 1.00014 | 1.00001 | 1.08 s | 487 MB | 0 |
| `gencode_simreads_100000` | 84,766 | 10,000,000 | 3,049,355 | 1.00000 | 2.54 s | 1.00003 | 1.00000 | 8.12 s | 2203 MB | 0 |
| `seqc_reads_20000` | 19,735 | 2,000,000 | 1,604,510 | 1.00004 | 0.28 s | 1.00017 | 1.00008 | 1.44 s | 710 MB | 0 |
| `seqc_reads_100000` | 95,419 | 10,000,000 | 6,984,207 | 1.00001 | 2.57 s | 1.00006 | 1.00004 | 9.06 s | 3464 MB | 1 |

On these inputs greedy is already within 0.005% of the lower bound W, so neither algorithm has
room to improve; ours certifies near-optimality, at 3.5-7x greedy's time. Periodic structure that
forces the paper's period rule (hard cases) is essentially absent. That contrasts with the
synthetic tandem-repeat genomes above, where greedy is 0.75-0.9% above W and ours reaches W.
