defmodule MansionClassifier do
  def run do
    :io.format("~n--- Booting Elixir Mansion Classifier (OOP V2) ---~n")

    house_data = [
      [4.5, 6.0],
      [3.8, 5.0],
      [1.5, 2.0],
      [0.9, 1.0],
      [5.5, 7.0],
      [2.0, 3.0]
    ]

    target_data = [
      [1.0],
      [1.0],
      [0.0],
      [0.0],
      [1.0],
      [0.0]
    ]

    model = Perceptron.new(0.1, 2000)
    model = Perceptron.fit(model, house_data, target_data, 400)

    :io.format("~n--- Final Inference ---~n")
    predictions = Perceptron.predict(model, house_data)

    Enum.with_index(predictions)
    |> Enum.each(fn {prob, i} ->
      truth = if Enum.at(Enum.at(target_data, i), 0) == 1.0, do: "Mansion", else: "Normal"
      guess = if prob > 0.5, do: "Mansion", else: "Normal"

      :io.format("House ~w (Truth: ~s) -> System: ~s (~.2f%)~n", [i + 1, truth, guess, prob * 100])
    end)

    run_graph_vm_inference(model, house_data)
  end

  defp run_graph_vm_inference(model, house_data) do
    if is_nil(model.weights) do
      raise "Expected trained perceptron weights before graph VM inference"
    end

    network =
      NeuralNetwork.create_neural_network("mansion-classifier")
      |> NeuralNetwork.Network.input("bedrooms")
      |> NeuralNetwork.Network.input("bathrooms")
      |> NeuralNetwork.Network.constant("bias", 1.0, %{"nn.role" => "bias"})
      |> NeuralNetwork.Network.weighted_sum(
        "mansion_logit",
        [
          NeuralNetwork.wi(
            "bedrooms",
            model.weights.data |> Enum.at(0) |> Enum.at(0),
            "bedrooms_weight"
          ),
          NeuralNetwork.wi(
            "bathrooms",
            model.weights.data |> Enum.at(1) |> Enum.at(0),
            "bathrooms_weight"
          ),
          NeuralNetwork.wi("bias", model.bias, "bias_weight")
        ],
        %{"nn.layer" => "output", "nn.role" => "weighted_sum"}
      )
      |> NeuralNetwork.Network.activation(
        "mansion_probability",
        "mansion_logit",
        "sigmoid",
        %{"nn.layer" => "output", "nn.role" => "activation"},
        "logit_to_sigmoid"
      )
      |> NeuralNetwork.Network.output(
        "mansion_output",
        "mansion_probability",
        "mansion_probability",
        %{"nn.layer" => "output"},
        "probability_to_output"
      )

    bytecode = NeuralGraphVM.compile_neural_network_to_bytecode(network)

    :io.format("~n--- Graph VM Inference ---~n")

    Enum.with_index(house_data)
    |> Enum.each(fn {house, i} ->
      outputs =
        NeuralGraphVM.run_neural_bytecode_forward(bytecode, %{
          "bedrooms" => Enum.at(house, 0),
          "bathrooms" => Enum.at(house, 1)
        })

      probability = outputs["mansion_probability"]
      guess = if probability > 0.5, do: "Mansion", else: "Normal"
      :io.format("House ~w -> VM: ~s (~.2f%)~n", [i + 1, guess, probability * 100])
    end)
  end
end
