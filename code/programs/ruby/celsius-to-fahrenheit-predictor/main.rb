require_relative '../../../packages/ruby/loss-functions/lib/loss_functions'
require_relative '../../../packages/ruby/gradient-descent/lib/gradient_descent'

CELSIUS = [-40.0, -10.0, 0.0, 8.0, 15.0, 22.0, 38.0]
FAHRENHEIT = [-40.0, 14.0, 32.0, 46.4, 59.0, 71.6, 100.4]

def train(loss_name, loss_method, deriv_method, learning_rate, max_epochs = 10000)
  w = 0.5
  b = 0.5
  puts "\n--- Celsius to Fahrenheit Predictor: Training with #{loss_name} ---"

  max_epochs.times do |epoch|
    y_pred = CELSIUS.map { |c| w * c + b }
    
    err = LossFunctions.send(loss_method, FAHRENHEIT, y_pred)
    
    if err < 0.5
      puts "Converged beautifully in #{epoch + 1} epochs! (Loss: #{err.round(6)})"
      puts "Final Formula: F = C * #{w.round(6)} + #{b.round(6)}"
      break
    end

    gradients = LossFunctions.send(deriv_method, FAHRENHEIT, y_pred)
    grad_w = gradients.zip(CELSIUS).map { |g, c| g * c }.sum
    grad_b = gradients.sum

    new_params = GradientDescent.sgd([w, b], [grad_w, grad_b], learning_rate)
    w, b = new_params

    if (epoch + 1) % 1000 == 0
      puts "Epoch #{(epoch + 1).to_s.rjust(4, '0')} -> Loss: #{err.round(6)} | w: #{w.round(4)} | b: #{b.round(4)}"
    end
  end

  pred_f = w * 100.0 + b
  puts "Prediction for 100.0 C -> #{pred_f.round(2)} F (Expected ~212.00 F)"
end

train("Mean Squared Error (MSE)", :mse, :mse_derivative, 0.0005)
train("Mean Absolute Error (MAE)", :mae, :mae_derivative, 0.01)
