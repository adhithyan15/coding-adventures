# Multi-Variable Linear Regression: House Price Predictor
# -------------------------------------------------------
# Harnessing native Ruby Module boundaries structurally parsing pure Object Orientated Logic
# using deeply literate programming variables reflecting exact physical representations natively!

require_relative '../../../packages/ruby/loss-functions/lib/loss_functions'
require_relative '../../../packages/ruby/matrix/lib/matrix_ml'

puts "\n--- Booting Multi-Variable Predictor: House Prices ---\n"

# 1. Initiating Native Object Topologies
house_features = Matrix.new([
  [2.0, 3.0],
  [1.5, 2.0],
  [2.5, 4.0],
  [1.0, 1.0]
])

true_prices = Matrix.new([
  [400.0],
  [300.0],
  [500.0],
  [200.0]
])

feature_weights = Matrix.new([[0.5], [0.5]])
base_price_bias = 0.5
learning_rate = 0.01

puts "Beginning Training Epochs..."
1501.times do |epoch|
  # --- FORWARD CALCULUS ---
  final_predictions = house_features.dot(feature_weights) + base_price_bias

  # MSE Loss Native Evaluation Array Bound Mapping
  linear_true_prices = true_prices.data.map { |r| r[0] }
  linear_predictions = final_predictions.data.map { |r| r[0] }
  mean_squared_error = LossFunctions.mse(linear_true_prices, linear_predictions)

  # --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
  # How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
  # 1. We take our original (N BY 2) Data Grid and physically flip it on its side to become (2 BY N). 
  #    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
  # 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
  #    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
  prediction_errors = final_predictions - true_prices
  transposed_features = house_features.transpose
  features_dot_errors = transposed_features.dot(prediction_errors)
  
  # We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
  weight_gradients = features_dot_errors * (2.0 / true_prices.rows)

  # For the Bias, because it shifts the prediction unconditionally for every house,
  # its "share" of the blame is simply the average of all the mistakes combined!
  bias_gradient_total = 0.0
  prediction_errors.rows.times do |i|
    bias_gradient_total += prediction_errors.data[i][0]
  end
  bias_gradient = bias_gradient_total * (2.0 / true_prices.rows)

  # --- OPTIMIZATION STEP ---
  # Finally, we take our original Weights and Bias and nudge them against the slope.
  # We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't explode to infinity.
  feature_weights = feature_weights - (weight_gradients * learning_rate)
  base_price_bias = base_price_bias - (bias_gradient * learning_rate)

  if epoch % 150 == 0
    puts "Epoch %4d | Global Loss: %9.4f | Weights [SqFt: %6.2f, Bed: %6.2f] | Bias: %6.2f" % [epoch, mean_squared_error, feature_weights.data[0][0], feature_weights.data[1][0], base_price_bias]
  end
end

puts "\nFinal Optimal Mapping Achieved!"
prediction = (house_features.dot(feature_weights) + base_price_bias).data[0][0]
puts "Prediction for House 1 (Target $400k): $%.2fk" % prediction
