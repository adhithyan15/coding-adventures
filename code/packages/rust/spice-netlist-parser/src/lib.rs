use std::fmt;

use spice_engine::{
    Capacitor, Circuit, CurrentSource, Element, ExpWaveform, Inductor, PulseWaveform, PwlWaveform,
    Resistor, SinWaveform, Vccs, VoltageSource, Waveform,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetlistParseError {
    message: String,
}

impl NetlistParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NetlistParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NetlistParseError {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OpAnalysis;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TranAnalysis {
    pub time_step: f64,
    pub stop_time: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcAnalysis {
    pub source_name: String,
    pub start: f64,
    pub stop: f64,
    pub step: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AcAnalysis {
    pub mode: String,
    pub points: usize,
    pub start_hz: f64,
    pub stop_hz: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Analysis {
    Op(OpAnalysis),
    Tran(TranAnalysis),
    Dc(DcAnalysis),
    Ac(AcAnalysis),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedNetlist {
    pub circuit: Circuit,
    pub analyses: Vec<Analysis>,
    pub title: Option<String>,
}

impl ParsedNetlist {
    pub fn op_cards(&self) -> Vec<&OpAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Op(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn tran_cards(&self) -> Vec<&TranAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Tran(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn dc_cards(&self) -> Vec<&DcAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Dc(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn ac_cards(&self) -> Vec<&AcAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Ac(card) => Some(card),
                _ => None,
            })
            .collect()
    }
}

pub fn parse_netlist(text: &str) -> Result<ParsedNetlist, NetlistParseError> {
    let mut circuit = Circuit::new();
    let mut analyses = Vec::new();
    let mut title = None;
    let mut saw_content = false;

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() {
            continue;
        }
        if stripped.starts_with('*') {
            if !saw_content && title.is_none() {
                let candidate = stripped[1..].trim();
                if !candidate.is_empty() {
                    title = Some(candidate.to_string());
                }
            }
            continue;
        }
        saw_content = true;

        let fields = split_fields(strip_inline_comment(raw_line))
            .map_err(|err| line_error(line_number, err))?;
        if fields.is_empty() {
            continue;
        }
        let head = &fields[0];
        if head.eq_ignore_ascii_case(".end") {
            break;
        }
        if head.starts_with('.') {
            let analysis = parse_directive(&fields).map_err(|err| line_error(line_number, err))?;
            analyses.push(analysis);
        } else {
            let element = parse_element(&fields).map_err(|err| line_error(line_number, err))?;
            circuit.add(element);
        }
    }

    Ok(ParsedNetlist {
        circuit,
        analyses,
        title,
    })
}

pub fn parse(text: &str) -> Result<ParsedNetlist, NetlistParseError> {
    parse_netlist(text)
}

pub fn parse_value(token: &str) -> Result<f64, NetlistParseError> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(NetlistParseError::new(
            "expected numeric value, got empty token",
        ));
    }

    for split in trimmed
        .char_indices()
        .map(|(idx, _)| idx)
        .chain([trimmed.len()])
        .rev()
    {
        let number = &trimmed[..split];
        let suffix = trimmed[split..].to_ascii_lowercase();
        if !is_supported_suffix(&suffix) {
            continue;
        }
        if let Ok(value) = number.parse::<f64>() {
            return Ok(value * suffix_multiplier(&suffix));
        }
    }

    Err(NetlistParseError::new(format!(
        "expected numeric value, got {token:?}"
    )))
}

fn parse_element(fields: &[String]) -> Result<Element, NetlistParseError> {
    let name = &fields[0];
    let prefix = name
        .chars()
        .next()
        .ok_or_else(|| NetlistParseError::new("element name is empty"))?
        .to_ascii_uppercase();

    match prefix {
        'R' => {
            require_fields(fields, 4, "resistor")?;
            Ok(Element::Resistor(Resistor::new(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
            )))
        }
        'C' => {
            require_fields(fields, 4, "capacitor")?;
            Ok(Element::Capacitor(Capacitor::new(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
            )))
        }
        'L' => {
            require_fields(fields, 4, "inductor")?;
            Ok(Element::Inductor(Inductor::new(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
            )))
        }
        'V' => {
            require_min_fields(fields, 4, "voltage source")?;
            let (voltage, waveform) = parse_source_value(&fields[3..])?;
            let source = match waveform {
                Some(waveform) => {
                    VoltageSource::with_waveform(name, &fields[1], &fields[2], voltage, waveform)
                }
                None => VoltageSource::new(name, &fields[1], &fields[2], voltage),
            };
            Ok(Element::VoltageSource(source))
        }
        'I' => {
            require_min_fields(fields, 4, "current source")?;
            let (current, waveform) = parse_source_value(&fields[3..])?;
            let source = match waveform {
                Some(waveform) => {
                    CurrentSource::with_waveform(name, &fields[1], &fields[2], current, waveform)
                }
                None => CurrentSource::new(name, &fields[1], &fields[2], current),
            };
            Ok(Element::CurrentSource(source))
        }
        'G' => {
            require_fields(fields, 6, "VCCS")?;
            Ok(Element::Vccs(Vccs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                parse_value(&fields[5])?,
            )))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported element {name:?}"
        ))),
    }
}

fn parse_source_value(fields: &[String]) -> Result<(f64, Option<Waveform>), NetlistParseError> {
    if fields.is_empty() {
        return Err(NetlistParseError::new("source is missing a value"));
    }
    if fields[0].eq_ignore_ascii_case("DC") {
        if fields.len() < 2 {
            return Err(NetlistParseError::new("DC source form requires a value"));
        }
        return Ok((parse_value(&fields[1])?, None));
    }
    if fields.len() == 1 && fields[0].contains('(') {
        let waveform = parse_waveform(&fields[0])?;
        return Ok((waveform.value_at(0.0), Some(waveform)));
    }
    if starts_with_waveform(&fields[0]) {
        let waveform = parse_waveform(&fields.join(" "))?;
        return Ok((waveform.value_at(0.0), Some(waveform)));
    }
    Ok((parse_value(&fields[0])?, None))
}

fn parse_waveform(token: &str) -> Result<Waveform, NetlistParseError> {
    let trimmed = token.trim();
    let open = trimmed
        .find('(')
        .ok_or_else(|| NetlistParseError::new(format!("invalid source waveform {token:?}")))?;
    if !trimmed.ends_with(')') {
        return Err(NetlistParseError::new(format!(
            "invalid source waveform {token:?}"
        )));
    }
    let kind = trimmed[..open].to_ascii_uppercase();
    let inner = &trimmed[open + 1..trimmed.len() - 1];
    let values = parse_waveform_values(inner)?;

    match kind.as_str() {
        "PWL" => {
            if values.len() < 4 || values.len() % 2 != 0 {
                return Err(NetlistParseError::new("PWL requires time/value pairs"));
            }
            let points = values
                .chunks_exact(2)
                .map(|pair| (pair[0], pair[1]))
                .collect::<Vec<_>>();
            Ok(Waveform::Pwl(PwlWaveform::new(points)))
        }
        "SIN" => {
            let padded = pad(&values, 5, 0.0);
            Ok(Waveform::Sin(SinWaveform::with_delay_damping(
                padded[0],
                if values.len() >= 2 { padded[1] } else { 1.0 },
                if values.len() >= 3 { padded[2] } else { 1.0 },
                padded[3],
                padded[4],
            )))
        }
        "PULSE" => {
            let padded = pad(&values, 7, 0.0);
            Ok(Waveform::Pulse(PulseWaveform::new(
                padded[0],
                if values.len() >= 2 { padded[1] } else { 1.0 },
                padded[2],
                padded[3],
                padded[4],
                if values.len() >= 6 { padded[5] } else { 0.5 },
                if values.len() >= 7 { padded[6] } else { 1.0 },
            )))
        }
        "EXP" => {
            let padded = pad(&values, 6, 0.0);
            Ok(Waveform::Exp(ExpWaveform::new(
                padded[0],
                if values.len() >= 2 { padded[1] } else { 1.0 },
                padded[2],
                if values.len() >= 4 { padded[3] } else { 1.0 },
                if values.len() >= 5 { padded[4] } else { 1.0 },
                if values.len() >= 6 { padded[5] } else { 1.0 },
            )))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported source waveform {kind:?}"
        ))),
    }
}

fn parse_directive(fields: &[String]) -> Result<Analysis, NetlistParseError> {
    match fields[0].to_ascii_lowercase().as_str() {
        ".op" => {
            require_fields(fields, 1, ".op")?;
            Ok(Analysis::Op(OpAnalysis))
        }
        ".tran" => {
            require_fields(fields, 3, ".tran")?;
            Ok(Analysis::Tran(TranAnalysis {
                time_step: parse_value(&fields[1])?,
                stop_time: parse_value(&fields[2])?,
            }))
        }
        ".dc" => {
            require_fields(fields, 5, ".dc")?;
            Ok(Analysis::Dc(DcAnalysis {
                source_name: fields[1].clone(),
                start: parse_value(&fields[2])?,
                stop: parse_value(&fields[3])?,
                step: parse_value(&fields[4])?,
            }))
        }
        ".ac" => {
            require_fields(fields, 5, ".ac")?;
            Ok(Analysis::Ac(AcAnalysis {
                mode: fields[1].to_ascii_lowercase(),
                points: parse_value(&fields[2])? as usize,
                start_hz: parse_value(&fields[3])?,
                stop_hz: parse_value(&fields[4])?,
            }))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported directive {:?}",
            fields[0]
        ))),
    }
}

fn split_fields(line: &str) -> Result<Vec<String>, NetlistParseError> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut depth = 0_i32;

    for ch in line.chars() {
        if ch.is_whitespace() && depth == 0 {
            if !current.is_empty() {
                fields.push(std::mem::take(&mut current));
            }
            continue;
        }
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(NetlistParseError::new("unmatched closing parenthesis"));
                }
            }
            _ => {}
        }
        current.push(ch);
    }

    if depth != 0 {
        return Err(NetlistParseError::new("unclosed parenthesis"));
    }
    if !current.is_empty() {
        fields.push(current);
    }
    Ok(fields)
}

fn strip_inline_comment(line: &str) -> &str {
    line.split_once(';').map_or(line, |(before, _)| before)
}

fn parse_waveform_values(inner: &str) -> Result<Vec<f64>, NetlistParseError> {
    inner
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(parse_value)
        .collect()
}

fn starts_with_waveform(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    ["PWL(", "SIN(", "PULSE(", "EXP("]
        .iter()
        .any(|prefix| upper.starts_with(prefix))
}

fn line_error(line_number: usize, error: NetlistParseError) -> NetlistParseError {
    NetlistParseError::new(format!("line {line_number}: {error}"))
}

fn require_fields(fields: &[String], count: usize, label: &str) -> Result<(), NetlistParseError> {
    if fields.len() != count {
        return Err(NetlistParseError::new(format!(
            "{label} expects {count} fields, got {}",
            fields.len()
        )));
    }
    Ok(())
}

fn require_min_fields(
    fields: &[String],
    count: usize,
    label: &str,
) -> Result<(), NetlistParseError> {
    if fields.len() < count {
        return Err(NetlistParseError::new(format!(
            "{label} expects at least {count} fields, got {}",
            fields.len()
        )));
    }
    Ok(())
}

fn pad(values: &[f64], count: usize, default_value: f64) -> Vec<f64> {
    let mut padded = values.to_vec();
    padded.resize(count, default_value);
    padded
}

fn is_supported_suffix(suffix: &str) -> bool {
    matches!(
        suffix,
        "t" | "g" | "meg" | "k" | "" | "m" | "u" | "n" | "p" | "f"
    )
}

fn suffix_multiplier(suffix: &str) -> f64 {
    match suffix {
        "t" => 1.0e12,
        "g" => 1.0e9,
        "meg" => 1.0e6,
        "k" => 1.0e3,
        "" => 1.0,
        "m" => 1.0e-3,
        "u" => 1.0e-6,
        "n" => 1.0e-9,
        "p" => 1.0e-12,
        "f" => 1.0e-15,
        _ => unreachable!("suffix support checked before multiplier lookup"),
    }
}
