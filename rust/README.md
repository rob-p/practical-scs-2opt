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
Release build. `solve` ran with 16 threads (counts parallel, connection phase and order-merge
sequential); greedy ran with the default thread pool (its reduction and automaton construction are
parallel, its merging loop sequential). Lengths are relative to the lower bound W <= OPT.

| Input | L | greedy | greedy time | ours | ours + order-merge | ours, 1 thread | ours, 16 threads | ours RSS |
|---|---|---|---|---|---|---|---|---|
| `random_G20000_n1000_l100` | 10^5 | 1.0000 | 0.01 s | 1.0017 | 1.0000 | 0.05 s | 0.03 s | 26 MB |
| `repeats_G20000_n1000_l100` | 10^5 | 1.0080 | 0.01 s | 1.0208 | 1.0042 | 0.06 s | 0.06 s | 26 MB |
| `random_G200000_n20000_l100` | 2·10^6 | 1.0000 | 0.12 s | 1.0005 | 1.0000 | 1.46 s | 0.49 s | 486 MB |
| `repeats_G200000_n20000_l100` | 2·10^6 | 1.0091 | 0.12 s | 1.0075 | 1.0000 | 1.81 s | 0.92 s | 454 MB |
| `random_G1000000_n100000_l100` | 10^7 | 1.0000 | 0.64 s | 1.0001 | 1.0000 | 9.62 s | 2.71 s | 2289 MB |
| `repeats_G1000000_n100000_l100` | 10^7 | 1.0075 | 0.59 s | 1.0068 | 1.0002 | 13.45 s | 5.18 s | 2177 MB |

With 16 threads at 10^7 the time splits into counts (1.8-4.2 s, of which 0.6-1.3 s of mostly
parallel setup), connection phase and Euler tour (0.8-3 s, sequential) and order-merge (0.1 s).
`SCS2_TRACE=1` prints the breakdown. Greedy shares the instance reduction and the automaton
construction, so it also benefits from their parallelization. The Python reference needs 80-100 s for the
counts at 2·10^6.

## Real data

`scs/make_real_data.sh` extracts inputs from local copies of GENCODE v49 protein-coding transcripts
(human; real sequence, isoforms share exons), reads simulated from those transcripts, and SEQC
RNA-seq reads (real Illumina reads with sequencing errors and `N`). Same settings as above.

| Input | n (reduced) | L | W | greedy | greedy time | ours | ours + order-merge | ours time (16 threads) | ours RSS | hard cases |
|---|---|---|---|---|---|---|---|---|---|---|
| `gencode_tx_2000000` | 802 | 2,000,333 | 1,985,792 | 1.00000 | 0.17 s | 1.00137 | 1.00000 | 1.38 s | 656 MB | 0 |
| `gencode_tx_10000000` | 3,728 | 10,001,916 | 9,885,614 | 1.00000 | 1.15 s | 1.00058 | 1.00000 | 6.95 s | 2956 MB | 0 |
| `gencode_simreads_20000` | 17,345 | 2,000,000 | 597,614 | 1.00000 | 0.12 s | 1.00014 | 1.00001 | 0.87 s | 498 MB | 0 |
| `gencode_simreads_100000` | 84,766 | 10,000,000 | 3,049,355 | 1.00000 | 0.60 s | 1.00003 | 1.00000 | 5.71 s | 2202 MB | 0 |
| `seqc_reads_20000` | 19,735 | 2,000,000 | 1,604,510 | 1.00004 | 0.12 s | 1.00017 | 1.00008 | 1.09 s | 689 MB | 0 |
| `seqc_reads_100000` | 95,419 | 10,000,000 | 6,984,207 | 1.00001 | 0.56 s | 1.00006 | 1.00004 | 6.55 s | 3447 MB | 1 |

On these inputs greedy is already within 0.005% of the lower bound W, so neither algorithm has
room to improve; ours certifies near-optimality, at 6-12x greedy's time. Periodic structure that
forces the paper's period rule (hard cases) is essentially absent. That contrasts with the
synthetic tandem-repeat genomes above, where greedy is 0.75-0.9% above W and ours reaches W.
