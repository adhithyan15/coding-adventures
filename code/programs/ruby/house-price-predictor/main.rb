# Multi-Variable Linear Regression: House Price Predictor
# -------------------------------------------------------
# Demonstrates n input features -> one output with feature normalization and a
# short learning-rate sweep before the full training run.

require_relative '../../../packages/ruby/feature_normalization/lib/coding_adventures_feature_normalization'
require_relative '../../../packages/ruby/loss-functions/lib/loss_functions'
require_relative '../../../packages/ruby/matrix/lib/matrix_ml'

HOUSE_FEATURES_DATA = [
  [2000.0, 3.0],
  [1500.0, 2.0],
  [2500.0, 4.0],
  [1000.0, 1.0]
].freeze

TRUE_PRICES_DATA = [
  [400.0],
  [300.0],
  [500.0],
  [200.0]
].freeze

def run_training(features_data, prices_data, learning_rate, epochs, log_every: nil)
  house_features = Matrix.new(features_data)
  true_prices = Matrix.new(prices_data)
  feature_weights = Matrix.new([[0.5], [0.5]])
  base_price_bias = 0.0
  last_loss = Float::INFINITY

  (0..epochs).each do |epoch|
    final_predictions = house_features.dot(feature_weights) + base_price_bias
    linear_true_prices = true_prices.data.map { |row| row[0] }
    linear_predictions = final_predictions.data.map { |row| row[0] }
    last_loss = LossFunctions.mse(linear_true_prices, linear_predictions)

    if !last_loss.finite? || last_loss > 1.0e12
      return {
        learning_rate: learning_rate,
        loss: Float::INFINITY,
        diverged: true,
        weights: feature_weights,
        bias: base_price_bias
      }
    end

    if log_every && (epoch % log_every).zero?
      puts 'Epoch %4d | Loss: %10.4f | Weights [SqFt: %7.3f, Beds: %7.3f] | Bias: %7.3f' %
           [epoch, last_loss, feature_weights.data[0][0], feature_weights.data[1][0], base_price_bias]
    end

    prediction_errors = final_predictions - true_prices
    weight_gradients = house_features.transpose.dot(prediction_errors) * (2.0 / true_prices.rows)

    bias_gradient_total = prediction_errors.data.sum { |row| row[0] }
    bias_gradient = bias_gradient_total * (2.0 / true_prices.rows)

    feature_weights = feature_weights - (weight_gradients * learning_rate)
    base_price_bias -= bias_gradient * learning_rate
  end

  {
    learning_rate: learning_rate,
    loss: last_loss,
    diverged: false,
    weights: feature_weights,
    bias: base_price_bias
  }
end

def find_learning_rate(features_data, prices_data)
  candidates = [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.6]
  results = candidates.map { |learning_rate| run_training(features_data, prices_data, learning_rate, 120) }

  puts "\nShort learning-rate sweep over normalized features:"
  results.each do |result|
    loss_text = result[:diverged] ? 'diverged' : format('%.4f', result[:loss])
    puts "  lr=#{result[:learning_rate].to_s.ljust(6)} -> loss=#{loss_text}"
  end

  results.reject { |result| result[:diverged] }.min_by { |result| result[:loss] }
end

puts "\n--- Booting Multi-Variable Predictor: House Prices ---"
puts 'Features: square footage and bedroom count. Target: price in $1000s.'

scaler = CodingAdventures::FeatureNormalization.fit_standard_scaler(HOUSE_FEATURES_DATA)
normalized_features = CodingAdventures::FeatureNormalization.transform_standard(HOUSE_FEATURES_DATA, scaler)
best_trial = find_learning_rate(normalized_features, TRUE_PRICES_DATA)

puts "\nSelected learning rate: #{best_trial[:learning_rate]}"
puts 'Beginning full training run...'
final_result = run_training(
  normalized_features,
  TRUE_PRICES_DATA,
  best_trial[:learning_rate],
  1500,
  log_every: 150
)

puts "\nFinal Optimal Mapping Achieved!"
normalized_test_house = CodingAdventures::FeatureNormalization.transform_standard([[2000.0, 3.0]], scaler)
prediction = (Matrix.new(normalized_test_house).dot(final_result[:weights]) + final_result[:bias]).data[0][0]
puts 'Prediction for House 1 (Target $400k): $%.2fk' % prediction
