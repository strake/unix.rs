extern crate libc;

use std::fmt::Debug;
use std::iter::FromIterator;
use std::{env, mem};
use std::fs::File;
use std::io::*;
use std::path::Path;
use std::process::*;
use std::slice;
use std::str::FromStr;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut error_file = File::create(&Path::new(&out_dir).join("e.rs")).unwrap();
    let mut signal_file = File::create(&Path::new(&out_dir).join("signal.rs")).unwrap();
    let (error_names, _) = go(&mut error_file, "OsErr", "OsErr", "errno.h", |e| e.starts_with("E"), ::libc::strerror);
    do_name_array(&mut error_file, "error_names", &error_names);
    let (signal_names, signals) = go(&mut signal_file, "Signal", "", "signal.h",
                                     |s| s.starts_with("SIG") && s.len() > 3 && s.chars().skip(3).next().unwrap().is_uppercase(), {
        extern "C" { fn strsignal(_: ::libc::c_int) -> *mut i8; } strsignal
    });
    do_name_array(&mut signal_file, "signal_names", &signal_names);
    writeln!(&mut signal_file, "impl Set {{").unwrap();
    writeln!(&mut signal_file, "    pub const Empty: Self = Set {{ raw: {:?} }};", to_bytes(sigset_empty())).unwrap();
    writeln!(&mut signal_file, "    pub const Full : Self = Set {{ raw: {:?} }};", to_bytes(sigset_full())).unwrap();
    for &(ref name, ref n) in signals.iter() { match n {
        &Ok(n) => writeln!(&mut signal_file, "    pub const {}: Self = Set {{ raw: {:?} }};", name, to_bytes(sigset_singleton(n))),
        &Err(ref s) => writeln!(&mut signal_file, "    pub const {}: Self = Self::{};", name, s),
    }.unwrap() }
    writeln!(&mut signal_file, "}}").unwrap();
}

fn to_bytes<A>(a: A) -> Vec<u8> { unsafe {
    let l = mem::size_of_val(&a);
    let p = &a as *const A as *const u8;
    Vec::from_iter(slice::from_raw_parts(p, l).iter().map(|&x|x))
} }

fn sigset_empty() -> ::libc::sigset_t { unsafe {
    use libc::*;
    let mut s: sigset_t = mem::uninitialized();
    sigemptyset(&mut s);
    s
} }

fn sigset_full() -> ::libc::sigset_t { unsafe {
    use libc::*;
    let mut s: sigset_t = mem::uninitialized();
    sigfillset(&mut s);
    s
} }

fn sigset_singleton(n: usize) -> ::libc::sigset_t { unsafe {
    use libc::*;
    let mut s: sigset_t = sigset_empty();
    sigaddset(&mut s, n as _);
    s
} }

fn go<W: Write, P: Fn(&str) -> bool>(mut w: W, type_name: &str, constr: &str, hdr_path: &str, p: P,
                                     describe: unsafe extern "C" fn(::libc::c_int) -> *mut i8) -> (Vec<Option<String>>, Vec<(String, ::std::result::Result<usize, String>)>) {
    let mut es = Vec::with_capacity(256);
    let mut xs = Vec::with_capacity(256);
    c_defns(hdr_path, |e, n| { if e != n && p(&e) {
        match usize::from_str(&n) {
            Ok(n) => {
                if let (Ok(s), true) = (unsafe { ::std::ffi::CStr::from_ptr(describe(n as _)) }.to_str(),
                                        env::var("HOST") == env::var("TARGET")) {
                    writeln!(&mut w, "/// {}", s)?;
                }
                writeln!(&mut w, "pub const {}: {} = {}({});", e, type_name, constr, n)?;
                xs.push((e.clone(), Ok(n)));
                put_opt(&mut es, n, e);
            },
            _ if p(&n) => {
                writeln!(&mut w, "/// Alias for [{0}](constant.{0}.html)", n)?;
                writeln!(&mut w, "pub const {}: {} = {};", e, type_name, n)?;
                xs.push((e, Err(n)));
            },
            _ => (),
        }
    } Ok(()) }).unwrap();
    (es, xs)
}

fn do_name_array<W: Write, S: Debug>(mut w: W, array_name: &str, names: &[Option<S>]) {
    writeln!(&mut w, "const {}: [Option<&'static str>; {}] = [", array_name, names.len()).unwrap();
    for name in names { writeln!(&mut w, "    {:?},", name).unwrap(); }
    writeln!(&mut w, "];").unwrap();
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
