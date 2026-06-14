use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

pub type PropertyBag = HashMap<String, PropertyValue>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationKind {
    Relu,
    Sigmoid,
    Tanh,
    None,
}

impl ActivationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActivationKind::Relu => "relu",
            ActivationKind::Sigmoid => "sigmoid",
            ActivationKind::Tanh => "tanh",
            ActivationKind::None => "none",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub weight: f64,
    pub properties: PropertyBag,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeightedInput {
    pub from: String,
    pub weight: f64,
    pub edge_id: Option<String>,
    pub properties: PropertyBag,
}

impl WeightedInput {
    pub fn new(from: impl Into<String>, weight: f64, edge_id: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            weight,
            edge_id: Some(edge_id.into()),
            properties: PropertyBag::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NeuralGraph {
    graph_properties: PropertyBag,
    nodes: Vec<String>,
    node_properties: HashMap<String, PropertyBag>,
    edges: Vec<Edge>,
    next_edge_id: usize,
}

impl NeuralGraph {
    pub fn new(name: Option<&str>) -> Self {
        let mut graph_properties = PropertyBag::new();
        graph_properties.insert("nn.version".to_string(), PropertyValue::String("0".to_string()));
        if let Some(name) = name {
            graph_properties.insert("nn.name".to_string(), PropertyValue::String(name.to_string()));
        }
        Self {
            graph_properties,
            nodes: Vec::new(),
            node_properties: HashMap::new(),
            edges: Vec::new(),
            next_edge_id: 0,
        }
    }

    pub fn graph_properties(&self) -> PropertyBag { self.graph_properties.clone() }
    pub fn nodes(&self) -> Vec<String> { self.nodes.clone() }
    pub fn edges(&self) -> Vec<Edge> { self.edges.clone() }

    pub fn add_node(&mut self, node: impl Into<String>, properties: PropertyBag) {
        let node = node.into();
        if !self.node_properties.contains_key(&node) {
            self.nodes.push(node.clone());
            self.node_properties.insert(node.clone(), PropertyBag::new());
        }
        if let Some(target) = self.node_properties.get_mut(&node) {
            target.extend(properties);
        }
    }

    pub fn node_properties(&self, node: &str) -> PropertyBag {
        self.node_properties.get(node).cloned().unwrap_or_default()
    }

    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>, weight: f64, properties: PropertyBag, edge_id: Option<String>) -> String {
        let from = from.into();
        let to = to.into();
        self.add_node(from.clone(), PropertyBag::new());
        self.add_node(to.clone(), PropertyBag::new());
        let id = edge_id.unwrap_or_else(|| {
            let id = format!("e{}", self.next_edge_id);
            self.next_edge_id += 1;
            id
        });
        let mut merged = properties;
        merged.insert("weight".to_string(), PropertyValue::Number(weight));
        self.edges.push(Edge { id: id.clone(), from, to, weight, properties: merged });
        id
    }

    pub fn incoming_edges(&self, node: &str) -> Vec<Edge> {
        self.edges.iter().filter(|edge| edge.to == node).cloned().collect()
    }

    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        let mut indegree: HashMap<String, usize> = self.nodes.iter().map(|node| (node.clone(), 0)).collect();
        for edge in &self.edges {
            *indegree.entry(edge.to.clone()).or_insert(0) += 1;
            indegree.entry(edge.from.clone()).or_insert(0);
        }
        let mut ready: Vec<String> = indegree.iter().filter_map(|(node, degree)| if *degree == 0 { Some(node.clone()) } else { None }).collect();
        ready.sort();
        let mut queue: VecDeque<String> = ready.into();
        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node.clone());
            let mut released = Vec::new();
            for edge in self.edges.iter().filter(|edge| edge.from == node) {
                if let Some(degree) = indegree.get_mut(&edge.to) {
                    *degree -= 1;
                    if *degree == 0 { released.push(edge.to.clone()); }
                }
            }
            released.sort();
            for node in released { queue.push_back(node); }
        }
        if order.len() != indegree.len() { return Err("neural graph contains a cycle".to_string()); }
        Ok(order)
    }
}

pub struct NeuralNetwork {
    pub graph: NeuralGraph,
}

impl NeuralNetwork {
    pub fn new(name: Option<&str>) -> Self { Self { graph: create_neural_graph(name) } }
    pub fn input(mut self, node: &str) -> Self { add_input(&mut self.graph, node, node, PropertyBag::new()); self }
    pub fn constant(mut self, node: &str, value: f64, properties: PropertyBag) -> Self { add_constant(&mut self.graph, node, value, properties); self }
    pub fn weighted_sum(mut self, node: &str, inputs: Vec<WeightedInput>, properties: PropertyBag) -> Self { add_weighted_sum(&mut self.graph, node, inputs, properties); self }
    pub fn activation(mut self, node: &str, input: &str, activation: ActivationKind, properties: PropertyBag, edge_id: &str) -> Self { add_activation(&mut self.graph, node, input, activation, properties, Some(edge_id.to_string())); self }
    pub fn output(mut self, node: &str, input: &str, output_name: &str, properties: PropertyBag, edge_id: &str) -> Self { add_output(&mut self.graph, node, input, output_name, properties, Some(edge_id.to_string())); self }
}

pub fn create_neural_graph(name: Option<&str>) -> NeuralGraph { NeuralGraph::new(name) }
pub fn create_neural_network(name: Option<&str>) -> NeuralNetwork { NeuralNetwork::new(name) }

pub fn add_input(graph: &mut NeuralGraph, node: &str, input_name: &str, mut properties: PropertyBag) {
    properties.insert("nn.op".to_string(), PropertyValue::String("input".to_string()));
    properties.insert("nn.input".to_string(), PropertyValue::String(input_name.to_string()));
    graph.add_node(node, properties);
}

pub fn add_constant(graph: &mut NeuralGraph, node: &str, value: f64, mut properties: PropertyBag) {
    assert!(value.is_finite(), "constant value must be finite");
    properties.insert("nn.op".to_string(), PropertyValue::String("constant".to_string()));
    properties.insert("nn.value".to_string(), PropertyValue::Number(value));
    graph.add_node(node, properties);
}

pub fn add_weighted_sum(graph: &mut NeuralGraph, node: &str, inputs: Vec<WeightedInput>, mut properties: PropertyBag) {
    properties.insert("nn.op".to_string(), PropertyValue::String("weighted_sum".to_string()));
    graph.add_node(node, properties);
    for input in inputs {
        graph.add_edge(input.from, node.to_string(), input.weight, input.properties, input.edge_id);
    }
}

pub fn add_activation(graph: &mut NeuralGraph, node: &str, input: &str, activation: ActivationKind, mut properties: PropertyBag, edge_id: Option<String>) -> String {
    properties.insert("nn.op".to_string(), PropertyValue::String("activation".to_string()));
    properties.insert("nn.activation".to_string(), PropertyValue::String(activation.as_str().to_string()));
    graph.add_node(node, properties);
    graph.add_edge(input.to_string(), node.to_string(), 1.0, PropertyBag::new(), edge_id)
}

pub fn add_output(graph: &mut NeuralGraph, node: &str, input: &str, output_name: &str, mut properties: PropertyBag, edge_id: Option<String>) -> String {
    properties.insert("nn.op".to_string(), PropertyValue::String("output".to_string()));
    properties.insert("nn.output".to_string(), PropertyValue::String(output_name.to_string()));
    graph.add_node(node, properties);
    graph.add_edge(input.to_string(), node.to_string(), 1.0, PropertyBag::new(), edge_id)
}

pub fn create_xor_network(name: &str) -> NeuralNetwork {
    create_neural_network(Some(name))
        .input("x0")
        .input("x1")
        .constant("bias", 1.0, prop("nn.role", "bias"))
        .weighted_sum("h_or_sum", vec![wi("x0", 20.0, "x0_to_h_or"), wi("x1", 20.0, "x1_to_h_or"), wi("bias", -10.0, "bias_to_h_or")], prop("nn.layer", "hidden"))
        .activation("h_or", "h_or_sum", ActivationKind::Sigmoid, prop("nn.layer", "hidden"), "h_or_sum_to_h_or")
        .weighted_sum("h_nand_sum", vec![wi("x0", -20.0, "x0_to_h_nand"), wi("x1", -20.0, "x1_to_h_nand"), wi("bias", 30.0, "bias_to_h_nand")], prop("nn.layer", "hidden"))
        .activation("h_nand", "h_nand_sum", ActivationKind::Sigmoid, prop("nn.layer", "hidden"), "h_nand_sum_to_h_nand")
        .weighted_sum("out_sum", vec![wi("h_or", 20.0, "h_or_to_out"), wi("h_nand", 20.0, "h_nand_to_out"), wi("bias", -30.0, "bias_to_out")], prop("nn.layer", "output"))
        .activation("out_activation", "out_sum", ActivationKind::Sigmoid, prop("nn.layer", "output"), "out_sum_to_activation")
        .output("out", "out_activation", "prediction", prop("nn.layer", "output"), "activation_to_out")
}

fn prop(key: &str, value: &str) -> PropertyBag {
    let mut bag = PropertyBag::new();
    bag.insert(key.to_string(), PropertyValue::String(value.to_string()));
    bag
}

fn wi(from: &str, weight: f64, edge_id: &str) -> WeightedInput { WeightedInput::new(from, weight, edge_id) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tiny_weighted_graph() {
        let mut graph = create_neural_graph(Some("tiny"));
        add_input(&mut graph, "x0", "x0", PropertyBag::new());
        add_input(&mut graph, "x1", "x1", PropertyBag::new());
        add_constant(&mut graph, "bias", 1.0, PropertyBag::new());
        add_weighted_sum(&mut graph, "sum", vec![wi("x0", 0.25, "x0_to_sum"), wi("x1", 0.75, "x1_to_sum"), wi("bias", -1.0, "bias_to_sum")], PropertyBag::new());
        add_activation(&mut graph, "relu", "sum", ActivationKind::Relu, PropertyBag::new(), Some("sum_to_relu".to_string()));
        add_output(&mut graph, "out", "relu", "prediction", PropertyBag::new(), Some("relu_to_out".to_string()));
        assert_eq!(graph.incoming_edges("sum").len(), 3);
        assert_eq!(graph.topological_sort().unwrap().last().unwrap(), "out");
    }

    #[test]
    fn xor_network_has_hidden_layer_edges() {
        let network = create_xor_network("xor");
        assert_eq!(network.graph.incoming_edges("out_sum").len(), 3);
        assert!(network.graph.edges().iter().any(|edge| edge.id == "h_or_to_out"));
    }
}
