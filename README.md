# practical-scs-2opt

Toward a practical implementation of the polynomial-time 2-approximation for the
shortest common superstring (SCS) problem from the preprint
[*A Polynomial-Time 2-Approximation for Shortest Common Superstring*](https://github.com/openai/math/tree/main/preprints/A-Polynomial-Time-2-Approximation-for-Shortest-Common-Superstring-September-24-2026)
(OpenAI, September 2026).

The preprint's construction is defined over all O(L²) distinct substrings of the
input. The note in `scs/note/` proves that the construction's base graph lives on
the at most 2L prefixes and suffixes of the inputs. It also proves that the
period rule only needs pairs of *overlap words* (prefix and suffix at once), and
it gives a closed form for the blocking sums. Together these yield an algorithm
that never builds the substring set.

## Layout

| Path | Contents |
|---|---|
| `scs/note/overlap-support.typ` | The writeup (Typst, canonical); `overlap-support-typst.pdf` is the compiled version |
| `scs/note/overlap-support.tex` | Earlier LaTeX version (superseded: lacks Section 8) |
| `scs/counts.py` | Brute-force reference implementation of the preprint's Section 2 (small inputs only) |
| `scs/compact.py` | Compact count computation on prefixes/suffixes via the closed-form blocking sums |
| `scs/connect.py` | Connection phase (preprint Sections 3–7) and Euler-tour extraction of the superstring |
| `scs/greedy.py` | Efficient max-overlap greedy (Aho–Corasick) and the order-merge post-pass |
| `scs/experiments.py`, `scs/summarize.py` | Greedy-vs-2-approximation experiments; results in `scs/experiments_results.json` |
| `scs/validate_*.py`, `scs/test_*.py`, `scs/verify_lemmas.py` | Differential tests against the reference implementation and checks of the note's lemmas |
| `scs/make_fixtures*.py`, `scs/make_bench_data.py` | Oracle fixtures and benchmark inputs for the Rust code |
| `scs/fuzz_*.py`, `scs/search.py` | Randomized and coverage-guided fuzzing |
| `scs/exp*.py`, `scs/dominate.py`, `scs/leastcex.py` | Exploratory experiments behind the conjectures (now theorems) |

## Rust

`rust/` holds a Rust implementation of the full 2-approximation and of greedy, tested for exact
agreement (identical superstrings) with the Python reference; see [`rust/README.md`](rust/README.md).
[`OPTIMIZATIONS.md`](OPTIMIZATIONS.md) lists the optimizations that remain.

## Benchmarks

The full 2-approximation in Rust (`scs2 solve`: counts, connection phase, Euler tour, then
order-merge) against max-overlap greedy (`scs2 greedy`). Lengths are relative to the lower bound
W ≤ OPT, so 1.0000 means provably optimal. Inputs are 100 bp reads except the transcripts.
The 2-approximation ran with 16 threads on a 32-core workstation. Greedy's merging loop is
sequential; its reduction and automaton construction are parallel.

| Input | Total length L | Greedy length / W | Ours + order-merge length / W | Greedy time | Ours time | Ours peak memory |
|---|---|---|---|---|---|---|
| Reads from a random genome | 2·10⁶ | 1.0000 | 1.0000 | 0.12 s | 0.49 s | 0.5 GB |
| Reads from a random genome | 10⁷ | 1.0000 | 1.0000 | 0.64 s | 2.71 s | 2.2 GB |
| Reads from a repeat-rich genome | 2·10⁶ | 1.0091 | 1.0000 | 0.12 s | 0.92 s | 0.4 GB |
| Reads from a repeat-rich genome | 10⁷ | 1.0075 | 1.0002 | 0.59 s | 5.18 s | 2.1 GB |
| GENCODE human transcripts | 10⁷ | 1.0000 | 1.0000 | 1.15 s | 6.95 s | 2.9 GB |
| Reads simulated from GENCODE transcripts | 10⁷ | 1.0000 | 1.0000 | 0.60 s | 5.71 s | 2.2 GB |
| SEQC RNA-seq reads (real, with errors) | 10⁷ | 1.0000 | 1.0000 | 0.56 s | 6.55 s | 3.4 GB |

- On repeat-rich data, the period rule is active (114 and 519 hard cases). Greedy stays
  0.75-0.9% above W, and ours reaches W or comes within 0.02% of it.
- On real data, greedy is already within 0.005% of W, and so is ours. Here the 2-approximation
  adds a certificate of near-optimality rather than a shorter string.
- Ours costs 4-12x greedy's time. [`OPTIMIZATIONS.md`](OPTIMIZATIONS.md) lists where the
  remaining time goes.

More inputs, single-thread times and how to regenerate the inputs are in
[`rust/README.md`](rust/README.md#benchmarks). Section 9 of the note (`scs/note/`) describes the
implementation behind these numbers.

## Running

Python reference: pure Python 3, no dependencies (`typst` to rebuild the note).

```bash
cd scs
python3 experiments.py && python3 summarize.py   # comparison against greedy
python3 validate_compact.py 64                    # compact counts vs reference
python3 test_connect.py 64                        # end-to-end checks, ratio vs exact OPT
```

Rust implementation:

```bash
cd rust && cargo build --release
target/release/scs2 solve reads.txt -o superstring.txt   # FASTA or one string per line
target/release/scs2 greedy reads.txt -o greedy.txt
cargo test --release                                      # exact agreement with the Python oracle
```
