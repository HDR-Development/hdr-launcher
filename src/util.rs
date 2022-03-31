use std::fs::*;
use std::io::Read;

pub fn filename_to_string(s: &str) -> std::io::Result<String> {
    let mut file = File::open(s)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    Ok(s)
}

pub fn lines_split<'a>(s: &'a str, split_str: &str) -> Vec<Vec<&'a str>> {
    s.lines().map(|line| {
        line.split(split_str).collect()
    }).collect()
}

