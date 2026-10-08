# Remaining optimizations

These notes describe the state at commit `9d49004`. Numbers are for inputs of L = 10^7 on the
32-core workstation used throughout. "16 threads" means `RAYON_NUM_THREADS=16`, and
`SCS2_TRACE=1 scs2 solve <file>` prints the per-phase times quoted here.

## Where the time goes now

| Input (10^7) | greedy | solve, 16 threads | counts | connection phase + Euler tour |
|---|---|---|---|---|
| random genome reads | 0.64 s | 2.7 s | 1.8 s | 0.8 s |
| repeat-rich genome reads | 0.59 s | 5.2 s | 4.2 s | 0.9 s |
| GENCODE transcripts | 1.15 s | 7.0 s | 4.2 s | 2.7 s |
| SEQC RNA-seq reads | 0.56 s | 6.6 s | 3.2 s | 3.1 s |

The full algorithm costs 4-12x greedy. On real data W is close to L, so the connection phase is
O(W) and takes a large share. On repeat-rich data the parallel count loop dominates.

## 1. Connection phase (sequential; 0.8-3.1 s)

The largest remaining sequential block, and the main cost on real data:

- **Closed-walk decomposition (0.4-1.6 s).** Start vertices are sorted by (length, word) to match
  the Python order. Words in Q \ P need slice comparisons, while forward-trie nodes compare by id.
  - Precompute lexicographic ranks of Q \ P words once, by merging with the forward-trie preorder,
    so the sort becomes an integer sort.
  - Up lists are materialized with one entry per unit of multiplicity. Storing (child, count)
    pairs instead would make them O(support) rather than O(W).
- **Layer construction (0.1-0.9 s).** When the base graph is one giant walk (transcripts, W ≈ 10^7),
  the cyclic root, minimal rotation and per-position arrays run over the whole walk text.
  Independent walks and groups could be processed in parallel. The walk text could be kept as
  word ids rather than copied symbols.
- **Final graph and Euler tour (0.4-1 s).** Each added-walk vertex is resolved by trie lookups
  costing O(|word|). Incremental resolution along a path would make it O(1) per edge: the parent
  is the forward-trie parent, and the suffix is the reversed-trie parent. Hierholzer itself is
  sequential, but the tour could be stitched from per-component tours built in parallel.
- **Python-order constraints.** Several orders exist only so the output matches the Python
  oracle byte for byte: u-edge order, start-vertex order, and up-list insertion order. If both
  implementations switched to an order that is natural for integer ids (for example the
  forward-trie preorder for every vertex), the matching tests would still hold and several sorts
  would disappear.

## 2. Parallel count loop (1.2-3 s on 16 threads)

- **Memory-bound scaling.** Phase A stops improving beyond about 16 threads. Per-word CPU time
  roughly quadruples under load (32 cores, four 32 MB L3 slices), so the loop is limited by
  random access into large arrays, not by load imbalance.
  - Shrink the per-word arrays: `i64` → `i32` for λ, ρ, u, d (all ≤ L < 2^31).
  - Pack the per-node fields that are read together into one struct, for locality.
  - Renumber reversed-trie-only words so that same-length words are contiguous.
- **D_A walks (hash lookups).** Each rule evaluation walks A[0:n) to the maximum input length,
  with a hash lookup of the d support per step, because the prefix closure of the d support is
  not indexed. This is cheap for 100 bp reads but grows with input length (transcripts reach
  16 kb).
  - Options: stop at the longest d-support word extending the current one; index the d support by
    its reversed-trie nodes; or cache walks per template, since D_A depends only on A and on
    lengths above |x|.
- **Phase B (0.5-1 s).** Its sequential part (output lists, the d map, merging pending rules)
  could be split by target or sharded.

## 3. Count setup (0.6-1.3 s)

- **Instance reduction when lengths vary (0.57 s on transcripts).** The containment automaton
  over all inputs is built, then a second forward trie is built over the reduced set. The
  reduced trie could instead be derived from the first by pruning nodes with no surviving
  pattern and recomputing failure links only where they change. Alternatively, containment could
  be detected directly with the forward and reversed tries.
- **Trie construction itself** (about 0.1 s for the trie, 0.08 s for CSR and dense tables, per
  trie) is sequential. The sorted inputs could be split into blocks at points where the
  longest common prefix is 0, giving independent subtrees, so tries could be built in parallel
  and concatenated.

## 4. Memory (2.2-3.4 GB at 10^7, about 220-350 bytes per input symbol)

- Use 32-bit counts and sums (see 2).
- The dense child tables cost 4·σ bytes per node per trie. They could be dropped for one of the
  two tries, or replaced by a compact CSR for nodes with one child (most nodes).
- `reduce_instance` copies every input string. It could sort and deduplicate indices instead.
- Connection-phase edges carry a reference-counted `Text` clone each. Interning texts and storing
  a `u32` index would make each edge 3 words.

## 5. Record search

Each hard case enumerates O(maxlen²) candidate windows A[a:b) and looks each one up in the window
index. This is negligible for reads (at most 519 hard cases), but quadratic in input length.

- Enumerate only the ends b at which A[a:b) is still a vertex (stop at the first missing
  prefix), or index layer windows by their periodic text and offset rather than by word hash.

## 6. Not speed, but related

- Six connection-phase branches were never reached by coverage-guided fuzzing: lower layers below
  a baseline, self-cycles, same-group records, and hosts with requests from more than one child
  group. Hand-built instances would let their checks actually run.
- Output quality: on real data, greedy is within 0.005% of W and so is ours. Several choices the
  paper leaves free (which record, which cycle block, Euler start) could be made to favour shorter
  raw output. Order-merge already recovers most of the gap.
