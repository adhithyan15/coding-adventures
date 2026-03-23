require_relative "../../../packages/ruby/perceptron/lib/perceptron"

puts "\n--- Booting Ruby Space Launch Predictor (OOP V2) ---"

shuttle_data = [
  [12.0, 15.0], [35.0, 85.0], [5.0, 5.0],
  [40.0, 95.0], [15.0, 30.0], [28.0, 60.0]
]
target_data = [
  [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
]

model = Perceptron_ML::Perceptron.new(0.01, 3000)
model.fit(shuttle_data, target_data, 500)

puts "\n--- Final Inference ---"
predictions = model.predict(shuttle_data)
predictions.each_with_index do |prob, i|
  truth = target_data[i][0] == 1.0 ? "Safe" : "Abort"
  guess = prob > 0.5 ? "Safe" : "Abort"
  puts "Scenario #{i+1} (Truth: #{truth}) -> System: #{guess} (%.2f%%)" % [prob * 100]
end
