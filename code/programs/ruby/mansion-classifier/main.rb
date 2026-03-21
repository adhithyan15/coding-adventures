require_relative "../../../packages/ruby/perceptron/lib/perceptron"

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
