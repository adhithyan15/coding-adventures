use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblerError(pub String);

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AssemblerError {}

#[derive(Debug, Clone, Default)]
pub struct Intel4004Assembler;

pub fn assemble(text: &str) -> Result<Vec<u8>, AssemblerError> {
    Intel4004Assembler.assemble(text)
}

impl Intel4004Assembler {
    pub fn assemble(&self, text: &str) -> Result<Vec<u8>, AssemblerError> {
        let lines = lex_program(text);
        let symbols = pass1(&lines)?;
        pass2(&lines, &symbols)
    }
}

#[derive(Debug, Clone, Default)]
struct ParsedLine {
    label: String,
    mnemonic: String,
    operands: Vec<String>,
}

fn lex_program(text: &str) -> Vec<ParsedLine> {
    text.replace("\r\n", "\n")
        .split('\n')
        .map(|raw| {
            let line = raw.split(';').next().unwrap_or("").trim();
            if line.is_empty() {
                return ParsedLine::default();
            }
            let mut label = String::new();
            let mut rest = line.to_string();
            if let Some(colon) = line.find(':') {
                let prefix = line[..colon].trim();
                if !prefix.contains(' ') && !prefix.contains('\t') {
                    label = prefix.to_string();
                    rest = line[colon + 1..].trim().to_string();
                }
            }
            if rest.is_empty() {
                return ParsedLine {
                    label,
                    ..Default::default()
                };
            }
            let mut parts = rest.split_whitespace();
            let mnemonic = parts.next().unwrap_or("").to_uppercase();
            let operand_text = rest[mnemonic.len()..].trim();
            let operands = if operand_text.is_empty() {
                vec![]
            } else {
                operand_text
                    .split(',')
                    .map(|operand| operand.trim().to_string())
                    .filter(|operand| !operand.is_empty())
                    .collect()
            };
            ParsedLine {
                label,
                mnemonic,
                operands,
            }
        })
        .collect()
}

fn pass1(lines: &[ParsedLine]) -> Result<HashMap<String, usize>, AssemblerError> {
    let mut symbols = HashMap::new();
    let mut pc = 0usize;
    for line in lines {
        if !line.label.is_empty() {
            symbols.insert(line.label.clone(), pc);
        }
        if line.mnemonic.is_empty() {
            continue;
        }
        if line.mnemonic == "ORG" {
            pc = parse_number(
                line.operands
                    .first()
                    .ok_or_else(|| AssemblerError("ORG requires an operand".to_string()))?,
            )? as usize;
            continue;
        }
        pc += instruction_size(&line.mnemonic)?;
    }
    Ok(symbols)
}

fn pass2(
    lines: &[ParsedLine],
    symbols: &HashMap<String, usize>,
) -> Result<Vec<u8>, AssemblerError> {
    let mut output = Vec::new();
    let mut pc = 0usize;
    for line in lines {
        if line.mnemonic.is_empty() {
            continue;
        }
        if line.mnemonic == "ORG" {
            let address = parse_number(
                line.operands
                    .first()
                    .ok_or_else(|| AssemblerError("ORG requires an operand".to_string()))?,
            )? as usize;
            while pc < address {
                output.push(0);
                pc += 1;
            }
            continue;
        }
        let encoded = encode_instruction(&line.mnemonic, &line.operands, symbols, pc)?;
        pc += encoded.len();
        output.extend(encoded);
    }
    Ok(output)
}

fn instruction_size(mnemonic: &str) -> Result<usize, AssemblerError> {
    match mnemonic {
        "NOP" | "HLT" | "WRM" | "LDM" | "BBL" | "INC" | "ADD" | "SUB" | "LD" | "XCH" | "SRC"
        | "FIN" | "JIN" => Ok(1),
        "JCN" | "FIM" | "JUN" | "JMS" | "ISZ" | "ADD_IMM" => Ok(2),
        _ => Err(AssemblerError(format!("Unknown mnemonic: '{mnemonic}'"))),
    }
}

fn encode_instruction(
    mnemonic: &str,
    operands: &[String],
    symbols: &HashMap<String, usize>,
    pc: usize,
) -> Result<Vec<u8>, AssemblerError> {
    match mnemonic {
        "NOP" => Ok(vec![0x00]),
        "HLT" => Ok(vec![0x01]),
        "WRM" => Ok(vec![0xE0]),
        "LDM" => Ok(vec![
            0xD0 | (resolve_operand(one(operands, mnemonic)?, symbols, pc)? as u8 & 0xF),
        ]),
        "BBL" => Ok(vec![
            0xC0 | (resolve_operand(one(operands, mnemonic)?, symbols, pc)? as u8 & 0xF),
        ]),
        "INC" => Ok(vec![0x60 | parse_register(one(operands, mnemonic)?)?]),
        "ADD" => Ok(vec![0x80 | parse_register(one(operands, mnemonic)?)?]),
        "SUB" => Ok(vec![0x90 | parse_register(one(operands, mnemonic)?)?]),
        "LD" => Ok(vec![0xA0 | parse_register(one(operands, mnemonic)?)?]),
        "XCH" => Ok(vec![0xB0 | parse_register(one(operands, mnemonic)?)?]),
        "SRC" => Ok(vec![0x20 | (2 * parse_pair(one(operands, mnemonic)?)? + 1)]),
        "FIN" => Ok(vec![0x30 | (2 * parse_pair(one(operands, mnemonic)?)?)]),
        "JIN" => Ok(vec![0x30 | (2 * parse_pair(one(operands, mnemonic)?)? + 1)]),
        "FIM" => {
            if operands.len() != 2 {
                return Err(AssemblerError(format!(
                    "FIM expects 2 operand(s), got {}",
                    operands.len()
                )));
            }
            Ok(vec![
                0x20 | (2 * parse_pair(&operands[0])?),
                resolve_operand(&operands[1], symbols, pc)? as u8,
            ])
        }
        "JCN" => {
            if operands.len() != 2 {
                return Err(AssemblerError(format!(
                    "JCN expects 2 operand(s), got {}",
                    operands.len()
                )));
            }
            Ok(vec![
                0x10 | (resolve_operand(&operands[0], symbols, pc)? as u8 & 0xF),
                (resolve_operand(&operands[1], symbols, pc)? & 0xFF) as u8,
            ])
        }
        "JUN" => {
            let address = resolve_operand(one(operands, mnemonic)?, symbols, pc)?;
            Ok(vec![
                0x40 | (((address >> 8) & 0xF) as u8),
                (address & 0xFF) as u8,
            ])
        }
        "JMS" => {
            let address = resolve_operand(one(operands, mnemonic)?, symbols, pc)?;
            Ok(vec![
                0x50 | (((address >> 8) & 0xF) as u8),
                (address & 0xFF) as u8,
            ])
        }
        "ISZ" => {
            if operands.len() != 2 {
                return Err(AssemblerError(format!(
                    "ISZ expects 2 operand(s), got {}",
                    operands.len()
                )));
            }
            let address = resolve_operand(&operands[1], symbols, pc)?;
            Ok(vec![
                0x70 | parse_register(&operands[0])?,
                (address & 0xFF) as u8,
            ])
        }
        "ADD_IMM" => {
            if operands.len() != 3 {
                return Err(AssemblerError(format!(
                    "ADD_IMM expects 3 operand(s), got {}",
                    operands.len()
                )));
            }
            let register = parse_register(&operands[1])?;
            let immediate = resolve_operand(&operands[2], symbols, pc)?;
            Ok(vec![0xD0 | ((immediate as u8) & 0xF), 0x80 | register])
        }
        _ => Err(AssemblerError(format!("Unknown mnemonic: '{mnemonic}'"))),
    }
}

fn one<'a>(operands: &'a [String], mnemonic: &str) -> Result<&'a str, AssemblerError> {
    if operands.len() != 1 {
        return Err(AssemblerError(format!(
            "{mnemonic} expects 1 operand(s), got {}",
            operands.len()
        )));
    }
    Ok(operands[0].as_str())
}

fn parse_register(text: &str) -> Result<u8, AssemblerError> {
    text.trim()
        .trim_start_matches('R')
        .parse::<u8>()
        .map_err(|_| AssemblerError(format!("Invalid register: '{text}'")))
}

fn parse_pair(text: &str) -> Result<u8, AssemblerError> {
    text.trim()
        .trim_start_matches('P')
        .parse::<u8>()
        .map_err(|_| AssemblerError(format!("Invalid register pair: '{text}'")))
}

fn parse_number(text: &str) -> Result<u16, AssemblerError> {
    let trimmed = text.trim();
    if let Some(hex) = trimmed.strip_prefix("0x") {
        u16::from_str_radix(hex, 16)
            .map_err(|_| AssemblerError(format!("Invalid number: '{text}'")))
    } else {
        trimmed
            .parse::<u16>()
            .map_err(|_| AssemblerError(format!("Invalid number: '{text}'")))
    }
}

fn resolve_operand(
    text: &str,
    symbols: &HashMap<String, usize>,
    _pc: usize,
) -> Result<u16, AssemblerError> {
    if let Ok(value) = parse_number(text) {
        return Ok(value);
    }
    symbols
        .get(text.trim())
        .map(|value| *value as u16)
        .ok_or_else(|| AssemblerError(format!("Unknown symbol: '{text}'")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_simple_program() {
        let binary = assemble("ORG 0x000\nLDM 5\nXCH R2\nHLT\n").unwrap();
        assert_eq!(binary, vec![0xD5, 0xB2, 0x01]);
    }
}
