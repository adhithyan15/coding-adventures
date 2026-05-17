use std::collections::{BTreeMap, HashMap};
use std::fmt;

const PIVOT_EPSILON: f64 = 1.0e-12;
const SPARSE_SOLVER_THRESHOLD: usize = 30;
const TWO_PI: f64 = std::f64::consts::PI * 2.0;
const BOLTZMANN: f64 = 1.380_649e-23;

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
        Element::Diode(element) => Element::Diode(Diode::with_model(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.anode, instance_name, node_map),
            map_subckt_node(&element.cathode, instance_name, node_map),
            element.saturation_current,
            element.thermal_voltage,
        )),
        Element::Bjt(element) => Element::Bjt(Bjt::with_model(
            format!("{instance_name}.{}", element.name),
            map_subckt_node(&element.collector, instance_name, node_map),
            map_subckt_node(&element.base, instance_name, node_map),
            map_subckt_node(&element.emitter, instance_name, node_map),
            element.polarity,
            element.saturation_current,
            element.forward_beta,
            element.thermal_voltage,
        )),
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
    VoltageSource(VoltageSource),
    CurrentSource(CurrentSource),
    BSource(BSource),
    Diode(Diode),
    Bjt(Bjt),
    Mosfet(Mosfet),
    Vccs(Vccs),
    Vcvs(Vcvs),
    Cccs(Cccs),
    Ccvs(Ccvs),
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
        Self {
            name: name.into(),
            anode: anode.into(),
            cathode: cathode.into(),
            saturation_current,
            thermal_voltage,
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
    pub saturation_current: f64,
    pub n_sub: f64,
    pub t_nom: f64,
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
            saturation_current: 1.0e-15,
            n_sub: 1.4,
            t_nom: 300.15,
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

#[derive(Debug, Clone, PartialEq)]
pub struct DcResult {
    pub node_voltages: BTreeMap<String, f64>,
    pub branch_currents: BTreeMap<String, f64>,
    pub iterations: usize,
    pub converged: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DcOpOptions {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub convergence_aids: bool,
}

impl Default for DcOpOptions {
    fn default() -> Self {
        Self {
            max_iterations: 80,
            tolerance: 1.0e-9,
            convergence_aids: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcSweepPoint {
    pub value: f64,
    pub result: DcResult,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum NoiseType {
    Thermal,
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
pub struct TransientPoint {
    pub time: f64,
    pub node_voltages: BTreeMap<String, f64>,
    pub branch_currents: BTreeMap<String, f64>,
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
    validate_dc_op_options(options)?;
    let solution = solve_dc_newton(circuit, options, None)?;
    if solution.converged || !options.convergence_aids {
        return Ok(dc_result_from_linear_solution(solution));
    }

    let final_solution =
        if let Some(aided) = solve_dc_with_gmin_stepping(circuit, options, &solution.vector)? {
            aided
        } else if let Some(aided) = solve_dc_with_source_stepping(circuit, options)? {
            aided
        } else {
            solution
        };
    Ok(dc_result_from_linear_solution(final_solution))
}

fn dc_result_from_linear_solution(solution: LinearSolution) -> DcResult {
    DcResult {
        node_voltages: solution.node_voltages,
        branch_currents: solution.branch_currents,
        iterations: solution.iterations,
        converged: solution.converged,
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

pub fn ac_sweep(
    circuit: &Circuit,
    start_hz: f64,
    stop_hz: f64,
    points_per_decade: usize,
) -> Result<Vec<AcPoint>, SpiceError> {
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
    let uses_explicit_ac_sources = circuit_has_explicit_ac_sources(circuit);
    let operating_point = if uses_explicit_ac_sources && matrix_size > 0 {
        solve_linear_circuit(circuit, &[], &[], None)?.vector
    } else {
        vec![0.0; matrix_size]
    };
    let output_index = node_index(&node_indices, output_node);
    let noise_sources = collect_noise_sources(circuit, &node_indices, temperature_kelvin)?;
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
                entries: zero_noise_entries(&noise_sources),
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
                    entries: zero_noise_entries(&noise_sources),
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
                NoiseEntry {
                    element_name: source.element_name.clone(),
                    noise_type: source.noise_type,
                    source_psd: source.source_psd,
                    output_psd: transfer.abs().powi(2) * source.source_psd,
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

    let mut capacitor_states = initial_capacitor_states(circuit, time_step);
    let mut inductor_states = initial_inductor_states(circuit, time_step);
    let mut points = Vec::new();
    let mut time = time_step;
    while time <= stop_time + time_step * 1.0e-9 {
        let linear_solution =
            solve_linear_circuit(circuit, &capacitor_states, &inductor_states, Some(time))?;
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
        points.push(TransientPoint {
            time,
            node_voltages: linear_solution.node_voltages,
            branch_currents: linear_solution.branch_currents,
        });
        time += time_step;
    }
    Ok(points)
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
        Element::Diode(diode) => Some((
            diode.name.clone(),
            "saturation_current".to_string(),
            diode.saturation_current,
        )),
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
        Element::BSource(_) | Element::Capacitor(_) | Element::Inductor(_) => None,
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
        Element::Diode(diode) => diode.saturation_current += delta,
        Element::Bjt(bjt) => bjt.saturation_current += delta,
        Element::Mosfet(mosfet) => mosfet.params.kp += delta,
        Element::Vccs(source) => source.transconductance_siemens += delta,
        Element::Vcvs(source) => source.gain += delta,
        Element::Cccs(source) => source.gain += delta,
        Element::Ccvs(source) => source.transresistance_ohms += delta,
        Element::BSource(_) | Element::Capacitor(_) | Element::Inductor(_) => {}
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
        Element::Diode(diode) => {
            let mut varied = diode.clone();
            varied.saturation_current =
                randomized_value(varied.saturation_current, tolerance, distribution, rng);
            Element::Diode(varied)
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
        Element::BSource(_) | Element::Capacitor(_) | Element::Inductor(_) => element.clone(),
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
    time_step: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct InductorState {
    name: String,
    previous_current: f64,
    time_step: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct LinearSolution {
    node_voltages: BTreeMap<String, f64>,
    branch_currents: BTreeMap<String, f64>,
    vector: Vec<f64>,
    iterations: usize,
    converged: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct LinearSolveOptions<'a> {
    max_iterations: usize,
    tolerance: f64,
    initial_vector: Option<&'a [f64]>,
    return_singular_as_unconverged: bool,
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
        });
    }

    let has_nonlinear = circuit.elements().iter().any(|element| {
        matches!(
            element,
            Element::Diode(_) | Element::Bjt(_) | Element::Mosfet(_) | Element::BSource(_)
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
    if !has_nonlinear {
        return Ok(LinearSolution {
            iterations: 1,
            converged: solution.converged,
            ..solution
        });
    }

    let mut iterations = 1;
    while iterations < options.max_iterations {
        if !solution.converged {
            return Ok(LinearSolution {
                iterations,
                converged: false,
                ..solution
            });
        }
        let delta = max_vector_delta(&solution.vector, &operating_point);
        operating_point = solution.vector.clone();
        if delta < options.tolerance {
            return Ok(LinearSolution {
                iterations,
                converged: true,
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
        iterations += 1;
    }

    let delta = max_vector_delta(&solution.vector, &operating_point);
    Ok(LinearSolution {
        iterations,
        converged: delta < options.tolerance,
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
            Element::Inductor(inductor) => stamp_inductor(
                inductor,
                inductor_states,
                node_indices,
                &voltage_sources,
                node_count,
                &mut matrix,
                &mut rhs,
            )?,
            Element::VoltageSource(source) => stamp_voltage_source(
                source,
                node_indices,
                &voltage_sources,
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
            Element::Diode(diode) => {
                stamp_diode(diode, node_indices, &mut matrix, &mut rhs, operating_point)?
            }
            Element::Bjt(bjt) => {
                stamp_bjt(bjt, node_indices, &mut matrix, &mut rhs, operating_point)?
            }
            Element::Mosfet(mosfet) => {
                stamp_mosfet(mosfet, node_indices, &mut matrix, &mut rhs, operating_point)?
            }
            Element::Vccs(source) => stamp_vccs(source, node_indices, &mut matrix)?,
            Element::Vcvs(source) => stamp_vcvs(
                source,
                node_indices,
                &voltage_sources,
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

    let solution = solve_linear_system(matrix, rhs)?;
    Ok(linear_solution_from_vector(
        circuit,
        inductor_states,
        node_indices,
        voltage_sources,
        node_count,
        &solution,
        true,
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
    }
}

fn max_vector_delta(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (left - right).abs())
        .fold(0.0, f64::max)
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
            | Element::Diode(_)
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

    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => stamp_ac_resistor(resistor, node_indices, &mut matrix)?,
            Element::Capacitor(capacitor) => {
                stamp_ac_capacitor(capacitor, omega, node_indices, &mut matrix)?
            }
            Element::Inductor(inductor) => {
                stamp_ac_inductor(inductor, omega, node_indices, &mut matrix)?
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
            Element::Diode(diode) => {
                validate_diode(diode)?;
                let anode = node_index(node_indices, &diode.anode);
                let cathode = node_index(node_indices, &diode.cathode);
                let voltage = vector_voltage(operating_point, anode)
                    - vector_voltage(operating_point, cathode);
                let exponent = (voltage / diode.thermal_voltage).clamp(-40.0, 40.0);
                stamp_complex_conductance(
                    &mut matrix,
                    anode,
                    cathode,
                    Complex::new(
                        diode.saturation_current / diode.thermal_voltage * exponent.exp(),
                        0.0,
                    ),
                );
            }
            Element::Bjt(bjt) => {
                stamp_ac_bjt_small_signal(bjt, node_indices, &mut matrix, operating_point)?
            }
            Element::Mosfet(mosfet) => {
                stamp_ac_mosfet_small_signal(mosfet, node_indices, &mut matrix, operating_point)?
            }
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
            Element::Diode(diode) => {
                validate_diode(diode)?;
                let anode = node_index(node_indices, &diode.anode);
                let cathode = node_index(node_indices, &diode.cathode);
                let voltage = vector_voltage(operating_point, anode)
                    - vector_voltage(operating_point, cathode);
                let exponent = (voltage / diode.thermal_voltage).clamp(-40.0, 40.0);
                stamp_conductance(
                    &mut matrix,
                    anode,
                    cathode,
                    diode.saturation_current / diode.thermal_voltage * exponent.exp(),
                );
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
            Element::Diode(diode) => {
                insert_node(&mut names, &diode.anode);
                insert_node(&mut names, &diode.cathode);
            }
            Element::Bjt(bjt) => {
                insert_node(&mut names, &bjt.collector);
                insert_node(&mut names, &bjt.base);
                insert_node(&mut names, &bjt.emitter);
            }
            Element::Mosfet(mosfet) => {
                insert_node(&mut names, &mosfet.drain);
                insert_node(&mut names, &mosfet.gate);
                insert_node(&mut names, &mosfet.source);
                insert_node(&mut names, &mosfet.body);
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

fn collect_noise_sources(
    circuit: &Circuit,
    node_indices: &HashMap<String, usize>,
    temperature_kelvin: f64,
) -> Result<Vec<NoiseSource>, SpiceError> {
    let mut sources = Vec::new();
    for element in circuit.elements() {
        if let Element::Resistor(resistor) = element {
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
            });
        }
    }
    Ok(sources)
}

fn zero_noise_entries(sources: &[NoiseSource]) -> Vec<NoiseEntry> {
    sources
        .iter()
        .map(|source| NoiseEntry {
            element_name: source.element_name.clone(),
            noise_type: source.noise_type,
            source_psd: source.source_psd,
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

    let conductance = capacitor.capacitance_farads / state.time_step;
    let n1 = node_index(node_indices, &capacitor.n1);
    let n2 = node_index(node_indices, &capacitor.n2);
    stamp_conductance(matrix, n1, n2, conductance);

    let history_current = conductance * state.previous_voltage;
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

    let conductance = state.time_step / inductor.inductance_henrys;
    stamp_conductance(matrix, n1, n2, conductance);
    if let Some(i) = n1 {
        rhs[i] -= state.previous_current;
    }
    if let Some(j) = n2 {
        rhs[j] += state.previous_current;
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

fn stamp_diode(
    diode: &Diode,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_diode(diode)?;
    let anode = node_index(node_indices, &diode.anode);
    let cathode = node_index(node_indices, &diode.cathode);
    let voltage = anode.map_or(0.0, |index| operating_point[index])
        - cathode.map_or(0.0, |index| operating_point[index]);
    let exponent = (voltage / diode.thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    let current = diode.saturation_current * (exp_value - 1.0);
    let conductance = diode.saturation_current / diode.thermal_voltage * exp_value;
    let equivalent_current = current - conductance * voltage;

    stamp_conductance(matrix, anode, cathode, conductance);
    if let Some(index) = anode {
        rhs[index] -= equivalent_current;
    }
    if let Some(index) = cathode {
        rhs[index] += equivalent_current;
    }
    Ok(())
}

fn stamp_bjt(
    bjt: &Bjt,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_bjt(bjt)?;
    let collector = node_index(node_indices, &bjt.collector);
    let base = node_index(node_indices, &bjt.base);
    let emitter = node_index(node_indices, &bjt.emitter);
    let base_voltage = base.map_or(0.0, |index| operating_point[index]);
    let emitter_voltage = emitter.map_or(0.0, |index| operating_point[index]);
    let junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - base_voltage,
    };
    let exponent = (junction_voltage / bjt.thermal_voltage).clamp(-40.0, 40.0);
    let exp_value = exponent.exp();
    let collector_current = bjt.saturation_current * (exp_value - 1.0);
    let gm = bjt.saturation_current / bjt.thermal_voltage * exp_value;
    let gpi = gm / bjt.forward_beta;
    let base_current = collector_current / bjt.forward_beta;
    let equivalent_collector_current = collector_current - gm * junction_voltage;
    let equivalent_base_current = base_current - gpi * junction_voltage;

    match bjt.polarity {
        BjtPolarity::Npn => {
            stamp_conductance(matrix, base, emitter, gpi);
            stamp_transconductance(matrix, collector, emitter, base, emitter, gm);
            stamp_equivalent_current_source(rhs, base, emitter, equivalent_base_current);
            stamp_equivalent_current_source(rhs, collector, emitter, equivalent_collector_current);
        }
        BjtPolarity::Pnp => {
            stamp_conductance(matrix, emitter, base, gpi);
            stamp_transconductance(matrix, emitter, collector, emitter, base, gm);
            stamp_equivalent_current_source(rhs, emitter, base, equivalent_base_current);
            stamp_equivalent_current_source(rhs, emitter, collector, equivalent_collector_current);
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
    let collector = node_index(node_indices, &bjt.collector);
    let base = node_index(node_indices, &bjt.base);
    let emitter = node_index(node_indices, &bjt.emitter);
    let base_voltage = vector_voltage(operating_point, base);
    let emitter_voltage = vector_voltage(operating_point, emitter);
    let junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - base_voltage,
    };
    let exponent = (junction_voltage / bjt.thermal_voltage).clamp(-40.0, 40.0);
    let gm = bjt.saturation_current / bjt.thermal_voltage * exponent.exp();
    let gpi = gm / bjt.forward_beta;
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
) -> Result<(), SpiceError> {
    validate_bjt(bjt)?;
    let collector = node_index(node_indices, &bjt.collector);
    let base = node_index(node_indices, &bjt.base);
    let emitter = node_index(node_indices, &bjt.emitter);
    let base_voltage = vector_voltage(operating_point, base);
    let emitter_voltage = vector_voltage(operating_point, emitter);
    let junction_voltage = match bjt.polarity {
        BjtPolarity::Npn => base_voltage - emitter_voltage,
        BjtPolarity::Pnp => emitter_voltage - base_voltage,
    };
    let exponent = (junction_voltage / bjt.thermal_voltage).clamp(-40.0, 40.0);
    let gm = Complex::new(
        bjt.saturation_current / bjt.thermal_voltage * exponent.exp(),
        0.0,
    );
    let gpi = Complex::new(gm.real / bjt.forward_beta, 0.0);
    match bjt.polarity {
        BjtPolarity::Npn => {
            stamp_complex_conductance(matrix, base, emitter, gpi);
            stamp_complex_transconductance(matrix, collector, emitter, base, emitter, gm);
        }
        BjtPolarity::Pnp => {
            stamp_complex_conductance(matrix, emitter, base, gpi);
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
}

fn stamp_mosfet(
    mosfet: &Mosfet,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_mosfet(mosfet)?;
    let drain = node_index(node_indices, &mosfet.drain);
    let gate = node_index(node_indices, &mosfet.gate);
    let source = node_index(node_indices, &mosfet.source);
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
    let beta = params.kp * (params.w / params.l);
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
        };
    }

    let drain_current = 0.5 * beta * overdrive * overdrive * (1.0 + params.lambda * vds);
    let gm = beta * overdrive * (1.0 + params.lambda * vds);
    MosfetDcResult {
        drain_current,
        gm,
        gds: 0.5 * beta * overdrive * overdrive * params.lambda,
        gmb: gm * body_factor,
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
    let drain = node_index(node_indices, &mosfet.drain);
    let gate = node_index(node_indices, &mosfet.gate);
    let source = node_index(node_indices, &mosfet.source);
    let body = node_index(node_indices, &mosfet.body);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let body_voltage = vector_voltage(operating_point, body);
    let vgs = gate_voltage - source_voltage;
    let vds = drain_voltage - source_voltage;
    let vbs = body_voltage - source_voltage;
    let result = evaluate_mosfet_level1(mosfet, vgs, vds, vbs);
    stamp_conductance(matrix, drain, source, result.gds);
    stamp_transconductance(matrix, drain, source, gate, source, result.gm);
    stamp_transconductance(matrix, drain, source, body, source, result.gmb);
    Ok(())
}

fn stamp_ac_mosfet_small_signal(
    mosfet: &Mosfet,
    node_indices: &HashMap<String, usize>,
    matrix: &mut [Vec<Complex>],
    operating_point: &[f64],
) -> Result<(), SpiceError> {
    validate_mosfet(mosfet)?;
    let drain = node_index(node_indices, &mosfet.drain);
    let gate = node_index(node_indices, &mosfet.gate);
    let source = node_index(node_indices, &mosfet.source);
    let body = node_index(node_indices, &mosfet.body);
    let drain_voltage = vector_voltage(operating_point, drain);
    let gate_voltage = vector_voltage(operating_point, gate);
    let source_voltage = vector_voltage(operating_point, source);
    let body_voltage = vector_voltage(operating_point, body);
    let vgs = gate_voltage - source_voltage;
    let vds = drain_voltage - source_voltage;
    let vbs = body_voltage - source_voltage;
    let result = evaluate_mosfet_level1(mosfet, vgs, vds, vbs);
    stamp_complex_conductance(matrix, drain, source, Complex::new(result.gds, 0.0));
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
    Ok(())
}

fn validate_reactive_elements(circuit: &Circuit) -> Result<(), SpiceError> {
    for element in circuit.elements() {
        match element {
            Element::Capacitor(capacitor) => validate_capacitor(capacitor)?,
            Element::Inductor(inductor) => validate_inductor(inductor)?,
            _ => {}
        }
    }
    Ok(())
}

fn validate_diode(diode: &Diode) -> Result<(), SpiceError> {
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
    if !bjt.thermal_voltage.is_finite() || bjt.thermal_voltage <= 0.0 {
        return Err(SpiceError::InvalidElement {
            name: bjt.name.clone(),
            reason: "thermal voltage must be finite and positive".to_string(),
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
        ("IS", params.saturation_current),
        ("N_SUB", params.n_sub),
        ("T_NOM", params.t_nom),
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

fn initial_capacitor_states(circuit: &Circuit, time_step: f64) -> Vec<CapacitorState> {
    circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::Capacitor(capacitor) => Some(CapacitorState {
                name: capacitor.name.clone(),
                previous_voltage: capacitor.initial_voltage,
                time_step,
            }),
            _ => None,
        })
        .collect()
}

fn initial_inductor_states(circuit: &Circuit, time_step: f64) -> Vec<InductorState> {
    circuit
        .elements()
        .iter()
        .filter_map(|element| match element {
            Element::Inductor(inductor) => Some(InductorState {
                name: inductor.name.clone(),
                previous_current: inductor.initial_current,
                time_step,
            }),
            _ => None,
        })
        .collect()
}

fn update_capacitor_states(
    circuit: &Circuit,
    node_voltages: &BTreeMap<String, f64>,
    capacitor_states: &mut [CapacitorState],
) {
    for state in capacitor_states {
        let Some(capacitor) = circuit.elements().iter().find_map(|element| match element {
            Element::Capacitor(capacitor) if capacitor.name == state.name => Some(capacitor),
            _ => None,
        }) else {
            continue;
        };
        state.previous_voltage =
            voltage_at(node_voltages, &capacitor.n1) - voltage_at(node_voltages, &capacitor.n2);
    }
}

fn update_inductor_states(
    circuit: &Circuit,
    node_voltages: &BTreeMap<String, f64>,
    inductor_states: &mut [InductorState],
) {
    for state in inductor_states {
        let Some(inductor) = circuit.elements().iter().find_map(|element| match element {
            Element::Inductor(inductor) if inductor.name == state.name => Some(inductor),
            _ => None,
        }) else {
            continue;
        };
        state.previous_current = inductor_current(inductor, state, node_voltages);
    }
}

fn insert_transient_inductor_currents(
    circuit: &Circuit,
    inductor_states: &[InductorState],
    node_voltages: &BTreeMap<String, f64>,
    branch_currents: &mut BTreeMap<String, f64>,
) {
    for state in inductor_states {
        let Some(inductor) = circuit.elements().iter().find_map(|element| match element {
            Element::Inductor(inductor) if inductor.name == state.name => Some(inductor),
            _ => None,
        }) else {
            continue;
        };
        branch_currents.insert(
            format!("I({})", inductor.name),
            inductor_current(inductor, state, node_voltages),
        );
    }
}

fn inductor_current(
    inductor: &Inductor,
    state: &InductorState,
    node_voltages: &BTreeMap<String, f64>,
) -> f64 {
    let conductance = state.time_step / inductor.inductance_henrys;
    let voltage = voltage_at(node_voltages, &inductor.n1) - voltage_at(node_voltages, &inductor.n2);
    state.previous_current + conductance * voltage
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
        matrix[branch][node_indices[&node]] =
            matrix[branch][node_indices[&node]] - Complex::new(derivative, 0.0);
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
    if rhs.len() >= SPARSE_SOLVER_THRESHOLD {
        return solve_sparse_linear_system(matrix, rhs);
    }
    solve_dense_linear_system(matrix, rhs)
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

fn solve_sparse_linear_system(
    matrix: Vec<Vec<f64>>,
    rhs: Vec<f64>,
) -> Result<Vec<f64>, SpiceError> {
    let n = rhs.len();
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
    }

    let mut solution = vec![0.0; n];
    for row in (0..n).rev() {
        let diagonal = rows[row].get(&row).copied().unwrap_or(0.0);
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
    }

    Ok(solution)
}

fn solve_complex_linear_system(
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

fn transpose_complex_matrix(matrix: &[Vec<Complex>]) -> Vec<Vec<Complex>> {
    (0..matrix.len())
        .map(|row| (0..matrix.len()).map(|col| matrix[col][row]).collect())
        .collect()
}
