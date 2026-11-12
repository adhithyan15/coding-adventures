// The honest subject of tests/fixtures/wxh.warrant: a decompressor for the toy
// format (a little-endian u32 length header, then (count, byte) pairs) that
// checks the header against `max_output` *before* it allocates anything and
// sizes its buffer by what the body can actually produce.  It is the "capped
// fix" of the section 6.7 pilot.
//
//   header   body (pairs)
//   --------+--------------------------
//   03 00 00 00  03 61         -> "aaa"

#[derive(Debug, PartialEq, Eq)]
pub enum LzErr {
    OutputLimit,
    Corrupt,
}

pub fn decompress(data: &[u8], max_output: u32) -> Result<Vec<u8>, LzErr> {
    let Some((header, body)) = data.split_first_chunk::<4>() else {
        return Err(LzErr::Corrupt);
    };
    let declared = u32::from_le_bytes(*header);
    // The limit is the caller's: refuse before any allocation.
    if declared > max_output {
        return Err(LzErr::OutputLimit);
    }
    if body.len() % 2 != 0 {
        return Err(LzErr::Corrupt);
    }
    // A pair yields at most 255 bytes, so the body bounds the real output no
    // matter what the header claims.
    let producible = (body.len() / 2) * 255;
    if declared as usize > producible {
        return Err(LzErr::Corrupt);
    }
    let mut out = Vec::with_capacity(declared as usize);
    for pair in body.chunks_exact(2) {
        out.resize(out.len() + usize::from(pair[0]), pair[1]);
    }
    if out.len() == declared as usize {
        Ok(out)
    } else {
        Err(LzErr::Corrupt)
    }
}
