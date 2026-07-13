// gradient_descent.hpp — one step of stochastic gradient descent, C++17.
// =======================================================================
//
// A faithful port of the Rust `gradient-descent` crate, in namespace
// `ca::gradient_descent`.
//
// Stochastic gradient descent (SGD) is the workhorse of machine-learning
// optimisation. Given a vector of model **weights** and the **gradient** of the
// loss with respect to each weight, it nudges every weight a small step
// *downhill* — in the direction that reduces the loss:
//
//     new_weight[i] = weight[i] - learning_rate * gradient[i]
//
// The `learning_rate` (a small positive scalar like 0.01) controls the step
// size: too large and the optimiser overshoots and diverges; too small and it
// crawls. This function performs exactly one such update over the whole vector.
//
// Where the Rust crate returns `Result<Vec<f64>, &str>`, this port throws
// `ca::gradient_descent::GradientDescentError`. Pure ISO C++17.

#ifndef GRADIENT_DESCENT_HPP
#define GRADIENT_DESCENT_HPP

#include <cstddef>
#include <stdexcept>
#include <vector>

namespace ca {
namespace gradient_descent {

// Raised when the inputs are malformed (mismatched or empty vectors).
class GradientDescentError : public std::invalid_argument {
  public:
    explicit GradientDescentError(const std::string& message)
        : std::invalid_argument(message) {}
};

// Apply one SGD update: returns `weights[i] - learning_rate * gradients[i]`.
//
// Throws `GradientDescentError` if the two vectors differ in length or are
// empty (matching the Rust crate's error condition).
inline std::vector<double> sgd(const std::vector<double>& weights,
                               const std::vector<double>& gradients,
                               double learning_rate) {
    if (weights.size() != gradients.size() || weights.empty()) {
        throw GradientDescentError(
            "Arrays must have the same non-zero length");
    }
    std::vector<double> result;
    result.reserve(weights.size());
    for (std::size_t i = 0; i < weights.size(); ++i) {
        result.push_back(weights[i] - (learning_rate * gradients[i]));
    }
    return result;
}

}  // namespace gradient_descent
}  // namespace ca

#endif  // GRADIENT_DESCENT_HPP
