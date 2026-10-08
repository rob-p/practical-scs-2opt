use scs2::{counts::compute_counts, greedy, io::read_strings, strings::to_string};
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
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}
