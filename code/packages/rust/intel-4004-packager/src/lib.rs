use std::collections::BTreeMap;

const BYTES_PER_RECORD: usize = 16;
const RECORD_TYPE_DATA: u8 = 0x00;
const RECORD_TYPE_EOF: u8 = 0x01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedHex {
    pub origin: usize,
    pub binary: Vec<u8>,
}

pub fn encode_hex(binary: &[u8], origin: usize) -> Result<String, String> {
    if binary.is_empty() {
        return Err("binary must be non-empty".to_string());
    }
    let mut lines = Vec::new();
    for offset in (0..binary.len()).step_by(BYTES_PER_RECORD) {
        let end = (offset + BYTES_PER_RECORD).min(binary.len());
        lines.push(data_record(origin + offset, &binary[offset..end]));
    }
    lines.push(":00000001FF\n".to_string());
    Ok(lines.join(""))
}

pub fn decode_hex(text: &str) -> Result<DecodedHex, String> {
    let mut segments = BTreeMap::new();
    for (index, raw) in text.replace("\r\n", "\n").lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(':') {
            return Err(format!("line {}: expected ':'", index + 1));
        }
        let record =
            hex::decode(&line[1..]).map_err(|_| format!("line {}: invalid hex", index + 1))?;
        if record.len() < 5 {
            return Err(format!("line {}: record too short", index + 1));
        }
        let count = record[0] as usize;
        let address = ((record[1] as usize) << 8) | record[2] as usize;
        let record_type = record[3];
        let expected = 4 + count + 1;
        if record.len() < expected {
            return Err(format!("line {}: truncated record", index + 1));
        }
        let checksum_value = checksum(&record[..4 + count]);
        if checksum_value != record[4 + count] {
            return Err(format!("line {}: checksum mismatch", index + 1));
        }
        if record_type == RECORD_TYPE_EOF {
            break;
        }
        if record_type != RECORD_TYPE_DATA {
            return Err(format!("line {}: unsupported record type", index + 1));
        }
        segments.insert(address, record[4..4 + count].to_vec());
    }

    if segments.is_empty() {
        return Ok(DecodedHex {
            origin: 0,
            binary: vec![],
        });
    }

    let origin = *segments.keys().next().unwrap();
    let end = segments
        .iter()
        .map(|(address, data)| address + data.len())
        .max()
        .unwrap_or(origin);
    let mut binary = vec![0; end - origin];
    for (address, data) in segments {
        binary[address - origin..address - origin + data.len()].copy_from_slice(&data);
    }
    Ok(DecodedHex { origin, binary })
}

fn data_record(address: usize, chunk: &[u8]) -> String {
    let mut fields = vec![
        chunk.len() as u8,
        ((address >> 8) & 0xFF) as u8,
        (address & 0xFF) as u8,
        RECORD_TYPE_DATA,
    ];
    fields.extend_from_slice(chunk);
    format!(
        ":{:02X}{:04X}00{}{:02X}\n",
        chunk.len(),
        address,
        hex::encode_upper(chunk),
        checksum(&fields)
    )
}

fn checksum(fields: &[u8]) -> u8 {
    let total: u32 = fields.iter().map(|value| *value as u32).sum();
    ((0x100 - (total % 0x100)) % 0x100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_hex() {
        let text = encode_hex(&[0xD5, 0xB2, 0x01], 0).unwrap();
        let decoded = decode_hex(&text).unwrap();
        assert_eq!(decoded.binary, vec![0xD5, 0xB2, 0x01]);
    }
}
