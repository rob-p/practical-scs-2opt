use scs2::{connect, counts::compute_counts, greedy, io::read_strings, strings::to_string};
use std::time::Instant;

const USAGE: &str =
    "usage: scs2 <greedy|counts> <input (FASTA or one string per line), - for stdin> [-o output]";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let input = if args[2] == "-" {
        read_strings(std::io::stdin())
    } else {
        read_strings(std::fs::File::open(&args[2]).expect("cannot open input"))
    }
    .expect("read error");
    let out_path = args
        .iter()
        .position(|a| a == "-o")
        .map(|i| args[i + 1].clone());
    let total_len: usize = input.iter().map(|s| s.len()).sum();
    let t = Instant::now();
    match args[1].as_str() {
        "greedy" => {
            let g = greedy::greedy_scs(&input);
            let dt = t.elapsed().as_secs_f64();
            println!(
                "{{\"cmd\":\"greedy\",\"n\":{},\"L\":{},\"len\":{},\"secs\":{:.4}}}",
                input.len(),
                total_len,
                g.len(),
                dt
            );
            if let Some(p) = out_path {
                std::fs::write(p, to_string(&g)).unwrap();
            }
        }
        "counts" => {
            let c = compute_counts(&input);
            let dt = t.elapsed().as_secs_f64();
            println!(
                "{{\"cmd\":\"counts\",\"n\":{},\"L\":{},\"W\":{},\"u_support\":{},\"d_support\":{},\"words\":{},\"overlap_words\":{},\"rules\":{},\"fired\":{},\"t_setup\":{:.2},\"t_phase_a\":{:.2},\"t_phase_b\":{:.2},\"secs\":{:.4}}}",
                c.strings.len(),
                total_len,
                c.w,
                c.u.len(),
                c.d.len(),
                c.stats.words,
                c.stats.overlap_words,
                c.stats.rules,
                c.stats.fired,
                c.stats.t_setup,
                c.stats.t_phase_a,
                c.stats.t_phase_b,
                dt
            );
        }
        "solve" => {
            let c = compute_counts(&input);
            let t_counts = t.elapsed().as_secs_f64();
            let (s, st) = connect::superstring(&c);
            let t_connect = t.elapsed().as_secs_f64() - t_counts;
            let t_om = std::time::Instant::now();
            let om = greedy::order_merge_with(&s, &c.strings, &c.trie);
            if scs2::counts::tracing() {
                eprintln!(
                    "  [         order-merge] {:.2}s",
                    t_om.elapsed().as_secs_f64()
                );
            }
            let dt = t.elapsed().as_secs_f64();
            println!(
                "{{\"cmd\":\"solve\",\"n\":{},\"L\":{},\"W\":{},\"len\":{},\"len_om\":{},\"added\":{},\"groups\":{},\"layers\":{},\"blocks\":{},\"hard_cases\":{},\"requests\":{},\"cycles\":{},\"t_counts\":{:.3},\"t_connect\":{:.3},\"secs\":{:.3}}}",
                c.strings.len(),
                total_len,
                c.w,
                s.len(),
                om.len(),
                st.added,
                st.groups,
                st.layers,
                st.blocks,
                st.hard_cases,
                st.requests,
                st.cycles,
                t_counts,
                t_connect,
                dt
            );
            if let Some(p) = out_path {
                std::fs::write(p, to_string(&om)).unwrap();
            }
        }
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}
