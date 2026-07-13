// graph.hpp — an undirected weighted graph, header-only ISO C++17.
// ================================================================
//
// A faithful port of the Rust `graph` crate, in namespace `ca::graph`: an
// undirected, weighted graph that stores its edges in one of two classic
// representations — an **adjacency list** or an **adjacency matrix** — behind a
// single API, plus the standard graph algorithms (BFS, DFS, connectivity,
// connected components, cycle detection, shortest path, and a minimum spanning
// tree).
//
// The two representations produce identical observable results; the crate keeps
// both to show the trade-off (a matrix costs O(V^2) space but answers
// `has_edge` in O(1); a list is sparse-friendly). Every node is a string, every
// map is *ordered* (Rust `BTreeMap` -> C++ `std::map`), so traversals and edge
// listings come out in a deterministic, sorted order.
//
// Nodes and edges also carry **property bags** — ordered maps of string keys to
// tagged values (string / number / bool / null). Each edge always exposes a
// `"weight"` property mirroring its numeric weight.
//
// Where the Rust crate returns `Result`, this port throws `ca::graph::Error`
// (carrying an `ErrorKind`). Pure ISO C++17: <map>, <set>, <deque>, <vector>,
// <optional>, <string>. No extensions, no <cmath>.

#ifndef GRAPH_HPP
#define GRAPH_HPP

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <deque>
#include <limits>
#include <map>
#include <optional>
#include <set>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace graph {

// ── Representation ───────────────────────────────────────────────────────────
enum class Repr { AdjacencyList, AdjacencyMatrix };

// ── Errors ───────────────────────────────────────────────────────────────────
enum class ErrorKind { NodeNotFound, EdgeNotFound, NotConnected };

class Error : public std::runtime_error {
  public:
    explicit Error(ErrorKind kind, const std::string& what)
        : std::runtime_error(what), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

  private:
    ErrorKind kind_;
};

// ── Property values ──────────────────────────────────────────────────────────
//
// A tagged value: a string, a number (f64), a bool, or null. Ordered property
// bags map string keys to these.
class PropertyValue {
  public:
    enum class Kind { String, Number, Bool, Null };

    PropertyValue() : kind_(Kind::Null) {}
    static PropertyValue String(std::string s) {
        PropertyValue v;
        v.kind_ = Kind::String;
        v.string_ = std::move(s);
        return v;
    }
    static PropertyValue Number(double n) {
        PropertyValue v;
        v.kind_ = Kind::Number;
        v.number_ = n;
        return v;
    }
    static PropertyValue Bool(bool b) {
        PropertyValue v;
        v.kind_ = Kind::Bool;
        v.bool_ = b;
        return v;
    }
    static PropertyValue Null() { return PropertyValue(); }

    Kind kind() const noexcept { return kind_; }
    const std::string& as_string() const { return string_; }
    double as_number() const noexcept { return number_; }
    bool as_bool() const noexcept { return bool_; }

    bool operator==(const PropertyValue& o) const noexcept {
        if (kind_ != o.kind_) {
            return false;
        }
        switch (kind_) {
            case Kind::String: return string_ == o.string_;
            case Kind::Number: return number_ == o.number_;
            case Kind::Bool: return bool_ == o.bool_;
            case Kind::Null: return true;
        }
        return false;
    }
    bool operator!=(const PropertyValue& o) const noexcept {
        return !(*this == o);
    }

  private:
    Kind kind_;
    std::string string_;
    double number_ = 0.0;
    bool bool_ = false;
};

using PropertyBag = std::map<std::string, PropertyValue>;

// A weighted edge (endpoints in canonical, sorted order).
struct WeightedEdge {
    std::string left;
    std::string right;
    double weight;
    bool operator==(const WeightedEdge& o) const noexcept {
        return left == o.left && right == o.right && weight == o.weight;
    }
};

// Rust f64::total_cmp: a total order over all doubles (used to sort edges
// deterministically even across ±0 / NaN). Returns <0, 0, >0.
inline int total_cmp(double a, double b) {
    std::uint64_t ua, ub;
    std::memcpy(&ua, &a, sizeof ua);
    std::memcpy(&ub, &b, sizeof ub);
    ua ^= (static_cast<std::uint64_t>(0) - (ua >> 63)) |
          0x8000000000000000ULL;
    ub ^= (static_cast<std::uint64_t>(0) - (ub >> 63)) |
          0x8000000000000000ULL;
    if (ua < ub) return -1;
    if (ua > ub) return 1;
    return 0;
}

// ── The graph ────────────────────────────────────────────────────────────────
class Graph {
  public:
    explicit Graph(Repr repr = Repr::AdjacencyList) : repr_(repr) {}

    Repr repr() const noexcept { return repr_; }

    void add_node(const std::string& node) {
        add_node_with_properties(node, PropertyBag{});
    }

    void add_node_with_properties(const std::string& node,
                                  const PropertyBag& properties) {
        switch (repr_) {
            case Repr::AdjacencyList:
                adj_.try_emplace(node);
                break;
            case Repr::AdjacencyMatrix:
                if (node_index_.find(node) == node_index_.end()) {
                    std::size_t index = node_list_.size();
                    node_list_.push_back(node);
                    node_index_[node] = index;
                    for (auto& row : matrix_) {
                        row.push_back(std::nullopt);
                    }
                    matrix_.emplace_back(index + 1, std::nullopt);
                }
                break;
        }
        PropertyBag& bag = node_properties_[node];
        for (const auto& kv : properties) {
            bag[kv.first] = kv.second;
        }
    }

    void remove_node(const std::string& node) {
        switch (repr_) {
            case Repr::AdjacencyList: {
                auto it = adj_.find(node);
                if (it == adj_.end()) {
                    throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
                }
                std::map<std::string, double> neighbors = it->second;
                for (const auto& kv : neighbors) {
                    auto nb = adj_.find(kv.first);
                    if (nb != adj_.end()) {
                        nb->second.erase(node);
                    }
                    edge_properties_.erase(canonical_endpoints(node, kv.first));
                }
                adj_.erase(node);
                node_properties_.erase(node);
                break;
            }
            case Repr::AdjacencyMatrix: {
                auto it = node_index_.find(node);
                if (it == node_index_.end()) {
                    throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
                }
                std::size_t index = it->second;
                node_index_.erase(it);
                for (const auto& other : node_list_) {
                    edge_properties_.erase(canonical_endpoints(node, other));
                }
                node_list_.erase(node_list_.begin() +
                                 static_cast<std::ptrdiff_t>(index));
                node_properties_.erase(node);
                matrix_.erase(matrix_.begin() +
                              static_cast<std::ptrdiff_t>(index));
                for (auto& row : matrix_) {
                    row.erase(row.begin() +
                              static_cast<std::ptrdiff_t>(index));
                }
                for (std::size_t offset = index; offset < node_list_.size();
                     ++offset) {
                    node_index_[node_list_[offset]] = offset;
                }
                break;
            }
        }
    }

    bool has_node(const std::string& node) const {
        switch (repr_) {
            case Repr::AdjacencyList:
                return adj_.find(node) != adj_.end();
            case Repr::AdjacencyMatrix:
                return node_index_.find(node) != node_index_.end();
        }
        return false;
    }

    std::vector<std::string> nodes() const {
        std::vector<std::string> result;
        switch (repr_) {
            case Repr::AdjacencyList:
                for (const auto& kv : adj_) {
                    result.push_back(kv.first);
                }
                break;
            case Repr::AdjacencyMatrix:
                result = node_list_;
                break;
        }
        std::sort(result.begin(), result.end());
        return result;
    }

    void add_edge(const std::string& left, const std::string& right,
                  double weight) {
        add_edge_with_properties(left, right, weight, PropertyBag{});
    }

    void add_edge_with_properties(const std::string& left,
                                  const std::string& right, double weight,
                                  const PropertyBag& properties) {
        add_node(left);
        add_node(right);
        switch (repr_) {
            case Repr::AdjacencyList:
                adj_[left][right] = weight;
                adj_[right][left] = weight;
                break;
            case Repr::AdjacencyMatrix: {
                std::size_t li = node_index_.at(left);
                std::size_t ri = node_index_.at(right);
                matrix_[li][ri] = weight;
                matrix_[ri][li] = weight;
                break;
            }
        }
        PropertyBag& bag = edge_properties_[canonical_endpoints(left, right)];
        for (const auto& kv : properties) {
            bag[kv.first] = kv.second;
        }
        bag["weight"] = PropertyValue::Number(weight);
    }

    void remove_edge(const std::string& left, const std::string& right) {
        switch (repr_) {
            case Repr::AdjacencyList: {
                auto it = adj_.find(left);
                if (it == adj_.end() ||
                    it->second.find(right) == it->second.end()) {
                    throw Error(ErrorKind::EdgeNotFound,
                                "edge not found: " + left + " -- " + right);
                }
                adj_[left].erase(right);
                adj_[right].erase(left);
                edge_properties_.erase(canonical_endpoints(left, right));
                break;
            }
            case Repr::AdjacencyMatrix: {
                auto li = node_index_.find(left);
                auto ri = node_index_.find(right);
                if (li == node_index_.end() || ri == node_index_.end() ||
                    !matrix_[li->second][ri->second].has_value()) {
                    throw Error(ErrorKind::EdgeNotFound,
                                "edge not found: " + left + " -- " + right);
                }
                matrix_[li->second][ri->second] = std::nullopt;
                matrix_[ri->second][li->second] = std::nullopt;
                edge_properties_.erase(canonical_endpoints(left, right));
                break;
            }
        }
    }

    bool has_edge(const std::string& left, const std::string& right) const {
        switch (repr_) {
            case Repr::AdjacencyList: {
                auto it = adj_.find(left);
                return it != adj_.end() &&
                       it->second.find(right) != it->second.end();
            }
            case Repr::AdjacencyMatrix: {
                auto li = node_index_.find(left);
                auto ri = node_index_.find(right);
                return li != node_index_.end() && ri != node_index_.end() &&
                       matrix_[li->second][ri->second].has_value();
            }
        }
        return false;
    }

    double edge_weight(const std::string& left, const std::string& right) const {
        switch (repr_) {
            case Repr::AdjacencyList: {
                auto it = adj_.find(left);
                if (it != adj_.end()) {
                    auto w = it->second.find(right);
                    if (w != it->second.end()) {
                        return w->second;
                    }
                }
                break;
            }
            case Repr::AdjacencyMatrix: {
                auto li = node_index_.find(left);
                auto ri = node_index_.find(right);
                if (li != node_index_.end() && ri != node_index_.end()) {
                    const auto& cell = matrix_[li->second][ri->second];
                    if (cell.has_value()) {
                        return *cell;
                    }
                }
                break;
            }
        }
        throw Error(ErrorKind::EdgeNotFound,
                    "edge not found: " + left + " -- " + right);
    }

    PropertyBag graph_properties() const { return graph_properties_; }
    void set_graph_property(const std::string& key, PropertyValue value) {
        graph_properties_[key] = std::move(value);
    }
    void remove_graph_property(const std::string& key) {
        graph_properties_.erase(key);
    }

    PropertyBag node_properties(const std::string& node) const {
        if (!has_node(node)) {
            throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
        }
        auto it = node_properties_.find(node);
        return it == node_properties_.end() ? PropertyBag{} : it->second;
    }

    void set_node_property(const std::string& node, const std::string& key,
                           PropertyValue value) {
        if (!has_node(node)) {
            throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
        }
        node_properties_[node][key] = std::move(value);
    }

    void remove_node_property(const std::string& node, const std::string& key) {
        if (!has_node(node)) {
            throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
        }
        auto it = node_properties_.find(node);
        if (it != node_properties_.end()) {
            it->second.erase(key);
        }
    }

    PropertyBag edge_properties(const std::string& left,
                                const std::string& right) const {
        if (!has_edge(left, right)) {
            throw Error(ErrorKind::EdgeNotFound,
                        "edge not found: " + left + " -- " + right);
        }
        PropertyBag properties;
        auto it = edge_properties_.find(canonical_endpoints(left, right));
        if (it != edge_properties_.end()) {
            properties = it->second;
        }
        properties["weight"] = PropertyValue::Number(edge_weight(left, right));
        return properties;
    }

    void set_edge_property(const std::string& left, const std::string& right,
                           const std::string& key, PropertyValue value) {
        if (!has_edge(left, right)) {
            throw Error(ErrorKind::EdgeNotFound,
                        "edge not found: " + left + " -- " + right);
        }
        if (key == "weight") {
            if (value.kind() != PropertyValue::Kind::Number) {
                throw Error(ErrorKind::EdgeNotFound,
                            "edge not found: weight -- numeric property");
            }
            double weight = value.as_number();
            set_edge_weight(left, right, weight);
            edge_properties_[canonical_endpoints(left, right)]["weight"] =
                PropertyValue::Number(weight);
            return;
        }
        edge_properties_[canonical_endpoints(left, right)][key] =
            std::move(value);
    }

    void remove_edge_property(const std::string& left, const std::string& right,
                              const std::string& key) {
        if (!has_edge(left, right)) {
            throw Error(ErrorKind::EdgeNotFound,
                        "edge not found: " + left + " -- " + right);
        }
        if (key == "weight") {
            set_edge_weight(left, right, 1.0);
            edge_properties_[canonical_endpoints(left, right)]["weight"] =
                PropertyValue::Number(1.0);
            return;
        }
        auto it = edge_properties_.find(canonical_endpoints(left, right));
        if (it != edge_properties_.end()) {
            it->second.erase(key);
        }
    }

    std::vector<WeightedEdge> edges() const {
        std::vector<WeightedEdge> result;
        switch (repr_) {
            case Repr::AdjacencyList: {
                std::set<std::pair<std::string, std::string>> seen;
                for (const auto& outer : adj_) {
                    for (const auto& inner : outer.second) {
                        auto ends = canonical_endpoints(outer.first, inner.first);
                        if (seen.insert(ends).second) {
                            result.push_back({ends.first, ends.second,
                                              inner.second});
                        }
                    }
                }
                break;
            }
            case Repr::AdjacencyMatrix:
                for (std::size_t row = 0; row < node_list_.size(); ++row) {
                    for (std::size_t col = row; col < node_list_.size(); ++col) {
                        if (matrix_[row][col].has_value()) {
                            result.push_back({node_list_[row], node_list_[col],
                                              *matrix_[row][col]});
                        }
                    }
                }
                break;
        }
        std::sort(result.begin(), result.end(),
                  [](const WeightedEdge& a, const WeightedEdge& b) {
                      int c = total_cmp(a.weight, b.weight);
                      if (c != 0) return c < 0;
                      if (a.left != b.left) return a.left < b.left;
                      return a.right < b.right;
                  });
        return result;
    }

    std::vector<std::string> neighbors(const std::string& node) const {
        switch (repr_) {
            case Repr::AdjacencyList: {
                auto it = adj_.find(node);
                if (it == adj_.end()) {
                    throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
                }
                std::vector<std::string> result;
                for (const auto& kv : it->second) {
                    result.push_back(kv.first);
                }
                return result;
            }
            case Repr::AdjacencyMatrix: {
                auto it = node_index_.find(node);
                if (it == node_index_.end()) {
                    throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
                }
                std::vector<std::string> result;
                const auto& row = matrix_[it->second];
                for (std::size_t col = 0; col < row.size(); ++col) {
                    if (row[col].has_value()) {
                        result.push_back(node_list_[col]);
                    }
                }
                std::sort(result.begin(), result.end());
                return result;
            }
        }
        return {};
    }

    std::map<std::string, double> neighbors_weighted(
        const std::string& node) const {
        switch (repr_) {
            case Repr::AdjacencyList: {
                auto it = adj_.find(node);
                if (it == adj_.end()) {
                    throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
                }
                return it->second;
            }
            case Repr::AdjacencyMatrix: {
                auto it = node_index_.find(node);
                if (it == node_index_.end()) {
                    throw Error(ErrorKind::NodeNotFound, "node not found: " + node);
                }
                std::map<std::string, double> result;
                const auto& row = matrix_[it->second];
                for (std::size_t col = 0; col < row.size(); ++col) {
                    if (row[col].has_value()) {
                        result[node_list_[col]] = *row[col];
                    }
                }
                return result;
            }
        }
        return {};
    }

    std::size_t degree(const std::string& node) const {
        return neighbors(node).size();
    }

    std::size_t size() const {
        switch (repr_) {
            case Repr::AdjacencyList:
                return adj_.size();
            case Repr::AdjacencyMatrix:
                return node_list_.size();
        }
        return 0;
    }

  private:
    void set_edge_weight(const std::string& left, const std::string& right,
                         double weight) {
        switch (repr_) {
            case Repr::AdjacencyList:
                if (!has_edge(left, right)) {
                    throw Error(ErrorKind::EdgeNotFound,
                                "edge not found: " + left + " -- " + right);
                }
                adj_[left][right] = weight;
                adj_[right][left] = weight;
                break;
            case Repr::AdjacencyMatrix: {
                auto li = node_index_.find(left);
                auto ri = node_index_.find(right);
                if (li == node_index_.end() || ri == node_index_.end() ||
                    !matrix_[li->second][ri->second].has_value()) {
                    throw Error(ErrorKind::EdgeNotFound,
                                "edge not found: " + left + " -- " + right);
                }
                matrix_[li->second][ri->second] = weight;
                matrix_[ri->second][li->second] = weight;
                break;
            }
        }
    }

    static std::pair<std::string, std::string> canonical_endpoints(
        const std::string& left, const std::string& right) {
        if (left <= right) {
            return {left, right};
        }
        return {right, left};
    }

    Repr repr_;
    std::map<std::string, std::map<std::string, double>> adj_;
    std::vector<std::string> node_list_;
    std::map<std::string, std::size_t> node_index_;
    std::vector<std::vector<std::optional<double>>> matrix_;
    PropertyBag graph_properties_;
    std::map<std::string, PropertyBag> node_properties_;
    std::map<std::pair<std::string, std::string>, PropertyBag> edge_properties_;
};

// ── Algorithms ───────────────────────────────────────────────────────────────

inline std::vector<std::string> bfs(const Graph& graph, const std::string& start) {
    (void)graph.neighbors(start);  // validate start exists (throws otherwise)
    std::set<std::string> visited;
    visited.insert(start);
    std::deque<std::string> queue{start};
    std::vector<std::string> result;
    while (!queue.empty()) {
        std::string node = queue.front();
        queue.pop_front();
        result.push_back(node);
        for (const auto& neighbor : graph.neighbors(node)) {
            if (visited.insert(neighbor).second) {
                queue.push_back(neighbor);
            }
        }
    }
    return result;
}

inline std::vector<std::string> dfs(const Graph& graph, const std::string& start) {
    (void)graph.neighbors(start);
    std::set<std::string> visited;
    std::vector<std::string> stack{start};
    std::vector<std::string> result;
    while (!stack.empty()) {
        std::string node = stack.back();
        stack.pop_back();
        if (!visited.insert(node).second) {
            continue;
        }
        result.push_back(node);
        std::vector<std::string> neighbors = graph.neighbors(node);
        std::reverse(neighbors.begin(), neighbors.end());
        for (const auto& neighbor : neighbors) {
            if (visited.find(neighbor) == visited.end()) {
                stack.push_back(neighbor);
            }
        }
    }
    return result;
}

inline bool is_connected(const Graph& graph) {
    if (graph.size() == 0) {
        return true;
    }
    std::vector<std::string> nodes = graph.nodes();
    try {
        return bfs(graph, nodes[0]).size() == graph.size();
    } catch (const Error&) {
        return false;
    }
}

inline std::vector<std::vector<std::string>> connected_components(
    const Graph& graph) {
    std::set<std::string> remaining;
    for (const auto& node : graph.nodes()) {
        remaining.insert(node);
    }
    std::vector<std::vector<std::string>> result;
    while (!remaining.empty()) {
        std::string start = *remaining.begin();
        std::vector<std::string> component = bfs(graph, start);
        for (const auto& node : component) {
            remaining.erase(node);
        }
        result.push_back(component);
    }
    return result;
}

namespace detail {
inline bool cycle_visit(const Graph& graph, const std::string& node,
                        const std::string* parent,
                        std::set<std::string>& visited) {
    visited.insert(node);
    for (const auto& neighbor : graph.neighbors(node)) {
        if (visited.find(neighbor) == visited.end()) {
            if (cycle_visit(graph, neighbor, &node, visited)) {
                return true;
            }
        } else if (parent == nullptr || neighbor != *parent) {
            return true;
        }
    }
    return false;
}
}  // namespace detail

inline bool has_cycle(const Graph& graph) {
    std::set<std::string> visited;
    for (const auto& node : graph.nodes()) {
        if (visited.find(node) == visited.end() &&
            detail::cycle_visit(graph, node, nullptr, visited)) {
            return true;
        }
    }
    return false;
}

namespace detail {
inline std::vector<std::string> bfs_shortest_path(const Graph& graph,
                                                  const std::string& start,
                                                  const std::string& end) {
    std::map<std::string, std::optional<std::string>> parent;
    parent[start] = std::nullopt;
    std::deque<std::string> queue{start};
    while (!queue.empty()) {
        std::string node = queue.front();
        queue.pop_front();
        if (node == end) {
            break;
        }
        for (const auto& neighbor : graph.neighbors(node)) {
            if (parent.find(neighbor) == parent.end()) {
                parent[neighbor] = node;
                queue.push_back(neighbor);
            }
        }
    }
    if (parent.find(end) == parent.end()) {
        return {};
    }
    std::vector<std::string> path;
    std::optional<std::string> current = end;
    while (current.has_value()) {
        std::string node = *current;
        auto it = parent.find(node);
        current = (it != parent.end()) ? it->second : std::nullopt;
        path.push_back(node);
    }
    std::reverse(path.begin(), path.end());
    return path;
}

inline std::vector<std::string> dijkstra_shortest_path(const Graph& graph,
                                                       const std::string& start,
                                                       const std::string& end) {
    constexpr double kInf = std::numeric_limits<double>::infinity();
    std::map<std::string, double> distances;
    std::map<std::string, std::string> parent;
    for (const auto& node : graph.nodes()) {
        distances[node] = kInf;
    }
    distances[start] = 0.0;

    struct Item {
        double distance;
        std::size_t sequence;
        std::string node;
    };
    std::size_t sequence = 0;
    std::vector<Item> queue{{0.0, sequence, start}};

    while (!queue.empty()) {
        std::sort(queue.begin(), queue.end(),
                  [](const Item& a, const Item& b) {
                      int c = total_cmp(a.distance, b.distance);
                      if (c != 0) return c < 0;
                      return a.sequence < b.sequence;
                  });
        Item top = queue.front();
        queue.erase(queue.begin());
        double distance = top.distance;
        std::string node = top.node;
        auto dit = distances.find(node);
        if (distance > (dit != distances.end() ? dit->second : kInf)) {
            continue;
        }
        if (node == end) {
            break;
        }
        for (const auto& kv : graph.neighbors_weighted(node)) {
            double next_distance = distance + kv.second;
            auto ndit = distances.find(kv.first);
            if (next_distance < (ndit != distances.end() ? ndit->second : kInf)) {
                distances[kv.first] = next_distance;
                parent[kv.first] = node;
                ++sequence;
                queue.push_back({next_distance, sequence, kv.first});
            }
        }
    }

    auto eit = distances.find(end);
    if ((eit != distances.end() ? eit->second : kInf) == kInf) {
        return {};
    }
    std::vector<std::string> path;
    std::string current = end;
    for (;;) {
        path.push_back(current);
        if (current == start) {
            break;
        }
        auto pit = parent.find(current);
        if (pit == parent.end()) {
            return {};
        }
        current = pit->second;
    }
    std::reverse(path.begin(), path.end());
    return path;
}
}  // namespace detail

inline std::vector<std::string> shortest_path(const Graph& graph,
                                              const std::string& start,
                                              const std::string& end) {
    if (!graph.has_node(start) || !graph.has_node(end)) {
        return {};
    }
    if (start == end) {
        return {start};
    }
    bool all_unit = true;
    for (const auto& edge : graph.edges()) {
        if (edge.weight != 1.0) {
            all_unit = false;
            break;
        }
    }
    return all_unit ? detail::bfs_shortest_path(graph, start, end)
                    : detail::dijkstra_shortest_path(graph, start, end);
}

namespace detail {
// Union-find with path compression and union-by-rank (ordered maps to match the
// Rust crate exactly).
class UnionFind {
  public:
    explicit UnionFind(const std::vector<std::string>& nodes) {
        for (const auto& node : nodes) {
            parent_[node] = node;
            rank_[node] = 0;
        }
    }
    std::size_t size() const { return parent_.size(); }
    std::string find(const std::string& node) {
        std::string p = parent_.at(node);
        if (p != node) {
            std::string root = find(p);
            parent_[node] = root;
            return root;
        }
        return p;
    }
    void unite(const std::string& left, const std::string& right) {
        std::string lr = find(left);
        std::string rr = find(right);
        if (lr == rr) {
            return;
        }
        std::size_t lrank = rank_.count(lr) ? rank_[lr] : 0;
        std::size_t rrank = rank_.count(rr) ? rank_[rr] : 0;
        if (lrank < rrank) {
            std::swap(lr, rr);
        }
        parent_[rr] = lr;
        if (lrank == rrank) {
            rank_[lr] = lrank + 1;
        }
    }

  private:
    std::map<std::string, std::string> parent_;
    std::map<std::string, std::size_t> rank_;
};
}  // namespace detail

inline std::vector<WeightedEdge> minimum_spanning_tree(const Graph& graph) {
    std::vector<std::string> nodes = graph.nodes();
    std::vector<WeightedEdge> all_edges = graph.edges();
    if (nodes.size() <= 1 || all_edges.empty()) {
        return {};
    }
    if (!is_connected(graph)) {
        throw Error(ErrorKind::NotConnected, "graph is not connected");
    }
    std::vector<WeightedEdge> result;
    detail::UnionFind uf(nodes);
    for (const auto& edge : all_edges) {
        if (uf.find(edge.left) != uf.find(edge.right)) {
            uf.unite(edge.left, edge.right);
            result.push_back(edge);
            if (result.size() == uf.size() - 1) {
                break;
            }
        }
    }
    return result;
}

}  // namespace graph
}  // namespace ca

#endif  // GRAPH_HPP
