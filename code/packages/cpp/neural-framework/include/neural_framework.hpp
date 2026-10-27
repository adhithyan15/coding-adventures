// Dense learning as value transformation, not shared mutable tensor storage.
//
// Each layer maps A (samples x inputs) to activation(A W + b). Backpropagation
// walks that same chain in reverse. A train step builds a separate candidate:
// there is exactly one publication point, after its parameters AND loss pass.
#ifndef CA_NEURAL_FRAMEWORK_HPP
#define CA_NEURAL_FRAMEWORK_HPP

#include "matrix.hpp"
#include "float_math.hpp"
#include "bounded_json.hpp"
#include <charconv>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace ca { namespace neural_framework {
using Matrix = ca::matrix::Matrix;
inline constexpr std::size_t max_layers = 16, max_width = 64, max_samples = 256;
inline constexpr std::size_t max_parameters = 65536, max_epochs = 100000;
inline constexpr std::uint64_t max_step = 9007199254740991ULL;
enum class Activation { linear, sigmoid, tanh, relu };
enum class Code { argument, capacity, shape, nonfinite, schema, json };
// Messages are deliberately closed: untrusted JSON text never becomes an error
// message, command, filename or format string. Allocation failures remain the
// standard std::bad_alloc, and also cannot partially commit a training step.
class Error : public std::runtime_error {
    Code code_;
public:
    explicit Error(Code code) : std::runtime_error("native dense operation rejected"), code_(code) {}
    Code code() const noexcept { return code_; }
};
struct Layer { Matrix weights; std::vector<double> biases; Activation activation; };
struct Checkpoint { std::vector<Layer> layers; std::uint64_t step = 0; };
struct Gradients { double loss; std::vector<Layer> layers; };

namespace detail {
inline void require(bool yes, Code code) { if (!yes) throw Error(code); }
inline double finite(double d) {
    require(float_math::isfinite(d), Code::nonfinite); return d;
}
inline void activation_valid(Activation a) {
    require(a == Activation::linear || a == Activation::sigmoid ||
            a == Activation::tanh || a == Activation::relu, Code::argument);
}
// Matrix is a dependency with a public representation. Cached dimensions alone
// do not establish its invariants. Check storage BEFORE calling dot/transpose.
inline void matrix_valid(const Matrix& m, std::size_t row_limit) {
    require(m.rows > 0 && m.rows <= row_limit && m.cols > 0 && m.cols <= max_width,
            Code::shape);
    require(m.data.size() == m.rows, Code::shape);
    for (const auto& row : m.data) {
        require(row.size() == m.cols, Code::shape);
        for (double d : row) finite(d);
    }
}
inline void layers_valid(const std::vector<Layer>& layers, std::uint64_t step) {
    require(!layers.empty() && layers.size() <= max_layers && step <= max_step, Code::capacity);
    std::size_t previous = 0, parameters = 0;
    for (const auto& l : layers) {
        matrix_valid(l.weights, max_width); activation_valid(l.activation);
        require(previous == 0 || l.weights.rows == previous, Code::shape);
        require(l.biases.size() == l.weights.cols, Code::shape);
        for (double d : l.biases) finite(d);
        // Both factors were bounded by 64, so multiplication cannot overflow.
        const auto count = l.weights.rows * l.weights.cols + l.biases.size();
        require(count <= max_parameters - parameters, Code::capacity);
        parameters += count; previous = l.weights.cols;
    }
}
inline double activate(double z, Activation a) {
    finite(z);
    switch (a) {
    case Activation::linear: return z;
    case Activation::relu: return z > 0 ? z : 0;
    case Activation::tanh: return finite(float_math::tanh(z));
    case Activation::sigmoid:
        // Exp only sees a nonpositive argument. Large finite logits saturate
        // without creating an infinite intermediate on the negative side.
        if (z >= 0) return finite(1 / (1 + float_math::exp(-z)));
        { double e = float_math::exp(z); return finite(e / (1 + e)); }
    }
    throw Error(Code::argument);
}
inline double derivative(double activated, Activation a) {
    switch (a) {
    case Activation::linear: return 1;
    case Activation::relu: return activated > 0 ? 1 : 0;
    case Activation::tanh: return 1 - activated * activated;
    case Activation::sigmoid: return activated * (1 - activated);
    }
    throw Error(Code::argument);
}
inline const char* activation_name(Activation a) {
    switch (a) {
    case Activation::linear: return "linear";
    case Activation::sigmoid: return "sigmoid";
    case Activation::tanh: return "tanh";
    case Activation::relu: return "relu";
    }
    throw Error(Code::argument);
}
inline Activation activation_read(std::string_view a) {
    if (a == "linear") return Activation::linear;
    if (a == "sigmoid") return Activation::sigmoid;
    if (a == "tanh") return Activation::tanh;
    if (a == "relu") return Activation::relu;
    throw Error(Code::schema);
}
using Value = bounded_json::Value;
using Kind = bounded_json::Kind;
// Closed object matching rejects duplicate, missing, unknown and NUL-altered
// field names without C-string truncation. Width is a tiny trusted constant.
inline std::vector<const Value*> fields(const Value& v, std::initializer_list<std::string_view> names) {
    require(v.kind() == Kind::object && v.size() == names.size(), Code::schema);
    std::vector<const Value*> result(names.size(), nullptr);
    for (std::size_t i = 0; i < v.size(); ++i) {
        auto member = v.member(i); std::size_t k = 0;
        for (auto name : names) { if (member.first == name) break; ++k; }
        require(k < result.size() && result[k] == nullptr, Code::schema);
        result[k] = member.second;
    }
    for (auto p : result) require(p != nullptr, Code::schema);
    return result;
}
inline std::uint64_t integer(const Value& v) {
    require(v.kind() == Kind::integer, Code::schema);
    const auto i = v.integer();
    require(i >= 0 && static_cast<std::uint64_t>(i) <= max_step, Code::schema);
    return static_cast<std::uint64_t>(i);
}
inline std::string_view text(const Value& v) {
    require(v.kind() == Kind::string, Code::schema); return v.string();
}
inline std::size_t array_size(const Value& v, std::size_t limit) {
    require(v.kind() == Kind::array && v.size() > 0 && v.size() <= limit, Code::schema);
    return v.size();
}
inline double number(const Value& v) {
    require(v.kind() == Kind::integer || v.kind() == Kind::floating, Code::schema);
    return finite(v.number());
}
inline void append_number(std::string& out, double d) {
    finite(d);
    // to_chars is locale-independent. max_digits10 guarantees reconstruction
    // of every finite binary64 value, including representable subnormals.
    if (d == 0 && float_math::copysign(1, d) < 0) { out += "-0.0"; return; }
    char buffer[128];
    const auto r = std::to_chars(buffer, buffer + sizeof buffer, d,
        std::chars_format::general, std::numeric_limits<double>::max_digits10);
    require(r.ec == std::errc{}, Code::nonfinite);
    out.append(buffer, r.ptr);
}
} // namespace detail

class Network {
    std::vector<Layer> layers_;
    std::uint64_t step_;
    void inputs_valid(const Matrix& x) const {
        detail::matrix_valid(x, max_samples);
        detail::require(x.cols == layers_.front().weights.rows, Code::shape);
    }
    void batch_valid(const Matrix& x, const Matrix& y) const {
        inputs_valid(x); detail::matrix_valid(y, max_samples);
        detail::require(x.rows == y.rows && y.cols == layers_.back().weights.cols, Code::shape);
    }
    std::vector<Matrix> forward(const Matrix& x) const {
        inputs_valid(x);
        std::vector<Matrix> values; values.reserve(layers_.size() + 1);
        values.push_back(x);
        for (const auto& l : layers_) {
            auto z = values.back().dot(l.weights);
            // Check the dot product before bias addition or activation can hide
            // a nonfinite intermediate through saturation.
            detail::matrix_valid(z, max_samples);
            for (auto& row : z.data)
                for (std::size_t j = 0; j < z.cols; ++j)
                    row[j] = detail::activate(detail::finite(row[j] + l.biases[j]), l.activation);
            values.push_back(std::move(z));
        }
        return values;
    }
    static double mse(const Matrix& p, const Matrix& y) {
        double sum = 0;
        for (std::size_t i = 0; i < p.rows; ++i)
            for (std::size_t j = 0; j < p.cols; ++j) {
                double d = detail::finite(p.data[i][j] - y.data[i][j]);
                sum = detail::finite(sum + detail::finite(d * d));
            }
        return detail::finite(sum / static_cast<double>(p.rows * p.cols));
    }
public:
    explicit Network(const std::vector<Layer>& layers, std::uint64_t step = 0) : step_(step) {
        // Validate before making any potentially large copy of caller storage.
        detail::layers_valid(layers, step); layers_ = layers;
    }
    // Explicit copy declarations suppress implicit destructive move. Rvalues
    // also copy: source and destination remain valid independent networks.
    Network(const Network&) = default;
    Network& operator=(const Network& other) {
        if (this != &other) { auto copy = other.layers_; layers_.swap(copy); step_ = other.step_; }
        return *this;
    }
    static Network seeded(const std::vector<std::size_t>& widths,
                          const std::vector<Activation>& activations, std::uint32_t seed) {
        detail::require(widths.size() >= 2 && widths.size() <= max_layers + 1 &&
                        activations.size() == widths.size() - 1, Code::capacity);
        std::size_t total = 0;
        for (auto w : widths) detail::require(w > 0 && w <= max_width, Code::capacity);
        for (std::size_t k = 0; k < activations.size(); ++k) {
            detail::activation_valid(activations[k]);
            const auto count = widths[k] * widths[k + 1] + widths[k + 1];
            detail::require(count <= max_parameters - total, Code::capacity); total += count;
        }
        std::vector<Layer> layers; layers.reserve(activations.size());
        for (std::size_t k = 0; k < activations.size(); ++k) {
            auto w = Matrix::zeros(widths[k], widths[k + 1]);
            const double divisor = float_math::sqrt(static_cast<double>(widths[k]));
            for (auto& row : w.data) for (double& d : row) {
                seed = std::uint32_t(1664525U * seed + 1013904223U);
                d = (2 * (static_cast<double>(seed) / 4294967296.0) - 1) / divisor;
            }
            layers.push_back({std::move(w), std::vector<double>(widths[k + 1], 0), activations[k]});
        }
        return Network(layers);
    }
    std::uint64_t step() const noexcept { return step_; }
    std::vector<Layer> layers() const { return layers_; }
    Checkpoint checkpoint() const { return {layers_, step_}; }
    static Network from_checkpoint(const Checkpoint& c) { return Network(c.layers, c.step); }
    Matrix predict(const Matrix& x) const { return forward(x).back(); }
    double loss(const Matrix& x, const Matrix& y) const {
        batch_valid(x, y); return mse(predict(x), y);
    }
    Gradients gradients(const Matrix& x, const Matrix& y) const {
        batch_valid(x, y); auto values = forward(x);
        const auto& predictions = values.back();
        Gradients result{mse(predictions, y), layers_};
        auto delta = predictions.subtract(y).scale(2 / static_cast<double>(x.rows * y.cols));
        detail::matrix_valid(delta, max_samples);
        for (std::size_t k = layers_.size(); k-- > 0;) {
            for (std::size_t i = 0; i < delta.rows; ++i)
                for (std::size_t j = 0; j < delta.cols; ++j)
                    delta.data[i][j] = detail::finite(delta.data[i][j] *
                        detail::derivative(values[k + 1].data[i][j], layers_[k].activation));
            auto dw = values[k].transpose().dot(delta);
            auto db = delta.sum_cols();
            detail::matrix_valid(dw, max_width); detail::matrix_valid(db, 1);
            result.layers[k].weights = std::move(dw); result.layers[k].biases = db.data[0];
            if (k != 0) {
                delta = delta.dot(layers_[k].weights.transpose());
                detail::matrix_valid(delta, max_samples);
            }
        }
        return result;
    }
    double train_step(const Matrix& x, const Matrix& y, double rate) {
        detail::require(float_math::isfinite(rate) && rate > 0, Code::argument);
        detail::require(step_ < max_step, Code::capacity);
        auto g = gradients(x, y); auto candidate = layers_;
        for (std::size_t k = 0; k < candidate.size(); ++k) {
            auto& l = candidate[k];
            for (std::size_t i = 0; i < l.weights.rows; ++i)
                for (std::size_t j = 0; j < l.weights.cols; ++j)
                    l.weights.data[i][j] = detail::finite(l.weights.data[i][j] -
                        detail::finite(rate * g.layers[k].weights.data[i][j]));
            for (std::size_t j = 0; j < l.biases.size(); ++j)
                l.biases[j] = detail::finite(l.biases[j] - detail::finite(rate * g.layers[k].biases[j]));
        }
        const double next_loss = Network(candidate, step_ + 1).loss(x, y);
        layers_.swap(candidate); ++step_; // nonthrowing publication, last of all
        return next_loss;
    }
    std::vector<double> fit(const Matrix& x, const Matrix& y, double rate, std::size_t epochs) {
        detail::require(epochs <= max_epochs, Code::capacity);
        detail::require(float_math::isfinite(rate) && rate > 0, Code::argument);
        std::vector<double> losses; losses.reserve(epochs + 1); losses.push_back(loss(x, y));
        for (std::size_t i = 0; i < epochs; ++i) losses.push_back(train_step(x, y, rate));
        return losses;
    }
    static Network from_json(const detail::Value& v) {
        auto f = detail::fields(v, {"format", "version", "step", "layers"});
        detail::require(detail::text(*f[0]) == "coding-adventures-dense" && detail::integer(*f[1]) == 1, Code::schema);
        const auto step = detail::integer(*f[2]);
        const auto count = detail::array_size(*f[3], max_layers);
        std::vector<Layer> layers; layers.reserve(count);
        std::size_t parameters = 0, previous = 0;
        for (std::size_t k = 0; k < count; ++k) {
            auto l = detail::fields(f[3]->at(k), {"activation", "weights", "biases"});
            const auto a = detail::activation_read(detail::text(*l[0]));
            const auto rows = detail::array_size(*l[1], max_width);
            const auto cols = detail::array_size(l[1]->at(0), max_width);
            detail::require((previous == 0 || previous == rows) && detail::array_size(*l[2], max_width) == cols, Code::schema);
            const auto n = rows * cols + cols;
            detail::require(n <= max_parameters - parameters, Code::capacity);
            parameters += n; previous = cols;
            auto w = Matrix::zeros(rows, cols); std::vector<double> b; b.reserve(cols);
            for (std::size_t i = 0; i < rows; ++i) {
                detail::require(detail::array_size(l[1]->at(i), max_width) == cols, Code::schema);
                for (std::size_t j = 0; j < cols; ++j) w.data[i][j] = detail::number(l[1]->at(i).at(j));
            }
            for (std::size_t j = 0; j < cols; ++j) b.push_back(detail::number(l[2]->at(j)));
            layers.push_back({std::move(w), std::move(b), a});
        }
        return Network(layers, step);
    }
    static Network from_json(std::string_view text) {
        auto result = bounded_json::parse(text);
        detail::require(static_cast<bool>(result), Code::json);
        return from_json(result.document->root());
    }
    std::string to_json() const {
        std::string out = "{\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":";
        char step_buffer[32];
        auto r = std::to_chars(step_buffer, step_buffer + sizeof step_buffer, step_);
        out.append(step_buffer, r.ptr); out += ",\"layers\":[";
        for (std::size_t k = 0; k < layers_.size(); ++k) {
            if (k != 0) out += ',';
            const auto& l = layers_[k];
            out += "{\"activation\":\""; out += detail::activation_name(l.activation); out += "\",\"weights\":[";
            for (std::size_t i = 0; i < l.weights.rows; ++i) {
                if (i != 0) out += ',';
                out += '[';
                for (std::size_t j = 0; j < l.weights.cols; ++j) {
                    if (j != 0) out += ',';
                    detail::append_number(out, l.weights.data[i][j]);
                }
                out += ']';
            }
            out += "],\"biases\":[";
            for (std::size_t j = 0; j < l.biases.size(); ++j) {
                if (j != 0) out += ',';
                detail::append_number(out, l.biases[j]);
            }
            out += "]}";
        }
        out += "]}"; return out;
    }
};
}} // namespace ca::neural_framework
#endif
