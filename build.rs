extern crate libc;

use std::{env, mem};
use std::fs::File;
use std::io::*;
use std::path::Path;
use std::process::*;
use std::str::FromStr;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut f = File::create(&Path::new(&out_dir).join("e.rs")).unwrap();
    let mut ecf = File::create(&Path::new(&out_dir).join("error_consts.rs")).unwrap(); // blah
    let mut es = Vec::with_capacity(256);
    let mut ss = Vec::with_capacity(256);
    writeln!(&mut f, "impl Error {{").unwrap();
    c_defns("errno.h", |e, n| { if e.starts_with("E") {
        match usize::from_str(&n) {
            Ok(0) => (),
            Ok(n) => {
                if let (Ok(s), true) = (unsafe { ::std::ffi::CStr::from_ptr(::libc::strerror(n as _)) }.to_str(),
                                        env::var("HOST") == env::var("TARGET")) {
                    writeln!(&mut f, "/// {}", s)?;
                    put_opt(&mut ss, n, s);
                }
                writeln!(&mut ecf, "pub const {0}: OsErr = OsErr::{0};", e).unwrap();
                writeln!(&mut f, "pub const {}: Self = Error(unsafe {{ NonZeroUsize::new_unchecked({}) }});", e, n)?;
                put_opt(&mut es, n, e);
            },
            _ => {
                writeln!(&mut f, "/// Alias for [{0}](#associatedconstant.{0})", n)?;
                writeln!(&mut f, "pub const {}: Self = Self::{};", e, n)?;
            },
        }
    } Ok(()) }).unwrap();
    writeln!(&mut f, "}}").unwrap(); // blah

    writeln!(&mut f, "const error_names: [Option<&str>; {}] = [", es.len()).unwrap();
    for e in &es { writeln!(&mut f, "    {:?},", e).unwrap(); }
    writeln!(&mut f, "];").unwrap();

    writeln!(&mut f, "const error_messages: [&str; {}] = [", ss.len()).unwrap();
    for s in &ss { writeln!(&mut f, "    {:?},", s.unwrap_or("")).unwrap(); }
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

fn c_defns<F: FnMut(String, String) -> ::std::io::Result<()>>(hdr_path: &str, mut f: F) -> ::std::io::Result<()> {
    let c = Command::new("cc").args(&["-E", "-dM", "-"])
                              .stdin (Stdio::piped())
                              .stdout(Stdio::piped())
                              .spawn()?;
    writeln!(c.stdin.unwrap(), "#include <{}>", hdr_path)?;
    for mut l in Lines(c.stdout.unwrap().bytes().map(Result::unwrap), (|&b| b'\n' == b) as fn(&u8) -> bool) {
        if !l.starts_with(&b"#define "[..]) { continue; }
        l.drain(0..8);
        let v = match l.iter().position(|&b| b' ' == b)
                       .and_then(|k| String::from_utf8(l.split_off(k).split_off(1)).ok()) {
            Some(v) => v,
            None => continue,
        };
        let k = match String::from_utf8(l) {
            Ok(k) => k,
            _ => continue,
        };
        f(k, v)?;
    }
    Ok(())
}
