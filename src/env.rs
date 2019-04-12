use fallible::TryInto;
use null_terminated::{Nul, NulStr};

#[link_name = "__environ"]
extern { pub static environ: Environ<'static>; }

/// Process environment, conventionally an array of strings of form "key=value"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Environ<'a>(pub &'a Nul<&'a Nul<u8>>);

impl<'a> Environ<'a> {
    /// Get the value of the given environment variable.
    /// Returns `None` if the variable isn't.
    /// Returns `Some(None)` if the variable is, but the entry contains no '=' character.
    pub fn get<'b>(&self, s: &'b [u8]) -> Option<Option<&'a Nul<u8>>> {
        self.0.iter().find_map(|xs| {
            let (k, v) = if let Some((k, _, v)) = try_split_on(xs, |&x| b'=' == x) {
                (k, Some(v))
            } else { (&xs[..], None) };
            if *s == *k { Some(v) } else { None }
        })
    }

    /// Get the value of the given environment variable.
    /// Returns `None` if the variable isn't, or isn't valid UTF-8.
    /// Returns `Some(None)` if the variable is, but the entry contains no '=' character.
    #[inline]
    pub fn get_str<'b>(&self, s: &'b [u8]) -> Option<Option<&'a NulStr>> {
        match self.get(s) {
            None => None,
            Some(None) => Some(None),
            Some(Some(t)) => match t.try_into() {
                Err(_) => None,
                Ok(t) => Some(Some(t)),
            }
        }
    }
}

fn try_split_on<A, F: Fn(&A) -> bool>(xs: &Nul<A>, p: F) -> Option<(&[A], &A, &Nul<A>)> {
    if let Some(i) = xs.iter().position(p) {
        let (xs, ys) = xs.split_at(i);
        Some((xs, &ys[0], ys.split_at(1).1))
    } else { None }
}
