// The shared fixtures are data, not a second implementation of the learner.
// All model admission below goes through the public native API.
#include "neural_framework.hpp"
#include "iso_test.h"
#include <cstring>
#include <fstream>
#include <iterator>
#include <limits>
#include <locale>
#include <string>

namespace nf = ca::neural_framework;
namespace bj = ca::bounded_json;
using nf::Matrix;
std::size_t allocation_atomicity();
bool second_translation_unit();
static const nf::Network& identity(const nf::Network& n) { return n; }

static const bj::Value& field(const bj::Value& v, std::string_view key) {
    for (std::size_t i = 0; i < v.size(); ++i) {
        auto m = v.member(i);
        if (m.first == key) return *m.second;
    }
    throw std::runtime_error("fixture field missing");
}
static bj::Result fixture(const char* name) {
    std::ifstream f(std::string("../../../specs/fixtures/neural-framework-v1/") + name);
    if (!f) throw std::runtime_error("fixture not found: run from package cwd");
    return bj::parse(std::string(std::istreambuf_iterator<char>(f), {}));
}
static Matrix matrix(const bj::Value& v) {
    auto m = Matrix::zeros(v.size(), v.at(0).size());
    for (std::size_t i = 0; i < m.rows; ++i)
        for (std::size_t j = 0; j < m.cols; ++j) m.data[i][j] = v.at(i).at(j).number();
    return m;
}
static void close(const Matrix& a, const Matrix& b, double eps = 1e-12) {
    ISO_CHECK(a.rows == b.rows && a.cols == b.cols);
    for (std::size_t i = 0; i < a.rows; ++i)
        for (std::size_t j = 0; j < a.cols; ++j) {
            ISO_CHECK(ca::float_math::isfinite(a.data[i][j]));
            ISO_CHECK_EQ_DBL(a.data[i][j], b.data[i][j], eps);
        }
}
template<class F> static void rejects(F f, nf::Code code) {
    bool rejected = false;
    try { f(); } catch (const nf::Error& e) { rejected = e.code() == code; }
    ISO_CHECK(rejected);
}
static void paper() {
    auto doc = fixture("dense.json");
    ISO_CHECK(static_cast<bool>(doc));
    const auto& p = field(doc.document->root(), "paper");
    auto net = nf::Network::from_json(field(p, "checkpoint"));
    auto x = matrix(field(p, "inputs")), y = matrix(field(p, "targets"));
    close(net.predict(x), matrix(field(p, "predictions")));
    auto g = net.gradients(x, y);
    ISO_CHECK_EQ_DBL(g.loss, field(p, "loss").number(), 1e-12);
    for (std::size_t k = 0; k < g.layers.size(); ++k) {
        close(g.layers[k].weights, matrix(field(p, "weight_gradients").at(k)));
        for (std::size_t j = 0; j < g.layers[k].biases.size(); ++j)
            ISO_CHECK_EQ_DBL(g.layers[k].biases[j], field(p, "bias_gradients").at(k).at(j).number(), 1e-12);
    }
    const auto initial = net.to_json();
    auto inspected = net.layers(); inspected[0].weights.data[0][0] = 999;
    g.layers[0].weights.data[0][0] = 999;
    ISO_CHECK(net.to_json() == initial);
    ISO_CHECK_EQ_DBL(net.train_step(x, y, field(p, "learning_rate").number()), field(p, "updated_loss").number(), 1e-12);
    close(net.predict(x), matrix(field(p, "updated_predictions")));
    auto expected = nf::Network::from_json(field(p, "updated_checkpoint"));
    auto actual_layers = net.layers(), expected_layers = expected.layers();
    for (std::size_t k = 0; k < actual_layers.size(); ++k) {
        close(actual_layers[k].weights, expected_layers[k].weights);
        for (std::size_t j = 0; j < actual_layers[k].biases.size(); ++j)
            ISO_CHECK_EQ_DBL(actual_layers[k].biases[j], expected_layers[k].biases[j], 1e-12);
    }
    auto cp = net.checkpoint(); cp.layers[0].biases[0] = 900;
    auto copy = net; copy.train_step(x, y, .01);
    ISO_CHECK(net.step() == 1 && copy.step() == 2);
    // Public loader copies values: destroying the document must not dangle.
    doc.document.reset();
    close(net.predict(x), Matrix({{1.77}}));
    auto seeded = nf::Network::seeded({2, 2}, {nf::Activation::tanh}, 7);
    auto seed_doc = fixture("dense.json");
    close(seeded.layers()[0].weights, matrix(field(field(seed_doc.document->root(), "seed"), "first_weights")));
}
static void corpus() {
    auto doc = fixture("checkpoints.json");
    const auto& cases = field(doc.document->root(), "cases");
    std::size_t accepted = 0, rejected = 0;
    for (std::size_t i = 0; i < cases.size(); ++i) {
        const auto& c = cases.at(i);
        bool ok = false;
        try { auto n = nf::Network::from_json(field(c, "checkpoint")); ok = !n.layers().empty(); }
        catch (const nf::Error&) {}
        ISO_CHECK_MSG(ok == field(c, "accept").boolean(), std::string(field(c, "id").string()).c_str());
        if (ok) ++accepted; else ++rejected;
    }
    ISO_CHECK(accepted == 3 && rejected == 52);
}
static void derivatives() {
    const auto x = Matrix({{.2, -.3}, {.7, .5}, {-.4, .1}});
    const auto y = Matrix({{.1, -.2}, {.3, .2}, {-.1, .4}});
    for (auto a : {nf::Activation::linear, nf::Activation::sigmoid, nf::Activation::tanh, nf::Activation::relu}) {
        auto n = nf::Network::seeded({2, 3, 2}, {a, nf::Activation::linear}, 9);
        auto cp = n.checkpoint();
        // Move ReLU away from its kink; test its zero derivative separately.
        if (a == nf::Activation::relu) for (auto& b : cp.layers[0].biases) b = 2;
        n = nf::Network(cp.layers);
        auto g = n.gradients(x, y);
        for (std::size_t k = 0; k < cp.layers.size(); ++k) {
            auto& l = cp.layers[k];
            for (std::size_t i = 0; i < l.weights.rows; ++i)
                for (std::size_t j = 0; j < l.weights.cols; ++j) {
                    double old = l.weights.data[i][j];
                    l.weights.data[i][j] = old + 1e-6; double hi = nf::Network(cp.layers).loss(x, y);
                    l.weights.data[i][j] = old - 1e-6; double lo = nf::Network(cp.layers).loss(x, y);
                    l.weights.data[i][j] = old;
                    ISO_CHECK_EQ_DBL(g.layers[k].weights.data[i][j], (hi - lo) / 2e-6, 1e-9);
                }
            for (std::size_t j = 0; j < l.biases.size(); ++j) {
                double old = l.biases[j];
                l.biases[j] = old + 1e-6; double hi = nf::Network(cp.layers).loss(x, y);
                l.biases[j] = old - 1e-6; double lo = nf::Network(cp.layers).loss(x, y);
                l.biases[j] = old;
                ISO_CHECK_EQ_DBL(g.layers[k].biases[j], (hi - lo) / 2e-6, 1e-9);
            }
        }
    }
    auto zero = nf::Network({{Matrix({{0.0}}), {0}, nf::Activation::relu}});
    ISO_CHECK(zero.gradients(Matrix({{1.0}}), Matrix({{1.0}})).layers[0].weights.data[0][0] == 0);
}
static void failures() {
    auto n = nf::Network::seeded({2, 1}, {nf::Activation::linear}, 7);
    const auto before = n.to_json();
    const auto x = Matrix({{1, 2}}), y = Matrix({{1.0}});
    for (double rate : {0., -1., std::numeric_limits<double>::infinity()})
        rejects([&] { n.train_step(x, y, rate); }, nf::Code::argument);
    for (int mode = 0; mode < 5; ++mode) {
        auto bad = x;
        if (mode == 0) bad.rows = 999;
        if (mode == 1) bad.cols = 0;
        if (mode == 2) bad.data[0].pop_back();
        if (mode == 3) bad.data.clear();
        if (mode == 4) bad.data[0][0] = std::numeric_limits<double>::infinity();
        rejects([&] { n.train_step(bad, y, 1); }, mode == 4 ? nf::Code::nonfinite : nf::Code::shape);
    }
    ISO_CHECK(n.to_json() == before);
    auto exhausted = nf::Network(n.layers(), nf::max_step);
    rejects([&] { exhausted.train_step(x, y, 1); }, nf::Code::capacity);
    rejects([&] { n.fit(x, y, 1, nf::max_epochs + 1); }, nf::Code::capacity);
    ISO_CHECK(n.fit(x, y, 1, 0).size() == 1 && n.step() == 0);
    rejects([&] { n.train_step(Matrix({{1e308, 1e308}}), y, 1e308); }, nf::Code::nonfinite);
    ISO_CHECK(n.to_json() == before);
    rejects([] { nf::Network::seeded({2, 0}, {nf::Activation::linear}, 1); }, nf::Code::capacity);
    rejects([] { nf::Network({}, 0); }, nf::Code::capacity);
    rejects([] { nf::Network::from_json("{}"); }, nf::Code::schema);
    rejects([] { nf::Network::from_json("["); }, nf::Code::json);
    rejects([&] { n.loss(x, Matrix({{1.0, 2.0}})); }, nf::Code::shape);
    rejects([&] { n.predict(Matrix::zeros(257, 2)); }, nf::Code::shape);
    rejects([&] { n.fit(x, y, 0, 0); }, nf::Code::argument);
    rejects([] { nf::Network::seeded({1, 65}, {nf::Activation::linear}, 7); }, nf::Code::capacity);
    rejects([] { nf::Network::seeded({1}, {}, 7); }, nf::Code::capacity);
    rejects([] { nf::Network::seeded({1, 1}, {static_cast<nf::Activation>(999)}, 7); }, nf::Code::argument);
    rejects([] { nf::Network({{Matrix({{1.0}}), {}, nf::Activation::linear}}); }, nf::Code::shape);
    rejects([] { nf::Network({{Matrix({{1.0}}), {1}, nf::Activation::linear}}, nf::max_step + 1); }, nf::Code::capacity);
    auto layers = n.layers(); layers[0].weights.data[0][0] = 1;
    auto owned = nf::Network(layers); const auto owned_before = owned.to_json();
    layers[0].weights.data[0][0] = 99;
    ISO_CHECK(owned.to_json() == owned_before);
    auto moved = std::move(owned); // documented value-preserving rvalue copy
    ISO_CHECK(owned.to_json() == moved.to_json());
    moved = identity(moved);
    ISO_CHECK(owned.to_json() == moved.to_json());
    // Fit is per-step atomic: step 1 is retained when the next epoch exhausts.
    auto near_end = nf::Network(n.layers(), nf::max_step - 1);
    rejects([&] { near_end.fit(x, y, .01, 2); }, nf::Code::capacity);
    ISO_CHECK(near_end.step() == nf::max_step);
    // Finite updated parameters can still yield an overflowing candidate loss.
    auto candidate = nf::Network({{Matrix({{1.0}}), {0}, nf::Activation::linear}});
    auto candidate_before = candidate.to_json();
    rejects([&] { candidate.train_step(Matrix({{1.0}}), Matrix({{0.0}}), 1e155); }, nf::Code::nonfinite);
    ISO_CHECK(candidate.to_json() == candidate_before);
    // Total-parameter limit is independent of per-layer width/depth limits.
    rejects([] { nf::Network::seeded(std::vector<std::size_t>(17, 64), std::vector<nf::Activation>(16, nf::Activation::linear), 7); }, nf::Code::capacity);
}
static void raw_checkpoints() {
    const std::string valid = "{\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":0,\"layers\":[{\"activation\":\"linear\",\"weights\":[[1]],\"biases\":[0]}]}";
    for (const auto& pair : std::vector<std::pair<std::string, std::string>>{
        {"\"step\":0", "\"step\":0.0"}, {"\"version\":1", "\"version\":1e0"},
        {"\"step\":0", "\"step\\u0000\":0"}, {"\"step\":0", "\"step\":0,\"step\":1"},
        {"\"activation\":\"linear\"", "\"activation\":\"linear\\u0000\""},
        {"\"weights\":[[1]]", "\"weights\":[[null]]"},
        {"\"biases\":[0]", "\"biases\":[false]"}}) {
        auto text = valid; text.replace(text.find(pair.first), pair.first.size(), pair.second);
        rejects([&] { nf::Network::from_json(text); }, nf::Code::schema);
    }
    rejects([] { nf::Network::from_json(std::string(ca::bounded_json::max_bytes + 1, ' ')); }, nf::Code::json);
}
class CommaPunct : public std::numpunct<char> {
    char do_decimal_point() const override { return ','; }
};
static void roundtrips() {
    const auto previous = std::locale();
    std::locale::global(std::locale(previous, new CommaPunct));
    for (double d : {-0.0, std::numeric_limits<double>::max(), std::numeric_limits<double>::min(), std::numeric_limits<double>::denorm_min()}) {
        auto n = nf::Network({{Matrix({{d}}), {d}, nf::Activation::linear}});
        auto restored = nf::Network::from_json(n.to_json());
        const double r = restored.layers()[0].weights.data[0][0];
        ISO_CHECK(std::memcmp(&d, &r, sizeof d) == 0);
    }
    auto n = nf::Network({{Matrix({{.123}}), {.456}, nf::Activation::linear}});
    ISO_CHECK(n.to_json().find("0.123") != std::string::npos);
    ISO_CHECK(std::use_facet<std::numpunct<char>>(std::locale()).decimal_point() == ',');
    std::locale::global(previous);
}
static void xor_training() {
    auto doc = fixture("dense.json");
    const auto& v = field(doc.document->root(), "xor");
    auto x = matrix(field(v, "inputs")), y = matrix(field(v, "targets"));
    auto n = nf::Network::seeded({2, 4, 1}, {nf::Activation::tanh, nf::Activation::sigmoid}, 7);
    auto losses = n.fit(x, y, field(v, "learning_rate").number(), static_cast<std::size_t>(field(v, "epochs").integer()));
    ISO_CHECK(losses.size() == 3001 && n.step() == 3000);
    ISO_CHECK(losses.back() < field(v, "maximum_final_loss").number());
    auto a = nf::Network::from_json(n.to_json());
    for (int i = 0; i < 2; ++i) {
        n.train_step(x, y, 1); a.train_step(x, y, 1);
        ISO_CHECK(n.to_json() == a.to_json());
    }
    ISO_CHECK(n.step() == 3002);
    std::printf("NN51 native XOR step=%llu loss=%.17g\n", static_cast<unsigned long long>(n.step()), n.loss(x, y));
}
int main() {
    paper(); corpus(); derivatives(); failures(); raw_checkpoints(); roundtrips(); xor_training();
    ISO_CHECK(second_translation_unit());
#ifndef NN51_NO_ALLOCATION_FAULTS
    const auto failed_allocations = allocation_atomicity();
    ISO_CHECK(failed_allocations > 0);
    std::printf("NN51 allocation sweep: %zu real failure positions preserved state\n", failed_allocations);
#endif
    return ISO_TEST_RESULT();
}
