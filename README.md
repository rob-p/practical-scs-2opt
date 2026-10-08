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

## Running

Pure Python 3, no dependencies (`typst` to rebuild the note).

```bash
cd scs
python3 experiments.py && python3 summarize.py   # comparison against greedy
python3 validate_compact.py 64                    # compact counts vs reference
python3 test_connect.py 64                        # end-to-end checks, ratio vs exact OPT
```
