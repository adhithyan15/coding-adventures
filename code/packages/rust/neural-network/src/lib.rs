pub use multi_directed_graph::{
    GraphError, MultiDirectedEdge as Edge, MultiDirectedGraph, PropertyBag, PropertyValue,
};

pub type NeuralGraph = MultiDirectedGraph<String>;

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

pub struct NeuralNetwork {
    pub graph: NeuralGraph,
}

impl NeuralNetwork {
    pub fn new(name: Option<&str>) -> Self {
        Self {
            graph: create_neural_graph(name),
        }
    }

    pub fn input(mut self, node: &str) -> Self {
        add_input(&mut self.graph, node, node, PropertyBag::new());
        self
    }

    pub fn constant(mut self, node: &str, value: f64, properties: PropertyBag) -> Self {
        add_constant(&mut self.graph, node, value, properties);
        self
    }

    pub fn weighted_sum(
        mut self,
        node: &str,
        inputs: Vec<WeightedInput>,
        properties: PropertyBag,
    ) -> Self {
        add_weighted_sum(&mut self.graph, node, inputs, properties);
        self
    }

    pub fn activation(
        mut self,
        node: &str,
        input: &str,
        activation: ActivationKind,
        properties: PropertyBag,
        edge_id: &str,
    ) -> Self {
        add_activation(
            &mut self.graph,
            node,
            input,
            activation,
            properties,
            Some(edge_id.to_string()),
        );
        self
    }

    pub fn output(
        mut self,
        node: &str,
        input: &str,
        output_name: &str,
        properties: PropertyBag,
        edge_id: &str,
    ) -> Self {
        add_output(
            &mut self.graph,
            node,
            input,
            output_name,
            properties,
            Some(edge_id.to_string()),
        );
        self
    }
}

pub fn create_neural_graph(name: Option<&str>) -> NeuralGraph {
    let mut graph = MultiDirectedGraph::new();
    graph.set_graph_property("nn.version", PropertyValue::String("0".to_string()));
    if let Some(name) = name {
        graph.set_graph_property("nn.name", PropertyValue::String(name.to_string()));
    }
    graph
}

pub fn create_neural_network(name: Option<&str>) -> NeuralNetwork {
    NeuralNetwork::new(name)
}

pub fn add_input(
    graph: &mut NeuralGraph,
    node: &str,
    input_name: &str,
    mut properties: PropertyBag,
) {
    properties.insert(
        "nn.op".to_string(),
        PropertyValue::String("input".to_string()),
    );
    properties.insert(
        "nn.input".to_string(),
        PropertyValue::String(
            if input_name.is_empty() {
                node
            } else {
                input_name
            }
            .to_string(),
        ),
    );
    graph.add_node(node.to_string(), properties);
}

pub fn add_constant(graph: &mut NeuralGraph, node: &str, value: f64, mut properties: PropertyBag) {
    assert!(value.is_finite(), "constant value must be finite");
    properties.insert(
        "nn.op".to_string(),
        PropertyValue::String("constant".to_string()),
    );
    properties.insert("nn.value".to_string(), PropertyValue::Number(value));
    graph.add_node(node.to_string(), properties);
}

pub fn add_weighted_sum(
    graph: &mut NeuralGraph,
    node: &str,
    inputs: Vec<WeightedInput>,
    mut properties: PropertyBag,
) {
    properties.insert(
        "nn.op".to_string(),
        PropertyValue::String("weighted_sum".to_string()),
    );
    graph.add_node(node.to_string(), properties);
    for input in inputs {
        graph
            .add_edge(
                input.from,
                node.to_string(),
                input.weight,
                input.properties,
                input.edge_id,
            )
            .expect("invalid weighted_sum edge");
    }
}

pub fn add_activation(
    graph: &mut NeuralGraph,
    node: &str,
    input: &str,
    activation: ActivationKind,
    mut properties: PropertyBag,
    edge_id: Option<String>,
) -> String {
    properties.insert(
        "nn.op".to_string(),
        PropertyValue::String("activation".to_string()),
    );
    properties.insert(
        "nn.activation".to_string(),
        PropertyValue::String(activation.as_str().to_string()),
    );
    graph.add_node(node.to_string(), properties);
    graph
        .add_edge(
            input.to_string(),
            node.to_string(),
            1.0,
            PropertyBag::new(),
            edge_id,
        )
        .expect("invalid activation edge")
}

pub fn add_output(
    graph: &mut NeuralGraph,
    node: &str,
    input: &str,
    output_name: &str,
    mut properties: PropertyBag,
    edge_id: Option<String>,
) -> String {
    properties.insert(
        "nn.op".to_string(),
        PropertyValue::String("output".to_string()),
    );
    properties.insert(
        "nn.output".to_string(),
        PropertyValue::String(output_name.to_string()),
    );
    graph.add_node(node.to_string(), properties);
    graph
        .add_edge(
            input.to_string(),
            node.to_string(),
            1.0,
            PropertyBag::new(),
            edge_id,
        )
        .expect("invalid output edge")
}

pub fn create_xor_network(name: &str) -> NeuralNetwork {
    create_neural_network(Some(if name.is_empty() { "xor" } else { name }))
        .input("x0")
        .input("x1")
        .constant("bias", 1.0, prop("nn.role", "bias"))
        .weighted_sum(
            "h_or_sum",
            vec![
                wi("x0", 20.0, "x0_to_h_or"),
                wi("x1", 20.0, "x1_to_h_or"),
                wi("bias", -10.0, "bias_to_h_or"),
            ],
            prop("nn.layer", "hidden"),
        )
        .activation(
            "h_or",
            "h_or_sum",
            ActivationKind::Sigmoid,
            prop("nn.layer", "hidden"),
            "h_or_sum_to_h_or",
        )
        .weighted_sum(
            "h_nand_sum",
            vec![
                wi("x0", -20.0, "x0_to_h_nand"),
                wi("x1", -20.0, "x1_to_h_nand"),
                wi("bias", 30.0, "bias_to_h_nand"),
            ],
            prop("nn.layer", "hidden"),
        )
        .activation(
            "h_nand",
            "h_nand_sum",
            ActivationKind::Sigmoid,
            prop("nn.layer", "hidden"),
            "h_nand_sum_to_h_nand",
        )
        .weighted_sum(
            "out_sum",
            vec![
                wi("h_or", 20.0, "h_or_to_out"),
                wi("h_nand", 20.0, "h_nand_to_out"),
                wi("bias", -30.0, "bias_to_out"),
            ],
            prop("nn.layer", "output"),
        )
        .activation(
            "out_activation",
            "out_sum",
            ActivationKind::Sigmoid,
            prop("nn.layer", "output"),
            "out_sum_to_activation",
        )
        .output(
            "out",
            "out_activation",
            "prediction",
            prop("nn.layer", "output"),
            "activation_to_out",
        )
}

fn prop(key: &str, value: &str) -> PropertyBag {
    let mut bag = PropertyBag::new();
    bag.insert(key.to_string(), PropertyValue::String(value.to_string()));
    bag
}

fn wi(from: &str, weight: f64, edge_id: &str) -> WeightedInput {
    WeightedInput::new(from, weight, edge_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_prop<'a>(bag: &'a PropertyBag, key: &str) -> &'a str {
        match bag.get(key) {
            Some(PropertyValue::String(value)) => value,
            other => panic!("expected string property {key}, got {other:?}"),
        }
    }

    #[test]
    fn creates_neural_graph_metadata_on_generic_graph() {
        let graph = create_neural_graph(Some("tiny"));
        assert_eq!(string_prop(&graph.graph_properties(), "nn.version"), "0");
        assert_eq!(string_prop(&graph.graph_properties(), "nn.name"), "tiny");
    }

    #[test]
    fn builds_tiny_weighted_graph() {
        let mut graph = create_neural_graph(Some("tiny"));
        add_input(&mut graph, "x0", "x0", PropertyBag::new());
        add_input(&mut graph, "x1", "x1", PropertyBag::new());
        add_constant(&mut graph, "bias", 1.0, PropertyBag::new());
        add_weighted_sum(
            &mut graph,
            "sum",
            vec![
                wi("x0", 0.25, "x0_to_sum"),
                wi("x1", 0.75, "x1_to_sum"),
                wi("bias", -1.0, "bias_to_sum"),
            ],
            PropertyBag::new(),
        );
        add_activation(
            &mut graph,
            "relu",
            "sum",
            ActivationKind::Relu,
            PropertyBag::new(),
            Some("sum_to_relu".to_string()),
        );
        add_output(
            &mut graph,
            "out",
            "relu",
            "prediction",
            PropertyBag::new(),
            Some("relu_to_out".to_string()),
        );
        assert_eq!(graph.incoming_edges(&"sum".to_string()).unwrap().len(), 3);
        assert_eq!(graph.topological_sort().unwrap().last().unwrap(), "out");
        assert_eq!(
            graph.edge_properties("x0_to_sum").unwrap()["weight"],
            PropertyValue::Number(0.25)
        );
    }

    #[test]
    fn xor_network_has_hidden_layer_edges() {
        let network = create_xor_network("xor");
        assert_eq!(
            network
                .graph
                .incoming_edges(&"out_sum".to_string())
                .unwrap()
                .len(),
            3
        );
        assert!(network
            .graph
            .edges()
            .iter()
            .any(|edge| edge.id == "h_or_to_out"));
    }
}
