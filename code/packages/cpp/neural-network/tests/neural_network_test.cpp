// Tests for the C++ neural-network library, using the header-only iso_test.h
// harness (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <optional>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

#include "neural_network.hpp"

namespace nn = ca::neural_network;
using nn::ActivationKind;
using nn::WeightedInput;

static bool has_edge_id(const nn::NeuralGraph& g, const std::string& id) {
    for (const nn::Edge& e : g.edges())
        if (e.id == id) return true;
    return false;
}

int main() {
    // ── activation names ──────────────────────────────────────────────────
    ISO_CHECK_STR_EQ(nn::as_str(ActivationKind::Relu), "relu");
    ISO_CHECK_STR_EQ(nn::as_str(ActivationKind::Sigmoid), "sigmoid");
    ISO_CHECK_STR_EQ(nn::as_str(ActivationKind::Tanh), "tanh");
    ISO_CHECK_STR_EQ(nn::as_str(ActivationKind::None), "none");

    // ── new graph seeds nn.version (+ nn.name) ────────────────────────────
    {
        nn::NeuralGraph g(std::string("tiny"));
        const auto& gp = g.graph_properties();
        ISO_CHECK(std::get<std::string>(gp.at("nn.version")) == "0");
        ISO_CHECK(std::get<std::string>(gp.at("nn.name")) == "tiny");
    }

    // ── builds a tiny weighted graph; incoming + topo sort ────────────────
    {
        nn::NeuralGraph g(std::string("tiny"));
        nn::add_input(g, "x0", "x0", {});
        nn::add_input(g, "x1", "x1", {});
        nn::add_constant(g, "bias", 1.0, {});
        nn::add_weighted_sum(
            g, "sum",
            {WeightedInput("x0", 0.25, "x0_to_sum"),
             WeightedInput("x1", 0.75, "x1_to_sum"),
             WeightedInput("bias", -1.0, "bias_to_sum")},
            {});
        nn::add_activation(g, "relu", "sum", ActivationKind::Relu, {},
                           std::string("sum_to_relu"));
        nn::add_output(g, "out", "relu", "prediction", {},
                       std::string("relu_to_out"));

        ISO_CHECK_EQ_UINT(g.incoming_edges("sum").size(), 3u);
        auto order = g.topological_sort();
        ISO_CHECK(order.has_value());
        ISO_CHECK(!order->empty());
        ISO_CHECK_STR_EQ(order->back().c_str(), "out");
    }

    // ── weighted-sum node carries nn.op; edge carries "weight" ────────────
    {
        nn::NeuralGraph g;
        nn::add_weighted_sum(g, "s", {WeightedInput("a", 2.0, "a_to_s")}, {});
        ISO_CHECK(std::get<std::string>(g.node_properties("s").at("nn.op")) ==
                  "weighted_sum");
        auto inc = g.incoming_edges("s");
        ISO_CHECK_EQ_UINT(inc.size(), 1u);
        ISO_CHECK(std::get<double>(inc[0].properties.at("weight")) == 2.0);
    }

    // ── a non-finite constant throws ──────────────────────────────────────
    {
        nn::NeuralGraph g;
        volatile double zero = 0.0;
        double nan = zero / zero;
        bool threw = false;
        try {
            nn::add_constant(g, "c", nan, {});
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
        nn::add_constant(g, "c", 3.5, {});  // finite is fine
    }

    // ── auto-generated edge ids ───────────────────────────────────────────
    {
        nn::NeuralGraph g;
        ISO_CHECK_STR_EQ(g.add_edge("a", "b", 1.0, {}, std::nullopt).c_str(),
                         "e0");
        ISO_CHECK_STR_EQ(g.add_edge("b", "c", 1.0, {}, std::nullopt).c_str(),
                         "e1");
    }

    // ── cycle detection ───────────────────────────────────────────────────
    {
        nn::NeuralGraph g;
        g.add_edge("a", "b", 1.0, {}, std::string("e_ab"));
        g.add_edge("b", "a", 1.0, {}, std::string("e_ba"));
        ISO_CHECK(!g.topological_sort().has_value());  // cycle -> nullopt
    }

    // ── the XOR network topology ──────────────────────────────────────────
    {
        nn::NeuralNetwork net = nn::create_xor_network("xor");
        ISO_CHECK_EQ_UINT(net.graph.incoming_edges("out_sum").size(), 3u);
        ISO_CHECK(has_edge_id(net.graph, "h_or_to_out"));
        ISO_CHECK(net.graph.topological_sort().has_value());  // it's a DAG
    }

    return ISO_TEST_RESULT();
}
