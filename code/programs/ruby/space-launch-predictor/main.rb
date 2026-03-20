require_relative "../../../packages/ruby/loss-functions/lib/loss_functions"
require_relative "../../../packages/ruby/activation-functions/lib/activation_functions"
require_relative "../../../packages/ruby/matrix/lib/matrix_ml"

puts "\n--- Booting Ruby Space Launch Predictor ---"

shuttle_data = [
  [12.0, 15.0], [35.0, 85.0], [5.0, 5.0],
  [40.0, 95.0], [15.0, 30.0], [28.0, 60.0]
]
target_data = [
  [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
]

features = Matrix.new(shuttle_data)
true_labels = Matrix.new(target_data)

weights = Matrix.new([[0.0], [0.0]])
bias = 0.0
lr = 0.01
epochs = 3000

(0..epochs).each do |epoch|
  raw = features.dot(weights)
  raw = raw + bias

  probs = raw.data.map { |r| [ActivationFunctions.sigmoid(r[0])] }
  linear_truth = target_data.map { |r| r[0] }
  linear_probs = probs.map { |r| r[0] }

  log_loss = LossFunctions.bce(linear_truth, linear_probs)
  loss_grad = LossFunctions.bce_derivative(linear_truth, linear_probs)

  grad_data = []
  bias_grad = 0.0
  features.rows.times do |i|
    act_grad = ActivationFunctions.sigmoid_derivative(raw.data[i][0])
    combined = loss_grad[i] * act_grad
    grad_data << [combined]
    bias_grad += combined
  end
  
  grad_matrix = Matrix.new(grad_data)
  weight_grads = features.transpose.dot(grad_matrix)

  scaled_weights = weight_grads * lr
  weights = weights - scaled_weights
  bias -= bias_grad * lr

  puts "Epoch %4d | BCE Loss: %.4f | Bias: %.2f" % [epoch, log_loss, bias] if epoch % 500 == 0
end

puts "\n--- Final Inference ---"
final_raw = features.dot(weights)
final_raw = final_raw + bias
true_labels.rows.times do |i|
  prob = ActivationFunctions.sigmoid(final_raw.data[i][0])
  target = target_data[i][0] == 1.0 ? "Safe" : "Abort"
  guess = prob > 0.5 ? "Safe" : "Abort"
  puts "Scenario #{i+1} (Truth: #{target}) -> System: #{guess} (%.2f%%)" % [prob * 100]
end
