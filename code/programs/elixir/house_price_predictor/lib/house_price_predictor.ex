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

    # --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
    # How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
    # 1. We take our original (N BY 2) Data Grid (X) and physically flip it on its side to become (2 BY N). 
    #    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
    # 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
    #    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
    err_mat = Matrix.subtract(y_pred, y)
    x_t = Matrix.transpose(x)
    dot_err = Matrix.dot(x_t, err_mat)
    
    # We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
    dw = Matrix.scale(dot_err, 2.0 / y.rows)

    # For the Bias (b), because it shifts the prediction unconditionally for every house,
    # its "share" of the blame is simply the average of all the mistakes combined!
    # We take the raw (N BY 1) Error array, sum up the N values, and scale it by 2/N.
    db_total = Enum.map(err_mat.data, fn [v] -> v end) |> Enum.sum()
    db = db_total * (2.0 / y.rows)

    # --- OPTIMIZATION STEP ---
    # Finally, we take our original Weights and Bias and nudge them against the slope.
    # We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't 
    # overshoot the target and cause the math to explode into infinity!
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
