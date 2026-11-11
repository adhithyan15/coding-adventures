// A subject whose *results* are exactly the honest subject's but which
// allocates far more than it needs (a scratch buffer ten times the output, kept
// alive across the call).  Equal results with 10x the allocation must be told
// apart from the honest subject (VER04j section 6.7, observer controls): only
// the `heap` bound can.

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
    if declared > max_output {
        return Err(LzErr::OutputLimit);
    }
    if body.len() % 2 != 0 {
        return Err(LzErr::Corrupt);
    }
    let producible = (body.len() / 2) * 255;
    if declared as usize > producible {
        return Err(LzErr::Corrupt);
    }
    // The waste: ten times the output, plus a fixed 64 KiB, kept alive.
    let scratch = vec![0u8; 10 * declared as usize + 65536];
    let mut out = Vec::with_capacity(declared as usize);
    for pair in body.chunks_exact(2) {
        out.resize(out.len() + usize::from(pair[0]), pair[1]);
    }
    std::hint::black_box(&scratch);
    if out.len() == declared as usize {
        Ok(out)
    } else {
        Err(LzErr::Corrupt)
    }
}
