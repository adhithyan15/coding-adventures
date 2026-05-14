use std::collections::{BTreeMap, HashMap};
use std::fmt;

const PIVOT_EPSILON: f64 = 1.0e-12;

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
    let linear_solution = solve_linear_circuit(circuit, &[])?;

    Ok(DcResult {
        node_voltages: linear_solution.node_voltages,
        branch_currents: linear_solution.branch_currents,
    })
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

    validate_capacitors(circuit)?;

    let mut capacitor_states = initial_capacitor_states(circuit, time_step);
    let mut points = Vec::new();
    let mut time = time_step;
    while time <= stop_time + time_step * 1.0e-9 {
        let linear_solution = solve_linear_circuit(circuit, &capacitor_states)?;
        update_capacitor_states(
            circuit,
            &linear_solution.node_voltages,
            &mut capacitor_states,
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

#[derive(Debug, Clone, PartialEq)]
struct CapacitorState {
    name: String,
    previous_voltage: f64,
    time_step: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct LinearSolution {
    node_voltages: BTreeMap<String, f64>,
    branch_currents: BTreeMap<String, f64>,
}

fn solve_linear_circuit(
    circuit: &Circuit,
    capacitor_states: &[CapacitorState],
) -> Result<LinearSolution, SpiceError> {
    let node_indices = collect_node_indices(circuit);
    let voltage_sources = collect_voltage_sources(circuit)?;
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

    Ok(LinearSolution {
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

fn collect_voltage_sources(circuit: &Circuit) -> Result<BTreeMap<String, usize>, SpiceError> {
    let mut sources = BTreeMap::new();
    for element in circuit.elements() {
        if let Element::VoltageSource(source) = element {
            if sources.contains_key(&source.name) {
                return Err(SpiceError::InvalidElement {
                    name: source.name.clone(),
                    reason: "duplicate voltage source name".to_string(),
                });
            }
            sources.insert(source.name.clone(), sources.len());
        }
    }
    Ok(sources)
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

fn validate_capacitors(circuit: &Circuit) -> Result<(), SpiceError> {
    for element in circuit.elements() {
        if let Element::Capacitor(capacitor) = element {
            validate_capacitor(capacitor)?;
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

    if let Some(i) = positive {
        matrix[i][branch] += 1.0;
        matrix[branch][i] += 1.0;
    }
    if let Some(j) = negative {
        matrix[j][branch] -= 1.0;
        matrix[branch][j] -= 1.0;
    }
    rhs[branch] += source.voltage;
    Ok(())
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
