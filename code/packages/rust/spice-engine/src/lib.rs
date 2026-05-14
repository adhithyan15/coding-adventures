use std::collections::{BTreeMap, HashMap};
use std::fmt;

const PIVOT_EPSILON: f64 = 1.0e-12;
const TWO_PI: f64 = std::f64::consts::PI * 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Circuit {
    elements: Vec<Element>,
}

impl Circuit {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    pub fn add(&mut self, element: Element) {
        self.elements.push(element);
    }

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }
}

impl Default for Circuit {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Resistor(Resistor),
    Capacitor(Capacitor),
    Inductor(Inductor),
    VoltageSource(VoltageSource),
    CurrentSource(CurrentSource),
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentSource {
    pub name: String,
    pub positive: String,
    pub negative: String,
    pub current: f64,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcResult {
    pub node_voltages: BTreeMap<String, f64>,
    pub branch_currents: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DcSweepPoint {
    pub value: f64,
    pub result: DcResult,
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
    let linear_solution = solve_linear_circuit(circuit, &[], &[])?;

    Ok(DcResult {
        node_voltages: linear_solution.node_voltages,
        branch_currents: linear_solution.branch_currents,
    })
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
        let linear_solution = solve_linear_circuit(circuit, &capacitor_states, &inductor_states)?;
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
}

#[derive(Debug, Clone, PartialEq)]
struct AcSolution {
    node_voltages: BTreeMap<String, Complex>,
    branch_currents: BTreeMap<String, Complex>,
}

fn solve_linear_circuit(
    circuit: &Circuit,
    capacitor_states: &[CapacitorState],
    inductor_states: &[InductorState],
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
        });
    }

    let mut matrix = vec![vec![0.0; matrix_size]; matrix_size];
    let mut rhs = vec![0.0; matrix_size];

    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => stamp_resistor(resistor, &node_indices, &mut matrix)?,
            Element::Capacitor(capacitor) => stamp_capacitor(
                capacitor,
                capacitor_states,
                &node_indices,
                &mut matrix,
                &mut rhs,
            )?,
            Element::Inductor(inductor) => stamp_inductor(
                inductor,
                inductor_states,
                &node_indices,
                &voltage_sources,
                node_count,
                &mut matrix,
                &mut rhs,
            )?,
            Element::VoltageSource(source) => stamp_voltage_source(
                source,
                &node_indices,
                &voltage_sources,
                node_count,
                &mut matrix,
                &mut rhs,
            )?,
            Element::CurrentSource(source) => {
                stamp_current_source(source, &node_indices, &mut rhs)?
            }
        }
    }

    let solution = solve_linear_system(matrix, rhs)?;
    let node_voltages = node_voltages_from_solution(&node_indices, &solution);
    let mut branch_currents = BTreeMap::new();
    for (source_name, branch_index) in voltage_sources {
        branch_currents.insert(
            format!("I({source_name})"),
            solution[node_count + branch_index],
        );
    }
    insert_transient_inductor_currents(
        circuit,
        inductor_states,
        &node_voltages,
        &mut branch_currents,
    );

    Ok(LinearSolution {
        node_voltages,
        branch_currents,
    })
}

fn solve_ac_circuit(circuit: &Circuit, omega: f64) -> Result<AcSolution, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    let voltage_sources = collect_ac_voltage_sources(circuit)?;
    let node_count = node_indices.len();
    let branch_count = voltage_sources.len();
    let matrix_size = node_count + branch_count;

    if matrix_size == 0 {
        return Ok(AcSolution {
            node_voltages: BTreeMap::new(),
            branch_currents: BTreeMap::new(),
        });
    }

    let mut matrix = vec![vec![Complex::zero(); matrix_size]; matrix_size];
    let mut rhs = vec![Complex::zero(); matrix_size];

    for element in circuit.elements() {
        match element {
            Element::Resistor(resistor) => stamp_ac_resistor(resistor, &node_indices, &mut matrix)?,
            Element::Capacitor(capacitor) => {
                stamp_ac_capacitor(capacitor, omega, &node_indices, &mut matrix)?
            }
            Element::Inductor(inductor) => {
                stamp_ac_inductor(inductor, omega, &node_indices, &mut matrix)?
            }
            Element::VoltageSource(source) => stamp_ac_voltage_source(
                source,
                &node_indices,
                &voltage_sources,
                node_count,
                &mut matrix,
                &mut rhs,
            )?,
            Element::CurrentSource(source) => {
                stamp_ac_current_source(source, &node_indices, &mut rhs)?
            }
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
        if let Element::VoltageSource(source) = element {
            insert_branch_name(&mut sources, &source.name, "duplicate voltage source name")?;
        }
    }
    Ok(sources)
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
    matrix: &mut [Vec<f64>],
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    if !source.voltage.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage must be finite".to_string(),
        });
    }

    let branch = node_count + voltage_sources[&source.name];
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);

    stamp_branch_matrix(matrix, branch, positive, negative);
    rhs[branch] += source.voltage;
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
    rhs: &mut [f64],
) -> Result<(), SpiceError> {
    if !source.current.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "current must be finite".to_string(),
        });
    }

    if let Some(i) = node_index(node_indices, &source.positive) {
        rhs[i] -= source.current;
    }
    if let Some(j) = node_index(node_indices, &source.negative) {
        rhs[j] += source.current;
    }
    Ok(())
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
    node_indices: &HashMap<String, usize>,
    voltage_sources: &BTreeMap<String, usize>,
    node_count: usize,
    matrix: &mut [Vec<Complex>],
    rhs: &mut [Complex],
) -> Result<(), SpiceError> {
    if !source.voltage.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "voltage must be finite".to_string(),
        });
    }

    let branch = node_count + voltage_sources[&source.name];
    let positive = node_index(node_indices, &source.positive);
    let negative = node_index(node_indices, &source.negative);

    stamp_complex_branch_matrix(matrix, branch, positive, negative);
    rhs[branch] += Complex::new(source.voltage, 0.0);
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
    rhs: &mut [Complex],
) -> Result<(), SpiceError> {
    if !source.current.is_finite() {
        return Err(SpiceError::InvalidElement {
            name: source.name.clone(),
            reason: "current must be finite".to_string(),
        });
    }

    let current = Complex::new(source.current, 0.0);
    if let Some(i) = node_index(node_indices, &source.positive) {
        rhs[i] -= current;
    }
    if let Some(j) = node_index(node_indices, &source.negative) {
        rhs[j] += current;
    }
    Ok(())
}

fn solve_linear_system(
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
