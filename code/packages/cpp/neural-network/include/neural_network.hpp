// neural_network.hpp — a property-graph representation of neural-network
// topologies, header-only in pure ISO C++17 (namespace ca::neural_network). A
// faithful port of the Rust `neural-network` crate.
// ===========================================================================
//
// This is NOT a trainable network — it is the graph IR describing one: named
// nodes (input / constant / weighted_sum / activation / output), weighted
// directed edges, and a property bag on the graph, each node, and each edge.
// On top sits a fluent builder and a topological sort.
//
//   PropertyValue = std::variant<std::string, double, bool, std::monostate>
//   PropertyBag   = std::unordered_map<std::string, PropertyValue>
//   Edge          { id, from, to, weight, properties }
//   NeuralGraph   nodes + per-node bags + edges + an edge-id counter
//
// `add_edge` auto-creates its endpoints and mints an id ("e0", "e1", ...) when
// none is given; `topological_sort` runs Kahn's algorithm with deterministic
// (lexicographic) tie-breaking.
//
// DIVERGENCE FROM RUST. `add_constant` throws std::invalid_argument on a
// non-finite value (the Rust panic); `topological_sort` returns
// std::optional<std::vector<std::string>> (std::nullopt on a cycle) in place of
// Rust's `Result<_, String>`.
//
// PORTABILITY. Pure ISO C++17 — standard library only, no <cmath>, no compiler
// extensions.
#ifndef CA_NEURAL_NETWORK_HPP
#define CA_NEURAL_NETWORK_HPP

#include <algorithm>
#include <cstddef>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

namespace ca {
namespace neural_network {

// A property value: String / Number / Boolean / Null (std::monostate).
using PropertyValue = std::variant<std::string, double, bool, std::monostate>;
using PropertyBag = std::unordered_map<std::string, PropertyValue>;

enum class ActivationKind { Relu, Sigmoid, Tanh, None };

inline const char* as_str(ActivationKind a) {
    switch (a) {
        case ActivationKind::Relu: return "relu";
        case ActivationKind::Sigmoid: return "sigmoid";
        case ActivationKind::Tanh: return "tanh";
        case ActivationKind::None: return "none";
    }
    return "none";
}

struct Edge {
    std::string id;
    std::string from;
    std::string to;
    double weight = 0.0;
    PropertyBag properties;
};

struct WeightedInput {
    std::string from;
    double weight = 0.0;
    std::optional<std::string> edge_id;
    PropertyBag properties;

    WeightedInput(std::string from_, double weight_, std::string edge_id_)
        : from(std::move(from_)),
          weight(weight_),
          edge_id(std::move(edge_id_)) {}
};

namespace detail {
// Finite (not NaN / inf) without <cmath>: x - x == 0 only for finite x.
inline bool is_finite(double x) { return (x - x) == 0.0; }
}  // namespace detail

class NeuralGraph {
public:
    explicit NeuralGraph(std::optional<std::string> name = std::nullopt) {
        graph_properties_["nn.version"] = std::string("0");
        if (name) graph_properties_["nn.name"] = *name;
    }

    const PropertyBag& graph_properties() const { return graph_properties_; }
    const std::vector<std::string>& nodes() const { return nodes_; }
    const std::vector<Edge>& edges() const { return edges_; }

    void add_node(const std::string& node, const PropertyBag& properties) {
        auto it = node_properties_.find(node);
        if (it == node_properties_.end()) {
            nodes_.push_back(node);
            it = node_properties_.emplace(node, PropertyBag{}).first;
        }
        for (const auto& kv : properties) it->second[kv.first] = kv.second;
    }

    PropertyBag node_properties(const std::string& node) const {
        auto it = node_properties_.find(node);
        return it == node_properties_.end() ? PropertyBag{} : it->second;
    }

    std::string add_edge(const std::string& from, const std::string& to,
                         double weight, PropertyBag properties,
                         std::optional<std::string> edge_id) {
        add_node(from, {});
        add_node(to, {});
        std::string id;
        if (edge_id) {
            id = *edge_id;
        } else {
            id = "e" + std::to_string(next_edge_id_);
            next_edge_id_++;
        }
        properties["weight"] = weight;
        edges_.push_back(Edge{id, from, to, weight, std::move(properties)});
        return id;
    }

    std::vector<Edge> incoming_edges(const std::string& node) const {
        std::vector<Edge> out;
        for (const Edge& e : edges_)
            if (e.to == node) out.push_back(e);
        return out;
    }

    // Kahn's algorithm; std::nullopt on a cycle. Deterministic ordering.
    std::optional<std::vector<std::string>> topological_sort() const {
        std::unordered_map<std::string, std::size_t> indegree;
        for (const std::string& n : nodes_) indegree[n] = 0;
        for (const Edge& e : edges_) {
            indegree[e.to]++;
            indegree.emplace(e.from, 0);
        }
        std::vector<std::string> ready;
        for (const auto& kv : indegree)
            if (kv.second == 0) ready.push_back(kv.first);
        std::sort(ready.begin(), ready.end());

        std::vector<std::string> queue = ready;
        std::size_t head = 0;
        std::vector<std::string> order;
        while (head < queue.size()) {
            std::string node = queue[head++];
            order.push_back(node);
            std::vector<std::string> released;
            for (const Edge& e : edges_) {
                if (e.from != node) continue;
                auto it = indegree.find(e.to);
                if (it != indegree.end() && it->second > 0) {
                    it->second--;
                    if (it->second == 0) released.push_back(e.to);
                }
            }
            std::sort(released.begin(), released.end());
            for (const std::string& r : released) queue.push_back(r);
        }
        if (order.size() != indegree.size()) return std::nullopt;  // cycle
        return order;
    }

private:
    PropertyBag graph_properties_;
    std::vector<std::string> nodes_;
    std::unordered_map<std::string, PropertyBag> node_properties_;
    std::vector<Edge> edges_;
    std::size_t next_edge_id_ = 0;
};

// ── free-function layer builders ─────────────────────────────────────────────

inline void add_input(NeuralGraph& graph, const std::string& node,
                      const std::string& input_name, PropertyBag properties) {
    properties["nn.op"] = std::string("input");
    properties["nn.input"] = input_name;
    graph.add_node(node, properties);
}

inline void add_constant(NeuralGraph& graph, const std::string& node,
                         double value, PropertyBag properties) {
    if (!detail::is_finite(value))
        throw std::invalid_argument("constant value must be finite");
    properties["nn.op"] = std::string("constant");
    properties["nn.value"] = value;
    graph.add_node(node, properties);
}

inline void add_weighted_sum(NeuralGraph& graph, const std::string& node,
                             const std::vector<WeightedInput>& inputs,
                             PropertyBag properties) {
    properties["nn.op"] = std::string("weighted_sum");
    graph.add_node(node, properties);
    for (const WeightedInput& in : inputs)
        graph.add_edge(in.from, node, in.weight, in.properties, in.edge_id);
}

inline std::string add_activation(NeuralGraph& graph, const std::string& node,
                                  const std::string& input,
                                  ActivationKind activation,
                                  PropertyBag properties,
                                  std::optional<std::string> edge_id) {
    properties["nn.op"] = std::string("activation");
    properties["nn.activation"] = std::string(as_str(activation));
    graph.add_node(node, properties);
    return graph.add_edge(input, node, 1.0, {}, std::move(edge_id));
}

inline std::string add_output(NeuralGraph& graph, const std::string& node,
                              const std::string& input,
                              const std::string& output_name,
                              PropertyBag properties,
                              std::optional<std::string> edge_id) {
    properties["nn.op"] = std::string("output");
    properties["nn.output"] = output_name;
    graph.add_node(node, properties);
    return graph.add_edge(input, node, 1.0, {}, std::move(edge_id));
}

inline NeuralGraph create_neural_graph(std::optional<std::string> name) {
    return NeuralGraph(std::move(name));
}

// A fluent builder wrapping a graph.
class NeuralNetwork {
public:
    NeuralGraph graph;

    explicit NeuralNetwork(std::optional<std::string> name = std::nullopt)
        : graph(std::move(name)) {}

    NeuralNetwork& input(const std::string& node) {
        add_input(graph, node, node, {});
        return *this;
    }
    NeuralNetwork& constant(const std::string& node, double value,
                            PropertyBag properties) {
        add_constant(graph, node, value, std::move(properties));
        return *this;
    }
    NeuralNetwork& weighted_sum(const std::string& node,
                                const std::vector<WeightedInput>& inputs,
                                PropertyBag properties) {
        add_weighted_sum(graph, node, inputs, std::move(properties));
        return *this;
    }
    NeuralNetwork& activation(const std::string& node, const std::string& input,
                              ActivationKind kind, PropertyBag properties,
                              const std::string& edge_id) {
        add_activation(graph, node, input, kind, std::move(properties),
                       edge_id);
        return *this;
    }
    NeuralNetwork& output(const std::string& node, const std::string& input,
                          const std::string& output_name, PropertyBag properties,
                          const std::string& edge_id) {
        add_output(graph, node, input, output_name, std::move(properties),
                   edge_id);
        return *this;
    }
};

inline NeuralNetwork create_neural_network(std::optional<std::string> name) {
    return NeuralNetwork(std::move(name));
}

namespace detail {
inline PropertyBag prop(const std::string& key, const std::string& value) {
    PropertyBag bag;
    bag[key] = value;
    return bag;
}
inline WeightedInput wi(const std::string& from, double weight,
                        const std::string& edge_id) {
    return WeightedInput(from, weight, edge_id);
}
}  // namespace detail

inline NeuralNetwork create_xor_network(const std::string& name) {
    return create_neural_network(name)
        .input("x0")
        .input("x1")
        .constant("bias", 1.0, detail::prop("nn.role", "bias"))
        .weighted_sum("h_or_sum",
                      {detail::wi("x0", 20.0, "x0_to_h_or"),
                       detail::wi("x1", 20.0, "x1_to_h_or"),
                       detail::wi("bias", -10.0, "bias_to_h_or")},
                      detail::prop("nn.layer", "hidden"))
        .activation("h_or", "h_or_sum", ActivationKind::Sigmoid,
                    detail::prop("nn.layer", "hidden"), "h_or_sum_to_h_or")
        .weighted_sum("h_nand_sum",
                      {detail::wi("x0", -20.0, "x0_to_h_nand"),
                       detail::wi("x1", -20.0, "x1_to_h_nand"),
                       detail::wi("bias", 30.0, "bias_to_h_nand")},
                      detail::prop("nn.layer", "hidden"))
        .activation("h_nand", "h_nand_sum", ActivationKind::Sigmoid,
                    detail::prop("nn.layer", "hidden"), "h_nand_sum_to_h_nand")
        .weighted_sum("out_sum",
                      {detail::wi("h_or", 20.0, "h_or_to_out"),
                       detail::wi("h_nand", 20.0, "h_nand_to_out"),
                       detail::wi("bias", -30.0, "bias_to_out")},
                      detail::prop("nn.layer", "output"))
        .activation("out_activation", "out_sum", ActivationKind::Sigmoid,
                    detail::prop("nn.layer", "output"), "out_sum_to_activation")
        .output("out", "out_activation", "prediction",
                detail::prop("nn.layer", "output"), "activation_to_out");
}

}  // namespace neural_network
}  // namespace ca

#endif  // CA_NEURAL_NETWORK_HPP
