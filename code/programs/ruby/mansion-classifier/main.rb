require_relative "../../../packages/ruby/perceptron/lib/perceptron"
$LOAD_PATH.unshift(File.expand_path("../../../packages/ruby/neural-network/lib", __dir__))
$LOAD_PATH.unshift(File.expand_path("../../../packages/ruby/neural-graph-vm/lib", __dir__))
require "neural_network"
require "neural_graph_vm"

puts "\n--- Booting Ruby Mansion Classifier (OOP V2) ---"

house_data = [
  [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
  [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
]
target_data = [
  [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
]

model = Perceptron_ML::Perceptron.new(0.1, 2000)
model.fit(house_data, target_data, 400)

puts "\n--- Final Inference ---"
predictions = model.predict(house_data)
predictions.each_with_index do |prob, i|
  target = target_data[i][0] == 1.0 ? "Mansion" : "Normal"
  guess = prob > 0.5 ? "Mansion" : "Normal"
  puts "House #{i+1} (Truth: #{target}) -> System: #{guess} (%.2f%%)" % [prob * 100]
end

raise "Expected trained perceptron weights before graph VM inference" if model.weights.nil?

network = NeuralNetwork.create_neural_network("mansion-classifier")
  .input("bedrooms")
  .input("bathrooms")
  .constant("bias", 1.0, "nn.role" => "bias")
  .weighted_sum("mansion_logit", [
    NeuralNetwork.wi("bedrooms", model.weights.data[0][0], "bedrooms_weight"),
    NeuralNetwork.wi("bathrooms", model.weights.data[1][0], "bathrooms_weight"),
    NeuralNetwork.wi("bias", model.bias, "bias_weight")
  ], "nn.layer" => "output", "nn.role" => "weighted_sum")
  .activation("mansion_probability", "mansion_logit", "sigmoid", { "nn.layer" => "output", "nn.role" => "activation" }, "logit_to_sigmoid")
  .output("mansion_output", "mansion_probability", "mansion_probability", { "nn.layer" => "output" }, "probability_to_output")

bytecode = NeuralGraphVM.compile_neural_network_to_bytecode(network)

puts "\n--- Graph VM Inference ---"
house_data.each_with_index do |house, i|
  outputs = NeuralGraphVM.run_neural_bytecode_forward(
    bytecode,
    "bedrooms" => house[0],
    "bathrooms" => house[1]
  )
  probability = outputs.fetch("mansion_probability")
  guess = probability > 0.5 ? "Mansion" : "Normal"
  puts "House #{i+1} -> VM: #{guess} (%.2f%%)" % [probability * 100]
end
