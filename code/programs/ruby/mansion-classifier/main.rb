require_relative "../../../packages/ruby/loss-functions/lib/loss_functions"
require_relative "../../../packages/ruby/activation-functions/lib/activation_functions"
require_relative "../../../packages/ruby/matrix/lib/matrix_ml"

puts "\n--- Booting Ruby Mansion Classifier ---"

house_data = [
  [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
  [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
]
target_data = [
  [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
]

features = Matrix.new(house_data)
true_labels = Matrix.new(target_data)

weights = Matrix.new([[0.0], [0.0]])
bias = 0.0
lr = 0.1
epochs = 2000

(0..epochs).each do |epoch|
  raw = features.dot(weights)
  raw = raw + bias

  probs = raw.data.map { |r| [ActivationFunctions.sigmoid(r[0])] }
  prob_matrix = Matrix.new(probs)

  linear_truth = true_labels.data.map { |r| r[0] }
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

  puts "Epoch %4d | BCE Loss: %.4f | Bias: %.2f" % [epoch, log_loss, bias] if epoch % 400 == 0
end

puts "\n--- Final Inference ---"
final_raw = features.dot(weights)
final_raw = final_raw + bias
true_labels.rows.times do |i|
  prob = ActivationFunctions.sigmoid(final_raw.data[i][0])
  target = target_data[i][0] == 1.0 ? "Mansion" : "Normal"
  guess = prob > 0.5 ? "Mansion" : "Normal"
  puts "House #{i+1} (Truth: #{target}) -> System: #{guess} (%.2f%%)" % [prob * 100]
end
