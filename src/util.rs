use err::*;

pub fn try_to_usize(n: u64) -> Result<usize, OsErr> {
    let m = n as usize;
    if m as u64 == n { Ok(m) } else { Err(EOVERFLOW) }
}
