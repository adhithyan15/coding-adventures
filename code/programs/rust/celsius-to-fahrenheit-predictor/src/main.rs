use gradient_descent::sgd;
use loss_functions::{mae, mae_derivative, mse, mse_derivative};
use neural_graph_vm::{compile_neural_network_to_bytecode, run_neural_bytecode_forward};
use neural_network::{
    create_neural_network, ActivationKind, PropertyBag, PropertyValue, WeightedInput,
};
use std::collections::HashMap;

fn train(
    loss_name: &str,
    loss_fn: fn(&[f64], &[f64]) -> Result<f64, &'static str>,
    deriv_fn: fn(&[f64], &[f64]) -> Result<Vec<f64>, &'static str>,
    learning_rate: f64,
    epochs: usize,
) {
    let celsius = &[-40.0, -10.0, 0.0, 8.0, 15.0, 22.0, 38.0];
    let fahrenheit = &[-40.0, 14.0, 32.0, 46.4, 59.0, 71.6, 100.4];

    let mut w = 0.5;
    let mut b = 0.5;

    println!(
        "\n--- Celsius to Fahrenheit Predictor: Training with {} ---",
        loss_name
    );

    for epoch in 0..epochs {
        let mut y_pred = Vec::with_capacity(celsius.len());
        for c in celsius {
            y_pred.push(w * c + b);
        }

        let err = loss_fn(fahrenheit, &y_pred).unwrap();

        if err < 0.5 {
            println!(
                "Converged beautifully in {} epochs! (Loss: {:.6})",
                epoch + 1,
                err
            );
            println!("Final Formula: F = C * {:.6} + {:.6}", w, b);
            break;
        }

        let gradients = deriv_fn(fahrenheit, &y_pred).unwrap();

        let mut grad_w = 0.0;
        let mut grad_b = 0.0;
        for i in 0..gradients.len() {
            grad_w += gradients[i] * celsius[i];
            grad_b += gradients[i];
        }

        let new_params = sgd(&[w, b], &[grad_w, grad_b], learning_rate).unwrap();
        w = new_params[0];
        b = new_params[1];

        if (epoch + 1) % 1000 == 0 {
            println!(
                "Epoch {:04} -> Loss: {:.6} | w: {:.4} | b: {:.4}",
                epoch + 1,
                err,
                w,
                b
            );
        }
    }

    let pred_f = w * 100.0 + b;
    println!(
        "Prediction for 100.0 C -> {:.2} F (Expected ~212.00 F)",
        pred_f
    );
    run_graph_vm_inference(loss_name, w, b, 100.0);
}

fn run_graph_vm_inference(loss_name: &str, weight: f64, bias: f64, celsius_value: f64) {
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
        PropertyValue::String("identity_activation".to_string()),
    );

    let mut output_props = PropertyBag::new();
    output_props.insert(
        "nn.layer".to_string(),
        PropertyValue::String("output".to_string()),
    );

    let network = create_neural_network(Some(&format!("celsius-to-fahrenheit-{loss_name}")))
        .input("celsius")
        .constant("bias", 1.0, bias_props)
        .weighted_sum(
            "fahrenheit_sum",
            vec![
                WeightedInput::new("celsius", weight, "celsius_weight"),
                WeightedInput::new("bias", bias, "fahrenheit_bias"),
            ],
            sum_props,
        )
        .activation(
            "fahrenheit_linear",
            "fahrenheit_sum",
            ActivationKind::None,
            activation_props,
            "sum_to_identity",
        )
        .output(
            "fahrenheit",
            "fahrenheit_linear",
            "fahrenheit",
            output_props,
            "identity_to_output",
        );

    let bytecode = compile_neural_network_to_bytecode(&network).unwrap();
    let mut inputs = HashMap::new();
    inputs.insert("celsius".to_string(), celsius_value);
    let outputs = run_neural_bytecode_forward(&bytecode, &inputs).unwrap();
    println!(
        "Graph VM path -> {:.1} C = {:.2} F ({} bytecode ops)",
        celsius_value,
        outputs["fahrenheit"],
        bytecode.functions[0].instructions.len()
    );
}

fn main() {
    train(
        "Mean Squared Error (MSE)",
        mse,
        mse_derivative,
        0.0005,
        10000,
    );
    train(
        "Mean Absolute Error (MAE)",
        mae,
        mae_derivative,
        0.01,
        10000,
    );
}
