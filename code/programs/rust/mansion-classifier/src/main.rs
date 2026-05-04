use neural_graph_vm::{compile_neural_network_to_bytecode, run_neural_bytecode_forward};
use neural_network::{
    create_neural_network, ActivationKind, PropertyBag, PropertyValue, WeightedInput,
};
use perceptron::Perceptron;
use std::collections::HashMap;

fn main() {
    println!("\n--- Booting Rust Mansion Classifier (OOP V2) ---");

    let house_data = vec![
        vec![4.5, 6.0],
        vec![3.8, 5.0],
        vec![1.5, 2.0],
        vec![0.9, 1.0],
        vec![5.5, 7.0],
        vec![2.0, 3.0],
    ];
    let target_data = vec![
        vec![1.0],
        vec![1.0],
        vec![0.0],
        vec![0.0],
        vec![1.0],
        vec![0.0],
    ];

    let mut model = Perceptron::new(0.1, 2000);
    model.fit(house_data.clone(), target_data.clone(), 400);

    println!("\n--- Final Inference ---");
    let predictions = model.predict(house_data.clone());
    for (i, prob) in predictions.iter().enumerate() {
        let truth = if target_data[i][0] == 1.0 {
            "Mansion"
        } else {
            "Normal"
        };
        let guess = if *prob > 0.5 { "Mansion" } else { "Normal" };
        println!(
            "House {} (Truth: {}) -> System: {} ({:.2}%)",
            i + 1,
            truth,
            guess,
            prob * 100.0
        );
    }

    run_graph_vm_inference(&model, &house_data);
}

fn run_graph_vm_inference(model: &Perceptron, house_data: &[Vec<f64>]) {
    let weights = model
        .weights
        .as_ref()
        .expect("expected trained perceptron weights before graph VM inference");

    let mut bias_props = PropertyBag::new();
    bias_props.insert(
        "nn.role".to_string(),
        PropertyValue::String("bias".to_string()),
    );

    let mut sum_props = PropertyBag::new();
    sum_props.insert(
        "nn.layer".to_string(),
        PropertyValue::String("output".to_string()),
    );
    sum_props.insert(
        "nn.role".to_string(),
        PropertyValue::String("weighted_sum".to_string()),
    );

    let mut activation_props = PropertyBag::new();
    activation_props.insert(
        "nn.layer".to_string(),
        PropertyValue::String("output".to_string()),
    );
    activation_props.insert(
        "nn.role".to_string(),
        PropertyValue::String("activation".to_string()),
    );

    let mut output_props = PropertyBag::new();
    output_props.insert(
        "nn.layer".to_string(),
        PropertyValue::String("output".to_string()),
    );

    let network = create_neural_network(Some("mansion-classifier"))
        .input("bedrooms")
        .input("bathrooms")
        .constant("bias", 1.0, bias_props)
        .weighted_sum(
            "mansion_logit",
            vec![
                WeightedInput::new("bedrooms", weights.data[0][0], "bedrooms_weight"),
                WeightedInput::new("bathrooms", weights.data[1][0], "bathrooms_weight"),
                WeightedInput::new("bias", model.bias, "bias_weight"),
            ],
            sum_props,
        )
        .activation(
            "mansion_probability",
            "mansion_logit",
            ActivationKind::Sigmoid,
            activation_props,
            "logit_to_sigmoid",
        )
        .output(
            "mansion_output",
            "mansion_probability",
            "mansion_probability",
            output_props,
            "probability_to_output",
        );

    let bytecode = compile_neural_network_to_bytecode(&network).unwrap();
    println!("\n--- Graph VM Inference ---");
    for (index, house) in house_data.iter().enumerate() {
        let mut inputs = HashMap::new();
        inputs.insert("bedrooms".to_string(), house[0]);
        inputs.insert("bathrooms".to_string(), house[1]);
        let outputs = run_neural_bytecode_forward(&bytecode, &inputs).unwrap();
        let probability = outputs["mansion_probability"];
        let guess = if probability > 0.5 {
            "Mansion"
        } else {
            "Normal"
        };
        println!(
            "House {} -> VM: {} ({:.2}%)",
            index + 1,
            guess,
            probability * 100.0
        );
    }
}
