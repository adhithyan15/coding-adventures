// two_layer_network.hpp — a two-layer (one hidden layer) neural network with
// backpropagation, header-only in pure ISO C++17 (namespace
// ca::two_layer_network). A faithful port of the Rust `two-layer-network`
// crate.
// ===========================================================================
//
// A hidden layer lets the network learn non-linearly-separable functions such
// as XOR (a single layer cannot). Forward pass:
//
//     hidden_raw  = inputs · W_ih + b_h ;  hidden = activation(hidden_raw)
//     output_raw  = hidden · W_ho + b_o ;  prediction = activation(output_raw)
//
// `train_one_epoch` runs one full-batch mean-squared-error step, backpropagating
// the error through both layers and returning every gradient plus the next
// parameters. Activations: Linear and Sigmoid (numerically stable, from a
// libm-free e^x).
//
// Matrices are std::vector<std::vector<double>> (the same shape as the Rust
// crate), so ragged rows are representable and validated.
//
// DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port throws
// std::invalid_argument with the same message on a shape error.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions.
#ifndef CA_TWO_LAYER_NETWORK_HPP
#define CA_TWO_LAYER_NETWORK_HPP

#include <cstddef>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace two_layer_network {

inline constexpr const char* VERSION = "0.1.0";

using Matrix = std::vector<std::vector<double>>;

enum class ActivationName { Linear, Sigmoid };

struct Parameters {
    Matrix input_to_hidden_weights;
    std::vector<double> hidden_biases;
    Matrix hidden_to_output_weights;
    std::vector<double> output_biases;
};

struct ForwardPass {
    Matrix hidden_raw;
    Matrix hidden_activations;
    Matrix output_raw;
    Matrix predictions;
};

struct TrainingStep {
    Matrix predictions;
    Matrix errors;
    Matrix output_deltas;
    Matrix hidden_deltas;
    Matrix hidden_to_output_weight_gradients;
    std::vector<double> output_bias_gradients;
    Matrix input_to_hidden_weight_gradients;
    std::vector<double> hidden_bias_gradients;
    Parameters next_parameters;
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

inline double derivative(double activated, ActivationName activation) {
    if (activation == ActivationName::Linear) return 1.0;
    return activated * (1.0 - activated);
}

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

inline Matrix dot(const Matrix& left, const Matrix& right) {
    auto ls = validate_matrix("left", left);
    auto rs = validate_matrix("right", right);
    std::size_t rows = ls.first, width = ls.second;
    std::size_t right_rows = rs.first, cols = rs.second;
    if (width != right_rows)
        throw std::invalid_argument("matrix shapes do not align");
    Matrix out(rows, std::vector<double>(cols, 0.0));
    for (std::size_t row = 0; row < rows; row++)
        for (std::size_t col = 0; col < cols; col++) {
            double sum = 0.0;
            for (std::size_t k = 0; k < width; k++)
                sum += left[row][k] * right[k][col];
            out[row][col] = sum;
        }
    return out;
}

inline Matrix transpose(const Matrix& matrix) {
    auto s = validate_matrix("matrix", matrix);
    std::size_t rows = s.first, cols = s.second;
    Matrix out(cols, std::vector<double>(rows, 0.0));
    for (std::size_t row = 0; row < rows; row++)
        for (std::size_t col = 0; col < cols; col++)
            out[col][row] = matrix[row][col];
    return out;
}

inline Matrix add_biases(Matrix matrix, const std::vector<double>& biases) {
    for (auto& row : matrix)
        for (std::size_t col = 0; col < row.size(); col++)
            row[col] += biases[col];
    return matrix;
}

inline Matrix apply_activation(const Matrix& matrix, ActivationName activation) {
    Matrix out = matrix;
    for (auto& row : out)
        for (auto& v : row) v = activate(v, activation);
    return out;
}

inline std::vector<double> column_sums(const Matrix& matrix) {
    auto s = validate_matrix("matrix", matrix);
    std::size_t rows = s.first, cols = s.second;
    std::vector<double> out(cols, 0.0);
    for (std::size_t col = 0; col < cols; col++)
        for (std::size_t row = 0; row < rows; row++)
            out[col] += matrix[row][col];
    return out;
}

inline double mean_squared_error(const Matrix& errors) {
    double sum = 0.0;
    std::size_t n = 0;
    for (const auto& row : errors)
        for (double v : row) {
            sum += v * v;
            n++;
        }
    return sum / static_cast<double>(n);
}

inline Matrix subtract_scaled(const Matrix& matrix, const Matrix& gradients,
                              double learning_rate) {
    Matrix out = matrix;
    for (std::size_t row = 0; row < out.size(); row++)
        for (std::size_t col = 0; col < out[row].size(); col++)
            out[row][col] -= learning_rate * gradients[row][col];
    return out;
}

}  // namespace detail

inline Parameters xor_warm_start_parameters() {
    Parameters p;
    p.input_to_hidden_weights = {{4.0, -4.0}, {4.0, -4.0}};
    p.hidden_biases = {-2.0, 6.0};
    p.hidden_to_output_weights = {{4.0}, {4.0}};
    p.output_biases = {-6.0};
    return p;
}

inline ForwardPass forward(const Matrix& inputs, const Parameters& parameters,
                           ActivationName hidden_activation,
                           ActivationName output_activation) {
    Matrix hidden_raw = detail::add_biases(
        detail::dot(inputs, parameters.input_to_hidden_weights),
        parameters.hidden_biases);
    Matrix hidden_activations =
        detail::apply_activation(hidden_raw, hidden_activation);
    Matrix output_raw = detail::add_biases(
        detail::dot(hidden_activations, parameters.hidden_to_output_weights),
        parameters.output_biases);
    Matrix predictions =
        detail::apply_activation(output_raw, output_activation);
    return ForwardPass{std::move(hidden_raw), std::move(hidden_activations),
                       std::move(output_raw), std::move(predictions)};
}

inline TrainingStep train_one_epoch(const Matrix& inputs, const Matrix& targets,
                                    const Parameters& parameters,
                                    double learning_rate,
                                    ActivationName hidden_activation,
                                    ActivationName output_activation) {
    auto in_shape = detail::validate_matrix("inputs", inputs);
    auto t_shape = detail::validate_matrix("targets", targets);
    std::size_t sample_count = in_shape.first;
    std::size_t output_count = t_shape.second;

    ForwardPass passed =
        forward(inputs, parameters, hidden_activation, output_activation);
    double scale = 2.0 / static_cast<double>(sample_count * output_count);
    Matrix errors(sample_count, std::vector<double>(output_count, 0.0));
    Matrix output_deltas(sample_count, std::vector<double>(output_count, 0.0));
    for (std::size_t row = 0; row < sample_count; row++)
        for (std::size_t output = 0; output < output_count; output++) {
            double error =
                passed.predictions[row][output] - targets[row][output];
            errors[row][output] = error;
            output_deltas[row][output] =
                scale * error *
                detail::derivative(passed.predictions[row][output],
                                   output_activation);
        }

    Matrix h2o_gradients =
        detail::dot(detail::transpose(passed.hidden_activations), output_deltas);
    std::vector<double> output_bias_gradients =
        detail::column_sums(output_deltas);
    Matrix hidden_errors = detail::dot(
        output_deltas, detail::transpose(parameters.hidden_to_output_weights));
    std::size_t hidden_width = parameters.hidden_biases.size();
    Matrix hidden_deltas(sample_count,
                         std::vector<double>(hidden_width, 0.0));
    for (std::size_t row = 0; row < sample_count; row++)
        for (std::size_t hidden = 0; hidden < hidden_width; hidden++)
            hidden_deltas[row][hidden] =
                hidden_errors[row][hidden] *
                detail::derivative(passed.hidden_activations[row][hidden],
                                   hidden_activation);
    Matrix i2h_gradients = detail::dot(detail::transpose(inputs), hidden_deltas);
    std::vector<double> hidden_bias_gradients =
        detail::column_sums(hidden_deltas);

    Parameters next;
    next.input_to_hidden_weights = detail::subtract_scaled(
        parameters.input_to_hidden_weights, i2h_gradients, learning_rate);
    next.hidden_biases = parameters.hidden_biases;
    for (std::size_t i = 0; i < next.hidden_biases.size(); i++)
        next.hidden_biases[i] -= learning_rate * hidden_bias_gradients[i];
    next.hidden_to_output_weights = detail::subtract_scaled(
        parameters.hidden_to_output_weights, h2o_gradients, learning_rate);
    next.output_biases = parameters.output_biases;
    for (std::size_t i = 0; i < next.output_biases.size(); i++)
        next.output_biases[i] -= learning_rate * output_bias_gradients[i];

    TrainingStep step;
    step.predictions = std::move(passed.predictions);
    step.errors = errors;  // copied: also used for the loss below
    step.output_deltas = std::move(output_deltas);
    step.hidden_deltas = std::move(hidden_deltas);
    step.hidden_to_output_weight_gradients = std::move(h2o_gradients);
    step.output_bias_gradients = std::move(output_bias_gradients);
    step.input_to_hidden_weight_gradients = std::move(i2h_gradients);
    step.hidden_bias_gradients = std::move(hidden_bias_gradients);
    step.next_parameters = std::move(next);
    step.loss = detail::mean_squared_error(errors);
    return step;
}

}  // namespace two_layer_network
}  // namespace ca

#endif  // CA_TWO_LAYER_NETWORK_HPP
