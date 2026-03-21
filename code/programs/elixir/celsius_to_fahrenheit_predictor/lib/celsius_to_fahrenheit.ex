defmodule CodingAdventures.CelsiusToFahrenheit do
  alias CodingAdventures.LossFunctions, as: LF
  alias CodingAdventures.GradientDescent, as: GD

  @celsius [-40.0, -10.0, 0.0, 8.0, 15.0, 22.0, 38.0]
  @fahrenheit [-40.0, 14.0, 32.0, 46.4, 59.0, 71.6, 100.4]

  def train(loss_name, loss_fn, deriv_fn, lr, max_epochs \\ 10000) do
    IO.puts("\n--- Celsius to Fahrenheit Predictor: Training with #{loss_name} ---")
    train_loop(0, 0.5, 0.5, loss_fn, deriv_fn, lr, max_epochs)
  end

  defp train_loop(epoch, w, b, _loss_fn, _deriv_fn, _lr, max_epochs) when epoch >= max_epochs do
    print_final(w, b)
  end

  defp train_loop(epoch, w, b, loss_fn, deriv_fn, lr, max_epochs) do
    y_pred = Enum.map(@celsius, fn c -> w * c + b end)
    err = apply(LF, loss_fn, [@fahrenheit, y_pred])

    if err < 0.5 do
      IO.puts("Converged beautifully in #{epoch + 1} epochs! (Loss: #{Float.round(err, 6)})")
      IO.puts("Final Formula: F = C * #{Float.round(w, 6)} + #{Float.round(b, 6)}")
      print_final(w, b)
    else
      gradients = apply(LF, deriv_fn, [@fahrenheit, y_pred])
      
      grad_w = Enum.zip(gradients, @celsius) |> Enum.map(fn {g, c} -> g * c end) |> Enum.sum()
      grad_b = Enum.sum(gradients)

      [new_w, new_b] = GD.sgd([w, b], [grad_w, grad_b], lr)

      if rem(epoch + 1, 1000) == 0 do
        IO.puts("Epoch #{String.pad_leading(Integer.to_string(epoch + 1), 4, "0")} -> Loss: #{Float.round(err, 6)} | w: #{Float.round(new_w, 4)} | b: #{Float.round(new_b, 4)}")
      end

      train_loop(epoch + 1, new_w, new_b, loss_fn, deriv_fn, lr, max_epochs)
    end
  end

  defp print_final(w, b) do
    pred_f = w * 100.0 + b
    IO.puts("Prediction for 100.0 C -> #{Float.round(pred_f, 2)} F (Expected ~212.00 F)")
  end

  def run do
    train("Mean Squared Error (MSE)", :mse, :mse_derivative, 0.0005)
    train("Mean Absolute Error (MAE)", :mae, :mae_derivative, 0.01)
  end
end
