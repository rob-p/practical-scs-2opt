//! Input parsing: FASTA (records may span lines) or one string per line.

use crate::strings::{Word, to_word};
use std::io::{BufRead, BufReader, Read};

pub fn read_strings<R: Read>(r: R) -> std::io::Result<Vec<Word>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut fasta = false;
    for line in BufReader::new(r).lines() {
        let line = line?;
        let line = line.trim_end();
        if line.starts_with('>') {
            fasta = true;
            if !cur.is_empty() {
                out.push(to_word(&cur));
                cur.clear();
            }
        } else if fasta {
            cur.push_str(line);
        } else if !line.is_empty() {
            out.push(to_word(line));
        }
    }
    if !cur.is_empty() {
        out.push(to_word(&cur));
    }
    Ok(out)
}
