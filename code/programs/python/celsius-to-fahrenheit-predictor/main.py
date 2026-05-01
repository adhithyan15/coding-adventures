from loss_functions import mse, mse_derivative, mae, mae_derivative
from gradient_descent import sgd
from neural_network import WeightedInput, create_neural_network
from neural_graph_vm import compile_neural_network_to_bytecode, run_neural_bytecode_forward

# Target pairs: C -> F
celsius = [-40.0, -10.0, 0.0, 8.0, 15.0, 22.0, 38.0]
fahrenheit = [-40.0, 14.0, 32.0, 46.4, 59.0, 71.6, 100.4]

def train(loss_name, loss_fn, loss_deriv_fn, learning_rate=0.001, max_epochs=10000):
    w = 0.5
    b = 0.5
    print(f"\n--- Celsius to Fahrenheit Predictor: Training with {loss_name} ---")
    for epoch in range(max_epochs):
        y_pred = [(w * c) + b for c in celsius]
        err = loss_fn(fahrenheit, y_pred)
        
        # Stop early when error is sufficiently small
        # (MAE can oscillate slightly around the bare minimum without decay)
        if err < 0.5:
            print(f"Converged beautifully in {epoch+1} epochs! (Loss: {err:.6f})")
            print(f"Final Formula: F = C * {w:.6f} + {b:.6f}")
            break
            
        gradients = loss_deriv_fn(fahrenheit, y_pred)
        
        grad_w = sum(g * c for g, c in zip(gradients, celsius))
        grad_b = sum(gradients)
        
        # Pass parameters to our isolated optimizer mathematically
        w, b = sgd([w, b], [grad_w, grad_b], learning_rate)[0:2]

        if (epoch + 1) % 1000 == 0:
            print(f"Epoch {epoch+1:04d} -> Loss: {err:.6f} | w: {w:.4f} | b: {b:.4f}")
            
    test_c = 100.0
    pred_f = w * test_c + b
    print(f"Prediction for 100.0 C -> {pred_f:.2f} F (Expected ~212.00 F)")
    run_graph_vm_inference(loss_name, w, b, test_c)

    return w, b


def run_graph_vm_inference(loss_name, weight, bias, celsius_value):
    network = (
        create_neural_network(f"celsius-to-fahrenheit-{loss_name}")
        .input("celsius")
        .constant("bias", 1.0, {"nn.role": "bias"})
        .weighted_sum(
            "fahrenheit_sum",
            [
                WeightedInput("celsius", weight, "celsius_weight"),
                WeightedInput("bias", bias, "fahrenheit_bias"),
            ],
            {"nn.layer": "output", "nn.role": "weighted_sum"},
        )
        .activation(
            "fahrenheit_linear",
            "fahrenheit_sum",
            "none",
            {"nn.layer": "output", "nn.role": "identity_activation"},
            "sum_to_identity",
        )
        .output(
            "fahrenheit",
            "fahrenheit_linear",
            "fahrenheit",
            {"nn.layer": "output"},
            "identity_to_output",
        )
    )
    bytecode = compile_neural_network_to_bytecode(network)
    outputs = run_neural_bytecode_forward(bytecode, {"celsius": celsius_value})
    print(
        f"Graph VM path -> {celsius_value:.1f} C = {outputs['fahrenheit']:.2f} F "
        f"({len(bytecode.functions[0].instructions)} bytecode ops)"
    )

if __name__ == "__main__":
    train("Mean Squared Error (MSE)", mse, mse_derivative, learning_rate=0.0005)
    train("Mean Absolute Error (MAE)", mae, mae_derivative, learning_rate=0.01)
