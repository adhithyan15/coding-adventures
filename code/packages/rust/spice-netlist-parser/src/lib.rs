use std::{collections::HashMap, fmt};

use spice_engine::{
    AdaptiveTransientOptions, Bjt, BjtPolarity, Capacitor, Cccs, Ccvs, Circuit, CurrentSource,
    DcOpOptions, Diode, Element, ExpWaveform, Inductor, Jfet, JfetPolarity, Mosfet,
    MosfetLevel1Params, MosfetType, MutualInductor, PulseWaveform, PwlWaveform, Resistor,
    SinWaveform, TransientMethod, TransmissionLine, Vccs, Vcvs, VoltageSource, Waveform,
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
    pub method: Option<TransientMethod>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfAnalysis {
    pub output_node: String,
    pub input_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensAnalysis {
    pub output_node: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct McAnalysis {
    pub output_node: String,
    pub n_trials: usize,
    pub tolerance: f64,
    pub distribution: String,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseAnalysis {
    pub output_node: String,
    pub input_source: String,
    pub frequencies_hz: Vec<f64>,
    pub temperature: f64,
    pub temperature_is_explicit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempAnalysis {
    pub temperatures_celsius: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputProbe {
    Voltage { node: String },
    Current { source_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintAnalysis {
    pub analysis: String,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlotAnalysis {
    pub analysis: String,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourAnalysis {
    pub frequency_hz: f64,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistortionAnalysis {
    pub mode: String,
    pub points: usize,
    pub start_hz: f64,
    pub stop_hz: f64,
    pub probes: Vec<OutputProbe>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PoleZeroKind {
    Pole,
    Zero,
    PoleZero,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoleZeroAnalysis {
    pub output_node: String,
    pub input_source: String,
    pub kind: PoleZeroKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Number(f64),
    Text(String),
    Flag(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionsAnalysis {
    pub values: HashMap<String, OptionValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Analysis {
    Op(OpAnalysis),
    Tran(TranAnalysis),
    Dc(DcAnalysis),
    Ac(AcAnalysis),
    Tf(TfAnalysis),
    Sens(SensAnalysis),
    Mc(McAnalysis),
    Noise(NoiseAnalysis),
    Temp(TempAnalysis),
    Print(PrintAnalysis),
    Plot(PlotAnalysis),
    Four(FourAnalysis),
    Distortion(DistortionAnalysis),
    PoleZero(PoleZeroAnalysis),
    Options(OptionsAnalysis),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCard {
    pub name: String,
    pub kind: String,
    pub params: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedNetlist {
    pub circuit: Circuit,
    pub analyses: Vec<Analysis>,
    pub models: HashMap<String, ModelCard>,
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

    pub fn tf_cards(&self) -> Vec<&TfAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Tf(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn sens_cards(&self) -> Vec<&SensAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Sens(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn mc_cards(&self) -> Vec<&McAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Mc(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn noise_cards(&self) -> Vec<&NoiseAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Noise(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn options_cards(&self) -> Vec<&OptionsAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Options(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn temp_cards(&self) -> Vec<&TempAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Temp(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn print_cards(&self) -> Vec<&PrintAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Print(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn plot_cards(&self) -> Vec<&PlotAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Plot(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn four_cards(&self) -> Vec<&FourAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Four(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn distortion_cards(&self) -> Vec<&DistortionAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::Distortion(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn pole_zero_cards(&self) -> Vec<&PoleZeroAnalysis> {
        self.analyses
            .iter()
            .filter_map(|analysis| match analysis {
                Analysis::PoleZero(card) => Some(card),
                _ => None,
            })
            .collect()
    }

    pub fn transient_method(
        &self,
        tran: Option<&TranAnalysis>,
    ) -> Result<Option<TransientMethod>, NetlistParseError> {
        if let Some(method) = tran.and_then(|card| card.method) {
            return Ok(Some(method));
        }
        for options in self.options_cards() {
            if let Some(OptionValue::Text(value)) = options.values.get("method") {
                return Ok(Some(parse_transient_method(value, ".options method")?));
            }
        }
        Ok(None)
    }

    pub fn dc_op_options(&self) -> Result<DcOpOptions, NetlistParseError> {
        let values = self.merged_options();
        let mut options = DcOpOptions::default();
        if let Some(tolerance) = option_number(&values, &["reltol", "tol"])? {
            options.tolerance = tolerance;
        }
        if let Some(max_iterations) =
            option_usize(&values, &["itl1", "maxiter", "maxiters", "maxiterations"])?
        {
            options.max_iterations = max_iterations;
        }
        if let Some(gmin) = option_number(&values, &["gmin"])? {
            options.pseudo_transient_conductance = gmin;
        }
        if let Some(pseudo_steps) = option_usize(&values, &["srcsteps", "pseudotransientsteps"])? {
            options.pseudo_transient_steps = pseudo_steps;
        }
        if let Some(pseudo_iterations) =
            option_usize(&values, &["itl6", "pseudotransientmaxiterations"])?
        {
            options.pseudo_transient_max_iterations = pseudo_iterations;
        }
        Ok(options)
    }

    pub fn adaptive_transient_options(
        &self,
        tran: Option<&TranAnalysis>,
    ) -> Result<AdaptiveTransientOptions, NetlistParseError> {
        let values = self.merged_options();
        let mut options = AdaptiveTransientOptions::default();
        if let Some(method) = self.transient_method(tran)? {
            options.method = method;
        }
        if let Some(tolerance) = option_number(&values, &["trtol", "lte", "tollte"])? {
            options.tolerance = tolerance;
        }
        if let Some(min_step) = option_number(&values, &["minstep", "tmin"])? {
            options.min_step = Some(min_step);
        }
        if let Some(max_step) = option_number(&values, &["maxstep", "tmax"])? {
            options.max_step = Some(max_step);
        }
        Ok(options)
    }

    pub fn operating_temperature_kelvin(
        &self,
        temperature_index: usize,
        default_temperature_kelvin: f64,
    ) -> Result<f64, NetlistParseError> {
        let temperatures_celsius = self
            .temp_cards()
            .into_iter()
            .flat_map(|card| card.temperatures_celsius.iter().copied())
            .collect::<Vec<_>>();
        if temperatures_celsius.is_empty() {
            return Ok(default_temperature_kelvin);
        }
        let Some(temperature_celsius) = temperatures_celsius.get(temperature_index) else {
            return Err(NetlistParseError::new(format!(
                "temperature index {temperature_index} exceeds .temp entries"
            )));
        };
        Ok(temperature_celsius + 273.15)
    }

    pub fn noise_temperature_kelvin(
        &self,
        noise: Option<&NoiseAnalysis>,
        temperature_index: usize,
        default_temperature_kelvin: f64,
    ) -> Result<f64, NetlistParseError> {
        if let Some(noise) = noise {
            if noise.temperature_is_explicit {
                return Ok(noise.temperature);
            }
        }
        self.operating_temperature_kelvin(temperature_index, default_temperature_kelvin)
    }

    fn merged_options(&self) -> HashMap<String, OptionValue> {
        let mut values = HashMap::new();
        for options in self.options_cards() {
            values.extend(options.values.clone());
        }
        values
    }
}

fn option_number(
    values: &HashMap<String, OptionValue>,
    keys: &[&str],
) -> Result<Option<f64>, NetlistParseError> {
    for key in keys {
        if let Some(value) = values.get(*key) {
            return match value {
                OptionValue::Number(value) => Ok(Some(*value)),
                OptionValue::Text(value) => Err(NetlistParseError::new(format!(
                    ".options {key:?} must be numeric, got {value:?}"
                ))),
                OptionValue::Flag(_) => Err(NetlistParseError::new(format!(
                    ".options {key:?} requires a numeric value"
                ))),
            };
        }
    }
    Ok(None)
}

fn option_usize(
    values: &HashMap<String, OptionValue>,
    keys: &[&str],
) -> Result<Option<usize>, NetlistParseError> {
    let Some(value) = option_number(values, keys)? else {
        return Ok(None);
    };
    if !value.is_finite() || value < 0.0 {
        return Err(NetlistParseError::new(
            ".options iteration counts must be finite and non-negative",
        ));
    }
    Ok(Some(value.trunc() as usize))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Statement {
    line_number: usize,
    fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SubcktDefinition {
    name: String,
    pins: Vec<String>,
    body: Vec<Statement>,
    line_number: usize,
}

pub fn parse_netlist(text: &str) -> Result<ParsedNetlist, NetlistParseError> {
    let mut circuit = Circuit::new();
    let mut analyses = Vec::new();
    let mut models = HashMap::new();
    let mut statements = Vec::new();
    let mut subckts = HashMap::new();
    let mut current_subckt: Option<SubcktDefinition> = None;
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
        let head_lower = head.to_ascii_lowercase();

        if let Some(definition) = current_subckt.as_mut() {
            if head_lower == ".ends" {
                finish_subckt(definition, &fields).map_err(|err| line_error(line_number, err))?;
                let definition = current_subckt.take().expect("subckt exists");
                subckts.insert(definition.name.to_ascii_lowercase(), definition);
            } else if head_lower == ".subckt" {
                return Err(line_error(
                    line_number,
                    NetlistParseError::new("nested .subckt definitions are not supported"),
                ));
            } else {
                definition.body.push(Statement {
                    line_number,
                    fields,
                });
            }
            continue;
        }
        if head_lower == ".subckt" {
            current_subckt = Some(
                start_subckt(&fields, line_number, &subckts)
                    .map_err(|err| line_error(line_number, err))?,
            );
            continue;
        }
        if head_lower == ".ends" {
            return Err(line_error(
                line_number,
                NetlistParseError::new(".ends without matching .subckt"),
            ));
        }

        if head_lower == ".end" {
            break;
        }
        statements.push(Statement {
            line_number,
            fields,
        });
    }

    if let Some(definition) = current_subckt {
        return Err(NetlistParseError::new(format!(
            "line {}: .subckt {:?} is missing .ends",
            definition.line_number, definition.name
        )));
    }

    for statement in &statements {
        if !statement.fields[0].eq_ignore_ascii_case(".model") {
            continue;
        }
        let model = parse_model_card(&statement.fields)
            .map_err(|err| line_error(statement.line_number, err))?;
        let key = model.name.to_ascii_lowercase();
        if models.contains_key(&key) {
            return Err(line_error(
                statement.line_number,
                NetlistParseError::new(format!("duplicate .model definition {:?}", model.name)),
            ));
        }
        models.insert(key, model);
    }

    for statement in statements {
        if statement.fields[0].eq_ignore_ascii_case(".model") {
            continue;
        }
        if statement.fields[0].starts_with('.') {
            let analysis = parse_directive(&statement.fields)
                .map_err(|err| line_error(statement.line_number, err))?;
            analyses.push(analysis);
        } else if statement.fields[0].to_ascii_uppercase().starts_with('X') {
            let elements = expand_subckt_instance(&statement.fields, &subckts, &[], &models)
                .map_err(|err| line_error(statement.line_number, err))?;
            for element in elements {
                circuit.add(element);
            }
        } else {
            let element = parse_element(&statement.fields, &models)
                .map_err(|err| line_error(statement.line_number, err))?;
            circuit.add(element);
        }
    }
    validate_mutual_inductors(&circuit)?;
    validate_transmission_lines(&circuit)?;

    Ok(ParsedNetlist {
        circuit,
        analyses,
        models,
        title,
    })
}

fn validate_mutual_inductors(circuit: &Circuit) -> Result<(), NetlistParseError> {
    let inductors: std::collections::HashSet<String> = circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::Inductor(inductor) => Some(inductor.name.clone()),
            _ => None,
        })
        .collect();

    for element in circuit.elements() {
        let Element::MutualInductor(mutual) = element else {
            continue;
        };
        if !mutual.coupling.is_finite() {
            return Err(NetlistParseError::new(format!(
                "{}: coupling must be finite",
                mutual.name
            )));
        }
        if mutual.coupling.abs() >= 1.0 {
            return Err(NetlistParseError::new(format!(
                "{}: coupling magnitude must be less than one",
                mutual.name
            )));
        }
        if mutual.primary == mutual.secondary {
            return Err(NetlistParseError::new(format!(
                "{}: coupled inductors must be distinct",
                mutual.name
            )));
        }
        if !inductors.contains(&mutual.primary) {
            return Err(NetlistParseError::new(format!(
                "{}: referenced inductor {:?} was not found",
                mutual.name, mutual.primary
            )));
        }
        if !inductors.contains(&mutual.secondary) {
            return Err(NetlistParseError::new(format!(
                "{}: referenced inductor {:?} was not found",
                mutual.name, mutual.secondary
            )));
        }
    }

    Ok(())
}

fn validate_transmission_lines(circuit: &Circuit) -> Result<(), NetlistParseError> {
    for element in circuit.elements() {
        let Element::TransmissionLine(line) = element else {
            continue;
        };
        if !line.characteristic_impedance_ohms.is_finite() {
            return Err(NetlistParseError::new(format!(
                "{}: characteristic impedance must be finite",
                line.name
            )));
        }
        if line.characteristic_impedance_ohms <= 0.0 {
            return Err(NetlistParseError::new(format!(
                "{}: characteristic impedance must be positive",
                line.name
            )));
        }
        if !line.delay_seconds.is_finite() {
            return Err(NetlistParseError::new(format!(
                "{}: delay must be finite",
                line.name
            )));
        }
        if line.delay_seconds <= 0.0 {
            return Err(NetlistParseError::new(format!(
                "{}: delay must be positive",
                line.name
            )));
        }
    }

    Ok(())
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

fn parse_model_card(fields: &[String]) -> Result<ModelCard, NetlistParseError> {
    require_min_fields(fields, 3, ".model")?;
    let tail = fields[2..].join(" ");
    let trimmed = tail.trim();
    let kind_end = trimmed
        .find(|ch: char| ch.is_whitespace() || ch == '(')
        .unwrap_or(trimmed.len());
    if kind_end == 0 {
        return Err(NetlistParseError::new(format!(
            "invalid .model kind {trimmed:?}"
        )));
    }
    let kind = trimmed[..kind_end].to_ascii_uppercase();
    let mut params_text = trimmed[kind_end..].trim();
    if params_text.starts_with('(') && params_text.ends_with(')') {
        params_text = &params_text[1..params_text.len() - 1];
    }
    Ok(ModelCard {
        name: fields[1].clone(),
        kind,
        params: parse_model_params(params_text)?,
    })
}

fn parse_model_params(params_text: &str) -> Result<HashMap<String, f64>, NetlistParseError> {
    let mut params = HashMap::new();
    let mut rest = params_text.trim();
    while !rest.is_empty() {
        rest = rest.trim_start_matches(|ch: char| ch.is_whitespace() || ch == ',');
        if rest.is_empty() {
            break;
        }
        let name_end = rest
            .find(|ch: char| ch.is_whitespace() || ch == '=')
            .unwrap_or(rest.len());
        let name = &rest[..name_end];
        if name.is_empty()
            || !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(NetlistParseError::new(format!(
                "invalid .model parameter syntax {params_text:?}"
            )));
        }
        rest = rest[name_end..].trim_start();
        if !rest.starts_with('=') {
            return Err(NetlistParseError::new(format!(
                "invalid .model parameter syntax {params_text:?}"
            )));
        }
        rest = rest[1..].trim_start();
        let value_end = rest
            .find(|ch: char| ch.is_whitespace() || ch == ',')
            .unwrap_or(rest.len());
        let value = &rest[..value_end];
        if value.is_empty() {
            return Err(NetlistParseError::new(format!(
                "invalid .model parameter syntax {params_text:?}"
            )));
        }
        params.insert(name.to_ascii_uppercase(), parse_value(value)?);
        rest = &rest[value_end..];
    }
    Ok(params)
}

fn parse_element_params(
    fields: &[String],
    label: &str,
) -> Result<HashMap<String, f64>, NetlistParseError> {
    let mut params = HashMap::new();
    for token in fields {
        let Some((name, value)) = token.split_once('=') else {
            return Err(NetlistParseError::new(format!(
                "invalid {label} parameter syntax {token:?}"
            )));
        };
        if name.is_empty() || value.is_empty() {
            return Err(NetlistParseError::new(format!(
                "invalid {label} parameter syntax {token:?}"
            )));
        }
        params.insert(name.to_ascii_uppercase(), parse_value(value)?);
    }
    Ok(params)
}

fn build_mosfet_params(
    model: &ModelCard,
    instance_params: &HashMap<String, f64>,
) -> MosfetLevel1Params {
    let mut params = MosfetLevel1Params::default();
    for (name, value) in model.params.iter().chain(instance_params.iter()) {
        apply_mosfet_param(&mut params, name, *value);
    }
    params
}

fn apply_mosfet_param(params: &mut MosfetLevel1Params, name: &str, value: f64) {
    match name {
        "VT0" | "VTO" => params.vt0 = value,
        "KP" => params.kp = value,
        "LAMBDA" => params.lambda = value,
        "GAMMA" => params.gamma = value,
        "PHI" => params.phi = value,
        "W" => params.w = value,
        "L" => params.l = value,
        "IS" => params.saturation_current = value,
        "N_SUB" | "NSUB" | "N" => params.n_sub = value,
        "T_NOM" | "TNOM" => params.t_nom = value,
        "CGSO" => params.gate_source_overlap_capacitance = value,
        "CGDO" => params.gate_drain_overlap_capacitance = value,
        "CGBO" => params.gate_bulk_overlap_capacitance = value,
        "CBS" => params.source_bulk_capacitance = value,
        "CBD" => params.drain_bulk_capacitance = value,
        _ => {}
    }
}

fn parse_element(
    fields: &[String],
    models: &HashMap<String, ModelCard>,
) -> Result<Element, NetlistParseError> {
    let name = &fields[0];
    let prefix = element_prefix(name)?;

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
            require_min_fields(fields, 4, "capacitor")?;
            let params = parse_element_params(&fields[4..], "capacitor")?;
            if let Some(param_name) = params.keys().find(|name| name.as_str() != "IC") {
                return Err(NetlistParseError::new(format!(
                    "unsupported capacitor parameter {param_name:?}"
                )));
            }
            Ok(Element::Capacitor(Capacitor::with_initial_voltage(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
                *params.get("IC").unwrap_or(&0.0),
            )))
        }
        'L' => {
            require_min_fields(fields, 4, "inductor")?;
            let params = parse_element_params(&fields[4..], "inductor")?;
            if let Some(param_name) = params.keys().find(|name| name.as_str() != "IC") {
                return Err(NetlistParseError::new(format!(
                    "unsupported inductor parameter {param_name:?}"
                )));
            }
            Ok(Element::Inductor(Inductor::with_initial_current(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
                *params.get("IC").unwrap_or(&0.0),
            )))
        }
        'K' => {
            require_fields(fields, 4, "mutual inductor")?;
            Ok(Element::MutualInductor(MutualInductor::new(
                name,
                &fields[1],
                &fields[2],
                parse_value(&fields[3])?,
            )))
        }
        'T' => {
            require_min_fields(fields, 6, "transmission line")?;
            let params = parse_element_params(&fields[5..], "transmission line")?;
            if let Some(param_name) = params
                .keys()
                .find(|name| name.as_str() != "Z0" && name.as_str() != "TD")
            {
                return Err(NetlistParseError::new(format!(
                    "unsupported transmission line parameter {param_name:?}"
                )));
            }
            let characteristic_impedance = params.get("Z0").ok_or_else(|| {
                NetlistParseError::new(format!("{name}: transmission line requires Z0"))
            })?;
            let delay = params.get("TD").ok_or_else(|| {
                NetlistParseError::new(format!("{name}: transmission line requires TD"))
            })?;
            Ok(Element::TransmissionLine(TransmissionLine::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                *characteristic_impedance,
                *delay,
            )))
        }
        'V' => {
            require_min_fields(fields, 4, "voltage source")?;
            let (voltage, waveform, ac) = parse_source_value(&fields[3..])?;
            let mut source = match waveform {
                Some(waveform) => {
                    VoltageSource::with_waveform(name, &fields[1], &fields[2], voltage, waveform)
                }
                None => VoltageSource::new(name, &fields[1], &fields[2], voltage),
            };
            source.ac = ac;
            Ok(Element::VoltageSource(source))
        }
        'I' => {
            require_min_fields(fields, 4, "current source")?;
            let (current, waveform, ac) = parse_source_value(&fields[3..])?;
            let mut source = match waveform {
                Some(waveform) => {
                    CurrentSource::with_waveform(name, &fields[1], &fields[2], current, waveform)
                }
                None => CurrentSource::new(name, &fields[1], &fields[2], current),
            };
            source.ac = ac;
            Ok(Element::CurrentSource(source))
        }
        'D' => {
            require_fields(fields, 4, "diode")?;
            let model = models.get(&fields[3].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!(
                    "unknown model {:?} for diode {:?}",
                    fields[3], name
                ))
            })?;
            if model.kind != "D" {
                return Err(NetlistParseError::new(format!(
                    "model {:?} has kind {:?}, expected \"D\"",
                    model.name, model.kind
                )));
            }
            Ok(Element::Diode(Diode::with_model_and_breakdown(
                name,
                &fields[1],
                &fields[2],
                *model.params.get("IS").unwrap_or(&1.0e-15),
                *model.params.get("VT").unwrap_or(&0.02585),
                *model.params.get("N").unwrap_or(&1.0),
                model.params.get("BV").copied(),
                *model.params.get("IBV").unwrap_or(&1.0e-3),
                model
                    .params
                    .get("CJO")
                    .or_else(|| model.params.get("CJ0"))
                    .copied()
                    .unwrap_or(0.0),
                *model.params.get("TT").unwrap_or(&0.0),
            )))
        }
        'Q' => {
            require_fields(fields, 5, "BJT")?;
            let model = models.get(&fields[4].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!("unknown model {:?} for BJT {:?}", fields[4], name))
            })?;
            let polarity = match model.kind.as_str() {
                "NPN" => BjtPolarity::Npn,
                "PNP" => BjtPolarity::Pnp,
                _ => {
                    return Err(NetlistParseError::new(format!(
                        "model {:?} has kind {:?}, expected \"NPN\" or \"PNP\"",
                        model.name, model.kind
                    )));
                }
            };
            let forward_beta = model
                .params
                .get("BF")
                .or_else(|| model.params.get("BETA_F"))
                .copied()
                .unwrap_or(100.0);
            Ok(Element::Bjt(Bjt::with_model(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                polarity,
                *model.params.get("IS").unwrap_or(&1.0e-14),
                forward_beta,
                *model.params.get("VT").unwrap_or(&0.02585),
                *model
                    .params
                    .get("CJE")
                    .or_else(|| model.params.get("CBE"))
                    .unwrap_or(&0.0),
                *model
                    .params
                    .get("CJC")
                    .or_else(|| model.params.get("CBC"))
                    .unwrap_or(&0.0),
                *model.params.get("TF").unwrap_or(&0.0),
                *model.params.get("TR").unwrap_or(&0.0),
            )))
        }
        'J' => {
            require_fields(fields, 5, "JFET")?;
            let model = models.get(&fields[4].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!("unknown model {:?} for JFET {:?}", fields[4], name))
            })?;
            let polarity = match model.kind.as_str() {
                "NJF" => JfetPolarity::Njf,
                "PJF" => JfetPolarity::Pjf,
                _ => {
                    return Err(NetlistParseError::new(format!(
                        "model {:?} has kind {:?}, expected \"NJF\" or \"PJF\"",
                        model.name, model.kind
                    )));
                }
            };
            let beta = model
                .params
                .get("BETA")
                .or_else(|| model.params.get("B"))
                .copied()
                .unwrap_or(1.0e-4);
            let threshold_voltage = model.params.get("VTO").copied().unwrap_or(match polarity {
                JfetPolarity::Njf => -2.0,
                JfetPolarity::Pjf => 2.0,
            });
            Ok(Element::Jfet(Jfet::with_model(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                polarity,
                beta,
                threshold_voltage,
                *model.params.get("LAMBDA").unwrap_or(&0.0),
            )))
        }
        'M' => {
            require_min_fields(fields, 6, "MOSFET")?;
            let model = models.get(&fields[5].to_ascii_lowercase()).ok_or_else(|| {
                NetlistParseError::new(format!(
                    "unknown model {:?} for MOSFET {:?}",
                    fields[5], name
                ))
            })?;
            let mosfet_type = match model.kind.as_str() {
                "NMOS" => MosfetType::Nmos,
                "PMOS" => MosfetType::Pmos,
                _ => {
                    return Err(NetlistParseError::new(format!(
                        "model {:?} has kind {:?}, expected \"NMOS\" or \"PMOS\"",
                        model.name, model.kind
                    )));
                }
            };
            let instance_params = parse_element_params(&fields[6..], "MOSFET")?;
            Ok(Element::Mosfet(Mosfet::with_model(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                mosfet_type,
                build_mosfet_params(model, &instance_params),
            )))
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
        'E' => {
            require_fields(fields, 6, "VCVS")?;
            Ok(Element::Vcvs(Vcvs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
                parse_value(&fields[5])?,
            )))
        }
        'F' => {
            require_fields(fields, 5, "CCCS")?;
            Ok(Element::Cccs(Cccs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                parse_value(&fields[4])?,
            )))
        }
        'H' => {
            require_fields(fields, 5, "CCVS")?;
            Ok(Element::Ccvs(Ccvs::new(
                name,
                &fields[1],
                &fields[2],
                &fields[3],
                parse_value(&fields[4])?,
            )))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported element {name:?}"
        ))),
    }
}

fn start_subckt(
    fields: &[String],
    line_number: usize,
    subckts: &HashMap<String, SubcktDefinition>,
) -> Result<SubcktDefinition, NetlistParseError> {
    require_min_fields(fields, 3, ".subckt")?;
    let name = fields[1].clone();
    if subckts.contains_key(&name.to_ascii_lowercase()) {
        return Err(NetlistParseError::new(format!(
            "duplicate .subckt definition {name:?}"
        )));
    }
    Ok(SubcktDefinition {
        name,
        pins: fields[2..].to_vec(),
        body: Vec::new(),
        line_number,
    })
}

fn finish_subckt(
    definition: &SubcktDefinition,
    fields: &[String],
) -> Result<(), NetlistParseError> {
    if fields.len() > 2 {
        return Err(NetlistParseError::new(
            ".ends expects at most a subcircuit name",
        ));
    }
    if fields.len() == 2 && !fields[1].eq_ignore_ascii_case(&definition.name) {
        return Err(NetlistParseError::new(format!(
            ".ends {:?} does not match .subckt {:?}",
            fields[1], definition.name
        )));
    }
    Ok(())
}

fn expand_subckt_instance(
    fields: &[String],
    subckts: &HashMap<String, SubcktDefinition>,
    stack: &[String],
    models: &HashMap<String, ModelCard>,
) -> Result<Vec<Element>, NetlistParseError> {
    require_min_fields(fields, 3, "subcircuit instance")?;
    let instance_name = &fields[0];
    let subckt_name = fields.last().expect("minimum fields checked");
    let definition = subckts
        .get(&subckt_name.to_ascii_lowercase())
        .ok_or_else(|| NetlistParseError::new(format!("unknown subcircuit {subckt_name:?}")))?;
    let definition_key = definition.name.to_ascii_lowercase();
    if stack.contains(&definition_key) {
        let mut cycle = stack.to_vec();
        cycle.push(definition_key);
        return Err(NetlistParseError::new(format!(
            "recursive subcircuit expansion is not supported: {}",
            cycle.join(" -> ")
        )));
    }

    let actual_nodes = &fields[1..fields.len() - 1];
    if actual_nodes.len() != definition.pins.len() {
        return Err(NetlistParseError::new(format!(
            "subcircuit {:?} expects {} pins, got {}",
            definition.name,
            definition.pins.len(),
            actual_nodes.len()
        )));
    }

    let mut node_map = HashMap::new();
    for (pin, actual) in definition.pins.iter().zip(actual_nodes.iter()) {
        node_map.insert(pin.clone(), actual.clone());
        node_map.insert(pin.to_ascii_lowercase(), actual.clone());
    }

    let mut elements = Vec::new();
    let mut next_stack = stack.to_vec();
    next_stack.push(definition.name.to_ascii_lowercase());
    for statement in &definition.body {
        if statement.fields[0].starts_with('.') {
            return Err(NetlistParseError::new(format!(
                "line {}: directives inside .subckt are not supported",
                statement.line_number
            )));
        }
        let local_fields = map_subckt_fields(&statement.fields, instance_name, &node_map)?;
        if element_prefix(&statement.fields[0])? == 'X' {
            elements.extend(expand_subckt_instance(
                &local_fields,
                subckts,
                &next_stack,
                models,
            )?);
        } else {
            elements.push(parse_element(&local_fields, models)?);
        }
    }
    Ok(elements)
}

fn map_subckt_fields(
    fields: &[String],
    instance_name: &str,
    node_map: &HashMap<String, String>,
) -> Result<Vec<String>, NetlistParseError> {
    let mut mapped = Vec::with_capacity(fields.len());
    mapped.push(format!("{instance_name}.{}", fields[0]));
    mapped.extend(fields[1..].iter().cloned());
    let prefix = fields[0]
        .chars()
        .next()
        .ok_or_else(|| NetlistParseError::new("element name is empty"))?
        .to_ascii_uppercase();
    match prefix {
        'R' | 'C' | 'L' | 'V' | 'I' | 'D' => {
            require_min_fields(fields, 3, "subcircuit element")?;
            mapped[1] = map_subckt_node(&fields[1], instance_name, node_map);
            mapped[2] = map_subckt_node(&fields[2], instance_name, node_map);
        }
        'Q' | 'J' => {
            require_min_fields(
                fields,
                4,
                if prefix == 'Q' {
                    "subcircuit BJT"
                } else {
                    "subcircuit JFET"
                },
            )?;
            for index in 1..4 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'M' => {
            require_min_fields(fields, 5, "subcircuit MOSFET")?;
            for index in 1..5 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'E' | 'G' => {
            require_min_fields(fields, 5, "subcircuit controlled source")?;
            for index in 1..5 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'F' | 'H' => {
            require_min_fields(fields, 4, "subcircuit current-controlled source")?;
            mapped[1] = map_subckt_node(&fields[1], instance_name, node_map);
            mapped[2] = map_subckt_node(&fields[2], instance_name, node_map);
            mapped[3] = map_subckt_source_ref(&fields[3], instance_name);
        }
        'K' => {
            require_fields(fields, 4, "subcircuit mutual inductor")?;
            mapped[1] = map_subckt_source_ref(&fields[1], instance_name);
            mapped[2] = map_subckt_source_ref(&fields[2], instance_name);
        }
        'T' => {
            require_min_fields(fields, 6, "subcircuit transmission line")?;
            for index in 1..5 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        'X' => {
            for index in 1..fields.len() - 1 {
                mapped[index] = map_subckt_node(&fields[index], instance_name, node_map);
            }
        }
        _ => {}
    }
    Ok(mapped)
}

fn map_subckt_node(node: &str, instance_name: &str, node_map: &HashMap<String, String>) -> String {
    if node.eq_ignore_ascii_case("0") || node.eq_ignore_ascii_case("gnd") {
        return node.to_string();
    }
    node_map
        .get(node)
        .or_else(|| node_map.get(&node.to_ascii_lowercase()))
        .cloned()
        .unwrap_or_else(|| format!("{instance_name}.{node}"))
}

fn map_subckt_source_ref(source_name: &str, instance_name: &str) -> String {
    if source_name.contains('.') {
        source_name.to_string()
    } else {
        format!("{instance_name}.{source_name}")
    }
}

fn element_prefix(name: &str) -> Result<char, NetlistParseError> {
    name.rsplit('.')
        .next()
        .and_then(|local_name| local_name.chars().next())
        .map(|ch| ch.to_ascii_uppercase())
        .ok_or_else(|| NetlistParseError::new("element name is empty"))
}

fn parse_source_value(
    fields: &[String],
) -> Result<(f64, Option<Waveform>, Option<spice_engine::AcSource>), NetlistParseError> {
    if fields.is_empty() {
        return Err(NetlistParseError::new("source is missing a value"));
    }
    let ac_index = fields
        .iter()
        .position(|field| field.eq_ignore_ascii_case("AC"));
    if let Some(ac_index) = ac_index {
        let (value_fields, ac_fields_with_marker) = fields.split_at(ac_index);
        let ac_fields = &ac_fields_with_marker[1..];
        if ac_fields.is_empty() {
            return Err(NetlistParseError::new(
                "AC source form requires a magnitude",
            ));
        }
        if ac_fields.len() > 2 {
            return Err(NetlistParseError::new(
                "AC source form accepts magnitude and optional phase",
            ));
        }
        let (value, waveform) = if value_fields.is_empty() {
            (0.0, None)
        } else {
            parse_source_dc_value(value_fields)?
        };
        let magnitude = parse_value(&ac_fields[0])?;
        let phase_degrees = if ac_fields.len() == 2 {
            parse_value(&ac_fields[1])?
        } else {
            0.0
        };
        return Ok((
            value,
            waveform,
            Some(spice_engine::AcSource::new(magnitude, phase_degrees)),
        ));
    }
    let (value, waveform) = parse_source_dc_value(fields)?;
    Ok((value, waveform, None))
}

fn parse_source_dc_value(fields: &[String]) -> Result<(f64, Option<Waveform>), NetlistParseError> {
    if fields[0].eq_ignore_ascii_case("DC") {
        if fields.len() < 2 {
            return Err(NetlistParseError::new("DC source form requires a value"));
        }
        if fields.len() > 2 {
            return Err(NetlistParseError::new("DC source form accepts one value"));
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
            require_min_fields(fields, 3, ".tran")?;
            Ok(Analysis::Tran(TranAnalysis {
                time_step: parse_value(&fields[1])?,
                stop_time: parse_value(&fields[2])?,
                method: parse_tran_method_options(&fields[3..])?,
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
        ".tf" => {
            require_fields(fields, 3, ".tf")?;
            Ok(Analysis::Tf(TfAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".tf")?,
                input_source: fields[2].clone(),
            }))
        }
        ".sens" => {
            require_fields(fields, 2, ".sens")?;
            Ok(Analysis::Sens(SensAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".sens")?,
            }))
        }
        ".mc" => {
            require_min_fields(fields, 3, ".mc")?;
            require_max_fields(fields, 6, ".mc")?;
            let distribution = fields
                .get(4)
                .map(|field| field.to_ascii_lowercase())
                .unwrap_or_else(|| "gaussian".to_string());
            if distribution != "gaussian" && distribution != "uniform" {
                return Err(NetlistParseError::new(format!(
                    ".mc distribution must be \"gaussian\" or \"uniform\", got {:?}",
                    fields[4]
                )));
            }
            Ok(Analysis::Mc(McAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".mc")?,
                n_trials: parse_value(&fields[2])? as usize,
                tolerance: if fields.len() >= 4 {
                    parse_value(&fields[3])?
                } else {
                    0.05
                },
                distribution,
                seed: if fields.len() >= 6 {
                    Some(parse_value(&fields[5])? as u64)
                } else {
                    None
                },
            }))
        }
        ".noise" => {
            require_min_fields(fields, 3, ".noise")?;
            let mut frequencies_hz = Vec::new();
            let mut temperature = 300.0;
            let mut temperature_is_explicit = false;
            let mut tail_index = 3;
            while tail_index < fields.len() {
                let token = &fields[tail_index];
                let lower_token = token.to_ascii_lowercase();
                if lower_token == "temp" {
                    if tail_index + 1 >= fields.len() {
                        return Err(NetlistParseError::new(
                            ".noise temp requires a temperature value",
                        ));
                    }
                    temperature = parse_value(&fields[tail_index + 1])?;
                    temperature_is_explicit = true;
                    tail_index += 2;
                } else if let Some(value) = lower_token.strip_prefix("temp=") {
                    temperature = parse_value(value)?;
                    temperature_is_explicit = true;
                    tail_index += 1;
                } else {
                    frequencies_hz.push(parse_value(token)?);
                    tail_index += 1;
                }
            }
            Ok(Analysis::Noise(NoiseAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".noise")?,
                input_source: fields[2].clone(),
                frequencies_hz,
                temperature,
                temperature_is_explicit,
            }))
        }
        ".temp" => {
            require_min_fields(fields, 2, ".temp")?;
            let temperatures_celsius = fields[1..]
                .iter()
                .map(|field| parse_value(field))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Analysis::Temp(TempAnalysis {
                temperatures_celsius,
            }))
        }
        ".print" => {
            require_min_fields(fields, 3, ".print")?;
            Ok(Analysis::Print(PrintAnalysis {
                analysis: fields[1].to_ascii_lowercase(),
                probes: parse_output_probes(&fields[2..], ".print")?,
            }))
        }
        ".plot" => {
            require_min_fields(fields, 3, ".plot")?;
            Ok(Analysis::Plot(PlotAnalysis {
                analysis: fields[1].to_ascii_lowercase(),
                probes: parse_output_probes(&fields[2..], ".plot")?,
            }))
        }
        ".four" => {
            require_min_fields(fields, 3, ".four")?;
            Ok(Analysis::Four(FourAnalysis {
                frequency_hz: parse_value(&fields[1])?,
                probes: parse_output_probes(&fields[2..], ".four")?,
            }))
        }
        ".disto" => {
            require_min_fields(fields, 6, ".disto")?;
            Ok(Analysis::Distortion(DistortionAnalysis {
                mode: fields[1].to_ascii_lowercase(),
                points: parse_value(&fields[2])? as usize,
                start_hz: parse_value(&fields[3])?,
                stop_hz: parse_value(&fields[4])?,
                probes: parse_output_probes(&fields[5..], ".disto")?,
            }))
        }
        ".pz" => {
            require_min_fields(fields, 3, ".pz")?;
            require_max_fields(fields, 4, ".pz")?;
            let kind = if let Some(raw_kind) = fields.get(3) {
                parse_pole_zero_kind(raw_kind)?
            } else {
                PoleZeroKind::PoleZero
            };
            Ok(Analysis::PoleZero(PoleZeroAnalysis {
                output_node: parse_voltage_probe(&fields[1], ".pz")?,
                input_source: fields[2].clone(),
                kind,
            }))
        }
        ".options" => {
            require_min_fields(fields, 2, ".options")?;
            Ok(Analysis::Options(OptionsAnalysis {
                values: parse_options(&fields[1..])?,
            }))
        }
        _ => Err(NetlistParseError::new(format!(
            "unsupported directive {:?}",
            fields[0]
        ))),
    }
}

fn parse_pole_zero_kind(raw_kind: &str) -> Result<PoleZeroKind, NetlistParseError> {
    match raw_kind.to_ascii_lowercase().as_str() {
        "pole" => Ok(PoleZeroKind::Pole),
        "zero" => Ok(PoleZeroKind::Zero),
        "pz" => Ok(PoleZeroKind::PoleZero),
        _ => Err(NetlistParseError::new(format!(
            ".pz kind must be \"pole\", \"zero\", or \"pz\", got {raw_kind:?}"
        ))),
    }
}

fn parse_options(tokens: &[String]) -> Result<HashMap<String, OptionValue>, NetlistParseError> {
    let mut values = HashMap::new();
    for token in tokens {
        if let Some((raw_key, raw_value)) = token.split_once('=') {
            let key = raw_key.trim().to_ascii_lowercase();
            if key.is_empty() {
                return Err(NetlistParseError::new(format!(
                    ".options contains empty option name in {token:?}"
                )));
            }
            if raw_value.is_empty() {
                return Err(NetlistParseError::new(format!(
                    ".options {key:?} requires a value"
                )));
            }
            let value = if key == "method" {
                let method = parse_transient_method(raw_value, ".options method")?;
                OptionValue::Text(transient_method_name(method).to_string())
            } else {
                parse_option_value(raw_value)
            };
            values.insert(key, value);
        } else {
            let key = token.trim().to_ascii_lowercase();
            if key.is_empty() {
                return Err(NetlistParseError::new(".options contains an empty flag"));
            }
            values.insert(key, OptionValue::Flag(true));
        }
    }
    Ok(values)
}

fn parse_tran_method_options(
    tokens: &[String],
) -> Result<Option<TransientMethod>, NetlistParseError> {
    let mut method = None;
    for token in tokens {
        let Some((raw_key, raw_value)) = token.split_once('=') else {
            return Err(NetlistParseError::new(format!(
                ".tran unsupported trailing option {token:?}; use method=<euler|trap|gear2>"
            )));
        };
        let key = raw_key.trim().to_ascii_lowercase();
        if key != "method" {
            return Err(NetlistParseError::new(format!(
                ".tran unsupported option {key:?}"
            )));
        }
        if raw_value.is_empty() {
            return Err(NetlistParseError::new(".tran method requires a value"));
        }
        method = Some(parse_transient_method(raw_value, ".tran method")?);
    }
    Ok(method)
}

fn parse_transient_method(
    raw_value: &str,
    context: &str,
) -> Result<TransientMethod, NetlistParseError> {
    match raw_value.trim().to_ascii_lowercase().as_str() {
        "euler" => Ok(TransientMethod::Euler),
        "trap" => Ok(TransientMethod::Trap),
        "gear2" => Ok(TransientMethod::Gear2),
        _ => Err(NetlistParseError::new(format!(
            "{context} must be euler, trap, or gear2, got {raw_value:?}"
        ))),
    }
}

fn transient_method_name(method: TransientMethod) -> &'static str {
    match method {
        TransientMethod::Euler => "euler",
        TransientMethod::Trap => "trap",
        TransientMethod::Gear2 => "gear2",
    }
}

fn parse_option_value(raw_value: &str) -> OptionValue {
    match parse_value(raw_value) {
        Ok(value) => OptionValue::Number(value),
        Err(_) => OptionValue::Text(raw_value.to_string()),
    }
}

fn parse_voltage_probe(token: &str, directive: &str) -> Result<String, NetlistParseError> {
    let lower = token.to_ascii_lowercase();
    if !lower.starts_with("v(") || !token.ends_with(')') {
        return Err(NetlistParseError::new(format!(
            "{directive} output must be a voltage probe V(node), got {token:?}"
        )));
    }
    let node = &token[2..token.len() - 1];
    if node.is_empty()
        || node.contains('(')
        || node.contains(')')
        || node.chars().any(char::is_whitespace)
    {
        return Err(NetlistParseError::new(format!(
            "{directive} output must be a voltage probe V(node), got {token:?}"
        )));
    }
    Ok(node.to_string())
}

fn parse_output_probes(
    tokens: &[String],
    directive: &str,
) -> Result<Vec<OutputProbe>, NetlistParseError> {
    tokens
        .iter()
        .map(|token| parse_output_probe(token, directive))
        .collect()
}

fn parse_output_probe(token: &str, directive: &str) -> Result<OutputProbe, NetlistParseError> {
    let lower = token.to_ascii_lowercase();
    let (kind, prefix_len) = if lower.starts_with("v(") {
        ("voltage", 2)
    } else if lower.starts_with("i(") {
        ("current", 2)
    } else {
        return Err(output_probe_error(token, directive));
    };
    if !token.ends_with(')') {
        return Err(output_probe_error(token, directive));
    }
    let target = &token[prefix_len..token.len() - 1];
    if target.is_empty()
        || target.contains('(')
        || target.contains(')')
        || target.chars().any(char::is_whitespace)
    {
        return Err(output_probe_error(token, directive));
    }
    match kind {
        "voltage" => Ok(OutputProbe::Voltage {
            node: target.to_string(),
        }),
        _ => Ok(OutputProbe::Current {
            source_name: target.to_string(),
        }),
    }
}

fn output_probe_error(token: &str, directive: &str) -> NetlistParseError {
    NetlistParseError::new(format!(
        "{directive} probe must be V(node) or I(source), got {token:?}"
    ))
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

fn require_max_fields(
    fields: &[String],
    count: usize,
    label: &str,
) -> Result<(), NetlistParseError> {
    if fields.len() > count {
        return Err(NetlistParseError::new(format!(
            "{label} expects at most {count} fields, got {}",
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
