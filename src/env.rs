use null_terminated::Nul;

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
}
