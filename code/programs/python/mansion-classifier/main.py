from perceptron import Perceptron
from neural_network import WeightedInput, create_neural_network
from neural_graph_vm import compile_neural_network_to_bytecode, run_neural_bytecode_forward

def main():
    print("\n--- Booting Python Mansion Classifier (OOP V2) ---")
    house_data = [
        [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
        [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
    ]
    target_data = [
        [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
    ]

    model = Perceptron(learning_rate=0.1, epochs=2000)
    model.fit(house_data, target_data, log_steps=400)

    print("\n--- Final Probability Inferences ---")
    predictions = model.predict(house_data)
    for i, prob in enumerate(predictions):
        truth = "Mansion" if target_data[i][0] == 1.0 else "Normal"
        guess = "Mansion" if prob > 0.5 else "Normal"
        print(f"House {i+1} (Truth: {truth}) -> System: {guess} ({prob*100:.2f}%)")

    run_graph_vm_inference(model, house_data)


def run_graph_vm_inference(model, house_data):
    if model.weights is None:
        raise ValueError("Expected trained perceptron weights before graph VM inference")

    network = (
        create_neural_network("mansion-classifier")
        .input("bedrooms")
        .input("bathrooms")
        .constant("bias", 1.0, {"nn.role": "bias"})
        .weighted_sum(
            "mansion_logit",
            [
                WeightedInput("bedrooms", model.weights.data[0][0], "bedrooms_weight"),
                WeightedInput("bathrooms", model.weights.data[1][0], "bathrooms_weight"),
                WeightedInput("bias", model.bias, "bias_weight"),
            ],
            {"nn.layer": "output", "nn.role": "weighted_sum"},
        )
        .activation(
            "mansion_probability",
            "mansion_logit",
            "sigmoid",
            {"nn.layer": "output", "nn.role": "activation"},
            "logit_to_sigmoid",
        )
        .output(
            "mansion_output",
            "mansion_probability",
            "mansion_probability",
            {"nn.layer": "output"},
            "probability_to_output",
        )
    )
    bytecode = compile_neural_network_to_bytecode(network)

    print("\n--- Graph VM Inference ---")
    for index, house in enumerate(house_data):
        outputs = run_neural_bytecode_forward(
            bytecode,
            {"bedrooms": house[0], "bathrooms": house[1]},
        )
        probability = outputs["mansion_probability"]
        guess = "Mansion" if probability > 0.5 else "Normal"
        print(f"House {index+1} -> VM: {guess} ({probability*100:.2f}%)")

if __name__ == "__main__":
    main()
