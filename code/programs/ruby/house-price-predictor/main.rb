# Multi-Variable Linear Regression: House Price Predictor
# -------------------------------------------------------
# Harnessing native Ruby Module boundaries structurally parsing pure Object Orientated Logic
# entirely universally gracefully safely mathematically mapping directly flawlessly!

require_relative '../../../packages/ruby/loss-functions/lib/loss_functions'
require_relative '../../../packages/ruby/matrix/lib/matrix_ml'

puts "\n--- Booting Multi-Variable Predictor: House Prices ---\n"

# 1. Initiating Native Object Topologies dynamically instancing Grid values effortlessly.
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

puts "Beginning Training Epochs..."
1501.times do |epoch|
  # --- FORWARD CALCULUS ---
  # Native operator chaining strictly evaluated via internal matrix structures natively securely.
  pred = x.dot(w)
  y_pred = pred + b

  # MSE Loss Native Evaluation Array Bound Matching smoothly mapped natively dynamically!
  y_true_list = y.data.map { |r| r[0] }
  y_pred_list = y_pred.data.map { |r| r[0] }
  total_loss = LossFunctions.mse(y_true_list, y_pred_list)

  # --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
  # How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
  # 1. We take our original (N BY 2) Data Grid (X) and physically flip it on its side to become (2 BY N). 
  #    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
  # 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
  #    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
  err_mat = y_pred - y
  x_t = x.transpose
  dot_err = x_t.dot(err_mat)
  
  # We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
  dw = dot_err * (2.0 / y.rows)

  # For the Bias (b), because it shifts the prediction unconditionally for every house,
  # its "share" of the blame is simply the average of all the mistakes combined!
  # We take the raw (N BY 1) Error array, sum up the N values, and scale it by 2/N.
  db_total = 0.0
  err_mat.rows.times do |i|
    db_total += err_mat.data[i][0]
  end
  db = db_total * (2.0 / y.rows)

  # --- OPTIMIZATION STEP ---
  # Finally, we take our original Weights and Bias and nudge them against the slope.
  # We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't 
  # overshoot the target and cause the math to explode into infinity!
  w = w - (dw * lr)
  b = b - (db * lr)

  if epoch % 150 == 0
    puts "Epoch %4d | Global Loss: %9.4f | Weights [SqFt: %6.2f, Bed: %6.2f] | Bias: %6.2f" % [epoch, total_loss, w.data[0][0], w.data[1][0], b]
  end
end

puts "\nFinal Optimal Mapping Achieved!"
prediction = (x.dot(w) + b).data[0][0]
puts "Prediction for House 1 (Target $400k): $%.2fk" % prediction
