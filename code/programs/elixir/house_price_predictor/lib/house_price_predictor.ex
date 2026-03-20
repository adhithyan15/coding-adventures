# Multi-Variable Linear Regression: House Price Predictor
# -------------------------------------------------------
# Harnesses functional pipeline states sequentially traversing native module mappings recursively
# smoothly effectively cleanly beautifully universally dynamically locally safely purely accurately!

defmodule HousePricePredictor do
  def run do
    IO.puts("\n--- Booting Multi-Variable Predictor: House Prices ---\n")

    # Structuring matrix grids logically natively seamlessly strictly directly structurally.
    x = Matrix.new([
      [2.0, 3.0],
      [1.5, 2.0],
      [2.5, 4.0],
      [1.0, 1.0]
    ])

    y = Matrix.new([
      [400.0],
      [300.0],
      [500.0],
      [200.0]
    ])

    w = Matrix.new([[0.5], [0.5]])
    b = 0.5
    lr = 0.01

    IO.puts("Beginning Training Epochs...")
    train(x, y, w, b, lr, 0)
  end

  defp train(_x, _y, _w, _b, _lr, epoch) when epoch > 1500, do: :ok
  defp train(x, y, w, b, lr, epoch) do
    
    # --- PURE FUNCTIONAL FORWARD MATRIX EVALUATION ---
    # Generating matrix arrays safely traversing memory without mutation functionally sequentially.
    pred = Matrix.dot(x, w)
    y_pred = Matrix.add_scalar(pred, b)

    y_true_list = Enum.map(y.data, fn [v] -> v end)
    y_pred_list = Enum.map(y_pred.data, fn [v] -> v end)
    total_loss = CodingAdventures.LossFunctions.mse(y_true_list, y_pred_list)

    # --- DERIVATIVE BACKWARDS INVERSION PIPELINE ---
    # Binding the native mathematical vectors precisely evaluating matrix states globally sequentially effectively.
    err_mat = Matrix.subtract(y_pred, y)
    x_t = Matrix.transpose(x)
    dot_err = Matrix.dot(x_t, err_mat)
    dw = Matrix.scale(dot_err, 2.0 / y.rows)

    db_total = Enum.map(err_mat.data, fn [v] -> v end) |> Enum.sum()
    db = db_total * (2.0 / y.rows)

    # --- TAIL RECURSION ---
    # Structurally binding new memory paths continuously dynamically flawlessly functionally explicitly.
    w_new = Matrix.subtract(w, Matrix.scale(dw, lr))
    b_new = b - (db * lr)

    if rem(epoch, 150) == 0 do
      [[w1], [w2]] = w_new.data
      IO.puts(:io_lib.format("Epoch ~4w | Global Loss: ~9.4f | Weights [SqFt: ~5.2f, Bed: ~5.2f] | Bias: ~5.2f", [epoch, total_loss, w1, w2, b_new]))
    end
    
    if epoch == 1500 do
        IO.puts("\nFinal Optimal Mapping Achieved!")
        prediction = Matrix.add_scalar(Matrix.dot(x, w_new), b_new).data |> hd() |> hd()
        IO.puts(:io_lib.format("Prediction for House 1 (Target $400k): $~.2fk", [prediction]))
    end

    train(x, y, w_new, b_new, lr, epoch + 1)
  end
end
