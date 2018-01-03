use std::{env, mem};
use std::fs::File;
use std::io::*;
use std::path::Path;
use std::process::*;
use std::str::FromStr;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut f = File::create(&Path::new(&out_dir).join("e.rs")).unwrap();

    let c = Command::new("cc").args(&["-E", "-dM", "-"])
                              .stdin (Stdio::piped())
                              .stdout(Stdio::piped())
                              .spawn().unwrap();
    writeln!(c.stdin.unwrap(), "#include <errno.h>").unwrap();
    let mut es = Vec::with_capacity(256);
    for mut l in Lines(c.stdout.unwrap().bytes().map(Result::unwrap), (|&b| b'\n' == b) as fn(&u8) -> bool) {
        if !l.starts_with(&b"#define E"[..]) { continue; }
        l.drain(0..8);
        let n = match l.iter().position(|&b| b' ' == b)
                       .and_then(|k| String::from_utf8(l.split_off(k).split_off(1)).ok()) {
            Some(n) => n,
            None => continue,
        };
        let e = match String::from_utf8(l) {
            Ok(e) => e,
            _ => continue,
        };
        match usize::from_str(&n) {
            Ok(n) => {
                writeln!(&mut f, "pub const {}: OsErr = OsErr({});", e, n).unwrap();
                put_opt(&mut es, n, e);
            },
            _ => {
                writeln!(&mut f, "pub const {}: OsErr = {};", e, n).unwrap();
            }
        }
    }
    writeln!(&mut f, "const error_names: [Option<&'static str>; {}] = [", es.len()).unwrap();
    for e in &es { writeln!(&mut f, "    {:?},", e).unwrap(); }
    writeln!(&mut f, "];").unwrap();
}

fn put_opt<A>(xs: &mut Vec<Option<A>>, k: usize, x: A) -> Option<A> {
    if k >= xs.len() { for _ in xs.len()..k+1 { xs.push(None) } }
    mem::replace(&mut xs[k], Some(x))
}

struct Lines<I: Iterator, P>(I, P);

impl<I: Iterator, P: Fn(&I::Item) -> bool> Iterator for Lines<I, P> {
    type Item = Vec<I::Item>;

    #[inline]
    fn next(&mut self) -> Option<Vec<I::Item>> {
        let mut xs = Vec::new();
        while let Some(x) = self.0.next() {
            if self.1(&x) { return Some(xs); }
            xs.push(x);
        }
        None
    }
}
