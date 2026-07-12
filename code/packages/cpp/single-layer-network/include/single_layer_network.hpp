// single_layer_network.hpp — a single dense neural-network layer with batch
// gradient descent, header-only in pure ISO C++17 (namespace
// ca::single_layer_network). A faithful port of the Rust
// `single-layer-network` crate.
// ===========================================================================
//
// One dense layer maps input_count features to output_count outputs:
//
//     prediction[o] = activate( bias[o] + Σ_i input[i] * weight[i][o] )
//
// Training is full-batch mean-squared-error gradient descent (see the crate
// docs / the C header for the per-epoch update). Matrices are
// std::vector<std::vector<double>> (the same shape as the Rust crate), so
// ragged rows are representable and validated.
//
// ACTIVATIONS: Linear (identity) and Sigmoid (numerically stable, from a
// libm-free e^x). Sigmoid's derivative uses the output: out*(1-out).
//
// DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port throws
// std::invalid_argument with the same message on a shape error.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions.
#ifndef CA_SINGLE_LAYER_NETWORK_HPP
#define CA_SINGLE_LAYER_NETWORK_HPP

#include <cstddef>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace single_layer_network {

inline constexpr const char* VERSION = "0.1.0";

using Matrix = std::vector<std::vector<double>>;

enum class ActivationName { Linear, Sigmoid };

struct TrainingStep {
    Matrix predictions;
    Matrix errors;
    Matrix weight_gradients;
    std::vector<double> bias_gradients;
    Matrix next_weights;
    std::vector<double> next_biases;
    double loss = 0.0;
};

namespace detail {

inline double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) result *= base;
        base *= base;
        n >>= 1;
    }
    return result;
}

inline double d_exp(double x) {
    if (x != x) return x;
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return 1.7976931348623157e308;
    if (x < -745.13321910194) return 0.0;
    constexpr double INV_LN2 = 1.4426950408889634;
    constexpr double C1 = 0.693359375;
    constexpr double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = static_cast<int>(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - static_cast<double>(k) * C1) - static_cast<double>(k) * C2;
    double term = 1.0, sum = 1.0;
    for (int i = 1; i <= 17; i++) {
        term *= r / static_cast<double>(i);
        sum += term;
    }
    return sum * pow2i(k);
}

inline double activate(double value, ActivationName activation) {
    if (activation == ActivationName::Linear) return value;
    if (value >= 0.0) {
        double z = d_exp(-value);
        return 1.0 / (1.0 + z);
    }
    double z = d_exp(value);
    return z / (1.0 + z);
}

inline double derivative_from_output(double output, ActivationName activation) {
    if (activation == ActivationName::Linear) return 1.0;
    return output * (1.0 - output);
}

// Validate a matrix: at least one row/column and rectangular. Returns
// (rows, cols) or throws std::invalid_argument (mirroring the Rust Err).
inline std::pair<std::size_t, std::size_t> validate_matrix(
    const std::string& name, const Matrix& matrix) {
    if (matrix.empty())
        throw std::invalid_argument(name + " must contain at least one row");
    std::size_t width = matrix[0].size();
    if (width == 0)
        throw std::invalid_argument(name + " must contain at least one column");
    for (const auto& row : matrix)
        if (row.size() != width)
            throw std::invalid_argument(name + " must be rectangular");
    return {matrix.size(), width};
}

}  // namespace detail

inline Matrix predict_with_parameters(const Matrix& inputs,
                                      const Matrix& weights,
                                      const std::vector<double>& biases,
                                      ActivationName activation) {
    auto in_shape = detail::validate_matrix("inputs", inputs);
    auto w_shape = detail::validate_matrix("weights", weights);
    std::size_t samples = in_shape.first, input_count = in_shape.second;
    std::size_t weight_rows = w_shape.first, output_count = w_shape.second;
    if (input_count != weight_rows)
        throw std::invalid_argument(
            "input column count must match weight row count");
    if (biases.size() != output_count)
        throw std::invalid_argument("bias count must match output count");

    Matrix predictions(samples, std::vector<double>(output_count, 0.0));
    for (std::size_t row = 0; row < samples; row++) {
        for (std::size_t output = 0; output < output_count; output++) {
            double total = biases[output];
            for (std::size_t input = 0; input < input_count; input++)
                total += inputs[row][input] * weights[input][output];
            predictions[row][output] = detail::activate(total, activation);
        }
    }
    return predictions;
}

inline TrainingStep train_one_epoch_with_matrices(
    const Matrix& inputs, const Matrix& targets, const Matrix& weights,
    const std::vector<double>& biases, double learning_rate,
    ActivationName activation) {
    auto in_shape = detail::validate_matrix("inputs", inputs);
    auto t_shape = detail::validate_matrix("targets", targets);
    auto w_shape = detail::validate_matrix("weights", weights);
    std::size_t samples = in_shape.first, input_count = in_shape.second;
    std::size_t target_rows = t_shape.first, output_count = t_shape.second;
    std::size_t weight_rows = w_shape.first, weight_cols = w_shape.second;
    if (target_rows != samples)
        throw std::invalid_argument(
            "inputs and targets must have the same row count");
    if (weight_rows != input_count || weight_cols != output_count)
        throw std::invalid_argument(
            "weights must be shaped input_count x output_count");
    if (biases.size() != output_count)
        throw std::invalid_argument("bias count must match output count");

    Matrix predictions =
        predict_with_parameters(inputs, weights, biases, activation);
    double scale = 2.0 / static_cast<double>(samples * output_count);
    Matrix errors(samples, std::vector<double>(output_count, 0.0));
    Matrix deltas(samples, std::vector<double>(output_count, 0.0));
    double loss_total = 0.0;
    for (std::size_t row = 0; row < samples; row++) {
        for (std::size_t output = 0; output < output_count; output++) {
            double error = predictions[row][output] - targets[row][output];
            errors[row][output] = error;
            deltas[row][output] =
                scale * error *
                detail::derivative_from_output(predictions[row][output],
                                               activation);
            loss_total += error * error;
        }
    }

    Matrix weight_gradients(input_count,
                            std::vector<double>(output_count, 0.0));
    Matrix next_weights(input_count, std::vector<double>(output_count, 0.0));
    for (std::size_t input = 0; input < input_count; input++) {
        for (std::size_t output = 0; output < output_count; output++) {
            for (std::size_t row = 0; row < samples; row++)
                weight_gradients[input][output] +=
                    inputs[row][input] * deltas[row][output];
            next_weights[input][output] =
                weights[input][output] -
                learning_rate * weight_gradients[input][output];
        }
    }

    std::vector<double> bias_gradients(output_count, 0.0);
    std::vector<double> next_biases(output_count, 0.0);
    for (std::size_t output = 0; output < output_count; output++) {
        for (std::size_t row = 0; row < samples; row++)
            bias_gradients[output] += deltas[row][output];
        next_biases[output] =
            biases[output] - learning_rate * bias_gradients[output];
    }

    TrainingStep step;
    step.predictions = std::move(predictions);
    step.errors = std::move(errors);
    step.weight_gradients = std::move(weight_gradients);
    step.bias_gradients = std::move(bias_gradients);
    step.next_weights = std::move(next_weights);
    step.next_biases = std::move(next_biases);
    step.loss = loss_total / static_cast<double>(samples * output_count);
    return step;
}

class SingleLayerNetwork {
public:
    Matrix weights;
    std::vector<double> biases;
    ActivationName activation;

    SingleLayerNetwork(std::size_t input_count, std::size_t output_count,
                       ActivationName activation_)
        : weights(input_count, std::vector<double>(output_count, 0.0)),
          biases(output_count, 0.0),
          activation(activation_) {}

    Matrix predict(const Matrix& inputs) const {
        return predict_with_parameters(inputs, weights, biases, activation);
    }

    std::vector<TrainingStep> fit(const Matrix& inputs, const Matrix& targets,
                                  double learning_rate, std::size_t epochs) {
        std::vector<TrainingStep> history;
        history.reserve(epochs);
        for (std::size_t e = 0; e < epochs; e++) {
            TrainingStep step = train_one_epoch_with_matrices(
                inputs, targets, weights, biases, learning_rate, activation);
            weights = step.next_weights;
            biases = step.next_biases;
            history.push_back(std::move(step));
        }
        return history;
    }
};

inline std::pair<SingleLayerNetwork, std::vector<TrainingStep>>
fit_single_layer_network(const Matrix& inputs, const Matrix& targets,
                         double learning_rate, std::size_t epochs,
                         ActivationName activation) {
    auto in_shape = detail::validate_matrix("inputs", inputs);
    auto t_shape = detail::validate_matrix("targets", targets);
    SingleLayerNetwork network(in_shape.second, t_shape.second, activation);
    auto history = network.fit(inputs, targets, learning_rate, epochs);
    return {std::move(network), std::move(history)};
}

}  // namespace single_layer_network
}  // namespace ca

#endif  // CA_SINGLE_LAYER_NETWORK_HPP
