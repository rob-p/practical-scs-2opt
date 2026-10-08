#!/usr/bin/env bash
# Extract real-data benchmark inputs (one sequence per line) into rust/bench_data/real/.
# Usage: scs/make_real_data.sh <gencode_pc_transcripts.fa> <gencode_sim_reads.fasta> <seqc_reads.fq.zst>
set -euo pipefail
out="$(dirname "$0")/../rust/bench_data/real"
mkdir -p "$out"
tx="$1"; sim="$2"; seqc="$3"
# GENCODE transcripts, whole records, in file order (genes are contiguous); stop at ~N bases
for n in 2000000 10000000; do
  awk -v lim="$n" '/^>/{if(s!=""){print s; tot+=length(s); if(tot>=lim) exit}; s=""; next} {s=s toupper($0)} END{if(tot<lim && s!="") print s}' "$tx" > "$out/gencode_tx_${n}.txt"
done
# Simulated reads from GENCODE transcripts (first n reads)
for n in 20000 100000; do
  awk -v lim="$n" '!/^>/{print toupper($0); if(++k>=lim) exit}' "$sim" > "$out/gencode_simreads_${n}.txt"
done
# Real RNA-seq reads (SEQC, with sequencing errors)
for n in 20000 100000; do
  zstdcat "$seqc" | awk -v lim="$n" 'NR%4==2{print toupper($0); if(++k>=lim) exit}' > "$out/seqc_reads_${n}.txt" || true
done
wc -lc "$out"/*.txt
