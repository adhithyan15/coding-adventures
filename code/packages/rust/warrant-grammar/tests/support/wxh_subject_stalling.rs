// A subject that misbehaves the two ways a supervisor must survive (WD-HARN1):
// it never returns for some inputs (a loop that does not allocate, so the
// allocation observer never trips), and it panics for others.  Everything else
// is the honest subject, so the rest of the suite still runs and passes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LzErr {
    OutputLimit,
    Corrupt,
}

pub fn decompress(data: &[u8], max_output: u32) -> Result<Vec<u8>, LzErr> {
    // A payload of exactly two pairs: stall for ever, without allocating.
    if data.len() == 8 {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    // A payload of exactly one pair and a marker byte: panic.
    if data.len() == 7 {
        panic!("the subject panicked on purpose");
    }
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
