defmodule SpaceLaunchPredictor do
  def run do
    :io.format("~n--- Booting Elixir Space Launch Predictor (OOP V2) ---~n")

    shuttle_data = [
      [12.0, 15.0], [35.0, 85.0], [5.0, 5.0],
      [40.0, 95.0], [15.0, 30.0], [28.0, 60.0]
    ]
    target_data = [
      [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
    ]

    model = Perceptron.new(0.01, 3000)
    model = Perceptron.fit(model, shuttle_data, target_data, 500)

    :io.format("~n--- Final Inference ---~n")
    predictions = Perceptron.predict(model, shuttle_data)
    
    Enum.with_index(predictions) |> Enum.each(fn {prob, i} ->
      truth = if Enum.at(Enum.at(target_data, i), 0) == 1.0, do: "Safe", else: "Abort"
      guess = if prob > 0.5, do: "Safe", else: "Abort"
      :io.format("Scenario ~w (Truth: ~s) -> System: ~s (~.2f%)~n", [i + 1, truth, guess, prob * 100])
    end)
  end
end
