defmodule MansionClassifier do
  def run do
    :io.format("~n--- Booting Elixir Mansion Classifier (OOP V2) ---~n")

    house_data = [
      [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
      [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
    ]
    target_data = [
      [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
    ]

    model = Perceptron.new(0.1, 2000)
    model = Perceptron.fit(model, house_data, target_data, 400)

    :io.format("~n--- Final Inference ---~n")
    predictions = Perceptron.predict(model, house_data)
    
    Enum.with_index(predictions) |> Enum.each(fn {prob, i} ->
      truth = if Enum.at(Enum.at(target_data, i), 0) == 1.0, do: "Mansion", else: "Normal"
      guess = if prob > 0.5, do: "Mansion", else: "Normal"
      :io.format("House ~w (Truth: ~s) -> System: ~s (~.2f%)~n", [i + 1, truth, guess, prob * 100])
    end)
  end
end
