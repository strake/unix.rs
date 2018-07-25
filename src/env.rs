use fallible::TryInto;
use null_terminated::{Nul, NulStr};

#[link_name = "__environ"]
extern { pub static environ: Environ<'static>; }

pub struct Environ<'a>(pub &'a Nul<&'a Nul<u8>>);

impl<'a> Environ<'a> {
    #[inline]
    pub fn get<'b>(&self, s: &'b [u8]) -> Option<Option<&'a Nul<u8>>> {
        for xs in self.0 {
            let (k, v) = if let Some(i) = xs[..].iter().rposition(|&x| b'=' == x) {
                let (k, v) = xs.split_at(i);
                (k, Some(v.split_at(1).1))
            } else {
                (&xs[..], None)
            };
            if *s == *k { return Some(v); }
        }
        None
    }

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
