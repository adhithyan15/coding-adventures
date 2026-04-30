use std::collections::HashMap;

use neural_network::{NeuralGraph, NeuralNetwork, PropertyValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Opcode { LoadInput, LoadConst, LoadEdgeWeight, Mul, Add, Activate, StoreOutput }

#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub op: Opcode,
    pub dst: Option<String>,
    pub input_name: Option<String>,
    pub output_name: Option<String>,
    pub edge_id: Option<String>,
    pub value: Option<f64>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub inputs: Vec<String>,
    pub input: Option<String>,
    pub activation: Option<String>,
    pub source_node: Option<String>,
    pub source_edge: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function { pub id: String, pub kind: String, pub instructions: Vec<Instruction> }
#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge { pub id: String, pub from: String, pub to: String, pub weight: f64 }
#[derive(Clone, Debug, PartialEq)]
pub struct Module { pub magic: String, pub version: u8, pub nodes: Vec<String>, pub edges: Vec<GraphEdge>, pub functions: Vec<Function> }

pub fn compile_neural_network_to_bytecode(network: &NeuralNetwork) -> Result<Module, String> {
    compile_neural_graph_to_bytecode(&network.graph)
}

pub fn compile_neural_graph_to_bytecode(graph: &NeuralGraph) -> Result<Module, String> {
    let order = graph.topological_sort()?;
    let mut instructions = Vec::new();
    let mut values: HashMap<String, String> = HashMap::new();
    let mut next_value_id = 0usize;
    let mut alloc = || { let id = format!("v{}", next_value_id); next_value_id += 1; id };

    for node in order {
        let properties = graph.node_properties(&node);
        let op = string_prop(properties.get("nn.op")).unwrap_or("weighted_sum");
        match op {
            "input" => {
                let dst = alloc();
                values.insert(node.clone(), dst.clone());
                instructions.push(inst(Opcode::LoadInput, Some(dst), Some(string_prop(properties.get("nn.input")).unwrap_or(&node).to_string()), None, None, None, None, None, vec![], None, None, Some(node), None));
            }
            "constant" => {
                let dst = alloc();
                values.insert(node.clone(), dst.clone());
                let value = number_prop(properties.get("nn.value")).ok_or_else(|| format!("constant node {node} is missing nn.value"))?;
                instructions.push(inst(Opcode::LoadConst, Some(dst), None, None, None, Some(value), None, None, vec![], None, None, Some(node), None));
            }
            "weighted_sum" => {
                let mut incoming = graph.incoming_edges(&node);
                incoming.sort_by(|a, b| a.id.cmp(&b.id));
                let mut terms = Vec::new();
                for edge in incoming {
                    let source = values.get(&edge.from).ok_or_else(|| format!("source node has no value: {}", edge.from))?.clone();
                    let weight_value = alloc();
                    let term_value = alloc();
                    instructions.push(inst(Opcode::LoadEdgeWeight, Some(weight_value.clone()), None, None, Some(edge.id.clone()), None, None, None, vec![], None, None, None, Some(edge.id.clone())));
                    instructions.push(inst(Opcode::Mul, Some(term_value.clone()), None, None, None, None, Some(source), Some(weight_value), vec![], None, None, None, Some(edge.id)));
                    terms.push(term_value);
                }
                let dst = alloc();
                values.insert(node.clone(), dst.clone());
                if terms.is_empty() {
                    instructions.push(inst(Opcode::LoadConst, Some(dst), None, None, None, Some(0.0), None, None, vec![], None, None, Some(node), None));
                } else {
                    instructions.push(inst(Opcode::Add, Some(dst), None, None, None, None, None, None, terms, None, None, Some(node), None));
                }
            }
            "activation" => {
                let input = single_input_value(graph, &values, &node)?;
                let dst = alloc();
                values.insert(node.clone(), dst.clone());
                instructions.push(inst(Opcode::Activate, Some(dst), None, None, None, None, None, None, vec![], Some(input), Some(string_prop(properties.get("nn.activation")).unwrap_or("relu").to_string()), Some(node), None));
            }
            "output" => {
                let input = single_input_value(graph, &values, &node)?;
                values.insert(node.clone(), input.clone());
                instructions.push(inst(Opcode::StoreOutput, None, None, Some(string_prop(properties.get("nn.output")).unwrap_or(&node).to_string()), None, None, None, None, vec![], Some(input), None, Some(node), None));
            }
            other => return Err(format!("unsupported neural graph op: {other}")),
        }
    }

    Ok(Module {
        magic: "CANN".to_string(),
        version: 0,
        nodes: graph.nodes(),
        edges: graph.edges().into_iter().map(|edge| GraphEdge { id: edge.id, from: edge.from, to: edge.to, weight: edge.weight }).collect(),
        functions: vec![Function { id: "forward".to_string(), kind: "forward".to_string(), instructions }],
    })
}

pub fn run_neural_bytecode_forward(module: &Module, inputs: &HashMap<String, f64>) -> Result<HashMap<String, f64>, String> {
    let mut values: HashMap<String, f64> = HashMap::new();
    let edge_weights: HashMap<String, f64> = module.edges.iter().map(|edge| (edge.id.clone(), edge.weight)).collect();
    let forward = module.functions.iter().find(|f| f.kind == "forward").ok_or_else(|| "neural bytecode module has no forward function".to_string())?;
    let mut outputs = HashMap::new();
    for instruction in &forward.instructions {
        match instruction.op {
            Opcode::LoadInput => { values.insert(dst(instruction)?, *inputs.get(instruction.input_name.as_deref().unwrap_or("")).ok_or_else(|| "missing input".to_string())?); }
            Opcode::LoadConst => { values.insert(dst(instruction)?, instruction.value.unwrap_or(0.0)); }
            Opcode::LoadEdgeWeight => { values.insert(dst(instruction)?, *edge_weights.get(instruction.edge_id.as_deref().unwrap_or("")).unwrap_or(&1.0)); }
            Opcode::Mul => { values.insert(dst(instruction)?, read(&values, &instruction.left)? * read(&values, &instruction.right)?); }
            Opcode::Add => { values.insert(dst(instruction)?, instruction.inputs.iter().map(|id| values.get(id).copied().ok_or_else(|| format!("unknown value: {id}"))).collect::<Result<Vec<_>, _>>()?.iter().sum()); }
            Opcode::Activate => { values.insert(dst(instruction)?, apply_neural_activation(read(&values, &instruction.input)?, instruction.activation.as_deref().unwrap_or("relu"))); }
            Opcode::StoreOutput => { outputs.insert(instruction.output_name.clone().unwrap_or_else(|| "output".to_string()), read(&values, &instruction.input)?); }
        }
    }
    Ok(outputs)
}

pub fn apply_neural_activation(value: f64, activation: &str) -> f64 {
    match activation {
        "relu" => if value > 0.0 { value } else { 0.0 },
        "sigmoid" => 1.0 / (1.0 + (-value).exp()),
        "tanh" => value.tanh(),
        "none" => value,
        _ => value,
    }
}

fn inst(op: Opcode, dst: Option<String>, input_name: Option<String>, output_name: Option<String>, edge_id: Option<String>, value: Option<f64>, left: Option<String>, right: Option<String>, inputs: Vec<String>, input: Option<String>, activation: Option<String>, source_node: Option<String>, source_edge: Option<String>) -> Instruction {
    Instruction { op, dst, input_name, output_name, edge_id, value, left, right, inputs, input, activation, source_node, source_edge }
}
fn dst(instruction: &Instruction) -> Result<String, String> { instruction.dst.clone().ok_or_else(|| "instruction missing dst".to_string()) }
fn read(values: &HashMap<String, f64>, id: &Option<String>) -> Result<f64, String> { values.get(id.as_deref().unwrap_or("")).copied().ok_or_else(|| "unknown value".to_string()) }
fn string_prop(value: Option<&PropertyValue>) -> Option<&str> { match value { Some(PropertyValue::String(value)) => Some(value), _ => None } }
fn number_prop(value: Option<&PropertyValue>) -> Option<f64> { match value { Some(PropertyValue::Number(value)) => Some(*value), _ => None } }
fn single_input_value(graph: &NeuralGraph, values: &HashMap<String, String>, node: &str) -> Result<String, String> {
    let incoming = graph.incoming_edges(node);
    if incoming.len() != 1 { return Err(format!("node {node} expects exactly one input")); }
    values.get(&incoming[0].from).cloned().ok_or_else(|| format!("source node has no value: {}", incoming[0].from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use neural_network::{add_activation, add_constant, add_input, add_output, add_weighted_sum, create_neural_graph, create_xor_network, ActivationKind, PropertyBag, WeightedInput};

    fn tiny_graph() -> neural_network::NeuralGraph {
        let mut graph = create_neural_graph(Some("tiny"));
        add_input(&mut graph, "x0", "x0", PropertyBag::new());
        add_input(&mut graph, "x1", "x1", PropertyBag::new());
        add_constant(&mut graph, "bias", 1.0, PropertyBag::new());
        add_weighted_sum(&mut graph, "sum", vec![WeightedInput::new("x0", 0.25, "x0_to_sum"), WeightedInput::new("x1", 0.75, "x1_to_sum"), WeightedInput::new("bias", -1.0, "bias_to_sum")], PropertyBag::new());
        add_activation(&mut graph, "relu", "sum", ActivationKind::Relu, PropertyBag::new(), Some("sum_to_relu".to_string()));
        add_output(&mut graph, "out", "relu", "prediction", PropertyBag::new(), Some("relu_to_out".to_string()));
        graph
    }

    #[test]
    fn runs_tiny_weighted_sum() {
        let module = compile_neural_graph_to_bytecode(&tiny_graph()).unwrap();
        let mut inputs = HashMap::new();
        inputs.insert("x0".to_string(), 4.0);
        inputs.insert("x1".to_string(), 8.0);
        let outputs = run_neural_bytecode_forward(&module, &inputs).unwrap();
        assert!((outputs["prediction"] - 6.0).abs() < 1e-9);
    }

    #[test]
    fn runs_xor_network() {
        let module = compile_neural_network_to_bytecode(&create_xor_network("xor")).unwrap();
        let cases = [(0.0, 0.0, 0.0), (0.0, 1.0, 1.0), (1.0, 0.0, 1.0), (1.0, 1.0, 0.0)];
        for (x0, x1, expected) in cases {
            let mut inputs = HashMap::new();
            inputs.insert("x0".to_string(), x0);
            inputs.insert("x1".to_string(), x1);
            let output = run_neural_bytecode_forward(&module, &inputs).unwrap()["prediction"];
            if expected == 1.0 { assert!(output > 0.99); } else { assert!(output < 0.01); }
        }
    }
}
