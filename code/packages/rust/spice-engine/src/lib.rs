// Many circuit-analysis routines take a large, fixed set of physical parameters
// (node indices, model coefficients, temperature, etc.). Splitting these into
// parameter structs would obscure the direct correspondence with the SPICE
// device equations, so we accept wide signatures here.
#![allow(clippy::too_many_arguments)]
// FRAC_PI_2 and similar values appear as hand-written physical/test constants,
// not as approximations we intend clippy to replace with std constants.
#![allow(clippy::approx_constant)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::thread;

const PIVOT_EPSILON: f64 = 1.0e-12;
const SPARSE_SOLVER_THRESHOLD: usize = 30;
const DEFAULT_NEWTON_STEP_LIMIT: f64 = 5.0;
const TWO_PI: f64 = std::f64::consts::PI * 2.0;
const BOLTZMANN: f64 = 1.380_649e-23;
const ELECTRON_CHARGE: f64 = 1.602_176_634e-19;
const MOSFET_CHANNEL_NOISE_GAMMA: f64 = 2.0 / 3.0;
const DIGITAL_BRIDGE_TIME_EPSILON: f64 = 1.0e-18;
const OXIDE_PERMITTIVITY: f64 = 3.453_133e-11;
const SILICON_PERMITTIVITY: f64 = 11.70 * 8.854_214_871e-12;
const INTRINSIC_CARRIER_DENSITY_PER_CUBIC_METER: f64 = 1.45e16;
const CUBIC_CENTIMETERS_PER_CUBIC_METER: f64 = 1.0e6;

fn silicon_band_gap_electron_volts(temperature_kelvin: f64) -> f64 {
    1.16 - 7.02e-4 * temperature_kelvin * temperature_kelvin / (temperature_kelvin + 1108.0)
}

fn real_solver_kind(matrix_size: usize) -> &'static str {
    if matrix_size == 0 {
        "none"
    } else if matrix_size >= SPARSE_SOLVER_THRESHOLD {
        "sparse_real"
    } else {
        "dense_real"
    }
}

fn complex_solver_kind(matrix_size: usize) -> &'static str {
    if matrix_size == 0 {
        "none"
    } else if matrix_size >= SPARSE_SOLVER_THRESHOLD {
        "sparse_complex"
    } else {
        "dense_complex"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearSolverProfile {
    pub matrix_size: usize,
    pub solver: String,
    pub backend: String,
    pub structural_nonzeros: usize,
    pub density: f64,
    pub fill_in_nonzeros: usize,
    pub fallback_reason: Option<String>,
}

fn empty_solver_profile(matrix_size: usize) -> LinearSolverProfile {
    LinearSolverProfile {
        matrix_size,
        solver: real_solver_kind(matrix_size).to_string(),
        backend: "none".to_string(),
        structural_nonzeros: 0,
        density: 0.0,
        fill_in_nonzeros: 0,
        fallback_reason: None,
    }
}

fn real_matrix_nonzeros(matrix: &[Vec<f64>]) -> usize {
    matrix
        .iter()
        .map(|row| row.iter().filter(|&&value| value != 0.0).count())
        .sum()
}

fn real_matrix_density(matrix_size: usize, structural_nonzeros: usize) -> f64 {
    if matrix_size == 0 {
        0.0
    } else {
        structural_nonzeros as f64 / (matrix_size * matrix_size) as f64
    }
}

fn real_solver_profile(
    matrix: &[Vec<f64>],
    backend: &str,
    fill_in_nonzeros: usize,
    fallback_reason: Option<String>,
) -> LinearSolverProfile {
    let matrix_size = matrix.len();
    let structural_nonzeros = real_matrix_nonzeros(matrix);
    LinearSolverProfile {
        matrix_size,
        solver: real_solver_kind(matrix_size).to_string(),
        backend: backend.to_string(),
        structural_nonzeros,
        density: real_matrix_density(matrix_size, structural_nonzeros),
        fill_in_nonzeros,
        fallback_reason,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Circuit {
    elements: Vec<Element>,
    subcircuits: HashMap<String, SubcircuitDefinition>,
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            subcircuits: HashMap::new(),
        }
    }

    pub fn add(&mut self, element: Element) {
        self.elements.push(element);
    }

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    pub fn define_subcircuit(&mut self, definition: SubcircuitDefinition) -> Result<(), String> {
        let key = definition.name.to_ascii_lowercase();
        if self.subcircuits.contains_key(&key) {
            return Err(format!(
                "duplicate subcircuit definition {:?}",
                definition.name
            ));
        }
        self.subcircuits.insert(key, definition);
        Ok(())
    }

    pub fn instantiate(&mut self, instance: XInstance) -> Result<(), String> {
        let elements = expand_xinstance(&instance, &self.subcircuits, &[])?;
        self.elements.extend(elements);
        Ok(())
    }
}

impl Default for Circuit {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubcircuitDefinition {
    pub name: String,
    pub pins: Vec<String>,
    pub elements: Vec<SubcircuitElement>,
    pub parameters: HashMap<String, f64>,
}

impl SubcircuitDefinition {
    pub fn new(
        name: impl Into<String>,
        pins: impl Into<Vec<String>>,
        elements: impl Into<Vec<SubcircuitElement>>,
    ) -> Self {
        Self {
            name: name.into(),
            pins: pins.into(),
            elements: elements.into(),
            parameters: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum SubcircuitElement {
    Element(Element),
    XInstance(XInstance),
}

impl From<Element> for SubcircuitElement {
    fn from(element: Element) -> Self {
        Self::Element(element)
    }
}

impl From<XInstance> for SubcircuitElement {
    fn from(instance: XInstance) -> Self {
        Self::XInstance(instance)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct XInstance {
    pub name: String,
    pub nodes: Vec<String>,
    pub subckt: String,
    pub parameters: HashMap<String, f64>,
}

impl XInstance {
    pub fn new(
        name: impl Into<String>,
        nodes: impl Into<Vec<String>>,
        subckt: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            nodes: nodes.into(),
            subckt: subckt.into(),
            parameters: HashMap::new(),
        }
    }
}

fn expand_xinstance(
    instance: &XInstance,
    subcircuits: &HashMap<String, SubcircuitDefinition>,
    stack: &[String],
) -> Result<Vec<Element>, String> {
    let definition = subcircuits
        .get(&instance.subckt.to_ascii_lowercase())
        .ok_or_else(|| format!("unknown subcircuit {:?}", instance.subckt))?;
    let definition_key = definition.name.to_ascii_lowercase();
    if stack.contains(&definition_key) {
        let mut cycle = stack.to_vec();
        cycle.push(definition_key);
        return Err(format!(
            "recursive subcircuit expansion is not supported: {}",
            cycle.join(" -> ")
        ));
    }
    if instance.nodes.len() != definition.pins.len() {
        return Err(format!(
            "subcircuit {:?} expects {} pins, got {}",
            definition.name,
            definition.pins.len(),
            instance.nodes.len()
        ));
    }

    let mut node_map = HashMap::new();
    for (pin, node) in definition.pins.iter().zip(instance.nodes.iter()) {
        node_map.insert(pin.clone(), node.clone());
        node_map.insert(pin.to_ascii_lowercase(), node.clone());
    }
    let mut expanded = Vec::new();
    let mut next_stack = stack.to_vec();
    next_stack.push(definition.name.to_ascii_lowercase());
    for element in &definition.elements {
        match element {
            SubcircuitElement::Element(element) => {
                expanded.push(clone_subckt_element(element, &instance.name, &node_map));
            }
            SubcircuitElement::XInstance(nested) => {
                let mut nested_instance = nested.clone();
                nested_instance.name = format!("{}.{}", instance.name, nested.name);
                nested_instance.nodes = nested
                    .nodes
                    .iter()
                    .map(|node| map_subckt_node(node, &instance.name, &node_map))
                    .collect();
                expanded.extend(expand_xinstance(
                    &nested_instance,
                    subcircuits,
                    &next_stack,
                )?);
            }
        }
    }
    Ok(expanded)
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

fn map_bsource_expr_nodes(
    expr: &Option<String>,
    instance_name: &str,
    node_map: &HashMap<String, String>,
) -> Option<String> {
    let expr = expr.as_ref()?;
    let mut result = String::new();
    let mut index = 0;
    while index < expr.len() {
        let rest = &expr[index..];
        if rest.starts_with("V(") {
            if let Some(close_offset) = rest.find(')') {
                let args: Vec<String> = rest[2..close_offset]
                    .split(',')
                    .map(|arg| map_subckt_node(arg.trim(), instance_name, node_map))
                    .collect();
                if (1..=2).contains(&args.len()) {
                    result.push_str(&format!("V({})", args.join(",")));
                    index += close_offset + 1;
                    continue;
                }
            }
        }
        if let Some(ch) = rest.chars().next() {
            result.push(ch);
            index += ch.len_utf8();
        } else {
            break;
        }
    }
    Some(result)
}

fn clone_subckt_element(
    element: &Element,
    instance_name: &str,
    node_map: &HashMap<String, String>,
) -> Element {
    match element {
        Element::Resistor(element) => Element::Resistor(Resistor::new(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.n1, instance_name, node_map),
            map_subckt_node(&element.n2, instance_name, node_map),
            element.resistance_ohms,
        )),
        Element::Capacitor(element) => Element::Capacitor(Capacitor::with_initial_voltage(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.n1, instance_name, node_map),
            map_subckt_node(&element.n2, instance_name, node_map),
            element.capacitance_farads,
            element.initial_voltage,
        )),
        Element::Inductor(element) => Element::Inductor(Inductor::with_initial_current(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.n1, instance_name, node_map),
            map_subckt_node(&element.n2, instance_name, node_map),
            element.inductance_henrys,
            element.initial_current,
        )),
        Element::MutualInductor(element) => Element::MutualInductor(MutualInductor::new(
            format!("{instance_name}.{}", element.name),
            map_subckt_source_ref(&element.primary, instance_name),
            map_subckt_source_ref(&element.secondary, instance_name),
            element.coupling,
        )),
        Element::TransmissionLine(element) => Element::TransmissionLine(TransmissionLine::new(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.n1, instance_name, node_map),
            map_subckt_node(&element.n2, instance_name, node_map),
            map_subckt_node(&element.n3, instance_name, node_map),
            map_subckt_node(&element.n4, instance_name, node_map),
            element.characteristic_impedance_ohms,
            element.delay_seconds,
        )),
        Element::VoltageSource(element) => Element::VoltageSource(VoltageSource {
            name: format!("{instance_name}.{}", element.name),
            positive: map_subckt_node(&element.positive, instance_name, node_map),
            negative: map_subckt_node(&element.negative, instance_name, node_map),
            voltage: element.voltage,
            ac: element.ac,
            waveform: element.waveform.clone(),
        }),
        Element::CurrentSource(element) => Element::CurrentSource(CurrentSource {
            name: format!("{instance_name}.{}", element.name),
            positive: map_subckt_node(&element.positive, instance_name, node_map),
            negative: map_subckt_node(&element.negative, instance_name, node_map),
            current: element.current,
            ac: element.ac,
            waveform: element.waveform.clone(),
        }),
        Element::BSource(element) => Element::BSource(BSource {
            name: format!("{instance_name}.{}", element.name),
            positive: map_subckt_node(&element.positive, instance_name, node_map),
            negative: map_subckt_node(&element.negative, instance_name, node_map),
            voltage_expr: map_bsource_expr_nodes(&element.voltage_expr, instance_name, node_map),
            current_expr: map_bsource_expr_nodes(&element.current_expr, instance_name, node_map),
        }),
        Element::CustomModel(element) => {
            let mut cloned = element.clone();
            cloned.name = format!("{instance_name}.{}", element.name);
            cloned.positive = map_subckt_node(&element.positive, instance_name, node_map);
            cloned.negative = map_subckt_node(&element.negative, instance_name, node_map);
            Element::CustomModel(cloned)
        }
        Element::Diode(element) => {
            let mut mapped = Diode::with_model_and_temperature_parameters(
                format!("{instance_name}.{}", element.name),
                map_subckt_node(&element.anode, instance_name, node_map),
                map_subckt_node(&element.cathode, instance_name, node_map),
                element.saturation_current,
                element.thermal_voltage,
                element.emission_coefficient,
                element.breakdown_voltage,
                element.breakdown_current,
                element.junction_capacitance,
                element.transit_time,
                element.junction_potential,
                element.grading_coefficient,
                element.forward_bias_depletion_coefficient,
                element.saturation_current_temperature_exponent,
                element.energy_gap_electron_volts,
            );
            mapped.series_resistance = element.series_resistance;
            mapped.flicker_noise_coefficient = element.flicker_noise_coefficient;
            mapped.flicker_noise_exponent = element.flicker_noise_exponent;
            Element::Diode(mapped)
        }
        Element::Jfet(element) => {
            let mut mapped = Jfet::with_model_and_capacitance(
                format!("{instance_name}.{}", element.name),
                map_subckt_node(&element.drain, instance_name, node_map),
                map_subckt_node(&element.gate, instance_name, node_map),
                map_subckt_node(&element.source, instance_name, node_map),
                element.polarity,
                element.beta,
                element.threshold_voltage,
                element.channel_length_modulation,
                element.gate_source_capacitance,
                element.gate_drain_capacitance,
            );
            mapped.flicker_noise_coefficient = element.flicker_noise_coefficient;
            mapped.flicker_noise_exponent = element.flicker_noise_exponent;
            mapped.junction_potential = element.junction_potential;
            mapped.forward_bias_depletion_coefficient = element.forward_bias_depletion_coefficient;
            mapped.gate_saturation_current = element.gate_saturation_current;
            mapped.gate_saturation_current_temperature_exponent =
                element.gate_saturation_current_temperature_exponent;
            mapped.bandgap_voltage = element.bandgap_voltage;
            mapped.doping_tail_parameter = element.doping_tail_parameter;
            mapped.noise_equation_level = element.noise_equation_level;
            mapped.channel_noise_coefficient = element.channel_noise_coefficient;
            mapped.drain_resistance = element.drain_resistance;
            mapped.source_resistance = element.source_resistance;
            mapped.threshold_voltage_temperature_coefficient =
                element.threshold_voltage_temperature_coefficient;
            mapped.alternative_threshold_voltage_temperature_coefficient =
                element.alternative_threshold_voltage_temperature_coefficient;
            mapped.nominal_temperature_kelvin = element.nominal_temperature_kelvin;
            mapped.mobility_temperature_exponent = element.mobility_temperature_exponent;
            mapped.mobility_temperature_coefficient = element.mobility_temperature_coefficient;
            Element::Jfet(mapped)
        }
        Element::Bjt(element) => {
            let mut expanded = Bjt::with_model_temperature_depletion_early_rolloff_junction_leakage_and_reverse_beta_parameters(
                format!("{instance_name}.{}", element.name),
                map_subckt_node(&element.collector, instance_name, node_map),
                map_subckt_node(&element.base, instance_name, node_map),
                map_subckt_node(&element.emitter, instance_name, node_map),
                element.polarity,
                element.saturation_current,
                element.forward_beta,
                element.thermal_voltage,
                element.base_emitter_capacitance,
                element.base_collector_capacitance,
                element.forward_transit_time,
                element.reverse_transit_time,
                element.saturation_current_temperature_exponent,
                element.energy_gap_electron_volts,
                element.forward_early_voltage,
                element.forward_emission_coefficient,
                element.reverse_emission_coefficient,
                element.base_emitter_junction_potential,
                element.base_emitter_grading_coefficient,
                element.base_collector_junction_potential,
                element.base_collector_grading_coefficient,
                element.forward_bias_depletion_coefficient,
                element.reverse_early_voltage,
                element.forward_beta_rolloff_current,
                element.base_emitter_leakage_saturation_current,
                element.base_emitter_leakage_emission_coefficient,
                element.base_collector_leakage_saturation_current,
                element.base_collector_leakage_emission_coefficient,
                element.forward_beta_temperature_exponent,
                element.reverse_beta,
            );
            expanded.reverse_beta_rolloff_current = element.reverse_beta_rolloff_current;
            expanded.nominal_temperature_kelvin = element.nominal_temperature_kelvin;
            expanded.flicker_noise_coefficient = element.flicker_noise_coefficient;
            expanded.flicker_noise_exponent = element.flicker_noise_exponent;
            expanded.forward_excess_phase_degrees = element.forward_excess_phase_degrees;
            expanded.forward_transit_time_bias_coefficient =
                element.forward_transit_time_bias_coefficient;
            expanded.forward_transit_time_current = element.forward_transit_time_current;
            expanded.forward_transit_time_voltage = element.forward_transit_time_voltage;
            expanded.emitter_resistance = element.emitter_resistance;
            expanded.collector_resistance = element.collector_resistance;
            expanded.base_resistance = element.base_resistance;
            expanded.minimum_base_resistance = element.minimum_base_resistance;
            expanded.base_resistance_half_current = element.base_resistance_half_current;
            expanded.base_collector_capacitance_fraction =
                element.base_collector_capacitance_fraction;
            Element::Bjt(expanded)
        }
        Element::Mosfet(element) => Element::Mosfet(Mosfet::with_model(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.drain, instance_name, node_map),
            map_subckt_node(&element.gate, instance_name, node_map),
            map_subckt_node(&element.source, instance_name, node_map),
            map_subckt_node(&element.body, instance_name, node_map),
            element.mosfet_type,
            element.params,
        )),
        Element::Vccs(element) => Element::Vccs(Vccs::new(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.positive, instance_name, node_map),
            map_subckt_node(&element.negative, instance_name, node_map),
            map_subckt_node(&element.control_positive, instance_name, node_map),
            map_subckt_node(&element.control_negative, instance_name, node_map),
            element.transconductance_siemens,
        )),
        Element::Vcvs(element) => Element::Vcvs(Vcvs::new(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.positive, instance_name, node_map),
            map_subckt_node(&element.negative, instance_name, node_map),
            map_subckt_node(&element.control_positive, instance_name, node_map),
            map_subckt_node(&element.control_negative, instance_name, node_map),
            element.gain,
        )),
        Element::Cccs(element) => Element::Cccs(Cccs::new(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.positive, instance_name, node_map),
            map_subckt_node(&element.negative, instance_name, node_map),
            map_subckt_source_ref(&element.control_source, instance_name),
            element.gain,
        )),
        Element::Ccvs(element) => Element::Ccvs(Ccvs::new(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.positive, instance_name, node_map),
            map_subckt_node(&element.negative, instance_name, node_map),
            map_subckt_source_ref(&element.control_source, instance_name),
            element.transresistance_ohms,
        )),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Waveform {
    Pwl(PwlWaveform),
    Sin(SinWaveform),
    Pulse(PulseWaveform),
    Exp(ExpWaveform),
}

impl Waveform {
    pub fn value_at(&self, time_seconds: f64) -> f64 {
        match self {
            Self::Pwl(waveform) => waveform.value_at(time_seconds),
            Self::Sin(waveform) => waveform.value_at(time_seconds),
            Self::Pulse(waveform) => waveform.value_at(time_seconds),
            Self::Exp(waveform) => waveform.value_at(time_seconds),
        }
    }

    pub fn period_seconds(&self) -> Option<f64> {
        match self {
            Self::Sin(waveform)
                if waveform.frequency_hz.is_finite()
                    && waveform.frequency_hz > 0.0
                    && waveform.damping == 0.0 =>
            {
                Some(1.0 / waveform.frequency_hz)
            }
            Self::Pulse(waveform)
                if waveform.period_seconds.is_finite() && waveform.period_seconds > 0.0 =>
            {
                Some(waveform.period_seconds)
            }
            _ => None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Pwl(waveform) => waveform.validate(),
            Self::Sin(waveform) => waveform.validate(),
            Self::Pulse(waveform) => waveform.validate(),
            Self::Exp(waveform) => waveform.validate(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PwlWaveform {
    pub points: Vec<(f64, f64)>,
}

impl PwlWaveform {
    pub fn new(points: impl Into<Vec<(f64, f64)>>) -> Self {
        Self {
            points: points.into(),
        }
    }

    pub fn value_at(&self, time_seconds: f64) -> f64 {
        if self.points.is_empty() {
            return f64::NAN;
        }
        if time_seconds <= self.points[0].0 {
            return self.points[0].1;
        }
        if time_seconds >= self.points[self.points.len() - 1].0 {
            return self.points[self.points.len() - 1].1;
        }
        for window in self.points.windows(2) {
            let (left_time, left_value) = window[0];
            let (right_time, right_value) = window[1];
            if time_seconds <= right_time {
                let phase = (time_seconds - left_time) / (right_time - left_time);
                return left_value + (right_value - left_value) * phase;
            }
        }
        self.points[self.points.len() - 1].1
    }

    fn validate(&self) -> Result<(), String> {
        if self.points.len() < 2 {
            return Err("PWL waveform requires at least two points".to_string());
        }
        let mut previous_time = f64::NEG_INFINITY;
        for (time, value) in &self.points {
            if !time.is_finite() || !value.is_finite() {
                return Err("PWL waveform times and values must be finite".to_string());
            }
            if *time <= previous_time {
                return Err("PWL waveform times must be strictly increasing".to_string());
            }
            previous_time = *time;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DigitalState {
    Low,
    High,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DigitalEvent {
    pub time_seconds: f64,
    pub state: DigitalState,
}

impl DigitalEvent {
    pub fn new(time_seconds: f64, state: DigitalState) -> Self {
        Self {
            time_seconds,
            state,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DigitalEventStream {
    pub signal_name: String,
    pub events: Vec<DigitalEvent>,
}

impl DigitalEventStream {
    pub fn new(signal_name: impl Into<String>, events: impl Into<Vec<DigitalEvent>>) -> Self {
        Self {
            signal_name: signal_name.into(),
            events: events.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DigitalTransientBridgeResult {
    pub points: Vec<TransientPoint>,
    pub output_streams: Vec<DigitalEventStream>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerDigitalTransientBridgePoint {
    pub corner_name: String,
    pub result: DigitalTransientBridgeResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerDigitalTransientBridgeResult {
    pub points: Vec<CornerDigitalTransientBridgePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveDigitalTransientBridgeResult {
    pub result: AdaptiveTransientResult,
    pub output_streams: Vec<DigitalEventStream>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerAdaptiveDigitalTransientBridgePoint {
    pub corner_name: String,
    pub result: AdaptiveDigitalTransientBridgeResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerAdaptiveDigitalTransientBridgeResult {
    pub points: Vec<CornerAdaptiveDigitalTransientBridgePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DigitalBridgeSchedule {
    pub stop_time: f64,
    pub breakpoints: Vec<f64>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DigitalLogicLevels {
    pub low_voltage: f64,
    pub high_voltage: f64,
    pub transition_seconds: f64,
}

impl DigitalLogicLevels {
    pub fn new(low_voltage: f64, high_voltage: f64, transition_seconds: f64) -> Self {
        Self {
            low_voltage,
            high_voltage,
            transition_seconds,
        }
    }

    pub fn cmos_1v8(transition_seconds: f64) -> Self {
        Self::new(0.0, 1.8, transition_seconds)
    }

    pub fn voltage_for(self, state: DigitalState) -> f64 {
        match state {
            DigitalState::Low => self.low_voltage,
            DigitalState::High => self.high_voltage,
        }
    }

    fn validate(self) -> Result<(), String> {
        if !self.low_voltage.is_finite()
            || !self.high_voltage.is_finite()
            || !self.transition_seconds.is_finite()
        {
            return Err("digital logic levels must be finite".to_string());
        }
        if self.high_voltage <= self.low_voltage {
            return Err("digital high voltage must be greater than low voltage".to_string());
        }
        if self.transition_seconds <= 0.0 {
            return Err("digital transition time must be finite and positive".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DigitalThresholds {
    pub low_max_voltage: f64,
    pub high_min_voltage: f64,
}

impl DigitalThresholds {
    pub fn new(low_max_voltage: f64, high_min_voltage: f64) -> Self {
        Self {
            low_max_voltage,
            high_min_voltage,
        }
    }

    pub fn cmos_1v8() -> Self {
        Self::new(0.6, 1.2)
    }

    pub fn classify(self, voltage: f64) -> Option<DigitalState> {
        if voltage <= self.low_max_voltage {
            Some(DigitalState::Low)
        } else if voltage >= self.high_min_voltage {
            Some(DigitalState::High)
        } else {
            None
        }
    }

    fn validate(self) -> Result<(), String> {
        if !self.low_max_voltage.is_finite() || !self.high_min_voltage.is_finite() {
            return Err("digital thresholds must be finite".to_string());
        }
        if self.high_min_voltage <= self.low_max_voltage {
            return Err("digital high threshold must be greater than low threshold".to_string());
        }
        Ok(())
    }
}

pub fn digital_events_to_pwl_waveform(
    events: &[DigitalEvent],
    levels: DigitalLogicLevels,
) -> Result<PwlWaveform, SpiceError> {
    levels
        .validate()
        .map_err(|reason| SpiceError::InvalidElement {
            name: "digital_events".to_string(),
            reason,
        })?;
    if events.is_empty() {
        return Err(SpiceError::InvalidElement {
            name: "digital_events".to_string(),
            reason: "at least one digital event is required".to_string(),
        });
    }

    let mut previous_time = f64::NEG_INFINITY;
    for event in events {
        if !event.time_seconds.is_finite() || event.time_seconds < 0.0 {
            return Err(SpiceError::InvalidElement {
                name: "digital_events".to_string(),
                reason: "digital event times must be finite and non-negative".to_string(),
            });
        }
        if event.time_seconds <= previous_time {
            return Err(SpiceError::InvalidElement {
                name: "digital_events".to_string(),
                reason: "digital event times must be strictly increasing".to_string(),
            });
        }
        previous_time = event.time_seconds;
    }

    let mut points = Vec::new();
    let mut current_state = events[0].state;
    points.push((events[0].time_seconds, levels.voltage_for(current_state)));

    for event in events.iter().skip(1) {
        if event.state == current_state {
            continue;
        }
        let start_time = event.time_seconds;
        let end_time = start_time + levels.transition_seconds;
        let last_time = points
            .last()
            .map(|point| point.0)
            .unwrap_or(f64::NEG_INFINITY);
        if start_time <= last_time {
            return Err(SpiceError::InvalidElement {
                name: "digital_events".to_string(),
                reason: "digital transition overlaps the previous transition".to_string(),
            });
        }
        points.push((start_time, levels.voltage_for(current_state)));
        points.push((end_time, levels.voltage_for(event.state)));
        current_state = event.state;
    }

    if points.len() == 1 {
        points.push((
            points[0].0 + levels.transition_seconds,
            levels.voltage_for(current_state),
        ));
    }

    let waveform = PwlWaveform::new(points);
    waveform
        .validate()
        .map_err(|reason| SpiceError::InvalidElement {
            name: "digital_events".to_string(),
            reason,
        })?;
    Ok(waveform)
}

pub fn digital_events_to_voltage_source(
    name: impl Into<String>,
    positive: impl Into<String>,
    negative: impl Into<String>,
    events: &[DigitalEvent],
    levels: DigitalLogicLevels,
) -> Result<VoltageSource, SpiceError> {
    let initial_voltage = events
        .first()
        .map(|event| levels.voltage_for(event.state))
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "digital_events".to_string(),
            reason: "at least one digital event is required".to_string(),
        })?;
    let waveform = digital_events_to_pwl_waveform(events, levels)?;
    Ok(VoltageSource::with_waveform(
        name,
        positive,
        negative,
        initial_voltage,
        Waveform::Pwl(waveform),
    ))
}

pub fn digital_event_streams_to_voltage_sources(
    streams: &[DigitalEventStream],
    negative: impl AsRef<str>,
    levels: DigitalLogicLevels,
) -> Result<Vec<VoltageSource>, SpiceError> {
    let negative = negative.as_ref().trim();
    if negative.is_empty() {
        return Err(SpiceError::InvalidElement {
            name: "digital_event_streams".to_string(),
            reason: "digital event stream negative node must not be empty".to_string(),
        });
    }

    let mut sources = Vec::new();
    let mut seen_signal_names = HashSet::new();
    for stream in streams {
        let signal_name = validate_digital_event_stream_name(stream, &mut seen_signal_names)?;

        sources.push(digital_events_to_voltage_source(
            format!("V{signal_name}"),
            signal_name.to_string(),
            negative.to_string(),
            &stream.events,
            levels,
        )?);
    }
    Ok(sources)
}

pub fn digital_event_streams_to_bridge_schedule(
    streams: &[DigitalEventStream],
    levels: DigitalLogicLevels,
) -> Result<DigitalBridgeSchedule, SpiceError> {
    levels
        .validate()
        .map_err(|reason| SpiceError::InvalidElement {
            name: "digital_bridge_schedule".to_string(),
            reason,
        })?;

    let mut seen_signal_names = HashSet::new();
    let mut breakpoints = Vec::new();
    let mut stop_time: f64 = 0.0;
    for stream in streams {
        validate_digital_event_stream_name(stream, &mut seen_signal_names)?;
        digital_events_to_pwl_waveform(&stream.events, levels)?;

        let mut current_state = stream.events[0].state;
        for (index, event) in stream.events.iter().enumerate() {
            breakpoints.push(event.time_seconds);
            stop_time = stop_time.max(event.time_seconds);

            if index > 0 && event.state != current_state {
                let transition_end = event.time_seconds + levels.transition_seconds;
                breakpoints.push(transition_end);
                stop_time = stop_time.max(transition_end);
                current_state = event.state;
            }
        }
    }

    breakpoints.sort_by(|left, right| left.total_cmp(right));
    breakpoints.dedup_by(|left, right| (*left - *right).abs() <= DIGITAL_BRIDGE_TIME_EPSILON);

    Ok(DigitalBridgeSchedule {
        stop_time,
        breakpoints,
    })
}

pub fn transient_with_digital_event_streams(
    circuit: &Circuit,
    input_streams: &[DigitalEventStream],
    negative: impl AsRef<str>,
    levels: DigitalLogicLevels,
    time_step: f64,
    stop_time: f64,
    output_probes: &[(&str, &str)],
    thresholds: DigitalThresholds,
) -> Result<DigitalTransientBridgeResult, SpiceError> {
    let mut bridged = circuit.clone();
    for source in digital_event_streams_to_voltage_sources(input_streams, negative, levels)? {
        bridged.add(Element::VoltageSource(source));
    }
    let points = transient(&bridged, time_step, stop_time)?;
    let output_streams =
        sample_transient_probes_as_digital_event_streams(&points, output_probes, thresholds)?;
    Ok(DigitalTransientBridgeResult {
        points,
        output_streams,
    })
}

pub fn transient_with_digital_event_streams_corners(
    circuit: &Circuit,
    input_streams: &[DigitalEventStream],
    negative: impl AsRef<str>,
    levels: DigitalLogicLevels,
    time_step: f64,
    stop_time: f64,
    output_probes: &[(&str, &str)],
    thresholds: DigitalThresholds,
    corners: &[CornerSpec],
) -> Result<CornerDigitalTransientBridgeResult, SpiceError> {
    let negative = negative.as_ref();
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerDigitalTransientBridgePoint {
            corner_name: corner.name.clone(),
            result: transient_with_digital_event_streams(
                &corner_circuit,
                input_streams,
                negative,
                levels,
                time_step,
                stop_time,
                output_probes,
                thresholds,
            )?,
        });
    }
    Ok(CornerDigitalTransientBridgeResult { points })
}

pub fn transient_adaptive_with_digital_event_streams(
    circuit: &Circuit,
    input_streams: &[DigitalEventStream],
    negative: impl AsRef<str>,
    levels: DigitalLogicLevels,
    time_step: f64,
    stop_time: f64,
    options: AdaptiveTransientOptions,
    output_probes: &[(&str, &str)],
    thresholds: DigitalThresholds,
) -> Result<AdaptiveDigitalTransientBridgeResult, SpiceError> {
    let mut bridged = circuit.clone();
    for source in digital_event_streams_to_voltage_sources(input_streams, negative, levels)? {
        bridged.add(Element::VoltageSource(source));
    }
    let result = transient_adaptive(&bridged, time_step, stop_time, options)?;
    let output_streams = sample_transient_probes_as_digital_event_streams(
        &result.points,
        output_probes,
        thresholds,
    )?;
    Ok(AdaptiveDigitalTransientBridgeResult {
        result,
        output_streams,
    })
}

pub fn transient_adaptive_with_digital_event_streams_corners(
    circuit: &Circuit,
    input_streams: &[DigitalEventStream],
    negative: impl AsRef<str>,
    levels: DigitalLogicLevels,
    time_step: f64,
    stop_time: f64,
    options: AdaptiveTransientOptions,
    output_probes: &[(&str, &str)],
    thresholds: DigitalThresholds,
    corners: &[CornerSpec],
) -> Result<CornerAdaptiveDigitalTransientBridgeResult, SpiceError> {
    let negative = negative.as_ref();
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerAdaptiveDigitalTransientBridgePoint {
            corner_name: corner.name.clone(),
            result: transient_adaptive_with_digital_event_streams(
                &corner_circuit,
                input_streams,
                negative,
                levels,
                time_step,
                stop_time,
                options,
                output_probes,
                thresholds,
            )?,
        });
    }
    Ok(CornerAdaptiveDigitalTransientBridgeResult { points })
}

pub fn sample_transient_probe_as_digital_events(
    points: &[TransientPoint],
    probe: &str,
    thresholds: DigitalThresholds,
) -> Result<Vec<DigitalEvent>, SpiceError> {
    thresholds
        .validate()
        .map_err(|reason| SpiceError::InvalidElement {
            name: "digital_thresholds".to_string(),
            reason,
        })?;

    let mut events = Vec::new();
    let mut previous_state = None;
    let mut previous_time = f64::NEG_INFINITY;
    for point in points {
        if !point.time.is_finite() || point.time < 0.0 {
            return Err(SpiceError::InvalidElement {
                name: "transient_points".to_string(),
                reason: "transient sample times must be finite and non-negative".to_string(),
            });
        }
        if point.time <= previous_time {
            return Err(SpiceError::InvalidElement {
                name: "transient_points".to_string(),
                reason: "transient sample times must be strictly increasing".to_string(),
            });
        }
        previous_time = point.time;

        if let Some(state) = thresholds.classify(probe_value(point, probe)?) {
            if previous_state != Some(state) {
                events.push(DigitalEvent::new(point.time, state));
                previous_state = Some(state);
            }
        }
    }
    Ok(events)
}

pub fn sample_transient_probes_as_digital_event_streams(
    points: &[TransientPoint],
    probes: &[(&str, &str)],
    thresholds: DigitalThresholds,
) -> Result<Vec<DigitalEventStream>, SpiceError> {
    thresholds
        .validate()
        .map_err(|reason| SpiceError::InvalidElement {
            name: "digital_thresholds".to_string(),
            reason,
        })?;

    let mut streams = Vec::new();
    for (signal_name, probe) in probes {
        let signal_name = signal_name.trim();
        if signal_name.is_empty() {
            return Err(SpiceError::InvalidElement {
                name: "digital_event_stream".to_string(),
                reason: "digital event stream signal name must not be empty".to_string(),
            });
        }
        if streams
            .iter()
            .any(|stream: &DigitalEventStream| stream.signal_name == signal_name)
        {
            return Err(SpiceError::InvalidElement {
                name: signal_name.to_string(),
                reason: "digital event stream signal names must be unique".to_string(),
            });
        }

        let events = sample_transient_probe_as_digital_events(points, probe, thresholds)?;
        streams.push(DigitalEventStream::new(signal_name, events));
    }
    Ok(streams)
}

pub fn format_digital_event_table(events: &[DigitalEvent]) -> Result<String, SpiceError> {
    let mut rows = vec!["Index\tTime\tState".to_string()];
    let mut previous_time = f64::NEG_INFINITY;
    for (index, event) in events.iter().enumerate() {
        validate_digital_event_time(event.time_seconds, previous_time, "digital_events")?;
        previous_time = event.time_seconds;
        rows.push(format!(
            "{index}\t{}\t{}",
            format_table_number(event.time_seconds),
            format_digital_state(event.state)
        ));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_digital_event_stream_table(
    streams: &[DigitalEventStream],
) -> Result<String, SpiceError> {
    let mut rows = vec!["Signal\tIndex\tTime\tState".to_string()];
    for stream in streams {
        if stream.signal_name.trim().is_empty() {
            return Err(SpiceError::InvalidElement {
                name: "digital_event_stream".to_string(),
                reason: "digital event stream signal name must not be empty".to_string(),
            });
        }
        let mut previous_time = f64::NEG_INFINITY;
        for (index, event) in stream.events.iter().enumerate() {
            validate_digital_event_time(event.time_seconds, previous_time, &stream.signal_name)?;
            previous_time = event.time_seconds;
            rows.push(format!(
                "{}\t{index}\t{}\t{}",
                stream.signal_name,
                format_table_number(event.time_seconds),
                format_digital_state(event.state)
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_corner_digital_event_stream_table(
    result: &CornerDigitalTransientBridgeResult,
) -> Result<String, SpiceError> {
    let mut rows = vec!["Corner\tSignal\tIndex\tTime\tState".to_string()];
    for corner in &result.points {
        for stream in &corner.result.output_streams {
            if stream.signal_name.trim().is_empty() {
                return Err(SpiceError::InvalidElement {
                    name: "digital_event_stream".to_string(),
                    reason: "digital event stream signal name must not be empty".to_string(),
                });
            }
            let mut previous_time = f64::NEG_INFINITY;
            for (index, event) in stream.events.iter().enumerate() {
                validate_digital_event_time(
                    event.time_seconds,
                    previous_time,
                    &stream.signal_name,
                )?;
                previous_time = event.time_seconds;
                rows.push(format!(
                    "{}\t{}\t{index}\t{}\t{}",
                    corner.corner_name,
                    stream.signal_name,
                    format_table_number(event.time_seconds),
                    format_digital_state(event.state)
                ));
            }
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_adaptive_digital_event_stream_table(
    result: &AdaptiveDigitalTransientBridgeResult,
) -> Result<String, SpiceError> {
    let mut rows = vec!["Method\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState".to_string()];
    for stream in &result.output_streams {
        if stream.signal_name.trim().is_empty() {
            return Err(SpiceError::InvalidElement {
                name: "digital_event_stream".to_string(),
                reason: "digital event stream signal name must not be empty".to_string(),
            });
        }
        let mut previous_time = f64::NEG_INFINITY;
        for (index, event) in stream.events.iter().enumerate() {
            validate_digital_event_time(event.time_seconds, previous_time, &stream.signal_name)?;
            previous_time = event.time_seconds;
            rows.push(format!(
                "{}\t{}\t{}\t{}\t{index}\t{}\t{}",
                format_transient_method(result.result.method),
                result.result.steps_rejected,
                result.result.converged,
                stream.signal_name,
                format_table_number(event.time_seconds),
                format_digital_state(event.state)
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_corner_adaptive_digital_event_stream_table(
    result: &CornerAdaptiveDigitalTransientBridgeResult,
) -> Result<String, SpiceError> {
    let mut rows =
        vec!["Corner\tMethod\tStepsRejected\tConverged\tSignal\tIndex\tTime\tState".to_string()];
    for corner in &result.points {
        for stream in &corner.result.output_streams {
            if stream.signal_name.trim().is_empty() {
                return Err(SpiceError::InvalidElement {
                    name: "digital_event_stream".to_string(),
                    reason: "digital event stream signal name must not be empty".to_string(),
                });
            }
            let mut previous_time = f64::NEG_INFINITY;
            for (index, event) in stream.events.iter().enumerate() {
                validate_digital_event_time(
                    event.time_seconds,
                    previous_time,
                    &stream.signal_name,
                )?;
                previous_time = event.time_seconds;
                rows.push(format!(
                    "{}\t{}\t{}\t{}\t{}\t{index}\t{}\t{}",
                    corner.corner_name,
                    format_transient_method(corner.result.result.method),
                    corner.result.result.steps_rejected,
                    corner.result.result.converged,
                    stream.signal_name,
                    format_table_number(event.time_seconds),
                    format_digital_state(event.state)
                ));
            }
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_digital_bridge_schedule_table(
    schedule: &DigitalBridgeSchedule,
) -> Result<String, SpiceError> {
    if !schedule.stop_time.is_finite() || schedule.stop_time < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "digital_bridge_schedule".to_string(),
            reason: "digital bridge stop time must be finite and non-negative".to_string(),
        });
    }

    let mut rows = vec!["Index\tTime\tStopTime".to_string()];
    let mut previous_time = f64::NEG_INFINITY;
    for (index, time_seconds) in schedule.breakpoints.iter().enumerate() {
        validate_digital_event_time(*time_seconds, previous_time, "digital_bridge_schedule")?;
        if *time_seconds > schedule.stop_time {
            return Err(SpiceError::InvalidElement {
                name: "digital_bridge_schedule".to_string(),
                reason: "digital bridge breakpoint must not exceed stop time".to_string(),
            });
        }
        previous_time = *time_seconds;
        rows.push(format!(
            "{index}\t{}\t{}",
            format_table_number(*time_seconds),
            format_table_number(schedule.stop_time)
        ));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_digital_event_stream_vcd(
    streams: &[DigitalEventStream],
) -> Result<String, SpiceError> {
    format_digital_event_stream_vcd_with_options(streams, "spice_bridge", "1ps")
}

pub fn format_digital_event_stream_vcd_with_options(
    streams: &[DigitalEventStream],
    module_name: &str,
    timescale: &str,
) -> Result<String, SpiceError> {
    let module_name = module_name.trim();
    if module_name.is_empty() {
        return Err(SpiceError::InvalidElement {
            name: "digital_event_stream_vcd".to_string(),
            reason: "module name must not be empty".to_string(),
        });
    }
    if timescale != "1ps" {
        return Err(SpiceError::InvalidElement {
            name: "digital_event_stream_vcd".to_string(),
            reason: "only 1ps timescale is supported".to_string(),
        });
    }

    let mut seen_signal_names = HashSet::new();
    let mut signal_ids = HashMap::new();
    for (index, stream) in streams.iter().enumerate() {
        let signal_name = validate_digital_event_stream_name(stream, &mut seen_signal_names)?;
        signal_ids.insert(signal_name.to_string(), vcd_identifier(index));
        let mut previous_time = f64::NEG_INFINITY;
        for event in &stream.events {
            validate_digital_event_time(event.time_seconds, previous_time, signal_name)?;
            previous_time = event.time_seconds;
        }
    }

    let mut rows = vec![
        "$version coding-adventures spice-engine mixed-signal bridge $end".to_string(),
        format!("$timescale {timescale} $end"),
        format!("$scope module {module_name} $end"),
    ];
    for stream in streams {
        let signal_name = stream.signal_name.trim();
        rows.push(format!(
            "$var wire 1 {} {} $end",
            signal_ids[signal_name], signal_name
        ));
    }
    rows.push("$upscope $end".to_string());
    rows.push("$enddefinitions $end".to_string());
    rows.push("$dumpvars".to_string());
    for stream in streams {
        if let Some(event) = stream.events.first() {
            rows.push(format!(
                "{}{}",
                vcd_state_value(event.state),
                signal_ids[stream.signal_name.trim()]
            ));
        }
    }
    rows.push("$end".to_string());

    let mut events_by_tick: BTreeMap<i64, Vec<(String, DigitalState)>> = BTreeMap::new();
    for stream in streams {
        let signal_id = signal_ids[stream.signal_name.trim()].clone();
        for event in &stream.events {
            events_by_tick
                .entry(vcd_tick(event.time_seconds)?)
                .or_default()
                .push((signal_id.clone(), event.state));
        }
    }
    for (tick, events) in events_by_tick {
        rows.push(format!("#{tick}"));
        for (signal_id, state) in events {
            rows.push(format!("{}{}", vcd_state_value(state), signal_id));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

fn validate_digital_event_stream_name<'a>(
    stream: &'a DigitalEventStream,
    seen_signal_names: &mut HashSet<String>,
) -> Result<&'a str, SpiceError> {
    let signal_name = stream.signal_name.trim();
    if signal_name.is_empty() {
        return Err(SpiceError::InvalidElement {
            name: "digital_event_stream".to_string(),
            reason: "digital event stream signal name must not be empty".to_string(),
        });
    }
    if !seen_signal_names.insert(signal_name.to_string()) {
        return Err(SpiceError::InvalidElement {
            name: signal_name.to_string(),
            reason: "digital event stream signal names must be unique".to_string(),
        });
    }
    Ok(signal_name)
}

fn validate_digital_event_time(
    time_seconds: f64,
    previous_time: f64,
    name: &str,
) -> Result<(), SpiceError> {
    if !time_seconds.is_finite() || time_seconds < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: name.to_string(),
            reason: "digital event times must be finite and non-negative".to_string(),
        });
    }
    if time_seconds <= previous_time {
        return Err(SpiceError::InvalidElement {
            name: name.to_string(),
            reason: "digital event times must be strictly increasing".to_string(),
        });
    }
    Ok(())
}

fn format_digital_state(state: DigitalState) -> &'static str {
    match state {
        DigitalState::Low => "low",
        DigitalState::High => "high",
    }
}

fn vcd_identifier(index: usize) -> String {
    format!("s{index}")
}

fn vcd_tick(time_seconds: f64) -> Result<i64, SpiceError> {
    if !time_seconds.is_finite() || time_seconds < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "digital_event_stream_vcd".to_string(),
            reason: "event times must be finite and non-negative".to_string(),
        });
    }
    Ok((time_seconds / 1.0e-12).round() as i64)
}

fn vcd_state_value(state: DigitalState) -> &'static str {
    match state {
        DigitalState::Low => "0",
        DigitalState::High => "1",
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SinWaveform {
    pub offset: f64,
    pub amplitude: f64,
    pub frequency_hz: f64,
    pub delay_seconds: f64,
    pub damping: f64,
}

impl SinWaveform {
    pub fn new(offset: f64, amplitude: f64, frequency_hz: f64) -> Self {
        Self {
            offset,
            amplitude,
            frequency_hz,
            delay_seconds: 0.0,
            damping: 0.0,
        }
    }

    pub fn with_delay_damping(
        offset: f64,
        amplitude: f64,
        frequency_hz: f64,
        delay_seconds: f64,
        damping: f64,
    ) -> Self {
        Self {
            offset,
            amplitude,
            frequency_hz,
            delay_seconds,
            damping,
        }
    }

    pub fn value_at(&self, time_seconds: f64) -> f64 {
        if time_seconds < self.delay_seconds {
            return self.offset;
        }
        let shifted_time = time_seconds - self.delay_seconds;
        let envelope = if self.damping == 0.0 {
            1.0
        } else {
            (-self.damping * shifted_time).exp()
        };
        self.offset + self.amplitude * (TWO_PI * self.frequency_hz * shifted_time).sin() * envelope
    }

    fn validate(&self) -> Result<(), String> {
        if !self.offset.is_finite()
            || !self.amplitude.is_finite()
            || !self.frequency_hz.is_finite()
            || !self.delay_seconds.is_finite()
            || !self.damping.is_finite()
        {
            return Err("SIN waveform parameters must be finite".to_string());
        }
        if self.frequency_hz < 0.0 {
            return Err("SIN waveform frequency must be non-negative".to_string());
        }
        if self.delay_seconds < 0.0 {
            return Err("SIN waveform delay must be non-negative".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PulseWaveform {
    pub initial_value: f64,
    pub pulsed_value: f64,
    pub delay_seconds: f64,
    pub rise_time_seconds: f64,
    pub fall_time_seconds: f64,
    pub pulse_width_seconds: f64,
    pub period_seconds: f64,
}

impl PulseWaveform {
    pub fn new(
        initial_value: f64,
        pulsed_value: f64,
        delay_seconds: f64,
        rise_time_seconds: f64,
        fall_time_seconds: f64,
        pulse_width_seconds: f64,
        period_seconds: f64,
    ) -> Self {
        Self {
            initial_value,
            pulsed_value,
            delay_seconds,
            rise_time_seconds,
            fall_time_seconds,
            pulse_width_seconds,
            period_seconds,
        }
    }

    pub fn value_at(&self, time_seconds: f64) -> f64 {
        if time_seconds < self.delay_seconds {
            return self.initial_value;
        }
        let elapsed = (time_seconds - self.delay_seconds) % self.period_seconds;
        if self.rise_time_seconds > 0.0 && elapsed < self.rise_time_seconds {
            let phase = elapsed / self.rise_time_seconds;
            return self.initial_value + (self.pulsed_value - self.initial_value) * phase;
        }
        if elapsed < self.rise_time_seconds + self.pulse_width_seconds {
            return self.pulsed_value;
        }
        let fall_start = self.rise_time_seconds + self.pulse_width_seconds;
        if self.fall_time_seconds > 0.0 && elapsed < fall_start + self.fall_time_seconds {
            let phase = (elapsed - fall_start) / self.fall_time_seconds;
            return self.pulsed_value + (self.initial_value - self.pulsed_value) * phase;
        }
        self.initial_value
    }

    fn validate(&self) -> Result<(), String> {
        if !self.initial_value.is_finite()
            || !self.pulsed_value.is_finite()
            || !self.delay_seconds.is_finite()
            || !self.rise_time_seconds.is_finite()
            || !self.fall_time_seconds.is_finite()
            || !self.pulse_width_seconds.is_finite()
            || !self.period_seconds.is_finite()
        {
            return Err("PULSE waveform parameters must be finite".to_string());
        }
        if self.delay_seconds < 0.0
            || self.rise_time_seconds < 0.0
            || self.fall_time_seconds < 0.0
            || self.pulse_width_seconds < 0.0
            || self.period_seconds <= 0.0
        {
            return Err(
                "PULSE waveform timing values must be non-negative and period positive".to_string(),
            );
        }
        if self.rise_time_seconds + self.pulse_width_seconds + self.fall_time_seconds
            > self.period_seconds
        {
            return Err("PULSE waveform high interval must fit within the period".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ExpWaveform {
    pub initial_value: f64,
    pub pulsed_value: f64,
    pub rise_delay_seconds: f64,
    pub rise_time_constant_seconds: f64,
    pub fall_delay_seconds: f64,
    pub fall_time_constant_seconds: f64,
}

impl ExpWaveform {
    pub fn new(
        initial_value: f64,
        pulsed_value: f64,
        rise_delay_seconds: f64,
        rise_time_constant_seconds: f64,
        fall_delay_seconds: f64,
        fall_time_constant_seconds: f64,
    ) -> Self {
        Self {
            initial_value,
            pulsed_value,
            rise_delay_seconds,
            rise_time_constant_seconds,
            fall_delay_seconds,
            fall_time_constant_seconds,
        }
    }

    pub fn value_at(&self, time_seconds: f64) -> f64 {
        if time_seconds <= self.rise_delay_seconds {
            return self.initial_value;
        }
        let mut value = self.initial_value
            + (self.pulsed_value - self.initial_value)
                * (1.0
                    - (-(time_seconds - self.rise_delay_seconds)
                        / self.rise_time_constant_seconds)
                        .exp());
        if time_seconds >= self.fall_delay_seconds {
            value += (self.initial_value - self.pulsed_value)
                * (1.0
                    - (-(time_seconds - self.fall_delay_seconds)
                        / self.fall_time_constant_seconds)
                        .exp());
        }
        value
    }

    fn validate(&self) -> Result<(), String> {
        if !self.initial_value.is_finite()
            || !self.pulsed_value.is_finite()
            || !self.rise_delay_seconds.is_finite()
            || !self.rise_time_constant_seconds.is_finite()
            || !self.fall_delay_seconds.is_finite()
            || !self.fall_time_constant_seconds.is_finite()
        {
            return Err("EXP waveform parameters must be finite".to_string());
        }
        if self.rise_delay_seconds < 0.0
            || self.fall_delay_seconds < 0.0
            || self.rise_time_constant_seconds <= 0.0
            || self.fall_time_constant_seconds <= 0.0
        {
            return Err(
                "EXP waveform delays must be non-negative and time constants positive".to_string(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Resistor(Resistor),
    Capacitor(Capacitor),
    Inductor(Inductor),
    MutualInductor(MutualInductor),
    TransmissionLine(TransmissionLine),
    VoltageSource(VoltageSource),
    CurrentSource(CurrentSource),
    BSource(BSource),
    CustomModel(CustomModel),
    Diode(Diode),
    Jfet(Jfet),
    Bjt(Bjt),
    Mosfet(Mosfet),
    Vccs(Vccs),
    Vcvs(Vcvs),
    Cccs(Cccs),
    Ccvs(Ccvs),
}

fn is_integer_multiple(candidate: f64, period: f64, tolerance: f64) -> bool {
    let ratio = candidate / period;
    let nearest = ratio.round();
    nearest >= 1.0 && (ratio - nearest).abs() <= tolerance * ratio.abs().max(1.0)
}

pub fn estimate_period(circuit: &Circuit) -> Option<f64> {
    estimate_period_with_tolerance(circuit, 1.0e-9)
}

pub fn estimate_period_with_tolerance(circuit: &Circuit, tolerance: f64) -> Option<f64> {
    let mut periods = Vec::new();
    for element in circuit.elements() {
        let waveform = match element {
            Element::VoltageSource(source) => source.waveform.as_ref(),
            Element::CurrentSource(source) => source.waveform.as_ref(),
            _ => None,
        };
        if let Some(waveform) = waveform {
            let period = waveform.period_seconds()?;
            periods.push(period);
        }
    }
    if periods.is_empty() {
        return None;
    }

    let candidate = periods.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !candidate.is_finite() || candidate <= 0.0 {
        return None;
    }
    periods
        .iter()
        .all(|period| is_integer_multiple(candidate, *period, tolerance))
        .then_some(candidate)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resistor {
    pub name: String,
    pub n1: String,
    pub n2: String,
    pub resistance_ohms: f64,
}

impl Resistor {
    pub fn new(
        name: impl Into<String>,
        n1: impl Into<String>,
        n2: impl Into<String>,
        resistance_ohms: f64,
    ) -> Self {
        Self {
            name: name.into(),
            n1: n1.into(),
            n2: n2.into(),
            resistance_ohms,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Capacitor {
    pub name: String,
    pub n1: String,
    pub n2: String,
    pub capacitance_farads: f64,
    pub initial_voltage: f64,
}

impl Capacitor {
    pub fn new(
        name: impl Into<String>,
        n1: impl Into<String>,
        n2: impl Into<String>,
        capacitance_farads: f64,
    ) -> Self {
        Self::with_initial_voltage(name, n1, n2, capacitance_farads, 0.0)
    }

    pub fn with_initial_voltage(
        name: impl Into<String>,
        n1: impl Into<String>,
        n2: impl Into<String>,
        capacitance_farads: f64,
        initial_voltage: f64,
    ) -> Self {
        Self {
            name: name.into(),
            n1: n1.into(),
            n2: n2.into(),
            capacitance_farads,
            initial_voltage,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Inductor {
    pub name: String,
    pub n1: String,
    pub n2: String,
    pub inductance_henrys: f64,
    pub initial_current: f64,
}

impl Inductor {
    pub fn new(
        name: impl Into<String>,
        n1: impl Into<String>,
        n2: impl Into<String>,
        inductance_henrys: f64,
    ) -> Self {
        Self::with_initial_current(name, n1, n2, inductance_henrys, 0.0)
    }

    pub fn with_initial_current(
        name: impl Into<String>,
        n1: impl Into<String>,
        n2: impl Into<String>,
        inductance_henrys: f64,
        initial_current: f64,
    ) -> Self {
        Self {
            name: name.into(),
            n1: n1.into(),
            n2: n2.into(),
            inductance_henrys,
            initial_current,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutualInductor {
    pub name: String,
    pub primary: String,
    pub secondary: String,
    pub coupling: f64,
}

impl MutualInductor {
    pub fn new(
        name: impl Into<String>,
        primary: impl Into<String>,
        secondary: impl Into<String>,
        coupling: f64,
    ) -> Self {
        Self {
            name: name.into(),
            primary: primary.into(),
            secondary: secondary.into(),
            coupling,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransmissionLine {
    pub name: String,
    pub n1: String,
    pub n2: String,
    pub n3: String,
    pub n4: String,
    pub characteristic_impedance_ohms: f64,
    pub delay_seconds: f64,
}

impl TransmissionLine {
    pub fn new(
        name: impl Into<String>,
        n1: impl Into<String>,
        n2: impl Into<String>,
        n3: impl Into<String>,
        n4: impl Into<String>,
        characteristic_impedance_ohms: f64,
        delay_seconds: f64,
    ) -> Self {
        Self {
            name: name.into(),
            n1: n1.into(),
            n2: n2.into(),
            n3: n3.into(),
            n4: n4.into(),
            characteristic_impedance_ohms,
            delay_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoltageSource {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub voltage: f64,
    pub ac: Option<AcSource>,
    pub waveform: Option<Waveform>,
}

impl VoltageSource {
    pub fn new(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        voltage: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            voltage,
            ac: None,
            waveform: None,
        }
    }

    pub fn with_ac(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        voltage: f64,
        magnitude: f64,
        phase_degrees: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            voltage,
            ac: Some(AcSource::new(magnitude, phase_degrees)),
            waveform: None,
        }
    }

    pub fn with_waveform(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        voltage: f64,
        waveform: Waveform,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            voltage,
            ac: None,
            waveform: Some(waveform),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentSource {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub current: f64,
    pub ac: Option<AcSource>,
    pub waveform: Option<Waveform>,
}

impl CurrentSource {
    pub fn new(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        current: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            current,
            ac: None,
            waveform: None,
        }
    }

    pub fn with_ac(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        current: f64,
        magnitude: f64,
        phase_degrees: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            current,
            ac: Some(AcSource::new(magnitude, phase_degrees)),
            waveform: None,
        }
    }

    pub fn with_waveform(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        current: f64,
        waveform: Waveform,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            current,
            ac: None,
            waveform: Some(waveform),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BSource {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub voltage_expr: Option<String>,
    pub current_expr: Option<String>,
}

impl BSource {
    pub fn current(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        current_expr: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            voltage_expr: None,
            current_expr: Some(current_expr.into()),
        }
    }

    pub fn voltage(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        voltage_expr: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            voltage_expr: Some(voltage_expr.into()),
            current_expr: None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CustomModelEvaluation {
    pub current_amps: f64,
    pub conductance_siemens: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CustomModelKind {
    LinearConductance {
        conductance_siemens: f64,
        current_offset_amps: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomModel {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub model_name: String,
    pub parameters: BTreeMap<String, f64>,
    pub kind: CustomModelKind,
}

impl CustomModel {
    pub fn linear_conductance(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        conductance_siemens: f64,
    ) -> Self {
        Self::linear_conductance_with_offset(name, positive, negative, conductance_siemens, 0.0)
    }

    pub fn linear_conductance_with_offset(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        conductance_siemens: f64,
        current_offset_amps: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            model_name: "linear_conductance".to_string(),
            parameters: BTreeMap::new(),
            kind: CustomModelKind::LinearConductance {
                conductance_siemens,
                current_offset_amps,
            },
        }
    }

    pub fn evaluate(&self, voltage: f64) -> Result<CustomModelEvaluation, SpiceError> {
        validate_custom_model(self)?;
        let evaluation = match self.kind {
            CustomModelKind::LinearConductance {
                conductance_siemens,
                current_offset_amps,
            } => CustomModelEvaluation {
                current_amps: conductance_siemens * voltage + current_offset_amps,
                conductance_siemens,
            },
        };
        if !evaluation.current_amps.is_finite() {
            return Err(SpiceError::InvalidElement {
                name: self.name.clone(),
                reason: "custom-model current must be finite".to_string(),
            });
        }
        if !evaluation.conductance_siemens.is_finite() {
            return Err(SpiceError::InvalidElement {
                name: self.name.clone(),
                reason: "custom-model conductance must be finite".to_string(),
            });
        }
        Ok(evaluation)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CustomModelDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomModelDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: CustomModelDiagnosticSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomModelSourceAnalysis {
    pub accepted: bool,
    pub subset: String,
    pub module_name: Option<String>,
    pub terminals: Vec<String>,
    pub contribution: Option<(String, String)>,
    pub diagnostics: Vec<CustomModelDiagnostic>,
}

const CUSTOM_MODEL_SUBSET: &str = "two-terminal-current-contribution-v0";

pub fn analyze_custom_model_source(source: &str) -> CustomModelSourceAnalysis {
    let mut diagnostics = Vec::new();
    let trimmed = source.trim();
    if trimmed.is_empty() {
        diagnostics.push(custom_model_error(
            "CUSTOM_MODEL_EMPTY_SOURCE",
            "custom model source is empty",
        ));
        return CustomModelSourceAnalysis {
            accepted: false,
            subset: CUSTOM_MODEL_SUBSET.to_string(),
            module_name: None,
            terminals: Vec::new(),
            contribution: None,
            diagnostics,
        };
    }

    let lowered = trimmed.to_ascii_lowercase();
    for &(token, message) in CUSTOM_MODEL_FORBIDDEN_PATTERNS {
        if lowered.contains(token) {
            diagnostics.push(custom_model_error(
                "CUSTOM_MODEL_FORBIDDEN_CONSTRUCT",
                message,
            ));
        }
    }

    let (module_name, terminals) = match parse_custom_model_module_header(trimmed) {
        Some(parsed) => parsed,
        None => {
            diagnostics.push(custom_model_error(
                "CUSTOM_MODEL_MISSING_MODULE",
                "custom model source must declare a module with a port list",
            ));
            (None, Vec::new())
        }
    };
    if !terminals.is_empty() && terminals.len() < 2 {
        diagnostics.push(custom_model_error(
            "CUSTOM_MODEL_PORT_COUNT",
            "custom model module must expose at least two terminals",
        ));
    }

    let contribution = parse_custom_model_contribution(trimmed);
    if contribution.is_none() {
        diagnostics.push(custom_model_error(
            "CUSTOM_MODEL_MISSING_CONTRIBUTION",
            "custom model source must contain a two-terminal I(p,n) <+ contribution",
        ));
    } else if let Some((positive, negative)) = &contribution {
        if !terminals.is_empty() && (!terminals.contains(positive) || !terminals.contains(negative))
        {
            diagnostics.push(custom_model_error(
                "CUSTOM_MODEL_UNKNOWN_TERMINAL",
                "current contribution terminals must be declared module ports",
            ));
        }
    }

    CustomModelSourceAnalysis {
        accepted: !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == CustomModelDiagnosticSeverity::Error),
        subset: CUSTOM_MODEL_SUBSET.to_string(),
        module_name,
        terminals,
        contribution,
        diagnostics,
    }
}

const CUSTOM_MODEL_FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
    (
        "ddt",
        "dynamic charge operators are not accepted in this custom-model subset",
    ),
    (
        "idt",
        "dynamic integration operators are not accepted in this custom-model subset",
    ),
    (
        "laplace",
        "Laplace-domain operators are not accepted in this custom-model subset",
    ),
    (
        "cross",
        "event crossing operators are not accepted in this custom-model subset",
    ),
    (
        "timer",
        "timer events are not accepted in this custom-model subset",
    ),
    (
        "@(",
        "event controls are not accepted in this custom-model subset",
    ),
    (
        "$finish",
        "system tasks are not accepted in this custom-model subset",
    ),
    (
        "$stop",
        "system tasks are not accepted in this custom-model subset",
    ),
    (
        "$display",
        "system tasks are not accepted in this custom-model subset",
    ),
    (
        "initial",
        "procedural initial blocks are not accepted in this custom-model subset",
    ),
    (
        "always",
        "procedural always blocks are not accepted in this custom-model subset",
    ),
    (
        "analog function",
        "analog functions are not accepted in this custom-model subset",
    ),
    (
        "discipline",
        "discipline declarations are not accepted in this custom-model subset",
    ),
    (
        "branch ",
        "named branch declarations are not accepted in this custom-model subset",
    ),
];

fn custom_model_error(code: &str, message: &str) -> CustomModelDiagnostic {
    CustomModelDiagnostic {
        code: code.to_string(),
        message: message.to_string(),
        severity: CustomModelDiagnosticSeverity::Error,
    }
}

fn parse_custom_model_module_header(source: &str) -> Option<(Option<String>, Vec<String>)> {
    let lowered = source.to_ascii_lowercase();
    let module_index = lowered.find("module")?;
    let after_module = source[module_index + "module".len()..].trim_start();
    let name_end = after_module
        .char_indices()
        .find_map(|(index, ch)| (!is_identifier_char(ch)).then_some(index))
        .unwrap_or(after_module.len());
    let name = after_module[..name_end].trim();
    if name.is_empty() || !is_identifier_start(name.chars().next()?) {
        return None;
    }
    let after_name = after_module[name_end..].trim_start();
    let open = after_name.find('(')?;
    let close = after_name[open + 1..].find(')')? + open + 1;
    if !after_name[close + 1..].trim_start().starts_with(';') {
        return None;
    }
    let ports = after_name[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|port| !port.is_empty())
        .map(ToString::to_string)
        .collect();
    Some((Some(name.to_string()), ports))
}

fn parse_custom_model_contribution(source: &str) -> Option<(String, String)> {
    let lowered = source.to_ascii_lowercase();
    let current_index = lowered.find("i(")?;
    let after_current = &source[current_index + 2..];
    let close = after_current.find(')')?;
    if !after_current[close + 1..].trim_start().starts_with("<+") {
        return None;
    }
    let mut args = after_current[..close].split(',').map(str::trim);
    let positive = args.next()?.to_string();
    let negative = args.next()?.to_string();
    if args.next().is_some()
        || positive.is_empty()
        || negative.is_empty()
        || !positive.chars().next().is_some_and(is_identifier_start)
        || !negative.chars().next().is_some_and(is_identifier_start)
    {
        return None;
    }
    Some((positive, negative))
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_char(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphanumeric()
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AcSource {
    pub magnitude: f64,
    pub phase_degrees: f64,
}

impl AcSource {
    pub fn new(magnitude: f64, phase_degrees: f64) -> Self {
        Self {
            magnitude,
            phase_degrees,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diode {
    pub name: String,
    pub anode: String,
    pub cathode: String,
    pub saturation_current: f64,
    pub thermal_voltage: f64,
    pub emission_coefficient: f64,
    pub breakdown_voltage: Option<f64>,
    pub breakdown_current: f64,
    pub junction_capacitance: f64,
    pub transit_time: f64,
    pub junction_potential: f64,
    pub grading_coefficient: f64,
    pub forward_bias_depletion_coefficient: f64,
    pub saturation_current_temperature_exponent: f64,
    pub energy_gap_electron_volts: f64,
    pub series_resistance: f64,
    pub flicker_noise_coefficient: f64,
    pub flicker_noise_exponent: f64,
}

impl Diode {
    pub fn new(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
    ) -> Self {
        Self::with_model(name, anode, cathode, 1.0e-15, 0.02585)
    }

    pub fn with_model(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
        saturation_current: f64,
        thermal_voltage: f64,
    ) -> Self {
        Self::with_model_and_emission_coefficient(
            name,
            anode,
            cathode,
            saturation_current,
            thermal_voltage,
            1.0,
        )
    }

    pub fn with_model_and_emission_coefficient(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
        saturation_current: f64,
        thermal_voltage: f64,
        emission_coefficient: f64,
    ) -> Self {
        Self::with_model_and_breakdown(
            name,
            anode,
            cathode,
            saturation_current,
            thermal_voltage,
            emission_coefficient,
            None,
            1.0e-3,
            0.0,
            0.0,
        )
    }

    pub fn with_model_and_breakdown(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
        saturation_current: f64,
        thermal_voltage: f64,
        emission_coefficient: f64,
        breakdown_voltage: Option<f64>,
        breakdown_current: f64,
        junction_capacitance: f64,
        transit_time: f64,
    ) -> Self {
        Self::with_model_and_depletion(
            name,
            anode,
            cathode,
            saturation_current,
            thermal_voltage,
            emission_coefficient,
            breakdown_voltage,
            breakdown_current,
            junction_capacitance,
            transit_time,
            1.0,
            0.5,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_and_depletion(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
        saturation_current: f64,
        thermal_voltage: f64,
        emission_coefficient: f64,
        breakdown_voltage: Option<f64>,
        breakdown_current: f64,
        junction_capacitance: f64,
        transit_time: f64,
        junction_potential: f64,
        grading_coefficient: f64,
    ) -> Self {
        Self::with_model_and_forward_depletion(
            name,
            anode,
            cathode,
            saturation_current,
            thermal_voltage,
            emission_coefficient,
            breakdown_voltage,
            breakdown_current,
            junction_capacitance,
            transit_time,
            junction_potential,
            grading_coefficient,
            0.5,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_and_forward_depletion(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
        saturation_current: f64,
        thermal_voltage: f64,
        emission_coefficient: f64,
        breakdown_voltage: Option<f64>,
        breakdown_current: f64,
        junction_capacitance: f64,
        transit_time: f64,
        junction_potential: f64,
        grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
    ) -> Self {
        Self::with_model_and_temperature_exponent(
            name,
            anode,
            cathode,
            saturation_current,
            thermal_voltage,
            emission_coefficient,
            breakdown_voltage,
            breakdown_current,
            junction_capacitance,
            transit_time,
            junction_potential,
            grading_coefficient,
            forward_bias_depletion_coefficient,
            3.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_and_temperature_exponent(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
        saturation_current: f64,
        thermal_voltage: f64,
        emission_coefficient: f64,
        breakdown_voltage: Option<f64>,
        breakdown_current: f64,
        junction_capacitance: f64,
        transit_time: f64,
        junction_potential: f64,
        grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
        saturation_current_temperature_exponent: f64,
    ) -> Self {
        Self::with_model_and_temperature_parameters(
            name,
            anode,
            cathode,
            saturation_current,
            thermal_voltage,
            emission_coefficient,
            breakdown_voltage,
            breakdown_current,
            junction_capacitance,
            transit_time,
            junction_potential,
            grading_coefficient,
            forward_bias_depletion_coefficient,
            saturation_current_temperature_exponent,
            1.11,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_and_temperature_parameters(
        name: impl Into<String>,
        anode: impl Into<String>,
        cathode: impl Into<String>,
        saturation_current: f64,
        thermal_voltage: f64,
        emission_coefficient: f64,
        breakdown_voltage: Option<f64>,
        breakdown_current: f64,
        junction_capacitance: f64,
        transit_time: f64,
        junction_potential: f64,
        grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
    ) -> Self {
        Self {
            name: name.into(),
            anode: anode.into(),
            cathode: cathode.into(),
            saturation_current,
            thermal_voltage,
            emission_coefficient,
            breakdown_voltage,
            breakdown_current,
            junction_capacitance,
            transit_time,
            junction_potential,
            grading_coefficient,
            forward_bias_depletion_coefficient,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            series_resistance: 0.0,
            flicker_noise_coefficient: 0.0,
            flicker_noise_exponent: 1.0,
        }
    }
}

pub fn diode_at_temperature(
    diode: &Diode,
    temperature_kelvin: f64,
    nominal_temperature_kelvin: f64,
    energy_gap_electron_volts: f64,
) -> Result<Diode, SpiceError> {
    if !temperature_kelvin.is_finite() || temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "temperature must be finite and positive".to_string(),
        });
    }
    if !nominal_temperature_kelvin.is_finite() || nominal_temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "nominal temperature must be finite and positive".to_string(),
        });
    }
    if !energy_gap_electron_volts.is_finite() || energy_gap_electron_volts <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "energy gap must be finite and positive".to_string(),
        });
    }
    if !diode.emission_coefficient.is_finite() || diode.emission_coefficient <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "emission coefficient must be finite and positive".to_string(),
        });
    }
    if !diode.saturation_current_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "saturation-current temperature exponent must be finite".to_string(),
        });
    }
    let ratio = temperature_kelvin / nominal_temperature_kelvin;
    let exponent = energy_gap_electron_volts * ELECTRON_CHARGE
        / (diode.emission_coefficient * BOLTZMANN)
        * (1.0 / nominal_temperature_kelvin - 1.0 / temperature_kelvin);
    let saturation_scale = ratio.powf(diode.saturation_current_temperature_exponent)
        * exponent.clamp(-100.0, 100.0).exp();
    let mut adjusted = diode.clone();
    adjusted.saturation_current *= saturation_scale;
    adjusted.thermal_voltage *= ratio;
    Ok(adjusted)
}

pub fn bjt_at_temperature(
    bjt: &Bjt,
    temperature_kelvin: f64,
    nominal_temperature_kelvin: f64,
    energy_gap_electron_volts: f64,
) -> Result<Bjt, SpiceError> {
    if !temperature_kelvin.is_finite() || temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "temperature must be finite and positive".to_string(),
        });
    }
    let nominal_temperature_kelvin = bjt
        .nominal_temperature_kelvin
        .unwrap_or(nominal_temperature_kelvin);
    if !nominal_temperature_kelvin.is_finite() || nominal_temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "nominal temperature must be finite and positive".to_string(),
        });
    }
    if !energy_gap_electron_volts.is_finite() || energy_gap_electron_volts <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "energy gap must be finite and positive".to_string(),
        });
    }
    let ratio = temperature_kelvin / nominal_temperature_kelvin;
    let exponent = energy_gap_electron_volts * ELECTRON_CHARGE / BOLTZMANN
        * (1.0 / nominal_temperature_kelvin - 1.0 / temperature_kelvin);
    if !bjt.saturation_current_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "saturation-current temperature exponent must be finite".to_string(),
        });
    }
    if !bjt.forward_beta_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "beta temperature exponent must be finite".to_string(),
        });
    }
    if bjt.reverse_beta.is_nan() || bjt.reverse_beta <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "reverse beta must be positive".to_string(),
        });
    }
    let saturation_scale = ratio.powf(bjt.saturation_current_temperature_exponent)
        * exponent.clamp(-100.0, 100.0).exp();
    let mut adjusted = bjt.clone();
    adjusted.saturation_current *= saturation_scale;
    adjusted.base_emitter_leakage_saturation_current *= saturation_scale;
    adjusted.base_collector_leakage_saturation_current *= saturation_scale;
    adjusted.forward_beta *= ratio.powf(bjt.forward_beta_temperature_exponent);
    adjusted.reverse_beta *= ratio.powf(bjt.forward_beta_temperature_exponent);
    adjusted.thermal_voltage *= ratio;
    Ok(adjusted)
}

pub fn mosfet_at_temperature(
    mosfet: &Mosfet,
    temperature_kelvin: f64,
    nominal_temperature_kelvin: f64,
    energy_gap_electron_volts: f64,
) -> Result<Mosfet, SpiceError> {
    if !temperature_kelvin.is_finite() || temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "temperature must be finite and positive".to_string(),
        });
    }
    if !nominal_temperature_kelvin.is_finite() || nominal_temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "nominal temperature must be finite and positive".to_string(),
        });
    }
    let reference_temperature_kelvin = 300.15;
    let nominal_temperature = if mosfet.params.t_nom != reference_temperature_kelvin {
        mosfet.params.t_nom
    } else {
        nominal_temperature_kelvin
    };
    if !nominal_temperature.is_finite() || nominal_temperature <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "nominal temperature must be finite and positive".to_string(),
        });
    }
    if !energy_gap_electron_volts.is_finite() || energy_gap_electron_volts <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "energy gap must be finite and positive".to_string(),
        });
    }
    let ratio = temperature_kelvin / nominal_temperature;
    let potential_correction = |temperature: f64| {
        let thermal_voltage = BOLTZMANN * temperature / ELECTRON_CHARGE;
        let argument = -silicon_band_gap_electron_volts(temperature) * ELECTRON_CHARGE
            / (2.0 * BOLTZMANN * temperature)
            + 1.115_087_7 * ELECTRON_CHARGE / (2.0 * BOLTZMANN * reference_temperature_kelvin);
        -2.0 * thermal_voltage
            * (1.5 * (temperature / reference_temperature_kelvin).ln() + argument)
    };
    let nominal_factor = nominal_temperature / reference_temperature_kelvin;
    let temperature_factor = temperature_kelvin / reference_temperature_kelvin;
    let nominal_potential_correction = potential_correction(nominal_temperature);
    let temperature_potential_correction = potential_correction(temperature_kelvin);
    let nominal_phi = (mosfet.params.phi - nominal_potential_correction) / nominal_factor;
    let temperature_phi = temperature_factor * nominal_phi + temperature_potential_correction;
    let nominal_bulk_junction_potential =
        (mosfet.params.bulk_junction_potential - nominal_potential_correction) / nominal_factor;
    let temperature_bulk_junction_potential =
        temperature_factor * nominal_bulk_junction_potential + temperature_potential_correction;
    let nominal_bulk_potential_shift = (mosfet.params.bulk_junction_potential
        - nominal_bulk_junction_potential)
        / nominal_bulk_junction_potential;
    let temperature_bulk_potential_shift = (temperature_bulk_junction_potential
        - nominal_bulk_junction_potential)
        / nominal_bulk_junction_potential;
    let capacitance_scale = |grading_coefficient: f64| {
        let nominal_scale = 1.0
            / (1.0
                + grading_coefficient
                    * (4.0e-4 * (nominal_temperature - reference_temperature_kelvin)
                        - nominal_bulk_potential_shift));
        let temperature_scale = 1.0
            + grading_coefficient
                * (4.0e-4 * (temperature_kelvin - reference_temperature_kelvin)
                    - temperature_bulk_potential_shift);
        nominal_scale * temperature_scale
    };
    let bottom_capacitance_scale =
        capacitance_scale(mosfet.params.bulk_junction_grading_coefficient);
    let sidewall_capacitance_scale =
        capacitance_scale(mosfet.params.sidewall_junction_grading_coefficient);
    let polarity = match mosfet.mosfet_type {
        MosfetType::Nmos => 1.0,
        MosfetType::Pmos => -1.0,
    };
    let temperature_vbi = mosfet.params.vt0
        - polarity * mosfet.params.gamma * mosfet.params.phi.sqrt()
        + 0.5
            * (silicon_band_gap_electron_volts(nominal_temperature)
                - silicon_band_gap_electron_volts(temperature_kelvin))
        + polarity * 0.5 * (temperature_phi - mosfet.params.phi);
    let temperature_vt0 = temperature_vbi + polarity * mosfet.params.gamma * temperature_phi.sqrt();
    let saturation_exponent = energy_gap_electron_volts * ELECTRON_CHARGE / BOLTZMANN
        * (1.0 / nominal_temperature - 1.0 / temperature_kelvin);
    let saturation_scale = ratio.powi(3) * saturation_exponent.clamp(-100.0, 100.0).exp();
    let mut adjusted = mosfet.clone();
    adjusted.params.vt0 = temperature_vt0;
    adjusted.params.phi = temperature_phi;
    adjusted.params.bulk_junction_potential = temperature_bulk_junction_potential;
    adjusted.params.bottom_junction_capacitance *= bottom_capacitance_scale;
    adjusted.params.source_bulk_capacitance *= bottom_capacitance_scale;
    adjusted.params.drain_bulk_capacitance *= bottom_capacitance_scale;
    adjusted.params.sidewall_junction_capacitance *= sidewall_capacitance_scale;
    adjusted.params.kp *= ratio.powf(-1.5);
    adjusted.params.surface_mobility *= ratio.powf(-1.5);
    adjusted.params.saturation_current *= saturation_scale;
    adjusted.params.saturation_current_density *= saturation_scale;
    adjusted.params.t_nom = temperature_kelvin;
    Ok(adjusted)
}

pub fn jfet_at_temperature(
    jfet: &Jfet,
    temperature_kelvin: f64,
    nominal_temperature_kelvin: f64,
) -> Result<Jfet, SpiceError> {
    if !temperature_kelvin.is_finite() || temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "temperature must be finite and positive".to_string(),
        });
    }
    let nominal_temperature = jfet
        .nominal_temperature_kelvin
        .unwrap_or(nominal_temperature_kelvin);
    if !nominal_temperature.is_finite() || nominal_temperature <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "nominal temperature must be finite and positive".to_string(),
        });
    }
    if !jfet
        .gate_saturation_current_temperature_exponent
        .is_finite()
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "gate saturation-current temperature exponent must be finite".to_string(),
        });
    }
    if !jfet.bandgap_voltage.is_finite() || jfet.bandgap_voltage <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "bandgap voltage must be finite and positive".to_string(),
        });
    }
    if !jfet.doping_tail_parameter.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "doping-tail parameter must be finite".to_string(),
        });
    }
    if !jfet.noise_equation_level.is_finite()
        || jfet.noise_equation_level < 1.0
        || jfet.noise_equation_level.fract() != 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "noise equation level must be a finite integer greater than or equal to 1"
                .to_string(),
        });
    }
    if !jfet.channel_noise_coefficient.is_finite() || jfet.channel_noise_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "channel noise coefficient must be finite and non-negative".to_string(),
        });
    }
    if !jfet.threshold_voltage_temperature_coefficient.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "threshold-voltage temperature coefficient must be finite".to_string(),
        });
    }
    if jfet
        .alternative_threshold_voltage_temperature_coefficient
        .is_some_and(|coefficient| !coefficient.is_finite())
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "alternative threshold-voltage temperature coefficient must be finite"
                .to_string(),
        });
    }
    if !jfet.mobility_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "mobility temperature exponent must be finite".to_string(),
        });
    }
    if jfet
        .mobility_temperature_coefficient
        .is_some_and(|coefficient| !coefficient.is_finite())
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "mobility temperature coefficient must be finite".to_string(),
        });
    }
    let temperature_ratio = temperature_kelvin / nominal_temperature;
    let saturation_exponent = jfet.bandgap_voltage * ELECTRON_CHARGE / BOLTZMANN
        * (1.0 / nominal_temperature - 1.0 / temperature_kelvin);
    let saturation_scale = temperature_ratio
        .powf(jfet.gate_saturation_current_temperature_exponent)
        * saturation_exponent.clamp(-100.0, 100.0).exp();
    let beta_scale = jfet.mobility_temperature_coefficient.map_or_else(
        || temperature_ratio.powf(jfet.mobility_temperature_exponent),
        |coefficient| 1.01_f64.powf(coefficient * (temperature_kelvin - nominal_temperature)),
    );
    let mut adjusted = jfet.clone();
    adjusted.threshold_voltage = jfet
        .alternative_threshold_voltage_temperature_coefficient
        .map_or_else(
            || {
                jfet.threshold_voltage
                    - jfet.threshold_voltage_temperature_coefficient
                        * (temperature_kelvin - nominal_temperature)
            },
            |coefficient| {
                jfet.threshold_voltage + coefficient * (temperature_kelvin - nominal_temperature)
            },
        );
    adjusted.beta *= beta_scale;
    adjusted.gate_saturation_current *= saturation_scale;
    Ok(adjusted)
}

pub fn circuit_at_temperature(
    circuit: &Circuit,
    temperature_kelvin: f64,
    nominal_temperature_kelvin: f64,
    energy_gap_electron_volts: f64,
) -> Result<Circuit, SpiceError> {
    let mut adjusted = Circuit {
        elements: Vec::new(),
        subcircuits: circuit.subcircuits.clone(),
    };
    for element in circuit.elements() {
        adjusted.add(match element {
            Element::Diode(diode) => Element::Diode(diode_at_temperature(
                diode,
                temperature_kelvin,
                nominal_temperature_kelvin,
                diode.energy_gap_electron_volts,
            )?),
            Element::Bjt(bjt) => Element::Bjt(bjt_at_temperature(
                bjt,
                temperature_kelvin,
                nominal_temperature_kelvin,
                bjt.energy_gap_electron_volts,
            )?),
            Element::Jfet(jfet) => Element::Jfet(jfet_at_temperature(
                jfet,
                temperature_kelvin,
                nominal_temperature_kelvin,
            )?),
            Element::Mosfet(mosfet) => Element::Mosfet(mosfet_at_temperature(
                mosfet,
                temperature_kelvin,
                nominal_temperature_kelvin,
                energy_gap_electron_volts,
            )?),
            _ => element.clone(),
        });
    }
    Ok(adjusted)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum JfetPolarity {
    Njf,
    Pjf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Jfet {
    pub name: String,
    pub drain: String,
    pub gate: String,
    pub source: String,
    pub polarity: JfetPolarity,
    pub beta: f64,
    pub threshold_voltage: f64,
    pub channel_length_modulation: f64,
    pub gate_source_capacitance: f64,
    pub gate_drain_capacitance: f64,
    pub flicker_noise_coefficient: f64,
    pub flicker_noise_exponent: f64,
    pub junction_potential: f64,
    pub forward_bias_depletion_coefficient: f64,
    pub gate_saturation_current: f64,
    pub gate_saturation_current_temperature_exponent: f64,
    pub bandgap_voltage: f64,
    pub doping_tail_parameter: f64,
    pub noise_equation_level: f64,
    pub channel_noise_coefficient: f64,
    pub drain_resistance: f64,
    pub source_resistance: f64,
    pub threshold_voltage_temperature_coefficient: f64,
    pub alternative_threshold_voltage_temperature_coefficient: Option<f64>,
    pub nominal_temperature_kelvin: Option<f64>,
    pub mobility_temperature_exponent: f64,
    pub mobility_temperature_coefficient: Option<f64>,
}

impl Jfet {
    pub fn new(
        name: impl Into<String>,
        drain: impl Into<String>,
        gate: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::with_model(
            name,
            drain,
            gate,
            source,
            JfetPolarity::Njf,
            1.0e-4,
            -2.0,
            0.0,
        )
    }

    pub fn with_model(
        name: impl Into<String>,
        drain: impl Into<String>,
        gate: impl Into<String>,
        source: impl Into<String>,
        polarity: JfetPolarity,
        beta: f64,
        threshold_voltage: f64,
        channel_length_modulation: f64,
    ) -> Self {
        Self::with_model_and_capacitance(
            name,
            drain,
            gate,
            source,
            polarity,
            beta,
            threshold_voltage,
            channel_length_modulation,
            0.0,
            0.0,
        )
    }

    pub fn with_model_and_capacitance(
        name: impl Into<String>,
        drain: impl Into<String>,
        gate: impl Into<String>,
        source: impl Into<String>,
        polarity: JfetPolarity,
        beta: f64,
        threshold_voltage: f64,
        channel_length_modulation: f64,
        gate_source_capacitance: f64,
        gate_drain_capacitance: f64,
    ) -> Self {
        Self {
            name: name.into(),
            drain: drain.into(),
            gate: gate.into(),
            source: source.into(),
            polarity,
            beta,
            threshold_voltage,
            channel_length_modulation,
            gate_source_capacitance,
            gate_drain_capacitance,
            flicker_noise_coefficient: 0.0,
            flicker_noise_exponent: 1.0,
            junction_potential: 1.0,
            forward_bias_depletion_coefficient: 0.5,
            gate_saturation_current: 1.0e-14,
            gate_saturation_current_temperature_exponent: 3.0,
            bandgap_voltage: 1.11,
            doping_tail_parameter: 1.0,
            noise_equation_level: 1.0,
            channel_noise_coefficient: 1.0,
            drain_resistance: 0.0,
            source_resistance: 0.0,
            threshold_voltage_temperature_coefficient: 0.0,
            alternative_threshold_voltage_temperature_coefficient: None,
            nominal_temperature_kelvin: None,
            mobility_temperature_exponent: 0.0,
            mobility_temperature_coefficient: None,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BjtPolarity {
    Npn,
    Pnp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bjt {
    pub name: String,
    pub collector: String,
    pub base: String,
    pub emitter: String,
    pub polarity: BjtPolarity,
    pub saturation_current: f64,
    pub forward_beta: f64,
    pub thermal_voltage: f64,
    pub base_emitter_capacitance: f64,
    pub base_collector_capacitance: f64,
    pub forward_transit_time: f64,
    pub reverse_transit_time: f64,
    pub saturation_current_temperature_exponent: f64,
    pub energy_gap_electron_volts: f64,
    pub forward_early_voltage: f64,
    pub reverse_early_voltage: f64,
    pub forward_emission_coefficient: f64,
    pub reverse_emission_coefficient: f64,
    pub base_emitter_junction_potential: f64,
    pub base_emitter_grading_coefficient: f64,
    pub base_collector_junction_potential: f64,
    pub base_collector_grading_coefficient: f64,
    pub forward_bias_depletion_coefficient: f64,
    pub forward_beta_rolloff_current: f64,
    pub base_emitter_leakage_saturation_current: f64,
    pub base_emitter_leakage_emission_coefficient: f64,
    pub base_collector_leakage_saturation_current: f64,
    pub base_collector_leakage_emission_coefficient: f64,
    pub forward_beta_temperature_exponent: f64,
    pub reverse_beta: f64,
    pub reverse_beta_rolloff_current: f64,
    pub nominal_temperature_kelvin: Option<f64>,
    pub flicker_noise_coefficient: f64,
    pub flicker_noise_exponent: f64,
    pub forward_excess_phase_degrees: f64,
    pub forward_transit_time_bias_coefficient: f64,
    pub forward_transit_time_current: f64,
    pub forward_transit_time_voltage: f64,
    pub emitter_resistance: f64,
    pub collector_resistance: f64,
    pub base_resistance: f64,
    pub minimum_base_resistance: Option<f64>,
    pub base_resistance_half_current: f64,
    pub base_collector_capacitance_fraction: f64,
}

impl Bjt {
    pub fn new(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
    ) -> Self {
        Self::with_model(
            name,
            collector,
            base,
            emitter,
            BjtPolarity::Npn,
            1.0e-14,
            100.0,
            0.02585,
            0.0,
            0.0,
            0.0,
            0.0,
        )
    }

    pub fn with_model(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
    ) -> Self {
        Self::with_model_and_temperature_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            3.0,
            1.11,
            0.0,
            1.0,
            1.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_and_temperature_parameters(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
        forward_early_voltage: f64,
        forward_emission_coefficient: f64,
        reverse_emission_coefficient: f64,
    ) -> Self {
        Self::with_model_temperature_and_depletion_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            forward_early_voltage,
            forward_emission_coefficient,
            reverse_emission_coefficient,
            0.75,
            0.33,
            0.75,
            0.33,
            0.5,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_temperature_and_depletion_parameters(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
        forward_early_voltage: f64,
        forward_emission_coefficient: f64,
        reverse_emission_coefficient: f64,
        base_emitter_junction_potential: f64,
        base_emitter_grading_coefficient: f64,
        base_collector_junction_potential: f64,
        base_collector_grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
    ) -> Self {
        Self::with_model_temperature_depletion_and_early_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            forward_early_voltage,
            forward_emission_coefficient,
            reverse_emission_coefficient,
            base_emitter_junction_potential,
            base_emitter_grading_coefficient,
            base_collector_junction_potential,
            base_collector_grading_coefficient,
            forward_bias_depletion_coefficient,
            0.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_temperature_depletion_and_early_parameters(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
        forward_early_voltage: f64,
        forward_emission_coefficient: f64,
        reverse_emission_coefficient: f64,
        base_emitter_junction_potential: f64,
        base_emitter_grading_coefficient: f64,
        base_collector_junction_potential: f64,
        base_collector_grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
        reverse_early_voltage: f64,
    ) -> Self {
        Self::with_model_temperature_depletion_early_and_rolloff_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            forward_early_voltage,
            forward_emission_coefficient,
            reverse_emission_coefficient,
            base_emitter_junction_potential,
            base_emitter_grading_coefficient,
            base_collector_junction_potential,
            base_collector_grading_coefficient,
            forward_bias_depletion_coefficient,
            reverse_early_voltage,
            0.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_temperature_depletion_early_and_rolloff_parameters(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
        forward_early_voltage: f64,
        forward_emission_coefficient: f64,
        reverse_emission_coefficient: f64,
        base_emitter_junction_potential: f64,
        base_emitter_grading_coefficient: f64,
        base_collector_junction_potential: f64,
        base_collector_grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
        reverse_early_voltage: f64,
        forward_beta_rolloff_current: f64,
    ) -> Self {
        Self::with_model_temperature_depletion_early_rolloff_and_leakage_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            forward_early_voltage,
            forward_emission_coefficient,
            reverse_emission_coefficient,
            base_emitter_junction_potential,
            base_emitter_grading_coefficient,
            base_collector_junction_potential,
            base_collector_grading_coefficient,
            forward_bias_depletion_coefficient,
            reverse_early_voltage,
            forward_beta_rolloff_current,
            0.0,
            1.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_temperature_depletion_early_rolloff_and_leakage_parameters(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
        forward_early_voltage: f64,
        forward_emission_coefficient: f64,
        reverse_emission_coefficient: f64,
        base_emitter_junction_potential: f64,
        base_emitter_grading_coefficient: f64,
        base_collector_junction_potential: f64,
        base_collector_grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
        reverse_early_voltage: f64,
        forward_beta_rolloff_current: f64,
        base_emitter_leakage_saturation_current: f64,
        base_emitter_leakage_emission_coefficient: f64,
    ) -> Self {
        Self::with_model_temperature_depletion_early_rolloff_and_junction_leakage_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            forward_early_voltage,
            forward_emission_coefficient,
            reverse_emission_coefficient,
            base_emitter_junction_potential,
            base_emitter_grading_coefficient,
            base_collector_junction_potential,
            base_collector_grading_coefficient,
            forward_bias_depletion_coefficient,
            reverse_early_voltage,
            forward_beta_rolloff_current,
            base_emitter_leakage_saturation_current,
            base_emitter_leakage_emission_coefficient,
            0.0,
            2.0,
            0.0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_temperature_depletion_early_rolloff_and_junction_leakage_parameters(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
        forward_early_voltage: f64,
        forward_emission_coefficient: f64,
        reverse_emission_coefficient: f64,
        base_emitter_junction_potential: f64,
        base_emitter_grading_coefficient: f64,
        base_collector_junction_potential: f64,
        base_collector_grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
        reverse_early_voltage: f64,
        forward_beta_rolloff_current: f64,
        base_emitter_leakage_saturation_current: f64,
        base_emitter_leakage_emission_coefficient: f64,
        base_collector_leakage_saturation_current: f64,
        base_collector_leakage_emission_coefficient: f64,
        forward_beta_temperature_exponent: f64,
    ) -> Self {
        Self::with_model_temperature_depletion_early_rolloff_junction_leakage_and_reverse_beta_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            forward_early_voltage,
            forward_emission_coefficient,
            reverse_emission_coefficient,
            base_emitter_junction_potential,
            base_emitter_grading_coefficient,
            base_collector_junction_potential,
            base_collector_grading_coefficient,
            forward_bias_depletion_coefficient,
            reverse_early_voltage,
            forward_beta_rolloff_current,
            base_emitter_leakage_saturation_current,
            base_emitter_leakage_emission_coefficient,
            base_collector_leakage_saturation_current,
            base_collector_leakage_emission_coefficient,
            forward_beta_temperature_exponent,
            f64::INFINITY,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_model_temperature_depletion_early_rolloff_junction_leakage_and_reverse_beta_parameters(
        name: impl Into<String>,
        collector: impl Into<String>,
        base: impl Into<String>,
        emitter: impl Into<String>,
        polarity: BjtPolarity,
        saturation_current: f64,
        forward_beta: f64,
        thermal_voltage: f64,
        base_emitter_capacitance: f64,
        base_collector_capacitance: f64,
        forward_transit_time: f64,
        reverse_transit_time: f64,
        saturation_current_temperature_exponent: f64,
        energy_gap_electron_volts: f64,
        forward_early_voltage: f64,
        forward_emission_coefficient: f64,
        reverse_emission_coefficient: f64,
        base_emitter_junction_potential: f64,
        base_emitter_grading_coefficient: f64,
        base_collector_junction_potential: f64,
        base_collector_grading_coefficient: f64,
        forward_bias_depletion_coefficient: f64,
        reverse_early_voltage: f64,
        forward_beta_rolloff_current: f64,
        base_emitter_leakage_saturation_current: f64,
        base_emitter_leakage_emission_coefficient: f64,
        base_collector_leakage_saturation_current: f64,
        base_collector_leakage_emission_coefficient: f64,
        forward_beta_temperature_exponent: f64,
        reverse_beta: f64,
    ) -> Self {
        Self {
            name: name.into(),
            collector: collector.into(),
            base: base.into(),
            emitter: emitter.into(),
            polarity,
            saturation_current,
            forward_beta,
            thermal_voltage,
            base_emitter_capacitance,
            base_collector_capacitance,
            forward_transit_time,
            reverse_transit_time,
            saturation_current_temperature_exponent,
            energy_gap_electron_volts,
            forward_early_voltage,
            forward_emission_coefficient,
            reverse_emission_coefficient,
            base_emitter_junction_potential,
            base_emitter_grading_coefficient,
            base_collector_junction_potential,
            base_collector_grading_coefficient,
            forward_bias_depletion_coefficient,
            reverse_early_voltage,
            forward_beta_rolloff_current,
            base_emitter_leakage_saturation_current,
            base_emitter_leakage_emission_coefficient,
            base_collector_leakage_saturation_current,
            base_collector_leakage_emission_coefficient,
            forward_beta_temperature_exponent,
            reverse_beta,
            reverse_beta_rolloff_current: 0.0,
            nominal_temperature_kelvin: None,
            flicker_noise_coefficient: 0.0,
            flicker_noise_exponent: 1.0,
            forward_excess_phase_degrees: 0.0,
            forward_transit_time_bias_coefficient: 0.0,
            forward_transit_time_current: 0.0,
            forward_transit_time_voltage: 0.0,
            emitter_resistance: 0.0,
            collector_resistance: 0.0,
            base_resistance: 0.0,
            minimum_base_resistance: None,
            base_resistance_half_current: 0.0,
            base_collector_capacitance_fraction: 1.0,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MosfetType {
    Nmos,
    Pmos,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MosfetLevel1Params {
    pub vt0: f64,
    pub kp: f64,
    pub lambda: f64,
    pub gamma: f64,
    pub phi: f64,
    pub w: f64,
    pub l: f64,
    pub lateral_diffusion_length: f64,
    pub oxide_thickness: f64,
    pub surface_mobility: f64,
    pub drain_resistance: f64,
    pub source_resistance: f64,
    pub sheet_resistance: f64,
    pub drain_squares: f64,
    pub source_squares: f64,
    pub drain_area: f64,
    pub source_area: f64,
    pub drain_perimeter: f64,
    pub source_perimeter: f64,
    pub bottom_junction_capacitance: f64,
    pub sidewall_junction_capacitance: f64,
    pub saturation_current: f64,
    pub saturation_current_density: f64,
    pub n_sub: f64,
    pub t_nom: f64,
    pub gate_source_overlap_capacitance: f64,
    pub gate_drain_overlap_capacitance: f64,
    pub gate_bulk_overlap_capacitance: f64,
    pub source_bulk_capacitance: f64,
    pub drain_bulk_capacitance: f64,
    pub bulk_junction_potential: f64,
    pub bulk_junction_grading_coefficient: f64,
    pub sidewall_junction_grading_coefficient: f64,
    pub forward_bias_depletion_coefficient: f64,
    pub flicker_noise_coefficient: f64,
    pub flicker_noise_exponent: f64,
}

impl Default for MosfetLevel1Params {
    fn default() -> Self {
        Self {
            vt0: 0.42,
            kp: 220.0e-6,
            lambda: 0.05,
            gamma: 0.27,
            phi: 0.84,
            w: 1.0e-6,
            l: 130.0e-9,
            lateral_diffusion_length: 0.0,
            oxide_thickness: 1.0e-7,
            surface_mobility: 600.0,
            drain_resistance: 0.0,
            source_resistance: 0.0,
            sheet_resistance: 0.0,
            drain_squares: 1.0,
            source_squares: 1.0,
            drain_area: 0.0,
            source_area: 0.0,
            drain_perimeter: 0.0,
            source_perimeter: 0.0,
            bottom_junction_capacitance: 0.0,
            sidewall_junction_capacitance: 0.0,
            saturation_current: 1.0e-15,
            saturation_current_density: 0.0,
            n_sub: 1.4,
            t_nom: 300.15,
            gate_source_overlap_capacitance: 0.0,
            gate_drain_overlap_capacitance: 0.0,
            gate_bulk_overlap_capacitance: 0.0,
            source_bulk_capacitance: 0.0,
            drain_bulk_capacitance: 0.0,
            bulk_junction_potential: 0.8,
            bulk_junction_grading_coefficient: 0.5,
            sidewall_junction_grading_coefficient: 0.33,
            forward_bias_depletion_coefficient: 0.5,
            flicker_noise_coefficient: 0.0,
            flicker_noise_exponent: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mosfet {
    pub name: String,
    pub drain: String,
    pub gate: String,
    pub source: String,
    pub body: String,
    pub mosfet_type: MosfetType,
    pub params: MosfetLevel1Params,
}

impl Mosfet {
    pub fn new(
        name: impl Into<String>,
        drain: impl Into<String>,
        gate: impl Into<String>,
        source: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::with_model(
            name,
            drain,
            gate,
            source,
            body,
            MosfetType::Nmos,
            MosfetLevel1Params::default(),
        )
    }

    pub fn with_model(
        name: impl Into<String>,
        drain: impl Into<String>,
        gate: impl Into<String>,
        source: impl Into<String>,
        body: impl Into<String>,
        mosfet_type: MosfetType,
        params: MosfetLevel1Params,
    ) -> Self {
        Self {
            name: name.into(),
            drain: drain.into(),
            gate: gate.into(),
            source: source.into(),
            body: body.into(),
            mosfet_type,
            params,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelCardKind {
    Diode,
    Npn,
    Pnp,
    Njf,
    Pjf,
    Nmos,
    Pmos,
}

impl ModelCardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Diode => "D",
            Self::Npn => "NPN",
            Self::Pnp => "PNP",
            Self::Njf => "NJF",
            Self::Pjf => "PJF",
            Self::Nmos => "NMOS",
            Self::Pmos => "PMOS",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedModelCard {
    pub name: String,
    pub kind: ModelCardKind,
    pub parameters: BTreeMap<String, f64>,
    pub unsupported_parameters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCardUnsupportedParameterIssue {
    pub model_name: String,
    pub kind: ModelCardKind,
    pub parameter: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCardSupportedParameterCoverage {
    pub kind: ModelCardKind,
    pub canonical_parameter: String,
    pub accepted_names: Vec<String>,
    pub alias_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCardSupportedParameterCoverageSummary {
    pub kind: ModelCardKind,
    pub canonical_parameter_count: usize,
    pub accepted_name_count: usize,
    pub aliased_parameter_count: usize,
    pub max_alias_count: usize,
    pub aliased_parameters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCardSupportedParameterCoverageGateIssue {
    pub kind: String,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCardSupportedParameterCoverageGateReport {
    pub passed: bool,
    pub kind_count: usize,
    pub expected_kind_count: usize,
    pub canonical_parameter_count: usize,
    pub expected_canonical_parameter_count: usize,
    pub accepted_name_count: usize,
    pub aliased_parameter_count: usize,
    pub max_alias_count: usize,
    pub issues: Vec<ModelCardSupportedParameterCoverageGateIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCardSupportedParameterCoverageDashboardRow {
    pub kind: ModelCardKind,
    pub passed: bool,
    pub canonical_parameter_count: usize,
    pub expected_canonical_parameter_count: usize,
    pub accepted_name_count: usize,
    pub expected_accepted_name_count: usize,
    pub aliased_parameter_count: usize,
    pub expected_aliased_parameter_count: usize,
    pub max_alias_count: usize,
    pub expected_max_alias_count: usize,
    pub issue_count: usize,
    pub issue_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceModelBehaviorFixture {
    pub name: String,
    pub kind: ModelCardKind,
    pub model: NormalizedModelCard,
    pub circuit: Circuit,
    pub probe_node: String,
    pub expected_min: f64,
    pub expected_max: f64,
    pub deck_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceModelTemperaturePoint {
    pub temperature_kelvin: f64,
    pub expected_min: f64,
    pub expected_max: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceModelTemperatureBehaviorFixture {
    pub name: String,
    pub kind: ModelCardKind,
    pub model: NormalizedModelCard,
    pub circuit: Circuit,
    pub probe_node: String,
    pub nominal_temperature_kelvin: f64,
    pub energy_gap_electron_volts: f64,
    pub temperature_behavior: String,
    pub temperature_points: Vec<DeviceModelTemperaturePoint>,
    pub deck_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceModelCapacitanceBehaviorFixture {
    pub name: String,
    pub kind: ModelCardKind,
    pub model: NormalizedModelCard,
    pub circuit: Circuit,
    pub probe_node: String,
    pub frequency_hz: f64,
    pub expected_magnitude_min: f64,
    pub expected_magnitude_max: f64,
    pub capacitance_behavior: String,
    pub deck_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceModelNoiseBehaviorFixture {
    pub name: String,
    pub kind: ModelCardKind,
    pub model: NormalizedModelCard,
    pub circuit: Circuit,
    pub output_node: String,
    pub input_source: String,
    pub frequency_hz: f64,
    pub expected_noise_element: String,
    pub expected_noise_type: NoiseType,
    pub expected_source_psd_min: f64,
    pub expected_source_psd_max: f64,
    pub expected_output_psd_min: f64,
    pub expected_output_psd_max: f64,
    pub noise_behavior: String,
    pub deck_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceModelChargeBehaviorFixture {
    pub name: String,
    pub kind: ModelCardKind,
    pub model: NormalizedModelCard,
    pub circuit: Circuit,
    pub probe_node: String,
    pub time_step_s: f64,
    pub stop_time_s: f64,
    pub storage_capacitance_f: f64,
    pub expected_initial_min: f64,
    pub expected_initial_max: f64,
    pub expected_final_min: f64,
    pub expected_final_max: f64,
    pub charge_behavior: String,
    pub deck_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceModelReferenceDeckAuditFixture {
    pub name: String,
    pub kind: ModelCardKind,
    pub model: NormalizedModelCard,
    pub analysis: String,
    pub reference: String,
    pub expected_behavior: String,
    pub deck_lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModelReferenceDeckAuditIssue {
    pub fixture_name: String,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModelReferenceDeckAuditGateReport {
    pub passed: bool,
    pub fixture_count: usize,
    pub expected_kinds: Vec<String>,
    pub expected_analyses: Vec<String>,
    pub issues: Vec<DeviceModelReferenceDeckAuditIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModelReferenceDeckAuditGateCoverageDigest {
    pub passed: bool,
    pub fixture_count: usize,
    pub expected_pair_count: usize,
    pub covered_pair_count: usize,
    pub missing_pair_count: usize,
    pub issue_count: usize,
    pub issue_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModelReferenceDeckAuditGateIssueSummary {
    pub field: String,
    pub issue_count: usize,
    pub fixture_names: Vec<String>,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModelReferenceDeckAuditSummary {
    pub kind: String,
    pub fixture_count: usize,
    pub analyses: Vec<String>,
    pub missing_analyses: Vec<String>,
    pub deck_line_count: usize,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModelReferenceDeckAuditAnalysisSummary {
    pub analysis: String,
    pub fixture_count: usize,
    pub kinds: Vec<String>,
    pub missing_kinds: Vec<String>,
    pub deck_line_count: usize,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceModelReferenceDeckAuditMatrixRow {
    pub kind: String,
    pub fixture_count: usize,
    pub op: String,
    pub temperature: String,
    pub ac: String,
    pub noise: String,
    pub tran: String,
    pub missing_analyses: Vec<String>,
    pub extra_analyses: Vec<String>,
    pub deck_line_count: usize,
}

const REFERENCE_DECK_AUDIT_EXPECTED_KINDS: &[ModelCardKind] = &[
    ModelCardKind::Diode,
    ModelCardKind::Npn,
    ModelCardKind::Njf,
    ModelCardKind::Nmos,
];
const REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES: &[&str] =
    &["op", "temperature", "ac", "noise", "tran"];
const MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS: &[ModelCardKind] = &[
    ModelCardKind::Diode,
    ModelCardKind::Npn,
    ModelCardKind::Pnp,
    ModelCardKind::Njf,
    ModelCardKind::Pjf,
    ModelCardKind::Nmos,
    ModelCardKind::Pmos,
];
const MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_EXPECTED_SUMMARIES: &[(
    ModelCardKind,
    usize,
    usize,
    usize,
    usize,
)] = &[
    (ModelCardKind::Diode, 15, 21, 5, 3),
    (ModelCardKind::Npn, 41, 58, 13, 4),
    (ModelCardKind::Pnp, 41, 58, 13, 4),
    (ModelCardKind::Njf, 22, 30, 7, 3),
    (ModelCardKind::Pjf, 22, 30, 7, 3),
    (ModelCardKind::Nmos, 33, 41, 7, 3),
    (ModelCardKind::Pmos, 33, 41, 7, 3),
];
const DIODE_PARAMETER_ALIAS_ENTRIES: &[(&str, &str)] = &[
    ("IS", "IS"),
    ("JS", "IS"),
    ("VT", "VT"),
    ("V_T", "VT"),
    ("N", "N"),
    ("BV", "BV"),
    ("IBV", "IBV"),
    ("CJO", "CJO"),
    ("CJ", "CJO"),
    ("CJ0", "CJO"),
    ("TT", "TT"),
    ("VJ", "VJ"),
    ("PB", "VJ"),
    ("M", "M"),
    ("MJ", "M"),
    ("FC", "FC"),
    ("XTI", "XTI"),
    ("EG", "EG"),
    ("RS", "RS"),
    ("KF", "KF"),
    ("AF", "AF"),
];
const BJT_PARAMETER_ALIAS_ENTRIES: &[(&str, &str)] = &[
    ("IS", "IS"),
    ("BF", "BF"),
    ("BETA", "BF"),
    ("BETA_F", "BF"),
    ("HFE", "BF"),
    ("VT", "VT"),
    ("V_T", "VT"),
    ("CJE", "CJE"),
    ("CJE0", "CJE"),
    ("CBE", "CJE"),
    ("CJC", "CJC"),
    ("CJC0", "CJC"),
    ("CBC", "CJC"),
    ("TF", "TF"),
    ("TR", "TR"),
    ("XTI", "XTI"),
    ("EG", "EG"),
    ("VAF", "VAF"),
    ("VA", "VAF"),
    ("VAR", "VAR"),
    ("VB", "VAR"),
    ("IKF", "IKF"),
    ("IK", "IKF"),
    ("IKR", "IKR"),
    ("TNOM", "TNOM"),
    ("T_NOM", "TNOM"),
    ("KF", "KF"),
    ("AF", "AF"),
    ("PTF", "PTF"),
    ("XTF", "XTF"),
    ("ITF", "ITF"),
    ("VTF", "VTF"),
    ("RE", "RE"),
    ("RC", "RC"),
    ("RB", "RB"),
    ("RBM", "RBM"),
    ("IRB", "IRB"),
    ("XCJC", "XCJC"),
    ("ISE", "ISE"),
    ("C2", "C2"),
    ("NE", "NE"),
    ("ISC", "ISC"),
    ("C4", "C4"),
    ("NC", "NC"),
    ("XTB", "XTB"),
    ("BR", "BR"),
    ("BETA_R", "BR"),
    ("NF", "NF"),
    ("NR", "NR"),
    ("VJE", "VJE"),
    ("PE", "VJE"),
    ("MJE", "MJE"),
    ("ME", "MJE"),
    ("VJC", "VJC"),
    ("PC", "VJC"),
    ("MJC", "MJC"),
    ("MC", "MJC"),
    ("FC", "FC"),
];
const JFET_PARAMETER_ALIAS_ENTRIES: &[(&str, &str)] = &[
    ("BETA", "BETA"),
    ("BET", "BETA"),
    ("VTO", "VTO"),
    ("VT0", "VTO"),
    ("VTH", "VTO"),
    ("LAMBDA", "LAMBDA"),
    ("LAM", "LAMBDA"),
    ("CGS", "CGS"),
    ("CGS0", "CGS"),
    ("CGD", "CGD"),
    ("CGD0", "CGD"),
    ("KF", "KF"),
    ("AF", "AF"),
    ("PB", "PB"),
    ("VJ", "PB"),
    ("FC", "FC"),
    ("IS", "IS"),
    ("XTI", "XTI"),
    ("EG", "EG"),
    ("B", "B"),
    ("NLEV", "NLEV"),
    ("GDSNOI", "GDSNOI"),
    ("RD", "RD"),
    ("RS", "RS"),
    ("TNOM", "TNOM"),
    ("T_NOM", "TNOM"),
    ("TCV", "TCV"),
    ("VTOTC", "VTOTC"),
    ("BEX", "BEX"),
    ("BETATCE", "BETATCE"),
];
const MOS_LEVEL1_PARAMETER_ALIAS_ENTRIES: &[(&str, &str)] = &[
    ("LEVEL", "LEVEL"),
    ("VT0", "VT0"),
    ("VTO", "VT0"),
    ("VTH", "VT0"),
    ("KP", "KP"),
    ("LAMBDA", "LAMBDA"),
    ("LAM", "LAMBDA"),
    ("GAMMA", "GAMMA"),
    ("PHI", "PHI"),
    ("W", "W"),
    ("L", "L"),
    ("LD", "LD"),
    ("TOX", "TOX"),
    ("U0", "U0"),
    ("UO", "U0"),
    ("RD", "RD"),
    ("RS", "RS"),
    ("RSH", "RSH"),
    ("IS", "IS"),
    ("JS", "JS"),
    ("NSUB", "N_SUB"),
    ("N_SUB", "N_SUB"),
    ("NSS", "NSS"),
    ("TPG", "TPG"),
    ("TNOM", "T_NOM"),
    ("T_NOM", "T_NOM"),
    ("CGSO", "CGSO"),
    ("CGDO", "CGDO"),
    ("CGBO", "CGBO"),
    ("CBS", "CBS"),
    ("CJS", "CBS"),
    ("CBD", "CBD"),
    ("CJD", "CBD"),
    ("CJ", "CJ"),
    ("CJSW", "CJSW"),
    ("PB", "PB"),
    ("MJ", "MJ"),
    ("MJSW", "MJSW"),
    ("FC", "FC"),
    ("KF", "KF"),
    ("AF", "AF"),
];

fn model_type_key(text: &str) -> String {
    text.trim()
        .chars()
        .filter(|character| *character != '-' && *character != '_')
        .flat_map(char::to_uppercase)
        .collect()
}

fn parameter_key(text: &str) -> String {
    text.trim()
        .chars()
        .map(|character| {
            if character == '-' {
                '_'
            } else {
                character.to_ascii_uppercase()
            }
        })
        .collect()
}

pub fn normalize_model_card_type(model_type: &str) -> Result<ModelCardKind, SpiceError> {
    match model_type_key(model_type).as_str() {
        "D" | "DIODE" => Ok(ModelCardKind::Diode),
        "NPN" => Ok(ModelCardKind::Npn),
        "PNP" => Ok(ModelCardKind::Pnp),
        "NJF" | "NJFET" | "NJ" => Ok(ModelCardKind::Njf),
        "PJF" | "PJFET" | "PJ" => Ok(ModelCardKind::Pjf),
        "NMOS" | "NCH" => Ok(ModelCardKind::Nmos),
        "PMOS" | "PCH" => Ok(ModelCardKind::Pmos),
        _ => Err(SpiceError::InvalidElement {
            name: model_type.to_string(),
            reason: "unsupported SPICE model type".to_string(),
        }),
    }
}

fn model_card_parameter_alias_entries(
    kind: ModelCardKind,
) -> &'static [(&'static str, &'static str)] {
    match kind {
        ModelCardKind::Diode => DIODE_PARAMETER_ALIAS_ENTRIES,
        ModelCardKind::Npn | ModelCardKind::Pnp => BJT_PARAMETER_ALIAS_ENTRIES,
        ModelCardKind::Njf | ModelCardKind::Pjf => JFET_PARAMETER_ALIAS_ENTRIES,
        ModelCardKind::Nmos | ModelCardKind::Pmos => MOS_LEVEL1_PARAMETER_ALIAS_ENTRIES,
    }
}

fn model_card_parameter_alias(kind: ModelCardKind, key: &str) -> Option<&'static str> {
    model_card_parameter_alias_entries(kind)
        .iter()
        .find_map(|&(accepted_name, canonical)| (accepted_name == key).then_some(canonical))
}

pub fn normalize_model_card(
    name: impl Into<String>,
    model_type: &str,
    parameters: &[(&str, f64)],
) -> Result<NormalizedModelCard, SpiceError> {
    let name = name.into();
    let kind = normalize_model_card_type(model_type)?;
    let mut normalized = BTreeMap::new();
    let mut unsupported = Vec::new();
    for (raw_name, raw_value) in parameters {
        let key = parameter_key(raw_name);
        if let Some(canonical) = model_card_parameter_alias(kind, &key) {
            if canonical == "LEVEL" {
                if (raw_value - 1.0).abs() > 1.0e-12 {
                    return Err(SpiceError::InvalidElement {
                        name: name.clone(),
                        reason: "only MOS LEVEL=1 model cards are supported".to_string(),
                    });
                }
                normalized.insert(canonical.to_string(), 1.0);
            } else if canonical == "TPG" && !matches!(*raw_value, -1.0 | 0.0 | 1.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET TPG must be -1, 0, or 1".to_string(),
                });
            } else if canonical == "NSS" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET NSS must be finite and non-negative".to_string(),
                });
            } else if canonical == "T_NOM" && (!raw_value.is_finite() || *raw_value <= 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET TNOM must be finite and positive".to_string(),
                });
            } else if canonical == "N_SUB" && (!raw_value.is_finite() || *raw_value <= 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET NSUB must be finite and positive".to_string(),
                });
            } else if canonical == "TOX" && (!raw_value.is_finite() || *raw_value <= 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET TOX must be finite and positive".to_string(),
                });
            } else if canonical == "U0" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET U0 must be finite and non-negative".to_string(),
                });
            } else if canonical == "KP" && (!raw_value.is_finite() || *raw_value <= 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET KP must be finite and positive".to_string(),
                });
            } else if canonical == "VT0" && !raw_value.is_finite() {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET VT0 must be finite".to_string(),
                });
            } else if canonical == "LAMBDA" && !raw_value.is_finite() {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET LAMBDA must be finite".to_string(),
                });
            } else if canonical == "PHI" && (!raw_value.is_finite() || *raw_value <= 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET PHI must be finite and positive".to_string(),
                });
            } else if canonical == "GAMMA" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET GAMMA must be finite and non-negative".to_string(),
                });
            } else if canonical == "PB" && (!raw_value.is_finite() || *raw_value <= 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET PB must be finite and positive".to_string(),
                });
            } else if canonical == "MJ" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET MJ must be finite and non-negative".to_string(),
                });
            } else if canonical == "FC"
                && (!raw_value.is_finite() || !(0.0..1.0).contains(raw_value))
            {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET FC must be finite and in [0, 1)".to_string(),
                });
            } else if canonical == "MJSW" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET MJSW must be finite and non-negative".to_string(),
                });
            } else if canonical == "CJ" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET CJ must be finite and non-negative".to_string(),
                });
            } else if canonical == "CJSW" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET CJSW must be finite and non-negative".to_string(),
                });
            } else if canonical == "CBS" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET CBS must be finite and non-negative".to_string(),
                });
            } else if canonical == "CBD" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET CBD must be finite and non-negative".to_string(),
                });
            } else if canonical == "CGSO" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET CGSO must be finite and non-negative".to_string(),
                });
            } else if canonical == "CGDO" && (!raw_value.is_finite() || *raw_value < 0.0) {
                return Err(SpiceError::InvalidElement {
                    name: name.clone(),
                    reason: "MOSFET CGDO must be finite and non-negative".to_string(),
                });
            } else {
                normalized.insert(canonical.to_string(), *raw_value);
            }
        } else if !unsupported.contains(&key) {
            unsupported.push(key);
        }
    }
    Ok(NormalizedModelCard {
        name,
        kind,
        parameters: normalized,
        unsupported_parameters: unsupported,
    })
}

pub fn model_card_unsupported_parameter_issues(
    model: &NormalizedModelCard,
) -> Vec<ModelCardUnsupportedParameterIssue> {
    model
        .unsupported_parameters
        .iter()
        .map(|parameter| ModelCardUnsupportedParameterIssue {
            model_name: model.name.clone(),
            kind: model.kind,
            parameter: parameter.clone(),
            message: format!(
                "unsupported {} model-card parameter {}",
                model.kind.as_str(),
                parameter
            ),
        })
        .collect()
}

pub fn format_model_card_unsupported_parameter_issue_table(model: &NormalizedModelCard) -> String {
    let mut lines = vec!["model_name\tkind\tparameter\tmessage".to_string()];
    lines.extend(
        model_card_unsupported_parameter_issues(model)
            .into_iter()
            .map(|issue| {
                format!(
                    "{}\t{}\t{}\t{}",
                    issue.model_name,
                    issue.kind.as_str(),
                    issue.parameter,
                    issue.message
                )
            }),
    );
    lines.join("\n")
}

pub fn model_card_unsupported_parameter_issue_records(
    model: &NormalizedModelCard,
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_model_card_unsupported_parameter_issue_table(model))
}

pub fn format_model_card_unsupported_parameter_issue_csv(model: &NormalizedModelCard) -> String {
    format_deck_table_csv(&format_model_card_unsupported_parameter_issue_table(model))
}

pub fn format_model_card_unsupported_parameter_issue_json(model: &NormalizedModelCard) -> String {
    format_deck_table_json(&format_model_card_unsupported_parameter_issue_table(model))
}

pub fn model_card_supported_parameter_coverage() -> Vec<ModelCardSupportedParameterCoverage> {
    let mut rows = Vec::new();
    for kind in MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS {
        let mut grouped: Vec<(&str, Vec<&str>)> = Vec::new();
        for &(accepted_name, canonical) in model_card_parameter_alias_entries(*kind) {
            if let Some((_, accepted_names)) = grouped
                .iter_mut()
                .find(|(candidate, _)| *candidate == canonical)
            {
                accepted_names.push(accepted_name);
            } else {
                grouped.push((canonical, vec![accepted_name]));
            }
        }
        rows.extend(
            grouped
                .into_iter()
                .map(
                    |(canonical_parameter, accepted_names)| ModelCardSupportedParameterCoverage {
                        kind: *kind,
                        canonical_parameter: canonical_parameter.to_string(),
                        alias_count: accepted_names.len(),
                        accepted_names: accepted_names.into_iter().map(str::to_string).collect(),
                    },
                ),
        );
    }
    rows
}

pub fn format_model_card_supported_parameter_coverage_table() -> String {
    let mut lines = vec!["kind\tcanonical_parameter\taccepted_names\talias_count".to_string()];
    lines.extend(
        model_card_supported_parameter_coverage()
            .into_iter()
            .map(|row| {
                format!(
                    "{}\t{}\t{}\t{}",
                    row.kind.as_str(),
                    row.canonical_parameter,
                    row.accepted_names.join("|"),
                    row.alias_count
                )
            }),
    );
    lines.join("\n")
}

pub fn model_card_supported_parameter_coverage_records() -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_model_card_supported_parameter_coverage_table())
}

pub fn format_model_card_supported_parameter_coverage_csv() -> String {
    format_deck_table_csv(&format_model_card_supported_parameter_coverage_table())
}

pub fn format_model_card_supported_parameter_coverage_json() -> String {
    format_deck_table_json(&format_model_card_supported_parameter_coverage_table())
}

fn model_card_supported_parameter_coverage_summary_from(
    coverage: &[ModelCardSupportedParameterCoverage],
) -> Vec<ModelCardSupportedParameterCoverageSummary> {
    MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS
        .iter()
        .map(|kind| {
            let rows = coverage
                .iter()
                .filter(|row| row.kind == *kind)
                .collect::<Vec<_>>();
            let aliased_parameters = rows
                .iter()
                .filter(|row| row.alias_count > 1)
                .map(|row| row.canonical_parameter.clone())
                .collect::<Vec<_>>();
            ModelCardSupportedParameterCoverageSummary {
                kind: *kind,
                canonical_parameter_count: rows.len(),
                accepted_name_count: rows.iter().map(|row| row.alias_count).sum(),
                aliased_parameter_count: aliased_parameters.len(),
                max_alias_count: rows.iter().map(|row| row.alias_count).max().unwrap_or(0),
                aliased_parameters,
            }
        })
        .collect()
}

pub fn model_card_supported_parameter_coverage_summary(
) -> Vec<ModelCardSupportedParameterCoverageSummary> {
    model_card_supported_parameter_coverage_summary_from(&model_card_supported_parameter_coverage())
}

pub fn format_model_card_supported_parameter_coverage_summary_table() -> String {
    let mut lines = vec![
        "kind\tcanonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\taliased_parameters"
            .to_string(),
    ];
    lines.extend(
        model_card_supported_parameter_coverage_summary()
            .into_iter()
            .map(|row| {
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    row.kind.as_str(),
                    row.canonical_parameter_count,
                    row.accepted_name_count,
                    row.aliased_parameter_count,
                    row.max_alias_count,
                    row.aliased_parameters.join("|")
                )
            }),
    );
    lines.join("\n")
}

pub fn model_card_supported_parameter_coverage_summary_records() -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_model_card_supported_parameter_coverage_summary_table())
}

pub fn format_model_card_supported_parameter_coverage_summary_csv() -> String {
    format_deck_table_csv(&format_model_card_supported_parameter_coverage_summary_table())
}

pub fn format_model_card_supported_parameter_coverage_summary_json() -> String {
    format_deck_table_json(&format_model_card_supported_parameter_coverage_summary_table())
}

pub fn model_card_supported_parameter_coverage_gate(
    coverage: &[ModelCardSupportedParameterCoverage],
) -> ModelCardSupportedParameterCoverageGateReport {
    let expected_kinds = MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS;
    let expected_summaries = MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_EXPECTED_SUMMARIES;
    let mut issues = Vec::new();
    let mut actual_kinds = Vec::new();
    for row in coverage {
        if !actual_kinds.contains(&row.kind) {
            actual_kinds.push(row.kind);
        }
    }

    if actual_kinds != expected_kinds {
        issues.push(ModelCardSupportedParameterCoverageGateIssue {
            kind: "catalog".to_string(),
            field: "kind_order".to_string(),
            message: format!(
                "expected model-card supported-parameter coverage kinds {}, found {}",
                expected_kinds
                    .iter()
                    .map(|kind| kind.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                actual_kinds
                    .iter()
                    .map(|kind| kind.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        });
    }

    let summaries = model_card_supported_parameter_coverage_summary_from(coverage);
    for (kind, expected_canonical, expected_accepted, expected_aliased, expected_max_alias) in
        expected_summaries
    {
        if let Some(summary) = summaries.iter().find(|summary| summary.kind == *kind) {
            if summary.canonical_parameter_count != *expected_canonical {
                issues.push(ModelCardSupportedParameterCoverageGateIssue {
                    kind: kind.as_str().to_string(),
                    field: "canonical_parameter_count".to_string(),
                    message: format!(
                        "expected {} to expose {} canonical supported parameters, found {}",
                        kind.as_str(),
                        expected_canonical,
                        summary.canonical_parameter_count
                    ),
                });
            }
            if summary.accepted_name_count != *expected_accepted {
                issues.push(ModelCardSupportedParameterCoverageGateIssue {
                    kind: kind.as_str().to_string(),
                    field: "accepted_name_count".to_string(),
                    message: format!(
                        "expected {} to expose {} accepted model-card names, found {}",
                        kind.as_str(),
                        expected_accepted,
                        summary.accepted_name_count
                    ),
                });
            }
            if summary.aliased_parameter_count != *expected_aliased {
                issues.push(ModelCardSupportedParameterCoverageGateIssue {
                    kind: kind.as_str().to_string(),
                    field: "aliased_parameter_count".to_string(),
                    message: format!(
                        "expected {} to expose {} alias-bearing parameters, found {}",
                        kind.as_str(),
                        expected_aliased,
                        summary.aliased_parameter_count
                    ),
                });
            }
            if summary.max_alias_count != *expected_max_alias {
                issues.push(ModelCardSupportedParameterCoverageGateIssue {
                    kind: kind.as_str().to_string(),
                    field: "max_alias_count".to_string(),
                    message: format!(
                        "expected {} max alias count {}, found {}",
                        kind.as_str(),
                        expected_max_alias,
                        summary.max_alias_count
                    ),
                });
            }
        }
    }

    ModelCardSupportedParameterCoverageGateReport {
        passed: issues.is_empty(),
        kind_count: actual_kinds.len(),
        expected_kind_count: expected_kinds.len(),
        canonical_parameter_count: coverage.len(),
        expected_canonical_parameter_count: expected_summaries
            .iter()
            .map(|(_, canonical_count, _, _, _)| canonical_count)
            .sum(),
        accepted_name_count: coverage.iter().map(|row| row.alias_count).sum(),
        aliased_parameter_count: coverage.iter().filter(|row| row.alias_count > 1).count(),
        max_alias_count: coverage
            .iter()
            .map(|row| row.alias_count)
            .max()
            .unwrap_or(0),
        issues,
    }
}

pub fn format_model_card_supported_parameter_coverage_gate_report(
    report: &ModelCardSupportedParameterCoverageGateReport,
) -> String {
    let mut lines = vec![
        "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\tissue_count"
            .to_string(),
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            report.passed,
            report.kind_count,
            report.expected_kind_count,
            report.canonical_parameter_count,
            report.expected_canonical_parameter_count,
            report.accepted_name_count,
            report.aliased_parameter_count,
            report.max_alias_count,
            report.issues.len()
        ),
    ];
    if !report.issues.is_empty() {
        lines.push("kind\tfield\tmessage".to_string());
        lines.extend(
            report
                .issues
                .iter()
                .map(|issue| format!("{}\t{}\t{}", issue.kind, issue.field, issue.message)),
        );
    }
    lines.join("\n")
}

pub fn format_model_card_supported_parameter_coverage_gate_issue_table(
    report: &ModelCardSupportedParameterCoverageGateReport,
) -> String {
    let mut lines = vec!["kind\tfield\tmessage".to_string()];
    lines.extend(
        report
            .issues
            .iter()
            .map(|issue| format!("{}\t{}\t{}", issue.kind, issue.field, issue.message)),
    );
    lines.join("\n")
}

pub fn model_card_supported_parameter_coverage_gate_issue_records(
    report: &ModelCardSupportedParameterCoverageGateReport,
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_model_card_supported_parameter_coverage_gate_issue_table(report))
}

pub fn format_model_card_supported_parameter_coverage_gate_issue_csv(
    report: &ModelCardSupportedParameterCoverageGateReport,
) -> String {
    format_deck_table_csv(&format_model_card_supported_parameter_coverage_gate_issue_table(report))
}

pub fn format_model_card_supported_parameter_coverage_gate_issue_json(
    report: &ModelCardSupportedParameterCoverageGateReport,
) -> String {
    format_deck_table_json(&format_model_card_supported_parameter_coverage_gate_issue_table(report))
}

pub fn model_card_supported_parameter_coverage_dashboard(
    coverage: &[ModelCardSupportedParameterCoverage],
) -> Vec<ModelCardSupportedParameterCoverageDashboardRow> {
    let summaries = model_card_supported_parameter_coverage_summary_from(coverage);
    let report = model_card_supported_parameter_coverage_gate(coverage);
    let mut global_issue_fields = Vec::new();
    for issue in report.issues.iter().filter(|issue| issue.kind == "catalog") {
        if !global_issue_fields.contains(&issue.field) {
            global_issue_fields.push(issue.field.clone());
        }
    }

    MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_EXPECTED_SUMMARIES
        .iter()
        .map(
            |(
                kind,
                expected_canonical_count,
                expected_accepted_count,
                expected_aliased_count,
                expected_max_alias_count,
            )| {
                let summary = summaries
                    .iter()
                    .find(|summary| summary.kind == *kind)
                    .expect("coverage summary should include every expected kind");
                let mut issue_fields = global_issue_fields.clone();
                for issue in report
                    .issues
                    .iter()
                    .filter(|issue| issue.kind == kind.as_str())
                {
                    if !issue_fields.contains(&issue.field) {
                        issue_fields.push(issue.field.clone());
                    }
                }
                ModelCardSupportedParameterCoverageDashboardRow {
                    kind: *kind,
                    passed: issue_fields.is_empty(),
                    canonical_parameter_count: summary.canonical_parameter_count,
                    expected_canonical_parameter_count: *expected_canonical_count,
                    accepted_name_count: summary.accepted_name_count,
                    expected_accepted_name_count: *expected_accepted_count,
                    aliased_parameter_count: summary.aliased_parameter_count,
                    expected_aliased_parameter_count: *expected_aliased_count,
                    max_alias_count: summary.max_alias_count,
                    expected_max_alias_count: *expected_max_alias_count,
                    issue_count: issue_fields.len(),
                    issue_fields,
                }
            },
        )
        .collect()
}

pub fn format_model_card_supported_parameter_coverage_dashboard_table(
    coverage: &[ModelCardSupportedParameterCoverage],
) -> String {
    let mut lines = vec![
        "kind\tpassed\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\texpected_accepted_name_count\taliased_parameter_count\texpected_aliased_parameter_count\tmax_alias_count\texpected_max_alias_count\tissue_count\tissue_fields"
            .to_string(),
    ];
    lines.extend(
        model_card_supported_parameter_coverage_dashboard(coverage)
            .into_iter()
            .map(|row| {
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    row.kind.as_str(),
                    row.passed,
                    row.canonical_parameter_count,
                    row.expected_canonical_parameter_count,
                    row.accepted_name_count,
                    row.expected_accepted_name_count,
                    row.aliased_parameter_count,
                    row.expected_aliased_parameter_count,
                    row.max_alias_count,
                    row.expected_max_alias_count,
                    row.issue_count,
                    row.issue_fields.join("|")
                )
            }),
    );
    lines.join("\n")
}

pub fn model_card_supported_parameter_coverage_dashboard_records(
    coverage: &[ModelCardSupportedParameterCoverage],
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_model_card_supported_parameter_coverage_dashboard_table(coverage))
}

pub fn format_model_card_supported_parameter_coverage_dashboard_csv(
    coverage: &[ModelCardSupportedParameterCoverage],
) -> String {
    format_deck_table_csv(&format_model_card_supported_parameter_coverage_dashboard_table(coverage))
}

pub fn format_model_card_supported_parameter_coverage_dashboard_json(
    coverage: &[ModelCardSupportedParameterCoverage],
) -> String {
    format_deck_table_json(
        &format_model_card_supported_parameter_coverage_dashboard_table(coverage),
    )
}

fn model_card_value(model: &NormalizedModelCard, key: &str, fallback: f64) -> f64 {
    model.parameters.get(key).copied().unwrap_or(fallback)
}

fn model_card_kind_error(instance_name: &str, expected: &str, actual: ModelCardKind) -> SpiceError {
    SpiceError::InvalidElement {
        name: instance_name.to_string(),
        reason: format!("expected {expected} model card, got {}", actual.as_str()),
    }
}

pub fn diode_from_model_card(
    name: impl Into<String>,
    anode: impl Into<String>,
    cathode: impl Into<String>,
    model: &NormalizedModelCard,
) -> Result<Diode, SpiceError> {
    let name = name.into();
    if model.kind != ModelCardKind::Diode {
        return Err(model_card_kind_error(&name, "diode", model.kind));
    }
    let mut diode = Diode::with_model_and_temperature_parameters(
        name,
        anode,
        cathode,
        model_card_value(model, "IS", 1.0e-15),
        model_card_value(model, "VT", 0.02585),
        model_card_value(model, "N", 1.0),
        model.parameters.get("BV").copied(),
        model_card_value(model, "IBV", 1.0e-3),
        model_card_value(model, "CJO", 0.0),
        model_card_value(model, "TT", 0.0),
        model_card_value(model, "VJ", 1.0),
        model_card_value(model, "M", 0.5),
        model_card_value(model, "FC", 0.5),
        model_card_value(model, "XTI", 3.0),
        model_card_value(model, "EG", 1.11),
    );
    diode.series_resistance = model_card_value(model, "RS", 0.0);
    diode.flicker_noise_coefficient = model_card_value(model, "KF", 0.0);
    diode.flicker_noise_exponent = model_card_value(model, "AF", 1.0);
    Ok(diode)
}

pub fn bjt_from_model_card(
    name: impl Into<String>,
    collector: impl Into<String>,
    base: impl Into<String>,
    emitter: impl Into<String>,
    model: &NormalizedModelCard,
) -> Result<Bjt, SpiceError> {
    let name = name.into();
    let polarity = match model.kind {
        ModelCardKind::Npn => BjtPolarity::Npn,
        ModelCardKind::Pnp => BjtPolarity::Pnp,
        _ => return Err(model_card_kind_error(&name, "BJT", model.kind)),
    };
    let saturation_current = model_card_value(model, "IS", 1.0e-14);
    let base_emitter_leakage_saturation_current = model
        .parameters
        .get("ISE")
        .copied()
        .unwrap_or_else(|| model_card_value(model, "C2", 0.0) * saturation_current);
    let base_collector_leakage_saturation_current = model
        .parameters
        .get("ISC")
        .copied()
        .unwrap_or_else(|| model_card_value(model, "C4", 0.0) * saturation_current);
    let mut bjt = Bjt::with_model_temperature_depletion_early_rolloff_junction_leakage_and_reverse_beta_parameters(
            name,
            collector,
            base,
            emitter,
            polarity,
            saturation_current,
            model_card_value(model, "BF", 100.0),
            model_card_value(model, "VT", 0.02585),
            model_card_value(model, "CJE", 0.0),
            model_card_value(model, "CJC", 0.0),
            model_card_value(model, "TF", 0.0),
            model_card_value(model, "TR", 0.0),
            model_card_value(model, "XTI", 3.0),
            model_card_value(model, "EG", 1.11),
            model_card_value(model, "VAF", 0.0),
            model_card_value(model, "NF", 1.0),
            model_card_value(model, "NR", 1.0),
            model_card_value(model, "VJE", 0.75),
            model_card_value(model, "MJE", 0.33),
            model_card_value(model, "VJC", 0.75),
            model_card_value(model, "MJC", 0.33),
            model_card_value(model, "FC", 0.5),
            model_card_value(model, "VAR", 0.0),
            model_card_value(model, "IKF", 0.0),
            base_emitter_leakage_saturation_current,
            model_card_value(model, "NE", 1.0),
            base_collector_leakage_saturation_current,
            model_card_value(model, "NC", 2.0),
            model_card_value(model, "XTB", 0.0),
            model_card_value(model, "BR", 1.0),
        );
    bjt.reverse_beta_rolloff_current = model_card_value(model, "IKR", 0.0);
    bjt.nominal_temperature_kelvin = model
        .parameters
        .get("TNOM")
        .map(|temperature_celsius| temperature_celsius + 273.15);
    bjt.flicker_noise_coefficient = model_card_value(model, "KF", 0.0);
    bjt.flicker_noise_exponent = model_card_value(model, "AF", 1.0);
    bjt.forward_excess_phase_degrees = model_card_value(model, "PTF", 0.0);
    bjt.forward_transit_time_bias_coefficient = model_card_value(model, "XTF", 0.0);
    bjt.forward_transit_time_current = model_card_value(model, "ITF", 0.0);
    bjt.forward_transit_time_voltage = model_card_value(model, "VTF", 0.0);
    bjt.emitter_resistance = model_card_value(model, "RE", 0.0);
    bjt.collector_resistance = model_card_value(model, "RC", 0.0);
    bjt.base_resistance = model_card_value(model, "RB", 0.0);
    bjt.minimum_base_resistance = model.parameters.get("RBM").copied();
    bjt.base_resistance_half_current = model_card_value(model, "IRB", 0.0);
    bjt.base_collector_capacitance_fraction = model_card_value(model, "XCJC", 1.0);
    Ok(bjt)
}

pub fn jfet_from_model_card(
    name: impl Into<String>,
    drain: impl Into<String>,
    gate: impl Into<String>,
    source: impl Into<String>,
    model: &NormalizedModelCard,
) -> Result<Jfet, SpiceError> {
    let name = name.into();
    let polarity = match model.kind {
        ModelCardKind::Njf => JfetPolarity::Njf,
        ModelCardKind::Pjf => JfetPolarity::Pjf,
        _ => return Err(model_card_kind_error(&name, "JFET", model.kind)),
    };
    let mut jfet = Jfet::with_model_and_capacitance(
        name,
        drain,
        gate,
        source,
        polarity,
        model_card_value(model, "BETA", 1.0e-4),
        model_card_value(
            model,
            "VTO",
            if model.kind == ModelCardKind::Njf {
                -2.0
            } else {
                2.0
            },
        ),
        model_card_value(model, "LAMBDA", 0.0),
        model_card_value(model, "CGS", 0.0),
        model_card_value(model, "CGD", 0.0),
    );
    jfet.flicker_noise_coefficient = model_card_value(model, "KF", 0.0);
    jfet.flicker_noise_exponent = model_card_value(model, "AF", 1.0);
    jfet.junction_potential = model_card_value(model, "PB", 1.0);
    jfet.forward_bias_depletion_coefficient = model_card_value(model, "FC", 0.5);
    jfet.gate_saturation_current = model_card_value(model, "IS", 1.0e-14);
    jfet.gate_saturation_current_temperature_exponent = model_card_value(model, "XTI", 3.0);
    jfet.bandgap_voltage = model_card_value(model, "EG", 1.11);
    jfet.doping_tail_parameter = model_card_value(model, "B", 1.0);
    jfet.noise_equation_level = model_card_value(model, "NLEV", 1.0);
    jfet.channel_noise_coefficient = model_card_value(model, "GDSNOI", 1.0);
    jfet.drain_resistance = model_card_value(model, "RD", 0.0);
    jfet.source_resistance = model_card_value(model, "RS", 0.0);
    jfet.threshold_voltage_temperature_coefficient = model_card_value(model, "TCV", 0.0);
    jfet.alternative_threshold_voltage_temperature_coefficient =
        model.parameters.get("VTOTC").copied();
    jfet.nominal_temperature_kelvin = model
        .parameters
        .get("TNOM")
        .map(|temperature| temperature + 273.15);
    jfet.mobility_temperature_exponent = model_card_value(model, "BEX", 0.0);
    jfet.mobility_temperature_coefficient = model.parameters.get("BETATCE").copied();
    Ok(jfet)
}

pub fn mosfet_from_model_card(
    name: impl Into<String>,
    drain: impl Into<String>,
    gate: impl Into<String>,
    source: impl Into<String>,
    body: impl Into<String>,
    model: &NormalizedModelCard,
) -> Result<Mosfet, SpiceError> {
    let name = name.into();
    let mosfet_type = match model.kind {
        ModelCardKind::Nmos => MosfetType::Nmos,
        ModelCardKind::Pmos => MosfetType::Pmos,
        _ => return Err(model_card_kind_error(&name, "MOSFET", model.kind)),
    };
    let mut params = MosfetLevel1Params::default();
    if let Some(value) = model.parameters.get("VT0") {
        params.vt0 = *value;
    }
    if let Some(value) = model.parameters.get("LAMBDA") {
        params.lambda = *value;
    }
    if let Some(value) = model.parameters.get("GAMMA") {
        params.gamma = *value;
    }
    if let Some(value) = model.parameters.get("PHI") {
        params.phi = *value;
    }
    if let Some(value) = model.parameters.get("W") {
        params.w = *value;
    }
    if let Some(value) = model.parameters.get("L") {
        params.l = *value;
    }
    if let Some(value) = model.parameters.get("LD") {
        params.lateral_diffusion_length = *value;
    }
    if let Some(value) = model.parameters.get("TOX") {
        params.oxide_thickness = *value;
    }
    if let Some(value) = model.parameters.get("U0") {
        params.surface_mobility = *value;
    }
    if let Some(value) = model.parameters.get("KP") {
        params.kp = *value;
    } else if model.parameters.contains_key("TOX") && params.oxide_thickness > 0.0 {
        params.kp = params.surface_mobility * 1.0e-4 * OXIDE_PERMITTIVITY / params.oxide_thickness;
    }
    if let Some(value) = model.parameters.get("RD") {
        params.drain_resistance = *value;
    }
    if let Some(value) = model.parameters.get("RS") {
        params.source_resistance = *value;
    }
    if let Some(value) = model.parameters.get("RSH") {
        params.sheet_resistance = *value;
    }
    if let Some(value) = model.parameters.get("IS") {
        params.saturation_current = *value;
    }
    if let Some(value) = model.parameters.get("JS") {
        params.saturation_current_density = *value;
    }
    if let Some(value) = model.parameters.get("N_SUB") {
        params.n_sub = *value;
    }
    if let Some(value) = model.parameters.get("T_NOM") {
        params.t_nom = *value;
    }
    if let (Some(substrate_doping), Some(oxide_thickness)) =
        (model.parameters.get("N_SUB"), model.parameters.get("TOX"))
    {
        let substrate_doping_per_cubic_meter = substrate_doping * CUBIC_CENTIMETERS_PER_CUBIC_METER;
        if substrate_doping_per_cubic_meter <= INTRINSIC_CARRIER_DENSITY_PER_CUBIC_METER {
            return Err(SpiceError::InvalidElement {
                name: name.clone(),
                reason: "MOSFET NSUB must exceed the intrinsic carrier density".to_string(),
            });
        }
        if *oxide_thickness > 0.0 {
            let oxide_capacitance = OXIDE_PERMITTIVITY / oxide_thickness;
            if !model.parameters.contains_key("PHI") {
                let thermal_voltage = BOLTZMANN * params.t_nom / ELECTRON_CHARGE;
                params.phi = (2.0
                    * thermal_voltage
                    * (substrate_doping_per_cubic_meter
                        / INTRINSIC_CARRIER_DENSITY_PER_CUBIC_METER)
                        .ln())
                .max(0.1);
            }
            if !model.parameters.contains_key("GAMMA") {
                params.gamma = (2.0
                    * SILICON_PERMITTIVITY
                    * ELECTRON_CHARGE
                    * substrate_doping_per_cubic_meter)
                    .sqrt()
                    / oxide_capacitance;
            }
            if !model.parameters.contains_key("VT0") {
                let polarity = match mosfet_type {
                    MosfetType::Nmos => 1.0,
                    MosfetType::Pmos => -1.0,
                };
                let band_gap = silicon_band_gap_electron_volts(params.t_nom);
                let gate_type = model_card_value(model, "TPG", 1.0);
                let substrate_fermi_potential = polarity * 0.5 * params.phi;
                let gate_work_function = if gate_type == 0.0 {
                    3.2
                } else {
                    let gate_fermi_potential = polarity * gate_type * 0.5 * band_gap;
                    3.25 + 0.5 * band_gap - gate_fermi_potential
                };
                let gate_substrate_work_function =
                    gate_work_function - (3.25 + 0.5 * band_gap + substrate_fermi_potential);
                let surface_state_shift =
                    model_card_value(model, "NSS", 0.0) * 1.0e4 * ELECTRON_CHARGE
                        / oxide_capacitance;
                params.vt0 = gate_substrate_work_function - surface_state_shift
                    + polarity * (params.gamma * params.phi.sqrt() + params.phi);
            }
        }
    }
    if let Some(value) = model.parameters.get("CGSO") {
        params.gate_source_overlap_capacitance = *value;
    }
    if let Some(value) = model.parameters.get("CGDO") {
        params.gate_drain_overlap_capacitance = *value;
    }
    if let Some(value) = model.parameters.get("CGBO") {
        params.gate_bulk_overlap_capacitance = *value;
    }
    if let Some(value) = model.parameters.get("CBS") {
        params.source_bulk_capacitance = *value;
    }
    if let Some(value) = model.parameters.get("CBD") {
        params.drain_bulk_capacitance = *value;
    }
    if let Some(value) = model.parameters.get("CJ") {
        params.bottom_junction_capacitance = *value;
    }
    if let Some(value) = model.parameters.get("CJSW") {
        params.sidewall_junction_capacitance = *value;
    }
    if let Some(value) = model.parameters.get("PB") {
        params.bulk_junction_potential = *value;
    }
    if let Some(value) = model.parameters.get("MJ") {
        params.bulk_junction_grading_coefficient = *value;
    }
    if let Some(value) = model.parameters.get("MJSW") {
        params.sidewall_junction_grading_coefficient = *value;
    }
    if let Some(value) = model.parameters.get("FC") {
        params.forward_bias_depletion_coefficient = *value;
    }
    if let Some(value) = model.parameters.get("KF") {
        params.flicker_noise_coefficient = *value;
    }
    if let Some(value) = model.parameters.get("AF") {
        params.flicker_noise_exponent = *value;
    }
    Ok(Mosfet::with_model(
        name,
        drain,
        gate,
        source,
        body,
        mosfet_type,
        params,
    ))
}

pub fn device_model_audit_fixtures() -> Result<Vec<NormalizedModelCard>, SpiceError> {
    Ok(vec![
        normalize_model_card(
            "Dfast",
            "diode",
            &[("JS", 2.0e-14), ("CJ", 1.5e-12), ("TT", 4.0e-9)],
        )?,
        normalize_model_card(
            "Qsmall",
            "npn",
            &[("BETA", 125.0), ("CBE", 2.0e-12), ("TF", 1.0e-10)],
        )?,
        normalize_model_card(
            "Jn",
            "njfet",
            &[("BET", 9.0e-4), ("VT0", -1.8), ("LAM", 0.02)],
        )?,
        normalize_model_card(
            "Mn",
            "nmos",
            &[
                ("LEVEL", 1.0),
                ("VTO", 0.55),
                ("LAM", 0.04),
                ("NSUB", 1.6),
                ("CJD", 3.0e-13),
            ],
        )?,
    ])
}

fn model_card_by_name(
    models: &[NormalizedModelCard],
) -> Result<BTreeMap<String, NormalizedModelCard>, SpiceError> {
    Ok(models
        .iter()
        .map(|model| (model.name.clone(), model.clone()))
        .collect())
}

pub fn device_model_behavior_audit_fixtures() -> Result<Vec<DeviceModelBehaviorFixture>, SpiceError>
{
    let models = model_card_by_name(&device_model_audit_fixtures()?)?;

    let diode_model = models
        .get("Dfast")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_behavior_audit_fixtures".to_string(),
            reason: "missing Dfast model fixture".to_string(),
        })?;
    let mut diode_circuit = Circuit::new();
    diode_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbias", "vin", "0", 0.8,
    )));
    diode_circuit.add(Element::Resistor(Resistor::new(
        "Rlimit", "vin", "out", 1_000.0,
    )));
    diode_circuit.add(Element::Diode(diode_from_model_card(
        "D1",
        "out",
        "0",
        diode_model,
    )?));

    let bjt_model = models
        .get("Qsmall")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_behavior_audit_fixtures".to_string(),
            reason: "missing Qsmall model fixture".to_string(),
        })?;
    let mut bjt_circuit = Circuit::new();
    bjt_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcc", "vcc", "0", 5.0,
    )));
    bjt_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.72,
    )));
    bjt_circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));
    bjt_circuit.add(Element::Bjt(bjt_from_model_card(
        "Q1", "vcc", "base", "out", bjt_model,
    )?));

    let jfet_model = models.get("Jn").ok_or_else(|| SpiceError::InvalidElement {
        name: "device_model_behavior_audit_fixtures".to_string(),
        reason: "missing Jn model fixture".to_string(),
    })?;
    let mut jfet_circuit = Circuit::new();
    jfet_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 10.0,
    )));
    jfet_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vg", "gate", "0", 0.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rd", "vdd", "drain", 2_000.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rs", "source", "0", 1_000.0,
    )));
    jfet_circuit.add(Element::Jfet(jfet_from_model_card(
        "J1", "drain", "gate", "source", jfet_model,
    )?));

    let mos_model = models.get("Mn").ok_or_else(|| SpiceError::InvalidElement {
        name: "device_model_behavior_audit_fixtures".to_string(),
        reason: "missing Mn model fixture".to_string(),
    })?;
    let mut mos_circuit = Circuit::new();
    mos_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 1.8,
    )));
    mos_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 1.8,
    )));
    mos_circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    mos_circuit.add(Element::Mosfet(mosfet_from_model_card(
        "M1", "out", "gate", "0", "0", mos_model,
    )?));

    Ok(vec![
        DeviceModelBehaviorFixture {
            name: "diode-forward-bias".to_string(),
            kind: diode_model.kind,
            model: diode_model.clone(),
            circuit: diode_circuit,
            probe_node: "out".to_string(),
            expected_min: 0.55,
            expected_max: 0.65,
            deck_lines: vec![
                "* device-model behavior fixture: diode-forward-bias".to_string(),
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)".to_string(),
                "Vbias vin 0 0.8".to_string(),
                "Rlimit vin out 1k".to_string(),
                "D1 out 0 Dfast".to_string(),
                ".op".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelBehaviorFixture {
            name: "bjt-emitter-follower".to_string(),
            kind: bjt_model.kind,
            model: bjt_model.clone(),
            circuit: bjt_circuit,
            probe_node: "out".to_string(),
            expected_min: 0.08,
            expected_max: 0.18,
            deck_lines: vec![
                "* device-model behavior fixture: bjt-emitter-follower".to_string(),
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)".to_string(),
                "Vcc vcc 0 5".to_string(),
                "Vbase base 0 0.72".to_string(),
                "Q1 vcc base out Qsmall".to_string(),
                "Rload out 0 1k".to_string(),
                ".op".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelBehaviorFixture {
            name: "jfet-source-bias".to_string(),
            kind: jfet_model.kind,
            model: jfet_model.clone(),
            circuit: jfet_circuit,
            probe_node: "source".to_string(),
            expected_min: 0.80,
            expected_max: 0.95,
            deck_lines: vec![
                "* device-model behavior fixture: jfet-source-bias".to_string(),
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02)".to_string(),
                "Vdd vdd 0 10".to_string(),
                "Vg gate 0 0".to_string(),
                "Rd vdd drain 2k".to_string(),
                "Rs source 0 1k".to_string(),
                "J1 drain gate source Jn".to_string(),
                ".op".to_string(),
                ".save V(source)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelBehaviorFixture {
            name: "mos-level1-common-source".to_string(),
            kind: mos_model.kind,
            model: mos_model.clone(),
            circuit: mos_circuit,
            probe_node: "out".to_string(),
            expected_min: 0.55,
            expected_max: 0.85,
            deck_lines: vec![
                "* device-model behavior fixture: mos-level1-common-source".to_string(),
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)".to_string(),
                "Vdd vdd 0 1.8".to_string(),
                "Vgate gate 0 1.8".to_string(),
                "Rload vdd out 1k".to_string(),
                "M1 out gate 0 0 Mn".to_string(),
                ".op".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
    ])
}

fn device_model_temperature_points(
    name: &str,
) -> Result<Vec<DeviceModelTemperaturePoint>, SpiceError> {
    let windows: &[(f64, f64, f64)] = match name {
        "diode-forward-bias" => &[
            (260.15, 0.63, 0.70),
            (300.15, 0.55, 0.65),
            (340.15, 0.49, 0.56),
        ],
        "bjt-emitter-follower" => &[
            (260.15, 0.03, 0.09),
            (300.15, 0.08, 0.18),
            (340.15, 0.15, 0.22),
        ],
        "jfet-source-bias" => &[
            (260.15, 0.86, 0.90),
            (300.15, 0.86, 0.90),
            (340.15, 0.86, 0.90),
        ],
        "mos-level1-common-source" => &[
            (260.15, 0.58, 0.68),
            (300.15, 0.55, 0.85),
            (340.15, 0.70, 0.82),
        ],
        _ => {
            return Err(SpiceError::InvalidElement {
                name: "device_model_temperature_audit_fixtures".to_string(),
                reason: format!("missing temperature windows for {name}"),
            })
        }
    };
    Ok(windows
        .iter()
        .map(
            |(temperature_kelvin, expected_min, expected_max)| DeviceModelTemperaturePoint {
                temperature_kelvin: *temperature_kelvin,
                expected_min: *expected_min,
                expected_max: *expected_max,
            },
        )
        .collect())
}

fn device_model_temperature_behavior(name: &str) -> Result<String, SpiceError> {
    match name {
        "diode-forward-bias" => {
            Ok("diode saturation current and thermal voltage scale with temperature".to_string())
        }
        "bjt-emitter-follower" => {
            Ok("BJT saturation current and thermal voltage scale with temperature".to_string())
        }
        "jfet-source-bias" => Ok(
            "JFET temperature scaling defaults to invariant; VTOTC overrides TCV for threshold-voltage scaling; BETATCE overrides BEX for beta scaling".to_string(),
        ),
        "mos-level1-common-source" => {
            Ok("Level-1 MOS threshold and transconductance scale with temperature".to_string())
        }
        _ => Err(SpiceError::InvalidElement {
            name: "device_model_temperature_audit_fixtures".to_string(),
            reason: format!("missing temperature behavior for {name}"),
        }),
    }
}

fn device_model_temperature_deck_lines(fixture: &DeviceModelBehaviorFixture) -> Vec<String> {
    let mut lines = fixture.deck_lines.clone();
    if let Some(first) = lines.first_mut() {
        *first = format!("* device-model temperature fixture: {}", fixture.name);
    }
    let op_index = lines
        .iter()
        .position(|line| line == ".op")
        .unwrap_or(lines.len());
    lines.insert(op_index, ".temp 260.15 300.15 340.15".to_string());
    lines
}

pub fn device_model_temperature_audit_fixtures(
) -> Result<Vec<DeviceModelTemperatureBehaviorFixture>, SpiceError> {
    device_model_behavior_audit_fixtures()?
        .into_iter()
        .map(|fixture| {
            Ok(DeviceModelTemperatureBehaviorFixture {
                name: fixture.name.clone(),
                kind: fixture.kind,
                model: fixture.model.clone(),
                circuit: fixture.circuit.clone(),
                probe_node: fixture.probe_node.clone(),
                nominal_temperature_kelvin: 300.15,
                energy_gap_electron_volts: 1.11,
                temperature_behavior: device_model_temperature_behavior(&fixture.name)?,
                temperature_points: device_model_temperature_points(&fixture.name)?,
                deck_lines: device_model_temperature_deck_lines(&fixture),
            })
        })
        .collect()
}

pub fn device_model_capacitance_audit_fixtures(
) -> Result<Vec<DeviceModelCapacitanceBehaviorFixture>, SpiceError> {
    let models = model_card_by_name(&device_model_audit_fixtures()?)?;
    let frequency_hz = 100_000.0;

    let diode_model = models
        .get("Dfast")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_capacitance_audit_fixtures".to_string(),
            reason: "missing Dfast model fixture".to_string(),
        })?;
    let mut diode_circuit = Circuit::new();
    diode_circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vdrive", "in", "0", 0.0, 1.0, 0.0,
    )));
    diode_circuit.add(Element::Resistor(Resistor::new(
        "Rin",
        "in",
        "out",
        1_000_000.0,
    )));
    diode_circuit.add(Element::Diode(diode_from_model_card(
        "D1",
        "out",
        "0",
        diode_model,
    )?));

    let bjt_model = models
        .get("Qsmall")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_capacitance_audit_fixtures".to_string(),
            reason: "missing Qsmall model fixture".to_string(),
        })?;
    let mut bjt_circuit = Circuit::new();
    bjt_circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vdrive", "in", "0", 0.0, 1.0, 0.0,
    )));
    bjt_circuit.add(Element::Resistor(Resistor::new(
        "Rin",
        "in",
        "base",
        1_000_000.0,
    )));
    bjt_circuit.add(Element::Resistor(Resistor::new("Rc", "col", "0", 1_000.0)));
    bjt_circuit.add(Element::Bjt(bjt_from_model_card(
        "Q1", "col", "base", "0", bjt_model,
    )?));

    let jfet_model = normalize_model_card(
        "Jn",
        "NJF",
        &[
            ("BETA", 9.0e-4),
            ("VTO", -1.8),
            ("LAMBDA", 0.02),
            ("CGS", 2.0e-9),
            ("CGD", 1.0e-10),
        ],
    )?;
    let mut jfet_circuit = Circuit::new();
    jfet_circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vdrive", "in", "0", 0.0, 1.0, 0.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rin", "in", "source", 1_000.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rd", "drain", "0", 2_000.0,
    )));
    jfet_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    jfet_circuit.add(Element::Jfet(jfet_from_model_card(
        "J1",
        "drain",
        "gate",
        "source",
        &jfet_model,
    )?));

    let mos_model = models.get("Mn").ok_or_else(|| SpiceError::InvalidElement {
        name: "device_model_capacitance_audit_fixtures".to_string(),
        reason: "missing Mn model fixture".to_string(),
    })?;
    let mut mos_circuit = Circuit::new();
    mos_circuit.add(Element::VoltageSource(VoltageSource::with_ac(
        "Vdrive", "in", "0", 0.0, 1.0, 0.0,
    )));
    mos_circuit.add(Element::Resistor(Resistor::new(
        "Rin",
        "in",
        "drain",
        5_000_000.0,
    )));
    mos_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 0.0,
    )));
    mos_circuit.add(Element::Mosfet(mosfet_from_model_card(
        "M1", "drain", "gate", "0", "0", mos_model,
    )?));

    Ok(vec![
        DeviceModelCapacitanceBehaviorFixture {
            name: "diode-capacitance-ac".to_string(),
            kind: diode_model.kind,
            model: diode_model.clone(),
            circuit: diode_circuit,
            probe_node: "out".to_string(),
            frequency_hz,
            expected_magnitude_min: 0.72,
            expected_magnitude_max: 0.74,
            capacitance_behavior: "diode CJO and TT contribute high-frequency shunt capacitance"
                .to_string(),
            deck_lines: vec![
                "* device-model capacitance fixture: diode-capacitance-ac".to_string(),
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)".to_string(),
                "Vdrive in 0 0 AC 1".to_string(),
                "Rin in out 1meg".to_string(),
                "D1 out 0 Dfast".to_string(),
                ".ac lin 1 100k 100k".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelCapacitanceBehaviorFixture {
            name: "bjt-capacitance-ac".to_string(),
            kind: bjt_model.kind,
            model: bjt_model.clone(),
            circuit: bjt_circuit,
            probe_node: "base".to_string(),
            frequency_hz,
            expected_magnitude_min: 0.61,
            expected_magnitude_max: 0.64,
            capacitance_behavior: "BJT CJE and TF contribute base-emitter AC capacitance"
                .to_string(),
            deck_lines: vec![
                "* device-model capacitance fixture: bjt-capacitance-ac".to_string(),
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)".to_string(),
                "Vdrive in 0 0 AC 1".to_string(),
                "Rin in base 1meg".to_string(),
                "Rc col 0 1k".to_string(),
                "Q1 col base 0 Qsmall".to_string(),
                ".ac lin 1 100k 100k".to_string(),
                ".save V(base)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelCapacitanceBehaviorFixture {
            name: "jfet-capacitance-ac".to_string(),
            kind: jfet_model.kind,
            model: jfet_model.clone(),
            circuit: jfet_circuit,
            probe_node: "source".to_string(),
            frequency_hz,
            expected_magnitude_min: 0.50,
            expected_magnitude_max: 0.54,
            capacitance_behavior: "JFET CGS/CGD contribute high-frequency gate-channel capacitance"
                .to_string(),
            deck_lines: vec![
                "* device-model capacitance fixture: jfet-capacitance-ac".to_string(),
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02 CGS=2n CGD=100p)".to_string(),
                "Vdrive in 0 0 AC 1".to_string(),
                "Rin in source 1k".to_string(),
                "Rd drain 0 2k".to_string(),
                "Vgate gate 0 0".to_string(),
                "J1 drain gate source Jn".to_string(),
                ".ac lin 1 100k 100k".to_string(),
                ".save V(source)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelCapacitanceBehaviorFixture {
            name: "mos-level1-capacitance-ac".to_string(),
            kind: mos_model.kind,
            model: mos_model.clone(),
            circuit: mos_circuit,
            probe_node: "drain".to_string(),
            frequency_hz,
            expected_magnitude_min: 0.72,
            expected_magnitude_max: 0.74,
            capacitance_behavior: "Level-1 MOS CBD contributes drain-bulk AC capacitance"
                .to_string(),
            deck_lines: vec![
                "* device-model capacitance fixture: mos-level1-capacitance-ac".to_string(),
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)".to_string(),
                "Vdrive in 0 0 AC 1".to_string(),
                "Rin in drain 5meg".to_string(),
                "Vgate gate 0 0".to_string(),
                "M1 drain gate 0 0 Mn".to_string(),
                ".ac lin 1 100k 100k".to_string(),
                ".save V(drain)".to_string(),
                ".end".to_string(),
            ],
        },
    ])
}

pub fn device_model_noise_audit_fixtures(
) -> Result<Vec<DeviceModelNoiseBehaviorFixture>, SpiceError> {
    let models = model_card_by_name(&device_model_audit_fixtures()?)?;
    let frequency_hz = 1_000.0;

    let diode_model = models
        .get("Dfast")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_noise_audit_fixtures".to_string(),
            reason: "missing Dfast model fixture".to_string(),
        })?;
    let mut diode_circuit = Circuit::new();
    diode_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbias", "vin", "0", 0.8,
    )));
    diode_circuit.add(Element::Resistor(Resistor::new(
        "Rlimit", "vin", "out", 1_000.0,
    )));
    diode_circuit.add(Element::Diode(diode_from_model_card(
        "D1",
        "out",
        "0",
        diode_model,
    )?));

    let bjt_model = models
        .get("Qsmall")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_noise_audit_fixtures".to_string(),
            reason: "missing Qsmall model fixture".to_string(),
        })?;
    let mut bjt_circuit = Circuit::new();
    bjt_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcc", "vcc", "0", 5.0,
    )));
    bjt_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.72,
    )));
    bjt_circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));
    bjt_circuit.add(Element::Bjt(bjt_from_model_card(
        "Q1", "vcc", "base", "out", bjt_model,
    )?));

    let jfet_model = models.get("Jn").ok_or_else(|| SpiceError::InvalidElement {
        name: "device_model_noise_audit_fixtures".to_string(),
        reason: "missing Jn model fixture".to_string(),
    })?;
    let mut jfet_circuit = Circuit::new();
    jfet_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 10.0,
    )));
    jfet_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vg", "gate", "0", 0.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rd", "vdd", "drain", 2_000.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rs", "source", "0", 1_000.0,
    )));
    jfet_circuit.add(Element::Jfet(jfet_from_model_card(
        "J1", "drain", "gate", "source", jfet_model,
    )?));

    let mos_model = models.get("Mn").ok_or_else(|| SpiceError::InvalidElement {
        name: "device_model_noise_audit_fixtures".to_string(),
        reason: "missing Mn model fixture".to_string(),
    })?;
    let mut mos_circuit = Circuit::new();
    mos_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 1.8,
    )));
    mos_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 1.8,
    )));
    mos_circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    mos_circuit.add(Element::Mosfet(mosfet_from_model_card(
        "M1", "out", "gate", "0", "0", mos_model,
    )?));

    Ok(vec![
        DeviceModelNoiseBehaviorFixture {
            name: "diode-shot-noise".to_string(),
            kind: diode_model.kind,
            model: diode_model.clone(),
            circuit: diode_circuit,
            output_node: "out".to_string(),
            input_source: "Vbias".to_string(),
            frequency_hz,
            expected_noise_element: "D1".to_string(),
            expected_noise_type: NoiseType::Shot,
            expected_source_psd_min: 6.4e-23,
            expected_source_psd_max: 6.7e-23,
            expected_output_psd_min: 8.0e-19,
            expected_output_psd_max: 8.5e-19,
            noise_behavior: "diode forward current contributes junction shot noise".to_string(),
            deck_lines: vec![
                "* device-model noise fixture: diode-shot-noise".to_string(),
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)".to_string(),
                "Vbias vin 0 0.8".to_string(),
                "Rlimit vin out 1k".to_string(),
                "D1 out 0 Dfast".to_string(),
                ".noise V(out) Vbias lin 1 1k 1k".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelNoiseBehaviorFixture {
            name: "bjt-shot-noise".to_string(),
            kind: bjt_model.kind,
            model: bjt_model.clone(),
            circuit: bjt_circuit,
            output_node: "out".to_string(),
            input_source: "Vbase".to_string(),
            frequency_hz,
            expected_noise_element: "Q1".to_string(),
            expected_noise_type: NoiseType::Shot,
            expected_source_psd_min: 3.7e-23,
            expected_source_psd_max: 3.9e-23,
            expected_output_psd_min: 1.1e-18,
            expected_output_psd_max: 1.3e-18,
            noise_behavior: "BJT forward-active collector current contributes shot noise"
                .to_string(),
            deck_lines: vec![
                "* device-model noise fixture: bjt-shot-noise".to_string(),
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)".to_string(),
                "Vcc vcc 0 5".to_string(),
                "Vbase base 0 0.72".to_string(),
                "Q1 vcc base out Qsmall".to_string(),
                "Rload out 0 1k".to_string(),
                ".noise V(out) Vbase lin 1 1k 1k".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelNoiseBehaviorFixture {
            name: "jfet-channel-noise".to_string(),
            kind: jfet_model.kind,
            model: jfet_model.clone(),
            circuit: jfet_circuit,
            output_node: "source".to_string(),
            input_source: "Vdd".to_string(),
            frequency_hz,
            expected_noise_element: "J1".to_string(),
            expected_noise_type: NoiseType::Thermal,
            expected_source_psd_min: 2.0e-23,
            expected_source_psd_max: 2.2e-23,
            expected_output_psd_min: 2.3e-18,
            expected_output_psd_max: 2.5e-18,
            noise_behavior: "JFET transconductance contributes long-channel channel thermal noise"
                .to_string(),
            deck_lines: vec![
                "* device-model noise fixture: jfet-channel-noise".to_string(),
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02)".to_string(),
                "Vdd vdd 0 10".to_string(),
                "Vg gate 0 0".to_string(),
                "Rd vdd drain 2k".to_string(),
                "Rs source 0 1k".to_string(),
                "J1 drain gate source Jn".to_string(),
                ".noise V(source) Vdd lin 1 1k 1k".to_string(),
                ".save V(source)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelNoiseBehaviorFixture {
            name: "mos-level1-channel-noise".to_string(),
            kind: mos_model.kind,
            model: mos_model.clone(),
            circuit: mos_circuit,
            output_node: "out".to_string(),
            input_source: "Vgate".to_string(),
            frequency_hz,
            expected_noise_element: "M1".to_string(),
            expected_noise_type: NoiseType::Thermal,
            expected_source_psd_min: 1.3e-23,
            expected_source_psd_max: 1.4e-23,
            expected_output_psd_min: 3.3e-18,
            expected_output_psd_max: 3.5e-18,
            noise_behavior: "Level-1 MOS gm contributes long-channel channel thermal noise"
                .to_string(),
            deck_lines: vec![
                "* device-model noise fixture: mos-level1-channel-noise".to_string(),
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)".to_string(),
                "Vdd vdd 0 1.8".to_string(),
                "Vgate gate 0 1.8".to_string(),
                "Rload vdd out 1k".to_string(),
                "M1 out gate 0 0 Mn".to_string(),
                ".noise V(out) Vgate lin 1 1k 1k".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
    ])
}

pub fn device_model_charge_audit_fixtures(
) -> Result<Vec<DeviceModelChargeBehaviorFixture>, SpiceError> {
    let models = model_card_by_name(&device_model_audit_fixtures()?)?;
    let time_step_s = 2.0e-8;
    let stop_time_s = 2.0e-6;
    let storage_capacitance_f = 1.0e-10;

    let diode_model = models
        .get("Dfast")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_charge_audit_fixtures".to_string(),
            reason: "missing Dfast model fixture".to_string(),
        })?;
    let mut diode_circuit = Circuit::new();
    diode_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbias", "vin", "0", 0.8,
    )));
    diode_circuit.add(Element::Resistor(Resistor::new(
        "Rlimit", "vin", "out", 1_000.0,
    )));
    diode_circuit.add(Element::Diode(diode_from_model_card(
        "D1",
        "out",
        "0",
        diode_model,
    )?));
    diode_circuit.add(Element::Capacitor(Capacitor::new(
        "Cstore",
        "out",
        "0",
        storage_capacitance_f,
    )));

    let bjt_model = models
        .get("Qsmall")
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "device_model_charge_audit_fixtures".to_string(),
            reason: "missing Qsmall model fixture".to_string(),
        })?;
    let mut bjt_circuit = Circuit::new();
    bjt_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vcc", "vcc", "0", 5.0,
    )));
    bjt_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vbase", "base", "0", 0.72,
    )));
    bjt_circuit.add(Element::Resistor(Resistor::new(
        "Rload", "out", "0", 1_000.0,
    )));
    bjt_circuit.add(Element::Bjt(bjt_from_model_card(
        "Q1", "vcc", "base", "out", bjt_model,
    )?));
    bjt_circuit.add(Element::Capacitor(Capacitor::new(
        "Cstore",
        "out",
        "0",
        storage_capacitance_f,
    )));

    let jfet_model = normalize_model_card(
        "Jn",
        "NJF",
        &[
            ("BETA", 9.0e-4),
            ("VTO", -1.8),
            ("LAMBDA", 0.02),
            ("CGS", 2.0e-11),
            ("CGD", 5.0e-12),
        ],
    )?;
    let mut jfet_circuit = Circuit::new();
    jfet_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 10.0,
    )));
    jfet_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vg", "gate", "0", 0.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rd", "vdd", "drain", 2_000.0,
    )));
    jfet_circuit.add(Element::Resistor(Resistor::new(
        "Rs", "source", "0", 1_000.0,
    )));
    jfet_circuit.add(Element::Jfet(jfet_from_model_card(
        "J1",
        "drain",
        "gate",
        "source",
        &jfet_model,
    )?));
    jfet_circuit.add(Element::Capacitor(Capacitor::new(
        "Cstore",
        "source",
        "0",
        storage_capacitance_f,
    )));

    let mos_model = normalize_model_card(
        "Mn",
        "NMOS",
        &[
            ("LEVEL", 1.0),
            ("VTO", 0.55),
            ("LAMBDA", 0.04),
            ("NSUB", 1.6),
            ("CGSO", 2.0e-11),
            ("CGDO", 5.0e-12),
            ("CGBO", 1.0e-12),
            ("CBS", 4.0e-13),
            ("CBD", 3.0e-13),
            ("PB", 0.9),
            ("MJ", 0.45),
        ],
    )?;
    let mut mos_circuit = Circuit::new();
    mos_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vdd", "vdd", "0", 1.8,
    )));
    mos_circuit.add(Element::VoltageSource(VoltageSource::new(
        "Vgate", "gate", "0", 1.8,
    )));
    mos_circuit.add(Element::Resistor(Resistor::new(
        "Rload", "vdd", "out", 1_000.0,
    )));
    mos_circuit.add(Element::Mosfet(mosfet_from_model_card(
        "M1", "out", "gate", "0", "0", &mos_model,
    )?));
    mos_circuit.add(Element::Capacitor(Capacitor::new(
        "Cstore",
        "out",
        "0",
        storage_capacitance_f,
    )));

    Ok(vec![
        DeviceModelChargeBehaviorFixture {
            name: "diode-storage-charge".to_string(),
            kind: diode_model.kind,
            model: diode_model.clone(),
            circuit: diode_circuit,
            probe_node: "out".to_string(),
            time_step_s,
            stop_time_s,
            storage_capacitance_f,
            expected_initial_min: -1.0e-9,
            expected_initial_max: 1.0,
            expected_final_min: 0.58,
            expected_final_max: 0.61,
            charge_behavior: "diode CJO/TT contribute transient anode-cathode storage; explicit Cstore keeps the fixture comparable with other charge audits".to_string(),
            deck_lines: vec![
                "* device-model charge fixture: diode-storage-charge".to_string(),
                ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)".to_string(),
                "Vbias vin 0 0.8".to_string(),
                "Rlimit vin out 1k".to_string(),
                "D1 out 0 Dfast".to_string(),
                "Cstore out 0 100p".to_string(),
                ".tran 20n 2u".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelChargeBehaviorFixture {
            name: "bjt-storage-charge".to_string(),
            kind: bjt_model.kind,
            model: bjt_model.clone(),
            circuit: bjt_circuit,
            probe_node: "out".to_string(),
            time_step_s,
            stop_time_s,
            storage_capacitance_f,
            expected_initial_min: -1.0e-9,
            expected_initial_max: 1.0,
            expected_final_min: 0.10,
            expected_final_max: 0.14,
            charge_behavior: "BJT CJE/CJC/TF/TR contribute transient base-emitter and base-collector storage; explicit Cstore keeps the fixture comparable with other charge audits".to_string(),
            deck_lines: vec![
                "* device-model charge fixture: bjt-storage-charge".to_string(),
                ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)".to_string(),
                "Vcc vcc 0 5".to_string(),
                "Vbase base 0 0.72".to_string(),
                "Q1 vcc base out Qsmall".to_string(),
                "Rload out 0 1k".to_string(),
                "Cstore out 0 100p".to_string(),
                ".tran 20n 2u".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelChargeBehaviorFixture {
            name: "jfet-storage-charge".to_string(),
            kind: jfet_model.kind,
            model: jfet_model.clone(),
            circuit: jfet_circuit,
            probe_node: "source".to_string(),
            time_step_s,
            stop_time_s,
            storage_capacitance_f,
            expected_initial_min: -1.0e-9,
            expected_initial_max: 1.0,
            expected_final_min: 0.86,
            expected_final_max: 0.90,
            charge_behavior: "JFET CGS/CGD contribute transient gate-source and gate-drain storage; explicit Cstore keeps the fixture comparable with other charge audits".to_string(),
            deck_lines: vec![
                "* device-model charge fixture: jfet-storage-charge".to_string(),
                ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02 CGS=20p CGD=5p)".to_string(),
                "Vdd vdd 0 10".to_string(),
                "Vg gate 0 0".to_string(),
                "Rd vdd drain 2k".to_string(),
                "Rs source 0 1k".to_string(),
                "J1 drain gate source Jn".to_string(),
                "Cstore source 0 100p".to_string(),
                ".tran 20n 2u".to_string(),
                ".save V(source)".to_string(),
                ".end".to_string(),
            ],
        },
        DeviceModelChargeBehaviorFixture {
            name: "mos-level1-storage-charge".to_string(),
            kind: mos_model.kind,
            model: mos_model,
            circuit: mos_circuit,
            probe_node: "out".to_string(),
            time_step_s,
            stop_time_s,
            storage_capacitance_f,
            expected_initial_min: -1.0e-9,
            expected_initial_max: 1.0,
            expected_final_min: 0.68,
            expected_final_max: 0.73,
            charge_behavior: "Level-1 MOS CGSO/CGDO/CGBO plus CBS/CBD contribute transient gate-overlap and depletion-shaped bulk-junction storage; explicit Cstore keeps the fixture comparable with other charge audits".to_string(),
            deck_lines: vec![
                "* device-model charge fixture: mos-level1-storage-charge".to_string(),
                ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CGSO=20p CGDO=5p CGBO=1p CBS=4e-13 CBD=3e-13 PB=0.9 MJ=0.45)".to_string(),
                "Vdd vdd 0 1.8".to_string(),
                "Vgate gate 0 1.8".to_string(),
                "Rload vdd out 1k".to_string(),
                "M1 out gate 0 0 Mn".to_string(),
                "Cstore out 0 100p".to_string(),
                ".tran 20n 2u".to_string(),
                ".save V(out)".to_string(),
                ".end".to_string(),
            ],
        },
    ])
}

pub fn device_model_reference_deck_audit_fixtures(
) -> Result<Vec<DeviceModelReferenceDeckAuditFixture>, SpiceError> {
    let reference = "SPICE2/SPICE3-style local model-depth fixture".to_string();
    let mut fixtures = Vec::new();
    for fixture in device_model_behavior_audit_fixtures()? {
        fixtures.push(DeviceModelReferenceDeckAuditFixture {
            name: format!("{}:op", fixture.name),
            kind: fixture.kind,
            model: fixture.model,
            analysis: "op".to_string(),
            reference: reference.clone(),
            expected_behavior: format!(
                "DC probe {} remains in [{}, {}] V",
                fixture.probe_node, fixture.expected_min, fixture.expected_max
            ),
            deck_lines: fixture.deck_lines,
        });
    }
    for fixture in device_model_temperature_audit_fixtures()? {
        fixtures.push(DeviceModelReferenceDeckAuditFixture {
            name: format!("{}:temperature", fixture.name),
            kind: fixture.kind,
            model: fixture.model,
            analysis: "temperature".to_string(),
            reference: reference.clone(),
            expected_behavior: fixture.temperature_behavior,
            deck_lines: fixture.deck_lines,
        });
    }
    for fixture in device_model_capacitance_audit_fixtures()? {
        fixtures.push(DeviceModelReferenceDeckAuditFixture {
            name: format!("{}:ac", fixture.name),
            kind: fixture.kind,
            model: fixture.model,
            analysis: "ac".to_string(),
            reference: reference.clone(),
            expected_behavior: fixture.capacitance_behavior,
            deck_lines: fixture.deck_lines,
        });
    }
    for fixture in device_model_noise_audit_fixtures()? {
        fixtures.push(DeviceModelReferenceDeckAuditFixture {
            name: format!("{}:noise", fixture.name),
            kind: fixture.kind,
            model: fixture.model,
            analysis: "noise".to_string(),
            reference: reference.clone(),
            expected_behavior: fixture.noise_behavior,
            deck_lines: fixture.deck_lines,
        });
    }
    for fixture in device_model_charge_audit_fixtures()? {
        fixtures.push(DeviceModelReferenceDeckAuditFixture {
            name: format!("{}:tran", fixture.name),
            kind: fixture.kind,
            model: fixture.model,
            analysis: "tran".to_string(),
            reference: reference.clone(),
            expected_behavior: fixture.charge_behavior,
            deck_lines: fixture.deck_lines,
        });
    }
    Ok(fixtures)
}

pub fn format_device_model_reference_deck_audit_table(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    let mut lines =
        vec!["name\tkind\tanalysis\tmodel\treference\texpected_behavior\tdeck_lines".to_string()];
    for fixture in fixtures {
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            fixture.name,
            fixture.kind.as_str(),
            fixture.analysis,
            fixture.model.name,
            fixture.reference,
            fixture.expected_behavior,
            fixture.deck_lines.len()
        ));
    }
    lines.join("\n")
}

pub fn device_model_reference_deck_audit_records(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_device_model_reference_deck_audit_table(fixtures))
}

pub fn format_device_model_reference_deck_audit_csv(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_csv(&format_device_model_reference_deck_audit_table(fixtures))
}

pub fn format_device_model_reference_deck_audit_json(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_json(&format_device_model_reference_deck_audit_table(fixtures))
}

pub fn device_model_reference_deck_audit_summary(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> Vec<DeviceModelReferenceDeckAuditSummary> {
    let expected_kinds = REFERENCE_DECK_AUDIT_EXPECTED_KINDS
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect::<Vec<_>>();
    let mut kinds = expected_kinds.clone();
    for kind in fixtures
        .iter()
        .map(|fixture| fixture.kind.as_str().to_string())
        .collect::<BTreeSet<_>>()
    {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }

    kinds
        .into_iter()
        .map(|kind| {
            let kind_rows = fixtures
                .iter()
                .filter(|fixture| fixture.kind.as_str() == kind)
                .collect::<Vec<_>>();
            let row_analyses = kind_rows
                .iter()
                .map(|fixture| fixture.analysis.clone())
                .collect::<BTreeSet<_>>();
            let mut analyses = REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES
                .iter()
                .filter(|analysis| row_analyses.contains::<str>(*analysis))
                .map(|analysis| analysis.to_string())
                .collect::<Vec<_>>();
            analyses.extend(
                row_analyses
                    .iter()
                    .filter(|analysis| {
                        !REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES.contains(&analysis.as_str())
                    })
                    .cloned(),
            );
            let missing_analyses = if expected_kinds.contains(&kind) {
                REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES
                    .iter()
                    .filter(|analysis| !row_analyses.contains::<str>(*analysis))
                    .map(|analysis| analysis.to_string())
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let mut references = Vec::new();
            for fixture in &kind_rows {
                if !fixture.reference.is_empty() && !references.contains(&fixture.reference) {
                    references.push(fixture.reference.clone());
                }
            }

            DeviceModelReferenceDeckAuditSummary {
                kind,
                fixture_count: kind_rows.len(),
                analyses,
                missing_analyses,
                deck_line_count: kind_rows
                    .iter()
                    .map(|fixture| fixture.deck_lines.len())
                    .sum(),
                references,
            }
        })
        .collect()
}

pub fn format_device_model_reference_deck_audit_summary_table(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    let mut lines =
        vec!["kind\tfixture_count\tanalyses\tmissing_analyses\tdeck_lines\treferences".to_string()];
    for summary in device_model_reference_deck_audit_summary(fixtures) {
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            summary.kind,
            summary.fixture_count,
            summary.analyses.join(","),
            summary.missing_analyses.join(","),
            summary.deck_line_count,
            summary.references.join(",")
        ));
    }
    lines.join("\n")
}

pub fn device_model_reference_deck_audit_summary_records(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_device_model_reference_deck_audit_summary_table(
        fixtures,
    ))
}

pub fn format_device_model_reference_deck_audit_summary_csv(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_csv(&format_device_model_reference_deck_audit_summary_table(
        fixtures,
    ))
}

pub fn format_device_model_reference_deck_audit_summary_json(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_json(&format_device_model_reference_deck_audit_summary_table(
        fixtures,
    ))
}

pub fn device_model_reference_deck_audit_analysis_summary(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> Vec<DeviceModelReferenceDeckAuditAnalysisSummary> {
    let expected_kinds = REFERENCE_DECK_AUDIT_EXPECTED_KINDS
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect::<Vec<_>>();
    let expected_analyses = REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES
        .iter()
        .map(|analysis| analysis.to_string())
        .collect::<Vec<_>>();
    let mut analyses = expected_analyses.clone();
    for analysis in fixtures
        .iter()
        .map(|fixture| fixture.analysis.clone())
        .collect::<BTreeSet<_>>()
    {
        if !analyses.contains(&analysis) {
            analyses.push(analysis);
        }
    }

    analyses
        .into_iter()
        .map(|analysis| {
            let analysis_rows = fixtures
                .iter()
                .filter(|fixture| fixture.analysis == analysis)
                .collect::<Vec<_>>();
            let row_kinds = analysis_rows
                .iter()
                .map(|fixture| fixture.kind.as_str().to_string())
                .collect::<BTreeSet<_>>();
            let mut kinds = expected_kinds
                .iter()
                .filter(|kind| row_kinds.contains(*kind))
                .cloned()
                .collect::<Vec<_>>();
            kinds.extend(
                row_kinds
                    .iter()
                    .filter(|kind| !expected_kinds.contains(kind))
                    .cloned(),
            );
            let missing_kinds = if expected_analyses.contains(&analysis) {
                expected_kinds
                    .iter()
                    .filter(|kind| !row_kinds.contains(*kind))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let mut references = Vec::new();
            for fixture in &analysis_rows {
                if !fixture.reference.is_empty() && !references.contains(&fixture.reference) {
                    references.push(fixture.reference.clone());
                }
            }

            DeviceModelReferenceDeckAuditAnalysisSummary {
                analysis,
                fixture_count: analysis_rows.len(),
                kinds,
                missing_kinds,
                deck_line_count: analysis_rows
                    .iter()
                    .map(|fixture| fixture.deck_lines.len())
                    .sum(),
                references,
            }
        })
        .collect()
}

pub fn format_device_model_reference_deck_audit_analysis_summary_table(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    let mut lines =
        vec!["analysis\tfixture_count\tkinds\tmissing_kinds\tdeck_lines\treferences".to_string()];
    for summary in device_model_reference_deck_audit_analysis_summary(fixtures) {
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            summary.analysis,
            summary.fixture_count,
            summary.kinds.join(","),
            summary.missing_kinds.join(","),
            summary.deck_line_count,
            summary.references.join(",")
        ));
    }
    lines.join("\n")
}

pub fn device_model_reference_deck_audit_analysis_summary_records(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_device_model_reference_deck_audit_analysis_summary_table(fixtures))
}

pub fn format_device_model_reference_deck_audit_analysis_summary_csv(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_csv(
        &format_device_model_reference_deck_audit_analysis_summary_table(fixtures),
    )
}

pub fn format_device_model_reference_deck_audit_analysis_summary_json(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_json(
        &format_device_model_reference_deck_audit_analysis_summary_table(fixtures),
    )
}

pub fn device_model_reference_deck_audit_matrix(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> Vec<DeviceModelReferenceDeckAuditMatrixRow> {
    let expected_kinds = REFERENCE_DECK_AUDIT_EXPECTED_KINDS
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect::<Vec<_>>();
    let expected_analyses = REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES
        .iter()
        .map(|analysis| analysis.to_string())
        .collect::<Vec<_>>();
    let mut kinds = expected_kinds.clone();
    for kind in fixtures
        .iter()
        .map(|fixture| fixture.kind.as_str().to_string())
        .collect::<BTreeSet<_>>()
    {
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }

    let analysis_names =
        |kind_rows: &[&DeviceModelReferenceDeckAuditFixture], analysis: &str| -> String {
            kind_rows
                .iter()
                .filter(|fixture| fixture.analysis == analysis)
                .map(|fixture| fixture.name.clone())
                .collect::<Vec<_>>()
                .join(",")
        };

    kinds
        .into_iter()
        .map(|kind| {
            let kind_rows = fixtures
                .iter()
                .filter(|fixture| fixture.kind.as_str() == kind)
                .collect::<Vec<_>>();
            let row_analyses = kind_rows
                .iter()
                .map(|fixture| fixture.analysis.clone())
                .collect::<BTreeSet<_>>();
            let missing_analyses = if expected_kinds.contains(&kind) {
                expected_analyses
                    .iter()
                    .filter(|analysis| !row_analyses.contains(*analysis))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let extra_analyses = row_analyses
                .iter()
                .filter(|analysis| !expected_analyses.contains(*analysis))
                .cloned()
                .collect::<Vec<_>>();

            DeviceModelReferenceDeckAuditMatrixRow {
                kind,
                fixture_count: kind_rows.len(),
                op: analysis_names(&kind_rows, "op"),
                temperature: analysis_names(&kind_rows, "temperature"),
                ac: analysis_names(&kind_rows, "ac"),
                noise: analysis_names(&kind_rows, "noise"),
                tran: analysis_names(&kind_rows, "tran"),
                missing_analyses,
                extra_analyses,
                deck_line_count: kind_rows
                    .iter()
                    .map(|fixture| fixture.deck_lines.len())
                    .sum(),
            }
        })
        .collect()
}

pub fn format_device_model_reference_deck_audit_matrix_table(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    let mut lines = vec![
        "kind\tfixture_count\top\ttemperature\tac\tnoise\ttran\tmissing_analyses\textra_analyses\tdeck_lines"
            .to_string(),
    ];
    for row in device_model_reference_deck_audit_matrix(fixtures) {
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.kind,
            row.fixture_count,
            row.op,
            row.temperature,
            row.ac,
            row.noise,
            row.tran,
            row.missing_analyses.join(","),
            row.extra_analyses.join(","),
            row.deck_line_count
        ));
    }
    lines.join("\n")
}

pub fn device_model_reference_deck_audit_matrix_records(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_device_model_reference_deck_audit_matrix_table(
        fixtures,
    ))
}

pub fn format_device_model_reference_deck_audit_matrix_csv(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_csv(&format_device_model_reference_deck_audit_matrix_table(
        fixtures,
    ))
}

pub fn format_device_model_reference_deck_audit_matrix_json(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> String {
    format_deck_table_json(&format_device_model_reference_deck_audit_matrix_table(
        fixtures,
    ))
}

pub fn device_model_reference_deck_audit_gate(
    fixtures: &[DeviceModelReferenceDeckAuditFixture],
) -> DeviceModelReferenceDeckAuditGateReport {
    let expected_kinds = REFERENCE_DECK_AUDIT_EXPECTED_KINDS
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect::<Vec<_>>();
    let expected_analyses = REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES
        .iter()
        .map(|analysis| analysis.to_string())
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    let mut seen_names = HashSet::new();
    let mut seen_pairs = HashSet::new();

    if fixtures.is_empty() {
        issues.push(DeviceModelReferenceDeckAuditIssue {
            fixture_name: "audit_matrix".to_string(),
            field: "fixture_count".to_string(),
            message: "audit matrix must contain at least one reference-deck row".to_string(),
        });
    }

    for fixture in fixtures {
        let fixture_name = if fixture.name.is_empty() {
            "<missing>".to_string()
        } else {
            fixture.name.clone()
        };

        if !seen_names.insert(fixture.name.clone()) {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "name".to_string(),
                message: "reference-deck audit fixture names must be unique".to_string(),
            });
        }
        if fixture.name.trim().is_empty() {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "name".to_string(),
                message: "field must be documented and non-empty".to_string(),
            });
        }
        if !REFERENCE_DECK_AUDIT_EXPECTED_KINDS.contains(&fixture.kind) {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "kind".to_string(),
                message: format!(
                    "unsupported reference-deck audit kind {:?}",
                    fixture.kind.as_str()
                ),
            });
        }
        if !REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES.contains(&fixture.analysis.as_str()) {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "analysis".to_string(),
                message: format!(
                    "unsupported reference-deck audit analysis {:?}",
                    fixture.analysis
                ),
            });
        }
        seen_pairs.insert((fixture.kind.as_str().to_string(), fixture.analysis.clone()));
        if fixture.model.name.trim().is_empty() {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "model.name".to_string(),
                message: "field must be documented and non-empty".to_string(),
            });
        }
        if fixture.reference.trim().is_empty() {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "reference".to_string(),
                message: "field must be documented and non-empty".to_string(),
            });
        }
        if fixture.expected_behavior.trim().is_empty() {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "expected_behavior".to_string(),
                message: "field must be documented and non-empty".to_string(),
            });
        }
        if fixture.deck_lines.is_empty() {
            issues.push(DeviceModelReferenceDeckAuditIssue {
                fixture_name: fixture_name.clone(),
                field: "deck_lines".to_string(),
                message: "reference deck must contain active deck lines".to_string(),
            });
        } else {
            if !fixture.deck_lines[0].starts_with("* device-model ") {
                issues.push(DeviceModelReferenceDeckAuditIssue {
                    fixture_name: fixture_name.clone(),
                    field: "deck_lines[0]".to_string(),
                    message: "reference deck must start with a device-model comment".to_string(),
                });
            }
            if !fixture
                .deck_lines
                .iter()
                .any(|line| line.starts_with(".model "))
            {
                issues.push(DeviceModelReferenceDeckAuditIssue {
                    fixture_name: fixture_name.clone(),
                    field: "deck_lines".to_string(),
                    message: "reference deck must include a .model card".to_string(),
                });
            }
            if fixture.deck_lines.last().map(String::as_str) != Some(".end") {
                issues.push(DeviceModelReferenceDeckAuditIssue {
                    fixture_name: fixture_name.clone(),
                    field: "deck_lines[-1]".to_string(),
                    message: "reference deck must end with .end".to_string(),
                });
            }
        }
    }

    for kind in REFERENCE_DECK_AUDIT_EXPECTED_KINDS {
        for analysis in REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES {
            let kind_text = kind.as_str();
            if !seen_pairs.contains(&(kind_text.to_string(), analysis.to_string())) {
                issues.push(DeviceModelReferenceDeckAuditIssue {
                    fixture_name: format!("{}:{}", kind_text, analysis),
                    field: "coverage".to_string(),
                    message: format!(
                        "missing required {} {} reference-deck audit row",
                        kind_text, analysis
                    ),
                });
            }
        }
    }

    DeviceModelReferenceDeckAuditGateReport {
        passed: issues.is_empty(),
        fixture_count: fixtures.len(),
        expected_kinds,
        expected_analyses,
        issues,
    }
}

pub fn format_device_model_reference_deck_audit_gate_report(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    let mut lines = vec![
        "passed\tfixture_count\texpected_kinds\texpected_analyses\tissue_count".to_string(),
        format!(
            "{}\t{}\t{}\t{}\t{}",
            report.passed,
            report.fixture_count,
            report.expected_kinds.join(","),
            report.expected_analyses.join(","),
            report.issues.len()
        ),
    ];
    if !report.issues.is_empty() {
        lines.push("fixture_name\tfield\tmessage".to_string());
        for issue in &report.issues {
            lines.push(format!(
                "{}\t{}\t{}",
                issue.fixture_name, issue.field, issue.message
            ));
        }
    }
    lines.join("\n")
}

pub fn device_model_reference_deck_audit_gate_coverage_digest(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> DeviceModelReferenceDeckAuditGateCoverageDigest {
    let expected_pair_count = report.expected_kinds.len() * report.expected_analyses.len();
    let missing_pair_count = report
        .issues
        .iter()
        .filter(|issue| issue.field == "coverage")
        .count();
    let issue_fields = report
        .issues
        .iter()
        .map(|issue| issue.field.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    DeviceModelReferenceDeckAuditGateCoverageDigest {
        passed: report.passed,
        fixture_count: report.fixture_count,
        expected_pair_count,
        covered_pair_count: expected_pair_count.saturating_sub(missing_pair_count),
        missing_pair_count,
        issue_count: report.issues.len(),
        issue_fields,
    }
}

pub fn format_device_model_reference_deck_audit_gate_coverage_digest_table(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    let digest = device_model_reference_deck_audit_gate_coverage_digest(report);
    [
        "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\tmissing_pair_count\tissue_count\tissue_fields".to_string(),
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            digest.passed,
            digest.fixture_count,
            digest.expected_pair_count,
            digest.covered_pair_count,
            digest.missing_pair_count,
            digest.issue_count,
            digest.issue_fields.join(",")
        ),
    ]
    .join("\n")
}

pub fn device_model_reference_deck_audit_gate_coverage_digest_records(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_device_model_reference_deck_audit_gate_coverage_digest_table(report))
}

pub fn format_device_model_reference_deck_audit_gate_coverage_digest_csv(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    format_deck_table_csv(
        &format_device_model_reference_deck_audit_gate_coverage_digest_table(report),
    )
}

pub fn format_device_model_reference_deck_audit_gate_coverage_digest_json(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    format_deck_table_json(
        &format_device_model_reference_deck_audit_gate_coverage_digest_table(report),
    )
}

pub fn format_device_model_reference_deck_audit_gate_issue_table(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    let mut lines = vec!["fixture_name\tfield\tmessage".to_string()];
    for issue in &report.issues {
        lines.push(format!(
            "{}\t{}\t{}",
            issue.fixture_name, issue.field, issue.message
        ));
    }
    lines.join("\n")
}

pub fn device_model_reference_deck_audit_gate_issue_records(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_device_model_reference_deck_audit_gate_issue_table(
        report,
    ))
}

pub fn format_device_model_reference_deck_audit_gate_issue_csv(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    format_deck_table_csv(&format_device_model_reference_deck_audit_gate_issue_table(
        report,
    ))
}

pub fn format_device_model_reference_deck_audit_gate_issue_json(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    format_deck_table_json(&format_device_model_reference_deck_audit_gate_issue_table(
        report,
    ))
}

pub fn device_model_reference_deck_audit_gate_issue_summary(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> Vec<DeviceModelReferenceDeckAuditGateIssueSummary> {
    let mut groups: BTreeMap<String, Vec<&DeviceModelReferenceDeckAuditIssue>> = BTreeMap::new();
    for issue in &report.issues {
        groups.entry(issue.field.clone()).or_default().push(issue);
    }

    groups
        .into_iter()
        .map(|(field, issues)| {
            let mut fixture_names = Vec::new();
            let mut messages = Vec::new();
            for issue in &issues {
                if !fixture_names.contains(&issue.fixture_name) {
                    fixture_names.push(issue.fixture_name.clone());
                }
                if !messages.contains(&issue.message) {
                    messages.push(issue.message.clone());
                }
            }
            DeviceModelReferenceDeckAuditGateIssueSummary {
                field,
                issue_count: issues.len(),
                fixture_names,
                messages,
            }
        })
        .collect()
}

pub fn format_device_model_reference_deck_audit_gate_issue_summary_table(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    let mut lines = vec!["field\tissue_count\tfixture_names\tmessages".to_string()];
    for summary in device_model_reference_deck_audit_gate_issue_summary(report) {
        lines.push(format!(
            "{}\t{}\t{}\t{}",
            summary.field,
            summary.issue_count,
            summary.fixture_names.join(","),
            summary.messages.join(",")
        ));
    }
    lines.join("\n")
}

pub fn device_model_reference_deck_audit_gate_issue_summary_records(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> Vec<BTreeMap<String, String>> {
    deck_table_records(&format_device_model_reference_deck_audit_gate_issue_summary_table(report))
}

pub fn format_device_model_reference_deck_audit_gate_issue_summary_csv(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    format_deck_table_csv(
        &format_device_model_reference_deck_audit_gate_issue_summary_table(report),
    )
}

pub fn format_device_model_reference_deck_audit_gate_issue_summary_json(
    report: &DeviceModelReferenceDeckAuditGateReport,
) -> String {
    format_deck_table_json(
        &format_device_model_reference_deck_audit_gate_issue_summary_table(report),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityOracle {
    pub reference: String,
    pub version: String,
    pub source: String,
}

impl CompatibilityOracle {
    pub fn new(
        reference: impl Into<String>,
        version: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            reference: reference.into(),
            version: version.into(),
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibilityGoldenValue {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
}

impl CompatibilityGoldenValue {
    pub fn new(
        name: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
        absolute_tolerance: f64,
        relative_tolerance: f64,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            unit: unit.into(),
            absolute_tolerance,
            relative_tolerance,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompatibilityDeck {
    pub id: String,
    pub title: String,
    pub analysis: String,
    pub netlist: String,
    pub oracle: CompatibilityOracle,
    pub golden_values: Vec<CompatibilityGoldenValue>,
    pub known_incompatibilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckControlDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckControlSummary {
    pub active_lines: Vec<String>,
    pub control_lines: Vec<String>,
    pub write_markers: Vec<String>,
    pub rawfile_options: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub diagnostics: Vec<DeckControlDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckResolutionDiagnostic {
    pub code: String,
    pub directive: String,
    pub source: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckResolutionSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub diagnostics: Vec<DeckResolutionDiagnostic>,
    pub included_paths: Vec<String>,
    pub library_sections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckParameterValue {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckParameterDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub parameter: Option<String>,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckParameterSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub parameters: Vec<DeckParameterValue>,
    pub diagnostics: Vec<DeckParameterDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckNodeCondition {
    pub directive: String,
    pub node: String,
    pub value: f64,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckInitialConditionDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckInitialConditionSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub initial_conditions: Vec<DeckNodeCondition>,
    pub nodesets: Vec<DeckNodeCondition>,
    pub diagnostics: Vec<DeckInitialConditionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckFunctionDefinition {
    pub name: String,
    pub arguments: Vec<String>,
    pub expression: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckFunctionDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub function_name: Option<String>,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckFunctionSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub functions: Vec<DeckFunctionDefinition>,
    pub diagnostics: Vec<DeckFunctionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckMeasurementCard {
    pub directive: String,
    pub analysis: String,
    pub name: String,
    pub mode: String,
    pub probe: String,
    pub line_number: usize,
    pub from_value: Option<f64>,
    pub to_value: Option<f64>,
    pub at_value: Option<f64>,
    pub target_value: Option<f64>,
    pub crossing_kind: Option<String>,
    pub crossing_count: Option<usize>,
    pub trigger_probe: Option<String>,
    pub trigger_value: Option<f64>,
    pub trigger_crossing_kind: Option<String>,
    pub trigger_crossing_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckMeasurementDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckMeasurementSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub measurements: Vec<DeckMeasurementCard>,
    pub diagnostics: Vec<DeckMeasurementDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckFourierCard {
    pub directive: String,
    pub fundamental_frequency_hz: f64,
    pub probes: Vec<String>,
    pub line_number: usize,
    pub harmonics: Option<usize>,
    pub from_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckFourierDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckFourierSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub fourier: Vec<DeckFourierCard>,
    pub diagnostics: Vec<DeckFourierDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckOutputSelection {
    pub directive: String,
    pub analysis: Option<String>,
    pub probes: Vec<String>,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckOutputDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckOutputSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub selections: Vec<DeckOutputSelection>,
    pub diagnostics: Vec<DeckOutputDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckAnalysisPlan {
    pub directive: String,
    pub analysis: String,
    pub line_number: usize,
    pub source_name: Option<String>,
    pub output_node: Option<String>,
    pub start_value: Option<f64>,
    pub stop_value: Option<f64>,
    pub step_value: Option<f64>,
    pub sweep_kind: Option<String>,
    pub point_count: Option<usize>,
    pub start_frequency_hz: Option<f64>,
    pub stop_frequency_hz: Option<f64>,
    pub step_time: Option<f64>,
    pub stop_time: Option<f64>,
    pub start_time: Option<f64>,
    pub max_step: Option<f64>,
    pub use_initial_conditions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckAnalysisDiagnostic {
    pub code: String,
    pub directive: String,
    pub line_number: usize,
    pub message: String,
    pub severity: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckAnalysisSummary {
    pub active_lines: Vec<String>,
    pub terminated: bool,
    pub end_line_number: Option<usize>,
    pub analyses: Vec<DeckAnalysisPlan>,
    pub diagnostics: Vec<DeckAnalysisDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeckAnalysisExecutionResult {
    Op(DcResult),
    DcSweep(Vec<DcSweepPoint>),
    Ac(Vec<AcPoint>),
    Tran(Vec<TransientPoint>),
    Tf(TfResult),
    Sens(SensResult),
    Noise(NoiseResult),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckRunArtifact {
    pub analysis: String,
    pub directive: String,
    pub analysis_directive_count: usize,
    pub analysis_directives: Vec<String>,
    pub deck_analysis_kind_count: usize,
    pub deck_analysis_kinds: Vec<String>,
    pub deck_analysis_directive_count: usize,
    pub deck_analysis_directives: Vec<String>,
    pub line_number: usize,
    pub source_name: Option<String>,
    pub output_node: Option<String>,
    pub sweep_kind: Option<String>,
    pub start_value: Option<f64>,
    pub stop_value: Option<f64>,
    pub step_value: Option<f64>,
    pub point_count: Option<usize>,
    pub start_frequency_hz: Option<f64>,
    pub stop_frequency_hz: Option<f64>,
    pub step_time: Option<f64>,
    pub stop_time: Option<f64>,
    pub start_time: Option<f64>,
    pub max_step: Option<f64>,
    pub use_initial_conditions: Option<bool>,
    pub result_rows: usize,
    pub result_column_count: usize,
    pub result_columns: Vec<String>,
    pub table_count: usize,
    pub tables: Vec<String>,
    pub output_probe_count: usize,
    pub output_probes: Vec<String>,
    pub output_directive_count: usize,
    pub output_directives: Vec<String>,
    pub measurement_count: usize,
    pub measurement_names: Vec<String>,
    pub fourier_count: usize,
    pub fourier_probes: Vec<String>,
    pub control_line_count: usize,
    pub control_lines: Vec<String>,
    pub write_marker_count: usize,
    pub write_markers: Vec<String>,
    pub rawfile_option_count: usize,
    pub rawfile_options: Vec<String>,
    pub control_policy_artifact_count: usize,
    pub control_policy_categories: Vec<String>,
    pub control_policy_codes: Vec<String>,
    pub control_policy_severities: Vec<String>,
    pub diagnostic_count: usize,
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckTableArtifact {
    pub name: String,
    pub table: String,
    pub csv: String,
    pub json: String,
    pub records: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckOutputPlanArtifact {
    pub analysis: String,
    pub directive: String,
    pub line_number: usize,
    pub source_name: Option<String>,
    pub output_node: Option<String>,
    pub sweep_kind: Option<String>,
    pub start_value: Option<f64>,
    pub stop_value: Option<f64>,
    pub step_value: Option<f64>,
    pub point_count: Option<usize>,
    pub start_frequency_hz: Option<f64>,
    pub stop_frequency_hz: Option<f64>,
    pub step_time: Option<f64>,
    pub stop_time: Option<f64>,
    pub start_time: Option<f64>,
    pub max_step: Option<f64>,
    pub use_initial_conditions: Option<bool>,
    pub result_row_count: usize,
    pub result_column_count: usize,
    pub result_columns: Vec<String>,
    pub output_probe_count: usize,
    pub output_probes: Vec<String>,
    pub output_probe_line_count: usize,
    pub output_probe_lines: Vec<usize>,
    pub output_directive_count: usize,
    pub output_directives: Vec<String>,
    pub output_directive_kind_count: usize,
    pub output_directive_kinds: Vec<String>,
    pub output_directive_analysis_kind_count: usize,
    pub output_directive_analysis_kinds: Vec<String>,
    pub output_directive_line_count: usize,
    pub output_directive_lines: Vec<usize>,
    pub table_count: usize,
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckControlPolicyArtifact {
    pub line_number: usize,
    pub category: String,
    pub command: String,
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckControlPolicySummaryArtifact {
    pub category: String,
    pub artifact_count: usize,
    pub line_numbers: Vec<usize>,
    pub commands: Vec<String>,
    pub codes: Vec<String>,
    pub severities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckRawfileArtifact {
    pub target: String,
    pub marker: String,
    pub probe_count: usize,
    pub probes: Vec<String>,
    pub matched_probe_count: usize,
    pub matched_probes: Vec<String>,
    pub unmatched_probe_count: usize,
    pub unmatched_probes: Vec<String>,
    pub option_count: usize,
    pub options: Vec<String>,
    pub rawfile: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckWrdataArtifact {
    pub target: String,
    pub marker: String,
    pub probe_count: usize,
    pub probes: Vec<String>,
    pub matched_probe_count: usize,
    pub matched_probes: Vec<String>,
    pub unmatched_probe_count: usize,
    pub unmatched_probes: Vec<String>,
    pub option_count: usize,
    pub options: Vec<String>,
    pub datafile: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckAnalysisExecution {
    pub plan: DeckAnalysisPlan,
    pub result: DeckAnalysisExecutionResult,
    pub table: String,
    pub output_probes: Vec<String>,
    pub output_directives: Vec<String>,
    pub analysis_directives: Vec<String>,
    pub deck_analysis_kind_count: usize,
    pub deck_analysis_kinds: Vec<String>,
    pub deck_analysis_directive_count: usize,
    pub deck_analysis_directives: Vec<String>,
    pub output_plan_artifact_count: usize,
    pub output_plan_artifacts: Vec<DeckOutputPlanArtifact>,
    pub output_plan_artifact_table: String,
    pub output_plan_artifact_csv: String,
    pub output_plan_artifact_json: String,
    pub output_plan_artifact_records: Vec<BTreeMap<String, String>>,
    pub control_line_count: usize,
    pub control_lines: Vec<String>,
    pub write_marker_count: usize,
    pub write_markers: Vec<String>,
    pub rawfile_option_count: usize,
    pub rawfile_options: Vec<String>,
    pub control_policy_artifact_count: usize,
    pub control_policy_artifacts: Vec<DeckControlPolicyArtifact>,
    pub control_policy_artifact_table: String,
    pub control_policy_artifact_csv: String,
    pub control_policy_artifact_json: String,
    pub control_policy_artifact_records: Vec<BTreeMap<String, String>>,
    pub control_policy_summary_artifact_count: usize,
    pub control_policy_summary_artifacts: Vec<DeckControlPolicySummaryArtifact>,
    pub control_policy_summary_artifact_table: String,
    pub control_policy_summary_artifact_csv: String,
    pub control_policy_summary_artifact_json: String,
    pub control_policy_summary_artifact_records: Vec<BTreeMap<String, String>>,
    pub rawfile_artifact_count: usize,
    pub rawfile_artifacts: Vec<DeckRawfileArtifact>,
    pub rawfile_artifact_table: String,
    pub rawfile_artifact_csv: String,
    pub rawfile_artifact_json: String,
    pub rawfile_artifact_records: Vec<BTreeMap<String, String>>,
    pub wrdata_artifact_count: usize,
    pub wrdata_artifacts: Vec<DeckWrdataArtifact>,
    pub wrdata_artifact_table: String,
    pub wrdata_artifact_csv: String,
    pub wrdata_artifact_json: String,
    pub wrdata_artifact_records: Vec<BTreeMap<String, String>>,
    pub diagnostic_count: usize,
    pub diagnostic_codes: Vec<String>,
    pub table_count: usize,
    pub tables: Vec<String>,
    pub table_artifacts: Vec<DeckTableArtifact>,
    pub measurements: Vec<ProbeMeasurement>,
    pub measurement_table: String,
    pub fourier: Vec<FourierResult>,
    pub fourier_table: String,
    pub run_artifacts: Vec<DeckRunArtifact>,
    pub run_artifact_table: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeckExecution {
    pub execution_count: usize,
    pub analysis_order: Vec<String>,
    pub analysis_directives: Vec<String>,
    pub executions: Vec<DeckAnalysisExecution>,
    pub run_artifact_count: usize,
    pub run_artifacts: Vec<DeckRunArtifact>,
    pub run_artifact_table: String,
    pub run_artifact_csv: String,
    pub run_artifact_json: String,
    pub run_artifact_records: Vec<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseReadinessIssue {
    pub deck_id: String,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseReadinessReport {
    pub passed: bool,
    pub deck_count: usize,
    pub analyses: Vec<String>,
    pub issues: Vec<ReleaseReadinessIssue>,
}

pub fn compatibility_corpus() -> Vec<CompatibilityDeck> {
    let common = common_known_incompatibilities();
    vec![
        CompatibilityDeck {
            id: "dc-op-resistive-divider".to_string(),
            title: "DC operating point resistive divider".to_string(),
            analysis: "op".to_string(),
            netlist: "* dc-op-resistive-divider\nV1 in 0 DC 10\nR1 in out 10000\nR2 out 0 10000\n.op\n.end\n".to_string(),
            oracle: CompatibilityOracle::new(
                "closed-form",
                "divider-v1",
                "V(out)=V1*R2/(R1+R2); I(V1)=-V1/(R1+R2)",
            ),
            golden_values: vec![
                CompatibilityGoldenValue::new("V(out)", 5.0, "V", 1.0e-9, 1.0e-9),
                CompatibilityGoldenValue::new("I(V1)", -5.0e-4, "A", 1.0e-12, 1.0e-9),
            ],
            known_incompatibilities: common.clone(),
        },
        CompatibilityDeck {
            id: "dc-sweep-resistive-divider".to_string(),
            title: "DC source sweep resistive divider".to_string(),
            analysis: "dc".to_string(),
            netlist: "* dc-sweep-resistive-divider\nV1 in 0 DC 0\nR1 in out 10000\nR2 out 0 10000\n.dc V1 0 10 5\n.end\n".to_string(),
            oracle: CompatibilityOracle::new(
                "closed-form",
                "divider-sweep-v1",
                "V(out)=V1*0.5 at each sweep point",
            ),
            golden_values: vec![
                CompatibilityGoldenValue::new("points", 3.0, "count", 0.0, 0.0),
                CompatibilityGoldenValue::new("V(out)@V1=10", 5.0, "V", 1.0e-9, 1.0e-9),
            ],
            known_incompatibilities: common.clone(),
        },
        CompatibilityDeck {
            id: "ac-rc-lowpass".to_string(),
            title: "AC RC low-pass cutoff".to_string(),
            analysis: "ac".to_string(),
            netlist: "* ac-rc-lowpass\nV1 in 0 DC 0 AC 1\nR1 in out 1000\nC1 out 0 1u\n.ac dec 1 1 1k\n.end\n".to_string(),
            oracle: CompatibilityOracle::new(
                "closed-form",
                "rc-lowpass-v1",
                "|V(out)|=1/sqrt(1+(2*pi*f*R*C)^2)",
            ),
            golden_values: vec![
                CompatibilityGoldenValue::new("f_c", 159.15494309189535, "Hz", 1.0e-9, 1.0e-9),
                CompatibilityGoldenValue::new(
                    "|V(out)|@f_c",
                    0.7071067811865475,
                    "V",
                    1.0e-9,
                    1.0e-9,
                ),
            ],
            known_incompatibilities: common.clone(),
        },
        CompatibilityDeck {
            id: "tran-rc-step".to_string(),
            title: "Transient RC step response".to_string(),
            analysis: "tran".to_string(),
            netlist: "* tran-rc-step\nV1 in 0 PULSE(0 1 0 1n 1n 1m 2m)\nR1 in out 1000\nC1 out 0 1u\n.tran 0.0001 0.001\n.end\n".to_string(),
            oracle: CompatibilityOracle::new(
                "closed-form",
                "rc-step-v1",
                "V(out,t)=1-exp(-t/(R*C)) after an ideal 1 V step",
            ),
            golden_values: vec![CompatibilityGoldenValue::new(
                "V(out)@1ms",
                0.6321205588285577,
                "V",
                1.0e-6,
                1.0e-6,
            )],
            known_incompatibilities: common
                .iter()
                .cloned()
                .chain(std::iter::once(
                    "finite-edge pulse decks compare at the idealized step oracle point"
                        .to_string(),
                ))
                .collect(),
        },
        CompatibilityDeck {
            id: "tf-resistive-divider".to_string(),
            title: "Transfer-function resistive divider".to_string(),
            analysis: "tf".to_string(),
            netlist: "* tf-resistive-divider\nV1 in 0 DC 10\nR1 in out 10000\nR2 out 0 10000\n.tf V(out) V1\n.end\n".to_string(),
            oracle: CompatibilityOracle::new(
                "closed-form",
                "divider-tf-v1",
                "gain=R2/(R1+R2); input resistance=R1+R2",
            ),
            golden_values: vec![
                CompatibilityGoldenValue::new("gain", 0.5, "V/V", 1.0e-9, 1.0e-9),
                CompatibilityGoldenValue::new(
                    "input_resistance",
                    20000.0,
                    "ohm",
                    1.0e-6,
                    1.0e-9,
                ),
            ],
            known_incompatibilities: common,
        },
    ]
}

pub fn analyze_deck_controls(netlist: &str) -> DeckControlSummary {
    let mut active_lines = Vec::new();
    let mut control_lines = Vec::new();
    let mut write_markers = Vec::new();
    let mut rawfile_options = Vec::new();
    let mut diagnostics = Vec::new();
    let mut end_line_number = None;
    let mut in_control_block = false;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if in_control_block {
            if directive.as_deref() == Some(".endc") {
                in_control_block = false;
                continue;
            }
            if let Some(control_line) = control_block_command_as_deck_line(stripped) {
                active_lines.push(control_line.clone());
                control_lines.push(control_line);
                continue;
            }
            if let Some(write_marker) = control_block_write_marker(stripped) {
                write_markers.push(write_marker);
                continue;
            }
            if let Some(rawfile_option) = control_block_rawfile_option(stripped) {
                rawfile_options.push(rawfile_option);
                continue;
            }
            if is_noop_control_block_command(stripped) {
                continue;
            }
            if is_script_control_block_command(stripped) {
                diagnostics.push(DeckControlDiagnostic {
                    code: "SPICE_DECK_CONTROL_SCRIPT_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    line_number,
                    message: control_block_script_policy_message(stripped),
                    severity: "error".to_string(),
                });
                continue;
            }
            if is_workdir_control_block_command(stripped) {
                diagnostics.push(DeckControlDiagnostic {
                    code: "SPICE_DECK_CONTROL_WORKDIR_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    line_number,
                    message: control_block_workdir_policy_message(stripped),
                    severity: "error".to_string(),
                });
                continue;
            }
            if is_control_flow_control_block_command(stripped) {
                diagnostics.push(DeckControlDiagnostic {
                    code: "SPICE_DECK_CONTROL_FLOW_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    line_number,
                    message: control_block_flow_policy_message(stripped),
                    severity: "error".to_string(),
                });
                continue;
            }
            if is_variable_control_block_command(stripped) {
                diagnostics.push(DeckControlDiagnostic {
                    code: "SPICE_DECK_CONTROL_VARIABLE_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    line_number,
                    message: control_block_variable_policy_message(stripped),
                    severity: "error".to_string(),
                });
                continue;
            }
            diagnostics.push(DeckControlDiagnostic {
                code: "SPICE_DECK_CONTROL_COMMAND".to_string(),
                directive: ".control".to_string(),
                line_number,
                message: format!(
                    "{stripped:?} inside .control is not executed by the deck execution foothold yet"
                ),
                severity: "error".to_string(),
            });
            continue;
        }
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if let Some(directive) = directive {
            if is_unsupported_deck_control_directive(&directive) {
                diagnostics.push(DeckControlDiagnostic {
                    code: "SPICE_DECK_UNSUPPORTED_DIRECTIVE".to_string(),
                    directive: directive.clone(),
                    line_number,
                    message: format!(
                        "{directive} is not supported by the deck execution foothold yet"
                    ),
                    severity: "error".to_string(),
                });
                if directive == ".control" {
                    in_control_block = true;
                    continue;
                }
            }
        }
        active_lines.push(stripped.to_string());
    }

    DeckControlSummary {
        active_lines,
        control_lines,
        write_markers,
        rawfile_options,
        terminated: end_line_number.is_some(),
        end_line_number,
        diagnostics,
    }
}

pub fn resolve_deck_sources(
    netlist: &str,
    sources: &HashMap<String, String>,
) -> DeckResolutionSummary {
    let mut state = DeckResolutionState::new();
    let (active_lines, terminated, end_line_number) =
        resolve_deck_lines(netlist, "<deck>", sources, &mut state, &[]);

    DeckResolutionSummary {
        active_lines,
        terminated,
        end_line_number,
        diagnostics: state.diagnostics,
        included_paths: state.included_paths,
        library_sections: state.library_sections,
    }
}

pub fn resolve_deck_parameters(netlist: &str) -> DeckParameterSummary {
    let mut state = DeckParameterState::new();
    collect_parameter_functions(netlist, &mut state);
    let mut active_lines = Vec::new();
    let mut end_line_number = None;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if directive.as_deref() == Some(".param") {
            resolve_param_line(stripped, line_number, &mut state);
            continue;
        }
        if directive.as_deref() == Some(".func") {
            continue;
        }
        if let Some(directive) = directive {
            if is_unsupported_parameter_directive(&directive) {
                add_parameter_diagnostic(
                    &mut state,
                    "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
                    &directive,
                    line_number,
                    &format!("{directive} is not supported by the parameter resolver yet"),
                    None,
                    None,
                );
                active_lines.push(stripped.to_string());
                continue;
            }
        }
        active_lines.push(rewrite_parameter_expressions(
            stripped,
            line_number,
            &mut state,
        ));
    }

    DeckParameterSummary {
        active_lines,
        terminated: end_line_number.is_some(),
        end_line_number,
        parameters: state.parameter_values(),
        diagnostics: state.diagnostics,
    }
}

pub fn resolve_deck_initial_conditions(netlist: &str) -> DeckInitialConditionSummary {
    let mut state = DeckInitialConditionState::new();
    let mut active_lines = Vec::new();
    let mut end_line_number = None;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if let Some(directive) = directive.as_deref() {
            if matches!(directive, ".ic" | ".nodeset") {
                resolve_node_condition_line(stripped, line_number, directive, &mut state);
                continue;
            }
        }
        active_lines.push(stripped.to_string());
    }

    DeckInitialConditionSummary {
        active_lines,
        terminated: end_line_number.is_some(),
        end_line_number,
        initial_conditions: state.initial_conditions,
        nodesets: state.nodesets,
        diagnostics: state.diagnostics,
    }
}

pub fn resolve_deck_functions(netlist: &str) -> DeckFunctionSummary {
    let mut state = DeckFunctionState::new();
    let mut active_lines = Vec::new();
    let mut end_line_number = None;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if directive.as_deref() == Some(".func") {
            resolve_function_line(stripped, line_number, &mut state);
            continue;
        }
        active_lines.push(stripped.to_string());
    }

    DeckFunctionSummary {
        active_lines,
        terminated: end_line_number.is_some(),
        end_line_number,
        functions: state.functions,
        diagnostics: state.diagnostics,
    }
}

pub fn resolve_deck_measurements(netlist: &str) -> DeckMeasurementSummary {
    let mut state = DeckMeasurementState::new();
    let mut active_lines = Vec::new();
    let mut end_line_number = None;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if matches!(directive.as_deref(), Some(".measure" | ".meas")) {
            resolve_measurement_line(
                stripped,
                line_number,
                directive.as_deref().unwrap(),
                &mut state,
            );
            continue;
        }
        active_lines.push(stripped.to_string());
    }

    DeckMeasurementSummary {
        active_lines,
        terminated: end_line_number.is_some(),
        end_line_number,
        measurements: state.measurements,
        diagnostics: state.diagnostics,
    }
}

pub fn resolve_deck_fourier(netlist: &str) -> DeckFourierSummary {
    let mut state = DeckFourierState::new();
    let mut active_lines = Vec::new();
    let mut end_line_number = None;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if directive.as_deref() == Some(".four") {
            resolve_fourier_line(stripped, line_number, &mut state);
            continue;
        }
        active_lines.push(stripped.to_string());
    }

    DeckFourierSummary {
        active_lines,
        terminated: end_line_number.is_some(),
        end_line_number,
        fourier: state.fourier,
        diagnostics: state.diagnostics,
    }
}

pub fn resolve_deck_outputs(netlist: &str) -> DeckOutputSummary {
    let mut state = DeckOutputState::new();
    let mut active_lines = Vec::new();
    let mut end_line_number = None;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if matches!(
            directive.as_deref(),
            Some(".save" | ".probe" | ".print" | ".plot")
        ) {
            resolve_output_line(
                stripped,
                line_number,
                directive.as_deref().unwrap(),
                &mut state,
            );
            continue;
        }
        active_lines.push(stripped.to_string());
    }

    DeckOutputSummary {
        active_lines,
        terminated: end_line_number.is_some(),
        end_line_number,
        selections: state.selections,
        diagnostics: state.diagnostics,
    }
}

pub fn select_deck_output_probes(netlist: &str, analysis: &str) -> Result<Vec<String>, SpiceError> {
    let summary = resolve_deck_outputs(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "select_deck_output_probes",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for selection in summary.selections {
        if !selection
            .analysis
            .as_deref()
            .is_none_or(|requested| deck_output_analysis_matches(requested, analysis))
        {
            continue;
        }
        for probe in selection.probes {
            let key = deck_output_probe_key(&probe);
            if seen.insert(key) {
                selected.push(probe);
            }
        }
    }
    Ok(selected)
}

pub fn select_deck_output_probe_lines(
    netlist: &str,
    analysis: &str,
) -> Result<Vec<usize>, SpiceError> {
    let summary = resolve_deck_outputs(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "select_deck_output_probe_lines",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for selection in summary.selections {
        if !selection
            .analysis
            .as_deref()
            .is_none_or(|requested| deck_output_analysis_matches(requested, analysis))
        {
            continue;
        }
        for probe in selection.probes {
            let key = deck_output_probe_key(&probe);
            if seen.insert(key) {
                selected.push(selection.line_number);
            }
        }
    }
    Ok(selected)
}

pub fn select_deck_output_directives(
    netlist: &str,
    analysis: &str,
) -> Result<Vec<String>, SpiceError> {
    let summary = resolve_deck_outputs(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "select_deck_output_directives",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for selection in summary.selections {
        if !selection
            .analysis
            .as_deref()
            .is_none_or(|requested| deck_output_analysis_matches(requested, analysis))
        {
            continue;
        }
        if seen.insert(selection.directive.clone()) {
            selected.push(selection.directive);
        }
    }
    Ok(selected)
}

pub fn select_deck_output_directive_analysis_kinds(
    netlist: &str,
    analysis: &str,
) -> Result<Vec<String>, SpiceError> {
    let summary = resolve_deck_outputs(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "select_deck_output_directive_analysis_kinds",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for selection in summary.selections {
        if !selection
            .analysis
            .as_deref()
            .is_none_or(|requested| deck_output_analysis_matches(requested, analysis))
        {
            continue;
        }
        let analysis_kind = selection.analysis.unwrap_or_else(|| "global".to_string());
        if seen.insert(analysis_kind.clone()) {
            selected.push(analysis_kind);
        }
    }
    Ok(selected)
}

pub fn select_deck_output_directive_lines(
    netlist: &str,
    analysis: &str,
) -> Result<Vec<usize>, SpiceError> {
    let summary = resolve_deck_outputs(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "select_deck_output_directive_lines",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for selection in summary.selections {
        if !selection
            .analysis
            .as_deref()
            .is_none_or(|requested| deck_output_analysis_matches(requested, analysis))
        {
            continue;
        }
        if seen.insert(selection.line_number) {
            selected.push(selection.line_number);
        }
    }
    Ok(selected)
}

pub fn resolve_deck_analyses(netlist: &str) -> DeckAnalysisSummary {
    let mut state = DeckAnalysisState::default();
    let mut active_lines = Vec::new();
    let mut end_line_number = None;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if matches!(
            directive.as_deref(),
            Some(".op" | ".dc" | ".ac" | ".tran" | ".tf" | ".sens" | ".noise")
        ) {
            resolve_analysis_line(
                stripped,
                line_number,
                directive.as_deref().unwrap(),
                &mut state,
            );
            continue;
        }
        active_lines.push(stripped.to_string());
    }

    DeckAnalysisSummary {
        active_lines,
        terminated: end_line_number.is_some(),
        end_line_number,
        analyses: state.analyses,
        diagnostics: state.diagnostics,
    }
}

pub fn select_deck_analysis_plan(
    netlist: &str,
    analysis: Option<&str>,
) -> Result<DeckAnalysisPlan, SpiceError> {
    let summary = resolve_deck_analyses(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "select_deck_analysis_plan",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }

    let requested_analysis = match analysis {
        Some(value) => Some(normalize_deck_analysis_name(value).ok_or_else(|| {
            table_error(
                "select_deck_analysis_plan",
                &format!("unsupported analysis {value:?}"),
            )
        })?),
        None => None,
    };

    let mut plans = summary.analyses;
    if let Some(requested_analysis) = requested_analysis {
        plans.retain(|plan| plan.analysis == requested_analysis);
        if plans.is_empty() {
            return Err(table_error(
                "select_deck_analysis_plan",
                &format!("no .{requested_analysis} analysis card found"),
            ));
        }
        if plans.len() > 1 {
            return Err(table_error(
                "select_deck_analysis_plan",
                &format!("multiple .{requested_analysis} analysis cards found"),
            ));
        }
        return Ok(plans.remove(0));
    }

    if plans.is_empty() {
        return Ok(implicit_deck_op_analysis_plan());
    }
    if plans.len() > 1 {
        return Err(table_error(
            "select_deck_analysis_plan",
            "multiple analysis cards found; pass analysis to select one",
        ));
    }
    Ok(plans.remove(0))
}

pub fn release_readiness_gates(corpus: &[CompatibilityDeck]) -> ReleaseReadinessReport {
    let mut issues = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut analyses = Vec::new();

    if corpus.is_empty() {
        issues.push(ReleaseReadinessIssue {
            deck_id: "corpus".to_string(),
            field: "deck_count".to_string(),
            message: "compatibility corpus must contain at least one deck".to_string(),
        });
    }

    for deck in corpus {
        let deck_id = if deck.id.is_empty() {
            "<missing>".to_string()
        } else {
            deck.id.clone()
        };
        validate_compatibility_non_empty(&deck_id, "id", &deck.id, &mut issues);
        validate_compatibility_non_empty(&deck_id, "title", &deck.title, &mut issues);
        validate_compatibility_non_empty(&deck_id, "netlist", &deck.netlist, &mut issues);
        validate_compatibility_non_empty(
            &deck_id,
            "oracle.reference",
            &deck.oracle.reference,
            &mut issues,
        );
        validate_compatibility_non_empty(
            &deck_id,
            "oracle.version",
            &deck.oracle.version,
            &mut issues,
        );
        validate_compatibility_non_empty(
            &deck_id,
            "oracle.source",
            &deck.oracle.source,
            &mut issues,
        );
        if !seen_ids.insert(deck.id.clone()) {
            issues.push(ReleaseReadinessIssue {
                deck_id: deck_id.clone(),
                field: "id".to_string(),
                message: "deck ids must be unique".to_string(),
            });
        }
        if !matches!(deck.analysis.as_str(), "op" | "dc" | "ac" | "tran" | "tf") {
            issues.push(ReleaseReadinessIssue {
                deck_id: deck_id.clone(),
                field: "analysis".to_string(),
                message: format!("unsupported analysis {:?}", deck.analysis),
            });
        } else if !analyses.contains(&deck.analysis) {
            analyses.push(deck.analysis.clone());
        }
        if !deck.netlist.to_ascii_lowercase().contains(".end") {
            issues.push(ReleaseReadinessIssue {
                deck_id: deck_id.clone(),
                field: "netlist".to_string(),
                message: "deck must include .end".to_string(),
            });
        }
        if deck.golden_values.is_empty() {
            issues.push(ReleaseReadinessIssue {
                deck_id: deck_id.clone(),
                field: "golden_values".to_string(),
                message: "deck must include at least one golden value".to_string(),
            });
        }
        for (index, golden) in deck.golden_values.iter().enumerate() {
            let field_prefix = format!("golden_values[{index}]");
            validate_compatibility_non_empty(
                &deck_id,
                &format!("{field_prefix}.name"),
                &golden.name,
                &mut issues,
            );
            validate_compatibility_non_empty(
                &deck_id,
                &format!("{field_prefix}.unit"),
                &golden.unit,
                &mut issues,
            );
            if !golden.value.is_finite() {
                issues.push(ReleaseReadinessIssue {
                    deck_id: deck_id.clone(),
                    field: format!("{field_prefix}.value"),
                    message: "golden value must be finite".to_string(),
                });
            }
            if !golden.absolute_tolerance.is_finite()
                || !golden.relative_tolerance.is_finite()
                || golden.absolute_tolerance < 0.0
                || golden.relative_tolerance < 0.0
            {
                issues.push(ReleaseReadinessIssue {
                    deck_id: deck_id.clone(),
                    field: format!("{field_prefix}.tolerance"),
                    message: "tolerances must be finite and non-negative".to_string(),
                });
            }
            if golden.absolute_tolerance == 0.0
                && golden.relative_tolerance == 0.0
                && golden.unit != "count"
            {
                issues.push(ReleaseReadinessIssue {
                    deck_id: deck_id.clone(),
                    field: format!("{field_prefix}.tolerance"),
                    message: "non-count golden values need an absolute or relative tolerance"
                        .to_string(),
                });
            }
        }
        if deck.known_incompatibilities.is_empty() {
            issues.push(ReleaseReadinessIssue {
                deck_id,
                field: "known_incompatibilities".to_string(),
                message: "deck must document known incompatibility boundaries".to_string(),
            });
        }
    }

    for analysis in ["op", "dc", "ac", "tran"] {
        if !analyses.iter().any(|seen| seen == analysis) {
            issues.push(ReleaseReadinessIssue {
                deck_id: "corpus".to_string(),
                field: "analysis_coverage".to_string(),
                message: format!("missing required {analysis:?} compatibility deck"),
            });
        }
    }

    ReleaseReadinessReport {
        passed: issues.is_empty(),
        deck_count: corpus.len(),
        analyses,
        issues,
    }
}

pub fn format_compatibility_corpus_table(corpus: &[CompatibilityDeck]) -> String {
    let mut lines =
        vec!["id\tanalysis\toracle\tgolden_values\tknown_incompatibilities".to_string()];
    for deck in corpus {
        let golden_values = deck
            .golden_values
            .iter()
            .map(|entry| {
                format!(
                    "{}={}{}",
                    entry.name,
                    format_table_number(entry.value),
                    entry.unit
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!(
            "{}\t{}\t{}@{}\t{}\t{}",
            deck.id,
            deck.analysis,
            deck.oracle.reference,
            deck.oracle.version,
            golden_values,
            deck.known_incompatibilities.len()
        ));
    }
    lines.join("\n")
}

pub fn format_release_readiness_report(report: &ReleaseReadinessReport) -> String {
    let mut lines = vec![
        "passed\tdeck_count\tanalyses\tissue_count".to_string(),
        format!(
            "{}\t{}\t{}\t{}",
            report.passed,
            report.deck_count,
            report.analyses.join(","),
            report.issues.len()
        ),
    ];
    if !report.issues.is_empty() {
        lines.push("deck_id\tfield\tmessage".to_string());
        lines.extend(
            report
                .issues
                .iter()
                .map(|issue| format!("{}\t{}\t{}", issue.deck_id, issue.field, issue.message)),
        );
    }
    lines.join("\n")
}

fn common_known_incompatibilities() -> Vec<String> {
    vec![
        "binary rawfile output is not part of this release gate".to_string(),
        ".control blocks and vendor-specific directives are intentionally excluded".to_string(),
        "golden values cover named probes, not byte-for-byte waveform dumps".to_string(),
    ]
}

fn validate_compatibility_non_empty(
    deck_id: &str,
    field: &str,
    value: &str,
    issues: &mut Vec<ReleaseReadinessIssue>,
) {
    if !value.trim().is_empty() {
        return;
    }
    issues.push(ReleaseReadinessIssue {
        deck_id: deck_id.to_string(),
        field: field.to_string(),
        message: "field must be documented and non-empty".to_string(),
    });
}

struct DeckResolutionState {
    diagnostics: Vec<DeckResolutionDiagnostic>,
    included_paths: Vec<String>,
    library_sections: Vec<String>,
}

impl DeckResolutionState {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            included_paths: Vec::new(),
            library_sections: Vec::new(),
        }
    }
}

struct DeckParameterState {
    diagnostics: Vec<DeckParameterDiagnostic>,
    parameters: HashMap<String, DeckParameterValue>,
    functions: HashMap<String, DeckFunctionDefinition>,
    order: Vec<String>,
}

impl DeckParameterState {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            parameters: HashMap::new(),
            functions: HashMap::new(),
            order: Vec::new(),
        }
    }

    fn set_parameter(&mut self, name: &str, value: f64) {
        let key = name.to_ascii_lowercase();
        if !self.parameters.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.parameters.insert(
            key,
            DeckParameterValue {
                name: name.to_string(),
                value,
            },
        );
    }

    fn get_parameter(&self, name: &str) -> Option<&DeckParameterValue> {
        self.parameters.get(&name.to_ascii_lowercase())
    }

    fn set_function(&mut self, definition: DeckFunctionDefinition) {
        self.functions
            .insert(definition.name.to_ascii_lowercase(), definition);
    }

    fn get_function(&self, name: &str) -> Option<&DeckFunctionDefinition> {
        self.functions.get(&name.to_ascii_lowercase())
    }

    fn parameter_values(&self) -> Vec<DeckParameterValue> {
        self.order
            .iter()
            .filter_map(|key| self.parameters.get(key).cloned())
            .collect()
    }
}

struct DeckInitialConditionState {
    diagnostics: Vec<DeckInitialConditionDiagnostic>,
    initial_conditions: Vec<DeckNodeCondition>,
    nodesets: Vec<DeckNodeCondition>,
}

impl DeckInitialConditionState {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            initial_conditions: Vec::new(),
            nodesets: Vec::new(),
        }
    }
}

struct DeckFunctionState {
    diagnostics: Vec<DeckFunctionDiagnostic>,
    functions: Vec<DeckFunctionDefinition>,
}

impl DeckFunctionState {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            functions: Vec::new(),
        }
    }
}

struct DeckMeasurementState {
    diagnostics: Vec<DeckMeasurementDiagnostic>,
    measurements: Vec<DeckMeasurementCard>,
}

impl DeckMeasurementState {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            measurements: Vec::new(),
        }
    }
}

struct DeckFourierState {
    diagnostics: Vec<DeckFourierDiagnostic>,
    fourier: Vec<DeckFourierCard>,
}

impl DeckFourierState {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            fourier: Vec::new(),
        }
    }
}

struct DeckOutputState {
    diagnostics: Vec<DeckOutputDiagnostic>,
    selections: Vec<DeckOutputSelection>,
}

#[derive(Default)]
struct DeckAnalysisState {
    diagnostics: Vec<DeckAnalysisDiagnostic>,
    analyses: Vec<DeckAnalysisPlan>,
}

impl DeckOutputState {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            selections: Vec::new(),
        }
    }
}

fn resolve_deck_lines(
    netlist: &str,
    source: &str,
    sources: &HashMap<String, String>,
    state: &mut DeckResolutionState,
    stack: &[String],
) -> (Vec<String>, bool, Option<usize>) {
    let mut active_lines = Vec::new();
    let mut end_line_number = None;
    let mut in_control_block = false;

    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if in_control_block {
            if directive.as_deref() == Some(".endc") {
                in_control_block = false;
                continue;
            }
            if let Some(control_line) = control_block_command_as_deck_line(stripped) {
                active_lines.push(control_line);
                continue;
            }
            if is_noop_control_block_command(stripped) {
                continue;
            }
            if is_script_control_block_command(stripped) {
                state.diagnostics.push(DeckResolutionDiagnostic {
                    code: "SPICE_DECK_CONTROL_SCRIPT_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    source: source.to_string(),
                    line_number,
                    message: control_block_script_policy_message(stripped),
                    severity: "error".to_string(),
                    target: None,
                });
                continue;
            }
            if is_workdir_control_block_command(stripped) {
                state.diagnostics.push(DeckResolutionDiagnostic {
                    code: "SPICE_DECK_CONTROL_WORKDIR_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    source: source.to_string(),
                    line_number,
                    message: control_block_workdir_policy_message(stripped),
                    severity: "error".to_string(),
                    target: None,
                });
                continue;
            }
            if is_control_flow_control_block_command(stripped) {
                state.diagnostics.push(DeckResolutionDiagnostic {
                    code: "SPICE_DECK_CONTROL_FLOW_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    source: source.to_string(),
                    line_number,
                    message: control_block_flow_policy_message(stripped),
                    severity: "error".to_string(),
                    target: None,
                });
                continue;
            }
            if is_variable_control_block_command(stripped) {
                state.diagnostics.push(DeckResolutionDiagnostic {
                    code: "SPICE_DECK_CONTROL_VARIABLE_COMMAND".to_string(),
                    directive: ".control".to_string(),
                    source: source.to_string(),
                    line_number,
                    message: control_block_variable_policy_message(stripped),
                    severity: "error".to_string(),
                    target: None,
                });
                continue;
            }
            state.diagnostics.push(DeckResolutionDiagnostic {
                code: "SPICE_DECK_CONTROL_COMMAND".to_string(),
                directive: ".control".to_string(),
                source: source.to_string(),
                line_number,
                message: format!(
                    "{stripped:?} inside .control is not executed by the deck source resolver yet"
                ),
                severity: "error".to_string(),
                target: None,
            });
            continue;
        }
        if directive.as_deref() == Some(".end") {
            end_line_number = Some(line_number);
            break;
        }
        if directive.as_deref() == Some(".include") {
            active_lines.extend(resolve_include_directive(
                stripped,
                source,
                line_number,
                sources,
                state,
                stack,
            ));
            continue;
        }
        if directive.as_deref() == Some(".lib") {
            active_lines.extend(resolve_library_directive(
                stripped,
                source,
                line_number,
                sources,
                state,
                stack,
            ));
            continue;
        }
        if directive.as_deref() == Some(".control") {
            state.diagnostics.push(DeckResolutionDiagnostic {
                code: "SPICE_DECK_UNSUPPORTED_DIRECTIVE".to_string(),
                directive: ".control".to_string(),
                source: source.to_string(),
                line_number,
                message: ".control is not supported by the deck source resolver yet".to_string(),
                severity: "error".to_string(),
                target: None,
            });
            in_control_block = true;
            continue;
        }
        active_lines.push(stripped.to_string());
    }

    (active_lines, end_line_number.is_some(), end_line_number)
}

fn resolve_include_directive(
    line: &str,
    source: &str,
    line_number: usize,
    sources: &HashMap<String, String>,
    state: &mut DeckResolutionState,
    stack: &[String],
) -> Vec<String> {
    let tokens = directive_tokens(line);
    let target = tokens.get(1).map(|token| unquote_token(token));
    let Some(target) = target.filter(|target| !target.is_empty()) else {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_INCLUDE_ARGUMENT",
            ".include",
            source,
            line_number,
            ".include requires a source path",
            None,
        );
        return Vec::new();
    };
    if stack.contains(&target) {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_INCLUDE_CYCLE",
            ".include",
            source,
            line_number,
            &format!(".include cycle detected for {target}"),
            Some(target),
        );
        return Vec::new();
    }
    let Some(content) = sources.get(&target) else {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_INCLUDE_NOT_FOUND",
            ".include",
            source,
            line_number,
            &format!(".include source {target:?} was not provided"),
            Some(target),
        );
        return Vec::new();
    };

    state.included_paths.push(target.clone());
    let mut next_stack = stack.to_vec();
    next_stack.push(target.clone());
    let (resolved, _, _) = resolve_deck_lines(content, &target, sources, state, &next_stack);
    resolved
}

fn resolve_library_directive(
    line: &str,
    source: &str,
    line_number: usize,
    sources: &HashMap<String, String>,
    state: &mut DeckResolutionState,
    stack: &[String],
) -> Vec<String> {
    let tokens = directive_tokens(line);
    let path = tokens.get(1).map(|token| unquote_token(token));
    let section = tokens.get(2).map(|token| unquote_token(token));
    let (Some(path), Some(section)) = (path, section) else {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_LIB_ARGUMENT",
            ".lib",
            source,
            line_number,
            ".lib requires a source path and section name",
            None,
        );
        return Vec::new();
    };
    if path.is_empty() || section.is_empty() {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_LIB_ARGUMENT",
            ".lib",
            source,
            line_number,
            ".lib requires a source path and section name",
            Some(path),
        );
        return Vec::new();
    }

    let target = format!("{path}:{section}");
    let Some(content) = sources.get(&path) else {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_LIB_NOT_FOUND",
            ".lib",
            source,
            line_number,
            &format!(".lib source {path:?} was not provided"),
            Some(target),
        );
        return Vec::new();
    };
    if stack.contains(&target) {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_LIB_CYCLE",
            ".lib",
            source,
            line_number,
            &format!(".lib cycle detected for {target}"),
            Some(target),
        );
        return Vec::new();
    }

    let Some(section_lines) =
        extract_library_section(content, &path, &section, source, line_number, state)
    else {
        return Vec::new();
    };
    state.library_sections.push(target.clone());
    let mut next_stack = stack.to_vec();
    next_stack.push(target.clone());
    let (resolved, _, _) = resolve_deck_lines(
        &section_lines.join("\n"),
        &target,
        sources,
        state,
        &next_stack,
    );
    resolved
}

fn extract_library_section(
    content: &str,
    path: &str,
    section: &str,
    call_source: &str,
    call_line_number: usize,
    state: &mut DeckResolutionState,
) -> Option<Vec<String>> {
    let mut in_section = false;
    let mut section_start_line = None;
    let mut section_lines = Vec::new();
    let wanted = section.to_ascii_lowercase();
    let target = format!("{path}:{section}");

    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            if in_section {
                section_lines.push(raw_line.to_string());
            }
            continue;
        }
        let directive = deck_directive(stripped);
        let tokens = directive_tokens(stripped);
        if !in_section {
            if directive.as_deref() == Some(".lib")
                && tokens
                    .get(1)
                    .map(|token| unquote_token(token).to_ascii_lowercase() == wanted)
                    .unwrap_or(false)
            {
                in_section = true;
                section_start_line = Some(line_number);
            }
            continue;
        }
        if matches!(directive.as_deref(), Some(".endl" | ".endlib")) {
            return Some(section_lines);
        }
        section_lines.push(raw_line.to_string());
    }

    if !in_section {
        add_resolution_diagnostic(
            state,
            "SPICE_DECK_LIB_SECTION_NOT_FOUND",
            ".lib",
            call_source,
            call_line_number,
            &format!(".lib section {section:?} was not found in {path:?}"),
            Some(target),
        );
        return None;
    }

    add_resolution_diagnostic(
        state,
        "SPICE_DECK_LIB_SECTION_UNTERMINATED",
        ".lib",
        path,
        section_start_line.unwrap_or(1),
        &format!(".lib section {section:?} in {path:?} is missing .endl"),
        Some(target),
    );
    None
}

fn add_resolution_diagnostic(
    state: &mut DeckResolutionState,
    code: &str,
    directive: &str,
    source: &str,
    line_number: usize,
    message: &str,
    target: Option<String>,
) {
    state.diagnostics.push(DeckResolutionDiagnostic {
        code: code.to_string(),
        directive: directive.to_string(),
        source: source.to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        target,
    });
}

fn resolve_node_condition_line(
    line: &str,
    line_number: usize,
    directive: &str,
    state: &mut DeckInitialConditionState,
) {
    let tokens = directive_tokens(line);
    if tokens.len() == 1 {
        add_initial_condition_diagnostic(
            state,
            "SPICE_DECK_CONDITION_ARGUMENT",
            directive,
            line_number,
            &format!("{directive} requires at least one V(node)=value assignment"),
            None,
        );
        return;
    }

    let empty_parameter_state = DeckParameterState::new();
    for token in tokens.iter().skip(1) {
        let Some((target, expression)) = token.split_once('=') else {
            add_initial_condition_diagnostic(
                state,
                "SPICE_DECK_CONDITION_ARGUMENT",
                directive,
                line_number,
                &format!("{directive} assignment {token:?} must use V(node)=value syntax"),
                Some((*token).to_string()),
            );
            continue;
        };
        let target = target.trim();
        let Some(node) = parse_node_condition_target(target) else {
            add_initial_condition_diagnostic(
                state,
                "SPICE_DECK_CONDITION_TARGET",
                directive,
                line_number,
                &format!("{directive} target {target:?} must use V(node) syntax"),
                Some((*token).to_string()),
            );
            continue;
        };
        let expression = strip_expression_delimiters(expression.trim());
        match evaluate_parameter_expression(&expression, &empty_parameter_state) {
            Ok(value) => {
                let condition = DeckNodeCondition {
                    directive: directive.to_string(),
                    node,
                    value,
                    line_number,
                };
                if directive == ".ic" {
                    state.initial_conditions.push(condition);
                } else {
                    state.nodesets.push(condition);
                }
            }
            Err(message) => add_initial_condition_diagnostic(
                state,
                "SPICE_DECK_CONDITION_EXPRESSION",
                directive,
                line_number,
                &message,
                Some((*token).to_string()),
            ),
        }
    }
}

fn resolve_function_line(line: &str, line_number: usize, state: &mut DeckFunctionState) {
    let Some((_, rest)) = line.split_once(char::is_whitespace) else {
        add_function_diagnostic(
            state,
            "SPICE_DECK_FUNC_ARGUMENT",
            line_number,
            ".func requires a name(args) expression definition",
            None,
            None,
        );
        return;
    };
    let rest = rest.trim();
    if rest.is_empty() {
        add_function_diagnostic(
            state,
            "SPICE_DECK_FUNC_ARGUMENT",
            line_number,
            ".func requires a name(args) expression definition",
            None,
            None,
        );
        return;
    }

    let Some((name, arguments, expression)) = parse_function_signature(rest) else {
        add_function_diagnostic(
            state,
            "SPICE_DECK_FUNC_SIGNATURE",
            line_number,
            ".func definition must use name(args) expression syntax",
            None,
            None,
        );
        return;
    };
    if !is_parameter_name(&name) {
        add_function_diagnostic(
            state,
            "SPICE_DECK_FUNC_SIGNATURE",
            line_number,
            &format!(".func name {name:?} is not a valid identifier"),
            Some(name),
            None,
        );
        return;
    }
    if let Some(invalid_argument) = arguments
        .iter()
        .find(|argument| !is_parameter_name(argument))
    {
        add_function_diagnostic(
            state,
            "SPICE_DECK_FUNC_ARGUMENT",
            line_number,
            &format!(".func argument {invalid_argument:?} is not a valid identifier"),
            Some(name),
            None,
        );
        return;
    }
    let mut seen = HashSet::new();
    if arguments
        .iter()
        .any(|argument| !seen.insert(argument.to_ascii_lowercase()))
    {
        add_function_diagnostic(
            state,
            "SPICE_DECK_FUNC_ARGUMENT",
            line_number,
            &format!(".func {name:?} has duplicate argument names"),
            Some(name),
            None,
        );
        return;
    }
    let expression = strip_expression_delimiters(expression.trim());
    if expression.is_empty() {
        add_function_diagnostic(
            state,
            "SPICE_DECK_FUNC_EXPRESSION",
            line_number,
            &format!(".func {name:?} requires a non-empty expression"),
            Some(name),
            None,
        );
        return;
    }
    state.functions.push(DeckFunctionDefinition {
        name,
        arguments,
        expression,
        line_number,
    });
}

fn resolve_measurement_line(
    line: &str,
    line_number: usize,
    directive: &str,
    state: &mut DeckMeasurementState,
) {
    let tokens = directive_tokens(line);
    if tokens.len() < 5 {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            &format!("{directive} requires analysis, name, mode, and probe tokens"),
            None,
        );
        return;
    }

    let analysis = tokens[1].trim().to_ascii_lowercase();
    if analysis != "tran" && analysis != "transient" && analysis != "dc" && analysis != "ac" {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ANALYSIS",
            directive,
            line_number,
            &format!(
                "only transient, dc, and ac .measure cards are supported, got {:?}",
                tokens[1]
            ),
            Some(tokens[1].to_string()),
        );
        return;
    }

    let name = tokens[2].trim();
    if !is_parameter_name(name) {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_NAME",
            directive,
            line_number,
            &format!("measurement name {name:?} is not a valid identifier"),
            Some(name.to_string()),
        );
        return;
    }

    if tokens[3].trim().eq_ignore_ascii_case("trig") {
        resolve_measurement_delay_line(
            tokens.as_slice(),
            line_number,
            directive,
            state,
            &analysis,
            name,
        );
        return;
    }

    let Some(mode) = normalize_measurement_mode_token(tokens[3]) else {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_MODE",
            directive,
            line_number,
            &format!("unsupported measurement mode {:?}", tokens[3]),
            Some(tokens[3].to_string()),
        );
        return;
    };

    let empty_parameter_state = DeckParameterState::new();
    let mut target_value = None;
    let probe = if mode == "when" {
        let Some((probe_token, target_expression)) = tokens[4].split_once('=') else {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                "WHEN measurements require probe=target syntax",
                Some(tokens[4].to_string()),
            );
            return;
        };
        match evaluate_parameter_expression(
            &strip_expression_delimiters(target_expression.trim()),
            &empty_parameter_state,
        ) {
            Ok(value) => target_value = Some(value),
            Err(message) => {
                add_measurement_diagnostic(
                    state,
                    "SPICE_DECK_MEASURE_EXPRESSION",
                    directive,
                    line_number,
                    &message,
                    Some(tokens[4].to_string()),
                );
                return;
            }
        }
        unquote_token(probe_token.trim())
    } else {
        unquote_token(tokens[4].trim())
    };
    if probe.is_empty() {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_PROBE",
            directive,
            line_number,
            "measurement probe must not be empty",
            Some(tokens[4].to_string()),
        );
        return;
    }

    let mut from_value = None;
    let mut to_value = None;
    let mut at_value = None;
    let mut crossing_kind = None;
    let mut crossing_count = None;
    let mut seen_window_tokens = Vec::new();
    let diagnostic_count = state.diagnostics.len();
    for token in tokens.iter().skip(5) {
        let Some((key, expression)) = token.split_once('=') else {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                &format!("measurement option {token:?} must use name=value syntax"),
                Some((*token).to_string()),
            );
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if key != "from"
            && key != "to"
            && key != "at"
            && key != "rise"
            && key != "fall"
            && key != "cross"
        {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                &format!("unsupported measurement option {key:?}"),
                Some((*token).to_string()),
            );
            continue;
        }
        if seen_window_tokens.iter().any(|seen| seen == &key) {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                &format!("duplicate measurement option {key:?}"),
                Some((*token).to_string()),
            );
            continue;
        }
        seen_window_tokens.push(key.clone());
        match evaluate_parameter_expression(
            &strip_expression_delimiters(expression.trim()),
            &empty_parameter_state,
        ) {
            Ok(value) if key == "rise" || key == "fall" || key == "cross" => {
                if mode != "when" {
                    add_measurement_diagnostic(
                        state,
                        "SPICE_DECK_MEASURE_ARGUMENT",
                        directive,
                        line_number,
                        "RISE, FALL, and CROSS options are only supported with WHEN mode",
                        Some((*token).to_string()),
                    );
                    continue;
                }
                if crossing_kind.is_some() {
                    add_measurement_diagnostic(
                        state,
                        "SPICE_DECK_MEASURE_ARGUMENT",
                        directive,
                        line_number,
                        "only one of RISE, FALL, or CROSS may be specified",
                        Some((*token).to_string()),
                    );
                    continue;
                }
                if !value.is_finite()
                    || value < 1.0
                    || value.fract() != 0.0
                    || value > usize::MAX as f64
                {
                    add_measurement_diagnostic(
                        state,
                        "SPICE_DECK_MEASURE_ARGUMENT",
                        directive,
                        line_number,
                        "RISE, FALL, and CROSS counts must be positive integers",
                        Some((*token).to_string()),
                    );
                    continue;
                }
                crossing_kind = Some(key);
                crossing_count = Some(value as usize);
            }
            Ok(value) if key == "from" => from_value = Some(value),
            Ok(value) if key == "to" => to_value = Some(value),
            Ok(value) => at_value = Some(value),
            Err(message) => add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_EXPRESSION",
                directive,
                line_number,
                &message,
                Some((*token).to_string()),
            ),
        }
    }

    if mode == "find" && at_value.is_none() {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            "FIND measurements require an AT value",
            None,
        );
    }
    if mode == "when" && target_value.is_none() {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            "WHEN measurements require a target value",
            None,
        );
    }
    if mode != "find" && at_value.is_some() {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            "measurement AT value is only supported with FIND mode",
            None,
        );
    }
    if at_value.is_some() && (from_value.is_some() || to_value.is_some()) {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            "measurement AT value cannot be combined with FROM or TO",
            None,
        );
    }

    if let (Some(from), Some(to)) = (from_value, to_value) {
        if from > to {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_WINDOW",
                directive,
                line_number,
                "measurement FROM value must be <= TO value",
                None,
            );
        }
    }

    if state.diagnostics.len() != diagnostic_count {
        return;
    }

    state.measurements.push(DeckMeasurementCard {
        directive: directive.to_string(),
        analysis,
        name: name.to_string(),
        mode: mode.to_string(),
        probe,
        line_number,
        from_value,
        to_value,
        at_value,
        target_value,
        crossing_kind,
        crossing_count,
        trigger_probe: None,
        trigger_value: None,
        trigger_crossing_kind: None,
        trigger_crossing_count: None,
    });
}

fn resolve_fourier_line(line: &str, line_number: usize, state: &mut DeckFourierState) {
    let tokens = directive_tokens(line);
    if tokens.len() < 3 {
        add_fourier_diagnostic(
            state,
            "SPICE_DECK_FOURIER_ARGUMENT",
            line_number,
            ".four requires a fundamental frequency and at least one probe",
            None,
        );
        return;
    }

    let empty_parameter_state = DeckParameterState::new();
    let frequency = match evaluate_parameter_expression(
        &strip_expression_delimiters(tokens[1].trim()),
        &empty_parameter_state,
    ) {
        Ok(value) if value.is_finite() && value > 0.0 => value,
        Ok(_) => {
            add_fourier_diagnostic(
                state,
                "SPICE_DECK_FOURIER_FREQUENCY",
                line_number,
                ".four fundamental frequency must be finite and positive",
                Some(tokens[1].to_string()),
            );
            return;
        }
        Err(message) => {
            add_fourier_diagnostic(
                state,
                "SPICE_DECK_FOURIER_EXPRESSION",
                line_number,
                &message,
                Some(tokens[1].to_string()),
            );
            return;
        }
    };

    let mut probes = Vec::new();
    let mut harmonics = None;
    let mut from_value = None;
    let mut seen_options = Vec::new();
    let diagnostic_count = state.diagnostics.len();
    for token in tokens.iter().skip(2) {
        if let Some((key, expression)) = token.split_once('=') {
            let key = key.trim().to_ascii_lowercase();
            if key != "harmonics" && key != "from" {
                add_fourier_diagnostic(
                    state,
                    "SPICE_DECK_FOURIER_ARGUMENT",
                    line_number,
                    &format!("unsupported .four option {key:?}"),
                    Some((*token).to_string()),
                );
                continue;
            }
            if seen_options.iter().any(|seen| seen == &key) {
                add_fourier_diagnostic(
                    state,
                    "SPICE_DECK_FOURIER_ARGUMENT",
                    line_number,
                    &format!("duplicate .four option {key:?}"),
                    Some((*token).to_string()),
                );
                continue;
            }
            seen_options.push(key.clone());
            match evaluate_parameter_expression(
                &strip_expression_delimiters(expression.trim()),
                &empty_parameter_state,
            ) {
                Ok(value) if key == "harmonics" => {
                    if !value.is_finite()
                        || value < 1.0
                        || value.fract() != 0.0
                        || value > usize::MAX as f64
                    {
                        add_fourier_diagnostic(
                            state,
                            "SPICE_DECK_FOURIER_ARGUMENT",
                            line_number,
                            ".four HARMONICS value must be a positive integer",
                            Some((*token).to_string()),
                        );
                        continue;
                    }
                    harmonics = Some(value as usize);
                }
                Ok(value) => from_value = Some(value),
                Err(message) => add_fourier_diagnostic(
                    state,
                    "SPICE_DECK_FOURIER_EXPRESSION",
                    line_number,
                    &message,
                    Some((*token).to_string()),
                ),
            }
            continue;
        }
        let probe = unquote_token(token.trim());
        if probe.is_empty() {
            add_fourier_diagnostic(
                state,
                "SPICE_DECK_FOURIER_PROBE",
                line_number,
                ".four probe must not be empty",
                Some((*token).to_string()),
            );
            continue;
        }
        probes.push(probe);
    }

    if probes.is_empty() && state.diagnostics.len() == diagnostic_count {
        add_fourier_diagnostic(
            state,
            "SPICE_DECK_FOURIER_PROBE",
            line_number,
            ".four requires at least one probe",
            None,
        );
    }
    if let Some(value) = from_value {
        if !value.is_finite() {
            add_fourier_diagnostic(
                state,
                "SPICE_DECK_FOURIER_WINDOW",
                line_number,
                ".four FROM value must be finite",
                None,
            );
        }
    }

    if state.diagnostics.len() != diagnostic_count {
        return;
    }

    state.fourier.push(DeckFourierCard {
        directive: ".four".to_string(),
        fundamental_frequency_hz: frequency,
        probes,
        line_number,
        harmonics,
        from_value,
    });
}

fn resolve_analysis_line(
    line: &str,
    line_number: usize,
    directive: &str,
    state: &mut DeckAnalysisState,
) {
    let tokens = directive_tokens(line);
    match directive {
        ".op" => resolve_op_analysis(&tokens, line_number, state),
        ".dc" => resolve_dc_analysis(&tokens, line_number, state),
        ".ac" => resolve_ac_analysis(&tokens, line_number, state),
        ".tran" => resolve_tran_analysis(&tokens, line_number, state),
        ".tf" => resolve_tf_analysis(&tokens, line_number, state),
        ".sens" => resolve_sens_analysis(&tokens, line_number, state),
        ".noise" => resolve_noise_analysis(&tokens, line_number, state),
        _ => {}
    }
}

fn resolve_op_analysis(tokens: &[&str], line_number: usize, state: &mut DeckAnalysisState) {
    if tokens.len() != 1 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".op",
            line_number,
            ".op does not accept analysis arguments",
            Some(tokens[1].to_string()),
        );
        return;
    }
    state.analyses.push(DeckAnalysisPlan {
        directive: ".op".to_string(),
        analysis: "op".to_string(),
        line_number,
        source_name: None,
        output_node: None,
        start_value: None,
        stop_value: None,
        step_value: None,
        sweep_kind: None,
        point_count: None,
        start_frequency_hz: None,
        stop_frequency_hz: None,
        step_time: None,
        stop_time: None,
        start_time: None,
        max_step: None,
        use_initial_conditions: false,
    });
}

fn resolve_dc_analysis(tokens: &[&str], line_number: usize, state: &mut DeckAnalysisState) {
    if tokens.len() != 5 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".dc",
            line_number,
            ".dc requires source, start, stop, and step tokens",
            None,
        );
        return;
    }
    let source_name = unquote_token(tokens[1]).trim().to_string();
    if source_name.is_empty() {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".dc",
            line_number,
            ".dc source name must not be empty",
            Some(tokens[1].to_string()),
        );
        return;
    }
    let start_value = parse_deck_analysis_value(tokens[2], ".dc", line_number, state);
    let stop_value = parse_deck_analysis_value(tokens[3], ".dc", line_number, state);
    let step_value = parse_deck_analysis_value(tokens[4], ".dc", line_number, state);
    let (Some(start_value), Some(stop_value), Some(step_value)) =
        (start_value, stop_value, step_value)
    else {
        return;
    };
    if step_value == 0.0 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_SWEEP",
            ".dc",
            line_number,
            ".dc step value must be non-zero",
            Some(tokens[4].to_string()),
        );
        return;
    }
    if (start_value < stop_value && step_value < 0.0)
        || (start_value > stop_value && step_value > 0.0)
    {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_SWEEP",
            ".dc",
            line_number,
            ".dc step direction must move from start toward stop",
            Some(tokens[4].to_string()),
        );
        return;
    }
    state.analyses.push(DeckAnalysisPlan {
        directive: ".dc".to_string(),
        analysis: "dc".to_string(),
        line_number,
        source_name: Some(source_name),
        output_node: None,
        start_value: Some(start_value),
        stop_value: Some(stop_value),
        step_value: Some(step_value),
        sweep_kind: None,
        point_count: None,
        start_frequency_hz: None,
        stop_frequency_hz: None,
        step_time: None,
        stop_time: None,
        start_time: None,
        max_step: None,
        use_initial_conditions: false,
    });
}

fn resolve_ac_analysis(tokens: &[&str], line_number: usize, state: &mut DeckAnalysisState) {
    if tokens.len() != 5 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".ac",
            line_number,
            ".ac requires sweep kind, point count, start frequency, and stop frequency",
            None,
        );
        return;
    }
    let Some(sweep_kind) = normalize_ac_sweep_kind(tokens[1]) else {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_MODE",
            ".ac",
            line_number,
            &format!(
                ".ac sweep kind must be LIN, DEC, or OCT, got {:?}",
                tokens[1]
            ),
            Some(tokens[1].to_string()),
        );
        return;
    };
    let point_count = parse_deck_analysis_integer(tokens[2], ".ac", line_number, state);
    let start_frequency_hz = parse_deck_analysis_value(tokens[3], ".ac", line_number, state);
    let stop_frequency_hz = parse_deck_analysis_value(tokens[4], ".ac", line_number, state);
    let (Some(point_count), Some(start_frequency_hz), Some(stop_frequency_hz)) =
        (point_count, start_frequency_hz, stop_frequency_hz)
    else {
        return;
    };
    if point_count < 1 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_SWEEP",
            ".ac",
            line_number,
            ".ac point count must be a positive integer",
            Some(tokens[2].to_string()),
        );
        return;
    }
    if start_frequency_hz <= 0.0
        || stop_frequency_hz <= 0.0
        || stop_frequency_hz < start_frequency_hz
    {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_SWEEP",
            ".ac",
            line_number,
            ".ac frequencies must be positive and stop must be >= start",
            None,
        );
        return;
    }
    state.analyses.push(DeckAnalysisPlan {
        directive: ".ac".to_string(),
        analysis: "ac".to_string(),
        line_number,
        source_name: None,
        output_node: None,
        start_value: None,
        stop_value: None,
        step_value: None,
        sweep_kind: Some(sweep_kind.to_string()),
        point_count: Some(point_count),
        start_frequency_hz: Some(start_frequency_hz),
        stop_frequency_hz: Some(stop_frequency_hz),
        step_time: None,
        stop_time: None,
        start_time: None,
        max_step: None,
        use_initial_conditions: false,
    });
}

fn resolve_tran_analysis(tokens: &[&str], line_number: usize, state: &mut DeckAnalysisState) {
    if tokens.len() < 3 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".tran",
            line_number,
            ".tran requires step time and stop time",
            None,
        );
        return;
    }
    let mut use_initial_conditions = false;
    let mut numeric_tokens = Vec::new();
    for token in tokens.iter().skip(3) {
        if token.trim().eq_ignore_ascii_case("uic") {
            use_initial_conditions = true;
            continue;
        }
        numeric_tokens.push(*token);
    }
    if numeric_tokens.len() > 2 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".tran",
            line_number,
            ".tran supports optional start time, max step, and UIC only",
            Some(numeric_tokens[2].to_string()),
        );
        return;
    }
    let step_time = parse_deck_analysis_value(tokens[1], ".tran", line_number, state);
    let stop_time = parse_deck_analysis_value(tokens[2], ".tran", line_number, state);
    let start_time = numeric_tokens
        .first()
        .and_then(|token| parse_deck_analysis_value(token, ".tran", line_number, state));
    let max_step = numeric_tokens
        .get(1)
        .and_then(|token| parse_deck_analysis_value(token, ".tran", line_number, state));
    let (Some(step_time), Some(stop_time)) = (step_time, stop_time) else {
        return;
    };
    if (!numeric_tokens.is_empty() && start_time.is_none())
        || (numeric_tokens.len() >= 2 && max_step.is_none())
    {
        return;
    }
    if step_time <= 0.0 || stop_time <= 0.0 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_INTERVAL",
            ".tran",
            line_number,
            ".tran step time and stop time must be positive",
            None,
        );
        return;
    }
    if let Some(start_time) = start_time {
        if start_time < 0.0 || start_time > stop_time {
            add_analysis_diagnostic(
                state,
                "SPICE_DECK_ANALYSIS_INTERVAL",
                ".tran",
                line_number,
                ".tran start time must be non-negative and <= stop time",
                None,
            );
            return;
        }
    }
    if max_step.is_some_and(|value| value <= 0.0) {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_INTERVAL",
            ".tran",
            line_number,
            ".tran max step must be positive",
            None,
        );
        return;
    }
    state.analyses.push(DeckAnalysisPlan {
        directive: ".tran".to_string(),
        analysis: "tran".to_string(),
        line_number,
        source_name: None,
        output_node: None,
        start_value: None,
        stop_value: None,
        step_value: None,
        sweep_kind: None,
        point_count: None,
        start_frequency_hz: None,
        stop_frequency_hz: None,
        step_time: Some(step_time),
        stop_time: Some(stop_time),
        start_time,
        max_step,
        use_initial_conditions,
    });
}

fn resolve_tf_analysis(tokens: &[&str], line_number: usize, state: &mut DeckAnalysisState) {
    if tokens.len() != 3 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".tf",
            line_number,
            ".tf requires output voltage probe and input source tokens",
            None,
        );
        return;
    }
    let output_probe = normalize_deck_output_probe(&unquote_token(tokens[1]));
    let Some(output_probe) = output_probe.filter(|probe| probe.starts_with("V(")) else {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".tf",
            line_number,
            &format!(
                ".tf output must be a voltage probe V(node), got {:?}",
                tokens[1]
            ),
            Some(tokens[1].to_string()),
        );
        return;
    };
    let input_source = unquote_token(tokens[2]).trim().to_string();
    if input_source.is_empty() {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".tf",
            line_number,
            ".tf input source name must not be empty",
            Some(tokens[2].to_string()),
        );
        return;
    }
    state.analyses.push(DeckAnalysisPlan {
        directive: ".tf".to_string(),
        analysis: "tf".to_string(),
        line_number,
        source_name: Some(input_source),
        output_node: Some(output_probe[2..output_probe.len() - 1].to_string()),
        start_value: None,
        stop_value: None,
        step_value: None,
        sweep_kind: None,
        point_count: None,
        start_frequency_hz: None,
        stop_frequency_hz: None,
        step_time: None,
        stop_time: None,
        start_time: None,
        max_step: None,
        use_initial_conditions: false,
    });
}

fn resolve_sens_analysis(tokens: &[&str], line_number: usize, state: &mut DeckAnalysisState) {
    if tokens.len() != 2 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".sens",
            line_number,
            ".sens requires one output voltage probe token",
            None,
        );
        return;
    }
    let output_probe = normalize_deck_output_probe(&unquote_token(tokens[1]));
    let Some(output_probe) = output_probe.filter(|probe| probe.starts_with("V(")) else {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".sens",
            line_number,
            &format!(
                ".sens output must be a voltage probe V(node), got {:?}",
                tokens[1]
            ),
            Some(tokens[1].to_string()),
        );
        return;
    };
    state.analyses.push(DeckAnalysisPlan {
        directive: ".sens".to_string(),
        analysis: "sens".to_string(),
        line_number,
        source_name: None,
        output_node: Some(output_probe[2..output_probe.len() - 1].to_string()),
        start_value: None,
        stop_value: None,
        step_value: None,
        sweep_kind: None,
        point_count: None,
        start_frequency_hz: None,
        stop_frequency_hz: None,
        step_time: None,
        stop_time: None,
        start_time: None,
        max_step: None,
        use_initial_conditions: false,
    });
}

fn resolve_noise_analysis(tokens: &[&str], line_number: usize, state: &mut DeckAnalysisState) {
    if !matches!(tokens.len(), 3 | 7) {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".noise",
            line_number,
            ".noise requires output voltage probe, input source, and optional sweep kind, point count, start frequency, and stop frequency tokens",
            None,
        );
        return;
    }
    let output_probe = normalize_deck_output_probe(&unquote_token(tokens[1]));
    let Some(output_probe) = output_probe.filter(|probe| probe.starts_with("V(")) else {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".noise",
            line_number,
            &format!(
                ".noise output must be a voltage probe V(node), got {:?}",
                tokens[1]
            ),
            Some(tokens[1].to_string()),
        );
        return;
    };
    let input_source = unquote_token(tokens[2]).trim().to_string();
    if input_source.is_empty() {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            ".noise",
            line_number,
            ".noise input source name must not be empty",
            Some(tokens[2].to_string()),
        );
        return;
    }

    let mut sweep_kind = None;
    let mut point_count = None;
    let mut start_frequency_hz = None;
    let mut stop_frequency_hz = None;
    if tokens.len() == 7 {
        let Some(parsed_sweep_kind) = normalize_ac_sweep_kind(tokens[3]) else {
            add_analysis_diagnostic(
                state,
                "SPICE_DECK_ANALYSIS_MODE",
                ".noise",
                line_number,
                &format!(
                    ".noise sweep kind must be LIN, DEC, or OCT, got {:?}",
                    tokens[3]
                ),
                Some(tokens[3].to_string()),
            );
            return;
        };
        let parsed_point_count =
            parse_deck_analysis_integer(tokens[4], ".noise", line_number, state);
        let parsed_start_frequency =
            parse_deck_analysis_value(tokens[5], ".noise", line_number, state);
        let parsed_stop_frequency =
            parse_deck_analysis_value(tokens[6], ".noise", line_number, state);
        let (Some(parsed_point_count), Some(parsed_start_frequency), Some(parsed_stop_frequency)) = (
            parsed_point_count,
            parsed_start_frequency,
            parsed_stop_frequency,
        ) else {
            return;
        };
        if parsed_point_count < 1 {
            add_analysis_diagnostic(
                state,
                "SPICE_DECK_ANALYSIS_SWEEP",
                ".noise",
                line_number,
                ".noise point count must be a positive integer",
                Some(tokens[4].to_string()),
            );
            return;
        }
        if parsed_start_frequency <= 0.0
            || parsed_stop_frequency <= 0.0
            || parsed_stop_frequency < parsed_start_frequency
        {
            add_analysis_diagnostic(
                state,
                "SPICE_DECK_ANALYSIS_SWEEP",
                ".noise",
                line_number,
                ".noise frequencies must be positive and stop must be >= start",
                None,
            );
            return;
        }
        sweep_kind = Some(parsed_sweep_kind.to_string());
        point_count = Some(parsed_point_count);
        start_frequency_hz = Some(parsed_start_frequency);
        stop_frequency_hz = Some(parsed_stop_frequency);
    }

    state.analyses.push(DeckAnalysisPlan {
        directive: ".noise".to_string(),
        analysis: "noise".to_string(),
        line_number,
        source_name: Some(input_source),
        output_node: Some(output_probe[2..output_probe.len() - 1].to_string()),
        start_value: None,
        stop_value: None,
        step_value: None,
        sweep_kind,
        point_count,
        start_frequency_hz,
        stop_frequency_hz,
        step_time: None,
        stop_time: None,
        start_time: None,
        max_step: None,
        use_initial_conditions: false,
    });
}

struct ParsedMeasurementEdge {
    probe: String,
    value: f64,
    crossing_kind: Option<String>,
    crossing_count: Option<usize>,
}

fn resolve_measurement_delay_line(
    tokens: &[&str],
    line_number: usize,
    directive: &str,
    state: &mut DeckMeasurementState,
    analysis: &str,
    name: &str,
) {
    if analysis != "tran" && analysis != "transient" {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            "TRIG/TARG measurements are only supported for transient analysis",
            Some(tokens[3].to_string()),
        );
        return;
    }
    let Some(target_index) = tokens
        .iter()
        .enumerate()
        .skip(4)
        .find_map(|(index, token)| token.trim().eq_ignore_ascii_case("targ").then_some(index))
    else {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            "TRIG measurements require a TARG section",
            None,
        );
        return;
    };

    let empty_parameter_state = DeckParameterState::new();
    let Some(trigger) = parse_measurement_delay_edge(
        &tokens[4..target_index],
        "TRIG",
        directive,
        line_number,
        state,
        &empty_parameter_state,
    ) else {
        return;
    };
    let Some((target, from_value, to_value)) = parse_measurement_delay_target_section(
        &tokens[target_index + 1..],
        directive,
        line_number,
        state,
        &empty_parameter_state,
    ) else {
        return;
    };

    if let (Some(from), Some(to)) = (from_value, to_value) {
        if from > to {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_WINDOW",
                directive,
                line_number,
                "measurement FROM value must be <= TO value",
                None,
            );
            return;
        }
    }

    state.measurements.push(DeckMeasurementCard {
        directive: directive.to_string(),
        analysis: analysis.to_string(),
        name: name.to_string(),
        mode: "delay".to_string(),
        probe: target.probe,
        line_number,
        from_value,
        to_value,
        at_value: None,
        target_value: Some(target.value),
        crossing_kind: target.crossing_kind,
        crossing_count: target.crossing_count,
        trigger_probe: Some(trigger.probe),
        trigger_value: Some(trigger.value),
        trigger_crossing_kind: trigger.crossing_kind,
        trigger_crossing_count: trigger.crossing_count,
    });
}

fn parse_measurement_delay_target_section(
    tokens: &[&str],
    directive: &str,
    line_number: usize,
    state: &mut DeckMeasurementState,
    parameter_state: &DeckParameterState,
) -> Option<(ParsedMeasurementEdge, Option<f64>, Option<f64>)> {
    let mut edge_tokens = Vec::new();
    let mut from_value = None;
    let mut to_value = None;
    let mut seen_window_tokens = Vec::new();
    for token in tokens {
        let Some((key, expression)) = token.split_once('=') else {
            edge_tokens.push(*token);
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if key != "from" && key != "to" {
            edge_tokens.push(*token);
            continue;
        }
        if seen_window_tokens.iter().any(|seen| seen == &key) {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                &format!("duplicate measurement option {key:?}"),
                Some((*token).to_string()),
            );
            return None;
        }
        seen_window_tokens.push(key.clone());
        match evaluate_parameter_expression(
            &strip_expression_delimiters(expression.trim()),
            parameter_state,
        ) {
            Ok(value) if key == "from" => from_value = Some(value),
            Ok(value) => to_value = Some(value),
            Err(message) => {
                add_measurement_diagnostic(
                    state,
                    "SPICE_DECK_MEASURE_EXPRESSION",
                    directive,
                    line_number,
                    &message,
                    Some((*token).to_string()),
                );
                return None;
            }
        }
    }
    parse_measurement_delay_edge(
        edge_tokens.as_slice(),
        "TARG",
        directive,
        line_number,
        state,
        parameter_state,
    )
    .map(|edge| (edge, from_value, to_value))
}

fn parse_measurement_delay_edge(
    tokens: &[&str],
    section: &str,
    directive: &str,
    line_number: usize,
    state: &mut DeckMeasurementState,
    parameter_state: &DeckParameterState,
) -> Option<ParsedMeasurementEdge> {
    let Some(first) = tokens.first() else {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            &format!("{section} measurements require a probe target"),
            None,
        );
        return None;
    };
    let mut value = None;
    let probe = if let Some((probe_token, expression)) = first.split_once('=') {
        match evaluate_parameter_expression(
            &strip_expression_delimiters(expression.trim()),
            parameter_state,
        ) {
            Ok(parsed) => value = Some(parsed),
            Err(message) => {
                add_measurement_diagnostic(
                    state,
                    "SPICE_DECK_MEASURE_EXPRESSION",
                    directive,
                    line_number,
                    &message,
                    Some((*first).to_string()),
                );
                return None;
            }
        }
        unquote_token(probe_token.trim())
    } else {
        unquote_token(first.trim())
    };
    if probe.is_empty() {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_PROBE",
            directive,
            line_number,
            &format!("{section} measurement probe must not be empty"),
            Some((*first).to_string()),
        );
        return None;
    }

    let mut crossing_kind = None;
    let mut crossing_count = None;
    let mut seen_tokens = Vec::new();
    for token in tokens.iter().skip(1) {
        let Some((key, expression)) = token.split_once('=') else {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                &format!("{section} measurement option {token:?} must use name=value syntax"),
                Some((*token).to_string()),
            );
            return None;
        };
        let key = key.trim().to_ascii_lowercase();
        if key != "val" && key != "rise" && key != "fall" && key != "cross" {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                &format!("unsupported {section} measurement option {key:?}"),
                Some((*token).to_string()),
            );
            return None;
        }
        if seen_tokens.iter().any(|seen| seen == &key) {
            add_measurement_diagnostic(
                state,
                "SPICE_DECK_MEASURE_ARGUMENT",
                directive,
                line_number,
                &format!("duplicate {section} measurement option {key:?}"),
                Some((*token).to_string()),
            );
            return None;
        }
        seen_tokens.push(key.clone());
        match evaluate_parameter_expression(
            &strip_expression_delimiters(expression.trim()),
            parameter_state,
        ) {
            Ok(parsed) if key == "val" => value = Some(parsed),
            Ok(parsed) if key == "rise" || key == "fall" || key == "cross" => {
                if crossing_kind.is_some() {
                    add_measurement_diagnostic(
                        state,
                        "SPICE_DECK_MEASURE_ARGUMENT",
                        directive,
                        line_number,
                        &format!("only one {section} RISE, FALL, or CROSS option may be specified"),
                        Some((*token).to_string()),
                    );
                    return None;
                }
                if !parsed.is_finite()
                    || parsed < 1.0
                    || parsed.fract() != 0.0
                    || parsed > usize::MAX as f64
                {
                    add_measurement_diagnostic(
                        state,
                        "SPICE_DECK_MEASURE_ARGUMENT",
                        directive,
                        line_number,
                        &format!(
                            "{section} RISE, FALL, and CROSS counts must be positive integers"
                        ),
                        Some((*token).to_string()),
                    );
                    return None;
                }
                crossing_kind = Some(key);
                crossing_count = Some(parsed as usize);
            }
            Err(message) => {
                add_measurement_diagnostic(
                    state,
                    "SPICE_DECK_MEASURE_EXPRESSION",
                    directive,
                    line_number,
                    &message,
                    Some((*token).to_string()),
                );
                return None;
            }
            Ok(_) => unreachable!(),
        }
    }
    let Some(value) = value else {
        add_measurement_diagnostic(
            state,
            "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            line_number,
            &format!("{section} measurements require a VAL value or probe=value target"),
            None,
        );
        return None;
    };
    Some(ParsedMeasurementEdge {
        probe,
        value,
        crossing_kind,
        crossing_count,
    })
}

fn resolve_output_line(
    line: &str,
    line_number: usize,
    directive: &str,
    state: &mut DeckOutputState,
) {
    let tokens = directive_tokens(line);
    if tokens.len() < 2 {
        let message = if matches!(directive, ".print" | ".plot") {
            format!("{directive} requires an analysis token and at least one probe token")
        } else {
            format!("{directive} requires at least one probe token")
        };
        add_output_diagnostic(
            state,
            "SPICE_DECK_OUTPUT_ARGUMENT",
            directive,
            line_number,
            &message,
            None,
        );
        return;
    }

    let (analysis, probe_tokens) = if matches!(directive, ".print" | ".plot") {
        if tokens.len() < 3 {
            add_output_diagnostic(
                state,
                "SPICE_DECK_OUTPUT_ARGUMENT",
                directive,
                line_number,
                &format!("{directive} requires an analysis token and at least one probe token"),
                None,
            );
            return;
        }
        let Some(analysis) = normalize_deck_output_analysis(tokens[1]) else {
            add_output_diagnostic(
                state,
                "SPICE_DECK_OUTPUT_ANALYSIS",
                directive,
                line_number,
                &format!(
                    "{directive} analysis must be op, dc, ac, or tran, got {:?}",
                    tokens[1],
                ),
                Some(tokens[1].to_string()),
            );
            return;
        };
        (Some(analysis.to_string()), &tokens[2..])
    } else if directive == ".probe"
        && tokens
            .get(1)
            .and_then(|token| normalize_deck_output_analysis(token))
            .is_some()
    {
        (
            normalize_deck_output_analysis(tokens[1]).map(str::to_string),
            &tokens[2..],
        )
    } else {
        (None, &tokens[1..])
    };
    if probe_tokens.is_empty() {
        add_output_diagnostic(
            state,
            "SPICE_DECK_OUTPUT_ARGUMENT",
            directive,
            line_number,
            &format!("{directive} requires at least one probe token"),
            None,
        );
        return;
    }

    let mut probes = Vec::new();
    for token in probe_tokens {
        let token = unquote_token(token);
        match normalize_deck_output_probe(&token) {
            Some(probe) => probes.push(probe),
            None => add_output_diagnostic(
                state,
                "SPICE_DECK_OUTPUT_PROBE",
                directive,
                line_number,
                &format!("{directive} probe must be V(node) or I(source), got {token:?}"),
                Some(token),
            ),
        }
    }
    if probes.is_empty() {
        return;
    }
    state.selections.push(DeckOutputSelection {
        directive: directive.to_string(),
        analysis,
        probes,
        line_number,
    });
}

fn resolve_param_line(line: &str, line_number: usize, state: &mut DeckParameterState) {
    let tokens = directive_tokens(line);
    if tokens.len() == 1 {
        add_parameter_diagnostic(
            state,
            "SPICE_DECK_PARAM_ARGUMENT",
            ".param",
            line_number,
            ".param requires at least one name=value assignment",
            None,
            None,
        );
        return;
    }

    for token in tokens.iter().skip(1) {
        let Some((name, expression)) = token.split_once('=') else {
            add_parameter_diagnostic(
                state,
                "SPICE_DECK_PARAM_ARGUMENT",
                ".param",
                line_number,
                &format!(".param assignment {token:?} must use name=value syntax"),
                Some((*token).to_string()),
                None,
            );
            continue;
        };
        let name = name.trim();
        let expression = strip_expression_delimiters(expression.trim());
        if !is_parameter_name(name) {
            add_parameter_diagnostic(
                state,
                "SPICE_DECK_PARAM_NAME",
                ".param",
                line_number,
                &format!(".param name {name:?} is not a valid identifier"),
                Some(name.to_string()),
                Some(expression.clone()),
            );
            continue;
        }
        match evaluate_parameter_expression(&expression, state) {
            Ok(value) => state.set_parameter(name, value),
            Err(message) => add_parameter_diagnostic(
                state,
                "SPICE_DECK_PARAM_EXPRESSION",
                ".param",
                line_number,
                &message,
                Some(name.to_string()),
                Some(expression),
            ),
        }
    }
}

fn collect_parameter_functions(netlist: &str, state: &mut DeckParameterState) {
    let mut function_state = DeckFunctionState::new();
    for (index, raw_line) in netlist.lines().enumerate() {
        let line_number = index + 1;
        let stripped = raw_line.trim();
        if stripped.is_empty() || stripped.starts_with('*') || stripped.starts_with(';') {
            continue;
        }
        let directive = deck_directive(stripped);
        if directive.as_deref() == Some(".end") {
            break;
        }
        if directive.as_deref() == Some(".func") {
            resolve_function_line(stripped, line_number, &mut function_state);
        }
    }

    for definition in function_state.functions {
        state.set_function(definition);
    }
    for diagnostic in function_state.diagnostics {
        add_parameter_diagnostic(
            state,
            &diagnostic.code,
            &diagnostic.directive,
            diagnostic.line_number,
            &diagnostic.message,
            diagnostic.function_name,
            diagnostic.expression,
        );
    }
}

fn rewrite_parameter_expressions(
    line: &str,
    line_number: usize,
    state: &mut DeckParameterState,
) -> String {
    let braced = replace_delimited_parameter_expressions(line, '{', '}', line_number, state);
    replace_delimited_parameter_expressions(&braced, '\'', '\'', line_number, state)
}

fn replace_delimited_parameter_expressions(
    line: &str,
    open_token: char,
    close_token: char,
    line_number: usize,
    state: &mut DeckParameterState,
) -> String {
    let mut result = String::new();
    let mut index = 0;
    while index < line.len() {
        let rest = &line[index..];
        if !rest.starts_with(open_token) {
            let Some(ch) = rest.chars().next() else {
                break;
            };
            result.push(ch);
            index += ch.len_utf8();
            continue;
        }

        let expression_start = index + open_token.len_utf8();
        let Some(close_offset) = line[expression_start..].find(close_token) else {
            add_parameter_diagnostic(
                state,
                "SPICE_DECK_PARAM_UNTERMINATED",
                ".param",
                line_number,
                &format!(
                    "unterminated parameter expression starting at column {}",
                    index + 1
                ),
                None,
                None,
            );
            result.push_str(&line[index..]);
            break;
        };
        let close_index = expression_start + close_offset;
        let expression = line[expression_start..close_index].trim();
        match evaluate_parameter_expression(expression, state) {
            Ok(value) => result.push_str(&format_parameter_number(value)),
            Err(message) => {
                add_parameter_diagnostic(
                    state,
                    "SPICE_DECK_PARAM_UNRESOLVED",
                    ".param",
                    line_number,
                    &message,
                    None,
                    Some(expression.to_string()),
                );
                result.push_str(&line[index..close_index + close_token.len_utf8()]);
            }
        }
        index = close_index + close_token.len_utf8();
    }
    result
}

fn evaluate_parameter_expression(
    expression: &str,
    state: &DeckParameterState,
) -> Result<f64, String> {
    let value = ParameterExpressionParser::new(expression, state).parse()?;
    if !value.is_finite() {
        return Err(format!(
            "parameter expression {expression:?} did not evaluate to a finite value"
        ));
    }
    Ok(value)
}

struct ParameterExpressionParser<'a> {
    expression: &'a str,
    state: &'a DeckParameterState,
    local_values: HashMap<String, f64>,
    call_stack: Vec<String>,
    index: usize,
}

impl<'a> ParameterExpressionParser<'a> {
    fn new(expression: &'a str, state: &'a DeckParameterState) -> Self {
        Self {
            expression,
            state,
            local_values: HashMap::new(),
            call_stack: Vec::new(),
            index: 0,
        }
    }

    fn new_with_context(
        expression: &'a str,
        state: &'a DeckParameterState,
        local_values: HashMap<String, f64>,
        call_stack: Vec<String>,
    ) -> Self {
        Self {
            expression,
            state,
            local_values,
            call_stack,
            index: 0,
        }
    }

    fn parse(mut self) -> Result<f64, String> {
        if self.expression.is_empty() {
            return Err("parameter expression must not be empty".to_string());
        }
        let value = self.parse_expression()?;
        self.skip_whitespace();
        if self.index != self.expression.len() {
            let token = self.current_char().unwrap_or('\0');
            return Err(format!(
                "unexpected token {token:?} in parameter expression"
            ));
        }
        Ok(value)
    }

    fn parse_expression(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            if self.match_token("+") {
                value += self.parse_term()?;
            } else if self.match_token("-") {
                value -= self.parse_term()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_power()?;
        loop {
            self.skip_whitespace();
            if self.match_token("*") {
                value *= self.parse_power()?;
            } else if self.match_token("/") {
                let denominator = self.parse_power()?;
                if denominator == 0.0 {
                    return Err("division by zero in parameter expression".to_string());
                }
                value /= denominator;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let mut value = self.parse_unary()?;
        self.skip_whitespace();
        if self.match_token("^") {
            value = value.powf(self.parse_power()?);
        }
        Ok(value)
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        if self.match_token("+") {
            return self.parse_unary();
        }
        if self.match_token("-") {
            return Ok(-self.parse_unary()?);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        if self.match_token("(") {
            let value = self.parse_expression()?;
            self.skip_whitespace();
            if !self.match_token(")") {
                return Err("missing ')' in parameter expression".to_string());
            }
            return Ok(value);
        }
        let Some(ch) = self.current_char() else {
            return Err("unexpected end of parameter expression".to_string());
        };
        if ch.is_ascii_digit() || ch == '.' {
            return self.parse_number();
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            return self.parse_identifier();
        }
        Err(format!("unexpected token {ch:?} in parameter expression"))
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.index;
        let mut saw_digit = false;
        while self.current_char().is_some_and(|ch| ch.is_ascii_digit()) {
            saw_digit = true;
            self.advance_char();
        }
        if self.current_char() == Some('.') {
            self.advance_char();
            while self.current_char().is_some_and(|ch| ch.is_ascii_digit()) {
                saw_digit = true;
                self.advance_char();
            }
        }
        if !saw_digit {
            return Err("expected digit in numeric parameter expression".to_string());
        }
        if self
            .current_char()
            .is_some_and(|ch| matches!(ch, 'e' | 'E'))
        {
            let exponent_index = self.index;
            self.advance_char();
            if self
                .current_char()
                .is_some_and(|ch| matches!(ch, '+' | '-'))
            {
                self.advance_char();
            }
            let exponent_start = self.index;
            while self.current_char().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance_char();
            }
            if exponent_start == self.index {
                self.index = exponent_index;
            }
        }

        let numeric = self.expression[start..self.index]
            .parse::<f64>()
            .map_err(|_| "invalid numeric parameter expression".to_string())?;
        let suffix_start = self.index;
        while self
            .current_char()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        {
            self.advance_char();
        }
        let suffix = self.expression[suffix_start..self.index].to_ascii_lowercase();
        if suffix.is_empty() {
            return Ok(numeric);
        }
        let Some(factor) = spice_suffix_factor(&suffix) else {
            return Err(format!("unsupported numeric suffix {suffix:?}"));
        };
        Ok(numeric * factor)
    }

    fn parse_identifier(&mut self) -> Result<f64, String> {
        let start = self.index;
        while self
            .current_char()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            self.advance_char();
        }
        let name = self.expression[start..self.index].to_string();
        self.skip_whitespace();
        if self.current_char() == Some('(') {
            let values = self.parse_call_arguments()?;
            return self.evaluate_function_call(&name, &values);
        }
        if let Some(value) = self.local_values.get(&name.to_ascii_lowercase()) {
            return Ok(*value);
        }
        if name.eq_ignore_ascii_case("pi") {
            return Ok(std::f64::consts::PI);
        }
        let Some(parameter) = self.state.get_parameter(&name) else {
            return Err(format!("unknown parameter {name:?}"));
        };
        Ok(parameter.value)
    }

    fn parse_call_arguments(&mut self) -> Result<Vec<f64>, String> {
        if !self.match_token("(") {
            return Err("expected '(' in function call".to_string());
        }
        self.skip_whitespace();
        if self.match_token(")") {
            return Ok(Vec::new());
        }
        let mut values = Vec::new();
        loop {
            values.push(self.parse_expression()?);
            self.skip_whitespace();
            if self.match_token(",") {
                continue;
            }
            if self.match_token(")") {
                return Ok(values);
            }
            return Err("missing ')' in function call".to_string());
        }
    }

    fn evaluate_function_call(&self, name: &str, values: &[f64]) -> Result<f64, String> {
        let Some(definition) = self.state.get_function(name) else {
            return Err(format!("unknown function {name:?}"));
        };
        if values.len() != definition.arguments.len() {
            return Err(format!(
                "function {name:?} expected {} arguments but got {}",
                definition.arguments.len(),
                values.len()
            ));
        }
        let key = definition.name.to_ascii_lowercase();
        if self.call_stack.contains(&key) {
            return Err(format!("recursive function call {name:?}"));
        }
        let mut local_values = self.local_values.clone();
        for (argument, value) in definition.arguments.iter().zip(values.iter()) {
            local_values.insert(argument.to_ascii_lowercase(), *value);
        }
        let mut call_stack = self.call_stack.clone();
        call_stack.push(key);
        ParameterExpressionParser::new_with_context(
            &definition.expression,
            self.state,
            local_values,
            call_stack,
        )
        .parse()
    }

    fn skip_whitespace(&mut self) {
        while self.current_char().is_some_and(|ch| ch.is_whitespace()) {
            self.advance_char();
        }
    }

    fn match_token(&mut self, token: &str) -> bool {
        if self.expression[self.index..].starts_with(token) {
            self.index += token.len();
            true
        } else {
            false
        }
    }

    fn current_char(&self) -> Option<char> {
        self.expression[self.index..].chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(ch) = self.current_char() {
            self.index += ch.len_utf8();
        }
    }
}

fn add_parameter_diagnostic(
    state: &mut DeckParameterState,
    code: &str,
    directive: &str,
    line_number: usize,
    message: &str,
    parameter: Option<String>,
    expression: Option<String>,
) {
    state.diagnostics.push(DeckParameterDiagnostic {
        code: code.to_string(),
        directive: directive.to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        parameter,
        expression,
    });
}

fn add_initial_condition_diagnostic(
    state: &mut DeckInitialConditionState,
    code: &str,
    directive: &str,
    line_number: usize,
    message: &str,
    token: Option<String>,
) {
    state.diagnostics.push(DeckInitialConditionDiagnostic {
        code: code.to_string(),
        directive: directive.to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        token,
    });
}

fn add_function_diagnostic(
    state: &mut DeckFunctionState,
    code: &str,
    line_number: usize,
    message: &str,
    function_name: Option<String>,
    expression: Option<String>,
) {
    state.diagnostics.push(DeckFunctionDiagnostic {
        code: code.to_string(),
        directive: ".func".to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        function_name,
        expression,
    });
}

fn add_measurement_diagnostic(
    state: &mut DeckMeasurementState,
    code: &str,
    directive: &str,
    line_number: usize,
    message: &str,
    token: Option<String>,
) {
    state.diagnostics.push(DeckMeasurementDiagnostic {
        code: code.to_string(),
        directive: directive.to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        token,
    });
}

fn add_fourier_diagnostic(
    state: &mut DeckFourierState,
    code: &str,
    line_number: usize,
    message: &str,
    token: Option<String>,
) {
    state.diagnostics.push(DeckFourierDiagnostic {
        code: code.to_string(),
        directive: ".four".to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        token,
    });
}

fn add_output_diagnostic(
    state: &mut DeckOutputState,
    code: &str,
    directive: &str,
    line_number: usize,
    message: &str,
    token: Option<String>,
) {
    state.diagnostics.push(DeckOutputDiagnostic {
        code: code.to_string(),
        directive: directive.to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        token,
    });
}

fn add_analysis_diagnostic(
    state: &mut DeckAnalysisState,
    code: &str,
    directive: &str,
    line_number: usize,
    message: &str,
    token: Option<String>,
) {
    state.diagnostics.push(DeckAnalysisDiagnostic {
        code: code.to_string(),
        directive: directive.to_string(),
        line_number,
        message: message.to_string(),
        severity: "error".to_string(),
        token,
    });
}

fn parse_node_condition_target(target: &str) -> Option<String> {
    if target.len() < 4 || !target.to_ascii_lowercase().starts_with("v(") || !target.ends_with(')')
    {
        return None;
    }
    let node = target[2..target.len() - 1].trim();
    if node.is_empty() {
        None
    } else {
        Some(node.to_string())
    }
}

fn parse_function_signature(rest: &str) -> Option<(String, Vec<String>, String)> {
    let open_index = rest.find('(')?;
    let close_index = rest[open_index + 1..].find(')')? + open_index + 1;
    let name = rest[..open_index].trim().to_string();
    let arguments_raw = rest[open_index + 1..close_index].trim();
    let expression = rest[close_index + 1..].trim().to_string();
    let arguments = if arguments_raw.is_empty() {
        Vec::new()
    } else {
        arguments_raw
            .split(',')
            .map(|argument| argument.trim().to_string())
            .collect()
    };
    Some((name, arguments, expression))
}

fn is_unsupported_parameter_directive(directive: &str) -> bool {
    let _ = directive;
    false
}

fn is_parameter_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn normalize_measurement_mode_token(mode: &str) -> Option<&'static str> {
    let normalized = mode.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "max" => Some("max"),
        "min" => Some("min"),
        "avg" | "average" | "mean" => Some("avg"),
        "rms" | "root-mean-square" => Some("rms"),
        "pp" | "p-p" | "p2p" | "peak-to-peak" | "peak2peak" => Some("pp"),
        "last" | "final" => Some("last"),
        "find" => Some("find"),
        "when" => Some("when"),
        _ => None,
    }
}

fn normalize_deck_output_analysis(analysis: &str) -> Option<&'static str> {
    match analysis.trim().to_ascii_lowercase().as_str() {
        "op" | "dcop" => Some("op"),
        "dc" => Some("dc"),
        "ac" => Some("ac"),
        "tran" | "transient" => Some("tran"),
        _ => None,
    }
}

fn normalize_deck_analysis_name(analysis: &str) -> Option<&'static str> {
    match analysis
        .trim()
        .trim_start_matches('.')
        .replace('_', "-")
        .to_ascii_lowercase()
        .as_str()
    {
        "op" | "dcop" | "operating-point" | "operatingpoint" => Some("op"),
        "dc" | "dc-sweep" | "dcsweep" => Some("dc"),
        "ac" | "ac-sweep" | "acsweep" => Some("ac"),
        "tran" | "transient" => Some("tran"),
        "tf" | "transfer-function" | "transferfunction" => Some("tf"),
        "sens" | "sensitivity" => Some("sens"),
        "noise" | "ac-noise" | "noise-ac" => Some("noise"),
        _ => None,
    }
}

fn implicit_deck_op_analysis_plan() -> DeckAnalysisPlan {
    DeckAnalysisPlan {
        directive: ".op".to_string(),
        analysis: "op".to_string(),
        line_number: 0,
        source_name: None,
        output_node: None,
        start_value: None,
        stop_value: None,
        step_value: None,
        sweep_kind: None,
        point_count: None,
        start_frequency_hz: None,
        stop_frequency_hz: None,
        step_time: None,
        stop_time: None,
        start_time: None,
        max_step: None,
        use_initial_conditions: false,
    }
}

fn deck_output_analysis_matches(requested: &str, analysis: &str) -> bool {
    normalize_deck_output_analysis(requested) == normalize_deck_output_analysis(analysis)
}

fn normalize_deck_output_probe(token: &str) -> Option<String> {
    let text = token.trim();
    if !text.ends_with(')') {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let prefix = if lower.starts_with("v(") {
        "V"
    } else if lower.starts_with("i(") {
        "I"
    } else {
        return None;
    };
    let target = text[2..text.len() - 1].trim();
    if target.is_empty()
        || target.contains('(')
        || target.contains(')')
        || target.contains(',')
        || target.chars().any(char::is_whitespace)
    {
        return None;
    }
    Some(format!("{prefix}({target})"))
}

fn deck_output_probe_key(probe: &str) -> String {
    probe.to_ascii_lowercase()
}

fn parse_deck_analysis_value(
    token: &str,
    directive: &str,
    line_number: usize,
    state: &mut DeckAnalysisState,
) -> Option<f64> {
    let empty_parameter_state = DeckParameterState::new();
    match evaluate_parameter_expression(
        &strip_expression_delimiters(unquote_token(token).trim()),
        &empty_parameter_state,
    ) {
        Ok(value) => Some(value),
        Err(message) => {
            add_analysis_diagnostic(
                state,
                "SPICE_DECK_ANALYSIS_EXPRESSION",
                directive,
                line_number,
                &message,
                Some(token.to_string()),
            );
            None
        }
    }
}

fn parse_deck_analysis_integer(
    token: &str,
    directive: &str,
    line_number: usize,
    state: &mut DeckAnalysisState,
) -> Option<usize> {
    let value = parse_deck_analysis_value(token, directive, line_number, state)?;
    if value < 0.0 || value.fract() != 0.0 || value > usize::MAX as f64 {
        add_analysis_diagnostic(
            state,
            "SPICE_DECK_ANALYSIS_ARGUMENT",
            directive,
            line_number,
            &format!("{directive} point count must be an integer"),
            Some(token.to_string()),
        );
        return None;
    }
    Some(value as usize)
}

fn normalize_ac_sweep_kind(token: &str) -> Option<&'static str> {
    match token.trim().to_ascii_lowercase().as_str() {
        "lin" => Some("lin"),
        "dec" => Some("dec"),
        "oct" => Some("oct"),
        _ => None,
    }
}

fn strip_expression_delimiters(expression: &str) -> String {
    if expression.len() >= 2 {
        let first = expression.as_bytes()[0] as char;
        let last = expression.as_bytes()[expression.len() - 1] as char;
        if (first == '{' && last == '}') || (first == '\'' && last == '\'') {
            return expression[1..expression.len() - 1].trim().to_string();
        }
    }
    expression.to_string()
}

fn spice_suffix_factor(suffix: &str) -> Option<f64> {
    match suffix {
        "t" => Some(1.0e12),
        "g" => Some(1.0e9),
        "meg" => Some(1.0e6),
        "k" => Some(1.0e3),
        "m" => Some(1.0e-3),
        "mil" => Some(25.4e-6),
        "u" => Some(1.0e-6),
        "n" => Some(1.0e-9),
        "p" => Some(1.0e-12),
        "f" => Some(1.0e-15),
        _ => None,
    }
}

fn format_parameter_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let abs_value = value.abs();
    if (1.0e-12..1.0e12).contains(&abs_value) {
        let mut formatted = format!("{value:.12}");
        if formatted.contains('.') {
            while formatted.ends_with('0') {
                formatted.pop();
            }
            if formatted.ends_with('.') {
                formatted.pop();
            }
        }
        if formatted == "-0" {
            "0".to_string()
        } else {
            formatted
        }
    } else {
        let raw = format!("{value:.12e}");
        let (mantissa, exponent) = raw.split_once('e').unwrap_or((raw.as_str(), "0"));
        let mut mantissa = mantissa.to_string();
        while mantissa.ends_with('0') {
            mantissa.pop();
        }
        if mantissa.ends_with('.') {
            mantissa.pop();
        }
        let exponent_value = exponent.parse::<i32>().unwrap_or(0);
        format!("{mantissa}e{exponent_value:+}")
    }
}

fn directive_tokens(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

fn unquote_token(token: &str) -> String {
    if token.len() >= 2 {
        let first = token.as_bytes()[0] as char;
        let last = token.as_bytes()[token.len() - 1] as char;
        if first == last && (first == '"' || first == '\'') {
            return token[1..token.len() - 1].to_string();
        }
    }
    token.to_string()
}

fn deck_directive(line: &str) -> Option<String> {
    if !line.starts_with('.') {
        return None;
    }
    Some(
        line.split_whitespace()
            .next()
            .unwrap_or(line)
            .to_ascii_lowercase(),
    )
}

fn control_block_command_as_deck_line(line: &str) -> Option<String> {
    let command_token = line.split_whitespace().next()?;
    let command = command_token.to_ascii_lowercase();
    let directive = match command.as_str() {
        "op" | ".op" => ".op",
        "dc" | ".dc" => ".dc",
        "ac" | ".ac" => ".ac",
        "tran" | ".tran" => ".tran",
        "save" | ".save" => ".save",
        "probe" | ".probe" => ".probe",
        "measure" | ".measure" => ".measure",
        "meas" | ".meas" => ".meas",
        "four" | ".four" | "fourier" | ".fourier" => ".four",
        "print" | ".print" => ".print",
        "plot" | ".plot" => ".plot",
        _ => return None,
    };
    let rest = line[command_token.len()..].trim_start();
    if rest.is_empty() {
        Some(directive.to_string())
    } else {
        Some(format!("{directive} {rest}"))
    }
}

fn control_block_write_marker(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    let command = parts.next()?.to_ascii_lowercase();
    if matches!(command.as_str(), "write" | ".write") {
        let rest = parts.collect::<Vec<_>>();
        if rest.is_empty() {
            return None;
        }
        return Some(format!("write {}", rest.join(" ")));
    }
    if matches!(command.as_str(), "wrdata" | ".wrdata") {
        let rest = parts.collect::<Vec<_>>();
        if rest.len() < 2 {
            return None;
        }
        return Some(format!("wrdata {}", rest.join(" ")));
    }
    None
}

fn control_block_rawfile_option(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    let command = parts.next()?.to_ascii_lowercase();
    if !matches!(command.as_str(), "set" | ".set") {
        return None;
    }
    let option = parts.next()?.to_ascii_lowercase();
    if parts.next().is_some() {
        return None;
    }
    if matches!(
        option.as_str(),
        "filetype=ascii" | "wr_vecnames" | "wr_singlescale" | "appendwrite"
    ) {
        return Some(format!("set {option}"));
    }
    None
}

fn is_noop_control_block_command(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let Some(command) = parts.next().map(|command| command.to_ascii_lowercase()) else {
        return false;
    };
    if matches!(
        command.as_str(),
        "display"
            | ".display"
            | "listing"
            | ".listing"
            | "show"
            | ".show"
            | "showmod"
            | ".showmod"
            | "status"
            | ".status"
            | "version"
            | ".version"
            | "help"
            | ".help"
            | "echo"
            | ".echo"
            | "rusage"
            | ".rusage"
            | "where"
            | ".where"
            | "run"
            | ".run"
            | "reset"
            | ".reset"
            | "quit"
            | ".quit"
    ) {
        return true;
    }
    if matches!(command.as_str(), "write" | ".write") {
        return parts.next().is_some();
    }
    if matches!(command.as_str(), "wrdata" | ".wrdata") {
        return parts.nth(1).is_some();
    }
    if !matches!(command.as_str(), "set" | ".set") {
        return false;
    }
    matches!(parts.next().map(|option| option.to_ascii_lowercase()), Some(option) if matches!(option.as_str(), "noaskquit" | "filetype=ascii" | "wr_vecnames" | "wr_singlescale" | "appendwrite"))
        && parts.next().is_none()
}

fn is_script_control_block_command(line: &str) -> bool {
    let Some(command) = line
        .split_whitespace()
        .next()
        .map(|command| command.to_ascii_lowercase())
    else {
        return false;
    };
    matches!(command.as_str(), "source" | ".source" | "shell" | ".shell")
}

fn is_workdir_control_block_command(line: &str) -> bool {
    let Some(command) = line
        .split_whitespace()
        .next()
        .map(|command| command.to_ascii_lowercase())
    else {
        return false;
    };
    matches!(command.as_str(), "cd" | ".cd")
}

fn is_control_flow_control_block_command(line: &str) -> bool {
    let Some(command) = line
        .split_whitespace()
        .next()
        .map(|command| command.to_ascii_lowercase())
    else {
        return false;
    };
    matches!(
        command.as_str(),
        "if" | ".if"
            | "else"
            | ".else"
            | "end"
            | ".end"
            | "while"
            | ".while"
            | "foreach"
            | ".foreach"
            | "repeat"
            | ".repeat"
            | "dowhile"
            | ".dowhile"
            | "break"
            | ".break"
            | "continue"
            | ".continue"
    )
}

fn is_variable_control_block_command(line: &str) -> bool {
    let Some(command) = line
        .split_whitespace()
        .next()
        .map(|command| command.to_ascii_lowercase())
    else {
        return false;
    };
    matches!(
        command.as_str(),
        "let"
            | ".let"
            | "alter"
            | ".alter"
            | "alterparam"
            | ".alterparam"
            | "set"
            | ".set"
            | "unset"
            | ".unset"
    )
}

fn control_block_script_policy_message(line: &str) -> String {
    format!(
        "{line:?} inside .control is not executed because external script and shell commands are disabled by the deck execution policy"
    )
}

fn control_block_workdir_policy_message(line: &str) -> String {
    format!(
        "{line:?} inside .control is not executed because working-directory mutation is disabled by the deck execution policy"
    )
}

fn control_block_flow_policy_message(line: &str) -> String {
    format!(
        "{line:?} inside .control is not executed because control-flow commands are disabled by the deck execution policy"
    )
}

fn control_block_variable_policy_message(line: &str) -> String {
    format!(
        "{line:?} inside .control is not executed because control variables and circuit mutation commands are disabled by the deck execution policy"
    )
}

fn is_unsupported_deck_control_directive(directive: &str) -> bool {
    matches!(directive, ".include" | ".lib" | ".control")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vccs {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub control_positive: String,
    pub control_negative: String,
    pub transconductance_siemens: f64,
}

impl Vccs {
    pub fn new(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        control_positive: impl Into<String>,
        control_negative: impl Into<String>,
        transconductance_siemens: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            control_positive: control_positive.into(),
            control_negative: control_negative.into(),
            transconductance_siemens,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vcvs {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub control_positive: String,
    pub control_negative: String,
    pub gain: f64,
}

impl Vcvs {
    pub fn new(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        control_positive: impl Into<String>,
        control_negative: impl Into<String>,
        gain: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            control_positive: control_positive.into(),
            control_negative: control_negative.into(),
            gain,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cccs {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub control_source: String,
    pub gain: f64,
}

impl Cccs {
    pub fn new(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        control_source: impl Into<String>,
        gain: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            control_source: control_source.into(),
            gain,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ccvs {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub control_source: String,
    pub transresistance_ohms: f64,
}

impl Ccvs {
    pub fn new(
        name: impl Into<String>,
        positive: impl Into<String>,
        negative: impl Into<String>,
        control_source: impl Into<String>,
        transresistance_ohms: f64,
    ) -> Self {
        Self {
            name: name.into(),
            positive: positive.into(),
            negative: negative.into(),
            control_source: control_source.into(),
            transresistance_ohms,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DcConvergenceAid {
    Newton,
    Gmin,
    Source,
    PseudoTransient,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcSolverDiagnostics {
    pub matrix_size: usize,
    pub solver: String,
    pub tolerance: f64,
    pub max_delta: f64,
    pub convergence_aid: DcConvergenceAid,
    pub newton_step_limit: Option<f64>,
    pub limited_newton_steps: usize,
    pub minimum_damping_factor: f64,
    pub solver_profile: LinearSolverProfile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcResult {
    pub node_voltages: BTreeMap<String, f64>,
    pub branch_currents: BTreeMap<String, f64>,
    pub iterations: usize,
    pub converged: bool,
    pub convergence_aid: DcConvergenceAid,
    pub diagnostics: DcSolverDiagnostics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerOverride {
    pub element_name: String,
    pub parameter: String,
    pub value: f64,
}

impl CornerOverride {
    pub fn new(element_name: impl Into<String>, parameter: impl Into<String>, value: f64) -> Self {
        Self {
            element_name: element_name.into(),
            parameter: parameter.into(),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerSpec {
    pub name: String,
    pub overrides: Vec<CornerOverride>,
}

impl CornerSpec {
    pub fn new(name: impl Into<String>, overrides: impl Into<Vec<CornerOverride>>) -> Self {
        Self {
            name: name.into(),
            overrides: overrides.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerPoint {
    pub corner_name: String,
    pub result: DcResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerSweepResult {
    pub points: Vec<CornerPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemperatureDcPoint {
    pub temperature_kelvin: f64,
    pub result: DcResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemperatureDcResult {
    pub points: Vec<TemperatureDcPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerTemperatureDcPoint {
    pub corner_name: String,
    pub points: Vec<TemperatureDcPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerTemperatureDcResult {
    pub points: Vec<CornerTemperatureDcPoint>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DcOpOptions {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub convergence_aids: bool,
    pub pseudo_transient_steps: usize,
    pub pseudo_transient_conductance: f64,
    pub pseudo_transient_max_iterations: usize,
    pub newton_step_limit: Option<f64>,
}

impl Default for DcOpOptions {
    fn default() -> Self {
        Self {
            max_iterations: 80,
            tolerance: 1.0e-9,
            convergence_aids: true,
            pseudo_transient_steps: 20,
            pseudo_transient_conductance: 1.0e-3,
            pseudo_transient_max_iterations: 80,
            newton_step_limit: Some(DEFAULT_NEWTON_STEP_LIMIT),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcSweepPoint {
    pub value: f64,
    pub result: DcResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerDcSweepPoint {
    pub corner_name: String,
    pub points: Vec<DcSweepPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerDcSweepResult {
    pub source_name: String,
    pub points: Vec<CornerDcSweepPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerAcSweepPoint {
    pub corner_name: String,
    pub points: Vec<AcPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerAcSweepResult {
    pub points: Vec<CornerAcSweepPoint>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum McDistribution {
    Gaussian,
    Uniform,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct McOptions {
    pub tolerance: f64,
    pub distribution: McDistribution,
    pub seed: Option<u64>,
}

impl Default for McOptions {
    fn default() -> Self {
        Self {
            tolerance: 0.05,
            distribution: McDistribution::Gaussian,
            seed: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct McPoint {
    pub trial: usize,
    pub node_voltages: BTreeMap<String, f64>,
    pub branch_currents: BTreeMap<String, f64>,
    pub converged: bool,
}

impl McPoint {
    pub fn voltage(&self, node: &str) -> Option<f64> {
        if is_ground(node) {
            Some(0.0)
        } else {
            self.node_voltages.get(node).copied()
        }
    }

    pub fn branch_current(&self, source_name: &str) -> Option<f64> {
        let key = if source_name.starts_with("I(") {
            source_name.to_string()
        } else {
            format!("I({source_name})")
        };
        self.branch_currents.get(&key).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct McResult {
    pub output_node: String,
    pub points: Vec<McPoint>,
    pub n_trials: usize,
    pub mean: f64,
    pub std_dev: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerMcPoint {
    pub corner_name: String,
    pub result: McResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerMcResult {
    pub output_node: String,
    pub points: Vec<CornerMcPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TfResult {
    pub transfer_ratio: f64,
    pub input_impedance_ohms: f64,
    pub output_impedance_ohms: f64,
}

impl TfResult {
    pub fn gain(&self) -> f64 {
        self.transfer_ratio
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerTfPoint {
    pub corner_name: String,
    pub result: TfResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerTfResult {
    pub input_source: String,
    pub output_node: String,
    pub points: Vec<CornerTfPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensEntry {
    pub element_name: String,
    pub parameter: String,
    pub nominal_value: f64,
    pub sensitivity: f64,
    pub relative_sensitivity: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensResult {
    pub output_node: String,
    pub nominal_voltage: f64,
    pub entries: Vec<SensEntry>,
}

impl SensResult {
    pub fn entry(&self, element_name: &str, parameter: &str) -> Option<&SensEntry> {
        self.entries
            .iter()
            .find(|entry| entry.element_name == element_name && entry.parameter == parameter)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerSensPoint {
    pub corner_name: String,
    pub result: SensResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerSensResult {
    pub output_node: String,
    pub points: Vec<CornerSensPoint>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}

impl Complex {
    pub fn new(real: f64, imag: f64) -> Self {
        Self { real, imag }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    pub fn abs(self) -> f64 {
        self.real.hypot(self.imag)
    }

    pub fn phase(self) -> f64 {
        self.imag.atan2(self.real)
    }

    fn is_finite(self) -> bool {
        self.real.is_finite() && self.imag.is_finite()
    }
}

impl std::ops::Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.real + rhs.real, self.imag + rhs.imag)
    }
}

impl std::ops::AddAssign for Complex {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.real - rhs.real, self.imag - rhs.imag)
    }
}

impl std::ops::SubAssign for Complex {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.real * rhs.real - self.imag * rhs.imag,
            self.real * rhs.imag + self.imag * rhs.real,
        )
    }
}

impl std::ops::Div for Complex {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let denominator = rhs.real * rhs.real + rhs.imag * rhs.imag;
        Self::new(
            (self.real * rhs.real + self.imag * rhs.imag) / denominator,
            (self.imag * rhs.real - self.real * rhs.imag) / denominator,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AcPoint {
    pub frequency_hz: f64,
    pub node_voltages: BTreeMap<String, Complex>,
    pub branch_currents: BTreeMap<String, Complex>,
}

impl AcPoint {
    pub fn voltage(&self, node: &str) -> Option<Complex> {
        if is_ground(node) {
            Some(Complex::zero())
        } else {
            self.node_voltages.get(node).copied()
        }
    }

    pub fn branch_current(&self, source_name: &str) -> Option<Complex> {
        let key = if source_name.starts_with("I(") {
            source_name.to_string()
        } else {
            format!("I({source_name})")
        };
        self.branch_currents.get(&key).copied()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SParameterPoint {
    pub frequency_hz: f64,
    pub s11: Complex,
    pub s21: Complex,
    pub s12: Complex,
    pub s22: Complex,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SParameterResult {
    pub port1_source: String,
    pub port2_source: String,
    pub reference_impedance_ohms: f64,
    pub points: Vec<SParameterPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerSParameterPoint {
    pub corner_name: String,
    pub result: SParameterResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerSParameterResult {
    pub port1_source: String,
    pub port2_source: String,
    pub reference_impedance_ohms: f64,
    pub points: Vec<CornerSParameterPoint>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum NoiseType {
    Thermal,
    Shot,
    Flicker,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseEntry {
    pub element_name: String,
    pub noise_type: NoiseType,
    pub source_psd: f64,
    pub output_psd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoisePoint {
    pub frequency_hz: f64,
    pub output_psd: f64,
    pub input_referred_psd: f64,
    pub entries: Vec<NoiseEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseResult {
    pub output_node: String,
    pub input_source: String,
    pub temperature_kelvin: f64,
    pub points: Vec<NoisePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerNoisePoint {
    pub corner_name: String,
    pub result: NoiseResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerNoiseResult {
    pub output_node: String,
    pub input_source: String,
    pub points: Vec<CornerNoisePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransientPoint {
    pub time: f64,
    pub node_voltages: BTreeMap<String, f64>,
    pub branch_currents: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProbeMeasurement {
    pub name: String,
    pub analysis: String,
    pub probe: String,
    pub mode: String,
    pub value: f64,
    pub from_value: Option<f64>,
    pub to_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerTransientPoint {
    pub corner_name: String,
    pub points: Vec<TransientPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerTransientResult {
    pub points: Vec<CornerTransientPoint>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TransientMethod {
    Euler,
    Trap,
    Gear2,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AdaptiveTransientOptions {
    pub method: TransientMethod,
    pub tolerance: f64,
    pub min_step: Option<f64>,
    pub max_step: Option<f64>,
}

impl Default for AdaptiveTransientOptions {
    fn default() -> Self {
        Self {
            method: TransientMethod::Trap,
            tolerance: 1.0e-4,
            min_step: None,
            max_step: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveTransientResult {
    pub points: Vec<TransientPoint>,
    pub method: TransientMethod,
    pub steps_rejected: usize,
    pub converged: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerAdaptiveTransientPoint {
    pub corner_name: String,
    pub result: AdaptiveTransientResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerAdaptiveTransientResult {
    pub points: Vec<CornerAdaptiveTransientPoint>,
}

impl TransientPoint {
    pub fn voltage(&self, node: &str) -> Option<f64> {
        if is_ground(node) {
            Some(0.0)
        } else {
            self.node_voltages.get(node).copied()
        }
    }

    pub fn branch_current(&self, source_name: &str) -> Option<f64> {
        let key = if source_name.starts_with("I(") {
            source_name.to_string()
        } else {
            format!("I({source_name})")
        };
        self.branch_currents.get(&key).copied()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourierHarmonic {
    pub harmonic: usize,
    pub frequency_hz: f64,
    pub cosine: f64,
    pub sine: f64,
    pub magnitude: f64,
    pub phase_degrees: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourierProbeResult {
    pub probe: String,
    pub dc: f64,
    pub harmonics: Vec<FourierHarmonic>,
    pub total_harmonic_distortion: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FourierResult {
    pub fundamental_frequency_hz: f64,
    pub start_time: f64,
    pub end_time: f64,
    pub probes: Vec<FourierProbeResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerFourierPoint {
    pub corner_name: String,
    pub result: FourierResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerFourierResult {
    pub fundamental_frequency_hz: f64,
    pub points: Vec<CornerFourierPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistortionHarmonic {
    pub harmonic: usize,
    pub frequency_hz: f64,
    pub magnitude: f64,
    pub phase_degrees: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistortionPoint {
    pub frequency_hz: f64,
    pub fundamental_magnitude: f64,
    pub harmonics: Vec<DistortionHarmonic>,
    pub total_harmonic_distortion: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DistortionResult {
    pub input_source: String,
    pub output_probe: String,
    pub points: Vec<DistortionPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerDistortionPoint {
    pub corner_name: String,
    pub result: DistortionResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerDistortionResult {
    pub input_source: String,
    pub output_probe: String,
    pub points: Vec<CornerDistortionPoint>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PoleZeroEntryKind {
    Pole,
    Zero,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoleZeroEntry {
    pub kind: PoleZeroEntryKind,
    pub real: f64,
    pub imaginary: f64,
    pub frequency_hz: f64,
    pub damping: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoleZeroResult {
    pub input_source: String,
    pub output_node: String,
    pub entries: Vec<PoleZeroEntry>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PoleZeroTopology {
    RcLowpass,
    RcHighpass,
    RlcLowpass,
    RlcHighpass,
    RlcBandpass,
    RlcNotch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerPoleZeroPoint {
    pub corner_name: String,
    pub result: PoleZeroResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerPoleZeroResult {
    pub input_source: String,
    pub output_node: String,
    pub topology: PoleZeroTopology,
    pub points: Vec<CornerPoleZeroPoint>,
}

pub fn pole_zero_corners(
    circuit: &Circuit,
    input_source: &str,
    output_node: &str,
    topology: PoleZeroTopology,
    corners: &[CornerSpec],
) -> Result<CornerPoleZeroResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        let result = match topology {
            PoleZeroTopology::RcLowpass => {
                pole_zero_rc_lowpass(&corner_circuit, input_source, output_node)?
            }
            PoleZeroTopology::RcHighpass => {
                pole_zero_rc_highpass(&corner_circuit, input_source, output_node)?
            }
            PoleZeroTopology::RlcLowpass => {
                pole_zero_rlc_lowpass(&corner_circuit, input_source, output_node)?
            }
            PoleZeroTopology::RlcHighpass => {
                pole_zero_rlc_highpass(&corner_circuit, input_source, output_node)?
            }
            PoleZeroTopology::RlcBandpass => {
                pole_zero_rlc_bandpass(&corner_circuit, input_source, output_node)?
            }
            PoleZeroTopology::RlcNotch => {
                pole_zero_rlc_notch(&corner_circuit, input_source, output_node)?
            }
        };
        points.push(CornerPoleZeroPoint {
            corner_name: corner.name.clone(),
            result,
        });
    }
    Ok(CornerPoleZeroResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        topology,
        points,
    })
}

pub fn pole_zero_rc_lowpass(
    circuit: &Circuit,
    input_source: &str,
    output_node: &str,
) -> Result<PoleZeroResult, SpiceError> {
    let source = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::VoltageSource(source) if source.name == input_source => Some(source),
            _ => None,
        })
        .ok_or_else(|| SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rc_lowpass: missing input source".to_string(),
        })?;
    if !is_ground(&source.negative) {
        return Err(SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rc_lowpass: input source negative terminal must be ground"
                .to_string(),
        });
    }

    let resistor = circuit.elements().iter().find_map(|element| match element {
        Element::Resistor(resistor)
            if (resistor.n1 == source.positive && resistor.n2 == output_node)
                || (resistor.n2 == source.positive && resistor.n1 == output_node) =>
        {
            Some(resistor)
        }
        _ => None,
    });
    let capacitor = circuit.elements().iter().find_map(|element| match element {
        Element::Capacitor(capacitor)
            if (capacitor.n1 == output_node || capacitor.n2 == output_node)
                && (is_ground(&capacitor.n1) || is_ground(&capacitor.n2)) =>
        {
            Some(capacitor)
        }
        _ => None,
    });
    let (Some(resistor), Some(capacitor)) = (resistor, capacitor) else {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "pole_zero_rc_lowpass: expected one resistor from input to output and one grounded output capacitor"
                .to_string(),
        });
    };
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "pole_zero_rc_lowpass: resistance must be finite and positive".to_string(),
        });
    }
    if !capacitor.capacitance_farads.is_finite() || capacitor.capacitance_farads <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "pole_zero_rc_lowpass: capacitance must be finite and positive".to_string(),
        });
    }

    let real = -1.0 / (resistor.resistance_ohms * capacitor.capacitance_farads);
    Ok(PoleZeroResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        entries: vec![PoleZeroEntry {
            kind: PoleZeroEntryKind::Pole,
            real,
            imaginary: 0.0,
            frequency_hz: real.abs() / TWO_PI,
            damping: 1.0,
        }],
    })
}

pub fn pole_zero_rc_highpass(
    circuit: &Circuit,
    input_source: &str,
    output_node: &str,
) -> Result<PoleZeroResult, SpiceError> {
    let source = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::VoltageSource(source) if source.name == input_source => Some(source),
            _ => None,
        })
        .ok_or_else(|| SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rc_highpass: missing input source".to_string(),
        })?;
    if !is_ground(&source.negative) {
        return Err(SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rc_highpass: input source negative terminal must be ground"
                .to_string(),
        });
    }

    let capacitor = circuit.elements().iter().find_map(|element| match element {
        Element::Capacitor(capacitor)
            if (capacitor.n1 == source.positive && capacitor.n2 == output_node)
                || (capacitor.n2 == source.positive && capacitor.n1 == output_node) =>
        {
            Some(capacitor)
        }
        _ => None,
    });
    let resistor = circuit.elements().iter().find_map(|element| match element {
        Element::Resistor(resistor)
            if (resistor.n1 == output_node || resistor.n2 == output_node)
                && (is_ground(&resistor.n1) || is_ground(&resistor.n2)) =>
        {
            Some(resistor)
        }
        _ => None,
    });
    let (Some(capacitor), Some(resistor)) = (capacitor, resistor) else {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "pole_zero_rc_highpass: expected one capacitor from input to output and one grounded output resistor"
                .to_string(),
        });
    };
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "pole_zero_rc_highpass: resistance must be finite and positive".to_string(),
        });
    }
    if !capacitor.capacitance_farads.is_finite() || capacitor.capacitance_farads <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "pole_zero_rc_highpass: capacitance must be finite and positive".to_string(),
        });
    }

    let real = -1.0 / (resistor.resistance_ohms * capacitor.capacitance_farads);
    Ok(PoleZeroResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        entries: vec![
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Zero,
                real: 0.0,
                imaginary: 0.0,
                frequency_hz: 0.0,
                damping: 1.0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real,
                imaginary: 0.0,
                frequency_hz: real.abs() / TWO_PI,
                damping: 1.0,
            },
        ],
    })
}

pub fn pole_zero_rlc_lowpass(
    circuit: &Circuit,
    input_source: &str,
    output_node: &str,
) -> Result<PoleZeroResult, SpiceError> {
    let source = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::VoltageSource(source) if source.name == input_source => Some(source),
            _ => None,
        })
        .ok_or_else(|| SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_lowpass: missing input source".to_string(),
        })?;
    if !is_ground(&source.negative) {
        return Err(SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_lowpass: input source negative terminal must be ground"
                .to_string(),
        });
    }

    let mut resistor = None;
    let mut intermediate: Option<&str> = None;
    for element in circuit.elements() {
        let Element::Resistor(candidate) = element else {
            continue;
        };
        if candidate.n1 == source.positive || candidate.n2 == source.positive {
            let other = if candidate.n1 == source.positive {
                candidate.n2.as_str()
            } else {
                candidate.n1.as_str()
            };
            if other != output_node && !is_ground(other) {
                resistor = Some(candidate);
                intermediate = Some(other);
                break;
            }
        }
    }
    let inductor = circuit.elements().iter().find_map(|element| match element {
        Element::Inductor(inductor)
            if intermediate.is_some_and(|node| {
                (inductor.n1 == node && inductor.n2 == output_node)
                    || (inductor.n2 == node && inductor.n1 == output_node)
            }) =>
        {
            Some(inductor)
        }
        _ => None,
    });
    let capacitor = circuit.elements().iter().find_map(|element| match element {
        Element::Capacitor(capacitor)
            if (capacitor.n1 == output_node || capacitor.n2 == output_node)
                && (is_ground(&capacitor.n1) || is_ground(&capacitor.n2)) =>
        {
            Some(capacitor)
        }
        _ => None,
    });
    let (Some(resistor), Some(inductor), Some(capacitor)) = (resistor, inductor, capacitor) else {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "pole_zero_rlc_lowpass: expected series resistor and inductor from input to output plus one grounded output capacitor"
                .to_string(),
        });
    };
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "pole_zero_rlc_lowpass: resistance must be finite and positive".to_string(),
        });
    }
    if !inductor.inductance_henrys.is_finite() || inductor.inductance_henrys <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: inductor.name.clone(),
            reason: "pole_zero_rlc_lowpass: inductance must be finite and positive".to_string(),
        });
    }
    if !capacitor.capacitance_farads.is_finite() || capacitor.capacitance_farads <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "pole_zero_rlc_lowpass: capacitance must be finite and positive".to_string(),
        });
    }

    let alpha = resistor.resistance_ohms / (2.0 * inductor.inductance_henrys);
    let omega0 = 1.0 / (inductor.inductance_henrys * capacitor.capacitance_farads).sqrt();
    let discriminant = alpha * alpha - omega0 * omega0;
    let entries = if discriminant >= 0.0 {
        let root = discriminant.sqrt();
        let first = -alpha + root;
        let second = -alpha - root;
        vec![
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: first,
                imaginary: 0.0,
                frequency_hz: first.abs() / TWO_PI,
                damping: 1.0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: second,
                imaginary: 0.0,
                frequency_hz: second.abs() / TWO_PI,
                damping: 1.0,
            },
        ]
    } else {
        let imaginary = (-discriminant).sqrt();
        vec![
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary: -imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
        ]
    };

    Ok(PoleZeroResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        entries,
    })
}

pub fn pole_zero_rlc_highpass(
    circuit: &Circuit,
    input_source: &str,
    output_node: &str,
) -> Result<PoleZeroResult, SpiceError> {
    let source = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::VoltageSource(source) if source.name == input_source => Some(source),
            _ => None,
        })
        .ok_or_else(|| SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_highpass: missing input source".to_string(),
        })?;
    if !is_ground(&source.negative) {
        return Err(SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_highpass: input source negative terminal must be ground"
                .to_string(),
        });
    }

    let mut resistor = None;
    let mut intermediate: Option<&str> = None;
    for element in circuit.elements() {
        let Element::Resistor(candidate) = element else {
            continue;
        };
        if candidate.n1 == source.positive || candidate.n2 == source.positive {
            let other = if candidate.n1 == source.positive {
                candidate.n2.as_str()
            } else {
                candidate.n1.as_str()
            };
            if other != output_node && !is_ground(other) {
                resistor = Some(candidate);
                intermediate = Some(other);
                break;
            }
        }
    }
    let capacitor = circuit.elements().iter().find_map(|element| match element {
        Element::Capacitor(capacitor)
            if intermediate.is_some_and(|node| {
                (capacitor.n1 == node && capacitor.n2 == output_node)
                    || (capacitor.n2 == node && capacitor.n1 == output_node)
            }) =>
        {
            Some(capacitor)
        }
        _ => None,
    });
    let inductor = circuit.elements().iter().find_map(|element| match element {
        Element::Inductor(inductor)
            if (inductor.n1 == output_node || inductor.n2 == output_node)
                && (is_ground(&inductor.n1) || is_ground(&inductor.n2)) =>
        {
            Some(inductor)
        }
        _ => None,
    });
    let (Some(resistor), Some(capacitor), Some(inductor)) = (resistor, capacitor, inductor) else {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "pole_zero_rlc_highpass: expected series resistor and capacitor from input to output plus one grounded output inductor"
                .to_string(),
        });
    };
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "pole_zero_rlc_highpass: resistance must be finite and positive".to_string(),
        });
    }
    if !capacitor.capacitance_farads.is_finite() || capacitor.capacitance_farads <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "pole_zero_rlc_highpass: capacitance must be finite and positive".to_string(),
        });
    }
    if !inductor.inductance_henrys.is_finite() || inductor.inductance_henrys <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: inductor.name.clone(),
            reason: "pole_zero_rlc_highpass: inductance must be finite and positive".to_string(),
        });
    }

    let alpha = resistor.resistance_ohms / (2.0 * inductor.inductance_henrys);
    let omega0 = 1.0 / (inductor.inductance_henrys * capacitor.capacitance_farads).sqrt();
    let discriminant = alpha * alpha - omega0 * omega0;
    let mut entries = vec![
        PoleZeroEntry {
            kind: PoleZeroEntryKind::Zero,
            real: 0.0,
            imaginary: 0.0,
            frequency_hz: 0.0,
            damping: 1.0,
        },
        PoleZeroEntry {
            kind: PoleZeroEntryKind::Zero,
            real: 0.0,
            imaginary: 0.0,
            frequency_hz: 0.0,
            damping: 1.0,
        },
    ];
    if discriminant >= 0.0 {
        let root = discriminant.sqrt();
        let first = -alpha + root;
        let second = -alpha - root;
        entries.extend([
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: first,
                imaginary: 0.0,
                frequency_hz: first.abs() / TWO_PI,
                damping: 1.0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: second,
                imaginary: 0.0,
                frequency_hz: second.abs() / TWO_PI,
                damping: 1.0,
            },
        ]);
    } else {
        let imaginary = (-discriminant).sqrt();
        entries.extend([
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary: -imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
        ]);
    }

    Ok(PoleZeroResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        entries,
    })
}

pub fn pole_zero_rlc_bandpass(
    circuit: &Circuit,
    input_source: &str,
    output_node: &str,
) -> Result<PoleZeroResult, SpiceError> {
    let source = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::VoltageSource(source) if source.name == input_source => Some(source),
            _ => None,
        })
        .ok_or_else(|| SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_bandpass: missing input source".to_string(),
        })?;
    if !is_ground(&source.negative) {
        return Err(SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_bandpass: input source negative terminal must be ground"
                .to_string(),
        });
    }

    let mut inductor = None;
    let mut intermediate: Option<&str> = None;
    for element in circuit.elements() {
        let Element::Inductor(candidate) = element else {
            continue;
        };
        if candidate.n1 == source.positive || candidate.n2 == source.positive {
            let other = if candidate.n1 == source.positive {
                candidate.n2.as_str()
            } else {
                candidate.n1.as_str()
            };
            if other != output_node && !is_ground(other) {
                inductor = Some(candidate);
                intermediate = Some(other);
                break;
            }
        }
    }
    let capacitor = circuit.elements().iter().find_map(|element| match element {
        Element::Capacitor(capacitor)
            if intermediate.is_some_and(|node| {
                (capacitor.n1 == node && capacitor.n2 == output_node)
                    || (capacitor.n2 == node && capacitor.n1 == output_node)
            }) =>
        {
            Some(capacitor)
        }
        _ => None,
    });
    let resistor = circuit.elements().iter().find_map(|element| match element {
        Element::Resistor(resistor)
            if (resistor.n1 == output_node || resistor.n2 == output_node)
                && (is_ground(&resistor.n1) || is_ground(&resistor.n2)) =>
        {
            Some(resistor)
        }
        _ => None,
    });
    let (Some(inductor), Some(capacitor), Some(resistor)) = (inductor, capacitor, resistor) else {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "pole_zero_rlc_bandpass: expected series inductor and capacitor from input to output plus one grounded output resistor"
                .to_string(),
        });
    };
    if !inductor.inductance_henrys.is_finite() || inductor.inductance_henrys <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: inductor.name.clone(),
            reason: "pole_zero_rlc_bandpass: inductance must be finite and positive".to_string(),
        });
    }
    if !capacitor.capacitance_farads.is_finite() || capacitor.capacitance_farads <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "pole_zero_rlc_bandpass: capacitance must be finite and positive".to_string(),
        });
    }
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "pole_zero_rlc_bandpass: resistance must be finite and positive".to_string(),
        });
    }

    let alpha = resistor.resistance_ohms / (2.0 * inductor.inductance_henrys);
    let omega0 = 1.0 / (inductor.inductance_henrys * capacitor.capacitance_farads).sqrt();
    let discriminant = alpha * alpha - omega0 * omega0;
    let mut entries = vec![PoleZeroEntry {
        kind: PoleZeroEntryKind::Zero,
        real: 0.0,
        imaginary: 0.0,
        frequency_hz: 0.0,
        damping: 1.0,
    }];
    if discriminant >= 0.0 {
        let root = discriminant.sqrt();
        let first = -alpha + root;
        let second = -alpha - root;
        entries.extend([
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: first,
                imaginary: 0.0,
                frequency_hz: first.abs() / TWO_PI,
                damping: 1.0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: second,
                imaginary: 0.0,
                frequency_hz: second.abs() / TWO_PI,
                damping: 1.0,
            },
        ]);
    } else {
        let imaginary = (-discriminant).sqrt();
        entries.extend([
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary: -imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
        ]);
    }

    Ok(PoleZeroResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        entries,
    })
}

pub fn pole_zero_rlc_notch(
    circuit: &Circuit,
    input_source: &str,
    output_node: &str,
) -> Result<PoleZeroResult, SpiceError> {
    let source = circuit
        .elements()
        .iter()
        .find_map(|element| match element {
            Element::VoltageSource(source) if source.name == input_source => Some(source),
            _ => None,
        })
        .ok_or_else(|| SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_notch: missing input source".to_string(),
        })?;
    if !is_ground(&source.negative) {
        return Err(SpiceError::InvalidElement {
            name: input_source.to_string(),
            reason: "pole_zero_rlc_notch: input source negative terminal must be ground"
                .to_string(),
        });
    }

    let resistor = circuit.elements().iter().find_map(|element| match element {
        Element::Resistor(resistor)
            if (resistor.n1 == source.positive || resistor.n2 == source.positive)
                && (resistor.n1 == output_node || resistor.n2 == output_node) =>
        {
            Some(resistor)
        }
        _ => None,
    });
    let mut inductor = None;
    let mut intermediate: Option<&str> = None;
    for element in circuit.elements() {
        let Element::Inductor(candidate) = element else {
            continue;
        };
        if candidate.n1 == output_node || candidate.n2 == output_node {
            let other = if candidate.n1 == output_node {
                candidate.n2.as_str()
            } else {
                candidate.n1.as_str()
            };
            if !is_ground(other) {
                inductor = Some(candidate);
                intermediate = Some(other);
                break;
            }
        }
    }
    let capacitor = circuit.elements().iter().find_map(|element| match element {
        Element::Capacitor(capacitor)
            if intermediate.is_some_and(|node| capacitor.n1 == node || capacitor.n2 == node)
                && (is_ground(&capacitor.n1) || is_ground(&capacitor.n2)) =>
        {
            Some(capacitor)
        }
        _ => None,
    });
    let (Some(resistor), Some(inductor), Some(capacitor)) = (resistor, inductor, capacitor) else {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "pole_zero_rlc_notch: expected series resistor from input to output plus a grounded series inductor-capacitor branch at output"
                .to_string(),
        });
    };
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "pole_zero_rlc_notch: resistance must be finite and positive".to_string(),
        });
    }
    if !inductor.inductance_henrys.is_finite() || inductor.inductance_henrys <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: inductor.name.clone(),
            reason: "pole_zero_rlc_notch: inductance must be finite and positive".to_string(),
        });
    }
    if !capacitor.capacitance_farads.is_finite() || capacitor.capacitance_farads <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "pole_zero_rlc_notch: capacitance must be finite and positive".to_string(),
        });
    }

    let alpha = resistor.resistance_ohms / (2.0 * inductor.inductance_henrys);
    let omega0 = 1.0 / (inductor.inductance_henrys * capacitor.capacitance_farads).sqrt();
    let discriminant = alpha * alpha - omega0 * omega0;
    let mut entries = vec![
        PoleZeroEntry {
            kind: PoleZeroEntryKind::Zero,
            real: 0.0,
            imaginary: omega0,
            frequency_hz: omega0 / TWO_PI,
            damping: 0.0,
        },
        PoleZeroEntry {
            kind: PoleZeroEntryKind::Zero,
            real: 0.0,
            imaginary: -omega0,
            frequency_hz: omega0 / TWO_PI,
            damping: 0.0,
        },
    ];
    if discriminant >= 0.0 {
        let root = discriminant.sqrt();
        let first = -alpha + root;
        let second = -alpha - root;
        entries.extend([
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: first,
                imaginary: 0.0,
                frequency_hz: first.abs() / TWO_PI,
                damping: 1.0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: second,
                imaginary: 0.0,
                frequency_hz: second.abs() / TWO_PI,
                damping: 1.0,
            },
        ]);
    } else {
        let imaginary = (-discriminant).sqrt();
        entries.extend([
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
            PoleZeroEntry {
                kind: PoleZeroEntryKind::Pole,
                real: -alpha,
                imaginary: -imaginary,
                frequency_hz: omega0 / TWO_PI,
                damping: alpha / omega0,
            },
        ]);
    }

    Ok(PoleZeroResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        entries,
    })
}

pub fn distortion_from_fourier(
    result: &FourierResult,
    input_source: &str,
    output_probe: &str,
) -> Result<DistortionResult, SpiceError> {
    let probe = result
        .probes
        .iter()
        .find(|probe| probe.probe == output_probe)
        .ok_or_else(|| SpiceError::InvalidElement {
            name: output_probe.to_string(),
            reason: "distortion_from_fourier: missing probe".to_string(),
        })?;
    let Some(fundamental) = probe.harmonics.first() else {
        return Err(SpiceError::InvalidElement {
            name: output_probe.to_string(),
            reason: "distortion_from_fourier: Fourier result has no harmonics".to_string(),
        });
    };
    Ok(DistortionResult {
        input_source: input_source.to_string(),
        output_probe: output_probe.to_string(),
        points: vec![DistortionPoint {
            frequency_hz: fundamental.frequency_hz,
            fundamental_magnitude: fundamental.magnitude,
            harmonics: probe
                .harmonics
                .iter()
                .skip(1)
                .map(|harmonic| DistortionHarmonic {
                    harmonic: harmonic.harmonic,
                    frequency_hz: harmonic.frequency_hz,
                    magnitude: harmonic.magnitude,
                    phase_degrees: harmonic.phase_degrees,
                })
                .collect(),
            total_harmonic_distortion: probe.total_harmonic_distortion,
        }],
    })
}

pub fn distortion_from_transient(
    points: &[TransientPoint],
    fundamental_frequency_hz: f64,
    input_source: &str,
    output_probe: &str,
    harmonics: usize,
) -> Result<DistortionResult, SpiceError> {
    distortion_from_transient_with_start_time(
        points,
        fundamental_frequency_hz,
        input_source,
        output_probe,
        harmonics,
        None,
    )
}

pub fn distortion_from_transient_with_start_time(
    points: &[TransientPoint],
    fundamental_frequency_hz: f64,
    input_source: &str,
    output_probe: &str,
    harmonics: usize,
    start_time: Option<f64>,
) -> Result<DistortionResult, SpiceError> {
    let fourier_result = fourier_with_start_time(
        points,
        fundamental_frequency_hz,
        &[output_probe],
        harmonics,
        start_time,
    )?;
    distortion_from_fourier(&fourier_result, input_source, output_probe)
}

pub fn distortion_from_transient_corners(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    fundamental_frequency_hz: f64,
    input_source: &str,
    output_probe: &str,
    harmonics: usize,
    corners: &[CornerSpec],
) -> Result<CornerDistortionResult, SpiceError> {
    distortion_from_transient_corners_with_start_time(
        circuit,
        time_step,
        stop_time,
        fundamental_frequency_hz,
        input_source,
        output_probe,
        harmonics,
        corners,
        None,
    )
}

pub fn distortion_from_transient_corners_with_start_time(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    fundamental_frequency_hz: f64,
    input_source: &str,
    output_probe: &str,
    harmonics: usize,
    corners: &[CornerSpec],
    start_time: Option<f64>,
) -> Result<CornerDistortionResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        let transient_points = transient(&corner_circuit, time_step, stop_time)?;
        let result = distortion_from_transient_with_start_time(
            &transient_points,
            fundamental_frequency_hz,
            input_source,
            output_probe,
            harmonics,
            start_time,
        )?;
        points.push(CornerDistortionPoint {
            corner_name: corner.name.clone(),
            result,
        });
    }
    Ok(CornerDistortionResult {
        input_source: input_source.to_string(),
        output_probe: output_probe.to_string(),
        points,
    })
}

pub fn fourier(
    points: &[TransientPoint],
    fundamental_frequency_hz: f64,
    probes: &[&str],
    harmonics: usize,
) -> Result<FourierResult, SpiceError> {
    fourier_with_start_time(points, fundamental_frequency_hz, probes, harmonics, None)
}

pub fn fourier_corners(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    fundamental_frequency_hz: f64,
    probes: &[&str],
    harmonics: usize,
    corners: &[CornerSpec],
) -> Result<CornerFourierResult, SpiceError> {
    fourier_corners_with_start_time(
        circuit,
        time_step,
        stop_time,
        fundamental_frequency_hz,
        probes,
        harmonics,
        corners,
        None,
    )
}

pub fn fourier_corners_with_start_time(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    fundamental_frequency_hz: f64,
    probes: &[&str],
    harmonics: usize,
    corners: &[CornerSpec],
    start_time: Option<f64>,
) -> Result<CornerFourierResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        let transient_points = transient(&corner_circuit, time_step, stop_time)?;
        points.push(CornerFourierPoint {
            corner_name: corner.name.clone(),
            result: fourier_with_start_time(
                &transient_points,
                fundamental_frequency_hz,
                probes,
                harmonics,
                start_time,
            )?,
        });
    }
    Ok(CornerFourierResult {
        fundamental_frequency_hz,
        points,
    })
}

pub fn fourier_with_start_time(
    points: &[TransientPoint],
    fundamental_frequency_hz: f64,
    probes: &[&str],
    harmonics: usize,
    start_time: Option<f64>,
) -> Result<FourierResult, SpiceError> {
    if !fundamental_frequency_hz.is_finite() || fundamental_frequency_hz <= 0.0 {
        return Err(fourier_error(
            "fundamental frequency must be finite and positive",
        ));
    }
    if harmonics < 1 {
        return Err(fourier_error("harmonics must be positive"));
    }
    if probes.is_empty() {
        return Err(fourier_error("at least one probe is required"));
    }
    if points.len() < 2 {
        return Err(fourier_error("at least two transient points are required"));
    }

    let mut sorted_points = points.to_vec();
    sorted_points.sort_by(|left, right| left.time.total_cmp(&right.time));
    let period = 1.0 / fundamental_frequency_hz;
    let end_time = sorted_points[sorted_points.len() - 1].time;
    let window_start = start_time.unwrap_or(end_time - period);
    if !window_start.is_finite() || window_start < sorted_points[0].time {
        return Err(fourier_error(
            "transient output does not contain a full analysis window",
        ));
    }
    if window_start >= end_time {
        return Err(fourier_error("analysis window must have positive duration"));
    }

    let mut probe_results = Vec::new();
    for probe in probes {
        probe_results.push(fourier_probe(
            &sorted_points,
            probe,
            fundamental_frequency_hz,
            harmonics,
            window_start,
            end_time,
        )?);
    }
    Ok(FourierResult {
        fundamental_frequency_hz,
        start_time: window_start,
        end_time,
        probes: probe_results,
    })
}

pub fn fourier_transient_cards(
    points: &[TransientPoint],
    fourier_cards: &[DeckFourierCard],
) -> Result<Vec<FourierResult>, SpiceError> {
    let mut results = Vec::with_capacity(fourier_cards.len());
    for card in fourier_cards {
        let probe_refs: Vec<&str> = card.probes.iter().map(String::as_str).collect();
        results.push(fourier_with_start_time(
            points,
            card.fundamental_frequency_hz,
            &probe_refs,
            card.harmonics.unwrap_or(9),
            card.from_value,
        )?);
    }
    Ok(results)
}

pub fn fourier_transient_deck(
    points: &[TransientPoint],
    netlist: &str,
) -> Result<Vec<FourierResult>, SpiceError> {
    let summary = resolve_deck_fourier(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "fourier_transient_deck",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    fourier_transient_cards(points, &summary.fourier)
}

fn fourier_probe(
    points: &[TransientPoint],
    probe: &str,
    fundamental_frequency_hz: f64,
    harmonics: usize,
    start_time: f64,
    end_time: f64,
) -> Result<FourierProbeResult, SpiceError> {
    let mut samples = vec![(start_time, interpolate_probe(points, probe, start_time)?)];
    for point in points {
        if start_time < point.time && point.time < end_time {
            samples.push((point.time, probe_value(point, probe)?));
        }
    }
    samples.push((end_time, interpolate_probe(points, probe, end_time)?));
    samples.sort_by(|left, right| left.0.total_cmp(&right.0));

    let duration = end_time - start_time;
    let dc = integrate_samples(&samples, |_| 1.0) / duration;
    let omega = 2.0 * std::f64::consts::PI * fundamental_frequency_hz;
    let mut components = Vec::new();
    for harmonic in 1..=harmonics {
        let n = harmonic as f64;
        let cosine = 2.0 / duration * integrate_samples(&samples, |time| (n * omega * time).cos());
        let sine = 2.0 / duration * integrate_samples(&samples, |time| (n * omega * time).sin());
        let magnitude = cosine.hypot(sine);
        components.push(FourierHarmonic {
            harmonic,
            frequency_hz: n * fundamental_frequency_hz,
            cosine,
            sine,
            magnitude,
            phase_degrees: cosine.atan2(sine).to_degrees(),
        });
    }
    let fundamental = components[0].magnitude;
    let distortion = components[1..]
        .iter()
        .map(|component| component.magnitude * component.magnitude)
        .sum::<f64>()
        .sqrt();
    let total_harmonic_distortion = if fundamental == 0.0 {
        if distortion > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        distortion / fundamental
    };
    Ok(FourierProbeResult {
        probe: probe.to_string(),
        dc,
        harmonics: components,
        total_harmonic_distortion,
    })
}

fn integrate_samples(samples: &[(f64, f64)], weight: impl Fn(f64) -> f64) -> f64 {
    samples
        .windows(2)
        .map(|window| {
            let (left_time, left_value) = window[0];
            let (right_time, right_value) = window[1];
            0.5 * (right_time - left_time)
                * (left_value * weight(left_time) + right_value * weight(right_time))
        })
        .sum()
}

fn interpolate_probe(points: &[TransientPoint], probe: &str, time: f64) -> Result<f64, SpiceError> {
    for point in points {
        if (point.time - time).abs() <= 1.0e-15 {
            return probe_value(point, probe);
        }
    }
    for window in points.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if left.time <= time && time <= right.time {
            let span = right.time - left.time;
            if span <= 0.0 {
                return probe_value(left, probe);
            }
            let alpha = (time - left.time) / span;
            return Ok(
                (1.0 - alpha) * probe_value(left, probe)? + alpha * probe_value(right, probe)?
            );
        }
    }
    Err(fourier_error("analysis window is outside transient output"))
}

fn probe_value(point: &TransientPoint, probe: &str) -> Result<f64, SpiceError> {
    let text = probe.trim();
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("v(") && text.ends_with(')') {
        let args: Vec<&str> = text[2..text.len() - 1]
            .split(',')
            .map(|arg| arg.trim())
            .collect();
        if args.len() == 1 {
            return point_voltage(point, args[0]);
        }
        if args.len() == 2 {
            return Ok(point_voltage(point, args[0])? - point_voltage(point, args[1])?);
        }
    }
    if lower.starts_with("i(") && text.ends_with(')') {
        let source_name = text[2..text.len() - 1].trim();
        return point
            .branch_current(source_name)
            .ok_or_else(|| fourier_error(&format!("missing branch current probe {probe}")));
    }
    if !text.is_empty() {
        return point_voltage(point, text);
    }
    Err(fourier_error("empty probe"))
}

fn point_voltage(point: &TransientPoint, node: &str) -> Result<f64, SpiceError> {
    point
        .voltage(node)
        .ok_or_else(|| fourier_error(&format!("missing node voltage {node}")))
}

fn fourier_error(reason: &str) -> SpiceError {
    SpiceError::InvalidElement {
        name: "fourier".to_string(),
        reason: reason.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssResidualEntry {
    pub kind: String,
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssResidualResult {
    pub period_seconds: f64,
    pub time_step_seconds: f64,
    pub node_residuals: BTreeMap<String, f64>,
    pub branch_residuals: BTreeMap<String, f64>,
    pub residual_vector: Vec<PssResidualEntry>,
    pub max_abs_branch_residual: f64,
    pub max_abs_residual: f64,
    pub residual_l2_norm: f64,
    pub residual_rms_norm: f64,
    pub residual_tolerance: f64,
    pub within_tolerance: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssStateEntry {
    pub kind: String,
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssResidualJacobianColumn {
    pub state: PssStateEntry,
    pub residual_derivatives: Vec<PssResidualEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssResidualJacobianResult {
    pub residual: PssResidualResult,
    pub state_vector: Vec<PssStateEntry>,
    pub perturbation: f64,
    pub columns: Vec<PssResidualJacobianColumn>,
    pub jacobian: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssNewtonUpdateResult {
    pub jacobian: PssResidualJacobianResult,
    pub state_updates: Vec<PssStateEntry>,
    pub next_state_vector: Vec<PssStateEntry>,
    pub update_l2_norm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssNewtonCandidateResult {
    pub update: PssNewtonUpdateResult,
    pub candidate_circuit: Circuit,
    pub candidate_state_vector: Vec<PssStateEntry>,
    pub candidate_residual: PssResidualResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssNewtonIterationResult {
    pub candidate: PssNewtonCandidateResult,
    pub accepted: bool,
    pub residual_l2_reduction: f64,
    pub residual_l2_ratio: f64,
    pub next_circuit: Circuit,
    pub next_state_vector: Vec<PssStateEntry>,
    pub next_residual: PssResidualResult,
    pub converged: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssNewtonSolveResult {
    pub iterations: Vec<PssNewtonIterationResult>,
    pub final_circuit: Circuit,
    pub final_state_vector: Vec<PssStateEntry>,
    pub final_residual: PssResidualResult,
    pub converged: bool,
    pub iteration_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PssResult {
    pub solve: PssNewtonSolveResult,
    pub steady_state: Vec<TransientPoint>,
    pub period_seconds: f64,
    pub time_step_seconds: f64,
    pub converged: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerPssPoint {
    pub corner_name: String,
    pub result: PssResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CornerPssResult {
    pub points: Vec<CornerPssPoint>,
}

impl DcResult {
    pub fn voltage(&self, node: &str) -> Option<f64> {
        if is_ground(node) {
            Some(0.0)
        } else {
            self.node_voltages.get(node).copied()
        }
    }

    pub fn branch_current(&self, source_name: &str) -> Option<f64> {
        let key = if source_name.starts_with("I(") {
            source_name.to_string()
        } else {
            format!("I({source_name})")
        };
        self.branch_currents.get(&key).copied()
    }
}

pub fn format_dc_table(result: &DcResult, probes: &[&str]) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        default_output_probes(&result.node_voltages, &result.branch_currents)
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let values: Result<Vec<String>, SpiceError> = selected_probes
        .iter()
        .map(|probe| {
            table_probe_value(
                &result.node_voltages,
                &result.branch_currents,
                probe,
                "format_dc_table",
            )
            .map(format_table_number)
        })
        .collect();
    Ok(format!(
        "Index\t{}\n0\t{}\n",
        selected_probes.join("\t"),
        values?.join("\t")
    ))
}

pub fn format_corner_dc_table(
    result: &CornerSweepResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .first()
            .map(|point| {
                default_output_probes(&point.result.node_voltages, &point.result.branch_currents)
            })
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!("Corner\tIndex\t{}", selected_probes.join("\t"))];
    for corner in &result.points {
        let values: Result<Vec<String>, SpiceError> = selected_probes
            .iter()
            .map(|probe| {
                table_probe_value(
                    &corner.result.node_voltages,
                    &corner.result.branch_currents,
                    probe,
                    "format_corner_dc_table",
                )
                .map(format_table_number)
            })
            .collect();
        rows.push(format!("{}\t0\t{}", corner.corner_name, values?.join("\t")));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_temperature_dc_table(
    result: &TemperatureDcResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .first()
            .map(|point| {
                default_output_probes(&point.result.node_voltages, &point.result.branch_currents)
            })
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Index\tTemperatureKelvin\t{}",
        selected_probes.join("\t")
    )];
    for (index, point) in result.points.iter().enumerate() {
        let values: Result<Vec<String>, SpiceError> = selected_probes
            .iter()
            .map(|probe| {
                table_probe_value(
                    &point.result.node_voltages,
                    &point.result.branch_currents,
                    probe,
                    "format_temperature_dc_table",
                )
                .map(format_table_number)
            })
            .collect();
        rows.push(format!(
            "{index}\t{}\t{}",
            format_table_number(point.temperature_kelvin),
            values?.join("\t")
        ));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_corner_temperature_dc_table(
    result: &CornerTemperatureDcResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .iter()
            .find_map(|corner| corner.points.first())
            .map(|point| {
                default_output_probes(&point.result.node_voltages, &point.result.branch_currents)
            })
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Corner\tIndex\tTemperatureKelvin\t{}",
        selected_probes.join("\t")
    )];
    for corner in &result.points {
        for (index, point) in corner.points.iter().enumerate() {
            let values: Result<Vec<String>, SpiceError> = selected_probes
                .iter()
                .map(|probe| {
                    table_probe_value(
                        &point.result.node_voltages,
                        &point.result.branch_currents,
                        probe,
                        "format_corner_temperature_dc_table",
                    )
                    .map(format_table_number)
                })
                .collect();
            rows.push(format!(
                "{}\t{index}\t{}\t{}",
                corner.corner_name,
                format_table_number(point.temperature_kelvin),
                values?.join("\t")
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_dc_sweep_table(
    source_name: &str,
    points: &[DcSweepPoint],
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        points
            .first()
            .map(|point| {
                default_output_probes(&point.result.node_voltages, &point.result.branch_currents)
            })
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Index\tSource\tValue\t{}",
        selected_probes.join("\t")
    )];
    for (index, point) in points.iter().enumerate() {
        let values: Result<Vec<String>, SpiceError> = selected_probes
            .iter()
            .map(|probe| {
                table_probe_value(
                    &point.result.node_voltages,
                    &point.result.branch_currents,
                    probe,
                    "format_dc_sweep_table",
                )
                .map(format_table_number)
            })
            .collect();
        rows.push(format!(
            "{index}\t{}\t{}\t{}",
            source_name,
            format_table_number(point.value),
            values?.join("\t")
        ));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_corner_dc_sweep_table(
    result: &CornerDcSweepResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .first()
            .and_then(|corner| corner.points.first())
            .map(|point| {
                default_output_probes(&point.result.node_voltages, &point.result.branch_currents)
            })
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Corner\tIndex\tSource\tValue\t{}",
        selected_probes.join("\t")
    )];
    for corner in &result.points {
        for (index, point) in corner.points.iter().enumerate() {
            let values: Result<Vec<String>, SpiceError> = selected_probes
                .iter()
                .map(|probe| {
                    table_probe_value(
                        &point.result.node_voltages,
                        &point.result.branch_currents,
                        probe,
                        "format_corner_dc_sweep_table",
                    )
                    .map(format_table_number)
                })
                .collect();
            rows.push(format!(
                "{}\t{index}\t{}\t{}\t{}",
                corner.corner_name,
                result.source_name,
                format_table_number(point.value),
                values?.join("\t")
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_transient_table(
    points: &[TransientPoint],
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        default_transient_output_probes(points)
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!("Index\tTime\t{}", selected_probes.join("\t"))];
    for (index, point) in points.iter().enumerate() {
        let values: Result<Vec<String>, SpiceError> = selected_probes
            .iter()
            .map(|probe| {
                table_probe_value(
                    &point.node_voltages,
                    &point.branch_currents,
                    probe,
                    "format_transient_table",
                )
                .map(format_table_number)
            })
            .collect();
        rows.push(format!(
            "{index}\t{}\t{}",
            format_table_number(point.time),
            values?.join("\t")
        ));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

fn format_transient_method(method: TransientMethod) -> &'static str {
    match method {
        TransientMethod::Euler => "euler",
        TransientMethod::Trap => "trap",
        TransientMethod::Gear2 => "gear2",
    }
}

pub fn format_adaptive_transient_table(
    result: &AdaptiveTransientResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        default_transient_output_probes(&result.points)
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Method\tStepsRejected\tConverged\tIndex\tTime\t{}",
        selected_probes.join("\t")
    )];
    for (index, point) in result.points.iter().enumerate() {
        let values: Result<Vec<String>, SpiceError> = selected_probes
            .iter()
            .map(|probe| {
                table_probe_value(
                    &point.node_voltages,
                    &point.branch_currents,
                    probe,
                    "format_adaptive_transient_table",
                )
                .map(format_table_number)
            })
            .collect();
        rows.push(format!(
            "{}\t{}\t{}\t{index}\t{}\t{}",
            format_transient_method(result.method),
            result.steps_rejected,
            result.converged,
            format_table_number(point.time),
            values?.join("\t")
        ));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_corner_transient_table(
    result: &CornerTransientResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .iter()
            .find(|point| !point.points.is_empty())
            .map(|point| default_transient_output_probes(&point.points))
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Corner\tIndex\tTime\t{}",
        selected_probes.join("\t")
    )];
    for corner in &result.points {
        for (index, point) in corner.points.iter().enumerate() {
            let values: Result<Vec<String>, SpiceError> = selected_probes
                .iter()
                .map(|probe| {
                    table_probe_value(
                        &point.node_voltages,
                        &point.branch_currents,
                        probe,
                        "format_corner_transient_table",
                    )
                    .map(format_table_number)
                })
                .collect();
            rows.push(format!(
                "{}\t{index}\t{}\t{}",
                corner.corner_name,
                format_table_number(point.time),
                values?.join("\t")
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_corner_adaptive_transient_table(
    result: &CornerAdaptiveTransientResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .iter()
            .find(|point| !point.result.points.is_empty())
            .map(|point| default_transient_output_probes(&point.result.points))
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Corner\tMethod\tStepsRejected\tConverged\tIndex\tTime\t{}",
        selected_probes.join("\t")
    )];
    for corner in &result.points {
        for (index, point) in corner.result.points.iter().enumerate() {
            let values: Result<Vec<String>, SpiceError> = selected_probes
                .iter()
                .map(|probe| {
                    table_probe_value(
                        &point.node_voltages,
                        &point.branch_currents,
                        probe,
                        "format_corner_adaptive_transient_table",
                    )
                    .map(format_table_number)
                })
                .collect();
            rows.push(format!(
                "{}\t{}\t{}\t{}\t{index}\t{}\t{}",
                corner.corner_name,
                format_transient_method(corner.result.method),
                corner.result.steps_rejected,
                corner.result.converged,
                format_table_number(point.time),
                values?.join("\t")
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_mc_table(result: &McResult) -> String {
    let mut rows = vec!["Trial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged".to_string()];
    for point in &result.points {
        let output_value = if point.converged {
            point
                .voltage(&result.output_node)
                .map(format_table_number)
                .unwrap_or_default()
        } else {
            String::new()
        };
        rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            point.trial,
            result.output_node,
            output_value,
            format_table_number(result.mean),
            format_table_number(result.std_dev),
            point.converged
        ));
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_corner_mc_table(result: &CornerMcResult) -> String {
    let mut rows =
        vec!["Corner\tTrial\tOutputNode\tOutputValue\tMean\tStdDev\tConverged".to_string()];
    for corner in &result.points {
        for point in &corner.result.points {
            let output_value = if point.converged {
                point
                    .voltage(&result.output_node)
                    .map(format_table_number)
                    .unwrap_or_default()
            } else {
                String::new()
            };
            rows.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                corner.corner_name,
                point.trial,
                result.output_node,
                output_value,
                format_table_number(corner.result.mean),
                format_table_number(corner.result.std_dev),
                point.converged
            ));
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_ac_table(points: &[AcPoint], probes: &[&str]) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        default_ac_output_probes(points)
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec!["Index\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase".to_string()];
    for (index, point) in points.iter().enumerate() {
        for probe in &selected_probes {
            let value = table_complex_probe_value(
                &point.node_voltages,
                &point.branch_currents,
                probe,
                "format_ac_table",
            )?;
            rows.push(format!(
                "{index}\t{}\t{}\t{}\t{}\t{}\t{}",
                format_table_number(point.frequency_hz),
                probe,
                format_table_number(value.real),
                format_table_number(value.imag),
                format_table_number(value.abs()),
                format_table_number(value.phase().to_degrees())
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_deck_op_table(result: &DcResult, netlist: &str) -> Result<String, SpiceError> {
    let probes = select_deck_output_probes(netlist, "op")?;
    let probe_refs = probes.iter().map(String::as_str).collect::<Vec<_>>();
    format_dc_table(result, &probe_refs)
}

pub fn format_deck_dc_sweep_table(
    source_name: &str,
    points: &[DcSweepPoint],
    netlist: &str,
) -> Result<String, SpiceError> {
    let probes = select_deck_output_probes(netlist, "dc")?;
    let probe_refs = probes.iter().map(String::as_str).collect::<Vec<_>>();
    format_dc_sweep_table(source_name, points, &probe_refs)
}

pub fn format_deck_ac_table(points: &[AcPoint], netlist: &str) -> Result<String, SpiceError> {
    let probes = select_deck_output_probes(netlist, "ac")?;
    let probe_refs = probes.iter().map(String::as_str).collect::<Vec<_>>();
    format_ac_table(points, &probe_refs)
}

pub fn format_deck_transient_table(
    points: &[TransientPoint],
    netlist: &str,
) -> Result<String, SpiceError> {
    let probes = select_deck_output_probes(netlist, "tran")?;
    let probe_refs = probes.iter().map(String::as_str).collect::<Vec<_>>();
    format_transient_table(points, &probe_refs)
}

pub fn format_deck_tf_table(result: &TfResult) -> String {
    format_tf_table(result)
}

pub fn format_deck_sens_table(result: &SensResult) -> String {
    format_sens_table(result)
}

pub fn format_deck_noise_table(result: &NoiseResult) -> String {
    format_noise_table(result)
}

fn deck_run_artifacts(
    plan: &DeckAnalysisPlan,
    result_rows: usize,
    result_columns: &[String],
    output_probes: &[String],
    output_directives: &[String],
    measurements: &[ProbeMeasurement],
    fourier: &[FourierResult],
    control_lines: &[String],
    write_markers: &[String],
    rawfile_options: &[String],
    diagnostic_codes: &[String],
    control_policy_artifacts: &[DeckControlPolicyArtifact],
    deck_analysis_kinds: &[String],
    deck_analysis_directive_inventory: &[String],
) -> Vec<DeckRunArtifact> {
    let is_transient = plan.analysis == "tran";
    let analysis_directives = deck_analysis_directives(plan);
    let tables = deck_stable_tables(measurements, fourier, control_policy_artifacts);
    let control_policy_summaries = deck_control_policy_summary_artifacts(control_policy_artifacts);
    let control_policy_categories = control_policy_summaries
        .iter()
        .map(|artifact| artifact.category.clone())
        .collect::<Vec<_>>();
    let control_policy_codes = control_policy_summaries
        .iter()
        .flat_map(|artifact| artifact.codes.iter().cloned())
        .collect::<Vec<_>>();
    let mut control_policy_severities = Vec::new();
    for artifact in &control_policy_summaries {
        for severity in &artifact.severities {
            push_unique_string(&mut control_policy_severities, severity);
        }
    }
    vec![DeckRunArtifact {
        analysis: plan.analysis.clone(),
        directive: plan.directive.clone(),
        analysis_directive_count: analysis_directives.len(),
        analysis_directives,
        deck_analysis_kind_count: deck_analysis_kinds.len(),
        deck_analysis_kinds: deck_analysis_kinds.to_vec(),
        deck_analysis_directive_count: deck_analysis_directive_inventory.len(),
        deck_analysis_directives: deck_analysis_directive_inventory.to_vec(),
        line_number: plan.line_number,
        source_name: plan.source_name.clone(),
        output_node: plan.output_node.clone(),
        sweep_kind: plan.sweep_kind.clone(),
        start_value: plan.start_value,
        stop_value: plan.stop_value,
        step_value: plan.step_value,
        point_count: plan.point_count,
        start_frequency_hz: plan.start_frequency_hz,
        stop_frequency_hz: plan.stop_frequency_hz,
        step_time: is_transient.then_some(plan.step_time).flatten(),
        stop_time: is_transient.then_some(plan.stop_time).flatten(),
        start_time: is_transient.then_some(plan.start_time).flatten(),
        max_step: is_transient.then_some(plan.max_step).flatten(),
        use_initial_conditions: is_transient.then_some(plan.use_initial_conditions),
        result_rows,
        result_column_count: result_columns.len(),
        result_columns: result_columns.to_vec(),
        table_count: tables.len(),
        tables,
        output_probe_count: output_probes.len(),
        output_probes: output_probes.to_vec(),
        output_directive_count: output_directives.len(),
        output_directives: output_directives.to_vec(),
        measurement_count: measurements.len(),
        measurement_names: measurements
            .iter()
            .map(|measurement| measurement.name.clone())
            .collect(),
        fourier_count: fourier.len(),
        fourier_probes: fourier
            .iter()
            .flat_map(|result| result.probes.iter().map(|probe| probe.probe.clone()))
            .collect(),
        control_line_count: control_lines.len(),
        control_lines: control_lines.to_vec(),
        write_marker_count: write_markers.len(),
        write_markers: write_markers.to_vec(),
        rawfile_option_count: rawfile_options.len(),
        rawfile_options: rawfile_options.to_vec(),
        control_policy_artifact_count: control_policy_artifacts.len(),
        control_policy_categories,
        control_policy_codes,
        control_policy_severities,
        diagnostic_count: diagnostic_codes.len(),
        diagnostic_codes: diagnostic_codes.to_vec(),
    }]
}

fn deck_output_plan_artifacts(
    plan: &DeckAnalysisPlan,
    result_row_count: usize,
    result_columns: &[String],
    output_probes: &[String],
    output_probe_lines: &[usize],
    output_directives: &[String],
    output_directive_analysis_kinds: &[String],
    output_directive_lines: &[usize],
    tables: &[String],
) -> Vec<DeckOutputPlanArtifact> {
    let output_directive_kinds = deck_output_directive_kinds(output_directives);
    let is_transient = plan.analysis == "tran";
    vec![DeckOutputPlanArtifact {
        analysis: plan.analysis.clone(),
        directive: plan.directive.clone(),
        line_number: plan.line_number,
        source_name: plan.source_name.clone(),
        output_node: plan.output_node.clone(),
        sweep_kind: plan.sweep_kind.clone(),
        start_value: plan.start_value,
        stop_value: plan.stop_value,
        step_value: plan.step_value,
        point_count: plan.point_count,
        start_frequency_hz: plan.start_frequency_hz,
        stop_frequency_hz: plan.stop_frequency_hz,
        step_time: is_transient.then_some(plan.step_time).flatten(),
        stop_time: is_transient.then_some(plan.stop_time).flatten(),
        start_time: is_transient.then_some(plan.start_time).flatten(),
        max_step: is_transient.then_some(plan.max_step).flatten(),
        use_initial_conditions: is_transient.then_some(plan.use_initial_conditions),
        result_row_count,
        result_column_count: result_columns.len(),
        result_columns: result_columns.to_vec(),
        output_probe_count: output_probes.len(),
        output_probes: output_probes.to_vec(),
        output_probe_line_count: output_probe_lines.len(),
        output_probe_lines: output_probe_lines.to_vec(),
        output_directive_count: output_directives.len(),
        output_directives: output_directives.to_vec(),
        output_directive_kind_count: output_directive_kinds.len(),
        output_directive_kinds,
        output_directive_analysis_kind_count: output_directive_analysis_kinds.len(),
        output_directive_analysis_kinds: output_directive_analysis_kinds.to_vec(),
        output_directive_line_count: output_directive_lines.len(),
        output_directive_lines: output_directive_lines.to_vec(),
        table_count: tables.len(),
        tables: tables.to_vec(),
    }]
}

fn deck_output_directive_kind(directive: &str) -> String {
    let token = directive
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    token.strip_prefix('.').unwrap_or(&token).to_string()
}

fn deck_output_directive_kinds(output_directives: &[String]) -> Vec<String> {
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for directive in output_directives {
        let kind = deck_output_directive_kind(directive);
        if kind.is_empty() || !seen.insert(kind.clone()) {
            continue;
        }
        selected.push(kind);
    }
    selected
}

const DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS: &[&str] = &[
    "Analysis",
    "Directive",
    "Line",
    "SourceName",
    "OutputNode",
    "SweepKind",
    "StartValue",
    "StopValue",
    "StepValue",
    "PointCount",
    "StartFrequencyHz",
    "StopFrequencyHz",
    "StepTime",
    "StopTime",
    "StartTime",
    "MaxStep",
    "UseInitialConditions",
    "ResultRows",
    "ResultColumns",
    "ResultColumnList",
    "OutputProbes",
    "OutputProbeList",
    "OutputProbeLines",
    "OutputProbeLineList",
    "OutputDirectives",
    "OutputDirectiveList",
    "OutputDirectiveKinds",
    "OutputDirectiveKindList",
    "OutputDirectiveAnalysisKinds",
    "OutputDirectiveAnalysisKindList",
    "OutputDirectiveLines",
    "OutputDirectiveLineList",
    "Tables",
    "TableList",
];

fn deck_output_plan_artifact_cells(artifact: &DeckOutputPlanArtifact) -> Vec<String> {
    vec![
        artifact.analysis.clone(),
        artifact.directive.clone(),
        artifact.line_number.to_string(),
        artifact.source_name.clone().unwrap_or_default(),
        artifact.output_node.clone().unwrap_or_default(),
        artifact.sweep_kind.clone().unwrap_or_default(),
        format_deck_artifact_float(artifact.start_value),
        format_deck_artifact_float(artifact.stop_value),
        format_deck_artifact_float(artifact.step_value),
        artifact
            .point_count
            .map(|point_count| point_count.to_string())
            .unwrap_or_default(),
        format_deck_artifact_float(artifact.start_frequency_hz),
        format_deck_artifact_float(artifact.stop_frequency_hz),
        format_deck_artifact_float(artifact.step_time),
        format_deck_artifact_float(artifact.stop_time),
        format_deck_artifact_float(artifact.start_time),
        format_deck_artifact_float(artifact.max_step),
        format_deck_artifact_bool(artifact.use_initial_conditions),
        artifact.result_row_count.to_string(),
        artifact.result_column_count.to_string(),
        artifact.result_columns.join(";"),
        artifact.output_probe_count.to_string(),
        artifact.output_probes.join(";"),
        artifact.output_probe_line_count.to_string(),
        artifact
            .output_probe_lines
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(";"),
        artifact.output_directive_count.to_string(),
        artifact.output_directives.join(";"),
        artifact.output_directive_kind_count.to_string(),
        artifact.output_directive_kinds.join(";"),
        artifact.output_directive_analysis_kind_count.to_string(),
        artifact.output_directive_analysis_kinds.join(";"),
        artifact.output_directive_line_count.to_string(),
        artifact
            .output_directive_lines
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(";"),
        artifact.table_count.to_string(),
        artifact.tables.join(";"),
    ]
}

pub fn deck_output_plan_artifact_records(
    artifacts: &[DeckOutputPlanArtifact],
) -> Vec<BTreeMap<String, String>> {
    artifacts
        .iter()
        .map(|artifact| {
            DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS
                .iter()
                .copied()
                .zip(deck_output_plan_artifact_cells(artifact))
                .map(|(key, value)| (key.to_string(), value))
                .collect()
        })
        .collect()
}

pub fn format_deck_output_plan_artifact_table(artifacts: &[DeckOutputPlanArtifact]) -> String {
    let mut rows = vec![DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS.join("\t")];
    for artifact in artifacts {
        rows.push(deck_output_plan_artifact_cells(artifact).join("\t"));
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_output_plan_artifact_csv(artifacts: &[DeckOutputPlanArtifact]) -> String {
    let mut rows = vec![DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS.join(",")];
    for artifact in artifacts {
        rows.push(
            deck_output_plan_artifact_cells(artifact)
                .iter()
                .map(|cell| format_csv_cell(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_output_plan_artifact_json(artifacts: &[DeckOutputPlanArtifact]) -> String {
    let records = deck_output_plan_artifact_records(artifacts)
        .into_iter()
        .map(|record| {
            let fields = record
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        format_json_string(&key),
                        format_json_string(&value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", fields)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]\n", records)
}

fn deck_output_plan_artifact_bundle(
    plan: &DeckAnalysisPlan,
    result_table: &str,
    output_probes: &[String],
    output_probe_lines: &[usize],
    output_directives: &[String],
    output_directive_analysis_kinds: &[String],
    output_directive_lines: &[usize],
    tables: &[String],
) -> (
    Vec<DeckOutputPlanArtifact>,
    String,
    String,
    String,
    Vec<BTreeMap<String, String>>,
) {
    let artifacts = deck_output_plan_artifacts(
        plan,
        deck_table_row_count(result_table),
        &deck_table_columns(result_table),
        output_probes,
        output_probe_lines,
        output_directives,
        output_directive_analysis_kinds,
        output_directive_lines,
        tables,
    );
    let table = format_deck_output_plan_artifact_table(&artifacts);
    let csv = format_deck_output_plan_artifact_csv(&artifacts);
    let json = format_deck_output_plan_artifact_json(&artifacts);
    let records = deck_output_plan_artifact_records(&artifacts);
    (artifacts, table, csv, json, records)
}

fn deck_analysis_directives(plan: &DeckAnalysisPlan) -> Vec<String> {
    if plan.directive.is_empty() {
        Vec::new()
    } else {
        vec![plan.directive.clone()]
    }
}

fn deck_analysis_inventory(netlist: &str) -> (Vec<String>, Vec<String>) {
    let summary = resolve_deck_analyses(netlist);
    let mut analysis_kinds = Vec::new();
    let mut directives = Vec::new();
    for plan in summary.analyses {
        if !plan.analysis.is_empty() {
            push_unique_string(&mut analysis_kinds, &plan.analysis);
        }
        if !plan.directive.is_empty() {
            directives.push(plan.directive);
        }
    }
    (analysis_kinds, directives)
}

fn deck_stable_tables(
    measurements: &[ProbeMeasurement],
    fourier: &[FourierResult],
    control_policy_artifacts: &[DeckControlPolicyArtifact],
) -> Vec<String> {
    let mut tables = vec!["result".to_string()];
    if !measurements.is_empty() {
        tables.push("measurement".to_string());
    }
    if !fourier.is_empty() {
        tables.push("fourier".to_string());
    }
    if !control_policy_artifacts.is_empty() {
        tables.push("control-policy".to_string());
        tables.push("control-policy-summary".to_string());
    }
    tables.push("output-plan".to_string());
    tables.push("run-artifact".to_string());
    tables
}

fn deck_analysis_diagnostic_codes(netlist: &str, plan: &DeckAnalysisPlan) -> Vec<String> {
    resolve_deck_analyses(netlist)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.line_number == plan.line_number && diagnostic.directive == plan.directive
        })
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn deck_control_diagnostic_codes(netlist: &str) -> Vec<String> {
    analyze_deck_controls(netlist)
        .diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.code.starts_with("SPICE_DECK_CONTROL_"))
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn deck_control_lines(netlist: &str) -> Vec<String> {
    analyze_deck_controls(netlist).control_lines
}

fn deck_control_write_markers(netlist: &str) -> Vec<String> {
    analyze_deck_controls(netlist).write_markers
}

fn deck_control_rawfile_options(netlist: &str) -> Vec<String> {
    analyze_deck_controls(netlist).rawfile_options
}

fn deck_run_diagnostic_codes(netlist: &str, plan: &DeckAnalysisPlan) -> Vec<String> {
    let mut codes = deck_analysis_diagnostic_codes(netlist, plan);
    codes.extend(deck_control_diagnostic_codes(netlist));
    codes
}

fn format_deck_artifact_float(value: Option<f64>) -> String {
    value.map(format_table_number).unwrap_or_default()
}

fn format_deck_artifact_bool(value: Option<bool>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

const DECK_RUN_ARTIFACT_COLUMNS: &[&str] = &[
    "Analysis",
    "Directive",
    "AnalysisDirectives",
    "AnalysisDirectiveList",
    "Line",
    "SourceName",
    "OutputNode",
    "SweepKind",
    "StartValue",
    "StopValue",
    "StepValue",
    "PointCount",
    "StartFrequencyHz",
    "StopFrequencyHz",
    "StepTime",
    "StopTime",
    "StartTime",
    "MaxStep",
    "UseInitialConditions",
    "ResultRows",
    "ResultColumns",
    "ResultColumnList",
    "Tables",
    "TableList",
    "OutputProbes",
    "OutputProbeList",
    "OutputDirectives",
    "OutputDirectiveList",
    "Measurements",
    "MeasurementList",
    "Fourier",
    "FourierList",
    "ControlLines",
    "ControlLineList",
    "WriteMarkers",
    "WriteMarkerList",
    "RawfileOptions",
    "RawfileOptionList",
    "ControlPolicyArtifacts",
    "ControlPolicyCategoryList",
    "ControlPolicyCodeList",
    "ControlPolicySeverityList",
    "Diagnostics",
    "DiagnosticCodeList",
    "DeckAnalysisKinds",
    "DeckAnalysisKindList",
    "DeckAnalysisDirectives",
    "DeckAnalysisDirectiveList",
];

fn deck_run_artifact_cells(artifact: &DeckRunArtifact) -> Vec<String> {
    vec![
        artifact.analysis.clone(),
        artifact.directive.clone(),
        artifact.analysis_directive_count.to_string(),
        artifact.analysis_directives.join(";"),
        artifact.line_number.to_string(),
        artifact.source_name.clone().unwrap_or_default(),
        artifact.output_node.clone().unwrap_or_default(),
        artifact.sweep_kind.clone().unwrap_or_default(),
        format_deck_artifact_float(artifact.start_value),
        format_deck_artifact_float(artifact.stop_value),
        format_deck_artifact_float(artifact.step_value),
        artifact
            .point_count
            .map(|value| value.to_string())
            .unwrap_or_default(),
        format_deck_artifact_float(artifact.start_frequency_hz),
        format_deck_artifact_float(artifact.stop_frequency_hz),
        format_deck_artifact_float(artifact.step_time),
        format_deck_artifact_float(artifact.stop_time),
        format_deck_artifact_float(artifact.start_time),
        format_deck_artifact_float(artifact.max_step),
        format_deck_artifact_bool(artifact.use_initial_conditions),
        artifact.result_rows.to_string(),
        artifact.result_column_count.to_string(),
        artifact.result_columns.join(";"),
        artifact.table_count.to_string(),
        artifact.tables.join(";"),
        artifact.output_probe_count.to_string(),
        artifact.output_probes.join(";"),
        artifact.output_directive_count.to_string(),
        artifact.output_directives.join(";"),
        artifact.measurement_count.to_string(),
        artifact.measurement_names.join(";"),
        artifact.fourier_count.to_string(),
        artifact.fourier_probes.join(";"),
        artifact.control_line_count.to_string(),
        artifact.control_lines.join(";"),
        artifact.write_marker_count.to_string(),
        artifact.write_markers.join(";"),
        artifact.rawfile_option_count.to_string(),
        artifact.rawfile_options.join(";"),
        artifact.control_policy_artifact_count.to_string(),
        artifact.control_policy_categories.join(";"),
        artifact.control_policy_codes.join(";"),
        artifact.control_policy_severities.join(";"),
        artifact.diagnostic_count.to_string(),
        artifact.diagnostic_codes.join(";"),
        artifact.deck_analysis_kind_count.to_string(),
        artifact.deck_analysis_kinds.join(";"),
        artifact.deck_analysis_directive_count.to_string(),
        artifact.deck_analysis_directives.join(";"),
    ]
}

fn deck_run_artifact_record(artifact: &DeckRunArtifact) -> Vec<(&'static str, String)> {
    DECK_RUN_ARTIFACT_COLUMNS
        .iter()
        .copied()
        .zip(deck_run_artifact_cells(artifact))
        .collect()
}

pub fn deck_run_artifact_records(artifacts: &[DeckRunArtifact]) -> Vec<BTreeMap<String, String>> {
    artifacts
        .iter()
        .map(|artifact| {
            deck_run_artifact_record(artifact)
                .into_iter()
                .map(|(column, value)| (column.to_string(), value))
                .collect()
        })
        .collect()
}

fn deck_table_columns(table: &str) -> Vec<String> {
    table
        .lines()
        .next()
        .map(|header| {
            header
                .split('\t')
                .map(|column| column.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn deck_table_row_count(table: &str) -> usize {
    let mut lines = table.lines();
    if lines.next().is_none() {
        0
    } else {
        lines.count()
    }
}

pub fn format_deck_run_artifact_table(artifacts: &[DeckRunArtifact]) -> String {
    let mut rows = vec![DECK_RUN_ARTIFACT_COLUMNS.join("\t")];
    for artifact in artifacts {
        rows.push(deck_run_artifact_cells(artifact).join("\t"));
    }
    format!("{}\n", rows.join("\n"))
}

fn format_csv_cell(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn format_deck_table_csv(table: &str) -> String {
    let rows = table
        .lines()
        .map(|row| {
            row.split('\t')
                .map(format_csv_cell)
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        String::new()
    } else {
        format!("{}\n", rows.join("\n"))
    }
}

pub fn deck_table_records(table: &str) -> Vec<BTreeMap<String, String>> {
    let mut lines = table.lines();
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let columns = header.split('\t').collect::<Vec<_>>();
    lines
        .map(|row| {
            let cells = row.split('\t').collect::<Vec<_>>();
            columns
                .iter()
                .enumerate()
                .map(|(index, column)| {
                    (
                        (*column).to_string(),
                        cells.get(index).copied().unwrap_or("").to_string(),
                    )
                })
                .collect()
        })
        .collect()
}

fn deck_table_artifact(name: &str, table: &str) -> DeckTableArtifact {
    DeckTableArtifact {
        name: name.to_string(),
        table: table.to_string(),
        csv: format_deck_table_csv(table),
        json: format_deck_table_json(table),
        records: deck_table_records(table),
    }
}

fn deck_table_artifacts(
    plan: &DeckAnalysisPlan,
    result_table: &str,
    measurement_table: &str,
    fourier_table: &str,
    run_artifact_table: &str,
    measurements: &[ProbeMeasurement],
    fourier: &[FourierResult],
    control_policy_artifacts: &[DeckControlPolicyArtifact],
    control_policy_artifact_table: &str,
    control_policy_summary_artifacts: &[DeckControlPolicySummaryArtifact],
    control_policy_summary_artifact_table: &str,
    output_probes: &[String],
    output_probe_lines: &[usize],
    output_directives: &[String],
    output_directive_analysis_kinds: &[String],
    output_directive_lines: &[usize],
    tables: &[String],
) -> Vec<DeckTableArtifact> {
    let mut artifacts = vec![deck_table_artifact("result", result_table)];
    if !measurements.is_empty() {
        artifacts.push(deck_table_artifact("measurement", measurement_table));
    }
    if !fourier.is_empty() {
        artifacts.push(deck_table_artifact("fourier", fourier_table));
    }
    if !control_policy_artifacts.is_empty() {
        artifacts.push(deck_table_artifact(
            "control-policy",
            control_policy_artifact_table,
        ));
    }
    if !control_policy_summary_artifacts.is_empty() {
        artifacts.push(deck_table_artifact(
            "control-policy-summary",
            control_policy_summary_artifact_table,
        ));
    }
    let (
        _,
        output_plan_artifact_table,
        output_plan_artifact_csv,
        output_plan_artifact_json,
        output_plan_artifact_records,
    ) = deck_output_plan_artifact_bundle(
        plan,
        result_table,
        output_probes,
        output_probe_lines,
        output_directives,
        output_directive_analysis_kinds,
        output_directive_lines,
        tables,
    );
    artifacts.push(DeckTableArtifact {
        name: "output-plan".to_string(),
        table: output_plan_artifact_table,
        csv: output_plan_artifact_csv,
        json: output_plan_artifact_json,
        records: output_plan_artifact_records,
    });
    artifacts.push(deck_table_artifact("run-artifact", run_artifact_table));
    artifacts
}

pub fn format_deck_rawfile_ascii(
    table: &str,
    analysis: &str,
    rawfile_options: &[String],
) -> String {
    format_deck_rawfile_ascii_with_probes(table, analysis, rawfile_options, &[])
}

fn format_deck_rawfile_ascii_with_probes(
    table: &str,
    analysis: &str,
    rawfile_options: &[String],
    probes: &[String],
) -> String {
    let rows = table.lines().collect::<Vec<_>>();
    if rows.is_empty() {
        return String::new();
    }
    let projected_rows = deck_rawfile_project_rows(&rows, probes);
    let columns = projected_rows[0].split('\t').collect::<Vec<_>>();
    let data_rows = projected_rows
        .iter()
        .skip(1)
        .map(|row| row.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut lines = vec![
        format!("Title: SPICE deck {analysis} result"),
        "Date: deterministic".to_string(),
        format!("Plotname: {analysis}"),
        "Flags: real".to_string(),
        format!("No. Variables: {}", columns.len()),
        format!("No. Points: {}", data_rows.len()),
        format!("Options: {}", rawfile_options.join(";")),
        "Variables:".to_string(),
    ];
    for (index, column) in columns.iter().enumerate() {
        lines.push(format!("\t{index}\t{column}\treal"));
    }
    lines.push("Values:".to_string());
    for (index, row) in data_rows.iter().enumerate() {
        let mut padded = row
            .iter()
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        padded.resize(columns.len(), String::new());
        lines.push(format!("{index}\t{}", padded.join("\t")));
    }
    format!("{}\n", lines.join("\n"))
}

fn deck_rawfile_project_rows(rows: &[&str], probes: &[String]) -> Vec<String> {
    let columns = rows[0].split('\t').collect::<Vec<_>>();
    if probes.is_empty() {
        return rows.iter().map(|row| (*row).to_string()).collect();
    }
    let (selected_indices, _, _) = deck_rawfile_probe_inventory(&columns, probes);
    rows.iter()
        .map(|row| {
            let cells = row.split('\t').collect::<Vec<_>>();
            selected_indices
                .iter()
                .map(|index| cells.get(*index).copied().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect()
}

fn deck_rawfile_probe_inventory(
    columns: &[&str],
    probes: &[String],
) -> (Vec<usize>, Vec<String>, Vec<String>) {
    let mut selected_indices = Vec::new();
    let mut matched_probes = Vec::new();
    let mut unmatched_probes = Vec::new();
    if !columns.is_empty() {
        selected_indices.push(0);
    }
    for probe in probes {
        if let Some(index) = columns
            .iter()
            .position(|column| column.eq_ignore_ascii_case(probe))
        {
            if !selected_indices.contains(&index) {
                selected_indices.push(index);
                matched_probes.push(columns[index].to_string());
            }
        } else {
            unmatched_probes.push(probe.clone());
        }
    }
    (selected_indices, matched_probes, unmatched_probes)
}

fn deck_control_policy_category(code: &str) -> Option<&'static str> {
    match code {
        "SPICE_DECK_CONTROL_SCRIPT_COMMAND" => Some("script"),
        "SPICE_DECK_CONTROL_WORKDIR_COMMAND" => Some("workdir"),
        "SPICE_DECK_CONTROL_FLOW_COMMAND" => Some("control-flow"),
        "SPICE_DECK_CONTROL_VARIABLE_COMMAND" => Some("variable"),
        _ => None,
    }
}

fn deck_control_policy_artifacts(netlist: &str) -> Vec<DeckControlPolicyArtifact> {
    let summary = analyze_deck_controls(netlist);
    let lines = netlist.lines().collect::<Vec<_>>();
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let category = deck_control_policy_category(&diagnostic.code)?;
            let command = lines
                .get(diagnostic.line_number.saturating_sub(1))
                .copied()
                .unwrap_or("")
                .trim()
                .to_string();
            Some(DeckControlPolicyArtifact {
                line_number: diagnostic.line_number,
                category: category.to_string(),
                command,
                code: diagnostic.code.clone(),
                severity: diagnostic.severity.clone(),
                message: diagnostic.message.clone(),
            })
        })
        .collect()
}

const DECK_CONTROL_POLICY_ARTIFACT_COLUMNS: &[&str] =
    &["Line", "Category", "Command", "Code", "Severity", "Message"];

fn deck_control_policy_artifact_cells(artifact: &DeckControlPolicyArtifact) -> Vec<String> {
    vec![
        artifact.line_number.to_string(),
        artifact.category.clone(),
        artifact.command.clone(),
        artifact.code.clone(),
        artifact.severity.clone(),
        artifact.message.clone(),
    ]
}

pub fn deck_control_policy_artifact_records(
    artifacts: &[DeckControlPolicyArtifact],
) -> Vec<BTreeMap<String, String>> {
    artifacts
        .iter()
        .map(|artifact| {
            DECK_CONTROL_POLICY_ARTIFACT_COLUMNS
                .iter()
                .copied()
                .zip(deck_control_policy_artifact_cells(artifact))
                .map(|(key, value)| (key.to_string(), value))
                .collect()
        })
        .collect()
}

pub fn format_deck_control_policy_artifact_table(
    artifacts: &[DeckControlPolicyArtifact],
) -> String {
    let mut rows = vec![DECK_CONTROL_POLICY_ARTIFACT_COLUMNS.join("\t")];
    for artifact in artifacts {
        rows.push(deck_control_policy_artifact_cells(artifact).join("\t"));
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_control_policy_artifact_csv(artifacts: &[DeckControlPolicyArtifact]) -> String {
    let mut rows = vec![DECK_CONTROL_POLICY_ARTIFACT_COLUMNS.join(",")];
    for artifact in artifacts {
        rows.push(
            deck_control_policy_artifact_cells(artifact)
                .iter()
                .map(|cell| format_csv_cell(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_control_policy_artifact_json(artifacts: &[DeckControlPolicyArtifact]) -> String {
    let records = deck_control_policy_artifact_records(artifacts)
        .into_iter()
        .map(|record| {
            let fields = record
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        format_json_string(&key),
                        format_json_string(&value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", fields)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]\n", records)
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

fn deck_control_policy_summary_artifacts(
    artifacts: &[DeckControlPolicyArtifact],
) -> Vec<DeckControlPolicySummaryArtifact> {
    let mut summaries: Vec<DeckControlPolicySummaryArtifact> = Vec::new();
    for artifact in artifacts {
        if let Some(summary) = summaries
            .iter_mut()
            .find(|summary| summary.category == artifact.category)
        {
            summary.artifact_count += 1;
            summary.line_numbers.push(artifact.line_number);
            summary.commands.push(artifact.command.clone());
            push_unique_string(&mut summary.codes, &artifact.code);
            push_unique_string(&mut summary.severities, &artifact.severity);
        } else {
            summaries.push(DeckControlPolicySummaryArtifact {
                category: artifact.category.clone(),
                artifact_count: 1,
                line_numbers: vec![artifact.line_number],
                commands: vec![artifact.command.clone()],
                codes: vec![artifact.code.clone()],
                severities: vec![artifact.severity.clone()],
            });
        }
    }
    summaries
}

const DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS: &[&str] = &[
    "Category",
    "Artifacts",
    "LineList",
    "CommandList",
    "CodeList",
    "SeverityList",
];

fn deck_control_policy_summary_artifact_cells(
    artifact: &DeckControlPolicySummaryArtifact,
) -> Vec<String> {
    vec![
        artifact.category.clone(),
        artifact.artifact_count.to_string(),
        artifact
            .line_numbers
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(";"),
        artifact.commands.join(";"),
        artifact.codes.join(";"),
        artifact.severities.join(";"),
    ]
}

pub fn deck_control_policy_summary_artifact_records(
    artifacts: &[DeckControlPolicySummaryArtifact],
) -> Vec<BTreeMap<String, String>> {
    artifacts
        .iter()
        .map(|artifact| {
            DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS
                .iter()
                .copied()
                .zip(deck_control_policy_summary_artifact_cells(artifact))
                .map(|(key, value)| (key.to_string(), value))
                .collect()
        })
        .collect()
}

pub fn format_deck_control_policy_summary_artifact_table(
    artifacts: &[DeckControlPolicySummaryArtifact],
) -> String {
    let mut rows = vec![DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS.join("\t")];
    for artifact in artifacts {
        rows.push(deck_control_policy_summary_artifact_cells(artifact).join("\t"));
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_control_policy_summary_artifact_csv(
    artifacts: &[DeckControlPolicySummaryArtifact],
) -> String {
    let mut rows = vec![DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS.join(",")];
    for artifact in artifacts {
        rows.push(
            deck_control_policy_summary_artifact_cells(artifact)
                .iter()
                .map(|cell| format_csv_cell(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_control_policy_summary_artifact_json(
    artifacts: &[DeckControlPolicySummaryArtifact],
) -> String {
    let records = deck_control_policy_summary_artifact_records(artifacts)
        .into_iter()
        .map(|record| {
            let fields = record
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        format_json_string(&key),
                        format_json_string(&value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", fields)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]\n", records)
}

fn deck_write_marker_parts(marker: &str) -> Option<(String, Vec<String>)> {
    let parts = marker.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 || parts[0] != "write" {
        return None;
    }
    Some((
        parts[1].to_string(),
        parts
            .iter()
            .skip(2)
            .map(|probe| (*probe).to_string())
            .collect(),
    ))
}

fn deck_rawfile_artifacts(
    plan: &DeckAnalysisPlan,
    table: &str,
    write_markers: &[String],
    rawfile_options: &[String],
) -> Vec<DeckRawfileArtifact> {
    let rows = table.lines().collect::<Vec<_>>();
    let columns = rows
        .first()
        .map(|row| row.split('\t').collect::<Vec<_>>())
        .unwrap_or_default();
    write_markers
        .iter()
        .filter_map(|marker| {
            let (target, probes) = deck_write_marker_parts(marker)?;
            let (_, matched_probes, unmatched_probes) =
                deck_rawfile_probe_inventory(&columns, &probes);
            let rawfile = format_deck_rawfile_ascii_with_probes(
                table,
                &plan.analysis,
                rawfile_options,
                &probes,
            );
            Some(DeckRawfileArtifact {
                target,
                marker: marker.clone(),
                probe_count: probes.len(),
                matched_probe_count: matched_probes.len(),
                matched_probes,
                unmatched_probe_count: unmatched_probes.len(),
                unmatched_probes,
                probes,
                option_count: rawfile_options.len(),
                options: rawfile_options.to_vec(),
                rawfile,
            })
        })
        .collect()
}

const DECK_RAWFILE_ARTIFACT_COLUMNS: &[&str] = &[
    "Target",
    "Marker",
    "Probes",
    "ProbeList",
    "MatchedProbes",
    "MatchedProbeList",
    "UnmatchedProbes",
    "UnmatchedProbeList",
    "Options",
    "RawfileOptionList",
    "Bytes",
];

fn deck_rawfile_artifact_cells(artifact: &DeckRawfileArtifact) -> Vec<String> {
    vec![
        artifact.target.clone(),
        artifact.marker.clone(),
        artifact.probe_count.to_string(),
        artifact.probes.join(";"),
        artifact.matched_probe_count.to_string(),
        artifact.matched_probes.join(";"),
        artifact.unmatched_probe_count.to_string(),
        artifact.unmatched_probes.join(";"),
        artifact.option_count.to_string(),
        artifact.options.join(";"),
        artifact.rawfile.len().to_string(),
    ]
}

pub fn deck_rawfile_artifact_records(
    artifacts: &[DeckRawfileArtifact],
) -> Vec<BTreeMap<String, String>> {
    artifacts
        .iter()
        .map(|artifact| {
            DECK_RAWFILE_ARTIFACT_COLUMNS
                .iter()
                .copied()
                .zip(deck_rawfile_artifact_cells(artifact))
                .map(|(key, value)| (key.to_string(), value))
                .collect()
        })
        .collect()
}

pub fn format_deck_rawfile_artifact_table(artifacts: &[DeckRawfileArtifact]) -> String {
    let mut rows = vec![DECK_RAWFILE_ARTIFACT_COLUMNS.join("\t")];
    for artifact in artifacts {
        rows.push(deck_rawfile_artifact_cells(artifact).join("\t"));
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_rawfile_artifact_csv(artifacts: &[DeckRawfileArtifact]) -> String {
    let mut rows = vec![DECK_RAWFILE_ARTIFACT_COLUMNS.join(",")];
    for artifact in artifacts {
        rows.push(
            deck_rawfile_artifact_cells(artifact)
                .iter()
                .map(|cell| format_csv_cell(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_rawfile_artifact_json(artifacts: &[DeckRawfileArtifact]) -> String {
    let records = deck_rawfile_artifact_records(artifacts)
        .into_iter()
        .map(|record| {
            let fields = record
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        format_json_string(&key),
                        format_json_string(&value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", fields)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]\n", records)
}

pub fn format_deck_wrdata_ascii(
    table: &str,
    probes: &[String],
    rawfile_options: &[String],
) -> String {
    let rows = table.lines().collect::<Vec<_>>();
    if rows.is_empty() {
        return String::new();
    }
    let projected_rows = deck_wrdata_project_rows(&rows, probes);
    let columns = projected_rows[0].split('\t').collect::<Vec<_>>();
    let mut lines = vec![
        "# SPICE deck wrdata artifact".to_string(),
        format!("Probes: {}", probes.join(";")),
    ];
    if !rawfile_options.is_empty() {
        lines.push(format!("Options: {}", rawfile_options.join(";")));
    }
    let normalized_options = rawfile_options
        .iter()
        .map(|option| option.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if normalized_options
        .iter()
        .any(|option| option == "set wr_vecnames")
    {
        lines.push(format!("VectorNames: {}", columns.join(";")));
    }
    if normalized_options
        .iter()
        .any(|option| option == "set wr_singlescale")
    {
        if let Some(scale) = columns.first() {
            lines.push(format!("Scale: {scale}"));
        }
    }
    lines.extend(projected_rows);
    format!("{}\n", lines.join("\n"))
}

fn deck_wrdata_project_rows(rows: &[&str], probes: &[String]) -> Vec<String> {
    let columns = rows[0].split('\t').collect::<Vec<_>>();
    if probes.is_empty() {
        return rows.iter().map(|row| (*row).to_string()).collect();
    }
    let (selected_indices, _, _) = deck_wrdata_probe_inventory(&columns, probes);
    rows.iter()
        .map(|row| {
            let cells = row.split('\t').collect::<Vec<_>>();
            selected_indices
                .iter()
                .map(|index| cells.get(*index).copied().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect()
}

fn deck_wrdata_probe_inventory(
    columns: &[&str],
    probes: &[String],
) -> (Vec<usize>, Vec<String>, Vec<String>) {
    let mut selected_indices = Vec::new();
    let mut matched_probes = Vec::new();
    let mut unmatched_probes = Vec::new();
    if !columns.is_empty() {
        selected_indices.push(0);
    }
    for probe in probes {
        if let Some(index) = columns
            .iter()
            .position(|column| column.eq_ignore_ascii_case(probe))
        {
            if !selected_indices.contains(&index) {
                selected_indices.push(index);
                matched_probes.push(columns[index].to_string());
            }
        } else {
            unmatched_probes.push(probe.clone());
        }
    }
    (selected_indices, matched_probes, unmatched_probes)
}

fn deck_wrdata_marker_parts(marker: &str) -> Option<(String, Vec<String>)> {
    let parts = marker.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 || parts[0] != "wrdata" {
        return None;
    }
    Some((
        parts[1].to_string(),
        parts
            .iter()
            .skip(2)
            .map(|probe| (*probe).to_string())
            .collect(),
    ))
}

fn deck_wrdata_artifacts(
    table: &str,
    write_markers: &[String],
    rawfile_options: &[String],
) -> Vec<DeckWrdataArtifact> {
    let rows = table.lines().collect::<Vec<_>>();
    let columns = rows
        .first()
        .map(|row| row.split('\t').collect::<Vec<_>>())
        .unwrap_or_default();
    write_markers
        .iter()
        .filter_map(|marker| {
            let (target, probes) = deck_wrdata_marker_parts(marker)?;
            let (_, matched_probes, unmatched_probes) =
                deck_wrdata_probe_inventory(&columns, &probes);
            Some(DeckWrdataArtifact {
                target,
                marker: marker.clone(),
                probe_count: probes.len(),
                matched_probe_count: matched_probes.len(),
                matched_probes,
                unmatched_probe_count: unmatched_probes.len(),
                unmatched_probes,
                option_count: rawfile_options.len(),
                options: rawfile_options.to_vec(),
                datafile: format_deck_wrdata_ascii(table, &probes, rawfile_options),
                probes,
            })
        })
        .collect()
}

const DECK_WRDATA_ARTIFACT_COLUMNS: &[&str] = &[
    "Target",
    "Marker",
    "Probes",
    "ProbeList",
    "MatchedProbes",
    "MatchedProbeList",
    "UnmatchedProbes",
    "UnmatchedProbeList",
    "Options",
    "RawfileOptionList",
    "Bytes",
];

fn deck_wrdata_artifact_cells(artifact: &DeckWrdataArtifact) -> Vec<String> {
    vec![
        artifact.target.clone(),
        artifact.marker.clone(),
        artifact.probe_count.to_string(),
        artifact.probes.join(";"),
        artifact.matched_probe_count.to_string(),
        artifact.matched_probes.join(";"),
        artifact.unmatched_probe_count.to_string(),
        artifact.unmatched_probes.join(";"),
        artifact.option_count.to_string(),
        artifact.options.join(";"),
        artifact.datafile.len().to_string(),
    ]
}

pub fn deck_wrdata_artifact_records(
    artifacts: &[DeckWrdataArtifact],
) -> Vec<BTreeMap<String, String>> {
    artifacts
        .iter()
        .map(|artifact| {
            DECK_WRDATA_ARTIFACT_COLUMNS
                .iter()
                .copied()
                .zip(deck_wrdata_artifact_cells(artifact))
                .map(|(key, value)| (key.to_string(), value))
                .collect()
        })
        .collect()
}

pub fn format_deck_wrdata_artifact_table(artifacts: &[DeckWrdataArtifact]) -> String {
    let mut rows = vec![DECK_WRDATA_ARTIFACT_COLUMNS.join("\t")];
    for artifact in artifacts {
        rows.push(deck_wrdata_artifact_cells(artifact).join("\t"));
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_wrdata_artifact_csv(artifacts: &[DeckWrdataArtifact]) -> String {
    let mut rows = vec![DECK_WRDATA_ARTIFACT_COLUMNS.join(",")];
    for artifact in artifacts {
        rows.push(
            deck_wrdata_artifact_cells(artifact)
                .iter()
                .map(|cell| format_csv_cell(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    format!("{}\n", rows.join("\n"))
}

pub fn format_deck_wrdata_artifact_json(artifacts: &[DeckWrdataArtifact]) -> String {
    let records = deck_wrdata_artifact_records(artifacts)
        .into_iter()
        .map(|record| {
            let fields = record
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        format_json_string(&key),
                        format_json_string(&value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", fields)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]\n", records)
}

pub fn format_deck_run_artifact_csv(artifacts: &[DeckRunArtifact]) -> String {
    let mut rows = vec![DECK_RUN_ARTIFACT_COLUMNS.join(",")];
    for artifact in artifacts {
        rows.push(
            deck_run_artifact_cells(artifact)
                .iter()
                .map(|cell| format_csv_cell(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    format!("{}\n", rows.join("\n"))
}

fn format_json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub fn format_deck_table_json(table: &str) -> String {
    let mut lines = table.lines();
    let Some(header) = lines.next() else {
        return "[]\n".to_string();
    };
    let columns = header.split('\t').collect::<Vec<_>>();
    let records = lines
        .map(|row| {
            let cells = row.split('\t').collect::<Vec<_>>();
            let fields = columns
                .iter()
                .enumerate()
                .map(|(index, column)| {
                    let value = cells.get(index).copied().unwrap_or("");
                    format!(
                        "{}:{}",
                        format_json_string(column),
                        format_json_string(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", fields)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]\n", records)
}

pub fn format_deck_run_artifact_json(artifacts: &[DeckRunArtifact]) -> String {
    let records = artifacts
        .iter()
        .map(|artifact| {
            let fields = deck_run_artifact_record(artifact)
                .into_iter()
                .map(|(key, value)| {
                    format!("{}:{}", format_json_string(key), format_json_string(&value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", fields)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]\n", records)
}

fn select_deck_measurement_cards_for_analysis(
    netlist: &str,
    analysis: &str,
) -> Result<Vec<DeckMeasurementCard>, SpiceError> {
    let summary = resolve_deck_measurements(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "run_deck_analysis",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    Ok(summary
        .measurements
        .into_iter()
        .filter(|measurement| {
            measurement.analysis == analysis
                || (analysis == "tran" && measurement.analysis == "transient")
        })
        .collect())
}

fn select_deck_fourier_cards_for_analysis(
    netlist: &str,
    analysis: &str,
) -> Result<Vec<DeckFourierCard>, SpiceError> {
    let summary = resolve_deck_fourier(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "run_deck_analysis",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    Ok(if analysis == "tran" {
        summary.fourier
    } else {
        Vec::new()
    })
}

pub fn run_deck_analysis(
    circuit: &Circuit,
    netlist: &str,
    analysis: Option<&str>,
) -> Result<DeckAnalysisExecution, SpiceError> {
    let plan = select_deck_analysis_plan(netlist, analysis)?;
    run_deck_analysis_plan(circuit, netlist, plan)
}

pub fn run_deck(circuit: &Circuit, netlist: &str) -> Result<DeckExecution, SpiceError> {
    let plans = deck_analysis_plans_for_execution(netlist, "run_deck")?;
    let mut executions = Vec::new();
    for plan in plans.iter().cloned() {
        executions.push(run_deck_analysis_plan(circuit, netlist, plan)?);
    }
    let run_artifacts = executions
        .iter()
        .flat_map(|execution| execution.run_artifacts.iter().cloned())
        .collect::<Vec<_>>();
    let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
    let run_artifact_csv = format_deck_run_artifact_csv(&run_artifacts);
    let run_artifact_json = format_deck_run_artifact_json(&run_artifacts);
    let run_artifact_records = deck_run_artifact_records(&run_artifacts);
    Ok(DeckExecution {
        execution_count: executions.len(),
        analysis_order: plans.iter().map(|plan| plan.analysis.clone()).collect(),
        analysis_directives: plans.iter().map(|plan| plan.directive.clone()).collect(),
        executions,
        run_artifact_count: run_artifacts.len(),
        run_artifacts,
        run_artifact_table,
        run_artifact_csv,
        run_artifact_json,
        run_artifact_records,
    })
}

fn deck_analysis_plans_for_execution(
    netlist: &str,
    context: &str,
) -> Result<Vec<DeckAnalysisPlan>, SpiceError> {
    let summary = resolve_deck_analyses(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            context,
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    if summary.analyses.is_empty() {
        Ok(vec![DeckAnalysisPlan {
            directive: ".op".to_string(),
            analysis: "op".to_string(),
            line_number: 0,
            source_name: None,
            output_node: None,
            start_value: None,
            stop_value: None,
            step_value: None,
            sweep_kind: None,
            point_count: None,
            start_frequency_hz: None,
            stop_frequency_hz: None,
            step_time: None,
            stop_time: None,
            start_time: None,
            max_step: None,
            use_initial_conditions: false,
        }])
    } else {
        Ok(summary.analyses)
    }
}

fn run_deck_analysis_plan(
    circuit: &Circuit,
    netlist: &str,
    plan: DeckAnalysisPlan,
) -> Result<DeckAnalysisExecution, SpiceError> {
    let diagnostic_codes = deck_run_diagnostic_codes(netlist, &plan);
    let control_lines = deck_control_lines(netlist);
    let write_markers = deck_control_write_markers(netlist);
    let rawfile_options = deck_control_rawfile_options(netlist);
    let control_policy_artifacts = deck_control_policy_artifacts(netlist);
    let control_policy_artifact_table =
        format_deck_control_policy_artifact_table(&control_policy_artifacts);
    let control_policy_artifact_csv =
        format_deck_control_policy_artifact_csv(&control_policy_artifacts);
    let control_policy_artifact_json =
        format_deck_control_policy_artifact_json(&control_policy_artifacts);
    let control_policy_artifact_records =
        deck_control_policy_artifact_records(&control_policy_artifacts);
    let control_policy_summary_artifacts =
        deck_control_policy_summary_artifacts(&control_policy_artifacts);
    let control_policy_summary_artifact_table =
        format_deck_control_policy_summary_artifact_table(&control_policy_summary_artifacts);
    let control_policy_summary_artifact_csv =
        format_deck_control_policy_summary_artifact_csv(&control_policy_summary_artifacts);
    let control_policy_summary_artifact_json =
        format_deck_control_policy_summary_artifact_json(&control_policy_summary_artifacts);
    let control_policy_summary_artifact_records =
        deck_control_policy_summary_artifact_records(&control_policy_summary_artifacts);
    let analysis_directives = deck_analysis_directives(&plan);
    let (deck_analysis_kinds, deck_analysis_directives) = deck_analysis_inventory(netlist);
    match plan.analysis.as_str() {
        "op" => {
            let result = dc_op(circuit)?;
            let table = format_deck_op_table(&result, netlist)?;
            select_deck_measurement_cards_for_analysis(netlist, "op")?;
            let measurements = Vec::new();
            let measurement_table = format_measurement_table(&measurements);
            select_deck_fourier_cards_for_analysis(netlist, "op")?;
            let fourier = Vec::new();
            let fourier_table = format_deck_fourier_table(&fourier);
            let output_probes = select_deck_output_probes(netlist, "op")?;
            let output_probe_lines = select_deck_output_probe_lines(netlist, "op")?;
            let output_directives = select_deck_output_directives(netlist, "op")?;
            let output_directive_analysis_kinds =
                select_deck_output_directive_analysis_kinds(netlist, "op")?;
            let output_directive_lines = select_deck_output_directive_lines(netlist, "op")?;
            let run_artifacts = deck_run_artifacts(
                &plan,
                1,
                &deck_table_columns(&table),
                &output_probes,
                &output_directives,
                &measurements,
                &fourier,
                &control_lines,
                &write_markers,
                &rawfile_options,
                &diagnostic_codes,
                &control_policy_artifacts,
                &deck_analysis_kinds,
                &deck_analysis_directives,
            );
            let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
            let tables = deck_stable_tables(&measurements, &fourier, &control_policy_artifacts);
            let table_artifacts = deck_table_artifacts(
                &plan,
                &table,
                &measurement_table,
                &fourier_table,
                &run_artifact_table,
                &measurements,
                &fourier,
                &control_policy_artifacts,
                &control_policy_artifact_table,
                &control_policy_summary_artifacts,
                &control_policy_summary_artifact_table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            let rawfile_artifacts =
                deck_rawfile_artifacts(&plan, &table, &write_markers, &rawfile_options);
            let rawfile_artifact_table = format_deck_rawfile_artifact_table(&rawfile_artifacts);
            let rawfile_artifact_csv = format_deck_rawfile_artifact_csv(&rawfile_artifacts);
            let rawfile_artifact_json = format_deck_rawfile_artifact_json(&rawfile_artifacts);
            let rawfile_artifact_records = deck_rawfile_artifact_records(&rawfile_artifacts);
            let wrdata_artifacts = deck_wrdata_artifacts(&table, &write_markers, &rawfile_options);
            let wrdata_artifact_table = format_deck_wrdata_artifact_table(&wrdata_artifacts);
            let wrdata_artifact_csv = format_deck_wrdata_artifact_csv(&wrdata_artifacts);
            let wrdata_artifact_json = format_deck_wrdata_artifact_json(&wrdata_artifacts);
            let wrdata_artifact_records = deck_wrdata_artifact_records(&wrdata_artifacts);
            let (
                output_plan_artifacts,
                output_plan_artifact_table,
                output_plan_artifact_csv,
                output_plan_artifact_json,
                output_plan_artifact_records,
            ) = deck_output_plan_artifact_bundle(
                &plan,
                &table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            Ok(DeckAnalysisExecution {
                plan,
                result: DeckAnalysisExecutionResult::Op(result),
                table,
                output_probes,
                output_directives,
                analysis_directives,
                deck_analysis_kind_count: deck_analysis_kinds.len(),
                deck_analysis_kinds,
                deck_analysis_directive_count: deck_analysis_directives.len(),
                deck_analysis_directives,

                output_plan_artifact_count: output_plan_artifacts.len(),

                output_plan_artifacts,

                output_plan_artifact_table,

                output_plan_artifact_csv,

                output_plan_artifact_json,

                output_plan_artifact_records,
                control_line_count: control_lines.len(),
                control_lines: control_lines.clone(),
                write_marker_count: write_markers.len(),
                write_markers: write_markers.clone(),
                rawfile_option_count: rawfile_options.len(),
                rawfile_options: rawfile_options.clone(),
                control_policy_artifact_count: control_policy_artifacts.len(),
                control_policy_artifacts: control_policy_artifacts.clone(),
                control_policy_artifact_table: control_policy_artifact_table.clone(),
                control_policy_artifact_csv: control_policy_artifact_csv.clone(),
                control_policy_artifact_json: control_policy_artifact_json.clone(),
                control_policy_artifact_records: control_policy_artifact_records.clone(),
                control_policy_summary_artifact_count: control_policy_summary_artifacts.len(),
                control_policy_summary_artifacts: control_policy_summary_artifacts.clone(),
                control_policy_summary_artifact_table: control_policy_summary_artifact_table
                    .clone(),
                control_policy_summary_artifact_csv: control_policy_summary_artifact_csv.clone(),
                control_policy_summary_artifact_json: control_policy_summary_artifact_json.clone(),
                control_policy_summary_artifact_records: control_policy_summary_artifact_records
                    .clone(),
                rawfile_artifact_count: rawfile_artifacts.len(),
                rawfile_artifacts,
                rawfile_artifact_table,
                rawfile_artifact_csv,
                rawfile_artifact_json,
                rawfile_artifact_records,
                wrdata_artifact_count: wrdata_artifacts.len(),
                wrdata_artifacts,
                wrdata_artifact_table,
                wrdata_artifact_csv,
                wrdata_artifact_json,
                wrdata_artifact_records,
                diagnostic_count: diagnostic_codes.len(),
                diagnostic_codes: diagnostic_codes.clone(),
                table_count: tables.len(),
                tables,
                table_artifacts,
                measurements,
                measurement_table,
                fourier,
                fourier_table,
                run_artifacts,
                run_artifact_table,
            })
        }
        "dc" => {
            let source_name =
                require_deck_plan_string(plan.source_name.as_deref(), &plan, "source_name")?
                    .to_string();
            let start = require_deck_plan_number(plan.start_value, &plan, "start_value")?;
            let stop = require_deck_plan_number(plan.stop_value, &plan, "stop_value")?;
            let step = require_deck_plan_number(plan.step_value, &plan, "step_value")?;
            let result = dc_sweep(circuit, &source_name, start, stop, step)?;
            let table = format_deck_dc_sweep_table(&source_name, &result, netlist)?;
            let measurement_cards = select_deck_measurement_cards_for_analysis(netlist, "dc")?;
            let measurements = measure_dc_sweep_cards(&result, &measurement_cards)?;
            let measurement_table = format_measurement_table(&measurements);
            select_deck_fourier_cards_for_analysis(netlist, "dc")?;
            let fourier = Vec::new();
            let fourier_table = format_deck_fourier_table(&fourier);
            let output_probes = select_deck_output_probes(netlist, "dc")?;
            let output_probe_lines = select_deck_output_probe_lines(netlist, "dc")?;
            let output_directives = select_deck_output_directives(netlist, "dc")?;
            let output_directive_analysis_kinds =
                select_deck_output_directive_analysis_kinds(netlist, "dc")?;
            let output_directive_lines = select_deck_output_directive_lines(netlist, "dc")?;
            let run_artifacts = deck_run_artifacts(
                &plan,
                result.len(),
                &deck_table_columns(&table),
                &output_probes,
                &output_directives,
                &measurements,
                &fourier,
                &control_lines,
                &write_markers,
                &rawfile_options,
                &diagnostic_codes,
                &control_policy_artifacts,
                &deck_analysis_kinds,
                &deck_analysis_directives,
            );
            let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
            let tables = deck_stable_tables(&measurements, &fourier, &control_policy_artifacts);
            let table_artifacts = deck_table_artifacts(
                &plan,
                &table,
                &measurement_table,
                &fourier_table,
                &run_artifact_table,
                &measurements,
                &fourier,
                &control_policy_artifacts,
                &control_policy_artifact_table,
                &control_policy_summary_artifacts,
                &control_policy_summary_artifact_table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            let rawfile_artifacts =
                deck_rawfile_artifacts(&plan, &table, &write_markers, &rawfile_options);
            let rawfile_artifact_table = format_deck_rawfile_artifact_table(&rawfile_artifacts);
            let rawfile_artifact_csv = format_deck_rawfile_artifact_csv(&rawfile_artifacts);
            let rawfile_artifact_json = format_deck_rawfile_artifact_json(&rawfile_artifacts);
            let rawfile_artifact_records = deck_rawfile_artifact_records(&rawfile_artifacts);
            let wrdata_artifacts = deck_wrdata_artifacts(&table, &write_markers, &rawfile_options);
            let wrdata_artifact_table = format_deck_wrdata_artifact_table(&wrdata_artifacts);
            let wrdata_artifact_csv = format_deck_wrdata_artifact_csv(&wrdata_artifacts);
            let wrdata_artifact_json = format_deck_wrdata_artifact_json(&wrdata_artifacts);
            let wrdata_artifact_records = deck_wrdata_artifact_records(&wrdata_artifacts);
            let (
                output_plan_artifacts,
                output_plan_artifact_table,
                output_plan_artifact_csv,
                output_plan_artifact_json,
                output_plan_artifact_records,
            ) = deck_output_plan_artifact_bundle(
                &plan,
                &table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            Ok(DeckAnalysisExecution {
                plan,
                result: DeckAnalysisExecutionResult::DcSweep(result),
                table,
                output_probes,
                output_directives,
                analysis_directives,
                deck_analysis_kind_count: deck_analysis_kinds.len(),
                deck_analysis_kinds,
                deck_analysis_directive_count: deck_analysis_directives.len(),
                deck_analysis_directives,

                output_plan_artifact_count: output_plan_artifacts.len(),

                output_plan_artifacts,

                output_plan_artifact_table,

                output_plan_artifact_csv,

                output_plan_artifact_json,

                output_plan_artifact_records,
                control_line_count: control_lines.len(),
                control_lines: control_lines.clone(),
                write_marker_count: write_markers.len(),
                write_markers: write_markers.clone(),
                rawfile_option_count: rawfile_options.len(),
                rawfile_options: rawfile_options.clone(),
                control_policy_artifact_count: control_policy_artifacts.len(),
                control_policy_artifacts: control_policy_artifacts.clone(),
                control_policy_artifact_table: control_policy_artifact_table.clone(),
                control_policy_artifact_csv: control_policy_artifact_csv.clone(),
                control_policy_artifact_json: control_policy_artifact_json.clone(),
                control_policy_artifact_records: control_policy_artifact_records.clone(),
                control_policy_summary_artifact_count: control_policy_summary_artifacts.len(),
                control_policy_summary_artifacts: control_policy_summary_artifacts.clone(),
                control_policy_summary_artifact_table: control_policy_summary_artifact_table
                    .clone(),
                control_policy_summary_artifact_csv: control_policy_summary_artifact_csv.clone(),
                control_policy_summary_artifact_json: control_policy_summary_artifact_json.clone(),
                control_policy_summary_artifact_records: control_policy_summary_artifact_records
                    .clone(),
                rawfile_artifact_count: rawfile_artifacts.len(),
                rawfile_artifacts,
                rawfile_artifact_table,
                rawfile_artifact_csv,
                rawfile_artifact_json,
                rawfile_artifact_records,
                wrdata_artifact_count: wrdata_artifacts.len(),
                wrdata_artifacts,
                wrdata_artifact_table,
                wrdata_artifact_csv,
                wrdata_artifact_json,
                wrdata_artifact_records,
                diagnostic_count: diagnostic_codes.len(),
                diagnostic_codes: diagnostic_codes.clone(),
                table_count: tables.len(),
                tables,
                table_artifacts,
                measurements,
                measurement_table,
                fourier,
                fourier_table,
                run_artifacts,
                run_artifact_table,
            })
        }
        "ac" => {
            let sweep_kind =
                require_deck_plan_string(plan.sweep_kind.as_deref(), &plan, "sweep_kind")?;
            let point_count = require_deck_plan_usize(plan.point_count, &plan, "point_count")?;
            let start =
                require_deck_plan_number(plan.start_frequency_hz, &plan, "start_frequency_hz")?;
            let stop =
                require_deck_plan_number(plan.stop_frequency_hz, &plan, "stop_frequency_hz")?;
            let result = run_deck_ac_sweep(circuit, &plan, sweep_kind, point_count, start, stop)?;
            let table = format_deck_ac_table(&result, netlist)?;
            let measurement_cards = select_deck_measurement_cards_for_analysis(netlist, "ac")?;
            let measurements = measure_ac_sweep_cards(&result, &measurement_cards)?;
            let measurement_table = format_measurement_table(&measurements);
            select_deck_fourier_cards_for_analysis(netlist, "ac")?;
            let fourier = Vec::new();
            let fourier_table = format_deck_fourier_table(&fourier);
            let output_probes = select_deck_output_probes(netlist, "ac")?;
            let output_probe_lines = select_deck_output_probe_lines(netlist, "ac")?;
            let output_directives = select_deck_output_directives(netlist, "ac")?;
            let output_directive_analysis_kinds =
                select_deck_output_directive_analysis_kinds(netlist, "ac")?;
            let output_directive_lines = select_deck_output_directive_lines(netlist, "ac")?;
            let run_artifacts = deck_run_artifacts(
                &plan,
                result.len(),
                &deck_table_columns(&table),
                &output_probes,
                &output_directives,
                &measurements,
                &fourier,
                &control_lines,
                &write_markers,
                &rawfile_options,
                &diagnostic_codes,
                &control_policy_artifacts,
                &deck_analysis_kinds,
                &deck_analysis_directives,
            );
            let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
            let tables = deck_stable_tables(&measurements, &fourier, &control_policy_artifacts);
            let table_artifacts = deck_table_artifacts(
                &plan,
                &table,
                &measurement_table,
                &fourier_table,
                &run_artifact_table,
                &measurements,
                &fourier,
                &control_policy_artifacts,
                &control_policy_artifact_table,
                &control_policy_summary_artifacts,
                &control_policy_summary_artifact_table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            let rawfile_artifacts =
                deck_rawfile_artifacts(&plan, &table, &write_markers, &rawfile_options);
            let rawfile_artifact_table = format_deck_rawfile_artifact_table(&rawfile_artifacts);
            let rawfile_artifact_csv = format_deck_rawfile_artifact_csv(&rawfile_artifacts);
            let rawfile_artifact_json = format_deck_rawfile_artifact_json(&rawfile_artifacts);
            let rawfile_artifact_records = deck_rawfile_artifact_records(&rawfile_artifacts);
            let wrdata_artifacts = deck_wrdata_artifacts(&table, &write_markers, &rawfile_options);
            let wrdata_artifact_table = format_deck_wrdata_artifact_table(&wrdata_artifacts);
            let wrdata_artifact_csv = format_deck_wrdata_artifact_csv(&wrdata_artifacts);
            let wrdata_artifact_json = format_deck_wrdata_artifact_json(&wrdata_artifacts);
            let wrdata_artifact_records = deck_wrdata_artifact_records(&wrdata_artifacts);
            let (
                output_plan_artifacts,
                output_plan_artifact_table,
                output_plan_artifact_csv,
                output_plan_artifact_json,
                output_plan_artifact_records,
            ) = deck_output_plan_artifact_bundle(
                &plan,
                &table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            Ok(DeckAnalysisExecution {
                plan,
                result: DeckAnalysisExecutionResult::Ac(result),
                table,
                output_probes,
                output_directives,
                analysis_directives,
                deck_analysis_kind_count: deck_analysis_kinds.len(),
                deck_analysis_kinds,
                deck_analysis_directive_count: deck_analysis_directives.len(),
                deck_analysis_directives,

                output_plan_artifact_count: output_plan_artifacts.len(),

                output_plan_artifacts,

                output_plan_artifact_table,

                output_plan_artifact_csv,

                output_plan_artifact_json,

                output_plan_artifact_records,
                control_line_count: control_lines.len(),
                control_lines: control_lines.clone(),
                write_marker_count: write_markers.len(),
                write_markers: write_markers.clone(),
                rawfile_option_count: rawfile_options.len(),
                rawfile_options: rawfile_options.clone(),
                control_policy_artifact_count: control_policy_artifacts.len(),
                control_policy_artifacts: control_policy_artifacts.clone(),
                control_policy_artifact_table: control_policy_artifact_table.clone(),
                control_policy_artifact_csv: control_policy_artifact_csv.clone(),
                control_policy_artifact_json: control_policy_artifact_json.clone(),
                control_policy_artifact_records: control_policy_artifact_records.clone(),
                control_policy_summary_artifact_count: control_policy_summary_artifacts.len(),
                control_policy_summary_artifacts: control_policy_summary_artifacts.clone(),
                control_policy_summary_artifact_table: control_policy_summary_artifact_table
                    .clone(),
                control_policy_summary_artifact_csv: control_policy_summary_artifact_csv.clone(),
                control_policy_summary_artifact_json: control_policy_summary_artifact_json.clone(),
                control_policy_summary_artifact_records: control_policy_summary_artifact_records
                    .clone(),
                rawfile_artifact_count: rawfile_artifacts.len(),
                rawfile_artifacts,
                rawfile_artifact_table,
                rawfile_artifact_csv,
                rawfile_artifact_json,
                rawfile_artifact_records,
                wrdata_artifact_count: wrdata_artifacts.len(),
                wrdata_artifacts,
                wrdata_artifact_table,
                wrdata_artifact_csv,
                wrdata_artifact_json,
                wrdata_artifact_records,
                diagnostic_count: diagnostic_codes.len(),
                diagnostic_codes: diagnostic_codes.clone(),
                table_count: tables.len(),
                tables,
                table_artifacts,
                measurements,
                measurement_table,
                fourier,
                fourier_table,
                run_artifacts,
                run_artifact_table,
            })
        }
        "tran" => {
            let step_time = require_deck_plan_number(plan.step_time, &plan, "step_time")?;
            let stop_time = require_deck_plan_number(plan.stop_time, &plan, "stop_time")?;
            let run_step = plan
                .max_step
                .map_or(step_time, |max_step| step_time.min(max_step));
            let result = sample_transient_points_print_step(
                transient(circuit, run_step, stop_time)?,
                step_time,
                plan.start_time,
                stop_time,
            )?;
            let table = format_deck_transient_table(&result, netlist)?;
            let measurement_cards = select_deck_measurement_cards_for_analysis(netlist, "tran")?;
            let measurements = measure_transient_cards(&result, &measurement_cards)?;
            let measurement_table = format_measurement_table(&measurements);
            let fourier_cards = select_deck_fourier_cards_for_analysis(netlist, "tran")?;
            let fourier = fourier_transient_cards(&result, &fourier_cards)?;
            let fourier_table = format_deck_fourier_table(&fourier);
            let output_probes = select_deck_output_probes(netlist, "tran")?;
            let output_probe_lines = select_deck_output_probe_lines(netlist, "tran")?;
            let output_directives = select_deck_output_directives(netlist, "tran")?;
            let output_directive_analysis_kinds =
                select_deck_output_directive_analysis_kinds(netlist, "tran")?;
            let output_directive_lines = select_deck_output_directive_lines(netlist, "tran")?;
            let run_artifacts = deck_run_artifacts(
                &plan,
                result.len(),
                &deck_table_columns(&table),
                &output_probes,
                &output_directives,
                &measurements,
                &fourier,
                &control_lines,
                &write_markers,
                &rawfile_options,
                &diagnostic_codes,
                &control_policy_artifacts,
                &deck_analysis_kinds,
                &deck_analysis_directives,
            );
            let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
            let tables = deck_stable_tables(&measurements, &fourier, &control_policy_artifacts);
            let table_artifacts = deck_table_artifacts(
                &plan,
                &table,
                &measurement_table,
                &fourier_table,
                &run_artifact_table,
                &measurements,
                &fourier,
                &control_policy_artifacts,
                &control_policy_artifact_table,
                &control_policy_summary_artifacts,
                &control_policy_summary_artifact_table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            let rawfile_artifacts =
                deck_rawfile_artifacts(&plan, &table, &write_markers, &rawfile_options);
            let rawfile_artifact_table = format_deck_rawfile_artifact_table(&rawfile_artifacts);
            let rawfile_artifact_csv = format_deck_rawfile_artifact_csv(&rawfile_artifacts);
            let rawfile_artifact_json = format_deck_rawfile_artifact_json(&rawfile_artifacts);
            let rawfile_artifact_records = deck_rawfile_artifact_records(&rawfile_artifacts);
            let wrdata_artifacts = deck_wrdata_artifacts(&table, &write_markers, &rawfile_options);
            let wrdata_artifact_table = format_deck_wrdata_artifact_table(&wrdata_artifacts);
            let wrdata_artifact_csv = format_deck_wrdata_artifact_csv(&wrdata_artifacts);
            let wrdata_artifact_json = format_deck_wrdata_artifact_json(&wrdata_artifacts);
            let wrdata_artifact_records = deck_wrdata_artifact_records(&wrdata_artifacts);
            let (
                output_plan_artifacts,
                output_plan_artifact_table,
                output_plan_artifact_csv,
                output_plan_artifact_json,
                output_plan_artifact_records,
            ) = deck_output_plan_artifact_bundle(
                &plan,
                &table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            Ok(DeckAnalysisExecution {
                plan,
                result: DeckAnalysisExecutionResult::Tran(result),
                table,
                output_probes,
                output_directives,
                analysis_directives,
                deck_analysis_kind_count: deck_analysis_kinds.len(),
                deck_analysis_kinds,
                deck_analysis_directive_count: deck_analysis_directives.len(),
                deck_analysis_directives,

                output_plan_artifact_count: output_plan_artifacts.len(),

                output_plan_artifacts,

                output_plan_artifact_table,

                output_plan_artifact_csv,

                output_plan_artifact_json,

                output_plan_artifact_records,
                control_line_count: control_lines.len(),
                control_lines: control_lines.clone(),
                write_marker_count: write_markers.len(),
                write_markers: write_markers.clone(),
                rawfile_option_count: rawfile_options.len(),
                rawfile_options: rawfile_options.clone(),
                control_policy_artifact_count: control_policy_artifacts.len(),
                control_policy_artifacts: control_policy_artifacts.clone(),
                control_policy_artifact_table: control_policy_artifact_table.clone(),
                control_policy_artifact_csv: control_policy_artifact_csv.clone(),
                control_policy_artifact_json: control_policy_artifact_json.clone(),
                control_policy_artifact_records: control_policy_artifact_records.clone(),
                control_policy_summary_artifact_count: control_policy_summary_artifacts.len(),
                control_policy_summary_artifacts: control_policy_summary_artifacts.clone(),
                control_policy_summary_artifact_table: control_policy_summary_artifact_table
                    .clone(),
                control_policy_summary_artifact_csv: control_policy_summary_artifact_csv.clone(),
                control_policy_summary_artifact_json: control_policy_summary_artifact_json.clone(),
                control_policy_summary_artifact_records: control_policy_summary_artifact_records
                    .clone(),
                rawfile_artifact_count: rawfile_artifacts.len(),
                rawfile_artifacts,
                rawfile_artifact_table,
                rawfile_artifact_csv,
                rawfile_artifact_json,
                rawfile_artifact_records,
                wrdata_artifact_count: wrdata_artifacts.len(),
                wrdata_artifacts,
                wrdata_artifact_table,
                wrdata_artifact_csv,
                wrdata_artifact_json,
                wrdata_artifact_records,
                diagnostic_count: diagnostic_codes.len(),
                diagnostic_codes: diagnostic_codes.clone(),
                table_count: tables.len(),
                tables,
                table_artifacts,
                measurements,
                measurement_table,
                fourier,
                fourier_table,
                run_artifacts,
                run_artifact_table,
            })
        }
        "tf" => {
            let output_node =
                require_deck_plan_string(plan.output_node.as_deref(), &plan, "output_node")?;
            let input_source =
                require_deck_plan_string(plan.source_name.as_deref(), &plan, "source_name")?;
            let result = tf(circuit, output_node, input_source)?;
            select_deck_measurement_cards_for_analysis(netlist, "tf")?;
            let measurements = Vec::new();
            let measurement_table = format_measurement_table(&measurements);
            select_deck_fourier_cards_for_analysis(netlist, "tf")?;
            let fourier = Vec::new();
            let fourier_table = format_deck_fourier_table(&fourier);
            let output_probes = vec![format!("V({output_node})")];
            let output_probe_lines = Vec::new();
            let output_directives = Vec::new();
            let output_directive_analysis_kinds = Vec::new();
            let output_directive_lines = Vec::new();
            let table = format_deck_tf_table(&result);
            let run_artifacts = deck_run_artifacts(
                &plan,
                1,
                &deck_table_columns(&table),
                &output_probes,
                &output_directives,
                &measurements,
                &fourier,
                &control_lines,
                &write_markers,
                &rawfile_options,
                &diagnostic_codes,
                &control_policy_artifacts,
                &deck_analysis_kinds,
                &deck_analysis_directives,
            );
            let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
            let tables = deck_stable_tables(&measurements, &fourier, &control_policy_artifacts);
            let table_artifacts = deck_table_artifacts(
                &plan,
                &table,
                &measurement_table,
                &fourier_table,
                &run_artifact_table,
                &measurements,
                &fourier,
                &control_policy_artifacts,
                &control_policy_artifact_table,
                &control_policy_summary_artifacts,
                &control_policy_summary_artifact_table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            let rawfile_artifacts =
                deck_rawfile_artifacts(&plan, &table, &write_markers, &rawfile_options);
            let rawfile_artifact_table = format_deck_rawfile_artifact_table(&rawfile_artifacts);
            let rawfile_artifact_csv = format_deck_rawfile_artifact_csv(&rawfile_artifacts);
            let rawfile_artifact_json = format_deck_rawfile_artifact_json(&rawfile_artifacts);
            let rawfile_artifact_records = deck_rawfile_artifact_records(&rawfile_artifacts);
            let wrdata_artifacts = deck_wrdata_artifacts(&table, &write_markers, &rawfile_options);
            let wrdata_artifact_table = format_deck_wrdata_artifact_table(&wrdata_artifacts);
            let wrdata_artifact_csv = format_deck_wrdata_artifact_csv(&wrdata_artifacts);
            let wrdata_artifact_json = format_deck_wrdata_artifact_json(&wrdata_artifacts);
            let wrdata_artifact_records = deck_wrdata_artifact_records(&wrdata_artifacts);
            let (
                output_plan_artifacts,
                output_plan_artifact_table,
                output_plan_artifact_csv,
                output_plan_artifact_json,
                output_plan_artifact_records,
            ) = deck_output_plan_artifact_bundle(
                &plan,
                &table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            Ok(DeckAnalysisExecution {
                plan,
                result: DeckAnalysisExecutionResult::Tf(result.clone()),
                table,
                output_probes,
                output_directives,
                analysis_directives,
                deck_analysis_kind_count: deck_analysis_kinds.len(),
                deck_analysis_kinds,
                deck_analysis_directive_count: deck_analysis_directives.len(),
                deck_analysis_directives,

                output_plan_artifact_count: output_plan_artifacts.len(),

                output_plan_artifacts,

                output_plan_artifact_table,

                output_plan_artifact_csv,

                output_plan_artifact_json,

                output_plan_artifact_records,
                control_line_count: control_lines.len(),
                control_lines: control_lines.clone(),
                write_marker_count: write_markers.len(),
                write_markers: write_markers.clone(),
                rawfile_option_count: rawfile_options.len(),
                rawfile_options: rawfile_options.clone(),
                control_policy_artifact_count: control_policy_artifacts.len(),
                control_policy_artifacts: control_policy_artifacts.clone(),
                control_policy_artifact_table: control_policy_artifact_table.clone(),
                control_policy_artifact_csv: control_policy_artifact_csv.clone(),
                control_policy_artifact_json: control_policy_artifact_json.clone(),
                control_policy_artifact_records: control_policy_artifact_records.clone(),
                control_policy_summary_artifact_count: control_policy_summary_artifacts.len(),
                control_policy_summary_artifacts: control_policy_summary_artifacts.clone(),
                control_policy_summary_artifact_table: control_policy_summary_artifact_table
                    .clone(),
                control_policy_summary_artifact_csv: control_policy_summary_artifact_csv.clone(),
                control_policy_summary_artifact_json: control_policy_summary_artifact_json.clone(),
                control_policy_summary_artifact_records: control_policy_summary_artifact_records
                    .clone(),
                rawfile_artifact_count: rawfile_artifacts.len(),
                rawfile_artifacts,
                rawfile_artifact_table,
                rawfile_artifact_csv,
                rawfile_artifact_json,
                rawfile_artifact_records,
                wrdata_artifact_count: wrdata_artifacts.len(),
                wrdata_artifacts,
                wrdata_artifact_table,
                wrdata_artifact_csv,
                wrdata_artifact_json,
                wrdata_artifact_records,
                diagnostic_count: diagnostic_codes.len(),
                diagnostic_codes: diagnostic_codes.clone(),
                table_count: tables.len(),
                tables,
                table_artifacts,
                measurements,
                measurement_table,
                fourier,
                fourier_table,
                run_artifacts,
                run_artifact_table,
            })
        }
        "sens" => {
            let output_node =
                require_deck_plan_string(plan.output_node.as_deref(), &plan, "output_node")?;
            let result = sens_dc(circuit, output_node)?;
            select_deck_measurement_cards_for_analysis(netlist, "sens")?;
            let measurements = Vec::new();
            let measurement_table = format_measurement_table(&measurements);
            select_deck_fourier_cards_for_analysis(netlist, "sens")?;
            let fourier = Vec::new();
            let fourier_table = format_deck_fourier_table(&fourier);
            let output_probes = vec![format!("V({output_node})")];
            let output_probe_lines = Vec::new();
            let output_directives = Vec::new();
            let output_directive_analysis_kinds = Vec::new();
            let output_directive_lines = Vec::new();
            let table = format_deck_sens_table(&result);
            let run_artifacts = deck_run_artifacts(
                &plan,
                1,
                &deck_table_columns(&table),
                &output_probes,
                &output_directives,
                &measurements,
                &fourier,
                &control_lines,
                &write_markers,
                &rawfile_options,
                &diagnostic_codes,
                &control_policy_artifacts,
                &deck_analysis_kinds,
                &deck_analysis_directives,
            );
            let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
            let tables = deck_stable_tables(&measurements, &fourier, &control_policy_artifacts);
            let table_artifacts = deck_table_artifacts(
                &plan,
                &table,
                &measurement_table,
                &fourier_table,
                &run_artifact_table,
                &measurements,
                &fourier,
                &control_policy_artifacts,
                &control_policy_artifact_table,
                &control_policy_summary_artifacts,
                &control_policy_summary_artifact_table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            let rawfile_artifacts =
                deck_rawfile_artifacts(&plan, &table, &write_markers, &rawfile_options);
            let rawfile_artifact_table = format_deck_rawfile_artifact_table(&rawfile_artifacts);
            let rawfile_artifact_csv = format_deck_rawfile_artifact_csv(&rawfile_artifacts);
            let rawfile_artifact_json = format_deck_rawfile_artifact_json(&rawfile_artifacts);
            let rawfile_artifact_records = deck_rawfile_artifact_records(&rawfile_artifacts);
            let wrdata_artifacts = deck_wrdata_artifacts(&table, &write_markers, &rawfile_options);
            let wrdata_artifact_table = format_deck_wrdata_artifact_table(&wrdata_artifacts);
            let wrdata_artifact_csv = format_deck_wrdata_artifact_csv(&wrdata_artifacts);
            let wrdata_artifact_json = format_deck_wrdata_artifact_json(&wrdata_artifacts);
            let wrdata_artifact_records = deck_wrdata_artifact_records(&wrdata_artifacts);
            let (
                output_plan_artifacts,
                output_plan_artifact_table,
                output_plan_artifact_csv,
                output_plan_artifact_json,
                output_plan_artifact_records,
            ) = deck_output_plan_artifact_bundle(
                &plan,
                &table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            Ok(DeckAnalysisExecution {
                plan,
                result: DeckAnalysisExecutionResult::Sens(result.clone()),
                table,
                output_probes,
                output_directives,
                analysis_directives,
                deck_analysis_kind_count: deck_analysis_kinds.len(),
                deck_analysis_kinds,
                deck_analysis_directive_count: deck_analysis_directives.len(),
                deck_analysis_directives,

                output_plan_artifact_count: output_plan_artifacts.len(),

                output_plan_artifacts,

                output_plan_artifact_table,

                output_plan_artifact_csv,

                output_plan_artifact_json,

                output_plan_artifact_records,
                control_line_count: control_lines.len(),
                control_lines: control_lines.clone(),
                write_marker_count: write_markers.len(),
                write_markers: write_markers.clone(),
                rawfile_option_count: rawfile_options.len(),
                rawfile_options: rawfile_options.clone(),
                control_policy_artifact_count: control_policy_artifacts.len(),
                control_policy_artifacts: control_policy_artifacts.clone(),
                control_policy_artifact_table: control_policy_artifact_table.clone(),
                control_policy_artifact_csv: control_policy_artifact_csv.clone(),
                control_policy_artifact_json: control_policy_artifact_json.clone(),
                control_policy_artifact_records: control_policy_artifact_records.clone(),
                control_policy_summary_artifact_count: control_policy_summary_artifacts.len(),
                control_policy_summary_artifacts: control_policy_summary_artifacts.clone(),
                control_policy_summary_artifact_table: control_policy_summary_artifact_table
                    .clone(),
                control_policy_summary_artifact_csv: control_policy_summary_artifact_csv.clone(),
                control_policy_summary_artifact_json: control_policy_summary_artifact_json.clone(),
                control_policy_summary_artifact_records: control_policy_summary_artifact_records
                    .clone(),
                rawfile_artifact_count: rawfile_artifacts.len(),
                rawfile_artifacts,
                rawfile_artifact_table,
                rawfile_artifact_csv,
                rawfile_artifact_json,
                rawfile_artifact_records,
                wrdata_artifact_count: wrdata_artifacts.len(),
                wrdata_artifacts,
                wrdata_artifact_table,
                wrdata_artifact_csv,
                wrdata_artifact_json,
                wrdata_artifact_records,
                diagnostic_count: diagnostic_codes.len(),
                diagnostic_codes: diagnostic_codes.clone(),
                table_count: tables.len(),
                tables,
                table_artifacts,
                measurements,
                measurement_table,
                fourier,
                fourier_table,
                run_artifacts,
                run_artifact_table,
            })
        }
        "noise" => {
            let output_node =
                require_deck_plan_string(plan.output_node.as_deref(), &plan, "output_node")?;
            let input_source =
                require_deck_plan_string(plan.source_name.as_deref(), &plan, "source_name")?;
            let frequencies = if let Some(sweep_kind) = plan.sweep_kind.as_deref() {
                let point_count = require_deck_plan_usize(plan.point_count, &plan, "point_count")?;
                let start =
                    require_deck_plan_number(plan.start_frequency_hz, &plan, "start_frequency_hz")?;
                let stop =
                    require_deck_plan_number(plan.stop_frequency_hz, &plan, "stop_frequency_hz")?;
                deck_ac_frequencies(&plan, sweep_kind, point_count, start, stop)?
            } else {
                Vec::new()
            };
            let result = noise_ac(circuit, output_node, input_source, &frequencies, 300.0)?;
            select_deck_measurement_cards_for_analysis(netlist, "noise")?;
            let measurements = Vec::new();
            let measurement_table = format_measurement_table(&measurements);
            select_deck_fourier_cards_for_analysis(netlist, "noise")?;
            let fourier = Vec::new();
            let fourier_table = format_deck_fourier_table(&fourier);
            let output_probes = vec![format!("V({output_node})")];
            let output_probe_lines = Vec::new();
            let output_directives = Vec::new();
            let output_directive_analysis_kinds = Vec::new();
            let output_directive_lines = Vec::new();
            let table = format_deck_noise_table(&result);
            let run_artifacts = deck_run_artifacts(
                &plan,
                result.points.len(),
                &deck_table_columns(&table),
                &output_probes,
                &output_directives,
                &measurements,
                &fourier,
                &control_lines,
                &write_markers,
                &rawfile_options,
                &diagnostic_codes,
                &control_policy_artifacts,
                &deck_analysis_kinds,
                &deck_analysis_directives,
            );
            let run_artifact_table = format_deck_run_artifact_table(&run_artifacts);
            let tables = deck_stable_tables(&measurements, &fourier, &control_policy_artifacts);
            let table_artifacts = deck_table_artifacts(
                &plan,
                &table,
                &measurement_table,
                &fourier_table,
                &run_artifact_table,
                &measurements,
                &fourier,
                &control_policy_artifacts,
                &control_policy_artifact_table,
                &control_policy_summary_artifacts,
                &control_policy_summary_artifact_table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            let rawfile_artifacts =
                deck_rawfile_artifacts(&plan, &table, &write_markers, &rawfile_options);
            let rawfile_artifact_table = format_deck_rawfile_artifact_table(&rawfile_artifacts);
            let rawfile_artifact_csv = format_deck_rawfile_artifact_csv(&rawfile_artifacts);
            let rawfile_artifact_json = format_deck_rawfile_artifact_json(&rawfile_artifacts);
            let rawfile_artifact_records = deck_rawfile_artifact_records(&rawfile_artifacts);
            let wrdata_artifacts = deck_wrdata_artifacts(&table, &write_markers, &rawfile_options);
            let wrdata_artifact_table = format_deck_wrdata_artifact_table(&wrdata_artifacts);
            let wrdata_artifact_csv = format_deck_wrdata_artifact_csv(&wrdata_artifacts);
            let wrdata_artifact_json = format_deck_wrdata_artifact_json(&wrdata_artifacts);
            let wrdata_artifact_records = deck_wrdata_artifact_records(&wrdata_artifacts);
            let (
                output_plan_artifacts,
                output_plan_artifact_table,
                output_plan_artifact_csv,
                output_plan_artifact_json,
                output_plan_artifact_records,
            ) = deck_output_plan_artifact_bundle(
                &plan,
                &table,
                &output_probes,
                &output_probe_lines,
                &output_directives,
                &output_directive_analysis_kinds,
                &output_directive_lines,
                &tables,
            );
            Ok(DeckAnalysisExecution {
                plan,
                result: DeckAnalysisExecutionResult::Noise(result.clone()),
                table,
                output_probes,
                output_directives,
                analysis_directives,
                deck_analysis_kind_count: deck_analysis_kinds.len(),
                deck_analysis_kinds,
                deck_analysis_directive_count: deck_analysis_directives.len(),
                deck_analysis_directives,

                output_plan_artifact_count: output_plan_artifacts.len(),

                output_plan_artifacts,

                output_plan_artifact_table,

                output_plan_artifact_csv,

                output_plan_artifact_json,

                output_plan_artifact_records,
                control_line_count: control_lines.len(),
                control_lines: control_lines.clone(),
                write_marker_count: write_markers.len(),
                write_markers: write_markers.clone(),
                rawfile_option_count: rawfile_options.len(),
                rawfile_options: rawfile_options.clone(),
                control_policy_artifact_count: control_policy_artifacts.len(),
                control_policy_artifacts: control_policy_artifacts.clone(),
                control_policy_artifact_table: control_policy_artifact_table.clone(),
                control_policy_artifact_csv: control_policy_artifact_csv.clone(),
                control_policy_artifact_json: control_policy_artifact_json.clone(),
                control_policy_artifact_records: control_policy_artifact_records.clone(),
                control_policy_summary_artifact_count: control_policy_summary_artifacts.len(),
                control_policy_summary_artifacts: control_policy_summary_artifacts.clone(),
                control_policy_summary_artifact_table: control_policy_summary_artifact_table
                    .clone(),
                control_policy_summary_artifact_csv: control_policy_summary_artifact_csv.clone(),
                control_policy_summary_artifact_json: control_policy_summary_artifact_json.clone(),
                control_policy_summary_artifact_records: control_policy_summary_artifact_records
                    .clone(),
                rawfile_artifact_count: rawfile_artifacts.len(),
                rawfile_artifacts,
                rawfile_artifact_table,
                rawfile_artifact_csv,
                rawfile_artifact_json,
                rawfile_artifact_records,
                wrdata_artifact_count: wrdata_artifacts.len(),
                wrdata_artifacts,
                wrdata_artifact_table,
                wrdata_artifact_csv,
                wrdata_artifact_json,
                wrdata_artifact_records,
                diagnostic_count: diagnostic_codes.len(),
                diagnostic_codes: diagnostic_codes.clone(),
                table_count: tables.len(),
                tables,
                table_artifacts,
                measurements,
                measurement_table,
                fourier,
                fourier_table,
                run_artifacts,
                run_artifact_table,
            })
        }
        _ => Err(SpiceError::InvalidElement {
            name: "run_deck_analysis".to_string(),
            reason: format!("unsupported analysis {:?}", plan.analysis),
        }),
    }
}

fn require_deck_plan_string<'a>(
    value: Option<&'a str>,
    plan: &DeckAnalysisPlan,
    field_name: &str,
) -> Result<&'a str, SpiceError> {
    if let Some(value) = value {
        if !value.is_empty() {
            return Ok(value);
        }
    }
    Err(deck_plan_error(
        plan,
        format!("{} analysis missing {field_name}", plan.directive),
    ))
}

fn require_deck_plan_number(
    value: Option<f64>,
    plan: &DeckAnalysisPlan,
    field_name: &str,
) -> Result<f64, SpiceError> {
    match value {
        Some(value) if value.is_finite() => Ok(value),
        _ => Err(deck_plan_error(
            plan,
            format!("{} analysis missing {field_name}", plan.directive),
        )),
    }
}

fn require_deck_plan_usize(
    value: Option<usize>,
    plan: &DeckAnalysisPlan,
    field_name: &str,
) -> Result<usize, SpiceError> {
    value.ok_or_else(|| {
        deck_plan_error(
            plan,
            format!("{} analysis missing {field_name}", plan.directive),
        )
    })
}

fn sample_transient_points_print_step(
    points: Vec<TransientPoint>,
    print_step: f64,
    start_time: Option<f64>,
    stop_time: f64,
) -> Result<Vec<TransientPoint>, SpiceError> {
    if points.is_empty() {
        return Ok(points);
    }
    let epsilon = stop_time.abs().max(print_step.abs()).max(1.0) * 1.0e-12;
    let report_start = if let Some(start_time) = start_time.filter(|value| *value > 0.0) {
        start_time
    } else if points[0].time.abs() <= epsilon {
        0.0
    } else {
        print_step
    };
    let mut sampled = Vec::new();
    let mut index = 0usize;
    loop {
        let sample_time = report_start + index as f64 * print_step;
        if sample_time > stop_time + epsilon {
            break;
        }
        sampled.push(interpolate_transient_point(&points, sample_time)?);
        index += 1;
    }
    Ok(sampled)
}

fn interpolate_transient_point(
    points: &[TransientPoint],
    time: f64,
) -> Result<TransientPoint, SpiceError> {
    let epsilon = time.abs().max(1.0) * 1.0e-12;
    for point in points {
        if (point.time - time).abs() <= epsilon {
            return Ok(TransientPoint {
                time,
                node_voltages: point.node_voltages.clone(),
                branch_currents: point.branch_currents.clone(),
            });
        }
    }
    for pair in points.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.time - epsilon <= time && time <= right.time + epsilon {
            let span = right.time - left.time;
            if span <= 0.0 {
                return Ok(TransientPoint {
                    time,
                    node_voltages: left.node_voltages.clone(),
                    branch_currents: left.branch_currents.clone(),
                });
            }
            let alpha = (time - left.time) / span;
            return Ok(TransientPoint {
                time,
                node_voltages: interpolate_value_map(
                    &left.node_voltages,
                    &right.node_voltages,
                    alpha,
                ),
                branch_currents: interpolate_value_map(
                    &left.branch_currents,
                    &right.branch_currents,
                    alpha,
                ),
            });
        }
    }
    Err(SpiceError::InvalidElement {
        name: "run_deck_analysis".to_string(),
        reason: "transient print point is outside output".to_string(),
    })
}

fn interpolate_value_map(
    left: &BTreeMap<String, f64>,
    right: &BTreeMap<String, f64>,
    alpha: f64,
) -> BTreeMap<String, f64> {
    let mut values = BTreeMap::new();
    for key in left
        .keys()
        .chain(right.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
    {
        let left_value = left
            .get(&key)
            .copied()
            .or_else(|| right.get(&key).copied())
            .unwrap_or(0.0);
        let right_value = right.get(&key).copied().unwrap_or(left_value);
        values.insert(key, (1.0 - alpha) * left_value + alpha * right_value);
    }
    values
}

fn deck_plan_error(plan: &DeckAnalysisPlan, reason: String) -> SpiceError {
    SpiceError::InvalidElement {
        name: "run_deck_analysis".to_string(),
        reason: format!("line {}: {reason}", plan.line_number),
    }
}

fn run_deck_ac_sweep(
    circuit: &Circuit,
    plan: &DeckAnalysisPlan,
    sweep_kind: &str,
    point_count: usize,
    start_hz: f64,
    stop_hz: f64,
) -> Result<Vec<AcPoint>, SpiceError> {
    let mut points = Vec::new();
    for frequency_hz in deck_ac_frequencies(plan, sweep_kind, point_count, start_hz, stop_hz)? {
        let point = ac_sweep(circuit, frequency_hz, frequency_hz, 1)?
            .into_iter()
            .next()
            .ok_or(SpiceError::SingularMatrix)?;
        points.push(point);
    }
    Ok(points)
}

fn deck_ac_frequencies(
    plan: &DeckAnalysisPlan,
    sweep_kind: &str,
    point_count: usize,
    start_hz: f64,
    stop_hz: f64,
) -> Result<Vec<f64>, SpiceError> {
    if point_count == 0 {
        return Err(deck_plan_error(
            plan,
            ".ac point_count must be positive".to_string(),
        ));
    }
    match sweep_kind {
        "lin" => {
            if point_count == 1 {
                return Ok(vec![start_hz]);
            }
            let step = (stop_hz - start_hz) / (point_count - 1) as f64;
            Ok((0..point_count)
                .map(|index| start_hz + index as f64 * step)
                .collect())
        }
        "dec" | "oct" => {
            let base = if sweep_kind == "dec" {
                10.0_f64
            } else {
                2.0_f64
            };
            let ratio = base.powf(1.0 / point_count as f64);
            let epsilon = stop_hz * 1.0e-12;
            let mut frequencies = Vec::new();
            let mut frequency_hz = start_hz;
            while frequency_hz <= stop_hz + epsilon {
                frequencies.push(frequency_hz);
                frequency_hz *= ratio;
            }
            Ok(frequencies)
        }
        _ => Err(deck_plan_error(
            plan,
            format!(
                ".ac {} execution is not supported yet",
                sweep_kind.to_ascii_uppercase()
            ),
        )),
    }
}

pub fn format_corner_ac_table(
    result: &CornerAcSweepResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .first()
            .map(|corner| default_ac_output_probes(&corner.points))
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows =
        vec!["Corner\tIndex\tFrequency\tProbe\tReal\tImaginary\tMagnitude\tPhase".to_string()];
    for corner in &result.points {
        for (index, point) in corner.points.iter().enumerate() {
            for probe in &selected_probes {
                let value = table_complex_probe_value(
                    &point.node_voltages,
                    &point.branch_currents,
                    probe,
                    "format_corner_ac_table",
                )?;
                rows.push(format!(
                    "{}\t{index}\t{}\t{}\t{}\t{}\t{}\t{}",
                    corner.corner_name,
                    format_table_number(point.frequency_hz),
                    probe,
                    format_table_number(value.real),
                    format_table_number(value.imag),
                    format_table_number(value.abs()),
                    format_table_number(value.phase().to_degrees())
                ));
            }
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_tf_table(result: &TfResult) -> String {
    format!(
        "TransferRatio\tInputImpedance\tOutputImpedance\n{}\t{}\t{}\n",
        format_table_number(result.transfer_ratio),
        format_table_number(result.input_impedance_ohms),
        format_table_number(result.output_impedance_ohms)
    )
}

pub fn format_corner_tf_table(result: &CornerTfResult) -> String {
    let mut rows = vec!["Corner\tTransferRatio\tInputImpedance\tOutputImpedance".to_string()];
    for point in &result.points {
        rows.push(format!(
            "{}\t{}\t{}\t{}",
            point.corner_name,
            format_table_number(point.result.transfer_ratio),
            format_table_number(point.result.input_impedance_ohms),
            format_table_number(point.result.output_impedance_ohms)
        ));
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_s_parameter_table(result: &SParameterResult) -> String {
    let mut rows = vec![
        "Index\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase".to_string(),
    ];
    for (index, point) in result.points.iter().enumerate() {
        for (parameter, value) in [
            ("S11", point.s11),
            ("S21", point.s21),
            ("S12", point.s12),
            ("S22", point.s22),
        ] {
            rows.push(format!(
                "{index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                format_table_number(point.frequency_hz),
                result.port1_source,
                result.port2_source,
                parameter,
                format_table_number(value.real),
                format_table_number(value.imag),
                format_table_number(value.abs()),
                format_table_number(value.phase().to_degrees())
            ));
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_corner_s_parameter_table(result: &CornerSParameterResult) -> String {
    let mut rows = vec![
        "Corner\tIndex\tFrequency\tPort1\tPort2\tParameter\tReal\tImaginary\tMagnitude\tPhase"
            .to_string(),
    ];
    for point in &result.points {
        for (index, s_parameter_point) in point.result.points.iter().enumerate() {
            for (parameter, value) in [
                ("S11", s_parameter_point.s11),
                ("S21", s_parameter_point.s21),
                ("S12", s_parameter_point.s12),
                ("S22", s_parameter_point.s22),
            ] {
                rows.push(format!(
                    "{}\t{index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    point.corner_name,
                    format_table_number(s_parameter_point.frequency_hz),
                    result.port1_source,
                    result.port2_source,
                    parameter,
                    format_table_number(value.real),
                    format_table_number(value.imag),
                    format_table_number(value.abs()),
                    format_table_number(value.phase().to_degrees())
                ));
            }
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_noise_table(result: &NoiseResult) -> String {
    let mut rows = vec![
        "Index\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD"
            .to_string(),
    ];
    for (index, point) in result.points.iter().enumerate() {
        for entry in &point.entries {
            rows.push(format!(
                "{index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                format_table_number(point.frequency_hz),
                result.output_node,
                result.input_source,
                format_table_number(point.output_psd),
                format_table_number(point.input_referred_psd),
                entry.element_name,
                format_noise_type(entry.noise_type),
                format_table_number(entry.source_psd),
                format_table_number(entry.output_psd)
            ));
        }
        if point.entries.is_empty() {
            rows.push(format!(
                "{index}\t{}\t{}\t{}\t{}\t{}\t\t\t\t",
                format_table_number(point.frequency_hz),
                result.output_node,
                result.input_source,
                format_table_number(point.output_psd),
                format_table_number(point.input_referred_psd)
            ));
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_corner_noise_table(result: &CornerNoiseResult) -> String {
    let mut rows = vec![
        "Corner\tIndex\tFrequency\tOutputNode\tInputSource\tOutputPSD\tInputReferredPSD\tElement\tType\tSourcePSD\tContributionPSD"
            .to_string(),
    ];
    for point in &result.points {
        for (index, noise_point) in point.result.points.iter().enumerate() {
            for entry in &noise_point.entries {
                rows.push(format!(
                    "{}\t{index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    point.corner_name,
                    format_table_number(noise_point.frequency_hz),
                    result.output_node,
                    result.input_source,
                    format_table_number(noise_point.output_psd),
                    format_table_number(noise_point.input_referred_psd),
                    entry.element_name,
                    format_noise_type(entry.noise_type),
                    format_table_number(entry.source_psd),
                    format_table_number(entry.output_psd)
                ));
            }
            if noise_point.entries.is_empty() {
                rows.push(format!(
                    "{}\t{index}\t{}\t{}\t{}\t{}\t{}\t\t\t\t",
                    point.corner_name,
                    format_table_number(noise_point.frequency_hz),
                    result.output_node,
                    result.input_source,
                    format_table_number(noise_point.output_psd),
                    format_table_number(noise_point.input_referred_psd)
                ));
            }
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_sens_table(result: &SensResult) -> String {
    let mut rows = vec![
        "OutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity"
            .to_string(),
    ];
    for entry in &result.entries {
        rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            result.output_node,
            format_table_number(result.nominal_voltage),
            entry.element_name,
            entry.parameter,
            format_table_number(entry.nominal_value),
            format_table_number(entry.sensitivity),
            format_table_number(entry.relative_sensitivity)
        ));
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_corner_sens_table(result: &CornerSensResult) -> String {
    let mut rows = vec![
        "Corner\tOutputNode\tNominalVoltage\tElement\tParameter\tNominalValue\tSensitivity\tRelativeSensitivity"
            .to_string(),
    ];
    for point in &result.points {
        for entry in &point.result.entries {
            rows.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                point.corner_name,
                result.output_node,
                format_table_number(point.result.nominal_voltage),
                entry.element_name,
                entry.parameter,
                format_table_number(entry.nominal_value),
                format_table_number(entry.sensitivity),
                format_table_number(entry.relative_sensitivity)
            ));
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_pss_table(result: &PssResult, probes: &[&str]) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        default_transient_output_probes(&result.steady_state)
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Index\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\t{}",
        selected_probes.join("\t")
    )];
    for (index, point) in result.steady_state.iter().enumerate() {
        let values: Result<Vec<String>, SpiceError> = selected_probes
            .iter()
            .map(|probe| {
                table_probe_value(
                    &point.node_voltages,
                    &point.branch_currents,
                    probe,
                    "format_pss_table",
                )
                .map(format_table_number)
            })
            .collect();
        rows.push(format!(
            "{index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            format_table_number(result.period_seconds),
            format_table_number(result.time_step_seconds),
            result.converged,
            result.solve.iteration_count,
            format_table_number(result.solve.final_residual.residual_l2_norm),
            format_table_number(point.time),
            values?.join("\t")
        ));
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_corner_pss_table(
    result: &CornerPssResult,
    probes: &[&str],
) -> Result<String, SpiceError> {
    let selected_probes = if probes.is_empty() {
        result
            .points
            .first()
            .map(|point| default_transient_output_probes(&point.result.steady_state))
            .unwrap_or_default()
    } else {
        probes.iter().map(|probe| probe.to_string()).collect()
    };
    let mut rows = vec![format!(
        "Corner\tIndex\tPeriod\tTimeStep\tConverged\tIterations\tResidualL2\tTime\t{}",
        selected_probes.join("\t")
    )];
    for point in &result.points {
        for (index, sample) in point.result.steady_state.iter().enumerate() {
            let values: Result<Vec<String>, SpiceError> = selected_probes
                .iter()
                .map(|probe| {
                    table_probe_value(
                        &sample.node_voltages,
                        &sample.branch_currents,
                        probe,
                        "format_corner_pss_table",
                    )
                    .map(format_table_number)
                })
                .collect();
            rows.push(format!(
                "{}\t{index}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                point.corner_name,
                format_table_number(point.result.period_seconds),
                format_table_number(point.result.time_step_seconds),
                point.result.converged,
                point.result.solve.iteration_count,
                format_table_number(point.result.solve.final_residual.residual_l2_norm),
                format_table_number(sample.time),
                values?.join("\t")
            ));
        }
    }
    rows.push(String::new());
    Ok(rows.join("\n"))
}

pub fn format_pole_zero_table(result: &PoleZeroResult) -> String {
    let mut rows = vec!["Index\tKind\tReal\tImaginary\tFrequency\tDamping".to_string()];
    for (index, entry) in result.entries.iter().enumerate() {
        let kind = match entry.kind {
            PoleZeroEntryKind::Pole => "pole",
            PoleZeroEntryKind::Zero => "zero",
        };
        rows.push(format!(
            "{index}\t{}\t{}\t{}\t{}\t{}",
            kind,
            format_table_number(entry.real),
            format_table_number(entry.imaginary),
            format_table_number(entry.frequency_hz),
            format_table_number(entry.damping)
        ));
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_corner_pole_zero_table(result: &CornerPoleZeroResult) -> String {
    let mut rows = vec!["Corner\tIndex\tKind\tReal\tImaginary\tFrequency\tDamping".to_string()];
    for point in &result.points {
        for (index, entry) in point.result.entries.iter().enumerate() {
            let kind = match entry.kind {
                PoleZeroEntryKind::Pole => "pole",
                PoleZeroEntryKind::Zero => "zero",
            };
            rows.push(format!(
                "{}\t{index}\t{}\t{}\t{}\t{}\t{}",
                point.corner_name,
                kind,
                format_table_number(entry.real),
                format_table_number(entry.imaginary),
                format_table_number(entry.frequency_hz),
                format_table_number(entry.damping)
            ));
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_distortion_table(result: &DistortionResult) -> String {
    let mut rows = vec!["Frequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD".to_string()];
    for point in &result.points {
        for harmonic in &point.harmonics {
            rows.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                format_table_number(point.frequency_hz),
                result.input_source,
                result.output_probe,
                harmonic.harmonic,
                format_table_number(harmonic.magnitude),
                format_table_number(harmonic.phase_degrees),
                format_table_number(point.total_harmonic_distortion)
            ));
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_corner_distortion_table(result: &CornerDistortionResult) -> String {
    let mut rows =
        vec!["Corner\tFrequency\tInput\tOutput\tHarmonic\tMagnitude\tPhase\tTHD".to_string()];
    for point in &result.points {
        for distortion_point in &point.result.points {
            for harmonic in &distortion_point.harmonics {
                rows.push(format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    point.corner_name,
                    format_table_number(distortion_point.frequency_hz),
                    result.input_source,
                    result.output_probe,
                    harmonic.harmonic,
                    format_table_number(harmonic.magnitude),
                    format_table_number(harmonic.phase_degrees),
                    format_table_number(distortion_point.total_harmonic_distortion)
                ));
            }
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_fourier_table(result: &FourierResult) -> String {
    let mut rows =
        vec!["Probe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD".to_string()];
    for probe in &result.probes {
        for harmonic in &probe.harmonics {
            rows.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                probe.probe,
                harmonic.harmonic,
                format_table_number(harmonic.frequency_hz),
                format_table_number(harmonic.cosine),
                format_table_number(harmonic.sine),
                format_table_number(harmonic.magnitude),
                format_table_number(harmonic.phase_degrees),
                format_table_number(probe.dc),
                format_table_number(probe.total_harmonic_distortion)
            ));
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

pub fn format_deck_fourier_table(results: &[FourierResult]) -> String {
    results
        .iter()
        .map(format_fourier_table)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_corner_fourier_table(result: &CornerFourierResult) -> String {
    let mut rows = vec![
        "Corner\tProbe\tHarmonic\tFrequency\tCosine\tSine\tMagnitude\tPhase\tDC\tTHD".to_string(),
    ];
    for point in &result.points {
        for probe in &point.result.probes {
            for harmonic in &probe.harmonics {
                rows.push(format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    point.corner_name,
                    probe.probe,
                    harmonic.harmonic,
                    format_table_number(harmonic.frequency_hz),
                    format_table_number(harmonic.cosine),
                    format_table_number(harmonic.sine),
                    format_table_number(harmonic.magnitude),
                    format_table_number(harmonic.phase_degrees),
                    format_table_number(probe.dc),
                    format_table_number(probe.total_harmonic_distortion)
                ));
            }
        }
    }
    rows.push(String::new());
    rows.join("\n")
}

fn format_noise_type(noise_type: NoiseType) -> &'static str {
    match noise_type {
        NoiseType::Thermal => "thermal",
        NoiseType::Shot => "shot",
        NoiseType::Flicker => "flicker",
    }
}

fn default_output_probes(
    node_voltages: &BTreeMap<String, f64>,
    branch_currents: &BTreeMap<String, f64>,
) -> Vec<String> {
    node_voltages
        .keys()
        .map(|name| format!("V({name})"))
        .chain(branch_currents.keys().cloned())
        .collect()
}

fn default_transient_output_probes(points: &[TransientPoint]) -> Vec<String> {
    let mut node_names = BTreeSet::new();
    let mut branch_names = BTreeSet::new();
    for point in points {
        node_names.extend(point.node_voltages.keys().cloned());
        branch_names.extend(point.branch_currents.keys().cloned());
    }
    node_names
        .iter()
        .map(|name| format!("V({name})"))
        .chain(branch_names)
        .collect()
}

fn default_ac_output_probes(points: &[AcPoint]) -> Vec<String> {
    let mut node_names = BTreeSet::new();
    let mut branch_names = BTreeSet::new();
    for point in points {
        node_names.extend(point.node_voltages.keys().cloned());
        branch_names.extend(point.branch_currents.keys().cloned());
    }
    node_names
        .iter()
        .map(|name| format!("V({name})"))
        .chain(branch_names)
        .collect()
}

fn format_table_number(value: f64) -> String {
    let raw = format!("{value:.6e}");
    if let Some((mantissa, exponent_text)) = raw.split_once('e') {
        let exponent = exponent_text.parse::<i32>().unwrap_or(0);
        return format!("{mantissa}e{exponent:+03}");
    }
    raw
}

pub fn measure_transient_probe(
    points: &[TransientPoint],
    name: &str,
    probe: &str,
    mode: &str,
    from_time: Option<f64>,
    to_time: Option<f64>,
) -> Result<ProbeMeasurement, SpiceError> {
    let normalized_mode = normalize_measurement_mode(mode)?;
    if let Some(value) = from_time {
        if !value.is_finite() {
            return Err(table_error(
                "measure_transient_probe",
                "from_time must be finite",
            ));
        }
    }
    if let Some(value) = to_time {
        if !value.is_finite() {
            return Err(table_error(
                "measure_transient_probe",
                "to_time must be finite",
            ));
        }
    }
    if let (Some(from), Some(to)) = (from_time, to_time) {
        if from > to {
            return Err(table_error(
                "measure_transient_probe",
                "from_time must be <= to_time",
            ));
        }
    }

    let mut values = Vec::new();
    for point in points {
        if from_time.is_some_and(|from| point.time < from)
            || to_time.is_some_and(|to| point.time > to)
        {
            continue;
        }
        values.push(table_probe_value(
            &point.node_voltages,
            &point.branch_currents,
            probe,
            "measure_transient_probe",
        )?);
    }
    if values.is_empty() {
        return Err(table_error(
            "measure_transient_probe",
            "no transient samples in window",
        ));
    }

    Ok(ProbeMeasurement {
        name: name.to_string(),
        analysis: "tran".to_string(),
        probe: probe.to_string(),
        mode: normalized_mode.to_string(),
        value: measure_values(&values, normalized_mode)?,
        from_value: from_time,
        to_value: to_time,
    })
}

pub fn measure_transient_find_at_probe(
    points: &[TransientPoint],
    name: &str,
    probe: &str,
    at_time: f64,
) -> Result<ProbeMeasurement, SpiceError> {
    if !at_time.is_finite() {
        return Err(table_error(
            "measure_transient_find_at_probe",
            "at_time must be finite",
        ));
    }
    let value =
        transient_probe_value_at(points, probe, at_time, "measure_transient_find_at_probe")?;
    Ok(ProbeMeasurement {
        name: name.to_string(),
        analysis: "tran".to_string(),
        probe: probe.to_string(),
        mode: "find".to_string(),
        value,
        from_value: Some(at_time),
        to_value: Some(at_time),
    })
}

pub fn measure_transient_when_probe(
    points: &[TransientPoint],
    name: &str,
    probe: &str,
    target_value: f64,
    from_time: Option<f64>,
    to_time: Option<f64>,
) -> Result<ProbeMeasurement, SpiceError> {
    if !target_value.is_finite() {
        return Err(table_error(
            "measure_transient_when_probe",
            "target_value must be finite",
        ));
    }
    if let Some(value) = from_time {
        if !value.is_finite() {
            return Err(table_error(
                "measure_transient_when_probe",
                "from_time must be finite",
            ));
        }
    }
    if let Some(value) = to_time {
        if !value.is_finite() {
            return Err(table_error(
                "measure_transient_when_probe",
                "to_time must be finite",
            ));
        }
    }
    if let (Some(from), Some(to)) = (from_time, to_time) {
        if from > to {
            return Err(table_error(
                "measure_transient_when_probe",
                "from_time must be <= to_time",
            ));
        }
    }

    let value = transient_probe_crossing_time(
        points,
        probe,
        target_value,
        TransientCrossingKind::Cross,
        1,
        from_time,
        to_time,
        "measure_transient_when_probe",
    )?;
    Ok(ProbeMeasurement {
        name: name.to_string(),
        analysis: "tran".to_string(),
        probe: probe.to_string(),
        mode: "when".to_string(),
        value,
        from_value: from_time,
        to_value: to_time,
    })
}

pub fn measure_transient_when_probe_counted(
    points: &[TransientPoint],
    name: &str,
    probe: &str,
    target_value: f64,
    crossing_kind: &str,
    crossing_count: usize,
    from_time: Option<f64>,
    to_time: Option<f64>,
) -> Result<ProbeMeasurement, SpiceError> {
    let context = "measure_transient_when_probe_counted";
    if !target_value.is_finite() {
        return Err(table_error(context, "target_value must be finite"));
    }
    let crossing_kind = parse_transient_crossing_kind(crossing_kind, context)?;
    if crossing_count == 0 {
        return Err(table_error(
            context,
            "crossing_count must be a positive integer",
        ));
    }
    if let Some(value) = from_time {
        if !value.is_finite() {
            return Err(table_error(context, "from_time must be finite"));
        }
    }
    if let Some(value) = to_time {
        if !value.is_finite() {
            return Err(table_error(context, "to_time must be finite"));
        }
    }
    if let (Some(from), Some(to)) = (from_time, to_time) {
        if from > to {
            return Err(table_error(context, "from_time must be <= to_time"));
        }
    }

    let value = transient_probe_crossing_time(
        points,
        probe,
        target_value,
        crossing_kind,
        crossing_count,
        from_time,
        to_time,
        context,
    )?;
    Ok(ProbeMeasurement {
        name: name.to_string(),
        analysis: "tran".to_string(),
        probe: probe.to_string(),
        mode: "when".to_string(),
        value,
        from_value: from_time,
        to_value: to_time,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn measure_transient_delay_between_probes(
    points: &[TransientPoint],
    name: &str,
    trigger_probe: &str,
    trigger_value: f64,
    trigger_crossing_kind: &str,
    trigger_crossing_count: usize,
    target_probe: &str,
    target_value: f64,
    target_crossing_kind: &str,
    target_crossing_count: usize,
    from_time: Option<f64>,
    to_time: Option<f64>,
) -> Result<ProbeMeasurement, SpiceError> {
    let context = "measure_transient_delay_between_probes";
    if !trigger_value.is_finite() {
        return Err(table_error(context, "trigger_value must be finite"));
    }
    if !target_value.is_finite() {
        return Err(table_error(context, "target_value must be finite"));
    }
    let trigger_crossing_kind = parse_transient_crossing_kind(trigger_crossing_kind, context)?;
    let target_crossing_kind = parse_transient_crossing_kind(target_crossing_kind, context)?;
    if trigger_crossing_count == 0 || target_crossing_count == 0 {
        return Err(table_error(
            context,
            "crossing counts must be positive integers",
        ));
    }
    if let Some(value) = from_time {
        if !value.is_finite() {
            return Err(table_error(context, "from_time must be finite"));
        }
    }
    if let Some(value) = to_time {
        if !value.is_finite() {
            return Err(table_error(context, "to_time must be finite"));
        }
    }
    if let (Some(from), Some(to)) = (from_time, to_time) {
        if from > to {
            return Err(table_error(context, "from_time must be <= to_time"));
        }
    }

    let trigger_time = transient_probe_crossing_time(
        points,
        trigger_probe,
        trigger_value,
        trigger_crossing_kind,
        trigger_crossing_count,
        from_time,
        to_time,
        context,
    )?;
    let target_from_time = Some(from_time.map_or(trigger_time, |from| from.max(trigger_time)));
    let target_time = transient_probe_crossing_time(
        points,
        target_probe,
        target_value,
        target_crossing_kind,
        target_crossing_count,
        target_from_time,
        to_time,
        context,
    )?;
    Ok(ProbeMeasurement {
        name: name.to_string(),
        analysis: "tran".to_string(),
        probe: format!("{trigger_probe}->{target_probe}"),
        mode: "delay".to_string(),
        value: target_time - trigger_time,
        from_value: from_time,
        to_value: to_time,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransientCrossingKind {
    Rise,
    Fall,
    Cross,
}

fn parse_transient_crossing_kind(
    crossing_kind: &str,
    context: &str,
) -> Result<TransientCrossingKind, SpiceError> {
    match crossing_kind.trim().to_ascii_lowercase().as_str() {
        "rise" => Ok(TransientCrossingKind::Rise),
        "fall" => Ok(TransientCrossingKind::Fall),
        "cross" => Ok(TransientCrossingKind::Cross),
        _ => Err(table_error(
            context,
            "crossing_kind must be rise, fall, or cross",
        )),
    }
}

fn transient_probe_value_at(
    points: &[TransientPoint],
    probe: &str,
    at_time: f64,
    context: &str,
) -> Result<f64, SpiceError> {
    let mut previous: Option<(f64, f64)> = None;
    for point in points {
        let value =
            table_probe_value(&point.node_voltages, &point.branch_currents, probe, context)?;
        if point.time == at_time {
            return Ok(value);
        }
        if point.time > at_time {
            let Some((previous_time, previous_value)) = previous else {
                return Err(table_error(
                    context,
                    "at_time is outside transient sample range",
                ));
            };
            if point.time == previous_time {
                return Err(table_error(
                    context,
                    "duplicate transient sample times around AT value",
                ));
            }
            let fraction = (at_time - previous_time) / (point.time - previous_time);
            return Ok(previous_value + (value - previous_value) * fraction);
        }
        previous = Some((point.time, value));
    }
    Err(table_error(
        context,
        "at_time is outside transient sample range",
    ))
}

fn transient_probe_crossing_time(
    points: &[TransientPoint],
    probe: &str,
    target_value: f64,
    crossing_kind: TransientCrossingKind,
    crossing_count: usize,
    from_time: Option<f64>,
    to_time: Option<f64>,
    context: &str,
) -> Result<f64, SpiceError> {
    let mut previous: Option<(f64, f64, f64)> = None;
    let mut selected_count = 0usize;
    let mut matched_count = 0usize;
    for point in points {
        if from_time.is_some_and(|from| point.time < from)
            || to_time.is_some_and(|to| point.time > to)
        {
            continue;
        }
        selected_count += 1;
        let value =
            table_probe_value(&point.node_voltages, &point.branch_currents, probe, context)?;
        let delta = value - target_value;
        let crossing_time = if let Some((previous_time, previous_value, previous_delta)) = previous
        {
            if delta == 0.0 {
                match crossing_kind {
                    TransientCrossingKind::Cross => Some(point.time),
                    TransientCrossingKind::Rise if previous_delta < 0.0 => Some(point.time),
                    TransientCrossingKind::Fall if previous_delta > 0.0 => Some(point.time),
                    _ => None,
                }
            } else if (previous_delta < 0.0
                && delta > 0.0
                && crossing_kind != TransientCrossingKind::Fall)
                || (previous_delta > 0.0
                    && delta < 0.0
                    && crossing_kind != TransientCrossingKind::Rise)
            {
                if point.time == previous_time {
                    return Err(table_error(
                        context,
                        "duplicate transient sample times around WHEN crossing",
                    ));
                }
                let fraction = (target_value - previous_value) / (value - previous_value);
                Some(previous_time + (point.time - previous_time) * fraction)
            } else {
                None
            }
        } else if delta == 0.0 && crossing_kind == TransientCrossingKind::Cross {
            Some(point.time)
        } else {
            None
        };
        if let Some(crossing_time) = crossing_time {
            matched_count += 1;
            if matched_count == crossing_count {
                return Ok(crossing_time);
            }
        }
        previous = Some((point.time, value, delta));
    }
    if selected_count == 0 {
        return Err(table_error(context, "no transient samples in window"));
    }
    Err(table_error(context, "no transient crossing in window"))
}

pub fn measure_transient_cards(
    points: &[TransientPoint],
    measurements: &[DeckMeasurementCard],
) -> Result<Vec<ProbeMeasurement>, SpiceError> {
    let mut results = Vec::new();
    for measurement in measurements {
        if measurement.analysis != "tran" && measurement.analysis != "transient" {
            return Err(table_error(
                "measure_transient_cards",
                "only transient measurement cards are supported",
            ));
        }
        if measurement.mode == "find" {
            let Some(at_time) = measurement.at_value else {
                return Err(table_error(
                    "measure_transient_cards",
                    "FIND measurement cards require an AT value",
                ));
            };
            results.push(measure_transient_find_at_probe(
                points,
                &measurement.name,
                &measurement.probe,
                at_time,
            )?);
        } else if measurement.mode == "when" {
            let Some(target_value) = measurement.target_value else {
                return Err(table_error(
                    "measure_transient_cards",
                    "WHEN measurement cards require a target value",
                ));
            };
            results.push(measure_transient_when_probe_counted(
                points,
                &measurement.name,
                &measurement.probe,
                target_value,
                measurement.crossing_kind.as_deref().unwrap_or("cross"),
                measurement.crossing_count.unwrap_or(1),
                measurement.from_value,
                measurement.to_value,
            )?);
        } else if measurement.mode == "delay" {
            let Some(trigger_probe) = measurement.trigger_probe.as_deref() else {
                return Err(table_error(
                    "measure_transient_cards",
                    "delay measurement cards require a trigger probe",
                ));
            };
            let Some(trigger_value) = measurement.trigger_value else {
                return Err(table_error(
                    "measure_transient_cards",
                    "delay measurement cards require a trigger value",
                ));
            };
            let Some(target_value) = measurement.target_value else {
                return Err(table_error(
                    "measure_transient_cards",
                    "delay measurement cards require a target value",
                ));
            };
            results.push(measure_transient_delay_between_probes(
                points,
                &measurement.name,
                trigger_probe,
                trigger_value,
                measurement
                    .trigger_crossing_kind
                    .as_deref()
                    .unwrap_or("cross"),
                measurement.trigger_crossing_count.unwrap_or(1),
                &measurement.probe,
                target_value,
                measurement.crossing_kind.as_deref().unwrap_or("cross"),
                measurement.crossing_count.unwrap_or(1),
                measurement.from_value,
                measurement.to_value,
            )?);
        } else {
            results.push(measure_transient_probe(
                points,
                &measurement.name,
                &measurement.probe,
                &measurement.mode,
                measurement.from_value,
                measurement.to_value,
            )?);
        }
    }
    Ok(results)
}

pub fn measure_transient_deck(
    points: &[TransientPoint],
    netlist: &str,
) -> Result<Vec<ProbeMeasurement>, SpiceError> {
    let summary = resolve_deck_measurements(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "measure_transient_deck",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    measure_transient_cards(points, &summary.measurements)
}

pub fn measure_dc_sweep_probe(
    points: &[DcSweepPoint],
    name: &str,
    probe: &str,
    mode: &str,
    from_value: Option<f64>,
    to_value: Option<f64>,
) -> Result<ProbeMeasurement, SpiceError> {
    let normalized_mode = normalize_measurement_mode_with_context(mode, "measure_dc_sweep_probe")?;
    if let Some(value) = from_value {
        if !value.is_finite() {
            return Err(table_error(
                "measure_dc_sweep_probe",
                "from_value must be finite",
            ));
        }
    }
    if let Some(value) = to_value {
        if !value.is_finite() {
            return Err(table_error(
                "measure_dc_sweep_probe",
                "to_value must be finite",
            ));
        }
    }
    if let (Some(from), Some(to)) = (from_value, to_value) {
        if from > to {
            return Err(table_error(
                "measure_dc_sweep_probe",
                "from_value must be <= to_value",
            ));
        }
    }

    let mut values = Vec::new();
    for point in points {
        if from_value.is_some_and(|from| point.value < from)
            || to_value.is_some_and(|to| point.value > to)
        {
            continue;
        }
        values.push(table_probe_value(
            &point.result.node_voltages,
            &point.result.branch_currents,
            probe,
            "measure_dc_sweep_probe",
        )?);
    }
    if values.is_empty() {
        return Err(table_error(
            "measure_dc_sweep_probe",
            "no dc sweep samples in window",
        ));
    }

    Ok(ProbeMeasurement {
        name: name.to_string(),
        analysis: "dc".to_string(),
        probe: probe.to_string(),
        mode: normalized_mode.to_string(),
        value: measure_values_with_context(&values, normalized_mode, "measure_dc_sweep_probe")?,
        from_value,
        to_value,
    })
}

pub fn measure_dc_sweep_cards(
    points: &[DcSweepPoint],
    measurements: &[DeckMeasurementCard],
) -> Result<Vec<ProbeMeasurement>, SpiceError> {
    let mut results = Vec::new();
    for measurement in measurements {
        if measurement.analysis != "dc" {
            return Err(table_error(
                "measure_dc_sweep_cards",
                "only dc measurement cards are supported",
            ));
        }
        results.push(measure_dc_sweep_probe(
            points,
            &measurement.name,
            &measurement.probe,
            &measurement.mode,
            measurement.from_value,
            measurement.to_value,
        )?);
    }
    Ok(results)
}

pub fn measure_dc_sweep_deck(
    points: &[DcSweepPoint],
    netlist: &str,
) -> Result<Vec<ProbeMeasurement>, SpiceError> {
    let summary = resolve_deck_measurements(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "measure_dc_sweep_deck",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    measure_dc_sweep_cards(points, &summary.measurements)
}

pub fn measure_ac_sweep_probe(
    points: &[AcPoint],
    name: &str,
    probe: &str,
    mode: &str,
    from_frequency: Option<f64>,
    to_frequency: Option<f64>,
) -> Result<ProbeMeasurement, SpiceError> {
    let normalized_mode = normalize_measurement_mode_with_context(mode, "measure_ac_sweep_probe")?;
    if let Some(value) = from_frequency {
        if !value.is_finite() {
            return Err(table_error(
                "measure_ac_sweep_probe",
                "from_frequency must be finite",
            ));
        }
    }
    if let Some(value) = to_frequency {
        if !value.is_finite() {
            return Err(table_error(
                "measure_ac_sweep_probe",
                "to_frequency must be finite",
            ));
        }
    }
    if let (Some(from), Some(to)) = (from_frequency, to_frequency) {
        if from > to {
            return Err(table_error(
                "measure_ac_sweep_probe",
                "from_frequency must be <= to_frequency",
            ));
        }
    }

    let mut values = Vec::new();
    for point in points {
        if from_frequency.is_some_and(|from| point.frequency_hz < from)
            || to_frequency.is_some_and(|to| point.frequency_hz > to)
        {
            continue;
        }
        values.push(
            table_complex_probe_value(
                &point.node_voltages,
                &point.branch_currents,
                probe,
                "measure_ac_sweep_probe",
            )?
            .abs(),
        );
    }
    if values.is_empty() {
        return Err(table_error(
            "measure_ac_sweep_probe",
            "no ac sweep samples in window",
        ));
    }

    Ok(ProbeMeasurement {
        name: name.to_string(),
        analysis: "ac".to_string(),
        probe: probe.to_string(),
        mode: normalized_mode.to_string(),
        value: measure_values_with_context(&values, normalized_mode, "measure_ac_sweep_probe")?,
        from_value: from_frequency,
        to_value: to_frequency,
    })
}

pub fn measure_ac_sweep_cards(
    points: &[AcPoint],
    measurements: &[DeckMeasurementCard],
) -> Result<Vec<ProbeMeasurement>, SpiceError> {
    let mut results = Vec::new();
    for measurement in measurements {
        if measurement.analysis != "ac" {
            return Err(table_error(
                "measure_ac_sweep_cards",
                "only ac measurement cards are supported",
            ));
        }
        results.push(measure_ac_sweep_probe(
            points,
            &measurement.name,
            &measurement.probe,
            &measurement.mode,
            measurement.from_value,
            measurement.to_value,
        )?);
    }
    Ok(results)
}

pub fn measure_ac_sweep_deck(
    points: &[AcPoint],
    netlist: &str,
) -> Result<Vec<ProbeMeasurement>, SpiceError> {
    let summary = resolve_deck_measurements(netlist);
    if let Some(diagnostic) = summary.diagnostics.first() {
        return Err(table_error(
            "measure_ac_sweep_deck",
            &format!("line {}: {}", diagnostic.line_number, diagnostic.message),
        ));
    }
    measure_ac_sweep_cards(points, &summary.measurements)
}

pub fn format_measurement_table(measurements: &[ProbeMeasurement]) -> String {
    let mut rows = vec!["Name\tAnalysis\tProbe\tMode\tFrom\tTo\tValue".to_string()];
    for measurement in measurements {
        rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            measurement.name,
            measurement.analysis,
            measurement.probe,
            measurement.mode,
            format_optional_table_number(measurement.from_value),
            format_optional_table_number(measurement.to_value),
            format_table_number(measurement.value),
        ));
    }
    rows.push(String::new());
    rows.join("\n")
}

fn normalize_measurement_mode(mode: &str) -> Result<&'static str, SpiceError> {
    normalize_measurement_mode_with_context(mode, "measure_transient_probe")
}

fn normalize_measurement_mode_with_context(
    mode: &str,
    context: &str,
) -> Result<&'static str, SpiceError> {
    let normalized = mode.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "max" => Ok("max"),
        "min" => Ok("min"),
        "avg" | "average" | "mean" => Ok("avg"),
        "rms" | "root-mean-square" => Ok("rms"),
        "pp" | "p-p" | "p2p" | "peak-to-peak" | "peak2peak" => Ok("pp"),
        "last" | "final" => Ok("last"),
        _ => Err(table_error(context, &format!("unsupported mode {mode:?}"))),
    }
}

fn measure_values(values: &[f64], mode: &str) -> Result<f64, SpiceError> {
    measure_values_with_context(values, mode, "measure_transient_probe")
}

fn measure_values_with_context(
    values: &[f64],
    mode: &str,
    context: &str,
) -> Result<f64, SpiceError> {
    match mode {
        "max" => Ok(values.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        "min" => Ok(values.iter().copied().fold(f64::INFINITY, f64::min)),
        "avg" => Ok(values.iter().sum::<f64>() / values.len() as f64),
        "rms" => Ok(
            (values.iter().map(|value| value * value).sum::<f64>() / values.len() as f64).sqrt(),
        ),
        "pp" => {
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            Ok(max - min)
        }
        "last" => Ok(*values.last().unwrap()),
        _ => Err(table_error(context, &format!("unsupported mode {mode:?}"))),
    }
}

fn format_optional_table_number(value: Option<f64>) -> String {
    value.map(format_table_number).unwrap_or_default()
}

fn table_probe_value(
    node_voltages: &BTreeMap<String, f64>,
    branch_currents: &BTreeMap<String, f64>,
    probe: &str,
    context: &str,
) -> Result<f64, SpiceError> {
    let text = probe.trim();
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("v(") && text.ends_with(')') {
        let args: Vec<&str> = text[2..text.len() - 1]
            .split(',')
            .map(|arg| arg.trim())
            .collect();
        if args.len() == 1 {
            return table_voltage(node_voltages, args[0], context);
        }
        if args.len() == 2 {
            return Ok(table_voltage(node_voltages, args[0], context)?
                - table_voltage(node_voltages, args[1], context)?);
        }
    }
    if lower.starts_with("i(") && text.ends_with(')') {
        let key = format!("I({})", text[2..text.len() - 1].trim());
        return branch_currents
            .get(&key)
            .copied()
            .ok_or_else(|| table_error(context, &format!("missing branch current probe {probe}")));
    }
    if !text.is_empty() {
        return table_voltage(node_voltages, text, context);
    }
    Err(table_error(context, "empty probe"))
}

fn table_complex_probe_value(
    node_voltages: &BTreeMap<String, Complex>,
    branch_currents: &BTreeMap<String, Complex>,
    probe: &str,
    context: &str,
) -> Result<Complex, SpiceError> {
    let text = probe.trim();
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("v(") && text.ends_with(')') {
        let args: Vec<&str> = text[2..text.len() - 1]
            .split(',')
            .map(|arg| arg.trim())
            .collect();
        if args.len() == 1 {
            return table_complex_voltage(node_voltages, args[0], context);
        }
        if args.len() == 2 {
            return Ok(table_complex_voltage(node_voltages, args[0], context)?
                - table_complex_voltage(node_voltages, args[1], context)?);
        }
    }
    if lower.starts_with("i(") && text.ends_with(')') {
        let key = format!("I({})", text[2..text.len() - 1].trim());
        return branch_currents
            .get(&key)
            .copied()
            .ok_or_else(|| table_error(context, &format!("missing branch current probe {probe}")));
    }
    if !text.is_empty() {
        return table_complex_voltage(node_voltages, text, context);
    }
    Err(table_error(context, "empty probe"))
}

fn table_complex_voltage(
    node_voltages: &BTreeMap<String, Complex>,
    node: &str,
    context: &str,
) -> Result<Complex, SpiceError> {
    if is_ground(node) {
        return Ok(Complex::zero());
    }
    node_voltages
        .get(node)
        .copied()
        .ok_or_else(|| table_error(context, &format!("missing node voltage {node}")))
}

fn table_voltage(
    node_voltages: &BTreeMap<String, f64>,
    node: &str,
    context: &str,
) -> Result<f64, SpiceError> {
    if is_ground(node) {
        return Ok(0.0);
    }
    node_voltages
        .get(node)
        .copied()
        .ok_or_else(|| table_error(context, &format!("missing node voltage {node}")))
}

fn table_error(context: &str, reason: &str) -> SpiceError {
    SpiceError::InvalidElement {
        name: context.to_string(),
        reason: reason.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpiceError {
    InvalidElement { name: String, reason: String },
    SingularMatrix,
}

impl fmt::Display for SpiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidElement { name, reason } => write!(f, "invalid element {name}: {reason}"),
            Self::SingularMatrix => write!(f, "circuit matrix is singular"),
        }
    }
}

impl std::error::Error for SpiceError {}

pub fn dc_op(circuit: &Circuit) -> Result<DcResult, SpiceError> {
    dc_op_with_options(circuit, DcOpOptions::default())
}

pub fn dc_op_with_options(circuit: &Circuit, options: DcOpOptions) -> Result<DcResult, SpiceError> {
    dc_op_with_optional_initial_vector(circuit, options, None)
}

pub fn dc_op_with_initial_vector(
    circuit: &Circuit,
    options: DcOpOptions,
    initial_vector: &[f64],
) -> Result<DcResult, SpiceError> {
    dc_op_with_optional_initial_vector(circuit, options, Some(initial_vector))
}

pub fn dc_op_with_initial_conditions(
    circuit: &Circuit,
    summary: &DeckInitialConditionSummary,
    options: DcOpOptions,
) -> Result<DcResult, SpiceError> {
    let initial_vector =
        dc_initial_vector_from_conditions(circuit, &summary.initial_conditions, &summary.nodesets)?;
    dc_op_with_initial_vector(circuit, options, &initial_vector)
}

pub fn dc_initial_vector_from_conditions(
    circuit: &Circuit,
    initial_conditions: &[DeckNodeCondition],
    nodesets: &[DeckNodeCondition],
) -> Result<Vec<f64>, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    let voltage_sources = collect_voltage_sources(circuit, &[])?;
    let mut vector = vec![0.0; node_indices.len() + voltage_sources.len()];

    for condition in nodesets {
        apply_node_condition_to_initial_vector(condition, &node_indices, &mut vector)?;
    }
    for condition in initial_conditions {
        apply_node_condition_to_initial_vector(condition, &node_indices, &mut vector)?;
    }
    Ok(vector)
}

fn dc_op_with_optional_initial_vector(
    circuit: &Circuit,
    options: DcOpOptions,
    initial_vector: Option<&[f64]>,
) -> Result<DcResult, SpiceError> {
    validate_dc_op_options(options)?;
    if let Some(vector) = initial_vector {
        validate_dc_initial_vector(circuit, vector)?;
    }
    let solution = solve_dc_newton(circuit, options, initial_vector)?;
    if solution.converged {
        return Ok(dc_result_from_linear_solution(
            solution,
            DcConvergenceAid::Newton,
            options.tolerance,
        ));
    }
    if !options.convergence_aids {
        return Ok(dc_result_from_linear_solution(
            solution,
            DcConvergenceAid::None,
            options.tolerance,
        ));
    }

    let (final_solution, convergence_aid) =
        if let Some(aided) = solve_dc_with_gmin_stepping(circuit, options, &solution.vector)? {
            (aided, DcConvergenceAid::Gmin)
        } else if let Some(aided) = solve_dc_with_source_stepping(circuit, options)? {
            (aided, DcConvergenceAid::Source)
        } else if let Some(aided) = solve_dc_with_pseudo_transient(circuit, options)? {
            (aided, DcConvergenceAid::PseudoTransient)
        } else {
            (solution, DcConvergenceAid::None)
        };
    Ok(dc_result_from_linear_solution(
        final_solution,
        convergence_aid,
        options.tolerance,
    ))
}

pub fn dc_corners(
    circuit: &Circuit,
    corners: &[CornerSpec],
    options: DcOpOptions,
) -> Result<CornerSweepResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerPoint {
            corner_name: corner.name.clone(),
            result: dc_op_with_options(&corner_circuit, options)?,
        });
    }
    Ok(CornerSweepResult { points })
}

pub fn dc_corners_parallel(
    circuit: &Circuit,
    corners: &[CornerSpec],
    options: DcOpOptions,
) -> Result<CornerSweepResult, SpiceError> {
    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                scope.spawn(move || -> Result<CornerPoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerPoint {
                        corner_name: corner.name.clone(),
                        result: dc_op_with_options(&corner_circuit, options)?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "dc_corners_parallel".to_string(),
                reason: "parallel DC corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerSweepResult { points })
}

pub fn dc_temperature_sweep(
    circuit: &Circuit,
    temperatures_kelvin: &[f64],
    nominal_temperature_kelvin: f64,
    silicon_energy_gap_ev: f64,
    options: DcOpOptions,
) -> Result<TemperatureDcResult, SpiceError> {
    let mut points = Vec::with_capacity(temperatures_kelvin.len());
    for &temperature_kelvin in temperatures_kelvin {
        let temperature_circuit = circuit_at_temperature(
            circuit,
            temperature_kelvin,
            nominal_temperature_kelvin,
            silicon_energy_gap_ev,
        )?;
        points.push(TemperatureDcPoint {
            temperature_kelvin,
            result: dc_op_with_options(&temperature_circuit, options)?,
        });
    }
    Ok(TemperatureDcResult { points })
}

pub fn dc_temperature_sweep_corners(
    circuit: &Circuit,
    temperatures_kelvin: &[f64],
    nominal_temperature_kelvin: f64,
    silicon_energy_gap_ev: f64,
    options: DcOpOptions,
    corners: &[CornerSpec],
) -> Result<CornerTemperatureDcResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerTemperatureDcPoint {
            corner_name: corner.name.clone(),
            points: dc_temperature_sweep(
                &corner_circuit,
                temperatures_kelvin,
                nominal_temperature_kelvin,
                silicon_energy_gap_ev,
                options,
            )?
            .points,
        });
    }
    Ok(CornerTemperatureDcResult { points })
}

fn circuit_with_corner(circuit: &Circuit, corner: &CornerSpec) -> Result<Circuit, SpiceError> {
    let mut overrides_by_name: HashMap<&str, Vec<&CornerOverride>> = HashMap::new();
    for override_ in &corner.overrides {
        overrides_by_name
            .entry(&override_.element_name)
            .or_default()
            .push(override_);
    }

    let mut seen = Vec::new();
    let mut corner_circuit = Circuit::new();
    for element in circuit.elements() {
        let mut element = element.clone();
        if let Some(name) = element_name(&element) {
            if let Some(overrides) = overrides_by_name.get(name) {
                seen.push(name.to_string());
                for override_ in overrides {
                    element = apply_corner_override(element, override_)?;
                }
            }
        }
        corner_circuit.add(element);
    }

    for element_name in overrides_by_name.keys() {
        if !seen.iter().any(|seen_name| seen_name == element_name) {
            return Err(SpiceError::InvalidElement {
                name: "dc_corners".to_string(),
                reason: format!("missing element for corner override {element_name:?}"),
            });
        }
    }
    Ok(corner_circuit)
}

fn element_name(element: &Element) -> Option<&str> {
    match element {
        Element::Resistor(element) => Some(&element.name),
        Element::Capacitor(element) => Some(&element.name),
        Element::Inductor(element) => Some(&element.name),
        Element::VoltageSource(element) => Some(&element.name),
        Element::CurrentSource(element) => Some(&element.name),
        Element::CustomModel(element) => Some(&element.name),
        _ => None,
    }
}

fn apply_corner_override(
    element: Element,
    override_: &CornerOverride,
) -> Result<Element, SpiceError> {
    if !override_.value.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: "dc_corners".to_string(),
            reason: "override values must be finite".to_string(),
        });
    }

    match element {
        Element::Resistor(mut element) if override_.parameter == "resistance" => {
            if override_.value <= 0.0 {
                return Err(SpiceError::InvalidElement {
                    name: "dc_corners".to_string(),
                    reason: "resistance overrides must be positive".to_string(),
                });
            }
            element.resistance_ohms = override_.value;
            Ok(Element::Resistor(element))
        }
        Element::Capacitor(mut element) if override_.parameter == "capacitance" => {
            if override_.value <= 0.0 {
                return Err(SpiceError::InvalidElement {
                    name: "dc_corners".to_string(),
                    reason: "capacitance overrides must be positive".to_string(),
                });
            }
            element.capacitance_farads = override_.value;
            Ok(Element::Capacitor(element))
        }
        Element::Inductor(mut element) if override_.parameter == "inductance" => {
            if override_.value <= 0.0 {
                return Err(SpiceError::InvalidElement {
                    name: "dc_corners".to_string(),
                    reason: "inductance overrides must be positive".to_string(),
                });
            }
            element.inductance_henrys = override_.value;
            Ok(Element::Inductor(element))
        }
        Element::VoltageSource(mut element) if override_.parameter == "voltage" => {
            element.voltage = override_.value;
            Ok(Element::VoltageSource(element))
        }
        Element::CurrentSource(mut element) if override_.parameter == "current" => {
            element.current = override_.value;
            Ok(Element::CurrentSource(element))
        }
        Element::CustomModel(mut element) if override_.parameter == "conductance" => {
            match &mut element.kind {
                CustomModelKind::LinearConductance {
                    conductance_siemens,
                    ..
                } => *conductance_siemens = override_.value,
            }
            Ok(Element::CustomModel(element))
        }
        _ => Err(SpiceError::InvalidElement {
            name: "dc_corners".to_string(),
            reason: format!(
                "unsupported override {:?}.{:?}",
                override_.element_name, override_.parameter
            ),
        }),
    }
}

fn dc_result_from_linear_solution(
    solution: LinearSolution,
    convergence_aid: DcConvergenceAid,
    tolerance: f64,
) -> DcResult {
    let matrix_size = solution.vector.len();
    DcResult {
        node_voltages: solution.node_voltages,
        branch_currents: solution.branch_currents,
        iterations: solution.iterations,
        converged: solution.converged,
        convergence_aid,
        diagnostics: DcSolverDiagnostics {
            matrix_size,
            solver: real_solver_kind(matrix_size).to_string(),
            tolerance,
            max_delta: solution.max_delta,
            convergence_aid,
            newton_step_limit: solution.newton_step_limit,
            limited_newton_steps: solution.limited_newton_steps,
            minimum_damping_factor: solution.minimum_damping_factor,
            solver_profile: solution.solver_profile,
        },
    }
}

fn validate_dc_op_options(options: DcOpOptions) -> Result<(), SpiceError> {
    if options.max_iterations == 0 {
        return Err(SpiceError::InvalidElement {
            name: "dc_op".to_string(),
            reason: "max_iterations must be positive".to_string(),
        });
    }
    if !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "dc_op".to_string(),
            reason: "tolerance must be finite and positive".to_string(),
        });
    }
    if !options.pseudo_transient_conductance.is_finite()
        || options.pseudo_transient_conductance <= 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: "dc_op".to_string(),
            reason: "pseudo_transient_conductance must be finite and positive".to_string(),
        });
    }
    if options.pseudo_transient_max_iterations == 0 {
        return Err(SpiceError::InvalidElement {
            name: "dc_op".to_string(),
            reason: "pseudo_transient_max_iterations must be positive".to_string(),
        });
    }
    if let Some(limit) = options.newton_step_limit {
        if !limit.is_finite() || limit <= 0.0 {
            return Err(SpiceError::InvalidElement {
                name: "dc_op".to_string(),
                reason: "newton_step_limit must be finite and positive".to_string(),
            });
        }
    }
    Ok(())
}

fn validate_dc_initial_vector(circuit: &Circuit, initial_vector: &[f64]) -> Result<(), SpiceError> {
    let node_indices = collect_node_indices(circuit);
    let voltage_sources = collect_voltage_sources(circuit, &[])?;
    let expected_len = node_indices.len() + voltage_sources.len();
    if initial_vector.len() != expected_len {
        return Err(SpiceError::InvalidElement {
            name: "dc_initial_vector".to_string(),
            reason: format!(
                "expected {expected_len} entries for circuit MNA ordering, got {}",
                initial_vector.len()
            ),
        });
    }
    if initial_vector.iter().any(|value| !value.is_finite()) {
        return Err(SpiceError::InvalidElement {
            name: "dc_initial_vector".to_string(),
            reason: "all entries must be finite".to_string(),
        });
    }
    Ok(())
}

fn apply_node_condition_to_initial_vector(
    condition: &DeckNodeCondition,
    node_indices: &HashMap<String, usize>,
    vector: &mut [f64],
) -> Result<(), SpiceError> {
    if !condition.value.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: condition.directive.clone(),
            reason: format!("V({}) must be finite", condition.node),
        });
    }
    if is_ground(&condition.node) {
        if condition.value != 0.0 {
            return Err(SpiceError::InvalidElement {
                name: condition.directive.clone(),
                reason: format!("V({}) conflicts with ground", condition.node),
            });
        }
        return Ok(());
    }
    let Some(index) = node_indices.get(&condition.node) else {
        return Err(SpiceError::InvalidElement {
            name: condition.directive.clone(),
            reason: format!("references unknown node {:?}", condition.node),
        });
    };
    vector[*index] = condition.value;
    Ok(())
}

fn solve_dc_with_gmin_stepping(
    circuit: &Circuit,
    options: DcOpOptions,
    initial_vector: &[f64],
) -> Result<Option<LinearSolution>, SpiceError> {
    let mut warm_start = initial_vector.to_vec();
    let mut final_solution = None;

    for gmin in dc_gmin_sequence() {
        let stepped_circuit = if gmin == 0.0 {
            circuit.clone()
        } else {
            circuit_with_gmin(circuit, gmin)
        };
        let solution = solve_dc_newton(&stepped_circuit, options, Some(&warm_start))?;
        if !solution.converged {
            return Ok(None);
        }
        warm_start = solution.vector.clone();
        final_solution = Some(solution);
    }

    Ok(final_solution)
}

fn solve_dc_with_source_stepping(
    circuit: &Circuit,
    options: DcOpOptions,
) -> Result<Option<LinearSolution>, SpiceError> {
    let mut warm_start: Option<Vec<f64>> = None;
    let mut final_solution = None;

    for step in 0..=10 {
        let scale = step as f64 / 10.0;
        let stepped_circuit = if scale == 1.0 {
            circuit.clone()
        } else {
            circuit_with_scaled_independent_sources(circuit, scale)
        };
        let solution = solve_dc_newton(&stepped_circuit, options, warm_start.as_deref())?;
        if !solution.converged {
            return Ok(None);
        }
        warm_start = Some(solution.vector.clone());
        final_solution = Some(solution);
    }

    Ok(final_solution)
}

fn solve_dc_with_pseudo_transient(
    circuit: &Circuit,
    options: DcOpOptions,
) -> Result<Option<LinearSolution>, SpiceError> {
    if options.pseudo_transient_steps == 0 {
        return Ok(None);
    }

    let node_indices = collect_node_indices(circuit);
    if node_indices.is_empty() {
        return Ok(None);
    }
    let mut nodes_by_index: Vec<(&String, &usize)> = node_indices.iter().collect();
    nodes_by_index.sort_by_key(|(_, index)| **index);
    let nodes: Vec<String> = nodes_by_index
        .into_iter()
        .map(|(node, _)| node.clone())
        .collect();

    let mut previous_node_voltages: BTreeMap<String, f64> =
        nodes.iter().map(|node| (node.clone(), 0.0)).collect();
    let mut warm_start: Option<Vec<f64>> = None;
    let mut last_solution = None;
    let pseudo_options = DcOpOptions {
        max_iterations: options.pseudo_transient_max_iterations,
        ..options
    };

    for step in 0..options.pseudo_transient_steps {
        let pseudo_circuit = circuit_with_pseudo_transient_companions(
            circuit,
            &nodes,
            &previous_node_voltages,
            options.pseudo_transient_conductance,
            step,
        );
        let solution = solve_dc_newton(&pseudo_circuit, pseudo_options, warm_start.as_deref())?;
        if !solution.converged {
            return Ok(None);
        }

        let mut delta: f64 = 0.0;
        let mut next_node_voltages = BTreeMap::new();
        for node in &nodes {
            let next = *solution.node_voltages.get(node).unwrap_or(&0.0);
            let previous = *previous_node_voltages.get(node).unwrap_or(&0.0);
            delta = delta.max((next - previous).abs());
            next_node_voltages.insert(node.clone(), next);
        }
        previous_node_voltages = next_node_voltages;
        warm_start = Some(solution.vector.clone());
        last_solution = Some(solution);
        if delta < options.tolerance {
            break;
        }
    }

    if last_solution.is_none() {
        return Ok(None);
    }

    let final_solution = solve_dc_newton(circuit, pseudo_options, warm_start.as_deref())?;
    if final_solution.converged {
        Ok(Some(final_solution))
    } else {
        Ok(None)
    }
}

fn circuit_with_pseudo_transient_companions(
    circuit: &Circuit,
    nodes: &[String],
    previous_node_voltages: &BTreeMap<String, f64>,
    conductance: f64,
    step: usize,
) -> Circuit {
    let mut pseudo_circuit = circuit.clone();
    for node in nodes {
        pseudo_circuit.add(Element::Resistor(Resistor::new(
            format!("__ptran_g_{step}_{node}"),
            node.clone(),
            "0",
            1.0 / conductance,
        )));
        let history_current = conductance * previous_node_voltages.get(node).unwrap_or(&0.0);
        if history_current != 0.0 {
            pseudo_circuit.add(Element::CurrentSource(CurrentSource::new(
                format!("__ptran_i_{step}_{node}"),
                "0",
                node.clone(),
                history_current,
            )));
        }
    }
    pseudo_circuit
}

fn dc_gmin_sequence() -> Vec<f64> {
    let mut sequence: Vec<f64> = (-12..=-3)
        .rev()
        .map(|exponent| 10.0_f64.powi(exponent))
        .collect();
    sequence.push(0.0);
    sequence
}

fn circuit_with_gmin(circuit: &Circuit, gmin_siemens: f64) -> Circuit {
    let mut aided = circuit.clone();
    for node in collect_node_indices(circuit).keys() {
        aided.add(Element::Resistor(Resistor::new(
            format!("__gmin_{node}"),
            node.clone(),
            "0",
            1.0 / gmin_siemens,
        )));
    }
    aided
}

fn circuit_with_scaled_independent_sources(circuit: &Circuit, scale: f64) -> Circuit {
    let mut scaled = circuit.clone();
    for element in &mut scaled.elements {
        match element {
            Element::VoltageSource(source) => source.voltage *= scale,
            Element::CurrentSource(source) => source.current *= scale,
            _ => {}
        }
    }
    scaled
}

pub fn dc_sweep(
    circuit: &Circuit,
    source_name: &str,
    start: f64,
    stop: f64,
    step: f64,
) -> Result<Vec<DcSweepPoint>, SpiceError> {
    validate_sweep(source_name, start, stop, step)?;

    let mut points = Vec::new();
    let mut value = start;
    let epsilon = step.abs() * 1.0e-9;
    while sweep_includes(value, stop, step, epsilon) {
        let mut swept = circuit.clone();
        set_source_value(&mut swept, source_name, value)?;
        points.push(DcSweepPoint {
            value,
            result: dc_op(&swept)?,
        });
        value += step;
    }
    Ok(points)
}

pub fn dc_sweep_corners(
    circuit: &Circuit,
    source_name: &str,
    start: f64,
    stop: f64,
    step: f64,
    corners: &[CornerSpec],
) -> Result<CornerDcSweepResult, SpiceError> {
    validate_sweep(source_name, start, stop, step)?;

    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerDcSweepPoint {
            corner_name: corner.name.clone(),
            points: dc_sweep(&corner_circuit, source_name, start, stop, step)?,
        });
    }
    Ok(CornerDcSweepResult {
        source_name: source_name.to_string(),
        points,
    })
}

pub fn dc_sweep_corners_parallel(
    circuit: &Circuit,
    source_name: &str,
    start: f64,
    stop: f64,
    step: f64,
    corners: &[CornerSpec],
) -> Result<CornerDcSweepResult, SpiceError> {
    validate_sweep(source_name, start, stop, step)?;

    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                let source_name = source_name.to_string();
                scope.spawn(move || -> Result<CornerDcSweepPoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerDcSweepPoint {
                        corner_name: corner.name.clone(),
                        points: dc_sweep(&corner_circuit, &source_name, start, stop, step)?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "dc_sweep_corners_parallel".to_string(),
                reason: "parallel DC source-sweep corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerDcSweepResult {
        source_name: source_name.to_string(),
        points,
    })
}

pub fn mc_dc(
    circuit: &Circuit,
    output_node: &str,
    n_trials: usize,
    options: McOptions,
) -> Result<McResult, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    if !is_ground(output_node) && !node_indices.contains_key(output_node) {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "output node was not found in circuit".to_string(),
        });
    }
    if n_trials == 0 {
        return Err(SpiceError::InvalidElement {
            name: "mc_dc".to_string(),
            reason: "n_trials must be positive".to_string(),
        });
    }
    if !options.tolerance.is_finite() || options.tolerance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "mc_dc".to_string(),
            reason: "tolerance must be finite and non-negative".to_string(),
        });
    }

    let mut rng = McRng::new(options.seed.unwrap_or(0x6d2b_79f5));
    let mut points = Vec::with_capacity(n_trials);
    for trial in 0..n_trials {
        let trial_circuit = circuit_with_randomized_elements(
            circuit,
            options.tolerance,
            options.distribution,
            &mut rng,
        );
        match dc_op(&trial_circuit) {
            Ok(result) => points.push(McPoint {
                trial,
                node_voltages: result.node_voltages,
                branch_currents: result.branch_currents,
                converged: true,
            }),
            Err(SpiceError::SingularMatrix) => points.push(McPoint {
                trial,
                node_voltages: BTreeMap::new(),
                branch_currents: BTreeMap::new(),
                converged: false,
            }),
            Err(error) => return Err(error),
        }
    }

    let converged_voltages: Vec<f64> = points
        .iter()
        .filter(|point| point.converged)
        .map(|point| point.voltage(output_node).unwrap_or(0.0))
        .collect();

    Ok(McResult {
        output_node: output_node.to_string(),
        points,
        n_trials,
        mean: sample_mean(&converged_voltages),
        std_dev: sample_std_dev(&converged_voltages),
    })
}

pub fn mc_dc_corners(
    circuit: &Circuit,
    output_node: &str,
    n_trials: usize,
    options: McOptions,
    corners: &[CornerSpec],
) -> Result<CornerMcResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerMcPoint {
            corner_name: corner.name.clone(),
            result: mc_dc(&corner_circuit, output_node, n_trials, options)?,
        });
    }
    Ok(CornerMcResult {
        output_node: output_node.to_string(),
        points,
    })
}

pub fn mc_dc_corners_parallel(
    circuit: &Circuit,
    output_node: &str,
    n_trials: usize,
    options: McOptions,
    corners: &[CornerSpec],
) -> Result<CornerMcResult, SpiceError> {
    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                let output_node = output_node.to_string();
                scope.spawn(move || -> Result<CornerMcPoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerMcPoint {
                        corner_name: corner.name.clone(),
                        result: mc_dc(&corner_circuit, &output_node, n_trials, options)?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "mc_dc_corners_parallel".to_string(),
                reason: "parallel Monte Carlo DC corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerMcResult {
        output_node: output_node.to_string(),
        points,
    })
}

pub fn tf(
    circuit: &Circuit,
    output_node: &str,
    input_source: &str,
) -> Result<TfResult, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    if !is_ground(output_node) && !node_indices.contains_key(output_node) {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "output node was not found in circuit".to_string(),
        });
    }

    let input = find_input_source(circuit, input_source)?;
    let voltage_sources = collect_ac_voltage_sources(circuit)?;
    let node_count = node_indices.len();
    let operating_solution = solve_linear_circuit(circuit, &[], &[], None)?;
    let matrix = build_small_signal_matrix(
        circuit,
        &node_indices,
        &voltage_sources,
        &operating_solution.vector,
    )?;
    let size = matrix.len();
    let output_index = node_index(&node_indices, output_node);

    let mut forward_rhs = vec![0.0; size];
    match input {
        InputSource::Voltage(source) => {
            let Some(source_index) = voltage_sources.get(&source.name) else {
                return Err(SpiceError::InvalidElement {
                    name: source.name.clone(),
                    reason: "voltage source was not indexed".to_string(),
                });
            };
            forward_rhs[node_count + source_index] = 1.0;
        }
        InputSource::Current(source) => {
            if let Some(i) = node_index(&node_indices, &source.positive) {
                forward_rhs[i] -= 1.0;
            }
            if let Some(j) = node_index(&node_indices, &source.negative) {
                forward_rhs[j] += 1.0;
            }
        }
    }

    let forward = solve_linear_system(matrix.clone(), forward_rhs)?;
    let transfer_ratio = output_index.map_or(0.0, |idx| forward[idx]);
    let input_impedance_ohms = match input {
        InputSource::Voltage(source) => {
            let source_index = voltage_sources[&source.name];
            let branch_current = forward[node_count + source_index];
            if branch_current.abs() > 1.0e-30 {
                -1.0 / branch_current
            } else {
                f64::INFINITY
            }
        }
        InputSource::Current(source) => {
            let v_plus =
                node_index(&node_indices, &source.positive).map_or(0.0, |idx| forward[idx]);
            let v_minus =
                node_index(&node_indices, &source.negative).map_or(0.0, |idx| forward[idx]);
            v_minus - v_plus
        }
    };

    let mut output_rhs = vec![0.0; size];
    if let Some(idx) = output_index {
        output_rhs[idx] = 1.0;
    }
    let output = solve_linear_system(matrix, output_rhs)?;
    let output_impedance_ohms = output_index.map_or(0.0, |idx| output[idx]);

    Ok(TfResult {
        transfer_ratio,
        input_impedance_ohms,
        output_impedance_ohms,
    })
}

pub fn tf_corners(
    circuit: &Circuit,
    output_node: &str,
    input_source: &str,
    corners: &[CornerSpec],
) -> Result<CornerTfResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerTfPoint {
            corner_name: corner.name.clone(),
            result: tf(&corner_circuit, output_node, input_source)?,
        });
    }
    Ok(CornerTfResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        points,
    })
}

pub fn tf_corners_parallel(
    circuit: &Circuit,
    output_node: &str,
    input_source: &str,
    corners: &[CornerSpec],
) -> Result<CornerTfResult, SpiceError> {
    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                let output_node = output_node.to_string();
                let input_source = input_source.to_string();
                scope.spawn(move || -> Result<CornerTfPoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerTfPoint {
                        corner_name: corner.name.clone(),
                        result: tf(&corner_circuit, &output_node, &input_source)?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "tf_corners_parallel".to_string(),
                reason: "parallel transfer-function corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerTfResult {
        input_source: input_source.to_string(),
        output_node: output_node.to_string(),
        points,
    })
}

pub fn sens_dc(circuit: &Circuit, output_node: &str) -> Result<SensResult, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    if !is_ground(output_node) && !node_indices.contains_key(output_node) {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "output node was not found in circuit".to_string(),
        });
    }

    let nominal = dc_op(circuit)?;
    let nominal_voltage = nominal.voltage(output_node).unwrap_or(0.0);
    let mut entries = Vec::new();

    for element_index in 0..circuit.elements.len() {
        if let Some((element_name, parameter, nominal_value)) =
            element_parameter(&circuit.elements[element_index])
        {
            let delta = perturbation_for(nominal_value);
            let mut perturbed = circuit.clone();
            perturb_element_parameter(&mut perturbed.elements[element_index], delta);
            let perturbed_result = dc_op(&perturbed)?;
            let perturbed_voltage = perturbed_result.voltage(output_node).unwrap_or(0.0);
            let sensitivity = (perturbed_voltage - nominal_voltage) / delta;
            let relative_sensitivity = if nominal_voltage.abs() > 1.0e-30 {
                sensitivity * nominal_value / nominal_voltage
            } else {
                0.0
            };
            entries.push(SensEntry {
                element_name,
                parameter,
                nominal_value,
                sensitivity,
                relative_sensitivity,
            });
        }
    }

    entries.sort_by(|left, right| {
        right
            .relative_sensitivity
            .abs()
            .total_cmp(&left.relative_sensitivity.abs())
            .then_with(|| left.element_name.cmp(&right.element_name))
            .then_with(|| left.parameter.cmp(&right.parameter))
    });

    Ok(SensResult {
        output_node: output_node.to_string(),
        nominal_voltage,
        entries,
    })
}

pub fn sens_dc_corners(
    circuit: &Circuit,
    output_node: &str,
    corners: &[CornerSpec],
) -> Result<CornerSensResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerSensPoint {
            corner_name: corner.name.clone(),
            result: sens_dc(&corner_circuit, output_node)?,
        });
    }
    Ok(CornerSensResult {
        output_node: output_node.to_string(),
        points,
    })
}

pub fn sens_dc_corners_parallel(
    circuit: &Circuit,
    output_node: &str,
    corners: &[CornerSpec],
) -> Result<CornerSensResult, SpiceError> {
    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                let output_node = output_node.to_string();
                scope.spawn(move || -> Result<CornerSensPoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerSensPoint {
                        corner_name: corner.name.clone(),
                        result: sens_dc(&corner_circuit, &output_node)?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "sens_dc_corners_parallel".to_string(),
                reason: "parallel DC sensitivity corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerSensResult {
        output_node: output_node.to_string(),
        points,
    })
}

pub fn ac_sweep(
    circuit: &Circuit,
    start_hz: f64,
    stop_hz: f64,
    points_per_decade: usize,
) -> Result<Vec<AcPoint>, SpiceError> {
    validate_ac_sweep(start_hz, stop_hz, points_per_decade)?;
    validate_reactive_elements(circuit)?;

    let mut points = Vec::new();
    let ratio = 10.0_f64.powf(1.0 / points_per_decade as f64);
    let epsilon = stop_hz * 1.0e-12;
    let mut frequency = start_hz;
    while frequency <= stop_hz + epsilon {
        let solution = solve_ac_circuit(circuit, TWO_PI * frequency)?;
        points.push(AcPoint {
            frequency_hz: frequency,
            node_voltages: solution.node_voltages,
            branch_currents: solution.branch_currents,
        });
        frequency *= ratio;
    }
    Ok(points)
}

fn validate_ac_sweep(
    start_hz: f64,
    stop_hz: f64,
    points_per_decade: usize,
) -> Result<(), SpiceError> {
    if !start_hz.is_finite() || !stop_hz.is_finite() || start_hz <= 0.0 || stop_hz <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "ac_sweep".to_string(),
            reason: "frequency bounds must be finite and positive".to_string(),
        });
    }
    if stop_hz < start_hz {
        return Err(SpiceError::InvalidElement {
            name: "ac_sweep".to_string(),
            reason: "stop frequency must be greater than or equal to start frequency".to_string(),
        });
    }
    if points_per_decade == 0 {
        return Err(SpiceError::InvalidElement {
            name: "ac_sweep".to_string(),
            reason: "points per decade must be positive".to_string(),
        });
    }
    Ok(())
}

pub fn ac_sweep_corners(
    circuit: &Circuit,
    start_hz: f64,
    stop_hz: f64,
    points_per_decade: usize,
    corners: &[CornerSpec],
) -> Result<CornerAcSweepResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerAcSweepPoint {
            corner_name: corner.name.clone(),
            points: ac_sweep(&corner_circuit, start_hz, stop_hz, points_per_decade)?,
        });
    }
    Ok(CornerAcSweepResult { points })
}

pub fn ac_sweep_corners_parallel(
    circuit: &Circuit,
    start_hz: f64,
    stop_hz: f64,
    points_per_decade: usize,
    corners: &[CornerSpec],
) -> Result<CornerAcSweepResult, SpiceError> {
    validate_ac_sweep(start_hz, stop_hz, points_per_decade)?;

    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                scope.spawn(move || -> Result<CornerAcSweepPoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerAcSweepPoint {
                        corner_name: corner.name.clone(),
                        points: ac_sweep(&corner_circuit, start_hz, stop_hz, points_per_decade)?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "ac_sweep_corners_parallel".to_string(),
                reason: "parallel AC corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerAcSweepResult { points })
}

pub fn s_parameters(
    circuit: &Circuit,
    port1_source: &str,
    port2_source: &str,
    frequencies_hz: &[f64],
    reference_impedance_ohms: f64,
) -> Result<SParameterResult, SpiceError> {
    if !reference_impedance_ohms.is_finite() || reference_impedance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "s_parameters".to_string(),
            reason: "reference impedance must be finite and positive".to_string(),
        });
    }
    for frequency in frequencies_hz {
        if !frequency.is_finite() || *frequency <= 0.0 {
            return Err(SpiceError::InvalidElement {
                name: "s_parameters".to_string(),
                reason: "frequencies must be finite and positive".to_string(),
            });
        }
    }

    let ports = [port1_source, port2_source];
    validate_sparameter_ports(circuit, &ports)?;

    let mut points = Vec::new();
    for frequency_hz in frequencies_hz {
        let mut columns = Vec::new();
        for driven_source in ports {
            let driven_circuit = circuit_with_sparameter_drive(circuit, &ports, driven_source);
            let point = ac_sweep(&driven_circuit, *frequency_hz, *frequency_hz, 1)?
                .into_iter()
                .next()
                .ok_or(SpiceError::SingularMatrix)?;
            columns.push([
                branch_current_into_network(&point, port1_source)?,
                branch_current_into_network(&point, port2_source)?,
            ]);
        }

        let (s11, s21, s12, s22) = y_to_s_2port(
            columns[0][0],
            columns[0][1],
            columns[1][0],
            columns[1][1],
            reference_impedance_ohms,
        )?;
        points.push(SParameterPoint {
            frequency_hz: *frequency_hz,
            s11,
            s21,
            s12,
            s22,
        });
    }

    Ok(SParameterResult {
        port1_source: port1_source.to_string(),
        port2_source: port2_source.to_string(),
        reference_impedance_ohms,
        points,
    })
}

pub fn s_parameters_corners(
    circuit: &Circuit,
    port1_source: &str,
    port2_source: &str,
    frequencies_hz: &[f64],
    reference_impedance_ohms: f64,
    corners: &[CornerSpec],
) -> Result<CornerSParameterResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerSParameterPoint {
            corner_name: corner.name.clone(),
            result: s_parameters(
                &corner_circuit,
                port1_source,
                port2_source,
                frequencies_hz,
                reference_impedance_ohms,
            )?,
        });
    }
    Ok(CornerSParameterResult {
        port1_source: port1_source.to_string(),
        port2_source: port2_source.to_string(),
        reference_impedance_ohms,
        points,
    })
}

pub fn s_parameters_corners_parallel(
    circuit: &Circuit,
    port1_source: &str,
    port2_source: &str,
    frequencies_hz: &[f64],
    reference_impedance_ohms: f64,
    corners: &[CornerSpec],
) -> Result<CornerSParameterResult, SpiceError> {
    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                let port1_source = port1_source.to_string();
                let port2_source = port2_source.to_string();
                let frequencies_hz = frequencies_hz.to_vec();
                scope.spawn(move || -> Result<CornerSParameterPoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerSParameterPoint {
                        corner_name: corner.name.clone(),
                        result: s_parameters(
                            &corner_circuit,
                            &port1_source,
                            &port2_source,
                            &frequencies_hz,
                            reference_impedance_ohms,
                        )?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "s_parameters_corners_parallel".to_string(),
                reason: "parallel S-parameter corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerSParameterResult {
        port1_source: port1_source.to_string(),
        port2_source: port2_source.to_string(),
        reference_impedance_ohms,
        points,
    })
}

fn validate_sparameter_ports(circuit: &Circuit, ports: &[&str; 2]) -> Result<(), SpiceError> {
    for port in ports {
        let found = circuit.elements().iter().any(
            |element| matches!(element, Element::VoltageSource(source) if source.name == *port),
        );
        if !found {
            return Err(SpiceError::InvalidElement {
                name: "s_parameters".to_string(),
                reason: format!("missing voltage-source port {port:?}"),
            });
        }
    }
    Ok(())
}

fn circuit_with_sparameter_drive(
    circuit: &Circuit,
    ports: &[&str; 2],
    driven_source: &str,
) -> Circuit {
    let mut driven = Circuit::new();
    for element in circuit.elements() {
        if let Element::VoltageSource(source) = element {
            if ports.iter().any(|port| *port == source.name) {
                let mut source = source.clone();
                source.ac = Some(AcSource::new(
                    if source.name == driven_source {
                        1.0
                    } else {
                        0.0
                    },
                    0.0,
                ));
                driven.add(Element::VoltageSource(source));
                continue;
            }
        }
        driven.add(element.clone());
    }
    driven
}

fn branch_current_into_network(point: &AcPoint, source_name: &str) -> Result<Complex, SpiceError> {
    point
        .branch_current(source_name)
        .map(|current| current * Complex::new(-1.0, 0.0))
        .ok_or_else(|| SpiceError::InvalidElement {
            name: "s_parameters".to_string(),
            reason: format!("missing branch current for {source_name:?}"),
        })
}

fn y_to_s_2port(
    y11: Complex,
    y21: Complex,
    y12: Complex,
    y22: Complex,
    z0: f64,
) -> Result<(Complex, Complex, Complex, Complex), SpiceError> {
    let one = Complex::new(1.0, 0.0);
    let z0_c = Complex::new(z0, 0.0);
    let a11 = one - y11 * z0_c;
    let a12 = y12 * Complex::new(-z0, 0.0);
    let a21 = y21 * Complex::new(-z0, 0.0);
    let a22 = one - y22 * z0_c;

    let b11 = one + y11 * z0_c;
    let b12 = y12 * z0_c;
    let b21 = y21 * z0_c;
    let b22 = one + y22 * z0_c;
    let det = b11 * b22 - b12 * b21;
    if det.abs() < 1.0e-18 {
        return Err(SpiceError::InvalidElement {
            name: "s_parameters".to_string(),
            reason: "singular Y-to-S conversion".to_string(),
        });
    }

    let inv_b11 = b22 / det;
    let inv_b12 = b12 * Complex::new(-1.0, 0.0) / det;
    let inv_b21 = b21 * Complex::new(-1.0, 0.0) / det;
    let inv_b22 = b11 / det;

    Ok((
        a11 * inv_b11 + a12 * inv_b21,
        a21 * inv_b11 + a22 * inv_b21,
        a11 * inv_b12 + a12 * inv_b22,
        a21 * inv_b12 + a22 * inv_b22,
    ))
}

pub fn noise_ac(
    circuit: &Circuit,
    output_node: &str,
    input_source: &str,
    frequencies_hz: &[f64],
    temperature_kelvin: f64,
) -> Result<NoiseResult, SpiceError> {
    if !temperature_kelvin.is_finite() || temperature_kelvin <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "noise_ac".to_string(),
            reason: "temperature must be finite and positive".to_string(),
        });
    }
    for frequency in frequencies_hz {
        if !frequency.is_finite() || *frequency <= 0.0 {
            return Err(SpiceError::InvalidElement {
                name: "noise_ac".to_string(),
                reason: "frequencies must be finite and positive".to_string(),
            });
        }
    }

    validate_reactive_elements(circuit)?;

    let node_indices = collect_node_indices(circuit);
    if !is_ground(output_node) && !node_indices.contains_key(output_node) {
        return Err(SpiceError::InvalidElement {
            name: output_node.to_string(),
            reason: "output node was not found in circuit".to_string(),
        });
    }

    let input = find_input_source(circuit, input_source)?;
    let voltage_sources = collect_ac_voltage_sources(circuit)?;
    let node_count = node_indices.len();
    let matrix_size = node_count + voltage_sources.len();
    let operating_point = if matrix_size > 0 {
        solve_linear_circuit(circuit, &[], &[], None)?.vector
    } else {
        vec![0.0; matrix_size]
    };
    let output_index = node_index(&node_indices, output_node);
    let noise_sources =
        collect_noise_sources(circuit, &node_indices, &operating_point, temperature_kelvin)?;
    let frequencies = if frequencies_hz.is_empty() {
        default_noise_frequencies()
    } else {
        frequencies_hz.to_vec()
    };

    let mut points = Vec::with_capacity(frequencies.len());
    for frequency_hz in frequencies {
        if output_index.is_none() || matrix_size == 0 {
            points.push(NoisePoint {
                frequency_hz,
                output_psd: 0.0,
                input_referred_psd: 0.0,
                entries: zero_noise_entries(&noise_sources, frequency_hz),
            });
            continue;
        }

        let matrix = build_ac_matrix(
            circuit,
            TWO_PI * frequency_hz,
            &node_indices,
            &voltage_sources,
            &operating_point,
        )?;
        let mut rhs = vec![Complex::zero(); matrix_size];
        rhs[output_index.unwrap()] = Complex::new(1.0, 0.0);

        let adjoint = match solve_complex_linear_system(transpose_complex_matrix(&matrix), rhs) {
            Ok(solution) => solution,
            Err(SpiceError::SingularMatrix) => {
                points.push(NoisePoint {
                    frequency_hz,
                    output_psd: 0.0,
                    input_referred_psd: 0.0,
                    entries: zero_noise_entries(&noise_sources, frequency_hz),
                });
                continue;
            }
            Err(error) => return Err(error),
        };

        let mut entries: Vec<NoiseEntry> = noise_sources
            .iter()
            .map(|source| {
                let h_positive = source
                    .positive
                    .map_or(Complex::zero(), |index| adjoint[index]);
                let h_negative = source
                    .negative
                    .map_or(Complex::zero(), |index| adjoint[index]);
                let transfer = h_positive - h_negative;
                let source_psd = noise_source_psd(source, frequency_hz);
                NoiseEntry {
                    element_name: source.element_name.clone(),
                    noise_type: source.noise_type,
                    source_psd,
                    output_psd: transfer.abs().powi(2) * source_psd,
                }
            })
            .collect();
        entries.sort_by(|left, right| {
            right
                .output_psd
                .total_cmp(&left.output_psd)
                .then_with(|| left.element_name.cmp(&right.element_name))
        });

        let output_psd = entries.iter().map(|entry| entry.output_psd).sum();
        let input_gain =
            adjoint_input_gain(input, &adjoint, &node_indices, &voltage_sources, node_count)?;
        let gain_squared = input_gain.abs().powi(2);
        let input_referred_psd = if gain_squared > 1.0e-100 {
            output_psd / gain_squared
        } else {
            0.0
        };

        points.push(NoisePoint {
            frequency_hz,
            output_psd,
            input_referred_psd,
            entries,
        });
    }

    Ok(NoiseResult {
        output_node: output_node.to_string(),
        input_source: input_source.to_string(),
        temperature_kelvin,
        points,
    })
}

pub fn noise_ac_corners(
    circuit: &Circuit,
    output_node: &str,
    input_source: &str,
    frequencies_hz: &[f64],
    temperature_kelvin: f64,
    corners: &[CornerSpec],
) -> Result<CornerNoiseResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerNoisePoint {
            corner_name: corner.name.clone(),
            result: noise_ac(
                &corner_circuit,
                output_node,
                input_source,
                frequencies_hz,
                temperature_kelvin,
            )?,
        });
    }
    Ok(CornerNoiseResult {
        output_node: output_node.to_string(),
        input_source: input_source.to_string(),
        points,
    })
}

pub fn noise_ac_corners_parallel(
    circuit: &Circuit,
    output_node: &str,
    input_source: &str,
    frequencies_hz: &[f64],
    temperature_kelvin: f64,
    corners: &[CornerSpec],
) -> Result<CornerNoiseResult, SpiceError> {
    let points = thread::scope(|scope| {
        let handles = corners
            .iter()
            .map(|corner| {
                let circuit = circuit.clone();
                let output_node = output_node.to_string();
                let input_source = input_source.to_string();
                let frequencies_hz = frequencies_hz.to_vec();
                scope.spawn(move || -> Result<CornerNoisePoint, SpiceError> {
                    let corner_circuit = circuit_with_corner(&circuit, corner)?;
                    Ok(CornerNoisePoint {
                        corner_name: corner.name.clone(),
                        result: noise_ac(
                            &corner_circuit,
                            &output_node,
                            &input_source,
                            &frequencies_hz,
                            temperature_kelvin,
                        )?,
                    })
                })
            })
            .collect::<Vec<_>>();

        let mut points = Vec::with_capacity(handles.len());
        for handle in handles {
            let point = handle.join().map_err(|_| SpiceError::InvalidElement {
                name: "noise_ac_corners_parallel".to_string(),
                reason: "parallel AC noise corner worker panicked".to_string(),
            })??;
            points.push(point);
        }
        Ok(points)
    })?;

    Ok(CornerNoiseResult {
        output_node: output_node.to_string(),
        input_source: input_source.to_string(),
        points,
    })
}

pub fn noise_ac_default(
    circuit: &Circuit,
    output_node: &str,
    input_source: &str,
) -> Result<NoiseResult, SpiceError> {
    noise_ac(circuit, output_node, input_source, &[], 300.0)
}

pub fn transient(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
) -> Result<Vec<TransientPoint>, SpiceError> {
    transient_with_method(circuit, time_step, stop_time, TransientMethod::Euler)
}

pub fn transient_with_method(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    method: TransientMethod,
) -> Result<Vec<TransientPoint>, SpiceError> {
    if !time_step.is_finite() || time_step <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "transient".to_string(),
            reason: "time step must be finite and positive".to_string(),
        });
    }
    if !stop_time.is_finite() || stop_time < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "transient".to_string(),
            reason: "stop time must be finite and non-negative".to_string(),
        });
    }

    validate_reactive_elements(circuit)?;

    let mut capacitor_states = initial_capacitor_states(circuit, time_step, method);
    let mut inductor_states = initial_inductor_states(circuit, time_step, method);
    let mut line_states = initial_transmission_line_states(circuit);
    let initial_circuit = circuit_with_transmission_line_companions(circuit, &line_states, 0.0)?;
    let initial_solution = solve_linear_circuit(
        &initial_circuit,
        &capacitor_states,
        &inductor_states,
        Some(0.0),
    )?;
    seed_device_capacitor_states(
        circuit,
        &initial_solution.node_voltages,
        &mut capacitor_states,
    );
    update_transmission_line_states(
        circuit,
        &initial_solution.node_voltages,
        &mut line_states,
        0.0,
    )?;
    let mut points = Vec::new();
    let mut time = time_step;
    while time <= stop_time + time_step * 1.0e-9 {
        let step_method = if method == TransientMethod::Gear2 && points.is_empty() {
            TransientMethod::Euler
        } else {
            method
        };
        set_reactive_state_method(&mut capacitor_states, &mut inductor_states, step_method);
        let companion_circuit =
            circuit_with_transmission_line_companions(circuit, &line_states, time)?;
        let linear_solution = solve_linear_circuit(
            &companion_circuit,
            &capacitor_states,
            &inductor_states,
            Some(time),
        )?;
        update_capacitor_states(
            circuit,
            &linear_solution.node_voltages,
            &mut capacitor_states,
        );
        update_inductor_states(
            circuit,
            &linear_solution.node_voltages,
            &mut inductor_states,
        );
        let line_currents = update_transmission_line_states(
            circuit,
            &linear_solution.node_voltages,
            &mut line_states,
            time,
        )?;
        let mut branch_currents = linear_solution.branch_currents;
        branch_currents.extend(line_currents);
        points.push(TransientPoint {
            time,
            node_voltages: linear_solution.node_voltages,
            branch_currents,
        });
        time += time_step;
    }
    Ok(points)
}

pub fn transient_corners(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    corners: &[CornerSpec],
) -> Result<CornerTransientResult, SpiceError> {
    transient_corners_with_method(
        circuit,
        time_step,
        stop_time,
        TransientMethod::Euler,
        corners,
    )
}

pub fn transient_corners_with_method(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    method: TransientMethod,
    corners: &[CornerSpec],
) -> Result<CornerTransientResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerTransientPoint {
            corner_name: corner.name.clone(),
            points: transient_with_method(&corner_circuit, time_step, stop_time, method)?,
        });
    }
    Ok(CornerTransientResult { points })
}

pub fn transient_adaptive(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    options: AdaptiveTransientOptions,
) -> Result<AdaptiveTransientResult, SpiceError> {
    if !time_step.is_finite() || time_step <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "transient".to_string(),
            reason: "time step must be finite and positive".to_string(),
        });
    }
    if !stop_time.is_finite() || stop_time < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "transient".to_string(),
            reason: "stop time must be finite and non-negative".to_string(),
        });
    }
    if !options.tolerance.is_finite() || options.tolerance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "transient".to_string(),
            reason: "adaptive tolerance must be finite and non-negative".to_string(),
        });
    }
    let min_step = options.min_step.unwrap_or(time_step / 1_000.0);
    let max_step = options.max_step.unwrap_or(time_step * 10.0);
    if !min_step.is_finite() || min_step <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "transient".to_string(),
            reason: "minimum step must be finite and positive".to_string(),
        });
    }
    if !max_step.is_finite() || max_step < min_step {
        return Err(SpiceError::InvalidElement {
            name: "transient".to_string(),
            reason: "maximum step must be finite and at least the minimum step".to_string(),
        });
    }

    validate_reactive_elements(circuit)?;

    let mut capacitor_states = initial_capacitor_states(circuit, time_step, options.method);
    let mut inductor_states = initial_inductor_states(circuit, time_step, options.method);
    let mut line_states = initial_transmission_line_states(circuit);
    let initial_circuit = circuit_with_transmission_line_companions(circuit, &line_states, 0.0)?;
    let initial_solution = solve_linear_circuit(
        &initial_circuit,
        &capacitor_states,
        &inductor_states,
        Some(0.0),
    )?;
    seed_device_capacitor_states(
        circuit,
        &initial_solution.node_voltages,
        &mut capacitor_states,
    );
    update_transmission_line_states(
        circuit,
        &initial_solution.node_voltages,
        &mut line_states,
        0.0,
    )?;

    let mut points = Vec::new();
    let mut steps_rejected = 0;
    let mut current_time = 0.0;
    let mut step = time_step.min(max_step);
    let mut previous_cap_voltages = capacitor_voltages(circuit, &initial_solution.node_voltages);
    let mut previous_previous_cap_voltages = previous_cap_voltages.clone();

    while current_time < stop_time - time_step * 1.0e-12 {
        let remaining = stop_time - current_time;
        let proposed_step = if remaining <= min_step {
            remaining
        } else {
            step.min(remaining).max(min_step)
        };
        let proposed_time = current_time + proposed_step;
        let step_method = if options.method == TransientMethod::Gear2 && points.is_empty() {
            TransientMethod::Euler
        } else {
            options.method
        };
        set_reactive_state_method(&mut capacitor_states, &mut inductor_states, step_method);
        set_reactive_state_step(&mut capacitor_states, &mut inductor_states, proposed_step);
        let companion_circuit =
            circuit_with_transmission_line_companions(circuit, &line_states, proposed_time)?;
        let linear_solution = solve_linear_circuit(
            &companion_circuit,
            &capacitor_states,
            &inductor_states,
            Some(proposed_time),
        )?;
        let proposed_cap_voltages = capacitor_voltages(circuit, &linear_solution.node_voltages);
        let can_estimate_lte = options.method != TransientMethod::Euler && !points.is_empty();
        let lte = if can_estimate_lte {
            transient_lte_estimate(
                circuit,
                &proposed_cap_voltages,
                &previous_cap_voltages,
                &previous_previous_cap_voltages,
            )
        } else {
            0.0
        };
        if can_estimate_lte && lte > options.tolerance && proposed_step > min_step + 1.0e-20 {
            step = (proposed_step / 2.0).max(min_step);
            steps_rejected += 1;
            continue;
        }

        update_capacitor_states(
            circuit,
            &linear_solution.node_voltages,
            &mut capacitor_states,
        );
        update_inductor_states(
            circuit,
            &linear_solution.node_voltages,
            &mut inductor_states,
        );
        let line_currents = update_transmission_line_states(
            circuit,
            &linear_solution.node_voltages,
            &mut line_states,
            proposed_time,
        )?;
        let mut branch_currents = linear_solution.branch_currents;
        branch_currents.extend(line_currents);
        points.push(TransientPoint {
            time: proposed_time,
            node_voltages: linear_solution.node_voltages,
            branch_currents,
        });
        current_time = proposed_time;
        previous_previous_cap_voltages = previous_cap_voltages;
        previous_cap_voltages = proposed_cap_voltages;
        step = if can_estimate_lte && lte < options.tolerance / 8.0 {
            (proposed_step * 2.0).min(max_step)
        } else {
            proposed_step
        };
    }

    Ok(AdaptiveTransientResult {
        points,
        method: options.method,
        steps_rejected,
        converged: true,
    })
}

pub fn transient_adaptive_corners(
    circuit: &Circuit,
    time_step: f64,
    stop_time: f64,
    options: AdaptiveTransientOptions,
    corners: &[CornerSpec],
) -> Result<CornerAdaptiveTransientResult, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        points.push(CornerAdaptiveTransientPoint {
            corner_name: corner.name.clone(),
            result: transient_adaptive(&corner_circuit, time_step, stop_time, options)?,
        });
    }
    Ok(CornerAdaptiveTransientResult { points })
}

pub fn pss_residual(
    circuit: &Circuit,
    steps_per_period: usize,
) -> Result<Option<PssResidualResult>, SpiceError> {
    pss_residual_with_tolerance(circuit, steps_per_period, 1.0e-6)
}

pub fn pss_residual_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
) -> Result<Option<PssResidualResult>, SpiceError> {
    let Some(period) = estimate_period(circuit) else {
        return Ok(None);
    };
    if steps_per_period == 0 {
        return Err(SpiceError::InvalidElement {
            name: "pss_residual".to_string(),
            reason: "steps per period must be positive".to_string(),
        });
    }
    if !residual_tolerance.is_finite() || residual_tolerance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "pss_residual".to_string(),
            reason: "residual tolerance must be finite and non-negative".to_string(),
        });
    }

    let time_step = period / steps_per_period as f64;
    validate_reactive_elements(circuit)?;
    let initial_solution = solve_linear_circuit(
        circuit,
        &initial_capacitor_states(circuit, time_step, TransientMethod::Euler),
        &initial_inductor_states(circuit, time_step, TransientMethod::Euler),
        Some(0.0),
    )?;
    let points = transient(circuit, time_step, period)?;
    let Some(last) = points.last() else {
        return Ok(Some(PssResidualResult {
            period_seconds: period,
            time_step_seconds: time_step,
            node_residuals: BTreeMap::new(),
            branch_residuals: BTreeMap::new(),
            residual_vector: Vec::new(),
            max_abs_branch_residual: 0.0,
            max_abs_residual: 0.0,
            residual_l2_norm: 0.0,
            residual_rms_norm: 0.0,
            residual_tolerance,
            within_tolerance: false,
        }));
    };

    let mut nodes: Vec<String> = initial_solution
        .node_voltages
        .keys()
        .chain(last.node_voltages.keys())
        .cloned()
        .collect();
    nodes.sort();
    nodes.dedup();

    let mut node_residuals = BTreeMap::new();
    let mut residual_vector = Vec::new();
    let mut max_abs_residual = 0.0;
    for node in nodes {
        let residual = last.node_voltages.get(&node).copied().unwrap_or(0.0)
            - initial_solution
                .node_voltages
                .get(&node)
                .copied()
                .unwrap_or(0.0);
        if residual.abs() > max_abs_residual {
            max_abs_residual = residual.abs();
        }
        residual_vector.push(PssResidualEntry {
            kind: "node".to_string(),
            name: node.clone(),
            value: residual,
        });
        node_residuals.insert(node, residual);
    }

    let mut branches: Vec<String> = initial_solution
        .branch_currents
        .keys()
        .chain(last.branch_currents.keys())
        .cloned()
        .collect();
    branches.sort();
    branches.dedup();

    let mut branch_residuals = BTreeMap::new();
    let mut max_abs_branch_residual = 0.0;
    for branch in branches {
        let residual = last.branch_currents.get(&branch).copied().unwrap_or(0.0)
            - initial_solution
                .branch_currents
                .get(&branch)
                .copied()
                .unwrap_or(0.0);
        if residual.abs() > max_abs_branch_residual {
            max_abs_branch_residual = residual.abs();
        }
        residual_vector.push(PssResidualEntry {
            kind: "branch_current".to_string(),
            name: branch.clone(),
            value: residual,
        });
        branch_residuals.insert(branch, residual);
    }
    if max_abs_branch_residual > max_abs_residual {
        max_abs_residual = max_abs_branch_residual;
    }
    let residual_l2_norm = residual_vector
        .iter()
        .map(|entry| entry.value * entry.value)
        .sum::<f64>()
        .sqrt();
    let residual_rms_norm = if residual_vector.is_empty() {
        0.0
    } else {
        residual_l2_norm / (residual_vector.len() as f64).sqrt()
    };

    Ok(Some(PssResidualResult {
        period_seconds: period,
        time_step_seconds: time_step,
        node_residuals,
        branch_residuals,
        residual_vector,
        max_abs_branch_residual,
        max_abs_residual,
        residual_l2_norm,
        residual_rms_norm,
        residual_tolerance,
        within_tolerance: max_abs_residual <= residual_tolerance,
    }))
}

fn pss_state_vector(circuit: &Circuit) -> Vec<PssStateEntry> {
    let mut state_vector = Vec::new();
    for element in circuit.elements() {
        match element {
            Element::Capacitor(capacitor) => state_vector.push(PssStateEntry {
                kind: "capacitor_voltage".to_string(),
                name: capacitor.name.clone(),
                value: capacitor.initial_voltage,
            }),
            Element::Inductor(inductor) => state_vector.push(PssStateEntry {
                kind: "inductor_current".to_string(),
                name: inductor.name.clone(),
                value: inductor.initial_current,
            }),
            _ => {}
        }
    }
    state_vector
}

fn with_perturbed_pss_state(
    circuit: &Circuit,
    target: &PssStateEntry,
    perturbation: f64,
) -> Circuit {
    let mut perturbed = circuit.clone();
    for element in &mut perturbed.elements {
        match element {
            Element::Capacitor(capacitor)
                if target.kind == "capacitor_voltage" && capacitor.name == target.name =>
            {
                capacitor.initial_voltage += perturbation;
            }
            Element::Inductor(inductor)
                if target.kind == "inductor_current" && inductor.name == target.name =>
            {
                inductor.initial_current += perturbation;
            }
            _ => {}
        }
    }
    perturbed
}

fn with_pss_state_vector(circuit: &Circuit, state_vector: &[PssStateEntry]) -> Circuit {
    let mut candidate = circuit.clone();
    for element in &mut candidate.elements {
        match element {
            Element::Capacitor(capacitor) => {
                if let Some(state) = state_vector
                    .iter()
                    .find(|state| state.kind == "capacitor_voltage" && state.name == capacitor.name)
                {
                    capacitor.initial_voltage = state.value;
                }
            }
            Element::Inductor(inductor) => {
                if let Some(state) = state_vector
                    .iter()
                    .find(|state| state.kind == "inductor_current" && state.name == inductor.name)
                {
                    inductor.initial_current = state.value;
                }
            }
            _ => {}
        }
    }
    candidate
}

pub fn pss_residual_jacobian(
    circuit: &Circuit,
    steps_per_period: usize,
) -> Result<Option<PssResidualJacobianResult>, SpiceError> {
    pss_residual_jacobian_with_tolerance(circuit, steps_per_period, 1.0e-6, 1.0e-6)
}

pub fn pss_residual_jacobian_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
    perturbation: f64,
) -> Result<Option<PssResidualJacobianResult>, SpiceError> {
    if !perturbation.is_finite() || perturbation <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: "pss_residual_jacobian".to_string(),
            reason: "perturbation must be finite and positive".to_string(),
        });
    }

    let Some(residual) =
        pss_residual_with_tolerance(circuit, steps_per_period, residual_tolerance)?
    else {
        return Ok(None);
    };

    let state_vector = pss_state_vector(circuit);
    let mut columns = Vec::new();
    for state in &state_vector {
        let Some(perturbed) = pss_residual_with_tolerance(
            &with_perturbed_pss_state(circuit, state, perturbation),
            steps_per_period,
            residual_tolerance,
        )?
        else {
            return Err(SpiceError::InvalidElement {
                name: "pss_residual_jacobian".to_string(),
                reason: "perturbed circuit no longer has an estimated period".to_string(),
            });
        };
        if perturbed.residual_vector.len() != residual.residual_vector.len() {
            return Err(SpiceError::InvalidElement {
                name: "pss_residual_jacobian".to_string(),
                reason: "perturbed residual vector changed shape".to_string(),
            });
        }

        let mut residual_derivatives = Vec::new();
        for (base_entry, perturbed_entry) in residual
            .residual_vector
            .iter()
            .zip(perturbed.residual_vector.iter())
        {
            if base_entry.kind != perturbed_entry.kind || base_entry.name != perturbed_entry.name {
                return Err(SpiceError::InvalidElement {
                    name: "pss_residual_jacobian".to_string(),
                    reason: "perturbed residual vector changed ordering".to_string(),
                });
            }
            residual_derivatives.push(PssResidualEntry {
                kind: base_entry.kind.clone(),
                name: base_entry.name.clone(),
                value: (perturbed_entry.value - base_entry.value) / perturbation,
            });
        }
        columns.push(PssResidualJacobianColumn {
            state: state.clone(),
            residual_derivatives,
        });
    }

    let jacobian = (0..residual.residual_vector.len())
        .map(|row_index| {
            columns
                .iter()
                .map(|column| column.residual_derivatives[row_index].value)
                .collect()
        })
        .collect();

    Ok(Some(PssResidualJacobianResult {
        residual,
        state_vector,
        perturbation,
        columns,
        jacobian,
    }))
}

fn solve_pss_normal_equations(
    jacobian: &PssResidualJacobianResult,
) -> Result<Vec<f64>, SpiceError> {
    let column_count = jacobian.state_vector.len();
    if column_count == 0 {
        return Ok(Vec::new());
    }

    let mut normal_matrix = vec![vec![0.0; column_count]; column_count];
    let mut normal_rhs = vec![0.0; column_count];
    for (row_index, row) in jacobian.jacobian.iter().enumerate() {
        let residual_value = jacobian.residual.residual_vector[row_index].value;
        for col in 0..column_count {
            normal_rhs[col] -= row[col] * residual_value;
            for other_col in 0..column_count {
                normal_matrix[col][other_col] += row[col] * row[other_col];
            }
        }
    }
    solve_linear_system(normal_matrix, normal_rhs)
}

pub fn pss_newton_update(
    circuit: &Circuit,
    steps_per_period: usize,
) -> Result<Option<PssNewtonUpdateResult>, SpiceError> {
    pss_newton_update_with_tolerance(circuit, steps_per_period, 1.0e-6, 1.0e-6)
}

pub fn pss_newton_update_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
    perturbation: f64,
) -> Result<Option<PssNewtonUpdateResult>, SpiceError> {
    let Some(jacobian) = pss_residual_jacobian_with_tolerance(
        circuit,
        steps_per_period,
        residual_tolerance,
        perturbation,
    )?
    else {
        return Ok(None);
    };

    let update_values = solve_pss_normal_equations(&jacobian)?;
    let state_updates = jacobian
        .state_vector
        .iter()
        .zip(update_values.iter())
        .map(|(state, update)| PssStateEntry {
            kind: state.kind.clone(),
            name: state.name.clone(),
            value: *update,
        })
        .collect::<Vec<_>>();
    let next_state_vector = jacobian
        .state_vector
        .iter()
        .zip(update_values.iter())
        .map(|(state, update)| PssStateEntry {
            kind: state.kind.clone(),
            name: state.name.clone(),
            value: state.value + update,
        })
        .collect::<Vec<_>>();
    let update_l2_norm = update_values
        .iter()
        .map(|update| update * update)
        .sum::<f64>()
        .sqrt();

    Ok(Some(PssNewtonUpdateResult {
        jacobian,
        state_updates,
        next_state_vector,
        update_l2_norm,
    }))
}

pub fn pss_newton_candidate(
    circuit: &Circuit,
    steps_per_period: usize,
) -> Result<Option<PssNewtonCandidateResult>, SpiceError> {
    pss_newton_candidate_with_tolerance(circuit, steps_per_period, 1.0e-6, 1.0e-6)
}

pub fn pss_newton_candidate_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
    perturbation: f64,
) -> Result<Option<PssNewtonCandidateResult>, SpiceError> {
    let Some(update) = pss_newton_update_with_tolerance(
        circuit,
        steps_per_period,
        residual_tolerance,
        perturbation,
    )?
    else {
        return Ok(None);
    };

    let candidate_circuit = with_pss_state_vector(circuit, &update.next_state_vector);
    let Some(candidate_residual) =
        pss_residual_with_tolerance(&candidate_circuit, steps_per_period, residual_tolerance)?
    else {
        return Err(SpiceError::InvalidElement {
            name: "pss_newton_candidate".to_string(),
            reason: "candidate circuit no longer has an estimated period".to_string(),
        });
    };
    let candidate_state_vector = pss_state_vector(&candidate_circuit);

    Ok(Some(PssNewtonCandidateResult {
        update,
        candidate_circuit,
        candidate_state_vector,
        candidate_residual,
    }))
}

pub fn pss_newton_iteration(
    circuit: &Circuit,
    steps_per_period: usize,
) -> Result<Option<PssNewtonIterationResult>, SpiceError> {
    pss_newton_iteration_with_tolerance(circuit, steps_per_period, 1.0e-6, 1.0e-6)
}

pub fn pss_newton_iteration_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
    perturbation: f64,
) -> Result<Option<PssNewtonIterationResult>, SpiceError> {
    let Some(candidate) = pss_newton_candidate_with_tolerance(
        circuit,
        steps_per_period,
        residual_tolerance,
        perturbation,
    )?
    else {
        return Ok(None);
    };

    let base_residual = &candidate.update.jacobian.residual;
    let candidate_residual = &candidate.candidate_residual;
    let base_norm = base_residual.residual_l2_norm;
    let candidate_norm = candidate_residual.residual_l2_norm;
    let accepted = candidate_norm <= base_norm;
    let next_circuit = if accepted {
        candidate.candidate_circuit.clone()
    } else {
        circuit.clone()
    };
    let next_state_vector = if accepted {
        candidate.candidate_state_vector.clone()
    } else {
        candidate.update.jacobian.state_vector.clone()
    };
    let next_residual = if accepted {
        candidate.candidate_residual.clone()
    } else {
        candidate.update.jacobian.residual.clone()
    };
    let residual_l2_ratio = if base_norm > 0.0 {
        candidate_norm / base_norm
    } else {
        0.0
    };

    Ok(Some(PssNewtonIterationResult {
        candidate,
        accepted,
        residual_l2_reduction: base_norm - candidate_norm,
        residual_l2_ratio,
        next_circuit,
        next_state_vector,
        converged: next_residual.within_tolerance,
        next_residual,
    }))
}

pub fn pss_newton_solve(
    circuit: &Circuit,
    steps_per_period: usize,
) -> Result<Option<PssNewtonSolveResult>, SpiceError> {
    pss_newton_solve_with_tolerance(circuit, steps_per_period, 1.0e-6, 1.0e-6, 8)
}

pub fn pss_newton_solve_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
    perturbation: f64,
    max_newton_iterations: usize,
) -> Result<Option<PssNewtonSolveResult>, SpiceError> {
    if max_newton_iterations == 0 {
        return Err(SpiceError::InvalidElement {
            name: "pss_newton_solve".to_string(),
            reason: "max Newton iterations must be positive".to_string(),
        });
    }

    let mut current_circuit = circuit.clone();
    let mut iterations = Vec::new();
    for _ in 0..max_newton_iterations {
        let Some(iteration) = pss_newton_iteration_with_tolerance(
            &current_circuit,
            steps_per_period,
            residual_tolerance,
            perturbation,
        )?
        else {
            return Ok(None);
        };

        current_circuit = iteration.next_circuit.clone();
        let should_stop = iteration.converged || !iteration.accepted;
        iterations.push(iteration);
        if should_stop {
            break;
        }
    }

    let final_iteration = iterations.last().expect("at least one iteration").clone();
    let final_residual = final_iteration.next_residual.clone();
    Ok(Some(PssNewtonSolveResult {
        final_circuit: final_iteration.next_circuit,
        final_state_vector: final_iteration.next_state_vector,
        converged: final_residual.within_tolerance,
        iteration_count: iterations.len(),
        iterations,
        final_residual,
    }))
}

pub fn pss(circuit: &Circuit, steps_per_period: usize) -> Result<Option<PssResult>, SpiceError> {
    pss_with_tolerance(circuit, steps_per_period, 1.0e-6, 1.0e-6, 8)
}

pub fn pss_corners(
    circuit: &Circuit,
    steps_per_period: usize,
    corners: &[CornerSpec],
) -> Result<Option<CornerPssResult>, SpiceError> {
    pss_corners_with_tolerance(circuit, steps_per_period, 1.0e-6, 1.0e-6, 8, corners)
}

pub fn pss_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
    perturbation: f64,
    max_newton_iterations: usize,
) -> Result<Option<PssResult>, SpiceError> {
    let Some(solve) = pss_newton_solve_with_tolerance(
        circuit,
        steps_per_period,
        residual_tolerance,
        perturbation,
        max_newton_iterations,
    )?
    else {
        return Ok(None);
    };

    let steady_state = transient(
        &solve.final_circuit,
        solve.final_residual.time_step_seconds,
        solve.final_residual.period_seconds,
    )?;
    Ok(Some(PssResult {
        period_seconds: solve.final_residual.period_seconds,
        time_step_seconds: solve.final_residual.time_step_seconds,
        converged: solve.converged,
        solve,
        steady_state,
    }))
}

pub fn pss_corners_with_tolerance(
    circuit: &Circuit,
    steps_per_period: usize,
    residual_tolerance: f64,
    perturbation: f64,
    max_newton_iterations: usize,
    corners: &[CornerSpec],
) -> Result<Option<CornerPssResult>, SpiceError> {
    let mut points = Vec::with_capacity(corners.len());
    for corner in corners {
        let corner_circuit = circuit_with_corner(circuit, corner)?;
        let Some(result) = pss_with_tolerance(
            &corner_circuit,
            steps_per_period,
            residual_tolerance,
            perturbation,
            max_newton_iterations,
        )?
        else {
            return Ok(None);
        };
        points.push(CornerPssPoint {
            corner_name: corner.name.clone(),
            result,
        });
    }
    Ok(Some(CornerPssResult { points }))
}

fn validate_sweep(source_name: &str, start: f64, stop: f64, step: f64) -> Result<(), SpiceError> {
    if source_name.is_empty() {
        return Err(SpiceError::InvalidElement {
            name: "dc_sweep".to_string(),
            reason: "source name must not be empty".to_string(),
        });
    }
    if !start.is_finite() || !stop.is_finite() || !step.is_finite() || step == 0.0 {
        return Err(SpiceError::InvalidElement {
            name: source_name.to_string(),
            reason: "sweep bounds and step must be finite, with non-zero step".to_string(),
        });
    }
    if (stop - start).signum() != step.signum() && start != stop {
        return Err(SpiceError::InvalidElement {
            name: source_name.to_string(),
            reason: "sweep step direction must move from start toward stop".to_string(),
        });
    }
    Ok(())
}

fn sweep_includes(value: f64, stop: f64, step: f64, epsilon: f64) -> bool {
    if step > 0.0 {
        value <= stop + epsilon
    } else {
        value >= stop - epsilon
    }
}

fn set_source_value(
    circuit: &mut Circuit,
    source_name: &str,
    value: f64,
) -> Result<(), SpiceError> {
    for element in &mut circuit.elements {
        match element {
            Element::VoltageSource(source) if source.name == source_name => {
                source.voltage = value;
                return Ok(());
            }
            Element::CurrentSource(source) if source.name == source_name => {
                source.current = value;
                return Ok(());
            }
            _ => {}
        }
    }
    Err(SpiceError::InvalidElement {
        name: source_name.to_string(),
        reason: "sweep source must be an independent voltage or current source".to_string(),
    })
}

fn element_parameter(element: &Element) -> Option<(String, String, f64)> {
    match element {
        Element::Resistor(resistor) => Some((
            resistor.name.clone(),
            "resistance_ohms".to_string(),
            resistor.resistance_ohms,
        )),
        Element::VoltageSource(source) => {
            Some((source.name.clone(), "voltage".to_string(), source.voltage))
        }
        Element::CurrentSource(source) => {
            Some((source.name.clone(), "current".to_string(), source.current))
        }
        Element::CustomModel(model) => match model.kind {
            CustomModelKind::LinearConductance {
                conductance_siemens,
                ..
            } => Some((
                model.name.clone(),
                "conductance_siemens".to_string(),
                conductance_siemens,
            )),
        },
        Element::Diode(diode) => Some((
            diode.name.clone(),
            "saturation_current".to_string(),
            diode.saturation_current,
        )),
        Element::Jfet(jfet) => Some((jfet.name.clone(), "beta".to_string(), jfet.beta)),
        Element::Bjt(bjt) => Some((
            bjt.name.clone(),
            "saturation_current".to_string(),
            bjt.saturation_current,
        )),
        Element::Mosfet(mosfet) => Some((mosfet.name.clone(), "kp".to_string(), mosfet.params.kp)),
        Element::Vccs(source) => Some((
            source.name.clone(),
            "transconductance_siemens".to_string(),
            source.transconductance_siemens,
        )),
        Element::Vcvs(source) => Some((source.name.clone(), "gain".to_string(), source.gain)),
        Element::Cccs(source) => Some((source.name.clone(), "gain".to_string(), source.gain)),
        Element::Ccvs(source) => Some((
            source.name.clone(),
            "transresistance_ohms".to_string(),
            source.transresistance_ohms,
        )),
        Element::BSource(_)
        | Element::Capacitor(_)
        | Element::Inductor(_)
        | Element::MutualInductor(_)
        | Element::TransmissionLine(_) => None,
    }
}

fn perturbation_for(value: f64) -> f64 {
    (value.abs() * 1.0e-6).max(1.0e-9)
}

fn perturb_element_parameter(element: &mut Element, delta: f64) {
    match element {
        Element::Resistor(resistor) => resistor.resistance_ohms += delta,
        Element::VoltageSource(source) => source.voltage += delta,
        Element::CurrentSource(source) => source.current += delta,
        Element::CustomModel(model) => match &mut model.kind {
            CustomModelKind::LinearConductance {
                conductance_siemens,
                ..
            } => *conductance_siemens += delta,
        },
        Element::Diode(diode) => diode.saturation_current += delta,
        Element::Jfet(jfet) => jfet.beta += delta,
        Element::Bjt(bjt) => bjt.saturation_current += delta,
        Element::Mosfet(mosfet) => mosfet.params.kp += delta,
        Element::Vccs(source) => source.transconductance_siemens += delta,
        Element::Vcvs(source) => source.gain += delta,
        Element::Cccs(source) => source.gain += delta,
        Element::Ccvs(source) => source.transresistance_ohms += delta,
        Element::BSource(_)
        | Element::Capacitor(_)
        | Element::Inductor(_)
        | Element::MutualInductor(_)
        | Element::TransmissionLine(_) => {}
    }
}

#[derive(Debug, Clone)]
struct McRng {
    state: u64,
}

impl McRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut value = self.state as u32;
        value = (value ^ (value >> 15)).wrapping_mul(value | 1);
        value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
        ((value ^ (value >> 14)) as f64) / 4_294_967_296.0
    }

    fn gaussian(&mut self) -> f64 {
        let u1 = self.next_f64().max(f64::MIN_POSITIVE);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (TWO_PI * u2).cos()
    }
}

fn randomized_value(
    nominal_value: f64,
    tolerance: f64,
    distribution: McDistribution,
    rng: &mut McRng,
) -> f64 {
    if tolerance == 0.0 {
        return nominal_value;
    }
    match distribution {
        McDistribution::Gaussian => nominal_value * (1.0 + rng.gaussian() * tolerance / 3.0),
        McDistribution::Uniform => nominal_value * (1.0 + tolerance * (2.0 * rng.next_f64() - 1.0)),
    }
}

fn circuit_with_randomized_elements(
    circuit: &Circuit,
    tolerance: f64,
    distribution: McDistribution,
    rng: &mut McRng,
) -> Circuit {
    let mut randomized = Circuit::new();
    for element in circuit.elements() {
        randomized.add(randomized_element(element, tolerance, distribution, rng));
    }
    randomized
}

fn randomized_element(
    element: &Element,
    tolerance: f64,
    distribution: McDistribution,
    rng: &mut McRng,
) -> Element {
    match element {
        Element::Resistor(resistor) => {
            let mut varied = resistor.clone();
            varied.resistance_ohms =
                randomized_value(varied.resistance_ohms, tolerance, distribution, rng);
            Element::Resistor(varied)
        }
        Element::VoltageSource(source) => {
            let mut varied = source.clone();
            varied.voltage = randomized_value(varied.voltage, tolerance, distribution, rng);
            Element::VoltageSource(varied)
        }
        Element::CurrentSource(source) => {
            let mut varied = source.clone();
            varied.current = randomized_value(varied.current, tolerance, distribution, rng);
            Element::CurrentSource(varied)
        }
        Element::CustomModel(model) => {
            let mut varied = model.clone();
            match &mut varied.kind {
                CustomModelKind::LinearConductance {
                    conductance_siemens,
                    ..
                } => {
                    *conductance_siemens =
                        randomized_value(*conductance_siemens, tolerance, distribution, rng);
                }
            }
            Element::CustomModel(varied)
        }
        Element::Diode(diode) => {
            let mut varied = diode.clone();
            varied.saturation_current =
                randomized_value(varied.saturation_current, tolerance, distribution, rng);
            Element::Diode(varied)
        }
        Element::Jfet(jfet) => {
            let mut varied = jfet.clone();
            varied.beta = randomized_value(varied.beta, tolerance, distribution, rng);
            Element::Jfet(varied)
        }
        Element::Bjt(bjt) => {
            let mut varied = bjt.clone();
            varied.saturation_current =
                randomized_value(varied.saturation_current, tolerance, distribution, rng);
            Element::Bjt(varied)
        }
        Element::Mosfet(mosfet) => {
            let mut varied = mosfet.clone();
            varied.params.kp = randomized_value(varied.params.kp, tolerance, distribution, rng);
            Element::Mosfet(varied)
        }
        Element::Vccs(source) => {
            let mut varied = source.clone();
            varied.transconductance_siemens = randomized_value(
                varied.transconductance_siemens,
                tolerance,
                distribution,
                rng,
            );
            Element::Vccs(varied)
        }
        Element::Vcvs(source) => {
            let mut varied = source.clone();
            varied.gain = randomized_value(varied.gain, tolerance, distribution, rng);
            Element::Vcvs(varied)
        }
        Element::Cccs(source) => {
            let mut varied = source.clone();
            varied.gain = randomized_value(varied.gain, tolerance, distribution, rng);
            Element::Cccs(varied)
        }
        Element::Ccvs(source) => {
            let mut varied = source.clone();
            varied.transresistance_ohms =
                randomized_value(varied.transresistance_ohms, tolerance, distribution, rng);
            Element::Ccvs(varied)
        }
        Element::BSource(_)
        | Element::Capacitor(_)
        | Element::Inductor(_)
        | Element::MutualInductor(_)
        | Element::TransmissionLine(_) => element.clone(),
    }
}

fn sample_mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = sample_mean(values);
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    variance.sqrt()
}

#[derive(Debug, Clone, PartialEq)]
struct CapacitorState {
    name: String,
    previous_voltage: f64,
    previous_previous_voltage: f64,
    previous_current: f64,
    time_step: f64,
    method: TransientMethod,
}

#[derive(Debug, Clone, PartialEq)]
struct InductorState {
    name: String,
    previous_current: f64,
    previous_previous_current: f64,
    previous_voltage: f64,
    time_step: f64,
    method: TransientMethod,
}

#[derive(Debug, Clone, PartialEq)]
struct TransmissionLineSample {
    time: f64,
    port1_voltage: f64,
    port1_current: f64,
    port2_voltage: f64,
    port2_current: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct TransmissionLineState {
    name: String,
    samples: Vec<TransmissionLineSample>,
}

#[derive(Debug, Clone, PartialEq)]
struct LinearSolution {
    node_voltages: BTreeMap<String, f64>,
    branch_currents: BTreeMap<String, f64>,
    vector: Vec<f64>,
    iterations: usize,
    converged: bool,
    max_delta: f64,
    newton_step_limit: Option<f64>,
    limited_newton_steps: usize,
    minimum_damping_factor: f64,
    solver_profile: LinearSolverProfile,
}

#[derive(Debug, Clone, PartialEq)]
struct SolvedLinearSystem {
    solution: Vec<f64>,
    profile: LinearSolverProfile,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct LinearSolveOptions<'a> {
    max_iterations: usize,
    tolerance: f64,
    initial_vector: Option<&'a [f64]>,
    return_singular_as_unconverged: bool,
    newton_step_limit: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
struct AcSolution {
    node_voltages: BTreeMap<String, Complex>,
    branch_currents: BTreeMap<String, Complex>,
}

#[derive(Debug, Clone, PartialEq)]
struct NoiseSource {
    element_name: String,
    noise_type: NoiseType,
    positive: Option<usize>,
    negative: Option<usize>,
    source_psd: f64,
    frequency_exponent: f64,
}

#[derive(Debug, Copy, Clone)]
enum InputSource<'a> {
    Voltage(&'a VoltageSource),
    Current(&'a CurrentSource),
}

fn solve_linear_circuit(
    circuit: &Circuit,
    capacitor_states: &[CapacitorState],
    inductor_states: &[InductorState],
    source_time: Option<f64>,
) -> Result<LinearSolution, SpiceError> {
    solve_linear_circuit_with_options(
        circuit,
        capacitor_states,
        inductor_states,
        source_time,
        LinearSolveOptions {
            max_iterations: 80,
            tolerance: 1.0e-9,
            initial_vector: None,
            return_singular_as_unconverged: false,
            newton_step_limit: None,
        },
    )
}

fn solve_dc_newton(
    circuit: &Circuit,
    options: DcOpOptions,
    initial_vector: Option<&[f64]>,
) -> Result<LinearSolution, SpiceError> {
    solve_linear_circuit_with_options(
        circuit,
        &[],
        &[],
        None,
        LinearSolveOptions {
            max_iterations: options.max_iterations,
            tolerance: options.tolerance,
            initial_vector,
            return_singular_as_unconverged: true,
            newton_step_limit: options.newton_step_limit,
        },
    )
}

fn solve_linear_circuit_with_options(
    circuit: &Circuit,
    capacitor_states: &[CapacitorState],
    inductor_states: &[InductorState],
    source_time: Option<f64>,
    options: LinearSolveOptions<'_>,
) -> Result<LinearSolution, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    let voltage_sources = collect_voltage_sources(circuit, inductor_states)?;
    let node_count = node_indices.len();
    let branch_count = voltage_sources.len();
    let matrix_size = node_count + branch_count;

    if matrix_size == 0 {
        return Ok(LinearSolution {
            node_voltages: BTreeMap::new(),
            branch_currents: BTreeMap::new(),
            vector: Vec::new(),
            iterations: 0,
            converged: true,
            max_delta: 0.0,
            newton_step_limit: None,
            limited_newton_steps: 0,
            minimum_damping_factor: 1.0,
            solver_profile: empty_solver_profile(0),
        });
    }

    let has_nonlinear = circuit.elements().iter().any(|element| {
        matches!(
            element,
            Element::Diode(_)
                | Element::Jfet(_)
                | Element::Bjt(_)
                | Element::Mosfet(_)
                | Element::BSource(_)
                | Element::CustomModel(_)
        )
    });
    let return_singular_as_unconverged = options.return_singular_as_unconverged && has_nonlinear;
    let mut operating_point = match options.initial_vector {
        Some(vector) if vector.len() == matrix_size => vector.to_vec(),
        _ => vec![0.0; matrix_size],
    };
    let mut solution = solve_linear_circuit_at_operating_point_or_failure(
        circuit,
        capacitor_states,
        inductor_states,
        source_time,
        &node_indices,
        &voltage_sources,
        node_count,
        matrix_size,
        &operating_point,
        return_singular_as_unconverged,
    )?;
    let active_step_limit = if has_nonlinear {
        options.newton_step_limit
    } else {
        None
    };
    let mut limited_newton_steps = 0;
    let mut minimum_damping_factor: f64 = 1.0;
    if !has_nonlinear {
        return Ok(LinearSolution {
            iterations: 1,
            converged: solution.converged,
            ..solution
        });
    }
    if solution.converged {
        let step = limit_newton_step(&operating_point, &solution.vector, active_step_limit);
        if step.limited {
            limited_newton_steps += 1;
            minimum_damping_factor = minimum_damping_factor.min(step.damping_factor);
        }
        let solver_profile = solution.solver_profile.clone();
        solution = linear_solution_from_vector(
            circuit,
            inductor_states,
            &node_indices,
            &voltage_sources,
            node_count,
            &step.vector,
            solution.converged,
            step.max_delta,
            solver_profile,
        );
    }
    solution.newton_step_limit = active_step_limit;
    solution.limited_newton_steps = limited_newton_steps;
    solution.minimum_damping_factor = minimum_damping_factor;

    let mut iterations = 1;
    while iterations < options.max_iterations {
        if !solution.converged {
            return Ok(LinearSolution {
                iterations,
                converged: false,
                max_delta: f64::INFINITY,
                newton_step_limit: active_step_limit,
                limited_newton_steps,
                minimum_damping_factor,
                ..solution
            });
        }
        let delta = solution.max_delta;
        operating_point = solution.vector.clone();
        if delta < options.tolerance {
            return Ok(LinearSolution {
                iterations,
                converged: true,
                max_delta: delta,
                newton_step_limit: active_step_limit,
                limited_newton_steps,
                minimum_damping_factor,
                ..solution
            });
        }
        solution = solve_linear_circuit_at_operating_point_or_failure(
            circuit,
            capacitor_states,
            inductor_states,
            source_time,
            &node_indices,
            &voltage_sources,
            node_count,
            matrix_size,
            &operating_point,
            return_singular_as_unconverged,
        )?;
        if solution.converged {
            let step = limit_newton_step(&operating_point, &solution.vector, active_step_limit);
            if step.limited {
                limited_newton_steps += 1;
                minimum_damping_factor = minimum_damping_factor.min(step.damping_factor);
            }
            let solver_profile = solution.solver_profile.clone();
            solution = linear_solution_from_vector(
                circuit,
                inductor_states,
                &node_indices,
                &voltage_sources,
                node_count,
                &step.vector,
                solution.converged,
                step.max_delta,
                solver_profile,
            );
        }
        solution.newton_step_limit = active_step_limit;
        solution.limited_newton_steps = limited_newton_steps;
        solution.minimum_damping_factor = minimum_damping_factor;
        iterations += 1;
    }

    let delta = solution.max_delta;
    Ok(LinearSolution {
        iterations,
        converged: delta < options.tolerance,
        max_delta: delta,
        newton_step_limit: active_step_limit,
        limited_newton_steps,
        minimum_damping_factor,
        ..solution
    })
}

fn solve_linear_circuit_at_operating_point_or_failure(
    circuit: &Circuit,
    capacitor_states: &[CapacitorState],
    inductor_states: &[InductorState],
    source_time: Option<f64>,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix_size: usize,
    operating_point: &[f64],
    return_singular_as_unconverged: bool,
) -> Result<LinearSolution, SpiceError> {
    match solve_linear_circuit_at_operating_point(
        circuit,
        capacitor_states,
        inductor_states,
        source_time,
        node_indices,
        voltage_sources,
        node_count,
        matrix_size,
        operating_point,
    ) {
        Ok(solution) => Ok(LinearSolution {
            iterations: 1,
            converged: true,
            max_delta: 0.0,
            ..solution
        }),
        Err(SpiceError::SingularMatrix) if return_singular_as_unconverged => {
            Ok(linear_solution_from_vector(
                circuit,
                inductor_states,
                node_indices,
                voltage_sources,
                node_count,
                operating_point,
                false,
                f64::INFINITY,
                empty_solver_profile(matrix_size),
            ))
        }
        Err(error) => Err(error),
    }
}

fn solve_linear_circuit_at_operating_point(
    circuit: &Circuit,
    capacitor_states: &[CapacitorState],
    inductor_states: &[InductorState],
    source_time: Option<f64>,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix_size: usize,
    operating_point: &[f64],
) -> Result<LinearSolution, SpiceError> {
    let mut matrix = vec![vec![0.0; matrix_size]; matrix_size];
    let mut rhs = vec![0.0; matrix_size];
    let inductors = inductor_by_name(circuit);
    let coupled_names = coupled_inductor_names(circuit);
    let has_transient_inductor_states = !inductor_states.is_empty();

    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => stamp_resistor(resistor, node_indices, &mut matrix)?,
            Element::Capacitor(capacitor) => stamp_capacitor(
                capacitor,
                capacitor_states,
                node_indices,
                &mut matrix,
                &mut rhs,
            )?,
            Element::Inductor(inductor) => {
                if !has_transient_inductor_states || !coupled_names.contains(&inductor.name) {
                    stamp_inductor(
                        inductor,
                        inductor_states,
                        node_indices,
                        voltage_sources,
                        node_count,
                        &mut matrix,
                        &mut rhs,
                    )?
                }
            }
            Element::MutualInductor(mutual) => {
                if has_transient_inductor_states {
                    stamp_transient_mutual_inductor(
                        mutual,
                        &inductors,
                        inductor_states,
                        node_indices,
                        &mut matrix,
                        &mut rhs,
                    )?
                }
            }
            Element::TransmissionLine(_) => {}
            Element::VoltageSource(source) => stamp_voltage_source(
                source,
                node_indices,
                voltage_sources,
                node_count,
                source_time,
                &mut matrix,
                &mut rhs,
            )?,
            Element::CurrentSource(source) => {
                stamp_current_source(source, node_indices, source_time, &mut rhs)?
            }
            Element::BSource(source) => stamp_bsource(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
                &mut rhs,
                operating_point,
            )?,
            Element::CustomModel(model) => {
                stamp_custom_model(model, node_indices, &mut matrix, &mut rhs, operating_point)?
            }
            Element::Diode(diode) => stamp_diode(
                diode,
                capacitor_states,
                node_indices,
                &mut matrix,
                &mut rhs,
                operating_point,
            )?,
            Element::Jfet(jfet) => stamp_jfet(
                jfet,
                capacitor_states,
                node_indices,
                &mut matrix,
                &mut rhs,
                operating_point,
            )?,
            Element::Bjt(bjt) => stamp_bjt(
                bjt,
                capacitor_states,
                node_indices,
                &mut matrix,
                &mut rhs,
                operating_point,
            )?,
            Element::Mosfet(mosfet) => stamp_mosfet(
                mosfet,
                capacitor_states,
                node_indices,
                &mut matrix,
                &mut rhs,
                operating_point,
            )?,
            Element::Vccs(source) => stamp_vccs(source, node_indices, &mut matrix)?,
            Element::Vcvs(source) => stamp_vcvs(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
            )?,
            Element::Cccs(source) => {
                stamp_cccs(source, node_indices, voltage_sources, &mut matrix)?
            }
            Element::Ccvs(source) => stamp_ccvs(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
            )?,
        }
    }

    let solved = solve_linear_system_with_profile(matrix, rhs)?;
    Ok(linear_solution_from_vector(
        circuit,
        inductor_states,
        node_indices,
        voltage_sources,
        node_count,
        &solved.solution,
        true,
        0.0,
        solved.profile,
    ))
}

fn linear_solution_from_vector(
    circuit: &Circuit,
    inductor_states: &[InductorState],
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    solution: &[f64],
    converged: bool,
    max_delta: f64,
    solver_profile: LinearSolverProfile,
) -> LinearSolution {
    let node_voltages = node_voltages_from_solution(node_indices, solution);
    let mut branch_currents = BTreeMap::new();
    for (source_name, branch_index) in voltage_sources {
        branch_currents.insert(
            format!("I({source_name})"),
            solution[node_count + *branch_index],
        );
    }
    insert_transient_inductor_currents(
        circuit,
        inductor_states,
        &node_voltages,
        &mut branch_currents,
    );

    LinearSolution {
        node_voltages,
        branch_currents,
        vector: solution.to_vec(),
        iterations: 1,
        converged,
        max_delta,
        newton_step_limit: None,
        limited_newton_steps: 0,
        minimum_damping_factor: 1.0,
        solver_profile,
    }
}

fn max_vector_delta(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (left - right).abs())
        .fold(0.0, f64::max)
}

#[derive(Debug, Clone, PartialEq)]
struct NewtonStepLimitResult {
    vector: Vec<f64>,
    max_delta: f64,
    damping_factor: f64,
    limited: bool,
}

fn limit_newton_step(
    previous: &[f64],
    candidate: &[f64],
    step_limit: Option<f64>,
) -> NewtonStepLimitResult {
    let Some(step_limit) = step_limit else {
        return NewtonStepLimitResult {
            vector: candidate.to_vec(),
            max_delta: max_vector_delta(previous, candidate),
            damping_factor: 1.0,
            limited: false,
        };
    };
    let raw_delta = max_vector_delta(previous, candidate);
    if raw_delta <= step_limit {
        return NewtonStepLimitResult {
            vector: candidate.to_vec(),
            max_delta: raw_delta,
            damping_factor: 1.0,
            limited: false,
        };
    }
    if !raw_delta.is_finite() {
        let vector = previous
            .iter()
            .zip(candidate.iter())
            .map(|(old, new)| {
                let delta = *new - *old;
                if delta.is_finite() {
                    *old + step_limit.copysign(delta)
                } else {
                    *old
                }
            })
            .collect();
        return NewtonStepLimitResult {
            vector,
            max_delta: step_limit,
            damping_factor: 0.0,
            limited: true,
        };
    }

    let damping_factor = step_limit / raw_delta;
    let vector = previous
        .iter()
        .zip(candidate.iter())
        .map(|(old, new)| *old + (*new - *old) * damping_factor)
        .collect();
    NewtonStepLimitResult {
        vector,
        max_delta: step_limit,
        damping_factor,
        limited: true,
    }
}

fn solve_ac_circuit(circuit: &Circuit, omega: f64) -> Result<AcSolution, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    let voltage_sources = collect_ac_voltage_sources(circuit)?;
    let node_count = node_indices.len();
    let branch_count = voltage_sources.len();
    let matrix_size = node_count + branch_count;
    let uses_explicit_ac_sources = circuit_has_explicit_ac_sources(circuit);

    if matrix_size == 0 {
        return Ok(AcSolution {
            node_voltages: BTreeMap::new(),
            branch_currents: BTreeMap::new(),
        });
    }

    let operating_point = if uses_explicit_ac_sources {
        solve_linear_circuit(circuit, &[], &[], None)?.vector
    } else {
        vec![0.0; matrix_size]
    };
    let matrix = build_ac_matrix(
        circuit,
        omega,
        &node_indices,
        &voltage_sources,
        &operating_point,
    )?;
    let mut rhs = vec![Complex::zero(); matrix_size];

    for element in circuit.elements() {
        match element {
            Element::VoltageSource(source) => stamp_ac_voltage_source(
                source,
                &voltage_sources,
                node_count,
                uses_explicit_ac_sources,
                &mut rhs,
            )?,
            Element::CurrentSource(source) => {
                stamp_ac_current_source(source, &node_indices, uses_explicit_ac_sources, &mut rhs)?
            }
            Element::BSource(_) => {}
            Element::Resistor(_)
            | Element::Capacitor(_)
            | Element::Inductor(_)
            | Element::MutualInductor(_)
            | Element::TransmissionLine(_)
            | Element::CustomModel(_)
            | Element::Diode(_)
            | Element::Jfet(_)
            | Element::Bjt(_)
            | Element::Mosfet(_)
            | Element::Vccs(_)
            | Element::Vcvs(_)
            | Element::Cccs(_)
            | Element::Ccvs(_) => {}
        }
    }

    let solution = solve_complex_linear_system(matrix, rhs)?;
    let node_voltages = complex_node_voltages_from_solution(&node_indices, &solution);
    let mut branch_currents = BTreeMap::new();
    for (source_name, branch_index) in voltage_sources {
        branch_currents.insert(
            format!("I({source_name})"),
            solution[node_count + branch_index],
        );
    }

    Ok(AcSolution {
        node_voltages,
        branch_currents,
    })
}

fn build_ac_matrix(
    circuit: &Circuit,
    omega: f64,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    operating_point: &[f64],
) -> Result<Vec<Vec<Complex>>, SpiceError> {
    let node_count = node_indices.len();
    let matrix_size = node_count + voltage_sources.len();
    let mut matrix = vec![vec![Complex::zero(); matrix_size]; matrix_size];
    let inductors = inductor_by_name(circuit);
    let coupled_inductor_names = coupled_inductor_names(circuit);

    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => stamp_ac_resistor(resistor, node_indices, &mut matrix)?,
            Element::Capacitor(capacitor) => {
                stamp_ac_capacitor(capacitor, omega, node_indices, &mut matrix)?
            }
            Element::Inductor(inductor) => {
                if !coupled_inductor_names.contains(&inductor.name) {
                    stamp_ac_inductor(inductor, omega, node_indices, &mut matrix)?
                }
            }
            Element::MutualInductor(mutual) => {
                stamp_ac_mutual_inductor(mutual, &inductors, omega, node_indices, &mut matrix)?
            }
            Element::TransmissionLine(line) => {
                stamp_ac_transmission_line(line, omega, node_indices, &mut matrix)?
            }
            Element::VoltageSource(source) => stamp_ac_voltage_source_matrix(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
            )?,
            Element::CurrentSource(source) => {
                validate_ac_current_source(source)?;
            }
            Element::BSource(source) => stamp_ac_bsource(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
                operating_point,
            )?,
            Element::CustomModel(model) => stamp_complex_conductance(
                &mut matrix,
                node_index(node_indices, &model.positive),
                node_index(node_indices, &model.negative),
                Complex::new(
                    custom_model_conductance(model, node_indices, operating_point)?,
                    0.0,
                ),
            ),
            Element::Diode(diode) => {
                validate_diode(diode)?;
                let intrinsic_anode = diode_intrinsic_anode_node(diode);
                let anode = node_index(node_indices, &intrinsic_anode);
                let cathode = node_index(node_indices, &diode.cathode);
                let voltage = vector_voltage(operating_point, anode)
                    - vector_voltage(operating_point, cathode);
                let (_, conductance) = diode_current_conductance(diode, voltage);
                let capacitance = diode_dynamic_capacitance(diode, voltage);
                stamp_complex_conductance(
                    &mut matrix,
                    anode,
                    cathode,
                    Complex::new(conductance, omega * capacitance),
                );
                if diode.series_resistance > 0.0 {
                    stamp_complex_conductance(
                        &mut matrix,
                        node_index(node_indices, &diode.anode),
                        anode,
                        Complex::new(1.0 / diode.series_resistance, 0.0),
                    );
                }
            }
            Element::Jfet(jfet) => {
                stamp_ac_jfet_small_signal(jfet, node_indices, &mut matrix, operating_point, omega)?
            }
            Element::Bjt(bjt) => {
                stamp_ac_bjt_small_signal(bjt, node_indices, &mut matrix, operating_point, omega)?
            }
            Element::Mosfet(mosfet) => stamp_ac_mosfet_small_signal(
                mosfet,
                node_indices,
                &mut matrix,
                operating_point,
                omega,
            )?,
            Element::Vccs(source) => stamp_ac_vccs(source, node_indices, &mut matrix)?,
            Element::Vcvs(source) => stamp_ac_vcvs(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
            )?,
            Element::Cccs(source) => {
                stamp_ac_cccs(source, node_indices, voltage_sources, &mut matrix)?
            }
            Element::Ccvs(source) => stamp_ac_ccvs(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
            )?,
        }
    }

    Ok(matrix)
}

fn build_small_signal_matrix(
    circuit: &Circuit,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    operating_point: &[f64],
) -> Result<Vec<Vec<f64>>, SpiceError> {
    let node_count = node_indices.len();
    let matrix_size = node_count + voltage_sources.len();
    let mut matrix = vec![vec![0.0; matrix_size]; matrix_size];

    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => {
                stamp_resistor(resistor, node_indices, &mut matrix)?;
            }
            Element::Capacitor(capacitor) => {
                validate_capacitor(capacitor)?;
            }
            Element::Inductor(inductor) => {
                validate_inductor(inductor)?;
                let n1 = node_index(node_indices, &inductor.n1);
                let n2 = node_index(node_indices, &inductor.n2);
                stamp_conductance(&mut matrix, n1, n2, 1.0e12);
            }
            Element::MutualInductor(_) => {}
            Element::TransmissionLine(_) => {}
            Element::VoltageSource(source) => {
                if !source.voltage.is_finite() {
                    return Err(SpiceError::InvalidElement {
                        name: source.name.clone(),
                        reason: "voltage must be finite".to_string(),
                    });
                }
                let branch = node_count + voltage_sources[&source.name];
                let positive = node_index(node_indices, &source.positive);
                let negative = node_index(node_indices, &source.negative);
                stamp_branch_matrix(&mut matrix, branch, positive, negative);
            }
            Element::CurrentSource(source) => {
                if !source.current.is_finite() {
                    return Err(SpiceError::InvalidElement {
                        name: source.name.clone(),
                        reason: "current must be finite".to_string(),
                    });
                }
            }
            Element::BSource(source) => stamp_bsource_small_signal(
                source,
                node_indices,
                voltage_sources,
                node_count,
                &mut matrix,
                operating_point,
            )?,
            Element::CustomModel(model) => stamp_conductance(
                &mut matrix,
                node_index(node_indices, &model.positive),
                node_index(node_indices, &model.negative),
                custom_model_conductance(model, node_indices, operating_point)?,
            ),
            Element::Diode(diode) => {
                validate_diode(diode)?;
                let intrinsic_anode = diode_intrinsic_anode_node(diode);
                let anode = node_index(node_indices, &intrinsic_anode);
                let cathode = node_index(node_indices, &diode.cathode);
                let voltage = vector_voltage(operating_point, anode)
                    - vector_voltage(operating_point, cathode);
                let (_, conductance) = diode_current_conductance(diode, voltage);
                stamp_conductance(&mut matrix, anode, cathode, conductance);
                if diode.series_resistance > 0.0 {
                    stamp_conductance(
                        &mut matrix,
                        node_index(node_indices, &diode.anode),
                        anode,
                        1.0 / diode.series_resistance,
                    );
                }
            }
            Element::Jfet(jfet) => {
                stamp_jfet_small_signal(jfet, node_indices, &mut matrix, operating_point)?
            }
            Element::Bjt(bjt) => {
                stamp_bjt_small_signal(bjt, node_indices, &mut matrix, operating_point)?
            }
            Element::Mosfet(mosfet) => {
                stamp_mosfet_small_signal(mosfet, node_indices, &mut matrix, operating_point)?
            }
            Element::Vccs(source) => {
                stamp_vccs(source, node_indices, &mut matrix)?;
            }
            Element::Vcvs(source) => {
                stamp_vcvs(
                    source,
                    node_indices,
                    voltage_sources,
                    node_count,
                    &mut matrix,
                )?;
            }
            Element::Cccs(source) => {
                stamp_cccs(source, node_indices, voltage_sources, &mut matrix)?;
            }
            Element::Ccvs(source) => {
                stamp_ccvs(
                    source,
                    node_indices,
                    voltage_sources,
                    node_count,
                    &mut matrix,
                )?;
            }
        }
    }

    Ok(matrix)
}

fn node_voltages_from_solution(
    node_indices: &HashMap<String, usize>,
    solution: &[f64],
) -> BTreeMap<String, f64> {
    let mut node_voltages = BTreeMap::new();
    let mut nodes_by_index: Vec<_> = node_indices.iter().collect();
    nodes_by_index.sort_by_key(|(_, index)| **index);
    for (node, index) in nodes_by_index {
        node_voltages.insert(node.clone(), solution[*index]);
    }
    node_voltages
}

fn complex_node_voltages_from_solution(
    node_indices: &HashMap<String, usize>,
    solution: &[Complex],
) -> BTreeMap<String, Complex> {
    let mut node_voltages = BTreeMap::new();
    let mut nodes_by_index: Vec<_> = node_indices.iter().collect();
    nodes_by_index.sort_by_key(|(_, index)| **index);
    for (node, index) in nodes_by_index {
        node_voltages.insert(node.clone(), solution[*index]);
    }
    node_voltages
}

fn collect_node_indices(circuit: &Circuit) -> HashMap<String, usize> {
    let mut names = BTreeMap::new();
    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => {
                insert_node(&mut names, &resistor.n1);
                insert_node(&mut names, &resistor.n2);
            }
            Element::Capacitor(capacitor) => {
                insert_node(&mut names, &capacitor.n1);
                insert_node(&mut names, &capacitor.n2);
            }
            Element::Inductor(inductor) => {
                insert_node(&mut names, &inductor.n1);
                insert_node(&mut names, &inductor.n2);
            }
            Element::MutualInductor(_) => {}
            Element::TransmissionLine(line) => {
                insert_node(&mut names, &line.n1);
                insert_node(&mut names, &line.n2);
                insert_node(&mut names, &line.n3);
                insert_node(&mut names, &line.n4);
            }
            Element::VoltageSource(source) => {
                insert_node(&mut names, &source.positive);
                insert_node(&mut names, &source.negative);
            }
            Element::CurrentSource(source) => {
                insert_node(&mut names, &source.positive);
                insert_node(&mut names, &source.negative);
            }
            Element::BSource(source) => {
                insert_node(&mut names, &source.positive);
                insert_node(&mut names, &source.negative);
                let expr = source
                    .voltage_expr
                    .as_deref()
                    .or(source.current_expr.as_deref())
                    .unwrap_or("");
                for node in bsource_expr_nodes(expr) {
                    insert_node(&mut names, &node);
                }
            }
            Element::CustomModel(model) => {
                insert_node(&mut names, &model.positive);
                insert_node(&mut names, &model.negative);
            }
            Element::Diode(diode) => {
                insert_node(&mut names, &diode.anode);
                insert_node(&mut names, &diode.cathode);
                if diode.series_resistance > 0.0 {
                    insert_node(&mut names, &diode_intrinsic_anode_node(diode));
                }
            }
            Element::Jfet(jfet) => {
                insert_node(&mut names, &jfet.drain);
                insert_node(&mut names, &jfet.gate);
                insert_node(&mut names, &jfet.source);
                if jfet.drain_resistance > 0.0 {
                    insert_node(&mut names, &jfet_intrinsic_drain_node(jfet));
                }
                if jfet.source_resistance > 0.0 {
                    insert_node(&mut names, &jfet_intrinsic_source_node(jfet));
                }
            }
            Element::Bjt(bjt) => {
                insert_node(&mut names, &bjt.collector);
                insert_node(&mut names, &bjt.base);
                insert_node(&mut names, &bjt.emitter);
                if bjt.emitter_resistance > 0.0 {
                    insert_node(&mut names, &bjt_intrinsic_emitter_node(bjt));
                }
                if bjt.collector_resistance > 0.0 {
                    insert_node(&mut names, &bjt_intrinsic_collector_node(bjt));
                }
                if bjt.base_resistance > 0.0 {
                    insert_node(&mut names, &bjt_intrinsic_base_node(bjt));
                }
            }
            Element::Mosfet(mosfet) => {
                insert_node(&mut names, &mosfet.drain);
                insert_node(&mut names, &mosfet.gate);
                insert_node(&mut names, &mosfet.source);
                insert_node(&mut names, &mosfet.body);
                if mosfet_drain_resistance(mosfet) > 0.0 {
                    insert_node(&mut names, &mosfet_intrinsic_drain_node(mosfet));
                }
                if mosfet_source_resistance(mosfet) > 0.0 {
                    insert_node(&mut names, &mosfet_intrinsic_source_node(mosfet));
                }
            }
            Element::Vccs(source) => {
                insert_node(&mut names, &source.positive);
                insert_node(&mut names, &source.negative);
                insert_node(&mut names, &source.control_positive);
                insert_node(&mut names, &source.control_negative);
            }
            Element::Vcvs(source) => {
                insert_node(&mut names, &source.positive);
                insert_node(&mut names, &source.negative);
                insert_node(&mut names, &source.control_positive);
                insert_node(&mut names, &source.control_negative);
            }
            Element::Cccs(source) => {
                insert_node(&mut names, &source.positive);
                insert_node(&mut names, &source.negative);
            }
            Element::Ccvs(source) => {
                insert_node(&mut names, &source.positive);
                insert_node(&mut names, &source.negative);
            }
        }
    }
    names
        .keys()
        .enumerate()
        .map(|(index, node)| (node.clone(), index))
        .collect()
}

fn collect_voltage_sources(
    circuit: &Circuit,
    inductor_states: &[InductorState],
) -> Result<BTreeMap<String, usize>, SpiceError> {
    let mut sources = BTreeMap::new();
    for element in circuit.elements() {
        match element {
            Element::VoltageSource(source) => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            Element::Vcvs(source) => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            Element::Ccvs(source) => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            Element::BSource(source) if source.voltage_expr.is_some() => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            Element::Inductor(inductor) => {
                if sources.contains_key(&inductor.name) {
                    return Err(SpiceError::InvalidElement {
                        name: inductor.name.clone(),
                        reason: "duplicate branch element name".to_string(),
                    });
                }
                if !inductor_states
                    .iter()
                    .any(|state| state.name == inductor.name)
                {
                    sources.insert(inductor.name.clone(), sources.len());
                }
            }
            _ => {}
        }
    }
    Ok(sources)
}

fn collect_ac_voltage_sources(circuit: &Circuit) -> Result<BTreeMap<String, usize>, SpiceError> {
    let mut sources = BTreeMap::new();
    for element in circuit.elements() {
        match element {
            Element::VoltageSource(source) => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            Element::Vcvs(source) => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            Element::Ccvs(source) => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            Element::BSource(source) if source.voltage_expr.is_some() => {
                insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
            }
            _ => {}
        }
    }
    Ok(sources)
}

fn find_input_source<'a>(
    circuit: &'a Circuit,
    input_source: &str,
) -> Result<InputSource<'a>, SpiceError> {
    for element in circuit.elements() {
        match element {
            Element::VoltageSource(source) if source.name == input_source => {
                return Ok(InputSource::Voltage(source));
            }
            Element::CurrentSource(source) if source.name == input_source => {
                return Ok(InputSource::Current(source));
            }
            Element::BSource(source) if source.name == input_source => {
                return Err(input_source_type_error(input_source, "B-source"));
            }
            Element::CustomModel(model) if model.name == input_source => {
                return Err(input_source_type_error(input_source, "custom-model"));
            }
            Element::Resistor(resistor) if resistor.name == input_source => {
                return Err(input_source_type_error(input_source, "resistor"));
            }
            Element::Capacitor(capacitor) if capacitor.name == input_source => {
                return Err(input_source_type_error(input_source, "capacitor"));
            }
            Element::Inductor(inductor) if inductor.name == input_source => {
                return Err(input_source_type_error(input_source, "inductor"));
            }
            Element::Diode(diode) if diode.name == input_source => {
                return Err(input_source_type_error(input_source, "diode"));
            }
            Element::Bjt(bjt) if bjt.name == input_source => {
                return Err(input_source_type_error(input_source, "BJT"));
            }
            Element::Mosfet(mosfet) if mosfet.name == input_source => {
                return Err(input_source_type_error(input_source, "MOSFET"));
            }
            Element::Vccs(source) if source.name == input_source => {
                return Err(input_source_type_error(input_source, "VCCS"));
            }
            Element::Vcvs(source) if source.name == input_source => {
                return Err(input_source_type_error(input_source, "VCVS"));
            }
            Element::Cccs(source) if source.name == input_source => {
                return Err(input_source_type_error(input_source, "CCCS"));
            }
            Element::Ccvs(source) if source.name == input_source => {
                return Err(input_source_type_error(input_source, "CCVS"));
            }
            _ => {}
        }
    }
    Err(SpiceError::InvalidElement {
        name: input_source.to_string(),
        reason: "input source was not found".to_string(),
    })
}

fn input_source_type_error(input_source: &str, kind: &str) -> SpiceError {
    SpiceError::InvalidElement {
        name: input_source.to_string(),
        reason: format!(
            "input element must be an independent voltage or current source, got {kind}"
        ),
    }
}

fn jfet_channel_noise_conductance(jfet: &Jfet, vgs: f64, vds: f64, gm: f64) -> f64 {
    if jfet.noise_equation_level < 3.0 {
        return MOSFET_CHANNEL_NOISE_GAMMA * gm.abs();
    }
    let (vgs, vds, threshold_voltage) = match jfet.polarity {
        JfetPolarity::Njf => (vgs, vds, jfet.threshold_voltage),
        JfetPolarity::Pjf => (-vgs, -vds, -jfet.threshold_voltage),
    };
    let overdrive = vgs - threshold_voltage;
    if overdrive <= 0.0 || vds < 0.0 {
        return 0.0;
    }
    let alpha = if overdrive >= vds {
        1.0 - vds / overdrive
    } else {
        0.0
    };
    MOSFET_CHANNEL_NOISE_GAMMA * jfet.beta * overdrive * (1.0 + alpha + alpha * alpha)
        / (1.0 + alpha)
        * jfet.channel_noise_coefficient
}

fn collect_noise_sources(
    circuit: &Circuit,
    node_indices: &HashMap<String, usize>,
    operating_point: &[f64],
    temperature_kelvin: f64,
) -> Result<Vec<NoiseSource>, SpiceError> {
    let mut sources = Vec::new();
    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => {
                if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
                    return Err(SpiceError::InvalidElement {
                        name: resistor.name.clone(),
                        reason: "resistance must be finite and positive".to_string(),
                    });
                }
                sources.push(NoiseSource {
                    element_name: resistor.name.clone(),
                    noise_type: NoiseType::Thermal,
                    positive: node_index(node_indices, &resistor.n1),
                    negative: node_index(node_indices, &resistor.n2),
                    source_psd: 4.0 * BOLTZMANN * temperature_kelvin / resistor.resistance_ohms,
                    frequency_exponent: 0.0,
                });
            }
            Element::Diode(diode) => {
                validate_diode(diode)?;
                let intrinsic_anode = diode_intrinsic_anode_node(diode);
                let anode = node_index(node_indices, &intrinsic_anode);
                let cathode = node_index(node_indices, &diode.cathode);
                let anode_voltage = vector_voltage(operating_point, anode);
                let cathode_voltage = vector_voltage(operating_point, cathode);
                let (current, _) =
                    diode_current_conductance(diode, anode_voltage - cathode_voltage);
                sources.push(NoiseSource {
                    element_name: diode.name.clone(),
                    noise_type: NoiseType::Shot,
                    positive: anode,
                    negative: cathode,
                    source_psd: 2.0 * ELECTRON_CHARGE * current.abs(),
                    frequency_exponent: 0.0,
                });
                if diode.flicker_noise_coefficient > 0.0 {
                    sources.push(NoiseSource {
                        element_name: diode.name.clone(),
                        noise_type: NoiseType::Flicker,
                        positive: anode,
                        negative: cathode,
                        source_psd: diode.flicker_noise_coefficient
                            * current.abs().powf(diode.flicker_noise_exponent),
                        frequency_exponent: 1.0,
                    });
                }
                if diode.series_resistance > 0.0 {
                    sources.push(NoiseSource {
                        element_name: format!("{}:RS", diode.name),
                        noise_type: NoiseType::Thermal,
                        positive: node_index(node_indices, &diode.anode),
                        negative: anode,
                        source_psd: 4.0 * BOLTZMANN * temperature_kelvin / diode.series_resistance,
                        frequency_exponent: 0.0,
                    });
                }
            }
            Element::Bjt(bjt) => {
                validate_bjt(bjt)?;
                let intrinsic_bjt = if bjt.emitter_resistance > 0.0
                    || bjt.collector_resistance > 0.0
                    || bjt.base_resistance > 0.0
                {
                    let mut intrinsic = bjt.clone();
                    if bjt.emitter_resistance > 0.0 {
                        let intrinsic_emitter = bjt_intrinsic_emitter_node(bjt);
                        sources.push(NoiseSource {
                            element_name: format!("{}:RE", bjt.name),
                            noise_type: NoiseType::Thermal,
                            positive: node_index(node_indices, &bjt.emitter),
                            negative: node_index(node_indices, &intrinsic_emitter),
                            source_psd: 4.0 * BOLTZMANN * temperature_kelvin
                                / bjt.emitter_resistance,
                            frequency_exponent: 0.0,
                        });
                        intrinsic.emitter = intrinsic_emitter;
                        intrinsic.emitter_resistance = 0.0;
                    }
                    if bjt.collector_resistance > 0.0 {
                        let intrinsic_collector = bjt_intrinsic_collector_node(bjt);
                        sources.push(NoiseSource {
                            element_name: format!("{}:RC", bjt.name),
                            noise_type: NoiseType::Thermal,
                            positive: node_index(node_indices, &bjt.collector),
                            negative: node_index(node_indices, &intrinsic_collector),
                            source_psd: 4.0 * BOLTZMANN * temperature_kelvin
                                / bjt.collector_resistance,
                            frequency_exponent: 0.0,
                        });
                        intrinsic.collector = intrinsic_collector;
                        intrinsic.collector_resistance = 0.0;
                    }
                    if bjt.base_resistance > 0.0 {
                        let intrinsic_base = bjt_intrinsic_base_node(bjt);
                        let intrinsic_base_index = node_index(node_indices, &intrinsic_base);
                        let base_voltage = vector_voltage(operating_point, intrinsic_base_index);
                        let emitter_voltage = vector_voltage(
                            operating_point,
                            node_index(node_indices, &intrinsic.emitter),
                        );
                        let collector_voltage = vector_voltage(
                            operating_point,
                            node_index(node_indices, &intrinsic.collector),
                        );
                        let base_resistance = bjt_effective_base_resistance(
                            &intrinsic,
                            base_voltage,
                            emitter_voltage,
                            collector_voltage,
                        );
                        sources.push(NoiseSource {
                            element_name: format!("{}:RB", bjt.name),
                            noise_type: NoiseType::Thermal,
                            positive: node_index(node_indices, &bjt.base),
                            negative: intrinsic_base_index,
                            source_psd: 4.0 * BOLTZMANN * temperature_kelvin / base_resistance,
                            frequency_exponent: 0.0,
                        });
                        intrinsic.base = intrinsic_base;
                        intrinsic.base_resistance = 0.0;
                        intrinsic.minimum_base_resistance = None;
                        intrinsic.base_resistance_half_current = 0.0;
                    }
                    Some(intrinsic)
                } else {
                    None
                };
                let bjt = intrinsic_bjt.as_ref().unwrap_or(bjt);
                let base = node_index(node_indices, &bjt.base);
                let emitter = node_index(node_indices, &bjt.emitter);
                let collector = node_index(node_indices, &bjt.collector);
                let base_voltage = vector_voltage(operating_point, base);
                let emitter_voltage = vector_voltage(operating_point, emitter);
                let collector_voltage = vector_voltage(operating_point, collector);
                let junction_voltage = match bjt.polarity {
                    BjtPolarity::Npn => base_voltage - emitter_voltage,
                    BjtPolarity::Pnp => emitter_voltage - base_voltage,
                };
                let reverse_junction_voltage = match bjt.polarity {
                    BjtPolarity::Npn => base_voltage - collector_voltage,
                    BjtPolarity::Pnp => collector_voltage - base_voltage,
                };
                let forward_thermal_voltage =
                    bjt.thermal_voltage * bjt.forward_emission_coefficient;
                let exponent = (junction_voltage / forward_thermal_voltage).clamp(-40.0, 40.0);
                let output_voltage = match bjt.polarity {
                    BjtPolarity::Npn => collector_voltage - emitter_voltage,
                    BjtPolarity::Pnp => emitter_voltage - collector_voltage,
                };
                let early_factor = bjt_early_factor(bjt, junction_voltage, output_voltage);
                let exp_value = exponent.exp();
                let base_collector_current = bjt.saturation_current * (exp_value - 1.0);
                let base_gm = bjt.saturation_current / forward_thermal_voltage * exp_value;
                let (collector_current, _, _) =
                    bjt_forward_transport(bjt, base_collector_current, base_gm, early_factor);
                let (leakage_current, _) = bjt_base_emitter_leakage(bjt, junction_voltage);
                let (collector_leakage_current, _) =
                    bjt_base_collector_leakage(bjt, reverse_junction_voltage);
                let (reverse_base_current, _) =
                    bjt_reverse_base_current(bjt, reverse_junction_voltage);
                let (positive, negative) = match bjt.polarity {
                    BjtPolarity::Npn => (base, emitter),
                    BjtPolarity::Pnp => (emitter, base),
                };
                sources.push(NoiseSource {
                    element_name: bjt.name.clone(),
                    noise_type: NoiseType::Shot,
                    positive,
                    negative,
                    source_psd: 2.0
                        * ELECTRON_CHARGE
                        * (collector_current.abs()
                            + leakage_current.abs()
                            + collector_leakage_current.abs()
                            + reverse_base_current.abs()),
                    frequency_exponent: 0.0,
                });
                if bjt.flicker_noise_coefficient > 0.0 {
                    let base_current = base_collector_current / bjt.forward_beta + leakage_current;
                    sources.push(NoiseSource {
                        element_name: bjt.name.clone(),
                        noise_type: NoiseType::Flicker,
                        positive,
                        negative,
                        source_psd: bjt.flicker_noise_coefficient
                            * base_current.abs().powf(bjt.flicker_noise_exponent),
                        frequency_exponent: 1.0,
                    });
                }
            }
            Element::Jfet(jfet) => {
                validate_jfet(jfet)?;
                let intrinsic_drain = jfet_intrinsic_drain_node(jfet);
                let intrinsic_source = jfet_intrinsic_source_node(jfet);
                let drain = node_index(node_indices, &intrinsic_drain);
                let gate = node_index(node_indices, &jfet.gate);
                let source = node_index(node_indices, &intrinsic_source);
                let drain_voltage = vector_voltage(operating_point, drain);
                let gate_voltage = vector_voltage(operating_point, gate);
                let source_voltage = vector_voltage(operating_point, source);
                let result = evaluate_jfet(
                    jfet,
                    gate_voltage - source_voltage,
                    drain_voltage - source_voltage,
                );
                let gm = result.gm.max(0.0);
                let noise_conductance = jfet_channel_noise_conductance(
                    jfet,
                    gate_voltage - source_voltage,
                    drain_voltage - source_voltage,
                    gm,
                );
                if noise_conductance > 0.0 {
                    sources.push(NoiseSource {
                        element_name: jfet.name.clone(),
                        noise_type: NoiseType::Thermal,
                        positive: drain,
                        negative: source,
                        source_psd: 4.0 * BOLTZMANN * temperature_kelvin * noise_conductance,
                        frequency_exponent: 0.0,
                    });
                }
                let (gate_source_current, _) =
                    jfet_gate_junction_current_conductance(jfet, gate_voltage - source_voltage);
                let (gate_drain_current, _) =
                    jfet_gate_junction_current_conductance(jfet, gate_voltage - drain_voltage);
                sources.push(NoiseSource {
                    element_name: format!("{}:IGS", jfet.name),
                    noise_type: NoiseType::Shot,
                    positive: gate,
                    negative: source,
                    source_psd: 2.0 * ELECTRON_CHARGE * gate_source_current.abs(),
                    frequency_exponent: 0.0,
                });
                sources.push(NoiseSource {
                    element_name: format!("{}:IGD", jfet.name),
                    noise_type: NoiseType::Shot,
                    positive: gate,
                    negative: drain,
                    source_psd: 2.0 * ELECTRON_CHARGE * gate_drain_current.abs(),
                    frequency_exponent: 0.0,
                });
                if jfet.flicker_noise_coefficient > 0.0 {
                    sources.push(NoiseSource {
                        element_name: jfet.name.clone(),
                        noise_type: NoiseType::Flicker,
                        positive: drain,
                        negative: source,
                        source_psd: jfet.flicker_noise_coefficient
                            * result.drain_current.abs().powf(jfet.flicker_noise_exponent),
                        frequency_exponent: 1.0,
                    });
                }
                if jfet.drain_resistance > 0.0 {
                    sources.push(NoiseSource {
                        element_name: format!("{}:RD", jfet.name),
                        noise_type: NoiseType::Thermal,
                        positive: node_index(node_indices, &jfet.drain),
                        negative: drain,
                        source_psd: 4.0 * BOLTZMANN * temperature_kelvin / jfet.drain_resistance,
                        frequency_exponent: 0.0,
                    });
                }
                if jfet.source_resistance > 0.0 {
                    sources.push(NoiseSource {
                        element_name: format!("{}:RS", jfet.name),
                        noise_type: NoiseType::Thermal,
                        positive: node_index(node_indices, &jfet.source),
                        negative: source,
                        source_psd: 4.0 * BOLTZMANN * temperature_kelvin / jfet.source_resistance,
                        frequency_exponent: 0.0,
                    });
                }
            }
            Element::Mosfet(mosfet) => {
                validate_mosfet(mosfet)?;
                let intrinsic_drain = mosfet_intrinsic_drain_node(mosfet);
                let intrinsic_source = mosfet_intrinsic_source_node(mosfet);
                let drain = node_index(node_indices, &intrinsic_drain);
                let gate = node_index(node_indices, &mosfet.gate);
                let source = node_index(node_indices, &intrinsic_source);
                let body = node_index(node_indices, &mosfet.body);
                let drain_voltage = vector_voltage(operating_point, drain);
                let gate_voltage = vector_voltage(operating_point, gate);
                let source_voltage = vector_voltage(operating_point, source);
                let body_voltage = vector_voltage(operating_point, body);
                let result = evaluate_mosfet_level1(
                    mosfet,
                    gate_voltage - source_voltage,
                    drain_voltage - source_voltage,
                    body_voltage - source_voltage,
                );
                let gm = result.gm.max(0.0);
                if gm > 0.0 {
                    sources.push(NoiseSource {
                        element_name: mosfet.name.clone(),
                        noise_type: NoiseType::Thermal,
                        positive: drain,
                        negative: source,
                        source_psd: 4.0
                            * BOLTZMANN
                            * temperature_kelvin
                            * MOSFET_CHANNEL_NOISE_GAMMA
                            * gm,
                        frequency_exponent: 0.0,
                    });
                }
                if mosfet.params.flicker_noise_coefficient > 0.0 {
                    sources.push(NoiseSource {
                        element_name: mosfet.name.clone(),
                        noise_type: NoiseType::Flicker,
                        positive: drain,
                        negative: source,
                        source_psd: mosfet.params.flicker_noise_coefficient
                            * result
                                .drain_current
                                .abs()
                                .powf(mosfet.params.flicker_noise_exponent),
                        frequency_exponent: 1.0,
                    });
                }
                let (source_bulk_current, _) = mosfet_bulk_junction_current_conductance(
                    mosfet,
                    source_voltage,
                    body_voltage,
                    mosfet.params.source_area,
                );
                let (drain_bulk_current, _) = mosfet_bulk_junction_current_conductance(
                    mosfet,
                    drain_voltage,
                    body_voltage,
                    mosfet.params.drain_area,
                );
                let (source_bulk_positive, source_bulk_negative) = match mosfet.mosfet_type {
                    MosfetType::Nmos => (body, source),
                    MosfetType::Pmos => (source, body),
                };
                let (drain_bulk_positive, drain_bulk_negative) = match mosfet.mosfet_type {
                    MosfetType::Nmos => (body, drain),
                    MosfetType::Pmos => (drain, body),
                };
                sources.push(NoiseSource {
                    element_name: format!("{}:IBS", mosfet.name),
                    noise_type: NoiseType::Shot,
                    positive: source_bulk_positive,
                    negative: source_bulk_negative,
                    source_psd: 2.0 * ELECTRON_CHARGE * source_bulk_current.abs(),
                    frequency_exponent: 0.0,
                });
                sources.push(NoiseSource {
                    element_name: format!("{}:IBD", mosfet.name),
                    noise_type: NoiseType::Shot,
                    positive: drain_bulk_positive,
                    negative: drain_bulk_negative,
                    source_psd: 2.0 * ELECTRON_CHARGE * drain_bulk_current.abs(),
                    frequency_exponent: 0.0,
                });
                let drain_resistance = mosfet_drain_resistance(mosfet);
                if drain_resistance > 0.0 {
                    sources.push(NoiseSource {
                        element_name: format!("{}:RD", mosfet.name),
                        noise_type: NoiseType::Thermal,
                        positive: node_index(node_indices, &mosfet.drain),
                        negative: drain,
                        source_psd: 4.0 * BOLTZMANN * temperature_kelvin / drain_resistance,
                        frequency_exponent: 0.0,
                    });
                }
                let source_resistance = mosfet_source_resistance(mosfet);
                if source_resistance > 0.0 {
                    sources.push(NoiseSource {
                        element_name: format!("{}:RS", mosfet.name),
                        noise_type: NoiseType::Thermal,
                        positive: node_index(node_indices, &mosfet.source),
                        negative: source,
                        source_psd: 4.0 * BOLTZMANN * temperature_kelvin / source_resistance,
                        frequency_exponent: 0.0,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(sources)
}

fn noise_source_psd(source: &NoiseSource, frequency_hz: f64) -> f64 {
    source.source_psd / frequency_hz.powf(source.frequency_exponent)
}

fn zero_noise_entries(sources: &[NoiseSource], frequency_hz: f64) -> Vec<NoiseEntry> {
    sources
        .iter()
        .map(|source| NoiseEntry {
            element_name: source.element_name.clone(),
            noise_type: source.noise_type,
            source_psd: noise_source_psd(source, frequency_hz),
            output_psd: 0.0,
        })
        .collect()
}

fn default_noise_frequencies() -> Vec<f64> {
    (0..50)
        .map(|index| 10.0_f64.powf(6.0 * index as f64 / 49.0))
        .collect()
}

fn adjoint_input_gain(
    input: InputSource<'_>,
    adjoint: &[Complex],
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
) -> Result<Complex, SpiceError> {
    match input {
        InputSource::Voltage(source) => {
            let Some(source_index) = voltage_sources.get(&source.name) else {
                return Err(SpiceError::InvalidElement {
                    name: source.name.clone(),
                    reason: "voltage source was not indexed".to_string(),
                });
            };
            Ok(adjoint[node_count + source_index])
        }
        InputSource::Current(source) => {
            let h_positive = node_index(node_indices, &source.positive)
                .map_or(Complex::zero(), |index| adjoint[index]);
            let h_negative = node_index(node_indices, &source.negative)
                .map_or(Complex::zero(), |index| adjoint[index]);
            Ok(h_negative - h_positive)
        }
    }
}

fn insert_branch_name(
    sources: &mut BTreeMap<String, usize>,
    name: &str,
    duplicate_reason: &str,
) -> Result<(), SpiceError> {
    if sources.contains_key(name) {
        return Err(SpiceError::InvalidElement {
            name: name.to_string(),
            reason: duplicate_reason.to_string(),
        });
    }
    sources.insert(name.to_string(), sources.len());
    Ok(())
}

fn insert_node(nodes: &mut BTreeMap<String, ()>, node: &str) {
    if !is_ground(node) {
        nodes.insert(node.to_string(), ());
    }
}

fn is_ground(node: &str) -> bool {
    node == "0" || node.eq_ignore_ascii_case("gnd")
}

fn node_index(node_indices: &HashMap<String, usize>, node: &str) -> Option<usize> {
    if is_ground(node) {
        None
    } else {
        node_indices.get(node).copied()
    }
}

fn stamp_resistor(
    resistor: &Resistor,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
) -> Result<(), SpiceError> {
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "resistance must be finite and positive".to_string(),
        });
    }

    let conductance = 1.0 / resistor.resistance_ohms;
    let n1 = node_index(node_indices, &resistor.n1);
    let n2 = node_index(node_indices, &resistor.n2);
    stamp_conductance(matrix, n1, n2, conductance);
    Ok(())
}

fn stamp_capacitor(
    capacitor: &Capacitor,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    validate_capacitor(capacitor)?;
    let Some(state) = capacitor_states
        .iter()
        .find(|state| state.name == capacitor.name)
    else {
        return Ok(());
    };

    let conductance = match state.method {
        TransientMethod::Trap => 2.0 * capacitor.capacitance_farads / state.time_step,
        TransientMethod::Gear2 => 3.0 * capacitor.capacitance_farads / (2.0 * state.time_step),
        TransientMethod::Euler => capacitor.capacitance_farads / state.time_step,
    };
    let n1 = node_index(node_indices, &capacitor.n1);
    let n2 = node_index(node_indices, &capacitor.n2);
    stamp_conductance(matrix, n1, n2, conductance);

    let history_current = match state.method {
        TransientMethod::Trap => conductance * state.previous_voltage + state.previous_current,
        TransientMethod::Gear2 => {
            capacitor.capacitance_farads
                * (4.0 * state.previous_voltage - state.previous_previous_voltage)
                / (2.0 * state.time_step)
        }
        TransientMethod::Euler => conductance * state.previous_voltage,
    };
    if let Some(i) = n1 {
        rhs[i] += history_current;
    }
    if let Some(j) = n2 {
        rhs[j] -= history_current;
    }
    Ok(())
}

fn stamp_inductor(
    inductor: &Inductor,
    inductor_states: &[InductorState],
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    validate_inductor(inductor)?;
    let n1 = node_index(node_indices, &inductor.n1);
    let n2 = node_index(node_indices, &inductor.n2);
    let Some(state) = inductor_states
        .iter()
        .find(|state| state.name == inductor.name)
    else {
        stamp_zero_voltage_branch(&inductor.name, voltage_sources, node_count, matrix, n1, n2)?;
        return Ok(());
    };

    let conductance = match state.method {
        TransientMethod::Trap => state.time_step / (2.0 * inductor.inductance_henrys),
        TransientMethod::Gear2 => 2.0 * state.time_step / (3.0 * inductor.inductance_henrys),
        TransientMethod::Euler => state.time_step / inductor.inductance_henrys,
    };
    stamp_conductance(matrix, n1, n2, conductance);
    let history_current = match state.method {
        TransientMethod::Trap => state.previous_current + conductance * state.previous_voltage,
        TransientMethod::Gear2 => {
            (4.0 * state.previous_current - state.previous_previous_current) / 3.0
        }
        TransientMethod::Euler => state.previous_current,
    };
    if let Some(i) = n1 {
        rhs[i] -= history_current;
    }
    if let Some(j) = n2 {
        rhs[j] += history_current;
    }
    Ok(())
}

fn stamp_zero_voltage_branch(
    name: &str,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<f64>],
    positive: Option<usize>,
    negative: Option<usize>,
) -> Result<(), SpiceError> {
    let Some(source_index) = voltage_sources.get(name) else {
        return Err(SpiceError::InvalidElement {
            name: name.to_string(),
            reason: "branch element was not indexed".to_string(),
        });
    };
    let branch = node_count + source_index;
    stamp_branch_matrix(matrix, branch, positive, negative);
    Ok(())
}

fn stamp_conductance(
    matrix: &mut [Vec<f64>],
    n1: Option<usize>,
    n2: Option<usize>,
    conductance: f64,
) {
    if let Some(i) = n1 {
        matrix[i][i] += conductance;
    }
    if let Some(j) = n2 {
        matrix[j][j] += conductance;
    }
    if let (Some(i), Some(j)) = (n1, n2) {
        matrix[i][j] -= conductance;
        matrix[j][i] -= conductance;
    }
}

fn validate_custom_model(model: &CustomModel) -> Result<(), SpiceError> {
    for (name, value) in &model.parameters {
        if !value.is_finite() {
            return Err(SpiceError::InvalidElement {
                name: model.name.clone(),
                reason: format!("custom-model parameter {name} must be finite"),
            });
        }
    }
    match model.kind {
        CustomModelKind::LinearConductance {
            conductance_siemens,
            current_offset_amps,
        } => {
            if !conductance_siemens.is_finite() {
                return Err(SpiceError::InvalidElement {
                    name: model.name.clone(),
                    reason: "custom-model conductance must be finite".to_string(),
                });
            }
            if !current_offset_amps.is_finite() {
                return Err(SpiceError::InvalidElement {
                    name: model.name.clone(),
                    reason: "custom-model current offset must be finite".to_string(),
                });
            }
        }
    }
    Ok(())
}

fn custom_model_voltage(
    model: &CustomModel,
    node_indices: &HashMap<String, usize>,
    operating_point: &[f64],
) -> f64 {
    let positive = node_index(node_indices, &model.positive);
    let negative = node_index(node_indices, &model.negative);
    vector_voltage(operating_point, positive) - vector_voltage(operating_point, negative)
}

fn custom_model_conductance(
    model: &CustomModel,
    node_indices: &HashMap<String, usize>,
    operating_point: &[f64],
) -> Result<f64, SpiceError> {
    Ok(model
        .evaluate(custom_model_voltage(model, node_indices, operating_point))?
        .conductance_siemens)
}

fn stamp_custom_model(
    model: &CustomModel,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    let positive = node_index(node_indices, &model.positive);
    let negative = node_index(node_indices, &model.negative);
    let voltage = custom_model_voltage(model, node_indices, operating_point);
    let evaluation = model.evaluate(voltage)?;
    let equivalent_current = evaluation.current_amps - evaluation.conductance_siemens * voltage;

    stamp_conductance(matrix, positive, negative, evaluation.conductance_siemens);
    if let Some(index) = positive {
        rhs[index] -= equivalent_current;
    }
    if let Some(index) = negative {
        rhs[index] += equivalent_current;
    }
    Ok(())
}

fn stamp_diode(
    diode: &Diode,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_diode(diode)?;
    let intrinsic_anode = diode_intrinsic_anode_node(diode);
    let anode = node_index(node_indices, &intrinsic_anode);
    let cathode = node_index(node_indices, &diode.cathode);
    let voltage = anode.map_or(0.0, |index| operating_point[index])
        - cathode.map_or(0.0, |index| operating_point[index]);
    let (current, conductance) = diode_current_conductance(diode, voltage);
    let equivalent_current = current - conductance * voltage;

    stamp_conductance(matrix, anode, cathode, conductance);
    if let Some(index) = anode {
        rhs[index] -= equivalent_current;
    }
    if let Some(index) = cathode {
        rhs[index] += equivalent_current;
    }
    if diode.series_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &diode.anode),
            anode,
            1.0 / diode.series_resistance,
        );
    }
    stamp_diode_charge(diode, capacitor_states, node_indices, matrix, rhs)?;
    Ok(())
}

fn stamp_diode_charge(
    diode: &Diode,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    let state_name = diode_charge_state_name(diode);
    let Some(state) = capacitor_states
        .iter()
        .find(|state| state.name == state_name)
    else {
        return Ok(());
    };
    let capacitance = diode_dynamic_capacitance(diode, state.previous_voltage);
    if capacitance <= 0.0 {
        return Ok(());
    }

    let conductance = match state.method {
        TransientMethod::Trap => 2.0 * capacitance / state.time_step,
        TransientMethod::Gear2 => 3.0 * capacitance / (2.0 * state.time_step),
        TransientMethod::Euler => capacitance / state.time_step,
    };
    let history_current = match state.method {
        TransientMethod::Trap => conductance * state.previous_voltage + state.previous_current,
        TransientMethod::Gear2 => {
            capacitance * (4.0 * state.previous_voltage - state.previous_previous_voltage)
                / (2.0 * state.time_step)
        }
        TransientMethod::Euler => conductance * state.previous_voltage,
    };
    let intrinsic_anode = diode_intrinsic_anode_node(diode);
    let anode = node_index(node_indices, &intrinsic_anode);
    let cathode = node_index(node_indices, &diode.cathode);
    stamp_conductance(matrix, anode, cathode, conductance);
    if let Some(index) = anode {
        rhs[index] += history_current;
    }
    if let Some(index) = cathode {
        rhs[index] -= history_current;
    }
    Ok(())
}

fn bjt_early_factor(bjt: &Bjt, junction_voltage: f64, output_voltage: f64) -> f64 {
    let forward_term = if bjt.forward_early_voltage == 0.0 {
        0.0
    } else {
        output_voltage / bjt.forward_early_voltage
    };
    let reverse_term = if bjt.reverse_early_voltage == 0.0 {
        0.0
    } else {
        junction_voltage / bjt.reverse_early_voltage
    };
    1.0 + forward_term - reverse_term
}

fn bjt_forward_transconductance(
    bjt: &Bjt,
    base_collector_current: f64,
    base_gm: f64,
    early_factor: f64,
) -> f64 {
    let reverse_early_conductance = if bjt.reverse_early_voltage == 0.0 {
        0.0
    } else {
        base_collector_current / bjt.reverse_early_voltage
    };
    base_gm * early_factor - reverse_early_conductance
}

fn bjt_forward_transport(
    bjt: &Bjt,
    base_collector_current: f64,
    base_gm: f64,
    early_factor: f64,
) -> (f64, f64, f64) {
    let low_current_gm =
        bjt_forward_transconductance(bjt, base_collector_current, base_gm, early_factor);
    if bjt.forward_beta_rolloff_current == 0.0 || base_collector_current <= 0.0 {
        return (base_collector_current * early_factor, low_current_gm, 1.0);
    }
    let root = (1.0 + 4.0 * base_collector_current / bjt.forward_beta_rolloff_current).sqrt();
    let charge_factor = 0.5 * (1.0 + root);
    let charge_derivative = base_gm / (bjt.forward_beta_rolloff_current * root);
    let collector_current = base_collector_current * early_factor / charge_factor;
    let gm = low_current_gm / charge_factor
        - base_collector_current * early_factor * charge_derivative / charge_factor.powi(2);
    (collector_current, gm, charge_factor)
}

fn bjt_base_emitter_leakage(bjt: &Bjt, junction_voltage: f64) -> (f64, f64) {
    if bjt.base_emitter_leakage_saturation_current == 0.0 {
        return (0.0, 0.0);
    }
    let thermal_voltage = bjt.thermal_voltage * bjt.base_emitter_leakage_emission_coefficient;
    let exponent = (junction_voltage / thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    (
        bjt.base_emitter_leakage_saturation_current * (exp_value - 1.0),
        bjt.base_emitter_leakage_saturation_current / thermal_voltage * exp_value,
    )
}

fn bjt_base_collector_leakage(bjt: &Bjt, junction_voltage: f64) -> (f64, f64) {
    if bjt.base_collector_leakage_saturation_current == 0.0 {
        return (0.0, 0.0);
    }
    let thermal_voltage = bjt.thermal_voltage * bjt.base_collector_leakage_emission_coefficient;
    let exponent = (junction_voltage / thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    (
        bjt.base_collector_leakage_saturation_current * (exp_value - 1.0),
        bjt.base_collector_leakage_saturation_current / thermal_voltage * exp_value,
    )
}

fn bjt_reverse_base_current(bjt: &Bjt, junction_voltage: f64) -> (f64, f64) {
    if bjt.reverse_beta.is_infinite() {
        return (0.0, 0.0);
    }
    let thermal_voltage = bjt.thermal_voltage * bjt.reverse_emission_coefficient;
    let exponent = (junction_voltage / thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    let diffusion_current = bjt.saturation_current * (exp_value - 1.0);
    let diffusion_conductance = bjt.saturation_current / thermal_voltage * exp_value;
    if bjt.reverse_beta_rolloff_current == 0.0 || diffusion_current <= 0.0 {
        return (
            diffusion_current / bjt.reverse_beta,
            diffusion_conductance / bjt.reverse_beta,
        );
    }
    let root = (1.0 + 4.0 * diffusion_current / bjt.reverse_beta_rolloff_current).sqrt();
    let charge_factor = 0.5 * (1.0 + root);
    let charge_derivative = diffusion_conductance / (bjt.reverse_beta_rolloff_current * root);
    (
        diffusion_current * charge_factor / bjt.reverse_beta,
        (diffusion_conductance * charge_factor + diffusion_current * charge_derivative)
            / bjt.reverse_beta,
    )
}

fn bjt_effective_base_resistance(
    bjt: &Bjt,
    base_voltage: f64,
    emitter_voltage: f64,
    collector_voltage: f64,
) -> f64 {
    let minimum = bjt.minimum_base_resistance.unwrap_or(bjt.base_resistance);
    if minimum == bjt.base_resistance {
        return bjt.base_resistance;
    }
    let (junction_voltage, reverse_voltage, output_voltage) = match bjt.polarity {
        BjtPolarity::Npn => (
            base_voltage - emitter_voltage,
            base_voltage - collector_voltage,
            collector_voltage - emitter_voltage,
        ),
        BjtPolarity::Pnp => (
            emitter_voltage - base_voltage,
            collector_voltage - base_voltage,
            emitter_voltage - collector_voltage,
        ),
    };
    let forward_thermal_voltage = bjt.thermal_voltage * bjt.forward_emission_coefficient;
    let exp_value = (junction_voltage / forward_thermal_voltage)
        .clamp(-40.0, 40.0)
        .exp();
    let diffusion_current = bjt.saturation_current * (exp_value - 1.0);
    let diffusion_conductance = bjt.saturation_current / forward_thermal_voltage * exp_value;
    let early_factor = bjt_early_factor(bjt, junction_voltage, output_voltage);
    let (_, _, charge_factor) =
        bjt_forward_transport(bjt, diffusion_current, diffusion_conductance, early_factor);
    let (leakage_current, _) = bjt_base_emitter_leakage(bjt, junction_voltage);
    let (collector_leakage_current, _) = bjt_base_collector_leakage(bjt, reverse_voltage);
    let (reverse_base_current, _) = bjt_reverse_base_current(bjt, reverse_voltage);
    let base_current = diffusion_current / bjt.forward_beta
        + leakage_current
        + collector_leakage_current
        + reverse_base_current;
    let variable_resistance = bjt.base_resistance - minimum;
    if bjt.base_resistance_half_current == 0.0 {
        return minimum + variable_resistance / charge_factor;
    }
    let ratio = (base_current / bjt.base_resistance_half_current).max(1.0e-9);
    let angle = (-1.0 + (1.0 + 14.59025 * ratio).sqrt()) / (2.4317 * ratio.sqrt());
    let tangent = angle.tan();
    let transition = 3.0 * (tangent - angle) / (angle * tangent * tangent);
    minimum + variable_resistance * transition
}

fn stamp_bjt(
    bjt: &Bjt,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_bjt(bjt)?;
    let charge_bjt = bjt.clone();
    let intrinsic_bjt = if bjt.emitter_resistance > 0.0
        || bjt.collector_resistance > 0.0
        || bjt.base_resistance > 0.0
    {
        let mut intrinsic = bjt.clone();
        if bjt.emitter_resistance > 0.0 {
            let intrinsic_emitter = bjt_intrinsic_emitter_node(bjt);
            stamp_conductance(
                matrix,
                node_index(node_indices, &bjt.emitter),
                node_index(node_indices, &intrinsic_emitter),
                1.0 / bjt.emitter_resistance,
            );
            intrinsic.emitter = intrinsic_emitter;
            intrinsic.emitter_resistance = 0.0;
        }
        if bjt.collector_resistance > 0.0 {
            let intrinsic_collector = bjt_intrinsic_collector_node(bjt);
            stamp_conductance(
                matrix,
                node_index(node_indices, &bjt.collector),
                node_index(node_indices, &intrinsic_collector),
                1.0 / bjt.collector_resistance,
            );
            intrinsic.collector = intrinsic_collector;
            intrinsic.collector_resistance = 0.0;
        }
        if bjt.base_resistance > 0.0 {
            let intrinsic_base = bjt_intrinsic_base_node(bjt);
            let intrinsic_base_index = node_index(node_indices, &intrinsic_base);
            let base_voltage = vector_voltage(operating_point, intrinsic_base_index);
            let emitter_voltage = vector_voltage(
                operating_point,
                node_index(node_indices, &intrinsic.emitter),
            );
            let collector_voltage = vector_voltage(
                operating_point,
                node_index(node_indices, &intrinsic.collector),
            );
            let base_resistance = bjt_effective_base_resistance(
                &intrinsic,
                base_voltage,
                emitter_voltage,
                collector_voltage,
            );
            stamp_conductance(
                matrix,
                node_index(node_indices, &bjt.base),
                intrinsic_base_index,
                1.0 / base_resistance,
            );
            intrinsic.base = intrinsic_base;
            intrinsic.base_resistance = 0.0;
            intrinsic.minimum_base_resistance = None;
            intrinsic.base_resistance_half_current = 0.0;
        }
        Some(intrinsic)
    } else {
        None
    };
    let bjt = intrinsic_bjt.as_ref().unwrap_or(bjt);
    let collector = node_index(node_indices, &bjt.collector);
    let base = node_index(node_indices, &bjt.base);
    let emitter = node_index(node_indices, &bjt.emitter);
    let base_voltage = base.map_or(0.0, |index| operating_point[index]);
    let emitter_voltage = emitter.map_or(0.0, |index| operating_point[index]);
    let collector_voltage = collector.map_or(0.0, |index| operating_point[index]);
    let junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - base_voltage,
    };
    let reverse_junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - collector_voltage,
        BjtPolarity::Pnp => collector_voltage - base_voltage,
    };
    let forward_thermal_voltage = bjt.thermal_voltage * bjt.forward_emission_coefficient;
    let exponent = (junction_voltage / forward_thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    let base_collector_current = bjt.saturation_current * (exp_value - 1.0);
    let base_gm = bjt.saturation_current / forward_thermal_voltage * exp_value;
    let output_voltage = match bjt.polarity {
        BjtPolarity::Npn => collector_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - collector_voltage,
    };
    let early_factor = bjt_early_factor(bjt, junction_voltage, output_voltage);
    let (collector_current, gm, charge_factor) =
        bjt_forward_transport(bjt, base_collector_current, base_gm, early_factor);
    let output_conductance = if bjt.forward_early_voltage == 0.0 {
        0.0
    } else {
        base_collector_current / bjt.forward_early_voltage / charge_factor
    };
    let (leakage_current, leakage_conductance) = bjt_base_emitter_leakage(bjt, junction_voltage);
    let gpi = base_gm / bjt.forward_beta + leakage_conductance;
    let base_current = base_collector_current / bjt.forward_beta + leakage_current;
    let equivalent_collector_current =
        collector_current - gm * junction_voltage - output_conductance * output_voltage;
    let equivalent_base_current = base_current - gpi * junction_voltage;
    let (collector_leakage_current, collector_leakage_conductance) =
        bjt_base_collector_leakage(bjt, reverse_junction_voltage);
    let (reverse_base_current, reverse_base_conductance) =
        bjt_reverse_base_current(bjt, reverse_junction_voltage);
    let base_collector_current = collector_leakage_current + reverse_base_current;
    let base_collector_conductance = collector_leakage_conductance + reverse_base_conductance;
    let equivalent_collector_leakage_current =
        base_collector_current - base_collector_conductance * reverse_junction_voltage;

    stamp_conductance(matrix, collector, emitter, output_conductance);
    stamp_conductance(matrix, base, collector, base_collector_conductance);

    match bjt.polarity {
        BjtPolarity::Npn => {
            stamp_conductance(matrix, base, emitter, gpi);
            stamp_transconductance(matrix, collector, emitter, base, emitter, gm);
            stamp_equivalent_current_source(rhs, base, emitter, equivalent_base_current);
            stamp_equivalent_current_source(rhs, collector, emitter, equivalent_collector_current);
            stamp_equivalent_current_source(
                rhs,
                base,
                collector,
                equivalent_collector_leakage_current,
            );
        }
        BjtPolarity::Pnp => {
            stamp_conductance(matrix, emitter, base, gpi);
            stamp_transconductance(matrix, emitter, collector, emitter, base, gm);
            stamp_equivalent_current_source(rhs, emitter, base, equivalent_base_current);
            stamp_equivalent_current_source(rhs, emitter, collector, equivalent_collector_current);
            stamp_equivalent_current_source(
                rhs,
                collector,
                base,
                equivalent_collector_leakage_current,
            );
        }
    }
    stamp_bjt_charge(&charge_bjt, capacitor_states, node_indices, matrix, rhs)?;
    Ok(())
}

fn stamp_bjt_charge(
    bjt: &Bjt,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    let reverse_junction_voltage = capacitor_states
        .iter()
        .find(|state| state.name == bjt_base_collector_charge_state_name(bjt))
        .map(|state| state.previous_voltage)
        .unwrap_or(0.0);
    for spec in bjt_charge_state_specs(bjt) {
        let Some(state) = capacitor_states
            .iter()
            .find(|state| state.name == spec.name)
        else {
            continue;
        };
        let capacitance = bjt_charge_dynamic_capacitance(
            bjt,
            spec.kind,
            state.previous_voltage,
            reverse_junction_voltage,
        );
        if capacitance <= 0.0 {
            continue;
        }
        let conductance = match state.method {
            TransientMethod::Trap => 2.0 * capacitance / state.time_step,
            TransientMethod::Gear2 => 3.0 * capacitance / (2.0 * state.time_step),
            TransientMethod::Euler => capacitance / state.time_step,
        };
        let history_current = match state.method {
            TransientMethod::Trap => conductance * state.previous_voltage + state.previous_current,
            TransientMethod::Gear2 => {
                capacitance * (4.0 * state.previous_voltage - state.previous_previous_voltage)
                    / (2.0 * state.time_step)
            }
            TransientMethod::Euler => conductance * state.previous_voltage,
        };
        let positive = node_index(node_indices, &spec.positive);
        let negative = node_index(node_indices, &spec.negative);
        stamp_conductance(matrix, positive, negative, conductance);
        if let Some(index) = positive {
            rhs[index] += history_current;
        }
        if let Some(index) = negative {
            rhs[index] -= history_current;
        }
    }
    Ok(())
}

fn stamp_equivalent_current_source(
    rhs: &mut [f64],
    positive: Option<usize>,
    negative: Option<usize>,
    current: f64,
) {
    if let Some(index) = positive {
        rhs[index] -= current;
    }
    if let Some(index) = negative {
        rhs[index] += current;
    }
}

fn stamp_bjt_small_signal(
    bjt: &Bjt,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_bjt(bjt)?;
    let intrinsic_bjt = if bjt.emitter_resistance > 0.0
        || bjt.collector_resistance > 0.0
        || bjt.base_resistance > 0.0
    {
        let mut intrinsic = bjt.clone();
        if bjt.emitter_resistance > 0.0 {
            let intrinsic_emitter = bjt_intrinsic_emitter_node(bjt);
            stamp_conductance(
                matrix,
                node_index(node_indices, &bjt.emitter),
                node_index(node_indices, &intrinsic_emitter),
                1.0 / bjt.emitter_resistance,
            );
            intrinsic.emitter = intrinsic_emitter;
            intrinsic.emitter_resistance = 0.0;
        }
        if bjt.collector_resistance > 0.0 {
            let intrinsic_collector = bjt_intrinsic_collector_node(bjt);
            stamp_conductance(
                matrix,
                node_index(node_indices, &bjt.collector),
                node_index(node_indices, &intrinsic_collector),
                1.0 / bjt.collector_resistance,
            );
            intrinsic.collector = intrinsic_collector;
            intrinsic.collector_resistance = 0.0;
        }
        if bjt.base_resistance > 0.0 {
            let intrinsic_base = bjt_intrinsic_base_node(bjt);
            let intrinsic_base_index = node_index(node_indices, &intrinsic_base);
            let base_voltage = vector_voltage(operating_point, intrinsic_base_index);
            let emitter_voltage = vector_voltage(
                operating_point,
                node_index(node_indices, &intrinsic.emitter),
            );
            let collector_voltage = vector_voltage(
                operating_point,
                node_index(node_indices, &intrinsic.collector),
            );
            let base_resistance = bjt_effective_base_resistance(
                &intrinsic,
                base_voltage,
                emitter_voltage,
                collector_voltage,
            );
            stamp_conductance(
                matrix,
                node_index(node_indices, &bjt.base),
                intrinsic_base_index,
                1.0 / base_resistance,
            );
            intrinsic.base = intrinsic_base;
            intrinsic.base_resistance = 0.0;
            intrinsic.minimum_base_resistance = None;
            intrinsic.base_resistance_half_current = 0.0;
        }
        Some(intrinsic)
    } else {
        None
    };
    let bjt = intrinsic_bjt.as_ref().unwrap_or(bjt);
    let collector = node_index(node_indices, &bjt.collector);
    let base = node_index(node_indices, &bjt.base);
    let emitter = node_index(node_indices, &bjt.emitter);
    let base_voltage = vector_voltage(operating_point, base);
    let emitter_voltage = vector_voltage(operating_point, emitter);
    let collector_voltage = vector_voltage(operating_point, collector);
    let junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - base_voltage,
    };
    let reverse_junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - collector_voltage,
        BjtPolarity::Pnp => collector_voltage - base_voltage,
    };
    let forward_thermal_voltage = bjt.thermal_voltage * bjt.forward_emission_coefficient;
    let exponent = (junction_voltage / forward_thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    let base_collector_current = bjt.saturation_current * (exp_value - 1.0);
    let base_gm = bjt.saturation_current / forward_thermal_voltage * exp_value;
    let output_voltage = match bjt.polarity {
        BjtPolarity::Npn => collector_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - collector_voltage,
    };
    let early_factor = bjt_early_factor(bjt, junction_voltage, output_voltage);
    let (_, gm, charge_factor) =
        bjt_forward_transport(bjt, base_collector_current, base_gm, early_factor);
    let output_conductance = if bjt.forward_early_voltage == 0.0 {
        0.0
    } else {
        base_collector_current / bjt.forward_early_voltage / charge_factor
    };
    let (_, leakage_conductance) = bjt_base_emitter_leakage(bjt, junction_voltage);
    let gpi = base_gm / bjt.forward_beta + leakage_conductance;
    let (_, collector_leakage_conductance) =
        bjt_base_collector_leakage(bjt, reverse_junction_voltage);
    let (_, reverse_base_conductance) = bjt_reverse_base_current(bjt, reverse_junction_voltage);
    stamp_conductance(matrix, collector, emitter, output_conductance);
    stamp_conductance(
        matrix,
        base,
        collector,
        collector_leakage_conductance + reverse_base_conductance,
    );
    match bjt.polarity {
        BjtPolarity::Npn => {
            stamp_conductance(matrix, base, emitter, gpi);
            stamp_transconductance(matrix, collector, emitter, base, emitter, gm);
        }
        BjtPolarity::Pnp => {
            stamp_conductance(matrix, emitter, base, gpi);
            stamp_transconductance(matrix, emitter, collector, emitter, base, gm);
        }
    }
    Ok(())
}

fn stamp_ac_bjt_small_signal(
    bjt: &Bjt,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
    operating_point: &[f64],
    omega: f64,
) -> Result<(), SpiceError> {
    validate_bjt(bjt)?;
    let external_base = node_index(node_indices, &bjt.base);
    let intrinsic_bjt = if bjt.emitter_resistance > 0.0
        || bjt.collector_resistance > 0.0
        || bjt.base_resistance > 0.0
    {
        let mut intrinsic = bjt.clone();
        if bjt.emitter_resistance > 0.0 {
            let intrinsic_emitter = bjt_intrinsic_emitter_node(bjt);
            stamp_complex_conductance(
                matrix,
                node_index(node_indices, &bjt.emitter),
                node_index(node_indices, &intrinsic_emitter),
                Complex::new(1.0 / bjt.emitter_resistance, 0.0),
            );
            intrinsic.emitter = intrinsic_emitter;
            intrinsic.emitter_resistance = 0.0;
        }
        if bjt.collector_resistance > 0.0 {
            let intrinsic_collector = bjt_intrinsic_collector_node(bjt);
            stamp_complex_conductance(
                matrix,
                node_index(node_indices, &bjt.collector),
                node_index(node_indices, &intrinsic_collector),
                Complex::new(1.0 / bjt.collector_resistance, 0.0),
            );
            intrinsic.collector = intrinsic_collector;
            intrinsic.collector_resistance = 0.0;
        }
        if bjt.base_resistance > 0.0 {
            let intrinsic_base = bjt_intrinsic_base_node(bjt);
            let intrinsic_base_index = node_index(node_indices, &intrinsic_base);
            let base_voltage = vector_voltage(operating_point, intrinsic_base_index);
            let emitter_voltage = vector_voltage(
                operating_point,
                node_index(node_indices, &intrinsic.emitter),
            );
            let collector_voltage = vector_voltage(
                operating_point,
                node_index(node_indices, &intrinsic.collector),
            );
            let base_resistance = bjt_effective_base_resistance(
                &intrinsic,
                base_voltage,
                emitter_voltage,
                collector_voltage,
            );
            stamp_complex_conductance(
                matrix,
                node_index(node_indices, &bjt.base),
                intrinsic_base_index,
                Complex::new(1.0 / base_resistance, 0.0),
            );
            intrinsic.base = intrinsic_base;
            intrinsic.base_resistance = 0.0;
            intrinsic.minimum_base_resistance = None;
            intrinsic.base_resistance_half_current = 0.0;
        }
        Some(intrinsic)
    } else {
        None
    };
    let bjt = intrinsic_bjt.as_ref().unwrap_or(bjt);
    let collector = node_index(node_indices, &bjt.collector);
    let base = node_index(node_indices, &bjt.base);
    let emitter = node_index(node_indices, &bjt.emitter);
    let collector_voltage = vector_voltage(operating_point, collector);
    let base_voltage = vector_voltage(operating_point, base);
    let emitter_voltage = vector_voltage(operating_point, emitter);
    let junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - base_voltage,
    };
    let reverse_junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - collector_voltage,
        BjtPolarity::Pnp => collector_voltage - base_voltage,
    };
    let forward_thermal_voltage = bjt.thermal_voltage * bjt.forward_emission_coefficient;
    let exponent = (junction_voltage / forward_thermal_voltage).clamp(-40.0, 40.0);
    let reverse_thermal_voltage = bjt.thermal_voltage * bjt.reverse_emission_coefficient;
    let reverse_exponent = (reverse_junction_voltage / reverse_thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    let base_collector_current = bjt.saturation_current * (exp_value - 1.0);
    let base_gm = bjt.saturation_current / forward_thermal_voltage * exp_value;
    let output_voltage = match bjt.polarity {
        BjtPolarity::Npn => collector_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - collector_voltage,
    };
    let early_factor = bjt_early_factor(bjt, junction_voltage, output_voltage);
    let (_, forward_gm, charge_factor) =
        bjt_forward_transport(bjt, base_collector_current, base_gm, early_factor);
    let output_conductance = if bjt.forward_early_voltage == 0.0 {
        0.0
    } else {
        base_collector_current / bjt.forward_early_voltage / charge_factor
    };
    let excess_phase =
        omega * bjt.forward_transit_time * bjt.forward_excess_phase_degrees * std::f64::consts::PI
            / 180.0;
    let gm = Complex::new(
        forward_gm * excess_phase.cos(),
        -forward_gm * excess_phase.sin(),
    );
    let reverse_gm = bjt.saturation_current / reverse_thermal_voltage * reverse_exponent.exp();
    let diffusion_capacitance = bjt.forward_transit_time
        * bjt_forward_transit_time_scale(bjt, junction_voltage, reverse_junction_voltage)
        * forward_gm;
    let reverse_diffusion_capacitance = bjt.reverse_transit_time * reverse_gm;
    let (_, leakage_conductance) = bjt_base_emitter_leakage(bjt, junction_voltage);
    let (_, collector_leakage_conductance) =
        bjt_base_collector_leakage(bjt, reverse_junction_voltage);
    let (_, reverse_base_conductance) = bjt_reverse_base_current(bjt, reverse_junction_voltage);
    let gpi = Complex::new(
        base_gm / bjt.forward_beta + leakage_conductance,
        omega
            * (bjt_base_emitter_depletion_capacitance(bjt, junction_voltage)
                + diffusion_capacitance),
    );
    let base_collector_depletion =
        bjt_base_collector_depletion_capacitance(bjt, reverse_junction_voltage);
    let ybc = Complex::new(
        collector_leakage_conductance + reverse_base_conductance,
        omega
            * (bjt.base_collector_capacitance_fraction * base_collector_depletion
                + reverse_diffusion_capacitance),
    );
    let ybx = Complex::new(
        0.0,
        omega * (1.0 - bjt.base_collector_capacitance_fraction) * base_collector_depletion,
    );
    stamp_complex_conductance(
        matrix,
        collector,
        emitter,
        Complex::new(output_conductance, 0.0),
    );
    if ybx != Complex::new(0.0, 0.0) {
        stamp_complex_conductance(matrix, external_base, collector, ybx);
    }
    match bjt.polarity {
        BjtPolarity::Npn => {
            stamp_complex_conductance(matrix, base, emitter, gpi);
            stamp_complex_conductance(matrix, base, collector, ybc);
            stamp_complex_transconductance(matrix, collector, emitter, base, emitter, gm);
        }
        BjtPolarity::Pnp => {
            stamp_complex_conductance(matrix, emitter, base, gpi);
            stamp_complex_conductance(matrix, base, collector, ybc);
            stamp_complex_transconductance(matrix, emitter, collector, emitter, base, gm);
        }
    }
    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct MosfetDcResult {
    drain_current: f64,
    gm: f64,
    gds: f64,
    gmb: f64,
    cgs: f64,
    cgd: f64,
    cgb: f64,
    cbs: f64,
    cbd: f64,
}

fn mosfet_bulk_junction_capacitance(
    zero_bias_capacitance: f64,
    junction_voltage: f64,
    junction_potential: f64,
    grading_coefficient: f64,
    forward_bias_coefficient: f64,
) -> f64 {
    if zero_bias_capacitance <= 0.0 {
        return zero_bias_capacitance;
    }
    if junction_potential <= 0.0 || grading_coefficient == 0.0 {
        return zero_bias_capacitance;
    }
    let normalized_voltage = junction_voltage / junction_potential;
    if normalized_voltage < forward_bias_coefficient {
        return zero_bias_capacitance / (1.0 - normalized_voltage).powf(grading_coefficient);
    }
    let denominator = (1.0 - forward_bias_coefficient).powf(1.0 + grading_coefficient);
    let continuation = 1.0 - forward_bias_coefficient * (1.0 + grading_coefficient)
        + grading_coefficient * normalized_voltage;
    zero_bias_capacitance * continuation / denominator
}

struct JfetDcResult {
    drain_current: f64,
    gm: f64,
    gds: f64,
}

fn stamp_jfet(
    jfet: &Jfet,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_jfet(jfet)?;
    let intrinsic_drain = jfet_intrinsic_drain_node(jfet);
    let intrinsic_source = jfet_intrinsic_source_node(jfet);
    let drain = node_index(node_indices, &intrinsic_drain);
    let gate = node_index(node_indices, &jfet.gate);
    let source = node_index(node_indices, &intrinsic_source);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let vgs = gate_voltage - source_voltage;
    let vds = drain_voltage - source_voltage;
    let result = evaluate_jfet(jfet, vgs, vds);
    let equivalent_current = result.drain_current - result.gm * vgs - result.gds * vds;

    stamp_conductance(matrix, drain, source, result.gds);
    stamp_transconductance(matrix, drain, source, gate, source, result.gm);
    stamp_equivalent_current_source(rhs, drain, source, equivalent_current);
    stamp_jfet_gate_junction(
        jfet,
        gate,
        source,
        gate_voltage - source_voltage,
        matrix,
        rhs,
    );
    stamp_jfet_gate_junction(jfet, gate, drain, gate_voltage - drain_voltage, matrix, rhs);
    stamp_jfet_charge(jfet, capacitor_states, node_indices, matrix, rhs)?;
    if jfet.drain_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &jfet.drain),
            drain,
            1.0 / jfet.drain_resistance,
        );
    }
    if jfet.source_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &jfet.source),
            source,
            1.0 / jfet.source_resistance,
        );
    }
    Ok(())
}

const JFET_THERMAL_VOLTAGE: f64 = 0.02585;

fn jfet_gate_junction_current_conductance(jfet: &Jfet, gate_voltage: f64) -> (f64, f64) {
    let junction_voltage = match jfet.polarity {
        JfetPolarity::Njf => gate_voltage,
        JfetPolarity::Pjf => -gate_voltage,
    };
    let exp_value = (junction_voltage / JFET_THERMAL_VOLTAGE)
        .clamp(-40.0, 40.0)
        .exp();
    (
        jfet.gate_saturation_current * (exp_value - 1.0),
        jfet.gate_saturation_current / JFET_THERMAL_VOLTAGE * exp_value,
    )
}

fn stamp_jfet_gate_junction(
    jfet: &Jfet,
    gate: Option<usize>,
    terminal: Option<usize>,
    gate_voltage: f64,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) {
    let (current, conductance) = jfet_gate_junction_current_conductance(jfet, gate_voltage);
    let junction_voltage = match jfet.polarity {
        JfetPolarity::Njf => gate_voltage,
        JfetPolarity::Pjf => -gate_voltage,
    };
    let equivalent_current = current - conductance * junction_voltage;
    match jfet.polarity {
        JfetPolarity::Njf => {
            stamp_conductance(matrix, gate, terminal, conductance);
            stamp_equivalent_current_source(rhs, gate, terminal, equivalent_current);
        }
        JfetPolarity::Pjf => {
            stamp_conductance(matrix, terminal, gate, conductance);
            stamp_equivalent_current_source(rhs, terminal, gate, equivalent_current);
        }
    }
}

fn stamp_jfet_charge(
    jfet: &Jfet,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    for spec in jfet_charge_state_specs(jfet) {
        let Some(state) = capacitor_states
            .iter()
            .find(|state| state.name == spec.name)
        else {
            continue;
        };
        let capacitance =
            jfet_charge_dynamic_capacitance(jfet, spec.capacitance, state.previous_voltage);
        if capacitance <= 0.0 {
            continue;
        }
        let conductance = match state.method {
            TransientMethod::Trap => 2.0 * capacitance / state.time_step,
            TransientMethod::Gear2 => 3.0 * capacitance / (2.0 * state.time_step),
            TransientMethod::Euler => capacitance / state.time_step,
        };
        let history_current = match state.method {
            TransientMethod::Trap => conductance * state.previous_voltage + state.previous_current,
            TransientMethod::Gear2 => {
                capacitance * (4.0 * state.previous_voltage - state.previous_previous_voltage)
                    / (2.0 * state.time_step)
            }
            TransientMethod::Euler => conductance * state.previous_voltage,
        };
        let positive = node_index(node_indices, &spec.positive);
        let negative = node_index(node_indices, &spec.negative);
        stamp_conductance(matrix, positive, negative, conductance);
        if let Some(index) = positive {
            rhs[index] += history_current;
        }
        if let Some(index) = negative {
            rhs[index] -= history_current;
        }
    }
    Ok(())
}

fn evaluate_jfet(jfet: &Jfet, vgs: f64, vds: f64) -> JfetDcResult {
    match jfet.polarity {
        JfetPolarity::Pjf => {
            let result = evaluate_njf(
                -vgs,
                -vds,
                -jfet.threshold_voltage,
                jfet.beta,
                jfet.channel_length_modulation,
                jfet.junction_potential,
                jfet.doping_tail_parameter,
            );
            JfetDcResult {
                drain_current: -result.drain_current,
                gm: result.gm,
                gds: result.gds,
            }
        }
        JfetPolarity::Njf => evaluate_njf(
            vgs,
            vds,
            jfet.threshold_voltage,
            jfet.beta,
            jfet.channel_length_modulation,
            jfet.junction_potential,
            jfet.doping_tail_parameter,
        ),
    }
}

fn evaluate_njf(
    vgs: f64,
    vds: f64,
    threshold_voltage: f64,
    beta: f64,
    channel_length_modulation: f64,
    junction_potential: f64,
    doping_tail_parameter: f64,
) -> JfetDcResult {
    let overdrive = vgs - threshold_voltage;
    if overdrive <= 0.0 || vds < 0.0 {
        return JfetDcResult {
            drain_current: 0.0,
            gm: 0.0,
            gds: 0.0,
        };
    }
    let tail_factor = if doping_tail_parameter == 1.0 {
        0.0
    } else {
        (1.0 - doping_tail_parameter) / (junction_potential - threshold_voltage)
    };
    let modulation = 1.0 + channel_length_modulation * vds;
    if vds < overdrive {
        let slope = 2.0 * doping_tail_parameter + 3.0 * tail_factor * (overdrive - vds);
        let channel = vds * (vds * (tail_factor * vds - doping_tail_parameter) + overdrive * slope);
        return JfetDcResult {
            drain_current: beta * channel * modulation,
            gm: beta * modulation * vds * (slope + 3.0 * tail_factor * overdrive),
            gds: beta * modulation * (overdrive - vds) * slope
                + beta * channel * channel_length_modulation,
        };
    }
    let channel = overdrive * overdrive * (doping_tail_parameter + overdrive * tail_factor);
    JfetDcResult {
        drain_current: beta * channel * modulation,
        gm: beta
            * modulation
            * overdrive
            * (2.0 * doping_tail_parameter + 3.0 * overdrive * tail_factor),
        gds: beta * channel * channel_length_modulation,
    }
}

fn stamp_mosfet(
    mosfet: &Mosfet,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_mosfet(mosfet)?;
    let intrinsic_drain = mosfet_intrinsic_drain_node(mosfet);
    let intrinsic_source = mosfet_intrinsic_source_node(mosfet);
    let drain = node_index(node_indices, &intrinsic_drain);
    let gate = node_index(node_indices, &mosfet.gate);
    let source = node_index(node_indices, &intrinsic_source);
    let body = node_index(node_indices, &mosfet.body);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let body_voltage = vector_voltage(operating_point, body);
    let vgs = gate_voltage - source_voltage;
    let vds = drain_voltage - source_voltage;
    let vbs = body_voltage - source_voltage;
    let result = evaluate_mosfet_level1(mosfet, vgs, vds, vbs);
    let equivalent_current =
        result.drain_current - result.gm * vgs - result.gds * vds - result.gmb * vbs;

    stamp_conductance(matrix, drain, source, result.gds);
    stamp_transconductance(matrix, drain, source, gate, source, result.gm);
    stamp_transconductance(matrix, drain, source, body, source, result.gmb);
    stamp_equivalent_current_source(rhs, drain, source, equivalent_current);
    stamp_mosfet_bulk_junction(
        mosfet,
        source,
        body,
        source_voltage,
        body_voltage,
        mosfet.params.source_area,
        matrix,
        rhs,
    );
    stamp_mosfet_bulk_junction(
        mosfet,
        drain,
        body,
        drain_voltage,
        body_voltage,
        mosfet.params.drain_area,
        matrix,
        rhs,
    );
    stamp_mosfet_charge(mosfet, capacitor_states, node_indices, matrix, rhs)?;
    let drain_resistance = mosfet_drain_resistance(mosfet);
    if drain_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &mosfet.drain),
            drain,
            1.0 / drain_resistance,
        );
    }
    let source_resistance = mosfet_source_resistance(mosfet);
    if source_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &mosfet.source),
            source,
            1.0 / source_resistance,
        );
    }
    Ok(())
}

fn mosfet_bulk_junction_current_conductance(
    mosfet: &Mosfet,
    terminal_voltage: f64,
    body_voltage: f64,
    terminal_area: f64,
) -> (f64, f64) {
    let saturation_current = if mosfet.params.saturation_current_density > 0.0
        && mosfet.params.drain_area > 0.0
        && mosfet.params.source_area > 0.0
    {
        mosfet.params.saturation_current_density * terminal_area
    } else {
        mosfet.params.saturation_current
    };
    let junction_voltage = match mosfet.mosfet_type {
        MosfetType::Nmos => body_voltage - terminal_voltage,
        MosfetType::Pmos => terminal_voltage - body_voltage,
    };
    let thermal_voltage = BOLTZMANN * mosfet.params.t_nom / ELECTRON_CHARGE;
    let normalized_voltage = junction_voltage / thermal_voltage;
    let limited_exp = normalized_voltage.clamp(-40.0, 40.0).exp();
    let (current_factor, conductance_factor) = if normalized_voltage > 40.0 {
        (limited_exp * (1.0 + normalized_voltage - 40.0), limited_exp)
    } else {
        (limited_exp, limited_exp)
    };
    (
        saturation_current * (current_factor - 1.0),
        saturation_current / thermal_voltage * conductance_factor,
    )
}

fn stamp_mosfet_bulk_junction(
    mosfet: &Mosfet,
    terminal: Option<usize>,
    body: Option<usize>,
    terminal_voltage: f64,
    body_voltage: f64,
    terminal_area: f64,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) {
    let (current, conductance) = mosfet_bulk_junction_current_conductance(
        mosfet,
        terminal_voltage,
        body_voltage,
        terminal_area,
    );
    let junction_voltage = match mosfet.mosfet_type {
        MosfetType::Nmos => body_voltage - terminal_voltage,
        MosfetType::Pmos => terminal_voltage - body_voltage,
    };
    let (positive, negative) = match mosfet.mosfet_type {
        MosfetType::Nmos => (body, terminal),
        MosfetType::Pmos => (terminal, body),
    };
    stamp_conductance(matrix, positive, negative, conductance);
    stamp_equivalent_current_source(
        rhs,
        positive,
        negative,
        current - conductance * junction_voltage,
    );
}

fn stamp_mosfet_charge(
    mosfet: &Mosfet,
    capacitor_states: &[CapacitorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    for spec in mosfet_charge_state_specs(mosfet) {
        let Some(state) = capacitor_states
            .iter()
            .find(|state| state.name == spec.name)
        else {
            continue;
        };
        let capacitance = mosfet_charge_dynamic_capacitance(mosfet, &spec, state.previous_voltage);
        if capacitance <= 0.0 {
            continue;
        }
        let conductance = match state.method {
            TransientMethod::Trap => 2.0 * capacitance / state.time_step,
            TransientMethod::Gear2 => 3.0 * capacitance / (2.0 * state.time_step),
            TransientMethod::Euler => capacitance / state.time_step,
        };
        let history_current = match state.method {
            TransientMethod::Trap => conductance * state.previous_voltage + state.previous_current,
            TransientMethod::Gear2 => {
                capacitance * (4.0 * state.previous_voltage - state.previous_previous_voltage)
                    / (2.0 * state.time_step)
            }
            TransientMethod::Euler => conductance * state.previous_voltage,
        };
        let positive = node_index(node_indices, &spec.positive);
        let negative = node_index(node_indices, &spec.negative);
        stamp_conductance(matrix, positive, negative, conductance);
        if let Some(index) = positive {
            rhs[index] += history_current;
        }
        if let Some(index) = negative {
            rhs[index] -= history_current;
        }
    }
    Ok(())
}

fn evaluate_mosfet_level1(mosfet: &Mosfet, vgs: f64, vds: f64, vbs: f64) -> MosfetDcResult {
    match mosfet.mosfet_type {
        MosfetType::Pmos => {
            let result = evaluate_nmos_level1(&mosfet.params, -vgs, -vds, -vbs);
            MosfetDcResult {
                drain_current: -result.drain_current,
                gm: result.gm,
                gds: result.gds,
                gmb: result.gmb,
                cgs: result.cgs,
                cgd: result.cgd,
                cgb: result.cgb,
                cbs: result.cbs,
                cbd: result.cbd,
            }
        }
        MosfetType::Nmos => evaluate_nmos_level1(&mosfet.params, vgs, vds, vbs),
    }
}

fn evaluate_nmos_level1(
    params: &MosfetLevel1Params,
    vgs: f64,
    vds: f64,
    vbs: f64,
) -> MosfetDcResult {
    let effective_length = params.l - 2.0 * params.lateral_diffusion_length;
    let beta = params.kp * (params.w / effective_length);
    let cgs_overlap = params.gate_source_overlap_capacitance * params.w;
    let cgd_overlap = params.gate_drain_overlap_capacitance * params.w;
    let cgb_overlap = params.gate_bulk_overlap_capacitance * effective_length;
    let channel_capacitance =
        params.w * effective_length * (OXIDE_PERMITTIVITY / params.oxide_thickness);
    let cbs_bulk = mosfet_bulk_junction_capacitance(
        params.source_bulk_capacitance + params.bottom_junction_capacitance * params.source_area,
        vbs,
        params.bulk_junction_potential,
        params.bulk_junction_grading_coefficient,
        params.forward_bias_depletion_coefficient,
    ) + mosfet_bulk_junction_capacitance(
        params.sidewall_junction_capacitance * params.source_perimeter,
        vbs,
        params.bulk_junction_potential,
        params.sidewall_junction_grading_coefficient,
        params.forward_bias_depletion_coefficient,
    );
    let cbd_bulk = mosfet_bulk_junction_capacitance(
        params.drain_bulk_capacitance + params.bottom_junction_capacitance * params.drain_area,
        vbs - vds,
        params.bulk_junction_potential,
        params.bulk_junction_grading_coefficient,
        params.forward_bias_depletion_coefficient,
    ) + mosfet_bulk_junction_capacitance(
        params.sidewall_junction_capacitance * params.drain_perimeter,
        vbs - vds,
        params.bulk_junction_potential,
        params.sidewall_junction_grading_coefficient,
        params.forward_bias_depletion_coefficient,
    );
    let threshold = if params.phi - vbs >= 0.0 {
        params.vt0 + params.gamma * ((params.phi - vbs).sqrt() - params.phi.sqrt())
    } else {
        params.vt0
    };
    let overdrive = vgs - threshold;
    if overdrive <= 0.0 {
        return MosfetDcResult {
            drain_current: 0.0,
            gm: 0.0,
            gds: 0.0,
            gmb: 0.0,
            cgs: cgs_overlap + channel_capacitance,
            cgd: cgd_overlap,
            cgb: cgb_overlap,
            cbs: cbs_bulk,
            cbd: cbd_bulk,
        };
    }

    let body_factor = if params.phi - vbs > 0.0 {
        params.gamma / (2.0 * (params.phi - vbs).sqrt())
    } else {
        0.0
    };
    if vds < overdrive {
        let channel = overdrive * vds - 0.5 * vds * vds;
        let modulation = 1.0 + params.lambda * vds;
        let gm = beta * vds * modulation;
        return MosfetDcResult {
            drain_current: beta * channel * modulation,
            gm,
            gds: beta * (overdrive - vds) * modulation + beta * channel * params.lambda,
            gmb: gm * body_factor,
            cgs: cgs_overlap + channel_capacitance / 2.0,
            cgd: cgd_overlap,
            cgb: cgb_overlap,
            cbs: cbs_bulk,
            cbd: cbd_bulk,
        };
    }

    let drain_current = 0.5 * beta * overdrive * overdrive * (1.0 + params.lambda * vds);
    let gm = beta * overdrive * (1.0 + params.lambda * vds);
    MosfetDcResult {
        drain_current,
        gm,
        gds: 0.5 * beta * overdrive * overdrive * params.lambda,
        gmb: gm * body_factor,
        cgs: cgs_overlap + (2.0 / 3.0) * channel_capacitance,
        cgd: cgd_overlap,
        cgb: cgb_overlap,
        cbs: cbs_bulk,
        cbd: cbd_bulk,
    }
}

fn vector_voltage(vector: &[f64], index: Option<usize>) -> f64 {
    index.map_or(0.0, |index| vector[index])
}

fn stamp_mosfet_small_signal(
    mosfet: &Mosfet,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_mosfet(mosfet)?;
    let intrinsic_drain = mosfet_intrinsic_drain_node(mosfet);
    let intrinsic_source = mosfet_intrinsic_source_node(mosfet);
    let drain = node_index(node_indices, &intrinsic_drain);
    let gate = node_index(node_indices, &mosfet.gate);
    let source = node_index(node_indices, &intrinsic_source);
    let body = node_index(node_indices, &mosfet.body);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let body_voltage = vector_voltage(operating_point, body);
    let vgs = gate_voltage - source_voltage;
    let vds = drain_voltage - source_voltage;
    let vbs = body_voltage - source_voltage;
    let result = evaluate_mosfet_level1(mosfet, vgs, vds, vbs);
    let (_, source_bulk_conductance) = mosfet_bulk_junction_current_conductance(
        mosfet,
        source_voltage,
        body_voltage,
        mosfet.params.source_area,
    );
    let (_, drain_bulk_conductance) = mosfet_bulk_junction_current_conductance(
        mosfet,
        drain_voltage,
        body_voltage,
        mosfet.params.drain_area,
    );
    stamp_conductance(matrix, drain, source, result.gds);
    stamp_conductance(matrix, body, source, source_bulk_conductance);
    stamp_conductance(matrix, body, drain, drain_bulk_conductance);
    stamp_transconductance(matrix, drain, source, gate, source, result.gm);
    stamp_transconductance(matrix, drain, source, body, source, result.gmb);
    let drain_resistance = mosfet_drain_resistance(mosfet);
    if drain_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &mosfet.drain),
            drain,
            1.0 / drain_resistance,
        );
    }
    let source_resistance = mosfet_source_resistance(mosfet);
    if source_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &mosfet.source),
            source,
            1.0 / source_resistance,
        );
    }
    Ok(())
}

fn stamp_jfet_small_signal(
    jfet: &Jfet,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_jfet(jfet)?;
    let intrinsic_drain = jfet_intrinsic_drain_node(jfet);
    let intrinsic_source = jfet_intrinsic_source_node(jfet);
    let drain = node_index(node_indices, &intrinsic_drain);
    let gate = node_index(node_indices, &jfet.gate);
    let source = node_index(node_indices, &intrinsic_source);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let result = evaluate_jfet(
        jfet,
        gate_voltage - source_voltage,
        drain_voltage - source_voltage,
    );
    let (_, gate_source_conductance) =
        jfet_gate_junction_current_conductance(jfet, gate_voltage - source_voltage);
    let (_, gate_drain_conductance) =
        jfet_gate_junction_current_conductance(jfet, gate_voltage - drain_voltage);
    stamp_conductance(matrix, drain, source, result.gds);
    stamp_conductance(matrix, gate, source, gate_source_conductance);
    stamp_conductance(matrix, gate, drain, gate_drain_conductance);
    stamp_transconductance(matrix, drain, source, gate, source, result.gm);
    if jfet.drain_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &jfet.drain),
            drain,
            1.0 / jfet.drain_resistance,
        );
    }
    if jfet.source_resistance > 0.0 {
        stamp_conductance(
            matrix,
            node_index(node_indices, &jfet.source),
            source,
            1.0 / jfet.source_resistance,
        );
    }
    Ok(())
}

fn stamp_ac_mosfet_small_signal(
    mosfet: &Mosfet,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
    operating_point: &[f64],
    omega: f64,
) -> Result<(), SpiceError> {
    validate_mosfet(mosfet)?;
    let intrinsic_drain = mosfet_intrinsic_drain_node(mosfet);
    let intrinsic_source = mosfet_intrinsic_source_node(mosfet);
    let drain = node_index(node_indices, &intrinsic_drain);
    let gate = node_index(node_indices, &mosfet.gate);
    let source = node_index(node_indices, &intrinsic_source);
    let body = node_index(node_indices, &mosfet.body);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let body_voltage = vector_voltage(operating_point, body);
    let vgs = gate_voltage - source_voltage;
    let vds = drain_voltage - source_voltage;
    let vbs = body_voltage - source_voltage;
    let result = evaluate_mosfet_level1(mosfet, vgs, vds, vbs);
    let (_, source_bulk_conductance) = mosfet_bulk_junction_current_conductance(
        mosfet,
        source_voltage,
        body_voltage,
        mosfet.params.source_area,
    );
    let (_, drain_bulk_conductance) = mosfet_bulk_junction_current_conductance(
        mosfet,
        drain_voltage,
        body_voltage,
        mosfet.params.drain_area,
    );
    stamp_complex_conductance(matrix, drain, source, Complex::new(result.gds, 0.0));
    stamp_complex_conductance(matrix, gate, source, Complex::new(0.0, omega * result.cgs));
    stamp_complex_conductance(matrix, gate, drain, Complex::new(0.0, omega * result.cgd));
    stamp_complex_conductance(matrix, gate, body, Complex::new(0.0, omega * result.cgb));
    stamp_complex_conductance(
        matrix,
        body,
        source,
        Complex::new(source_bulk_conductance, omega * result.cbs),
    );
    stamp_complex_conductance(
        matrix,
        body,
        drain,
        Complex::new(drain_bulk_conductance, omega * result.cbd),
    );
    stamp_complex_transconductance(
        matrix,
        drain,
        source,
        gate,
        source,
        Complex::new(result.gm, 0.0),
    );
    stamp_complex_transconductance(
        matrix,
        drain,
        source,
        body,
        source,
        Complex::new(result.gmb, 0.0),
    );
    let drain_resistance = mosfet_drain_resistance(mosfet);
    if drain_resistance > 0.0 {
        stamp_complex_conductance(
            matrix,
            node_index(node_indices, &mosfet.drain),
            drain,
            Complex::new(1.0 / drain_resistance, 0.0),
        );
    }
    let source_resistance = mosfet_source_resistance(mosfet);
    if source_resistance > 0.0 {
        stamp_complex_conductance(
            matrix,
            node_index(node_indices, &mosfet.source),
            source,
            Complex::new(1.0 / source_resistance, 0.0),
        );
    }
    Ok(())
}

fn stamp_ac_jfet_small_signal(
    jfet: &Jfet,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
    operating_point: &[f64],
    omega: f64,
) -> Result<(), SpiceError> {
    validate_jfet(jfet)?;
    let intrinsic_drain = jfet_intrinsic_drain_node(jfet);
    let intrinsic_source = jfet_intrinsic_source_node(jfet);
    let drain = node_index(node_indices, &intrinsic_drain);
    let gate = node_index(node_indices, &jfet.gate);
    let source = node_index(node_indices, &intrinsic_source);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let result = evaluate_jfet(
        jfet,
        gate_voltage - source_voltage,
        drain_voltage - source_voltage,
    );
    let gate_source_capacitance = jfet_charge_dynamic_capacitance(
        jfet,
        jfet.gate_source_capacitance,
        gate_voltage - source_voltage,
    );
    let gate_drain_capacitance = jfet_charge_dynamic_capacitance(
        jfet,
        jfet.gate_drain_capacitance,
        gate_voltage - drain_voltage,
    );
    stamp_complex_conductance(matrix, drain, source, Complex::new(result.gds, 0.0));
    let (_, gate_source_conductance) =
        jfet_gate_junction_current_conductance(jfet, gate_voltage - source_voltage);
    let (_, gate_drain_conductance) =
        jfet_gate_junction_current_conductance(jfet, gate_voltage - drain_voltage);
    stamp_complex_conductance(
        matrix,
        gate,
        source,
        Complex::new(gate_source_conductance, omega * gate_source_capacitance),
    );
    stamp_complex_conductance(
        matrix,
        gate,
        drain,
        Complex::new(gate_drain_conductance, omega * gate_drain_capacitance),
    );
    stamp_complex_transconductance(
        matrix,
        drain,
        source,
        gate,
        source,
        Complex::new(result.gm, 0.0),
    );
    if jfet.drain_resistance > 0.0 {
        stamp_complex_conductance(
            matrix,
            node_index(node_indices, &jfet.drain),
            drain,
            Complex::new(1.0 / jfet.drain_resistance, 0.0),
        );
    }
    if jfet.source_resistance > 0.0 {
        stamp_complex_conductance(
            matrix,
            node_index(node_indices, &jfet.source),
            source,
            Complex::new(1.0 / jfet.source_resistance, 0.0),
        );
    }
    Ok(())
}

fn validate_reactive_elements(circuit: &Circuit) -> Result<(), SpiceError> {
    for element in circuit.elements() {
        match element {
            Element::Capacitor(capacitor) => validate_capacitor(capacitor)?,
            Element::Inductor(inductor) => validate_inductor(inductor)?,
            Element::TransmissionLine(line) => validate_transmission_line(line)?,
            _ => {}
        }
    }
    Ok(())
}

fn diode_effective_thermal_voltage(diode: &Diode) -> f64 {
    diode.thermal_voltage * diode.emission_coefficient
}

fn diode_current_conductance(diode: &Diode, voltage: f64) -> (f64, f64) {
    let vt_eff = diode_effective_thermal_voltage(diode);
    let forward_voltage = voltage.min(0.7 * diode.emission_coefficient);
    let exp_value = (forward_voltage / vt_eff).clamp(-40.0, 40.0).exp();
    let mut current = diode.saturation_current * (exp_value - 1.0);
    let mut conductance = diode.saturation_current / vt_eff * exp_value;
    if let Some(breakdown_voltage) = diode.breakdown_voltage {
        if voltage <= -breakdown_voltage {
            let breakdown_exp_value = (((-voltage) - breakdown_voltage) / vt_eff)
                .clamp(-40.0, 40.0)
                .exp();
            current -= diode.breakdown_current * breakdown_exp_value;
            conductance += diode.breakdown_current / vt_eff * breakdown_exp_value;
        }
    }
    (current, conductance)
}

fn diode_charge_state_name(diode: &Diode) -> String {
    format!("_D_{}_charge", diode.name)
}

fn diode_has_charge_storage(diode: &Diode) -> bool {
    diode.junction_capacitance > 0.0 || diode.transit_time > 0.0
}

fn diode_dynamic_capacitance(diode: &Diode, voltage: f64) -> f64 {
    let (_, conductance) = diode_current_conductance(diode, voltage);
    diode_depletion_capacitance(diode, voltage) + diode.transit_time * conductance
}

fn diode_depletion_capacitance(diode: &Diode, voltage: f64) -> f64 {
    if diode.junction_capacitance <= 0.0 || diode.grading_coefficient == 0.0 {
        return diode.junction_capacitance;
    }
    let normalized_voltage = voltage / diode.junction_potential;
    if normalized_voltage < diode.forward_bias_depletion_coefficient {
        return diode.junction_capacitance
            / (1.0 - normalized_voltage).powf(diode.grading_coefficient);
    }
    let coefficient = diode.forward_bias_depletion_coefficient;
    let transition_scale = (1.0 - coefficient).powf(1.0 + diode.grading_coefficient);
    let continuation = 1.0 - coefficient * (1.0 + diode.grading_coefficient)
        + diode.grading_coefficient * normalized_voltage;
    diode.junction_capacitance * continuation / transition_scale
}

fn diode_charge_voltage(diode: &Diode, node_voltages: &BTreeMap<String, f64>) -> f64 {
    voltage_at(node_voltages, &diode_intrinsic_anode_node(diode))
        - voltage_at(node_voltages, &diode.cathode)
}

fn diode_intrinsic_anode_node(diode: &Diode) -> String {
    if diode.series_resistance == 0.0 {
        diode.anode.clone()
    } else {
        format!("_D_{}_anode", diode.name)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum BjtChargeStateKind {
    BaseEmitter,
    BaseCollector,
    ExternalBaseCollector,
}

struct BjtChargeStateSpec {
    name: String,
    positive: String,
    negative: String,
    kind: BjtChargeStateKind,
}

fn bjt_base_emitter_charge_state_name(bjt: &Bjt) -> String {
    format!("_Q_{}_be_charge", bjt.name)
}

fn bjt_base_collector_charge_state_name(bjt: &Bjt) -> String {
    format!("_Q_{}_bc_charge", bjt.name)
}

fn bjt_external_base_collector_charge_state_name(bjt: &Bjt) -> String {
    format!("_Q_{}_bx_charge", bjt.name)
}

fn bjt_intrinsic_emitter_node(bjt: &Bjt) -> String {
    if bjt.emitter_resistance == 0.0 {
        bjt.emitter.clone()
    } else {
        format!("__spice_{}_emitter", bjt.name)
    }
}

fn bjt_intrinsic_collector_node(bjt: &Bjt) -> String {
    if bjt.collector_resistance == 0.0 {
        bjt.collector.clone()
    } else {
        format!("__spice_{}_collector", bjt.name)
    }
}

fn bjt_intrinsic_base_node(bjt: &Bjt) -> String {
    if bjt.base_resistance == 0.0 {
        bjt.base.clone()
    } else {
        format!("__spice_{}_base", bjt.name)
    }
}

fn bjt_junction_transconductance(bjt: &Bjt, voltage: f64, emission_coefficient: f64) -> f64 {
    let effective_thermal_voltage = bjt.thermal_voltage * emission_coefficient;
    bjt.saturation_current / effective_thermal_voltage
        * (voltage / effective_thermal_voltage)
            .clamp(-40.0, 40.0)
            .exp()
}

fn bjt_forward_transit_time_scale(bjt: &Bjt, voltage: f64, reverse_junction_voltage: f64) -> f64 {
    let effective_thermal_voltage = bjt.thermal_voltage * bjt.forward_emission_coefficient;
    let forward_current = (bjt.saturation_current
        * ((voltage / effective_thermal_voltage)
            .clamp(-40.0, 40.0)
            .exp()
            - 1.0))
        .max(0.0);
    let current_factor = if bjt.forward_transit_time_current == 0.0 {
        1.0
    } else {
        let ratio = forward_current / (forward_current + bjt.forward_transit_time_current);
        ratio * ratio
    };
    let voltage_factor = if bjt.forward_transit_time_voltage == 0.0 {
        1.0
    } else {
        (reverse_junction_voltage / (1.44 * bjt.forward_transit_time_voltage))
            .clamp(-40.0, 40.0)
            .exp()
    };
    1.0 + bjt.forward_transit_time_bias_coefficient * current_factor * voltage_factor
}

fn bjt_charge_dynamic_capacitance(
    bjt: &Bjt,
    kind: BjtChargeStateKind,
    voltage: f64,
    reverse_junction_voltage: f64,
) -> f64 {
    match kind {
        BjtChargeStateKind::BaseEmitter => {
            let conductance =
                bjt_junction_transconductance(bjt, voltage, bjt.forward_emission_coefficient);
            bjt_base_emitter_depletion_capacitance(bjt, voltage)
                + bjt.forward_transit_time
                    * bjt_forward_transit_time_scale(bjt, voltage, reverse_junction_voltage)
                    * conductance
        }
        BjtChargeStateKind::BaseCollector => {
            let conductance =
                bjt_junction_transconductance(bjt, voltage, bjt.reverse_emission_coefficient);
            bjt.base_collector_capacitance_fraction
                * bjt_base_collector_depletion_capacitance(bjt, voltage)
                + bjt.reverse_transit_time * conductance
        }
        BjtChargeStateKind::ExternalBaseCollector => {
            (1.0 - bjt.base_collector_capacitance_fraction)
                * bjt_base_collector_depletion_capacitance(bjt, voltage)
        }
    }
}

fn bjt_base_emitter_depletion_capacitance(bjt: &Bjt, voltage: f64) -> f64 {
    if bjt.base_emitter_capacitance <= 0.0 || bjt.base_emitter_grading_coefficient == 0.0 {
        return bjt.base_emitter_capacitance;
    }
    let normalized_voltage = voltage / bjt.base_emitter_junction_potential;
    let coefficient = bjt.forward_bias_depletion_coefficient;
    if normalized_voltage < coefficient {
        return bjt.base_emitter_capacitance
            / (1.0 - normalized_voltage).powf(bjt.base_emitter_grading_coefficient);
    }
    let transition_scale = (1.0_f64 - coefficient).powf(1.0 + bjt.base_emitter_grading_coefficient);
    let continuation = 1.0 - coefficient * (1.0 + bjt.base_emitter_grading_coefficient)
        + bjt.base_emitter_grading_coefficient * normalized_voltage;
    bjt.base_emitter_capacitance * continuation / transition_scale
}

fn bjt_base_collector_depletion_capacitance(bjt: &Bjt, voltage: f64) -> f64 {
    if bjt.base_collector_capacitance <= 0.0 || bjt.base_collector_grading_coefficient == 0.0 {
        return bjt.base_collector_capacitance;
    }
    let normalized_voltage = voltage / bjt.base_collector_junction_potential;
    let coefficient = bjt.forward_bias_depletion_coefficient;
    if normalized_voltage < coefficient {
        return bjt.base_collector_capacitance
            / (1.0 - normalized_voltage).powf(bjt.base_collector_grading_coefficient);
    }
    let transition_scale =
        (1.0_f64 - coefficient).powf(1.0 + bjt.base_collector_grading_coefficient);
    let continuation = 1.0 - coefficient * (1.0 + bjt.base_collector_grading_coefficient)
        + bjt.base_collector_grading_coefficient * normalized_voltage;
    bjt.base_collector_capacitance * continuation / transition_scale
}

fn bjt_charge_state_specs(bjt: &Bjt) -> Vec<BjtChargeStateSpec> {
    let mut specs = Vec::new();
    let base = bjt_intrinsic_base_node(bjt);
    if bjt.base_emitter_capacitance > 0.0 || bjt.forward_transit_time > 0.0 {
        let emitter = bjt_intrinsic_emitter_node(bjt);
        let (positive, negative) = match bjt.polarity {
            BjtPolarity::Npn => (base.clone(), emitter),
            BjtPolarity::Pnp => (emitter, base.clone()),
        };
        specs.push(BjtChargeStateSpec {
            name: bjt_base_emitter_charge_state_name(bjt),
            positive,
            negative,
            kind: BjtChargeStateKind::BaseEmitter,
        });
    }
    if bjt.base_collector_capacitance > 0.0
        || bjt.reverse_transit_time > 0.0
        || (bjt.forward_transit_time > 0.0
            && bjt.forward_transit_time_bias_coefficient > 0.0
            && bjt.forward_transit_time_voltage > 0.0)
    {
        let collector = bjt_intrinsic_collector_node(bjt);
        let (positive, negative) = match bjt.polarity {
            BjtPolarity::Npn => (base.clone(), collector),
            BjtPolarity::Pnp => (collector, base),
        };
        specs.push(BjtChargeStateSpec {
            name: bjt_base_collector_charge_state_name(bjt),
            positive,
            negative,
            kind: BjtChargeStateKind::BaseCollector,
        });
    }
    if bjt.base_collector_capacitance > 0.0 && bjt.base_collector_capacitance_fraction < 1.0 {
        let collector = bjt_intrinsic_collector_node(bjt);
        let (positive, negative) = match bjt.polarity {
            BjtPolarity::Npn => (bjt.base.clone(), collector),
            BjtPolarity::Pnp => (collector, bjt.base.clone()),
        };
        specs.push(BjtChargeStateSpec {
            name: bjt_external_base_collector_charge_state_name(bjt),
            positive,
            negative,
            kind: BjtChargeStateKind::ExternalBaseCollector,
        });
    }
    specs
}

fn bjt_charge_state_voltage(
    spec: &BjtChargeStateSpec,
    node_voltages: &BTreeMap<String, f64>,
) -> f64 {
    voltage_at(node_voltages, &spec.positive) - voltage_at(node_voltages, &spec.negative)
}

struct JfetChargeStateSpec {
    name: String,
    positive: String,
    negative: String,
    capacitance: f64,
}

fn jfet_gate_source_charge_state_name(jfet: &Jfet) -> String {
    format!("_J_{}_gs_charge", jfet.name)
}

fn jfet_gate_drain_charge_state_name(jfet: &Jfet) -> String {
    format!("_J_{}_gd_charge", jfet.name)
}

fn jfet_charge_state_specs(jfet: &Jfet) -> Vec<JfetChargeStateSpec> {
    let mut specs = Vec::new();
    if jfet.gate_source_capacitance > 0.0 {
        specs.push(JfetChargeStateSpec {
            name: jfet_gate_source_charge_state_name(jfet),
            positive: jfet.gate.clone(),
            negative: jfet_intrinsic_source_node(jfet),
            capacitance: jfet.gate_source_capacitance,
        });
    }
    if jfet.gate_drain_capacitance > 0.0 {
        specs.push(JfetChargeStateSpec {
            name: jfet_gate_drain_charge_state_name(jfet),
            positive: jfet.gate.clone(),
            negative: jfet_intrinsic_drain_node(jfet),
            capacitance: jfet.gate_drain_capacitance,
        });
    }
    specs
}

fn jfet_charge_state_voltage(
    spec: &JfetChargeStateSpec,
    node_voltages: &BTreeMap<String, f64>,
) -> f64 {
    voltage_at(node_voltages, &spec.positive) - voltage_at(node_voltages, &spec.negative)
}

fn jfet_intrinsic_drain_node(jfet: &Jfet) -> String {
    if jfet.drain_resistance == 0.0 {
        jfet.drain.clone()
    } else {
        format!("__spice_{}_drain", jfet.name)
    }
}

fn jfet_intrinsic_source_node(jfet: &Jfet) -> String {
    if jfet.source_resistance == 0.0 {
        jfet.source.clone()
    } else {
        format!("__spice_{}_source", jfet.name)
    }
}

fn mosfet_intrinsic_drain_node(mosfet: &Mosfet) -> String {
    let drain_resistance = mosfet_drain_resistance(mosfet);
    if !drain_resistance.is_finite() || drain_resistance <= 0.0 {
        mosfet.drain.clone()
    } else {
        format!("__spice_{}_drain", mosfet.name)
    }
}

fn mosfet_intrinsic_source_node(mosfet: &Mosfet) -> String {
    let source_resistance = mosfet_source_resistance(mosfet);
    if !source_resistance.is_finite() || source_resistance <= 0.0 {
        mosfet.source.clone()
    } else {
        format!("__spice_{}_source", mosfet.name)
    }
}

fn mosfet_drain_resistance(mosfet: &Mosfet) -> f64 {
    if mosfet.params.drain_resistance > 0.0 {
        mosfet.params.drain_resistance
    } else {
        mosfet.params.sheet_resistance * mosfet.params.drain_squares
    }
}

fn mosfet_source_resistance(mosfet: &Mosfet) -> f64 {
    if mosfet.params.source_resistance > 0.0 {
        mosfet.params.source_resistance
    } else {
        mosfet.params.sheet_resistance * mosfet.params.source_squares
    }
}

fn jfet_charge_dynamic_capacitance(
    jfet: &Jfet,
    zero_bias_capacitance: f64,
    junction_voltage: f64,
) -> f64 {
    const GRADING_COEFFICIENT: f64 = 0.5;
    let oriented_voltage = match jfet.polarity {
        JfetPolarity::Njf => junction_voltage,
        JfetPolarity::Pjf => -junction_voltage,
    };
    let normalized_voltage = oriented_voltage / jfet.junction_potential;
    if normalized_voltage < jfet.forward_bias_depletion_coefficient {
        return zero_bias_capacitance / (1.0 - normalized_voltage).powf(GRADING_COEFFICIENT);
    }
    let transition_scale =
        (1.0 - jfet.forward_bias_depletion_coefficient).powf(1.0 + GRADING_COEFFICIENT);
    let continuation = 1.0 - jfet.forward_bias_depletion_coefficient * (1.0 + GRADING_COEFFICIENT)
        + GRADING_COEFFICIENT * normalized_voltage;
    zero_bias_capacitance * continuation / transition_scale
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum MosfetChargeStateKind {
    GateOverlap,
    SourceBody,
    DrainBody,
}

struct MosfetChargeStateSpec {
    name: String,
    positive: String,
    negative: String,
    capacitance: f64,
    kind: MosfetChargeStateKind,
}

fn mosfet_gate_source_charge_state_name(mosfet: &Mosfet) -> String {
    format!("_M_{}_gs_charge", mosfet.name)
}

fn mosfet_gate_drain_charge_state_name(mosfet: &Mosfet) -> String {
    format!("_M_{}_gd_charge", mosfet.name)
}

fn mosfet_gate_body_charge_state_name(mosfet: &Mosfet) -> String {
    format!("_M_{}_gb_charge", mosfet.name)
}

fn mosfet_source_body_charge_state_name(mosfet: &Mosfet) -> String {
    format!("_M_{}_sb_charge", mosfet.name)
}

fn mosfet_drain_body_charge_state_name(mosfet: &Mosfet) -> String {
    format!("_M_{}_db_charge", mosfet.name)
}

fn mosfet_charge_state_specs(mosfet: &Mosfet) -> Vec<MosfetChargeStateSpec> {
    let mut specs = Vec::new();
    let params = mosfet.params;
    let gate_source_capacitance = params.gate_source_overlap_capacitance * params.w;
    let gate_drain_capacitance = params.gate_drain_overlap_capacitance * params.w;
    let gate_body_capacitance = params.gate_bulk_overlap_capacitance * params.l;
    let source_body_capacitance = params.source_bulk_capacitance
        + params.bottom_junction_capacitance * params.source_area
        + params.sidewall_junction_capacitance * params.source_perimeter;
    let drain_body_capacitance = params.drain_bulk_capacitance
        + params.bottom_junction_capacitance * params.drain_area
        + params.sidewall_junction_capacitance * params.drain_perimeter;
    if gate_source_capacitance > 0.0 {
        specs.push(MosfetChargeStateSpec {
            name: mosfet_gate_source_charge_state_name(mosfet),
            positive: mosfet.gate.clone(),
            negative: mosfet_intrinsic_source_node(mosfet),
            capacitance: gate_source_capacitance,
            kind: MosfetChargeStateKind::GateOverlap,
        });
    }
    if gate_drain_capacitance > 0.0 {
        specs.push(MosfetChargeStateSpec {
            name: mosfet_gate_drain_charge_state_name(mosfet),
            positive: mosfet.gate.clone(),
            negative: mosfet_intrinsic_drain_node(mosfet),
            capacitance: gate_drain_capacitance,
            kind: MosfetChargeStateKind::GateOverlap,
        });
    }
    if gate_body_capacitance > 0.0 {
        specs.push(MosfetChargeStateSpec {
            name: mosfet_gate_body_charge_state_name(mosfet),
            positive: mosfet.gate.clone(),
            negative: mosfet.body.clone(),
            capacitance: gate_body_capacitance,
            kind: MosfetChargeStateKind::GateOverlap,
        });
    }
    if source_body_capacitance > 0.0 {
        specs.push(MosfetChargeStateSpec {
            name: mosfet_source_body_charge_state_name(mosfet),
            positive: mosfet_intrinsic_source_node(mosfet),
            negative: mosfet.body.clone(),
            capacitance: source_body_capacitance,
            kind: MosfetChargeStateKind::SourceBody,
        });
    }
    if drain_body_capacitance > 0.0 {
        specs.push(MosfetChargeStateSpec {
            name: mosfet_drain_body_charge_state_name(mosfet),
            positive: mosfet_intrinsic_drain_node(mosfet),
            negative: mosfet.body.clone(),
            capacitance: drain_body_capacitance,
            kind: MosfetChargeStateKind::DrainBody,
        });
    }
    specs
}

fn mosfet_charge_state_voltage(
    spec: &MosfetChargeStateSpec,
    node_voltages: &BTreeMap<String, f64>,
) -> f64 {
    voltage_at(node_voltages, &spec.positive) - voltage_at(node_voltages, &spec.negative)
}

fn mosfet_charge_dynamic_capacitance(
    mosfet: &Mosfet,
    spec: &MosfetChargeStateSpec,
    state_voltage: f64,
) -> f64 {
    if !matches!(
        spec.kind,
        MosfetChargeStateKind::SourceBody | MosfetChargeStateKind::DrainBody
    ) {
        return spec.capacitance;
    }
    let junction_voltage = match mosfet.mosfet_type {
        MosfetType::Pmos => state_voltage,
        MosfetType::Nmos => -state_voltage,
    };
    let (bottom_capacitance, sidewall_capacitance) = match spec.kind {
        MosfetChargeStateKind::SourceBody => (
            mosfet.params.source_bulk_capacitance
                + mosfet.params.bottom_junction_capacitance * mosfet.params.source_area,
            mosfet.params.sidewall_junction_capacitance * mosfet.params.source_perimeter,
        ),
        MosfetChargeStateKind::DrainBody => (
            mosfet.params.drain_bulk_capacitance
                + mosfet.params.bottom_junction_capacitance * mosfet.params.drain_area,
            mosfet.params.sidewall_junction_capacitance * mosfet.params.drain_perimeter,
        ),
        _ => unreachable!(),
    };
    mosfet_bulk_junction_capacitance(
        bottom_capacitance,
        junction_voltage,
        mosfet.params.bulk_junction_potential,
        mosfet.params.bulk_junction_grading_coefficient,
        mosfet.params.forward_bias_depletion_coefficient,
    ) + mosfet_bulk_junction_capacitance(
        sidewall_capacitance,
        junction_voltage,
        mosfet.params.bulk_junction_potential,
        mosfet.params.sidewall_junction_grading_coefficient,
        mosfet.params.forward_bias_depletion_coefficient,
    )
}

fn validate_diode(diode: &Diode) -> Result<(), SpiceError> {
    if !diode.flicker_noise_exponent.is_finite() || diode.flicker_noise_exponent < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "flicker-noise exponent must be finite and non-negative".to_string(),
        });
    }
    if !diode.flicker_noise_coefficient.is_finite() || diode.flicker_noise_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "flicker-noise coefficient must be finite and non-negative".to_string(),
        });
    }
    if !diode.series_resistance.is_finite() || diode.series_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "series resistance must be finite and non-negative".to_string(),
        });
    }
    if !diode.saturation_current.is_finite() || diode.saturation_current <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "saturation current must be finite and positive".to_string(),
        });
    }
    if !diode.thermal_voltage.is_finite() || diode.thermal_voltage <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "thermal voltage must be finite and positive".to_string(),
        });
    }
    if !diode.emission_coefficient.is_finite() || diode.emission_coefficient <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "emission coefficient must be finite and positive".to_string(),
        });
    }
    if let Some(breakdown_voltage) = diode.breakdown_voltage {
        if !breakdown_voltage.is_finite() || breakdown_voltage <= 0.0 {
            return Err(SpiceError::InvalidElement {
                name: diode.name.clone(),
                reason: "breakdown voltage must be finite and positive".to_string(),
            });
        }
    }
    if !diode.breakdown_current.is_finite() || diode.breakdown_current <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "breakdown current must be finite and positive".to_string(),
        });
    }
    if !diode.junction_capacitance.is_finite() || diode.junction_capacitance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "junction capacitance must be finite and non-negative".to_string(),
        });
    }
    if !diode.junction_potential.is_finite() || diode.junction_potential <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "junction potential must be finite and positive".to_string(),
        });
    }
    if !diode.grading_coefficient.is_finite() || diode.grading_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "grading coefficient must be finite and non-negative".to_string(),
        });
    }
    if !diode.forward_bias_depletion_coefficient.is_finite()
        || diode.forward_bias_depletion_coefficient < 0.0
        || diode.forward_bias_depletion_coefficient >= 1.0
    {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "forward-bias depletion coefficient must be finite and in [0, 1)".to_string(),
        });
    }
    if !diode.saturation_current_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "saturation-current temperature exponent must be finite".to_string(),
        });
    }
    if !diode.energy_gap_electron_volts.is_finite() || diode.energy_gap_electron_volts <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "energy gap must be finite and positive".to_string(),
        });
    }
    if !diode.transit_time.is_finite() || diode.transit_time < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: diode.name.clone(),
            reason: "transit time must be finite and non-negative".to_string(),
        });
    }
    Ok(())
}

fn validate_bjt(bjt: &Bjt) -> Result<(), SpiceError> {
    if !bjt.saturation_current.is_finite() || bjt.saturation_current <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "saturation current must be finite and positive".to_string(),
        });
    }
    if !bjt.forward_beta.is_finite() || bjt.forward_beta <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward beta must be finite and positive".to_string(),
        });
    }
    if bjt.reverse_beta.is_nan() || bjt.reverse_beta <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "reverse beta must be positive".to_string(),
        });
    }
    if !bjt.thermal_voltage.is_finite() || bjt.thermal_voltage <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "thermal voltage must be finite and positive".to_string(),
        });
    }
    if !bjt.base_emitter_capacitance.is_finite() || bjt.base_emitter_capacitance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-emitter capacitance must be finite and non-negative".to_string(),
        });
    }
    if !bjt.base_collector_capacitance.is_finite() || bjt.base_collector_capacitance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-collector capacitance must be finite and non-negative".to_string(),
        });
    }
    if !bjt.forward_transit_time.is_finite() || bjt.forward_transit_time < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward transit time must be finite and non-negative".to_string(),
        });
    }
    if !bjt.reverse_transit_time.is_finite() || bjt.reverse_transit_time < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "reverse transit time must be finite and non-negative".to_string(),
        });
    }
    if !bjt.saturation_current_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "saturation-current temperature exponent must be finite".to_string(),
        });
    }
    if !bjt.forward_beta_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "beta temperature exponent must be finite".to_string(),
        });
    }
    if !bjt.energy_gap_electron_volts.is_finite() || bjt.energy_gap_electron_volts <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "energy gap must be finite and positive".to_string(),
        });
    }
    if !bjt.forward_early_voltage.is_finite() || bjt.forward_early_voltage < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward Early voltage must be finite and non-negative".to_string(),
        });
    }
    if !bjt.reverse_early_voltage.is_finite() || bjt.reverse_early_voltage < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "reverse Early voltage must be finite and non-negative".to_string(),
        });
    }
    if !bjt.forward_beta_rolloff_current.is_finite() || bjt.forward_beta_rolloff_current < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward beta roll-off current must be finite and non-negative".to_string(),
        });
    }
    if !bjt.reverse_beta_rolloff_current.is_finite() || bjt.reverse_beta_rolloff_current < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "reverse beta roll-off current must be finite and non-negative".to_string(),
        });
    }
    if let Some(nominal_temperature_kelvin) = bjt.nominal_temperature_kelvin {
        if !nominal_temperature_kelvin.is_finite() || nominal_temperature_kelvin <= 0.0 {
            return Err(SpiceError::InvalidElement {
                name: bjt.name.clone(),
                reason: "nominal temperature must be finite and positive".to_string(),
            });
        }
    }
    if !bjt.flicker_noise_coefficient.is_finite() || bjt.flicker_noise_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "flicker noise coefficient must be finite and non-negative".to_string(),
        });
    }
    if !bjt.flicker_noise_exponent.is_finite() || bjt.flicker_noise_exponent < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "flicker noise exponent must be finite and non-negative".to_string(),
        });
    }
    if !bjt.forward_excess_phase_degrees.is_finite() || bjt.forward_excess_phase_degrees < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward excess phase must be finite and non-negative".to_string(),
        });
    }
    if !bjt.forward_transit_time_bias_coefficient.is_finite()
        || bjt.forward_transit_time_bias_coefficient < 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward transit-time bias coefficient must be finite and non-negative"
                .to_string(),
        });
    }
    if !bjt.forward_transit_time_current.is_finite() || bjt.forward_transit_time_current < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward transit-time current must be finite and non-negative".to_string(),
        });
    }
    if !bjt.forward_transit_time_voltage.is_finite() || bjt.forward_transit_time_voltage < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward transit-time voltage must be finite and non-negative".to_string(),
        });
    }
    if !bjt.emitter_resistance.is_finite() || bjt.emitter_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "emitter resistance must be finite and non-negative".to_string(),
        });
    }
    if !bjt.collector_resistance.is_finite() || bjt.collector_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "collector resistance must be finite and non-negative".to_string(),
        });
    }
    if !bjt.base_resistance.is_finite() || bjt.base_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base resistance must be finite and non-negative".to_string(),
        });
    }
    if bjt
        .minimum_base_resistance
        .is_some_and(|resistance| !resistance.is_finite() || resistance < 0.0)
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "minimum base resistance must be finite and non-negative".to_string(),
        });
    }
    if !bjt.base_resistance_half_current.is_finite() || bjt.base_resistance_half_current < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-resistance half-current must be finite and non-negative".to_string(),
        });
    }
    if !bjt.base_collector_capacitance_fraction.is_finite()
        || !(0.0..=1.0).contains(&bjt.base_collector_capacitance_fraction)
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-collector capacitance fraction must be between zero and one".to_string(),
        });
    }
    if !bjt.base_emitter_leakage_saturation_current.is_finite()
        || bjt.base_emitter_leakage_saturation_current < 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-emitter leakage saturation current must be finite and non-negative"
                .to_string(),
        });
    }
    if !bjt.base_emitter_leakage_emission_coefficient.is_finite()
        || bjt.base_emitter_leakage_emission_coefficient <= 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-emitter leakage emission coefficient must be finite and positive"
                .to_string(),
        });
    }
    if !bjt.base_collector_leakage_saturation_current.is_finite()
        || bjt.base_collector_leakage_saturation_current < 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-collector leakage saturation current must be finite and non-negative"
                .to_string(),
        });
    }
    if !bjt.base_collector_leakage_emission_coefficient.is_finite()
        || bjt.base_collector_leakage_emission_coefficient <= 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-collector leakage emission coefficient must be finite and positive"
                .to_string(),
        });
    }
    if !bjt.forward_emission_coefficient.is_finite() || bjt.forward_emission_coefficient <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward emission coefficient must be finite and positive".to_string(),
        });
    }
    if !bjt.reverse_emission_coefficient.is_finite() || bjt.reverse_emission_coefficient <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "reverse emission coefficient must be finite and positive".to_string(),
        });
    }
    if !bjt.base_emitter_junction_potential.is_finite()
        || bjt.base_emitter_junction_potential <= 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-emitter junction potential must be finite and positive".to_string(),
        });
    }
    if !bjt.base_emitter_grading_coefficient.is_finite()
        || !(0.0..1.0).contains(&bjt.base_emitter_grading_coefficient)
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-emitter grading coefficient must be finite and in [0, 1)".to_string(),
        });
    }
    if !bjt.base_collector_junction_potential.is_finite()
        || bjt.base_collector_junction_potential <= 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-collector junction potential must be finite and positive".to_string(),
        });
    }
    if !bjt.base_collector_grading_coefficient.is_finite()
        || !(0.0..1.0).contains(&bjt.base_collector_grading_coefficient)
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "base-collector grading coefficient must be finite and in [0, 1)".to_string(),
        });
    }
    if !bjt.forward_bias_depletion_coefficient.is_finite()
        || !(0.0..1.0).contains(&bjt.forward_bias_depletion_coefficient)
    {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "forward-bias depletion coefficient must be finite and in [0, 1)".to_string(),
        });
    }
    Ok(())
}

fn validate_jfet(jfet: &Jfet) -> Result<(), SpiceError> {
    if !jfet.beta.is_finite() || jfet.beta <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "beta must be finite and positive".to_string(),
        });
    }
    if !jfet.threshold_voltage.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "threshold voltage must be finite".to_string(),
        });
    }
    if !jfet.channel_length_modulation.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "channel length modulation must be finite".to_string(),
        });
    }
    if !jfet.gate_source_capacitance.is_finite() || jfet.gate_source_capacitance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "gate-source capacitance must be finite and non-negative".to_string(),
        });
    }
    if !jfet.gate_drain_capacitance.is_finite() || jfet.gate_drain_capacitance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "gate-drain capacitance must be finite and non-negative".to_string(),
        });
    }
    if !jfet.flicker_noise_coefficient.is_finite() || jfet.flicker_noise_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "flicker-noise coefficient must be finite and non-negative".to_string(),
        });
    }
    if !jfet.flicker_noise_exponent.is_finite() || jfet.flicker_noise_exponent < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "flicker-noise exponent must be finite and non-negative".to_string(),
        });
    }
    if !jfet.junction_potential.is_finite() || jfet.junction_potential <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "junction potential must be finite and positive".to_string(),
        });
    }
    if !jfet.forward_bias_depletion_coefficient.is_finite()
        || jfet.forward_bias_depletion_coefficient < 0.0
        || jfet.forward_bias_depletion_coefficient >= 1.0
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "forward-bias depletion coefficient must be finite and in [0, 1)".to_string(),
        });
    }
    if !jfet.gate_saturation_current.is_finite() || jfet.gate_saturation_current < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "gate saturation current must be finite and non-negative".to_string(),
        });
    }
    if !jfet
        .gate_saturation_current_temperature_exponent
        .is_finite()
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "gate saturation-current temperature exponent must be finite".to_string(),
        });
    }
    if !jfet.bandgap_voltage.is_finite() || jfet.bandgap_voltage <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "bandgap voltage must be finite and positive".to_string(),
        });
    }
    if !jfet.doping_tail_parameter.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "doping-tail parameter must be finite".to_string(),
        });
    }
    if !jfet.noise_equation_level.is_finite()
        || jfet.noise_equation_level < 1.0
        || jfet.noise_equation_level.fract() != 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "noise equation level must be a finite integer greater than or equal to 1"
                .to_string(),
        });
    }
    if !jfet.channel_noise_coefficient.is_finite() || jfet.channel_noise_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "channel noise coefficient must be finite and non-negative".to_string(),
        });
    }
    let effective_threshold = match jfet.polarity {
        JfetPolarity::Njf => jfet.threshold_voltage,
        JfetPolarity::Pjf => -jfet.threshold_voltage,
    };
    if jfet.doping_tail_parameter != 1.0 && jfet.junction_potential == effective_threshold {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "junction potential minus effective threshold voltage must be non-zero when doping-tail parameter differs from 1".to_string(),
        });
    }
    if !jfet.drain_resistance.is_finite() || jfet.drain_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "drain resistance must be finite and non-negative".to_string(),
        });
    }
    if !jfet.source_resistance.is_finite() || jfet.source_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "source resistance must be finite and non-negative".to_string(),
        });
    }
    if !jfet.threshold_voltage_temperature_coefficient.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "threshold-voltage temperature coefficient must be finite".to_string(),
        });
    }
    if jfet
        .nominal_temperature_kelvin
        .is_some_and(|temperature| !temperature.is_finite() || temperature <= 0.0)
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "nominal temperature must be finite and positive".to_string(),
        });
    }
    if !jfet.mobility_temperature_exponent.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "mobility temperature exponent must be finite".to_string(),
        });
    }
    if jfet
        .alternative_threshold_voltage_temperature_coefficient
        .is_some_and(|coefficient| !coefficient.is_finite())
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "alternative threshold-voltage temperature coefficient must be finite"
                .to_string(),
        });
    }
    if jfet
        .mobility_temperature_coefficient
        .is_some_and(|coefficient| !coefficient.is_finite())
    {
        return Err(SpiceError::InvalidElement {
            name: jfet.name.clone(),
            reason: "mobility temperature coefficient must be finite".to_string(),
        });
    }
    Ok(())
}

fn validate_mosfet(mosfet: &Mosfet) -> Result<(), SpiceError> {
    let params = mosfet.params;
    for (name, value) in [
        ("VT0", params.vt0),
        ("KP", params.kp),
        ("LAMBDA", params.lambda),
        ("GAMMA", params.gamma),
        ("PHI", params.phi),
        ("W", params.w),
        ("L", params.l),
        ("LD", params.lateral_diffusion_length),
        ("RD", params.drain_resistance),
        ("RS", params.source_resistance),
        ("RSH", params.sheet_resistance),
        ("NRD", params.drain_squares),
        ("NRS", params.source_squares),
        ("AD", params.drain_area),
        ("AS", params.source_area),
        ("PD", params.drain_perimeter),
        ("PS", params.source_perimeter),
        ("CJ", params.bottom_junction_capacitance),
        ("CJSW", params.sidewall_junction_capacitance),
        ("TOX", params.oxide_thickness),
        ("U0", params.surface_mobility),
        ("IS", params.saturation_current),
        ("JS", params.saturation_current_density),
        ("N_SUB", params.n_sub),
        ("T_NOM", params.t_nom),
        ("CGSO", params.gate_source_overlap_capacitance),
        ("CGDO", params.gate_drain_overlap_capacitance),
        ("CGBO", params.gate_bulk_overlap_capacitance),
        ("CBS", params.source_bulk_capacitance),
        ("CBD", params.drain_bulk_capacitance),
        ("PB", params.bulk_junction_potential),
        ("MJ", params.bulk_junction_grading_coefficient),
        ("MJSW", params.sidewall_junction_grading_coefficient),
        ("FC", params.forward_bias_depletion_coefficient),
        ("KF", params.flicker_noise_coefficient),
        ("AF", params.flicker_noise_exponent),
    ] {
        if !value.is_finite() {
            return Err(SpiceError::InvalidElement {
                name: mosfet.name.clone(),
                reason: format!("MOSFET {name} must be finite"),
            });
        }
    }
    if params.kp <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET KP must be positive".to_string(),
        });
    }
    if params.w <= 0.0 || params.l <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET W and L must be positive".to_string(),
        });
    }
    if params.lateral_diffusion_length < 0.0
        || params.l - 2.0 * params.lateral_diffusion_length <= 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET LD must be non-negative with L - 2*LD > 0".to_string(),
        });
    }
    if params.drain_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET RD must be non-negative".to_string(),
        });
    }
    if params.source_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET RS must be non-negative".to_string(),
        });
    }
    if params.sheet_resistance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET RSH must be non-negative".to_string(),
        });
    }
    if params.drain_squares < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET NRD must be non-negative".to_string(),
        });
    }
    if params.source_squares < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET NRS must be non-negative".to_string(),
        });
    }
    if params.drain_area < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET AD must be non-negative".to_string(),
        });
    }
    if params.source_area < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET AS must be non-negative".to_string(),
        });
    }
    if params.drain_perimeter < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET PD must be non-negative".to_string(),
        });
    }
    if params.source_perimeter < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET PS must be non-negative".to_string(),
        });
    }
    if params.bottom_junction_capacitance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET CJ must be non-negative".to_string(),
        });
    }
    if params.sidewall_junction_capacitance < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET CJSW must be non-negative".to_string(),
        });
    }
    if params.oxide_thickness <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET TOX must be positive".to_string(),
        });
    }
    if params.surface_mobility < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET U0 must be non-negative".to_string(),
        });
    }
    if params.phi <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET PHI must be positive".to_string(),
        });
    }
    if params.saturation_current <= 0.0 || params.n_sub <= 0.0 || params.t_nom <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET IS, N_SUB, and T_NOM must be positive".to_string(),
        });
    }
    if params.saturation_current_density < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET JS must be non-negative".to_string(),
        });
    }
    if params.bulk_junction_potential <= 0.0 || params.bulk_junction_grading_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET PB must be positive and MJ must be non-negative".to_string(),
        });
    }
    if params.sidewall_junction_grading_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET MJSW must be non-negative".to_string(),
        });
    }
    if !(0.0..1.0).contains(&params.forward_bias_depletion_coefficient) {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET FC must be in [0, 1)".to_string(),
        });
    }
    if params.flicker_noise_coefficient < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET KF must be non-negative".to_string(),
        });
    }
    if params.flicker_noise_exponent < 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET AF must be non-negative".to_string(),
        });
    }
    if params.gate_source_overlap_capacitance < 0.0
        || params.gate_drain_overlap_capacitance < 0.0
        || params.gate_bulk_overlap_capacitance < 0.0
        || params.source_bulk_capacitance < 0.0
        || params.drain_bulk_capacitance < 0.0
    {
        return Err(SpiceError::InvalidElement {
            name: mosfet.name.clone(),
            reason: "MOSFET capacitances must be non-negative".to_string(),
        });
    }
    Ok(())
}

fn validate_capacitor(capacitor: &Capacitor) -> Result<(), SpiceError> {
    if !capacitor.capacitance_farads.is_finite() || capacitor.capacitance_farads <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "capacitance must be finite and positive".to_string(),
        });
    }
    if !capacitor.initial_voltage.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: capacitor.name.clone(),
            reason: "initial voltage must be finite".to_string(),
        });
    }
    Ok(())
}

fn validate_inductor(inductor: &Inductor) -> Result<(), SpiceError> {
    if !inductor.inductance_henrys.is_finite() || inductor.inductance_henrys <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: inductor.name.clone(),
            reason: "inductance must be finite and positive".to_string(),
        });
    }
    if !inductor.initial_current.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: inductor.name.clone(),
            reason: "initial current must be finite".to_string(),
        });
    }
    Ok(())
}

fn validate_transmission_line(line: &TransmissionLine) -> Result<(), SpiceError> {
    if !line.characteristic_impedance_ohms.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "characteristic impedance must be finite".to_string(),
        });
    }
    if line.characteristic_impedance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "characteristic impedance must be positive".to_string(),
        });
    }
    if !line.delay_seconds.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "delay must be finite".to_string(),
        });
    }
    if line.delay_seconds <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "delay must be positive".to_string(),
        });
    }
    Ok(())
}

fn initial_capacitor_states(
    circuit: &Circuit,
    time_step: f64,
    method: TransientMethod,
) -> Vec<CapacitorState> {
    let mut states = Vec::new();
    for element in circuit.elements() {
        match element {
            Element::Capacitor(capacitor) => states.push(CapacitorState {
                name: capacitor.name.clone(),
                previous_voltage: capacitor.initial_voltage,
                previous_previous_voltage: capacitor.initial_voltage,
                previous_current: 0.0,
                time_step,
                method,
            }),
            Element::Diode(diode) if diode_has_charge_storage(diode) => {
                states.push(CapacitorState {
                    name: diode_charge_state_name(diode),
                    previous_voltage: 0.0,
                    previous_previous_voltage: 0.0,
                    previous_current: 0.0,
                    time_step,
                    method,
                });
            }
            Element::Bjt(bjt) => {
                for spec in bjt_charge_state_specs(bjt) {
                    states.push(CapacitorState {
                        name: spec.name,
                        previous_voltage: 0.0,
                        previous_previous_voltage: 0.0,
                        previous_current: 0.0,
                        time_step,
                        method,
                    });
                }
            }
            Element::Jfet(jfet) => {
                for spec in jfet_charge_state_specs(jfet) {
                    states.push(CapacitorState {
                        name: spec.name,
                        previous_voltage: 0.0,
                        previous_previous_voltage: 0.0,
                        previous_current: 0.0,
                        time_step,
                        method,
                    });
                }
            }
            Element::Mosfet(mosfet) => {
                for spec in mosfet_charge_state_specs(mosfet) {
                    states.push(CapacitorState {
                        name: spec.name,
                        previous_voltage: 0.0,
                        previous_previous_voltage: 0.0,
                        previous_current: 0.0,
                        time_step,
                        method,
                    });
                }
            }
            _ => {}
        }
    }
    states
}

fn initial_inductor_states(
    circuit: &Circuit,
    time_step: f64,
    method: TransientMethod,
) -> Vec<InductorState> {
    circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::Inductor(inductor) => Some(InductorState {
                name: inductor.name.clone(),
                previous_current: inductor.initial_current,
                previous_previous_current: inductor.initial_current,
                previous_voltage: 0.0,
                time_step,
                method,
            }),
            _ => None,
        })
        .collect()
}

fn set_reactive_state_method(
    capacitor_states: &mut [CapacitorState],
    inductor_states: &mut [InductorState],
    method: TransientMethod,
) {
    for state in capacitor_states {
        state.method = method;
    }
    for state in inductor_states {
        state.method = method;
    }
}

fn set_reactive_state_step(
    capacitor_states: &mut [CapacitorState],
    inductor_states: &mut [InductorState],
    time_step: f64,
) {
    for state in capacitor_states {
        state.time_step = time_step;
    }
    for state in inductor_states {
        state.time_step = time_step;
    }
}

fn capacitor_voltages(
    circuit: &Circuit,
    node_voltages: &BTreeMap<String, f64>,
) -> BTreeMap<String, f64> {
    let mut voltages = BTreeMap::new();
    for element in circuit.elements() {
        match element {
            Element::Capacitor(capacitor) => {
                voltages.insert(
                    capacitor.name.clone(),
                    voltage_at(node_voltages, &capacitor.n1)
                        - voltage_at(node_voltages, &capacitor.n2),
                );
            }
            Element::Diode(diode) if diode_has_charge_storage(diode) => {
                voltages.insert(
                    diode_charge_state_name(diode),
                    diode_charge_voltage(diode, node_voltages),
                );
            }
            Element::Bjt(bjt) => {
                for spec in bjt_charge_state_specs(bjt) {
                    voltages.insert(
                        spec.name.clone(),
                        bjt_charge_state_voltage(&spec, node_voltages),
                    );
                }
            }
            Element::Jfet(jfet) => {
                for spec in jfet_charge_state_specs(jfet) {
                    voltages.insert(
                        spec.name.clone(),
                        jfet_charge_state_voltage(&spec, node_voltages),
                    );
                }
            }
            Element::Mosfet(mosfet) => {
                for spec in mosfet_charge_state_specs(mosfet) {
                    voltages.insert(
                        spec.name.clone(),
                        mosfet_charge_state_voltage(&spec, node_voltages),
                    );
                }
            }
            _ => {}
        }
    }
    voltages
}

fn transient_lte_estimate(
    circuit: &Circuit,
    current_voltages: &BTreeMap<String, f64>,
    previous_voltages: &BTreeMap<String, f64>,
    previous_previous_voltages: &BTreeMap<String, f64>,
) -> f64 {
    let mut max_lte = 0.0_f64;
    for element in circuit.elements() {
        match element {
            Element::Capacitor(capacitor) => {
                let current = current_voltages
                    .get(&capacitor.name)
                    .copied()
                    .unwrap_or(capacitor.initial_voltage);
                let previous = previous_voltages
                    .get(&capacitor.name)
                    .copied()
                    .unwrap_or(capacitor.initial_voltage);
                let previous_previous = previous_previous_voltages
                    .get(&capacitor.name)
                    .copied()
                    .unwrap_or(capacitor.initial_voltage);
                max_lte = max_lte.max((current - 2.0 * previous + previous_previous).abs() / 2.0);
            }
            Element::Diode(diode) if diode_has_charge_storage(diode) => {
                let state_name = diode_charge_state_name(diode);
                let current = current_voltages.get(&state_name).copied().unwrap_or(0.0);
                let previous = previous_voltages.get(&state_name).copied().unwrap_or(0.0);
                let previous_previous = previous_previous_voltages
                    .get(&state_name)
                    .copied()
                    .unwrap_or(0.0);
                max_lte = max_lte.max((current - 2.0 * previous + previous_previous).abs() / 2.0);
            }
            Element::Bjt(bjt) => {
                for spec in bjt_charge_state_specs(bjt) {
                    let current = current_voltages.get(&spec.name).copied().unwrap_or(0.0);
                    let previous = previous_voltages.get(&spec.name).copied().unwrap_or(0.0);
                    let previous_previous = previous_previous_voltages
                        .get(&spec.name)
                        .copied()
                        .unwrap_or(0.0);
                    max_lte =
                        max_lte.max((current - 2.0 * previous + previous_previous).abs() / 2.0);
                }
            }
            Element::Jfet(jfet) => {
                for spec in jfet_charge_state_specs(jfet) {
                    let current = current_voltages.get(&spec.name).copied().unwrap_or(0.0);
                    let previous = previous_voltages.get(&spec.name).copied().unwrap_or(0.0);
                    let previous_previous = previous_previous_voltages
                        .get(&spec.name)
                        .copied()
                        .unwrap_or(0.0);
                    max_lte =
                        max_lte.max((current - 2.0 * previous + previous_previous).abs() / 2.0);
                }
            }
            Element::Mosfet(mosfet) => {
                for spec in mosfet_charge_state_specs(mosfet) {
                    let current = current_voltages.get(&spec.name).copied().unwrap_or(0.0);
                    let previous = previous_voltages.get(&spec.name).copied().unwrap_or(0.0);
                    let previous_previous = previous_previous_voltages
                        .get(&spec.name)
                        .copied()
                        .unwrap_or(0.0);
                    max_lte =
                        max_lte.max((current - 2.0 * previous + previous_previous).abs() / 2.0);
                }
            }
            _ => {}
        }
    }
    max_lte
}

fn initial_transmission_line_states(circuit: &Circuit) -> Vec<TransmissionLineState> {
    circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::TransmissionLine(line) => Some(TransmissionLineState {
                name: line.name.clone(),
                samples: Vec::new(),
            }),
            _ => None,
        })
        .collect()
}

fn transmission_line_state_at(
    state: Option<&TransmissionLineState>,
    target_time: f64,
) -> TransmissionLineSample {
    let Some(state) = state else {
        return TransmissionLineSample {
            time: target_time,
            port1_voltage: 0.0,
            port1_current: 0.0,
            port2_voltage: 0.0,
            port2_current: 0.0,
        };
    };
    if state.samples.is_empty() || target_time < state.samples[0].time - 1.0e-18 {
        return TransmissionLineSample {
            time: target_time,
            port1_voltage: 0.0,
            port1_current: 0.0,
            port2_voltage: 0.0,
            port2_current: 0.0,
        };
    }
    if target_time <= state.samples[0].time {
        return state.samples[0].clone();
    }
    for window in state.samples.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if target_time <= right.time {
            let span = right.time - left.time;
            if span <= 0.0 {
                return right.clone();
            }
            let alpha = (target_time - left.time) / span;
            return TransmissionLineSample {
                time: target_time,
                port1_voltage: left.port1_voltage
                    + alpha * (right.port1_voltage - left.port1_voltage),
                port1_current: left.port1_current
                    + alpha * (right.port1_current - left.port1_current),
                port2_voltage: left.port2_voltage
                    + alpha * (right.port2_voltage - left.port2_voltage),
                port2_current: left.port2_current
                    + alpha * (right.port2_current - left.port2_current),
            };
        }
    }
    state.samples[state.samples.len() - 1].clone()
}

fn transmission_line_history_terms(
    line: &TransmissionLine,
    line_states: &[TransmissionLineState],
    time: f64,
) -> Result<(f64, f64), SpiceError> {
    validate_transmission_line(line)?;
    let delayed = transmission_line_state_at(
        line_states.iter().find(|state| state.name == line.name),
        time - line.delay_seconds,
    );
    Ok((
        delayed.port2_voltage / line.characteristic_impedance_ohms + delayed.port2_current,
        delayed.port1_voltage / line.characteristic_impedance_ohms + delayed.port1_current,
    ))
}

fn circuit_with_transmission_line_companions(
    circuit: &Circuit,
    line_states: &[TransmissionLineState],
    time: f64,
) -> Result<Circuit, SpiceError> {
    let mut companion = Circuit::new();
    for element in circuit.elements() {
        match element {
            Element::TransmissionLine(line) => {
                let (history1, history2) =
                    transmission_line_history_terms(line, line_states, time)?;
                companion.add(Element::Resistor(Resistor::new(
                    format!("_T_{}_P1_R", line.name),
                    line.n1.clone(),
                    line.n2.clone(),
                    line.characteristic_impedance_ohms,
                )));
                companion.add(Element::Resistor(Resistor::new(
                    format!("_T_{}_P2_R", line.name),
                    line.n3.clone(),
                    line.n4.clone(),
                    line.characteristic_impedance_ohms,
                )));
                companion.add(Element::CurrentSource(CurrentSource::new(
                    format!("_T_{}_P1_I", line.name),
                    line.n1.clone(),
                    line.n2.clone(),
                    -history1,
                )));
                companion.add(Element::CurrentSource(CurrentSource::new(
                    format!("_T_{}_P2_I", line.name),
                    line.n3.clone(),
                    line.n4.clone(),
                    -history2,
                )));
            }
            _ => companion.add(element.clone()),
        }
    }
    Ok(companion)
}

fn transmission_line_port_voltage(
    line: &TransmissionLine,
    node_voltages: &BTreeMap<String, f64>,
    first_port: bool,
) -> f64 {
    if first_port {
        return voltage_at(node_voltages, &line.n1) - voltage_at(node_voltages, &line.n2);
    }
    voltage_at(node_voltages, &line.n3) - voltage_at(node_voltages, &line.n4)
}

fn update_transmission_line_states(
    circuit: &Circuit,
    node_voltages: &BTreeMap<String, f64>,
    line_states: &mut [TransmissionLineState],
    time: f64,
) -> Result<BTreeMap<String, f64>, SpiceError> {
    let mut currents = BTreeMap::new();
    for element in circuit.elements() {
        let Element::TransmissionLine(line) = element else {
            continue;
        };
        let (history1, history2) = transmission_line_history_terms(line, line_states, time)?;
        let port1_voltage = transmission_line_port_voltage(line, node_voltages, true);
        let port2_voltage = transmission_line_port_voltage(line, node_voltages, false);
        let port1_current = port1_voltage / line.characteristic_impedance_ohms - history1;
        let port2_current = port2_voltage / line.characteristic_impedance_ohms - history2;
        currents.insert(format!("I({}:1)", line.name), port1_current);
        currents.insert(format!("I({}:2)", line.name), port2_current);
        if let Some(state) = line_states.iter_mut().find(|state| state.name == line.name) {
            state.samples.push(TransmissionLineSample {
                time,
                port1_voltage,
                port1_current,
                port2_voltage,
                port2_current,
            });
        }
    }
    Ok(currents)
}

fn update_capacitor_states(
    circuit: &Circuit,
    node_voltages: &BTreeMap<String, f64>,
    capacitor_states: &mut [CapacitorState],
) {
    let previous_voltages: HashMap<String, f64> = capacitor_states
        .iter()
        .map(|state| (state.name.clone(), state.previous_voltage))
        .collect();
    for state in capacitor_states.iter_mut() {
        let update = circuit.elements().iter().find_map(|element| match element {
            Element::Capacitor(capacitor) if capacitor.name == state.name => Some((
                voltage_at(node_voltages, &capacitor.n1) - voltage_at(node_voltages, &capacitor.n2),
                capacitor.capacitance_farads,
            )),
            Element::Diode(diode) if diode_charge_state_name(diode) == state.name => Some((
                diode_charge_voltage(diode, node_voltages),
                diode_dynamic_capacitance(diode, state.previous_voltage),
            )),
            Element::Bjt(bjt) => bjt_charge_state_specs(bjt)
                .into_iter()
                .find(|spec| spec.name == state.name)
                .map(|spec| {
                    let reverse_junction_voltage = previous_voltages
                        .get(&bjt_base_collector_charge_state_name(bjt))
                        .copied()
                        .unwrap_or(0.0);
                    (
                        bjt_charge_state_voltage(&spec, node_voltages),
                        bjt_charge_dynamic_capacitance(
                            bjt,
                            spec.kind,
                            state.previous_voltage,
                            reverse_junction_voltage,
                        ),
                    )
                }),
            Element::Jfet(jfet) => jfet_charge_state_specs(jfet)
                .into_iter()
                .find(|spec| spec.name == state.name)
                .map(|spec| {
                    (
                        jfet_charge_state_voltage(&spec, node_voltages),
                        jfet_charge_dynamic_capacitance(
                            jfet,
                            spec.capacitance,
                            state.previous_voltage,
                        ),
                    )
                }),
            Element::Mosfet(mosfet) => mosfet_charge_state_specs(mosfet)
                .into_iter()
                .find(|spec| spec.name == state.name)
                .map(|spec| {
                    let capacitance =
                        mosfet_charge_dynamic_capacitance(mosfet, &spec, state.previous_voltage);
                    (
                        mosfet_charge_state_voltage(&spec, node_voltages),
                        capacitance,
                    )
                }),
            _ => None,
        });
        let Some((voltage, capacitance)) = update else {
            continue;
        };
        let previous_voltage = state.previous_voltage;
        let previous_current = state.previous_current;
        state.previous_current = match state.method {
            TransientMethod::Trap => {
                let conductance = 2.0 * capacitance / state.time_step;
                conductance * (voltage - previous_voltage) - previous_current
            }
            TransientMethod::Gear2 => {
                capacitance
                    * (3.0 * voltage - 4.0 * previous_voltage + state.previous_previous_voltage)
                    / (2.0 * state.time_step)
            }
            TransientMethod::Euler => capacitance * (voltage - previous_voltage) / state.time_step,
        };
        state.previous_voltage = voltage;
        state.previous_previous_voltage = previous_voltage;
    }
}

fn seed_device_capacitor_states(
    circuit: &Circuit,
    node_voltages: &BTreeMap<String, f64>,
    capacitor_states: &mut [CapacitorState],
) {
    for element in circuit.elements() {
        match element {
            Element::Diode(diode) => {
                let state_name = diode_charge_state_name(diode);
                if let Some(state) = capacitor_states
                    .iter_mut()
                    .find(|state| state.name == state_name)
                {
                    let voltage = diode_charge_voltage(diode, node_voltages);
                    state.previous_voltage = voltage;
                    state.previous_previous_voltage = voltage;
                    state.previous_current = 0.0;
                }
            }
            Element::Bjt(bjt) => {
                for spec in bjt_charge_state_specs(bjt) {
                    if let Some(state) = capacitor_states
                        .iter_mut()
                        .find(|state| state.name == spec.name)
                    {
                        let voltage = bjt_charge_state_voltage(&spec, node_voltages);
                        state.previous_voltage = voltage;
                        state.previous_previous_voltage = voltage;
                        state.previous_current = 0.0;
                    }
                }
            }
            Element::Jfet(jfet) => {
                for spec in jfet_charge_state_specs(jfet) {
                    if let Some(state) = capacitor_states
                        .iter_mut()
                        .find(|state| state.name == spec.name)
                    {
                        let voltage = jfet_charge_state_voltage(&spec, node_voltages);
                        state.previous_voltage = voltage;
                        state.previous_previous_voltage = voltage;
                        state.previous_current = 0.0;
                    }
                }
            }
            Element::Mosfet(mosfet) => {
                for spec in mosfet_charge_state_specs(mosfet) {
                    if let Some(state) = capacitor_states
                        .iter_mut()
                        .find(|state| state.name == spec.name)
                    {
                        let voltage = mosfet_charge_state_voltage(&spec, node_voltages);
                        state.previous_voltage = voltage;
                        state.previous_previous_voltage = voltage;
                        state.previous_current = 0.0;
                    }
                }
            }
            _ => {}
        }
    }
}

fn update_inductor_states(
    circuit: &Circuit,
    node_voltages: &BTreeMap<String, f64>,
    inductor_states: &mut [InductorState],
) {
    let inductors = inductor_by_name(circuit);
    let coupled_currents =
        coupled_transient_inductor_currents(circuit, &inductors, inductor_states, node_voltages)
            .unwrap_or_default();
    for state in inductor_states {
        let Some(inductor) = circuit.elements().iter().find_map(|element| match element {
            Element::Inductor(inductor) if inductor.name == state.name => Some(inductor),
            _ => None,
        }) else {
            continue;
        };
        let previous_current = state.previous_current;
        let voltage =
            voltage_at(node_voltages, &inductor.n1) - voltage_at(node_voltages, &inductor.n2);
        state.previous_current = coupled_currents
            .get(&inductor.name)
            .copied()
            .unwrap_or_else(|| inductor_current(inductor, state, node_voltages));
        state.previous_previous_current = previous_current;
        state.previous_voltage = voltage;
    }
}

fn insert_transient_inductor_currents(
    circuit: &Circuit,
    inductor_states: &[InductorState],
    node_voltages: &BTreeMap<String, f64>,
    branch_currents: &mut BTreeMap<String, f64>,
) {
    let inductors = inductor_by_name(circuit);
    let coupled_currents =
        coupled_transient_inductor_currents(circuit, &inductors, inductor_states, node_voltages)
            .unwrap_or_default();
    for state in inductor_states {
        let Some(inductor) = circuit.elements().iter().find_map(|element| match element {
            Element::Inductor(inductor) if inductor.name == state.name => Some(inductor),
            _ => None,
        }) else {
            continue;
        };
        branch_currents.insert(
            format!("I({})", inductor.name),
            coupled_currents
                .get(&inductor.name)
                .copied()
                .unwrap_or_else(|| inductor_current(inductor, state, node_voltages)),
        );
    }
}

fn inductor_current(
    inductor: &Inductor,
    state: &InductorState,
    node_voltages: &BTreeMap<String, f64>,
) -> f64 {
    let voltage = voltage_at(node_voltages, &inductor.n1) - voltage_at(node_voltages, &inductor.n2);
    match state.method {
        TransientMethod::Trap => {
            let conductance = state.time_step / (2.0 * inductor.inductance_henrys);
            state.previous_current + conductance * state.previous_voltage + conductance * voltage
        }
        TransientMethod::Gear2 => {
            2.0 * state.time_step * voltage / (3.0 * inductor.inductance_henrys)
                + (4.0 * state.previous_current - state.previous_previous_current) / 3.0
        }
        TransientMethod::Euler => {
            state.previous_current + (state.time_step / inductor.inductance_henrys) * voltage
        }
    }
}

fn stamp_transient_mutual_inductor(
    mutual: &MutualInductor,
    inductors: &HashMap<String, &Inductor>,
    inductor_states: &[InductorState],
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    let (primary, secondary, mutual_inductance) = validate_mutual_inductor(mutual, inductors)?;
    let Some(primary_state) = inductor_states
        .iter()
        .find(|state| state.name == primary.name)
    else {
        return Ok(());
    };
    let Some(secondary_state) = inductor_states
        .iter()
        .find(|state| state.name == secondary.name)
    else {
        return Ok(());
    };
    let (g11, g12, g22) = transient_mutual_conductances(
        mutual,
        primary,
        secondary,
        mutual_inductance,
        primary_state.time_step,
        primary_state.method,
    )?;
    let p1 = node_index(node_indices, &primary.n1);
    let p2 = node_index(node_indices, &primary.n2);
    let s1 = node_index(node_indices, &secondary.n1);
    let s2 = node_index(node_indices, &secondary.n2);
    stamp_conductance(matrix, p1, p2, g11);
    stamp_conductance(matrix, s1, s2, g22);
    stamp_transconductance(matrix, p1, p2, s1, s2, g12);
    stamp_transconductance(matrix, s1, s2, p1, p2, g12);
    let mut primary_history_current = primary_state.previous_current;
    let mut secondary_history_current = secondary_state.previous_current;
    if primary_state.method == TransientMethod::Trap {
        primary_history_current +=
            g11 * primary_state.previous_voltage + g12 * secondary_state.previous_voltage;
        secondary_history_current +=
            g12 * primary_state.previous_voltage + g22 * secondary_state.previous_voltage;
    }
    stamp_equivalent_current_source(rhs, p1, p2, primary_history_current);
    stamp_equivalent_current_source(rhs, s1, s2, secondary_history_current);
    Ok(())
}

fn coupled_transient_inductor_currents(
    circuit: &Circuit,
    inductors: &HashMap<String, &Inductor>,
    inductor_states: &[InductorState],
    node_voltages: &BTreeMap<String, f64>,
) -> Result<HashMap<String, f64>, SpiceError> {
    let mut currents = HashMap::new();
    for element in circuit.elements() {
        let Element::MutualInductor(mutual) = element else {
            continue;
        };
        let (primary, secondary, mutual_inductance) = validate_mutual_inductor(mutual, inductors)?;
        let Some(primary_state) = inductor_states
            .iter()
            .find(|state| state.name == primary.name)
        else {
            continue;
        };
        let Some(secondary_state) = inductor_states
            .iter()
            .find(|state| state.name == secondary.name)
        else {
            continue;
        };
        let (g11, g12, g22) = transient_mutual_conductances(
            mutual,
            primary,
            secondary,
            mutual_inductance,
            primary_state.time_step,
            primary_state.method,
        )?;
        let primary_voltage =
            voltage_at(node_voltages, &primary.n1) - voltage_at(node_voltages, &primary.n2);
        let secondary_voltage =
            voltage_at(node_voltages, &secondary.n1) - voltage_at(node_voltages, &secondary.n2);
        let mut primary_history_current = primary_state.previous_current;
        let mut secondary_history_current = secondary_state.previous_current;
        if primary_state.method == TransientMethod::Trap {
            primary_history_current +=
                g11 * primary_state.previous_voltage + g12 * secondary_state.previous_voltage;
            secondary_history_current +=
                g12 * primary_state.previous_voltage + g22 * secondary_state.previous_voltage;
        }
        currents.insert(
            primary.name.clone(),
            primary_history_current + g11 * primary_voltage + g12 * secondary_voltage,
        );
        currents.insert(
            secondary.name.clone(),
            secondary_history_current + g12 * primary_voltage + g22 * secondary_voltage,
        );
    }
    Ok(currents)
}

fn transient_mutual_conductances(
    mutual: &MutualInductor,
    primary: &Inductor,
    secondary: &Inductor,
    mutual_inductance: f64,
    time_step: f64,
    method: TransientMethod,
) -> Result<(f64, f64, f64), SpiceError> {
    let determinant =
        primary.inductance_henrys * secondary.inductance_henrys - mutual_inductance.powi(2);
    if !determinant.is_finite() || determinant <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mutual.name.clone(),
            reason: "coupled inductance matrix is singular".to_string(),
        });
    }
    let scale = if method == TransientMethod::Trap {
        time_step / (2.0 * determinant)
    } else {
        time_step / determinant
    };
    Ok((
        secondary.inductance_henrys * scale,
        -mutual_inductance * scale,
        primary.inductance_henrys * scale,
    ))
}

fn voltage_at(node_voltages: &BTreeMap<String, f64>, node: &str) -> f64 {
    if is_ground(node) {
        0.0
    } else {
        node_voltages.get(node).copied().unwrap_or(0.0)
    }
}

fn stamp_voltage_source(
    source: &VoltageSource,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    source_time: Option<f64>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    let voltage = source_voltage_at(source, source_time)?;
    if !voltage.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage must be finite".to_string(),
        });
    }

    let branch = node_count + voltage_sources[&source.name];
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);

    stamp_branch_matrix(matrix, branch, positive, negative);
    rhs[branch] += voltage;
    Ok(())
}

fn stamp_branch_matrix(
    matrix: &mut [Vec<f64>],
    branch: usize,
    positive: Option<usize>,
    negative: Option<usize>,
) {
    if let Some(i) = positive {
        matrix[i][branch] += 1.0;
        matrix[branch][i] += 1.0;
    }
    if let Some(j) = negative {
        matrix[j][branch] -= 1.0;
        matrix[branch][j] -= 1.0;
    }
}

fn stamp_current_source(
    source: &CurrentSource,
    node_indices: &HashMap<String, usize>,
    source_time: Option<f64>,
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    let current = source_current_at(source, source_time)?;
    if !current.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "current must be finite".to_string(),
        });
    }

    if let Some(i) = node_index(node_indices, &source.positive) {
        rhs[i] -= current;
    }
    if let Some(j) = node_index(node_indices, &source.negative) {
        rhs[j] += current;
    }
    Ok(())
}

fn source_voltage_at(source: &VoltageSource, source_time: Option<f64>) -> Result<f64, SpiceError> {
    if let (Some(time), Some(waveform)) = (source_time, &source.waveform) {
        waveform
            .validate()
            .map_err(|reason| SpiceError::InvalidElement {
                name: source.name.clone(),
                reason,
            })?;
        let value = waveform.value_at(time);
        if !value.is_finite() {
            return Err(SpiceError::InvalidElement {
                name: source.name.clone(),
                reason: "waveform produced a non-finite voltage".to_string(),
            });
        }
        Ok(value)
    } else {
        Ok(source.voltage)
    }
}

fn source_current_at(source: &CurrentSource, source_time: Option<f64>) -> Result<f64, SpiceError> {
    if let (Some(time), Some(waveform)) = (source_time, &source.waveform) {
        waveform
            .validate()
            .map_err(|reason| SpiceError::InvalidElement {
                name: source.name.clone(),
                reason,
            })?;
        let value = waveform.value_at(time);
        if !value.is_finite() {
            return Err(SpiceError::InvalidElement {
                name: source.name.clone(),
                reason: "waveform produced a non-finite current".to_string(),
            });
        }
        Ok(value)
    } else {
        Ok(source.current)
    }
}

struct BSourceExprParser<'a, F: Fn(&str) -> f64> {
    input: &'a str,
    position: usize,
    resolver: F,
}

impl<'a, F: Fn(&str) -> f64> BSourceExprParser<'a, F> {
    fn new(input: &'a str, resolver: F) -> Self {
        Self {
            input,
            position: 0,
            resolver,
        }
    }

    fn parse(mut self) -> Result<f64, String> {
        let value = self.parse_expression()?;
        self.skip_whitespace();
        if self.position != self.input.len() {
            return Err("unexpected expression input".to_string());
        }
        Ok(value)
    }

    fn parse_expression(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            if self.consume("+") {
                value += self.parse_term()?;
            } else if self.consume("-") {
                value -= self.parse_term()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            if self.consume("*") {
                value *= self.parse_factor()?;
            } else if self.consume("/") {
                value /= self.parse_factor()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        if self.consume("+") {
            return self.parse_factor();
        }
        if self.consume("-") {
            return Ok(-self.parse_factor()?);
        }
        if self.consume("(") {
            let value = self.parse_expression()?;
            self.expect(")")?;
            return Ok(value);
        }
        if self.peek("V") {
            self.position += 1;
            self.expect("(")?;
            let first = self.parse_node_name()?;
            self.skip_whitespace();
            if self.consume(",") {
                let second = self.parse_node_name()?;
                self.expect(")")?;
                return Ok((self.resolver)(&first) - (self.resolver)(&second));
            }
            self.expect(")")?;
            return Ok((self.resolver)(&first));
        }
        self.parse_number()
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let start = self.position;
        while self
            .input
            .as_bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_digit)
        {
            self.position += 1;
        }
        if self.peek(".") {
            self.position += 1;
            while self
                .input
                .as_bytes()
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
        }
        if self.peek("e") || self.peek("E") {
            self.position += 1;
            if self.peek("+") || self.peek("-") {
                self.position += 1;
            }
            while self
                .input
                .as_bytes()
                .get(self.position)
                .is_some_and(u8::is_ascii_digit)
            {
                self.position += 1;
            }
        }
        if self.position == start {
            return Err("expected number".to_string());
        }
        let value = self.input[start..self.position]
            .parse::<f64>()
            .map_err(|_| "invalid number".to_string())?;
        if !value.is_finite() {
            return Err("number must be finite".to_string());
        }
        Ok(value)
    }

    fn parse_node_name(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        let start = self.position;
        while let Some(ch) = self.input[self.position..].chars().next() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '$' | ':' | '-') {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }
        if self.position == start {
            return Err("expected node name".to_string());
        }
        self.skip_whitespace();
        Ok(self.input[start..self.position].to_string())
    }

    fn consume(&mut self, token: &str) -> bool {
        self.skip_whitespace();
        if self.input[self.position..].starts_with(token) {
            self.position += token.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        if self.consume(token) {
            Ok(())
        } else {
            Err(format!("expected {token}"))
        }
    }

    fn peek(&self, token: &str) -> bool {
        self.input[self.position..].starts_with(token)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.input[self.position..].chars().next() {
            if ch.is_whitespace() {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }
    }
}

fn bsource_expr_nodes(expr: &str) -> Vec<String> {
    let mut nodes = Vec::new();
    let mut rest = expr;
    while let Some(start) = rest.find("V(") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else {
            break;
        };
        for node in rest[..end].split(',') {
            let trimmed = node.trim();
            if !trimmed.is_empty() && !is_ground(trimmed) {
                nodes.push(trimmed.to_string());
            }
        }
        rest = &rest[end + 1..];
    }
    nodes
}

fn eval_bsource_expr(
    expr: &str,
    node_indices: &HashMap<String, usize>,
    operating_point: &[f64],
) -> Result<f64, String> {
    let value = BSourceExprParser::new(expr, |node: &str| {
        node_index(node_indices, node).map_or(0.0, |index| operating_point[index])
    })
    .parse()?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("expression produced a non-finite value".to_string())
    }
}

fn bsource_linearization(
    expr: &str,
    node_indices: &HashMap<String, usize>,
    operating_point: &[f64],
) -> Result<(BTreeMap<String, f64>, f64), String> {
    let value = eval_bsource_expr(expr, node_indices, operating_point)?;
    let mut derivatives = BTreeMap::new();
    for (node, index) in node_indices {
        let h = (operating_point[*index].abs() * 1.0e-6).max(1.0e-6);
        let mut plus = operating_point.to_vec();
        let mut minus = operating_point.to_vec();
        plus[*index] += h;
        minus[*index] -= h;
        let derivative = (eval_bsource_expr(expr, node_indices, &plus)?
            - eval_bsource_expr(expr, node_indices, &minus)?)
            / (2.0 * h);
        derivatives.insert(node.clone(), derivative);
    }
    let linear_part = derivatives
        .iter()
        .map(|(node, derivative)| derivative * operating_point[node_indices[node]])
        .sum::<f64>();
    Ok((derivatives, value - linear_part))
}

fn validate_bsource(source: &BSource) -> Result<(), SpiceError> {
    if source.voltage_expr.is_some() == source.current_expr.is_some() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "B-source must define exactly one voltage_expr or current_expr".to_string(),
        });
    }
    Ok(())
}

fn stamp_bsource(
    source: &BSource,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_bsource(source)?;
    if let Some(expr) = &source.current_expr {
        let (derivatives, offset) = bsource_linearization(expr, node_indices, operating_point)
            .map_err(|reason| SpiceError::InvalidElement {
                name: source.name.clone(),
                reason,
            })?;
        let positive = node_index(node_indices, &source.positive);
        let negative = node_index(node_indices, &source.negative);
        if let Some(row) = positive {
            for (node, derivative) in &derivatives {
                matrix[row][node_indices[node]] += derivative;
            }
            rhs[row] -= offset;
        }
        if let Some(row) = negative {
            for (node, derivative) in &derivatives {
                matrix[row][node_indices[node]] -= derivative;
            }
            rhs[row] += offset;
        }
        return Ok(());
    }
    let Some(expr) = &source.voltage_expr else {
        return Ok(());
    };
    let Some(source_index) = voltage_sources.get(&source.name) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage B-source was not indexed".to_string(),
        });
    };
    let branch = node_count + source_index;
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_branch_matrix(matrix, branch, positive, negative);
    let (derivatives, offset) = bsource_linearization(expr, node_indices, operating_point)
        .map_err(|reason| SpiceError::InvalidElement {
            name: source.name.clone(),
            reason,
        })?;
    for (node, derivative) in derivatives {
        matrix[branch][node_indices[&node]] -= derivative;
    }
    rhs[branch] += offset;
    Ok(())
}

fn stamp_bsource_small_signal(
    source: &BSource,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<f64>],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_bsource(source)?;
    if let Some(expr) = &source.current_expr {
        let (derivatives, _) =
            bsource_linearization(expr, node_indices, operating_point).map_err(|reason| {
                SpiceError::InvalidElement {
                    name: source.name.clone(),
                    reason,
                }
            })?;
        let positive = node_index(node_indices, &source.positive);
        let negative = node_index(node_indices, &source.negative);
        for (node, derivative) in derivatives {
            let control = Some(node_indices[&node]);
            stamp_transconductance(matrix, positive, negative, control, None, derivative);
        }
        return Ok(());
    }

    let Some(expr) = &source.voltage_expr else {
        return Ok(());
    };
    let Some(source_index) = voltage_sources.get(&source.name) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage B-source was not indexed".to_string(),
        });
    };
    let branch = node_count + source_index;
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_branch_matrix(matrix, branch, positive, negative);
    let (derivatives, offset) = bsource_linearization(expr, node_indices, operating_point)
        .map_err(|reason| SpiceError::InvalidElement {
            name: source.name.clone(),
            reason,
        })?;
    for (node, derivative) in derivatives {
        matrix[branch][node_indices[&node]] -= derivative;
    }
    // DC callers add this offset to the RHS after this helper returns.
    let _ = offset;
    Ok(())
}

fn stamp_ac_bsource(
    source: &BSource,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<Complex>],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_bsource(source)?;
    if let Some(expr) = &source.current_expr {
        let (derivatives, _) =
            bsource_linearization(expr, node_indices, operating_point).map_err(|reason| {
                SpiceError::InvalidElement {
                    name: source.name.clone(),
                    reason,
                }
            })?;
        let positive = node_index(node_indices, &source.positive);
        let negative = node_index(node_indices, &source.negative);
        for (node, derivative) in derivatives {
            let control = Some(node_indices[&node]);
            stamp_complex_transconductance(
                matrix,
                positive,
                negative,
                control,
                None,
                Complex::new(derivative, 0.0),
            );
        }
        return Ok(());
    }

    let Some(expr) = &source.voltage_expr else {
        return Ok(());
    };
    let Some(source_index) = voltage_sources.get(&source.name) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage B-source was not indexed".to_string(),
        });
    };
    let branch = node_count + source_index;
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_complex_branch_matrix(matrix, branch, positive, negative);
    let (derivatives, _) =
        bsource_linearization(expr, node_indices, operating_point).map_err(|reason| {
            SpiceError::InvalidElement {
                name: source.name.clone(),
                reason,
            }
        })?;
    for (node, derivative) in derivatives {
        matrix[branch][node_indices[&node]] -= Complex::new(derivative, 0.0);
    }
    Ok(())
}

fn stamp_vccs(
    source: &Vccs,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
) -> Result<(), SpiceError> {
    if !source.transconductance_siemens.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "transconductance must be finite".to_string(),
        });
    }

    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    let control_positive = node_index(node_indices, &source.control_positive);
    let control_negative = node_index(node_indices, &source.control_negative);
    stamp_transconductance(
        matrix,
        positive,
        negative,
        control_positive,
        control_negative,
        source.transconductance_siemens,
    );
    Ok(())
}

fn stamp_vcvs(
    source: &Vcvs,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<f64>],
) -> Result<(), SpiceError> {
    if !source.gain.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "gain must be finite".to_string(),
        });
    }

    let Some(source_index) = voltage_sources.get(&source.name) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage source was not indexed".to_string(),
        });
    };

    let branch = node_count + source_index;
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    let control_positive = node_index(node_indices, &source.control_positive);
    let control_negative = node_index(node_indices, &source.control_negative);
    stamp_branch_matrix(matrix, branch, positive, negative);
    stamp_controlled_voltage_row(
        matrix,
        branch,
        control_positive,
        control_negative,
        source.gain,
    );
    Ok(())
}

fn stamp_cccs(
    source: &Cccs,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    matrix: &mut [Vec<f64>],
) -> Result<(), SpiceError> {
    if !source.gain.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "gain must be finite".to_string(),
        });
    }

    let Some(source_index) = voltage_sources.get(&source.control_source) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "control source was not indexed".to_string(),
        });
    };

    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_current_controlled_current(
        matrix,
        positive,
        negative,
        node_indices.len() + source_index,
        source.gain,
    );
    Ok(())
}

fn stamp_ccvs(
    source: &Ccvs,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<f64>],
) -> Result<(), SpiceError> {
    if !source.transresistance_ohms.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "transresistance must be finite".to_string(),
        });
    }

    let Some(source_index) = voltage_sources.get(&source.name) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage source was not indexed".to_string(),
        });
    };
    let Some(control_index) = voltage_sources.get(&source.control_source) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "control source was not indexed".to_string(),
        });
    };

    let branch = node_count + source_index;
    let control_branch = node_count + control_index;
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_branch_matrix(matrix, branch, positive, negative);
    matrix[branch][control_branch] -= source.transresistance_ohms;
    Ok(())
}

fn stamp_current_controlled_current(
    matrix: &mut [Vec<f64>],
    positive: Option<usize>,
    negative: Option<usize>,
    control_branch: usize,
    gain: f64,
) {
    if let Some(i) = positive {
        matrix[i][control_branch] += gain;
    }
    if let Some(j) = negative {
        matrix[j][control_branch] -= gain;
    }
}

fn stamp_controlled_voltage_row(
    matrix: &mut [Vec<f64>],
    branch: usize,
    control_positive: Option<usize>,
    control_negative: Option<usize>,
    gain: f64,
) {
    if let Some(cp) = control_positive {
        matrix[branch][cp] -= gain;
    }
    if let Some(cn) = control_negative {
        matrix[branch][cn] += gain;
    }
}

fn stamp_transconductance(
    matrix: &mut [Vec<f64>],
    positive: Option<usize>,
    negative: Option<usize>,
    control_positive: Option<usize>,
    control_negative: Option<usize>,
    transconductance: f64,
) {
    if let Some(i) = positive {
        if let Some(cp) = control_positive {
            matrix[i][cp] += transconductance;
        }
        if let Some(cn) = control_negative {
            matrix[i][cn] -= transconductance;
        }
    }
    if let Some(j) = negative {
        if let Some(cp) = control_positive {
            matrix[j][cp] -= transconductance;
        }
        if let Some(cn) = control_negative {
            matrix[j][cn] += transconductance;
        }
    }
}

fn stamp_ac_resistor(
    resistor: &Resistor,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    if !resistor.resistance_ohms.is_finite() || resistor.resistance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: resistor.name.clone(),
            reason: "resistance must be finite and positive".to_string(),
        });
    }

    let conductance = Complex::new(1.0 / resistor.resistance_ohms, 0.0);
    let n1 = node_index(node_indices, &resistor.n1);
    let n2 = node_index(node_indices, &resistor.n2);
    stamp_complex_conductance(matrix, n1, n2, conductance);
    Ok(())
}

fn stamp_ac_capacitor(
    capacitor: &Capacitor,
    omega: f64,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    validate_capacitor(capacitor)?;
    let admittance = Complex::new(0.0, omega * capacitor.capacitance_farads);
    let n1 = node_index(node_indices, &capacitor.n1);
    let n2 = node_index(node_indices, &capacitor.n2);
    stamp_complex_conductance(matrix, n1, n2, admittance);
    Ok(())
}

fn stamp_ac_inductor(
    inductor: &Inductor,
    omega: f64,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    validate_inductor(inductor)?;
    let admittance = Complex::new(0.0, -1.0 / (omega * inductor.inductance_henrys));
    let n1 = node_index(node_indices, &inductor.n1);
    let n2 = node_index(node_indices, &inductor.n2);
    stamp_complex_conductance(matrix, n1, n2, admittance);
    Ok(())
}

fn inductor_by_name(circuit: &Circuit) -> HashMap<String, &Inductor> {
    circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::Inductor(inductor) => Some((inductor.name.clone(), inductor)),
            _ => None,
        })
        .collect()
}

fn coupled_inductor_names(circuit: &Circuit) -> HashSet<String> {
    let mut names = HashSet::new();
    for element in circuit.elements() {
        if let Element::MutualInductor(mutual) = element {
            names.insert(mutual.primary.clone());
            names.insert(mutual.secondary.clone());
        }
    }
    names
}

fn validate_mutual_inductor<'a>(
    mutual: &MutualInductor,
    inductors: &'a HashMap<String, &Inductor>,
) -> Result<(&'a Inductor, &'a Inductor, f64), SpiceError> {
    if !mutual.coupling.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: mutual.name.clone(),
            reason: "coupling must be finite".to_string(),
        });
    }
    if mutual.coupling.abs() >= 1.0 {
        return Err(SpiceError::InvalidElement {
            name: mutual.name.clone(),
            reason: "coupling magnitude must be less than one".to_string(),
        });
    }
    if mutual.primary == mutual.secondary {
        return Err(SpiceError::InvalidElement {
            name: mutual.name.clone(),
            reason: "coupled inductors must be distinct".to_string(),
        });
    }
    let primary =
        inductors
            .get(&mutual.primary)
            .copied()
            .ok_or_else(|| SpiceError::InvalidElement {
                name: mutual.name.clone(),
                reason: format!("referenced inductor {:?} was not found", mutual.primary),
            })?;
    let secondary =
        inductors
            .get(&mutual.secondary)
            .copied()
            .ok_or_else(|| SpiceError::InvalidElement {
                name: mutual.name.clone(),
                reason: format!("referenced inductor {:?} was not found", mutual.secondary),
            })?;
    validate_inductor(primary)?;
    validate_inductor(secondary)?;
    let mutual_inductance =
        mutual.coupling * (primary.inductance_henrys * secondary.inductance_henrys).sqrt();
    Ok((primary, secondary, mutual_inductance))
}

fn stamp_ac_mutual_inductor(
    mutual: &MutualInductor,
    inductors: &HashMap<String, &Inductor>,
    omega: f64,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    let (primary, secondary, mutual_inductance) = validate_mutual_inductor(mutual, inductors)?;
    if omega == 0.0 {
        stamp_complex_conductance(
            matrix,
            node_index(node_indices, &primary.n1),
            node_index(node_indices, &primary.n2),
            Complex::new(1.0e12, 0.0),
        );
        stamp_complex_conductance(
            matrix,
            node_index(node_indices, &secondary.n1),
            node_index(node_indices, &secondary.n2),
            Complex::new(1.0e12, 0.0),
        );
        return Ok(());
    }

    let determinant =
        primary.inductance_henrys * secondary.inductance_henrys - mutual_inductance.powi(2);
    if !determinant.is_finite() || determinant <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: mutual.name.clone(),
            reason: "coupled inductance matrix is singular".to_string(),
        });
    }

    let scale = Complex::new(0.0, -1.0 / (omega * determinant));
    let y11 = Complex::new(
        scale.real * secondary.inductance_henrys,
        scale.imag * secondary.inductance_henrys,
    );
    let y12 = Complex::new(
        scale.real * -mutual_inductance,
        scale.imag * -mutual_inductance,
    );
    let y22 = Complex::new(
        scale.real * primary.inductance_henrys,
        scale.imag * primary.inductance_henrys,
    );
    let p1 = node_index(node_indices, &primary.n1);
    let p2 = node_index(node_indices, &primary.n2);
    let s1 = node_index(node_indices, &secondary.n1);
    let s2 = node_index(node_indices, &secondary.n2);
    stamp_complex_conductance(matrix, p1, p2, y11);
    stamp_complex_conductance(matrix, s1, s2, y22);
    stamp_complex_transconductance(matrix, p1, p2, s1, s2, y12);
    stamp_complex_transconductance(matrix, s1, s2, p1, p2, y12);
    Ok(())
}

fn stamp_ac_transmission_line(
    line: &TransmissionLine,
    omega: f64,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    if !line.characteristic_impedance_ohms.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "characteristic impedance must be finite".to_string(),
        });
    }
    if line.characteristic_impedance_ohms <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "characteristic impedance must be positive".to_string(),
        });
    }
    if !line.delay_seconds.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "delay must be finite".to_string(),
        });
    }
    if line.delay_seconds <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "delay must be positive".to_string(),
        });
    }
    let phase = omega * line.delay_seconds;
    let sin_phase = phase.sin();
    if sin_phase.abs() < 1.0e-12 {
        return Err(SpiceError::InvalidElement {
            name: line.name.clone(),
            reason: "transmission line phase is singular at this frequency".to_string(),
        });
    }
    let cos_phase = phase.cos();
    let y11 = Complex::new(
        0.0,
        -cos_phase / (line.characteristic_impedance_ohms * sin_phase),
    );
    let y12 = Complex::new(0.0, 1.0 / (line.characteristic_impedance_ohms * sin_phase));
    let n1 = node_index(node_indices, &line.n1);
    let n2 = node_index(node_indices, &line.n2);
    let n3 = node_index(node_indices, &line.n3);
    let n4 = node_index(node_indices, &line.n4);
    stamp_complex_conductance(matrix, n1, n2, y11);
    stamp_complex_conductance(matrix, n3, n4, y11);
    stamp_complex_transconductance(matrix, n1, n2, n3, n4, y12);
    stamp_complex_transconductance(matrix, n3, n4, n1, n2, y12);
    Ok(())
}

fn stamp_complex_conductance(
    matrix: &mut [Vec<Complex>],
    n1: Option<usize>,
    n2: Option<usize>,
    conductance: Complex,
) {
    if let Some(i) = n1 {
        matrix[i][i] += conductance;
    }
    if let Some(j) = n2 {
        matrix[j][j] += conductance;
    }
    if let (Some(i), Some(j)) = (n1, n2) {
        matrix[i][j] -= conductance;
        matrix[j][i] -= conductance;
    }
}

fn stamp_ac_voltage_source(
    source: &VoltageSource,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    uses_explicit_ac_sources: bool,
    rhs: &mut [Complex],
) -> Result<(), SpiceError> {
    validate_ac_voltage_source(source)?;

    let branch = node_count + voltage_sources[&source.name];
    rhs[branch] += voltage_source_ac_phasor(source, uses_explicit_ac_sources);
    Ok(())
}

fn validate_ac_voltage_source(source: &VoltageSource) -> Result<(), SpiceError> {
    if !source.voltage.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage must be finite".to_string(),
        });
    }
    if let Some(ac) = source.ac {
        validate_ac_source(&source.name, ac)?;
    }
    Ok(())
}

fn stamp_ac_voltage_source_matrix(
    source: &VoltageSource,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    validate_ac_voltage_source(source)?;

    let branch = node_count + voltage_sources[&source.name];
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_complex_branch_matrix(matrix, branch, positive, negative);
    Ok(())
}

fn stamp_complex_branch_matrix(
    matrix: &mut [Vec<Complex>],
    branch: usize,
    positive: Option<usize>,
    negative: Option<usize>,
) {
    if let Some(i) = positive {
        matrix[i][branch] += Complex::new(1.0, 0.0);
        matrix[branch][i] += Complex::new(1.0, 0.0);
    }
    if let Some(j) = negative {
        matrix[j][branch] -= Complex::new(1.0, 0.0);
        matrix[branch][j] -= Complex::new(1.0, 0.0);
    }
}

fn stamp_ac_current_source(
    source: &CurrentSource,
    node_indices: &HashMap<String, usize>,
    uses_explicit_ac_sources: bool,
    rhs: &mut [Complex],
) -> Result<(), SpiceError> {
    validate_ac_current_source(source)?;

    let current = current_source_ac_phasor(source, uses_explicit_ac_sources);
    if let Some(i) = node_index(node_indices, &source.positive) {
        rhs[i] -= current;
    }
    if let Some(j) = node_index(node_indices, &source.negative) {
        rhs[j] += current;
    }
    Ok(())
}

fn validate_ac_current_source(source: &CurrentSource) -> Result<(), SpiceError> {
    if !source.current.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "current must be finite".to_string(),
        });
    }
    if let Some(ac) = source.ac {
        validate_ac_source(&source.name, ac)?;
    }
    Ok(())
}

fn validate_ac_source(name: &str, source: AcSource) -> Result<(), SpiceError> {
    if !source.magnitude.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: name.to_string(),
            reason: "AC magnitude must be finite".to_string(),
        });
    }
    if !source.phase_degrees.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: name.to_string(),
            reason: "AC phase must be finite".to_string(),
        });
    }
    Ok(())
}

fn voltage_source_ac_phasor(source: &VoltageSource, uses_explicit_ac_sources: bool) -> Complex {
    match source.ac {
        Some(ac) => ac_source_phasor(ac),
        None if uses_explicit_ac_sources => Complex::zero(),
        None => Complex::new(source.voltage, 0.0),
    }
}

fn current_source_ac_phasor(source: &CurrentSource, uses_explicit_ac_sources: bool) -> Complex {
    match source.ac {
        Some(ac) => ac_source_phasor(ac),
        None if uses_explicit_ac_sources => Complex::zero(),
        None => Complex::new(source.current, 0.0),
    }
}

fn ac_source_phasor(source: AcSource) -> Complex {
    let phase = source.phase_degrees.to_radians();
    Complex::new(
        source.magnitude * phase.cos(),
        source.magnitude * phase.sin(),
    )
}

fn circuit_has_explicit_ac_sources(circuit: &Circuit) -> bool {
    circuit.elements().iter().any(|element| match element {
        Element::VoltageSource(source) => source.ac.is_some(),
        Element::CurrentSource(source) => source.ac.is_some(),
        _ => false,
    })
}

fn stamp_ac_vccs(
    source: &Vccs,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    if !source.transconductance_siemens.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "transconductance must be finite".to_string(),
        });
    }

    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    let control_positive = node_index(node_indices, &source.control_positive);
    let control_negative = node_index(node_indices, &source.control_negative);
    stamp_complex_transconductance(
        matrix,
        positive,
        negative,
        control_positive,
        control_negative,
        Complex::new(source.transconductance_siemens, 0.0),
    );
    Ok(())
}

fn stamp_ac_vcvs(
    source: &Vcvs,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    if !source.gain.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "gain must be finite".to_string(),
        });
    }

    let Some(source_index) = voltage_sources.get(&source.name) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage source was not indexed".to_string(),
        });
    };

    let branch = node_count + source_index;
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    let control_positive = node_index(node_indices, &source.control_positive);
    let control_negative = node_index(node_indices, &source.control_negative);
    stamp_complex_branch_matrix(matrix, branch, positive, negative);
    stamp_complex_controlled_voltage_row(
        matrix,
        branch,
        control_positive,
        control_negative,
        Complex::new(source.gain, 0.0),
    );
    Ok(())
}

fn stamp_ac_cccs(
    source: &Cccs,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    if !source.gain.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "gain must be finite".to_string(),
        });
    }

    let Some(source_index) = voltage_sources.get(&source.control_source) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "control source was not indexed".to_string(),
        });
    };

    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_complex_current_controlled_current(
        matrix,
        positive,
        negative,
        node_indices.len() + source_index,
        Complex::new(source.gain, 0.0),
    );
    Ok(())
}

fn stamp_ac_ccvs(
    source: &Ccvs,
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<Complex>],
) -> Result<(), SpiceError> {
    if !source.transresistance_ohms.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "transresistance must be finite".to_string(),
        });
    }

    let Some(source_index) = voltage_sources.get(&source.name) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage source was not indexed".to_string(),
        });
    };
    let Some(control_index) = voltage_sources.get(&source.control_source) else {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "control source was not indexed".to_string(),
        });
    };

    let branch = node_count + source_index;
    let control_branch = node_count + control_index;
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);
    stamp_complex_branch_matrix(matrix, branch, positive, negative);
    matrix[branch][control_branch] -= Complex::new(source.transresistance_ohms, 0.0);
    Ok(())
}

fn stamp_complex_current_controlled_current(
    matrix: &mut [Vec<Complex>],
    positive: Option<usize>,
    negative: Option<usize>,
    control_branch: usize,
    gain: Complex,
) {
    if let Some(i) = positive {
        matrix[i][control_branch] += gain;
    }
    if let Some(j) = negative {
        matrix[j][control_branch] -= gain;
    }
}

fn stamp_complex_controlled_voltage_row(
    matrix: &mut [Vec<Complex>],
    branch: usize,
    control_positive: Option<usize>,
    control_negative: Option<usize>,
    gain: Complex,
) {
    if let Some(cp) = control_positive {
        matrix[branch][cp] -= gain;
    }
    if let Some(cn) = control_negative {
        matrix[branch][cn] += gain;
    }
}

fn stamp_complex_transconductance(
    matrix: &mut [Vec<Complex>],
    positive: Option<usize>,
    negative: Option<usize>,
    control_positive: Option<usize>,
    control_negative: Option<usize>,
    transconductance: Complex,
) {
    if let Some(i) = positive {
        if let Some(cp) = control_positive {
            matrix[i][cp] += transconductance;
        }
        if let Some(cn) = control_negative {
            matrix[i][cn] -= transconductance;
        }
    }
    if let Some(j) = negative {
        if let Some(cp) = control_positive {
            matrix[j][cp] -= transconductance;
        }
        if let Some(cn) = control_negative {
            matrix[j][cn] += transconductance;
        }
    }
}

fn solve_linear_system(matrix: Vec<Vec<f64>>, rhs: Vec<f64>) -> Result<Vec<f64>, SpiceError> {
    Ok(solve_linear_system_with_profile(matrix, rhs)?.solution)
}

fn solve_linear_system_with_profile(
    matrix: Vec<Vec<f64>>,
    rhs: Vec<f64>,
) -> Result<SolvedLinearSystem, SpiceError> {
    if rhs.len() >= SPARSE_SOLVER_THRESHOLD {
        return solve_sparse_linear_system_with_profile(matrix, rhs);
    }
    let profile = real_solver_profile(&matrix, "dense_gaussian", 0, None);
    Ok(SolvedLinearSystem {
        solution: solve_dense_linear_system(matrix, rhs)?,
        profile,
    })
}

fn solve_dense_linear_system(
    mut matrix: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
) -> Result<Vec<f64>, SpiceError> {
    let n = rhs.len();
    for pivot_col in 0..n {
        let pivot_row = (pivot_col..n)
            .max_by(|&a, &b| {
                matrix[a][pivot_col]
                    .abs()
                    .partial_cmp(&matrix[b][pivot_col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or(SpiceError::SingularMatrix)?;

        if matrix[pivot_row][pivot_col].abs() < PIVOT_EPSILON {
            return Err(SpiceError::SingularMatrix);
        }

        matrix.swap(pivot_col, pivot_row);
        rhs.swap(pivot_col, pivot_row);

        let pivot = matrix[pivot_col][pivot_col];
        for row in (pivot_col + 1)..n {
            let factor = matrix[row][pivot_col] / pivot;
            if factor == 0.0 {
                continue;
            }
            matrix[row][pivot_col] = 0.0;
            // `col` indexes two distinct rows of `matrix` (row and pivot_col); an
            // iterator rewrite would require split_at_mut and obscure the algebra.
            #[allow(clippy::needless_range_loop)]
            for col in (pivot_col + 1)..n {
                matrix[row][col] -= factor * matrix[pivot_col][col];
            }
            rhs[row] -= factor * rhs[pivot_col];
        }
    }

    let mut solution = vec![0.0; n];
    for row in (0..n).rev() {
        let tail_sum: f64 = ((row + 1)..n)
            .map(|col| matrix[row][col] * solution[col])
            .sum();
        solution[row] = (rhs[row] - tail_sum) / matrix[row][row];
    }
    Ok(solution)
}

fn solve_sparse_linear_system_with_profile(
    matrix: Vec<Vec<f64>>,
    rhs: Vec<f64>,
) -> Result<SolvedLinearSystem, SpiceError> {
    let n = rhs.len();
    let initial_nonzeros = real_matrix_nonzeros(&matrix);
    let mut peak_nonzeros = initial_nonzeros;
    let mut profile = real_solver_profile(&matrix, "native_sparse_gaussian", 0, None);
    let mut rows: Vec<HashMap<usize, f64>> = matrix
        .into_iter()
        .map(|row| {
            row.into_iter()
                .enumerate()
                .filter_map(|(col, value)| (value != 0.0).then_some((col, value)))
                .collect()
        })
        .collect();
    let mut rhs = rhs;

    for pivot_col in 0..n {
        let pivot_row = (pivot_col..n)
            .max_by(|&a, &b| {
                rows[a]
                    .get(&pivot_col)
                    .copied()
                    .unwrap_or(0.0)
                    .abs()
                    .partial_cmp(&rows[b].get(&pivot_col).copied().unwrap_or(0.0).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or(SpiceError::SingularMatrix)?;

        if rows[pivot_row]
            .get(&pivot_col)
            .copied()
            .unwrap_or(0.0)
            .abs()
            < PIVOT_EPSILON
        {
            profile.fill_in_nonzeros = peak_nonzeros.saturating_sub(initial_nonzeros);
            return Err(SpiceError::SingularMatrix);
        }

        rows.swap(pivot_col, pivot_row);
        rhs.swap(pivot_col, pivot_row);

        let pivot = rows[pivot_col][&pivot_col];
        let pivot_entries: Vec<(usize, f64)> = rows[pivot_col]
            .iter()
            .filter_map(|(&col, &value)| (col > pivot_col).then_some((col, value)))
            .collect();
        for row in (pivot_col + 1)..n {
            let value = rows[row].get(&pivot_col).copied().unwrap_or(0.0);
            if value == 0.0 {
                continue;
            }
            let factor = value / pivot;
            rows[row].remove(&pivot_col);
            for (col, pivot_value) in &pivot_entries {
                let next_value = rows[row].get(col).copied().unwrap_or(0.0) - factor * pivot_value;
                if next_value.abs() < PIVOT_EPSILON {
                    rows[row].remove(col);
                } else {
                    rows[row].insert(*col, next_value);
                }
            }
            rhs[row] -= factor * rhs[pivot_col];
        }
        peak_nonzeros = peak_nonzeros.max(rows.iter().map(HashMap::len).sum());
    }

    let mut solution = vec![0.0; n];
    for row in (0..n).rev() {
        let diagonal = rows[row].get(&row).copied().unwrap_or(0.0);
        if diagonal.abs() < PIVOT_EPSILON {
            profile.fill_in_nonzeros = peak_nonzeros.saturating_sub(initial_nonzeros);
            return Err(SpiceError::SingularMatrix);
        }
        let mut value = rhs[row];
        for (&col, &entry) in &rows[row] {
            if col > row {
                value -= entry * solution[col];
            }
        }
        solution[row] = value / diagonal;
    }

    profile.fill_in_nonzeros = peak_nonzeros.saturating_sub(initial_nonzeros);
    Ok(SolvedLinearSystem { solution, profile })
}

fn solve_complex_linear_system(
    matrix: Vec<Vec<Complex>>,
    rhs: Vec<Complex>,
) -> Result<Vec<Complex>, SpiceError> {
    if complex_solver_kind(rhs.len()) == "sparse_complex" {
        return solve_sparse_complex_linear_system(matrix, rhs);
    }
    solve_dense_complex_linear_system(matrix, rhs)
}

fn solve_dense_complex_linear_system(
    mut matrix: Vec<Vec<Complex>>,
    mut rhs: Vec<Complex>,
) -> Result<Vec<Complex>, SpiceError> {
    let n = rhs.len();
    for pivot_col in 0..n {
        let pivot_row = (pivot_col..n)
            .max_by(|&a, &b| {
                matrix[a][pivot_col]
                    .abs()
                    .partial_cmp(&matrix[b][pivot_col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or(SpiceError::SingularMatrix)?;

        if matrix[pivot_row][pivot_col].abs() < PIVOT_EPSILON {
            return Err(SpiceError::SingularMatrix);
        }

        matrix.swap(pivot_col, pivot_row);
        rhs.swap(pivot_col, pivot_row);

        let pivot = matrix[pivot_col][pivot_col];
        for row in (pivot_col + 1)..n {
            let factor = matrix[row][pivot_col] / pivot;
            if factor == Complex::zero() {
                continue;
            }
            matrix[row][pivot_col] = Complex::zero();
            // `col` indexes two distinct rows of `matrix` (row and pivot_col); an
            // iterator rewrite would require split_at_mut and obscure the algebra.
            #[allow(clippy::needless_range_loop)]
            for col in (pivot_col + 1)..n {
                matrix[row][col] = matrix[row][col] - factor * matrix[pivot_col][col];
            }
            rhs[row] = rhs[row] - factor * rhs[pivot_col];
        }
    }

    let mut solution = vec![Complex::zero(); n];
    for row in (0..n).rev() {
        let tail_sum = ((row + 1)..n)
            .map(|col| matrix[row][col] * solution[col])
            .fold(Complex::zero(), |acc, value| acc + value);
        solution[row] = (rhs[row] - tail_sum) / matrix[row][row];
        if !solution[row].is_finite() {
            return Err(SpiceError::SingularMatrix);
        }
    }
    Ok(solution)
}

fn solve_sparse_complex_linear_system(
    matrix: Vec<Vec<Complex>>,
    rhs: Vec<Complex>,
) -> Result<Vec<Complex>, SpiceError> {
    let n = rhs.len();
    let mut rows: Vec<HashMap<usize, Complex>> = matrix
        .into_iter()
        .map(|row| {
            row.into_iter()
                .enumerate()
                .filter_map(|(col, value)| (value != Complex::zero()).then_some((col, value)))
                .collect()
        })
        .collect();
    let mut rhs = rhs;

    for pivot_col in 0..n {
        let pivot_row = (pivot_col..n)
            .max_by(|&a, &b| {
                rows[a]
                    .get(&pivot_col)
                    .copied()
                    .unwrap_or_else(Complex::zero)
                    .abs()
                    .partial_cmp(
                        &rows[b]
                            .get(&pivot_col)
                            .copied()
                            .unwrap_or_else(Complex::zero)
                            .abs(),
                    )
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or(SpiceError::SingularMatrix)?;

        if rows[pivot_row]
            .get(&pivot_col)
            .copied()
            .unwrap_or_else(Complex::zero)
            .abs()
            < PIVOT_EPSILON
        {
            return Err(SpiceError::SingularMatrix);
        }

        rows.swap(pivot_col, pivot_row);
        rhs.swap(pivot_col, pivot_row);

        let pivot = rows[pivot_col][&pivot_col];
        let pivot_entries: Vec<(usize, Complex)> = rows[pivot_col]
            .iter()
            .filter_map(|(&col, &value)| (col > pivot_col).then_some((col, value)))
            .collect();
        for row in (pivot_col + 1)..n {
            let value = rows[row]
                .get(&pivot_col)
                .copied()
                .unwrap_or_else(Complex::zero);
            if value == Complex::zero() {
                continue;
            }
            let factor = value / pivot;
            rows[row].remove(&pivot_col);
            for (col, pivot_value) in &pivot_entries {
                let next_value = rows[row].get(col).copied().unwrap_or_else(Complex::zero)
                    - factor * *pivot_value;
                if next_value.abs() < PIVOT_EPSILON {
                    rows[row].remove(col);
                } else {
                    rows[row].insert(*col, next_value);
                }
            }
            rhs[row] = rhs[row] - factor * rhs[pivot_col];
        }
    }

    let mut solution = vec![Complex::zero(); n];
    for row in (0..n).rev() {
        let diagonal = rows[row].get(&row).copied().unwrap_or_else(Complex::zero);
        if diagonal.abs() < PIVOT_EPSILON {
            return Err(SpiceError::SingularMatrix);
        }
        let mut value = rhs[row];
        for (&col, &entry) in &rows[row] {
            if col > row {
                value -= entry * solution[col];
            }
        }
        solution[row] = value / diagonal;
        if !solution[row].is_finite() {
            return Err(SpiceError::SingularMatrix);
        }
    }
    Ok(solution)
}

fn transpose_complex_matrix(matrix: &[Vec<Complex>]) -> Vec<Vec<Complex>> {
    (0..matrix.len())
        .map(|row| (0..matrix.len()).map(|col| matrix[col][row]).collect())
        .collect()
}
