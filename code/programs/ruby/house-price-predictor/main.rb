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

  # --- BACKPROPAGATION ARCHITECTURE --- 
  # Native Tensor Gradients purely executing matrix structural properties beautifully: dW = X^T . (y_pred - y) * (2/N)
  err_mat = y_pred - y
  x_t = x.transpose
  dot_err = x_t.dot(err_mat)
  dw = dot_err * (2.0 / y.rows)

  db_total = 0.0
  err_mat.rows.times do |i|
    db_total += err_mat.data[i][0]
  end
  db = db_total * (2.0 / y.rows)

  # Weights update applying dynamic gradient mapping natively iteratively smoothly effectively!
  w = w - (dw * lr)
  b = b - (db * lr)

  if epoch % 150 == 0
    puts "Epoch %4d | Global Loss: %9.4f | Weights [SqFt: %6.2f, Bed: %6.2f] | Bias: %6.2f" % [epoch, total_loss, w.data[0][0], w.data[1][0], b]
  end
end

puts "\nFinal Optimal Mapping Achieved!"
prediction = (x.dot(w) + b).data[0][0]
puts "Prediction for House 1 (Target $400k): $%.2fk" % prediction
