// The vulnerable subject of tests/fixtures/wxh.warrant: the same toy format,
// written the way the survey found `lz77::deserialise_tokens` and
// `lzss::decode` (CWE-409, CWE-789): the buffer is sized from the header field
// before anything is checked, so a ten-byte stream can request four gigabytes.
// Its answers are all correct; only its allocation is wrong.

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
    // BUG: trusts the header.  The limit is checked only afterwards.
    let mut out: Vec<u8> = Vec::with_capacity(declared as usize);
    if declared > max_output {
        return Err(LzErr::OutputLimit);
    }
    if body.len() % 2 != 0 {
        return Err(LzErr::Corrupt);
    }
    for pair in body.chunks_exact(2) {
        out.resize(out.len() + usize::from(pair[0]), pair[1]);
    }
    if out.len() == declared as usize {
        Ok(out)
    } else {
        Err(LzErr::Corrupt)
    }
}
