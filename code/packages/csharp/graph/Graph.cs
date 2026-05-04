using System.Text.Json;

// Graph.cs -- Two ways to store the same undirected weighted graph
// ================================================================
//
// This package teaches a useful systems lesson: the *interface* to a data
// structure can stay stable while the *representation* changes underneath.
//
// We expose one Graph<T> API, but let callers choose between:
//
//   1. Adjacency list   -- best when the graph is sparse
//   2. Adjacency matrix -- best when edge lookups matter more than space
//
// The algorithms later in the file (BFS, DFS, shortest path, MST) are written
// in terms of the public graph operations rather than the private fields, so
// they work for both representations without duplicating the algorithmic logic.

namespace CodingAdventures.Graph;

/// <summary>
/// Select which internal storage model backs the graph.
/// </summary>
public enum GraphRepr
{
    /// <summary>
    /// Store only the edges that actually exist.
    /// </summary>
    AdjacencyList,

    /// <summary>
    /// Reserve a table entry for every possible node-to-node connection.
    /// </summary>
    AdjacencyMatrix,
}

/// <summary>
/// A single undirected weighted edge.
///
/// The endpoints are stored in canonical order so equality checks and sorted
/// test assertions stay deterministic.
/// </summary>
public readonly record struct WeightedEdge<T>(T Left, T Right, double Weight) where T : notnull;

// Node ordering exists purely to make iteration deterministic. Hash-based
// collections do not promise a stable traversal order, but educational tests
// and examples are much easier to reason about when neighbors always appear in
// the same sequence.
internal static class NodeOrdering
{
    public static string CanonicalKey<T>(T node)
    {
        try
        {
            return $"{typeof(T).FullName}:{JsonSerializer.Serialize(node)}";
        }
        catch
        {
            return $"{typeof(T).FullName}:{node}";
        }
    }

    public static int Compare<T>(T left, T right) =>
        StringComparer.Ordinal.Compare(CanonicalKey(left), CanonicalKey(right));

    public static List<T> Sort<T>(IEnumerable<T> nodes)
    {
        var result = nodes.ToList();
        result.Sort(Compare);
        return result;
    }

    public static (T Left, T Right) CanonicalEdge<T>(T left, T right) =>
        Compare(left, right) <= 0 ? (left, right) : (right, left);

    public static string EdgeKey<T>(T left, T right)
    {
        var leftKey = CanonicalKey(left);
        var rightKey = CanonicalKey(right);
        return StringComparer.Ordinal.Compare(leftKey, rightKey) <= 0
            ? $"{leftKey}\0{rightKey}"
            : $"{rightKey}\0{leftKey}";
    }
}

/// <summary>
/// An undirected weighted graph with interchangeable adjacency-list and
/// adjacency-matrix storage.
/// </summary>
public sealed class Graph<T> where T : notnull
{
    // Adjacency-list storage:
    //   node -> (neighbor -> weight)
    private readonly GraphRepr _repr;
    private readonly Dictionary<T, Dictionary<T, double>> _adj = new();

    // Adjacency-matrix storage:
    //   _nodeList/_nodeIndex translate T <-> integer position
    //   _matrix[row][col] stores Some(weight) / None
    private readonly List<T> _nodeList = new();
    private readonly Dictionary<T, int> _nodeIndex = new();
    private readonly List<List<double?>> _matrix = new();
    private readonly Dictionary<string, object?> _graphProperties = new(StringComparer.Ordinal);
    private readonly Dictionary<T, Dictionary<string, object?>> _nodeProperties = new();
    private readonly Dictionary<string, Dictionary<string, object?>> _edgeProperties = new(StringComparer.Ordinal);

    /// <summary>
    /// Create an empty graph backed by the selected representation.
    /// </summary>
    public Graph(GraphRepr repr = GraphRepr.AdjacencyList)
    {
        _repr = repr;
    }

    /// <summary>
    /// Report which internal storage model this graph currently uses.
    /// </summary>
    public GraphRepr Representation => _repr;

    /// <summary>
    /// Count how many nodes are currently present in the graph.
    /// </summary>
    public int Size => _repr == GraphRepr.AdjacencyList ? _adj.Count : _nodeList.Count;

    /// <summary>
    /// Add a node if it is not already present.
    /// </summary>
    public void AddNode(T node, IReadOnlyDictionary<string, object?>? properties = null)
    {
        // In adjacency-list mode, "adding a node" just means ensuring it has an
        // empty neighbor map.
        if (_repr == GraphRepr.AdjacencyList)
        {
            if (!_adj.ContainsKey(node))
            {
                _adj[node] = new Dictionary<T, double>();
            }

            MergeNodeProperties(node, properties);
            return;
        }

        // In adjacency-matrix mode we must grow the matrix in both dimensions:
        // every existing row gets a new column, and we append one new row.
        if (_nodeIndex.ContainsKey(node))
        {
            MergeNodeProperties(node, properties);
            return;
        }

        var index = _nodeList.Count;
        _nodeList.Add(node);
        _nodeIndex[node] = index;

        foreach (var row in _matrix)
        {
            row.Add(null);
        }

        var newRow = new List<double?>();
        for (var i = 0; i <= index; i++)
        {
            newRow.Add(null);
        }

        _matrix.Add(newRow);
        MergeNodeProperties(node, properties);
    }

    /// <summary>
    /// Remove a node and every incident edge attached to it.
    /// </summary>
    public void RemoveNode(T node)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            if (!_adj.TryGetValue(node, out var neighbors))
            {
                throw new KeyNotFoundException($"Node not found: {node}");
            }

            foreach (var neighbor in neighbors.Keys.ToList())
            {
                _adj[neighbor].Remove(node);
                _edgeProperties.Remove(NodeOrdering.EdgeKey(node, neighbor));
            }

            _adj.Remove(node);
            _nodeProperties.Remove(node);
            return;
        }

        if (!_nodeIndex.TryGetValue(node, out var index))
        {
            throw new KeyNotFoundException($"Node not found: {node}");
        }

        foreach (var other in _nodeList)
        {
            _edgeProperties.Remove(NodeOrdering.EdgeKey(node, other));
        }

        _nodeProperties.Remove(node);
        _nodeIndex.Remove(node);
        _nodeList.RemoveAt(index);
        _matrix.RemoveAt(index);
        foreach (var row in _matrix)
        {
            row.RemoveAt(index);
        }

        for (var i = index; i < _nodeList.Count; i++)
        {
            _nodeIndex[_nodeList[i]] = i;
        }
    }

    /// <summary>
    /// Test whether the graph already contains the given node.
    /// </summary>
    public bool HasNode(T node) =>
        _repr == GraphRepr.AdjacencyList ? _adj.ContainsKey(node) : _nodeIndex.ContainsKey(node);

    /// <summary>
    /// Return the set of nodes currently stored in the graph.
    /// </summary>
    public IReadOnlyCollection<T> Nodes() =>
        _repr == GraphRepr.AdjacencyList
            ? new HashSet<T>(_adj.Keys)
            : new HashSet<T>(_nodeList);

    /// <summary>
    /// Add or overwrite an undirected weighted edge between two nodes.
    /// Missing endpoints are created automatically.
    /// </summary>
    public void AddEdge(T left, T right, double weight = 1.0, IReadOnlyDictionary<string, object?>? properties = null)
    {
        AddNode(left);
        AddNode(right);

        if (_repr == GraphRepr.AdjacencyList)
        {
            _adj[left][right] = weight;
            _adj[right][left] = weight;
            MergeEdgeProperties(left, right, weight, properties);
            return;
        }

        var leftIndex = _nodeIndex[left];
        var rightIndex = _nodeIndex[right];
        _matrix[leftIndex][rightIndex] = weight;
        _matrix[rightIndex][leftIndex] = weight;
        MergeEdgeProperties(left, right, weight, properties);
    }

    /// <summary>
    /// Remove an existing undirected edge.
    /// </summary>
    public void RemoveEdge(T left, T right)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            if (!_adj.TryGetValue(left, out var leftNeighbors) ||
                !_adj.TryGetValue(right, out var rightNeighbors) ||
                !leftNeighbors.ContainsKey(right))
            {
                throw new KeyNotFoundException($"Edge not found: {left} -- {right}");
            }

            leftNeighbors.Remove(right);
            rightNeighbors.Remove(left);
            _edgeProperties.Remove(NodeOrdering.EdgeKey(left, right));
            return;
        }

        if (!_nodeIndex.TryGetValue(left, out var leftIndex) ||
            !_nodeIndex.TryGetValue(right, out var rightIndex) ||
            _matrix[leftIndex][rightIndex] is null)
        {
            throw new KeyNotFoundException($"Edge not found: {left} -- {right}");
        }

        _matrix[leftIndex][rightIndex] = null;
        _matrix[rightIndex][leftIndex] = null;
        _edgeProperties.Remove(NodeOrdering.EdgeKey(left, right));
    }

    /// <summary>
    /// Test whether an undirected edge currently exists between two nodes.
    /// </summary>
    public bool HasEdge(T left, T right)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            return _adj.TryGetValue(left, out var neighbors) && neighbors.ContainsKey(right);
        }

        return _nodeIndex.TryGetValue(left, out var leftIndex) &&
               _nodeIndex.TryGetValue(right, out var rightIndex) &&
               _matrix[leftIndex][rightIndex] is not null;
    }

    /// <summary>
    /// Look up the stored weight for an existing undirected edge.
    /// </summary>
    public double EdgeWeight(T left, T right)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            if (_adj.TryGetValue(left, out var neighbors) && neighbors.TryGetValue(right, out var weight))
            {
                return weight;
            }

            throw new KeyNotFoundException($"Edge not found: {left} -- {right}");
        }

        if (!_nodeIndex.TryGetValue(left, out var leftIndex) ||
            !_nodeIndex.TryGetValue(right, out var rightIndex) ||
            _matrix[leftIndex][rightIndex] is not double weightValue)
        {
            throw new KeyNotFoundException($"Edge not found: {left} -- {right}");
        }

        return weightValue;
    }

    /// <summary>
    /// Return all graph edges in deterministic order.
    /// </summary>
    public IReadOnlyList<WeightedEdge<T>> Edges()
    {
        var result = new List<WeightedEdge<T>>();

        if (_repr == GraphRepr.AdjacencyList)
        {
            // Each undirected edge is stored twice internally (A -> B and B -> A),
            // so we track a canonical edge key to emit it only once.
            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (var (left, neighbors) in _adj)
            {
                foreach (var (right, weight) in neighbors)
                {
                    if (!seen.Add(NodeOrdering.EdgeKey(left, right)))
                    {
                        continue;
                    }

                    var (first, second) = NodeOrdering.CanonicalEdge(left, right);
                    result.Add(new WeightedEdge<T>(first, second, weight));
                }
            }
        }
        else
        {
            for (var row = 0; row < _nodeList.Count; row++)
            {
                for (var col = row; col < _nodeList.Count; col++)
                {
                    if (_matrix[row][col] is double weight)
                    {
                        result.Add(new WeightedEdge<T>(_nodeList[row], _nodeList[col], weight));
                    }
                }
            }
        }

        result.Sort(static (left, right) =>
        {
            var byWeight = left.Weight.CompareTo(right.Weight);
            if (byWeight != 0)
            {
                return byWeight;
            }

            var byFirst = NodeOrdering.Compare(left.Left, right.Left);
            if (byFirst != 0)
            {
                return byFirst;
            }

            return NodeOrdering.Compare(left.Right, right.Right);
        });

        return result;
    }

    /// <summary>
    /// Return the neighboring nodes of one node in deterministic order.
    /// </summary>
    public IReadOnlyList<T> Neighbors(T node)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            if (!_adj.TryGetValue(node, out var neighbors))
            {
                throw new KeyNotFoundException($"Node not found: {node}");
            }

            return NodeOrdering.Sort(neighbors.Keys);
        }

        if (!_nodeIndex.TryGetValue(node, out var index))
        {
            throw new KeyNotFoundException($"Node not found: {node}");
        }

        var result = new List<T>();
        for (var col = 0; col < _nodeList.Count; col++)
        {
            if (_matrix[index][col] is not null)
            {
                result.Add(_nodeList[col]);
            }
        }

        return NodeOrdering.Sort(result);
    }

    /// <summary>
    /// Return the neighbor-to-weight mapping for one node.
    /// </summary>
    public IReadOnlyDictionary<T, double> NeighborsWeighted(T node)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            if (!_adj.TryGetValue(node, out var neighbors))
            {
                throw new KeyNotFoundException($"Node not found: {node}");
            }

            return new Dictionary<T, double>(neighbors);
        }

        if (!_nodeIndex.TryGetValue(node, out var index))
        {
            throw new KeyNotFoundException($"Node not found: {node}");
        }

        var result = new Dictionary<T, double>();
        for (var col = 0; col < _nodeList.Count; col++)
        {
            if (_matrix[index][col] is double weight)
            {
                result[_nodeList[col]] = weight;
            }
        }

        return result;
    }

    /// <summary>
    /// Count how many neighbors a node has.
    /// </summary>
    public int Degree(T node) => Neighbors(node).Count;

    /// <summary>
    /// Return a copy of the graph-level property bag.
    /// </summary>
    public IReadOnlyDictionary<string, object?> GraphProperties() =>
        new Dictionary<string, object?>(_graphProperties, StringComparer.Ordinal);

    /// <summary>
    /// Add or overwrite one graph-level property.
    /// </summary>
    public void SetGraphProperty(string key, object? value) => _graphProperties[key] = value;

    /// <summary>
    /// Remove one graph-level property if present.
    /// </summary>
    public void RemoveGraphProperty(string key) => _graphProperties.Remove(key);

    /// <summary>
    /// Return a copy of one node's property bag.
    /// </summary>
    public IReadOnlyDictionary<string, object?> NodeProperties(T node)
    {
        if (!HasNode(node))
        {
            throw new KeyNotFoundException($"Node not found: {node}");
        }

        return _nodeProperties.TryGetValue(node, out var properties)
            ? new Dictionary<string, object?>(properties, StringComparer.Ordinal)
            : new Dictionary<string, object?>(StringComparer.Ordinal);
    }

    /// <summary>
    /// Add or overwrite one node property.
    /// </summary>
    public void SetNodeProperty(T node, string key, object? value)
    {
        if (!HasNode(node))
        {
            throw new KeyNotFoundException($"Node not found: {node}");
        }

        if (!_nodeProperties.TryGetValue(node, out var properties))
        {
            properties = new Dictionary<string, object?>(StringComparer.Ordinal);
            _nodeProperties[node] = properties;
        }

        properties[key] = value;
    }

    /// <summary>
    /// Remove one node property if present.
    /// </summary>
    public void RemoveNodeProperty(T node, string key)
    {
        if (!HasNode(node))
        {
            throw new KeyNotFoundException($"Node not found: {node}");
        }

        if (_nodeProperties.TryGetValue(node, out var properties))
        {
            properties.Remove(key);
        }
    }

    /// <summary>
    /// Return a copy of one edge's property bag, including the current weight.
    /// </summary>
    public IReadOnlyDictionary<string, object?> EdgeProperties(T left, T right)
    {
        if (!HasEdge(left, right))
        {
            throw new KeyNotFoundException($"Edge not found: {left} -- {right}");
        }

        var key = NodeOrdering.EdgeKey(left, right);
        var result = _edgeProperties.TryGetValue(key, out var properties)
            ? new Dictionary<string, object?>(properties, StringComparer.Ordinal)
            : new Dictionary<string, object?>(StringComparer.Ordinal);
        result["weight"] = EdgeWeight(left, right);
        return result;
    }

    /// <summary>
    /// Add or overwrite one edge property. Setting <c>weight</c> also updates the edge weight.
    /// </summary>
    public void SetEdgeProperty(T left, T right, string key, object? value)
    {
        if (!HasEdge(left, right))
        {
            throw new KeyNotFoundException($"Edge not found: {left} -- {right}");
        }

        if (key == "weight")
        {
            if (!TryConvertToDouble(value, out var weight))
            {
                throw new ArgumentException("Edge property 'weight' must be numeric.", nameof(value));
            }

            SetEdgeWeight(left, right, weight);
        }

        var edgeKey = NodeOrdering.EdgeKey(left, right);
        if (!_edgeProperties.TryGetValue(edgeKey, out var properties))
        {
            properties = new Dictionary<string, object?>(StringComparer.Ordinal);
            _edgeProperties[edgeKey] = properties;
        }

        properties[key] = value;
    }

    /// <summary>
    /// Remove one edge property if present. Removing <c>weight</c> resets the edge weight to 1.0.
    /// </summary>
    public void RemoveEdgeProperty(T left, T right, string key)
    {
        if (!HasEdge(left, right))
        {
            throw new KeyNotFoundException($"Edge not found: {left} -- {right}");
        }

        if (key == "weight")
        {
            SetEdgeWeight(left, right, 1.0);
            var edgeKey = NodeOrdering.EdgeKey(left, right);
            if (!_edgeProperties.TryGetValue(edgeKey, out var properties))
            {
                properties = new Dictionary<string, object?>(StringComparer.Ordinal);
                _edgeProperties[edgeKey] = properties;
            }

            properties["weight"] = 1.0;
            return;
        }

        if (_edgeProperties.TryGetValue(NodeOrdering.EdgeKey(left, right), out var bag))
        {
            bag.Remove(key);
        }
    }

    /// <summary>
    /// Produce a short human-readable graph summary.
    /// </summary>
    public override string ToString() =>
        $"Graph(nodes={Size}, edges={Edges().Count}, repr={Representation})";

    private void MergeNodeProperties(T node, IReadOnlyDictionary<string, object?>? properties)
    {
        if (!_nodeProperties.TryGetValue(node, out var target))
        {
            target = new Dictionary<string, object?>(StringComparer.Ordinal);
            _nodeProperties[node] = target;
        }

        if (properties is null)
        {
            return;
        }

        foreach (var (key, value) in properties)
        {
            target[key] = value;
        }
    }

    private void MergeEdgeProperties(T left, T right, double weight, IReadOnlyDictionary<string, object?>? properties)
    {
        var edgeKey = NodeOrdering.EdgeKey(left, right);
        if (!_edgeProperties.TryGetValue(edgeKey, out var target))
        {
            target = new Dictionary<string, object?>(StringComparer.Ordinal);
            _edgeProperties[edgeKey] = target;
        }

        if (properties is not null)
        {
            foreach (var (key, value) in properties)
            {
                target[key] = value;
            }
        }

        target["weight"] = weight;
    }

    private void SetEdgeWeight(T left, T right, double weight)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            _adj[left][right] = weight;
            _adj[right][left] = weight;
            return;
        }

        var leftIndex = _nodeIndex[left];
        var rightIndex = _nodeIndex[right];
        _matrix[leftIndex][rightIndex] = weight;
        _matrix[rightIndex][leftIndex] = weight;
    }

    private static bool TryConvertToDouble(object? value, out double result)
    {
        switch (value)
        {
            case byte numeric:
                result = numeric;
                return true;
            case sbyte numeric:
                result = numeric;
                return true;
            case short numeric:
                result = numeric;
                return true;
            case ushort numeric:
                result = numeric;
                return true;
            case int numeric:
                result = numeric;
                return true;
            case uint numeric:
                result = numeric;
                return true;
            case long numeric:
                result = numeric;
                return true;
            case ulong numeric:
                result = numeric;
                return true;
            case float numeric:
                result = numeric;
                return true;
            case double numeric:
                result = numeric;
                return true;
            case decimal numeric:
                result = (double)numeric;
                return true;
            default:
                result = 0;
                return false;
        }
    }
}

/// <summary>
/// Algorithms that operate on <see cref="Graph{T}"/> without caring which
/// internal representation stores the edges.
/// </summary>
public static class GraphAlgorithms
{
    /// <summary>
    /// Breadth-first search explores the graph in "layers" radiating from the
    /// start node: all nodes one hop away, then two hops away, and so on.
    /// </summary>
    public static IReadOnlyList<T> Bfs<T>(Graph<T> graph, T start) where T : notnull
    {
        EnsureNode(graph, start);

        var visited = new HashSet<T> { start };
        var queue = new Queue<T>();
        queue.Enqueue(start);
        var result = new List<T>();

        while (queue.Count > 0)
        {
            var node = queue.Dequeue();
            result.Add(node);

            foreach (var neighbor in graph.Neighbors(node))
            {
                if (visited.Add(neighbor))
                {
                    queue.Enqueue(neighbor);
                }
            }
        }

        return result;
    }

    /// <summary>
    /// Depth-first search follows one branch as far as possible before
    /// backtracking to try the next branch.
    /// </summary>
    public static IReadOnlyList<T> Dfs<T>(Graph<T> graph, T start) where T : notnull
    {
        // DFS uses an explicit stack so we can control the visit order without
        // recursion depth concerns.
        EnsureNode(graph, start);

        var visited = new HashSet<T>();
        var stack = new Stack<T>();
        stack.Push(start);
        var result = new List<T>();

        while (stack.Count > 0)
        {
            var node = stack.Pop();
            if (!visited.Add(node))
            {
                continue;
            }

            result.Add(node);

            foreach (var neighbor in graph.Neighbors(node).Reverse())
            {
                if (!visited.Contains(neighbor))
                {
                    stack.Push(neighbor);
                }
            }
        }

        return result;
    }

    /// <summary>
    /// Report whether every node is reachable from every other node.
    /// </summary>
    public static bool IsConnected<T>(Graph<T> graph) where T : notnull
    {
        if (graph.Size == 0)
        {
            return true;
        }

        var start = NodeOrdering.Sort(graph.Nodes()).First();
        return Bfs(graph, start).Count == graph.Size;
    }

    /// <summary>
    /// Partition the graph into its connected components.
    /// </summary>
    public static IReadOnlyList<HashSet<T>> ConnectedComponents<T>(Graph<T> graph) where T : notnull
    {
        var remaining = new HashSet<T>(graph.Nodes());
        var result = new List<HashSet<T>>();

        while (remaining.Count > 0)
        {
            var start = NodeOrdering.Sort(remaining).First();
            var component = new HashSet<T>(Bfs(graph, start));
            result.Add(component);
            remaining.ExceptWith(component);
        }

        return result;
    }

    /// <summary>
    /// Detect whether an undirected cycle exists anywhere in the graph.
    /// </summary>
    public static bool HasCycle<T>(Graph<T> graph) where T : notnull
    {
        var visited = new HashSet<T>();
        var comparer = EqualityComparer<T>.Default;

        foreach (var start in NodeOrdering.Sort(graph.Nodes()))
        {
            if (visited.Contains(start))
            {
                continue;
            }

            var stack = new Stack<(T Node, bool HasParent, T Parent)>();
            stack.Push((start, false, default!));

            while (stack.Count > 0)
            {
                var frame = stack.Pop();
                if (visited.Contains(frame.Node))
                {
                    continue;
                }

                visited.Add(frame.Node);
                foreach (var neighbor in graph.Neighbors(frame.Node))
                {
                    if (!visited.Contains(neighbor))
                    {
                        stack.Push((neighbor, true, frame.Node));
                    }
                    else if (!frame.HasParent || !comparer.Equals(neighbor, frame.Parent))
                    {
                        return true;
                    }
                }
            }
        }

        return false;
    }

    /// <summary>
    /// Compute the lowest-cost path between two nodes.
    /// For unit-weight graphs this uses BFS; otherwise it uses Dijkstra.
    /// </summary>
    public static IReadOnlyList<T> ShortestPath<T>(Graph<T> graph, T start, T end) where T : notnull
    {
        // Unweighted graphs can use BFS because every hop costs the same.
        // Weighted graphs need Dijkstra so a "longer in hops, cheaper in total"
        // route is still discovered.
        if (!graph.HasNode(start) || !graph.HasNode(end))
        {
            return Array.Empty<T>();
        }

        if (EqualityComparer<T>.Default.Equals(start, end))
        {
            return new[] { start };
        }

        return graph.Edges().All(static edge => edge.Weight == 1.0)
            ? BfsShortestPath(graph, start, end)
            : DijkstraShortestPath(graph, start, end);
    }

    /// <summary>
    /// Build a minimum spanning tree using Kruskal's algorithm.
    /// </summary>
    public static IReadOnlyList<WeightedEdge<T>> MinimumSpanningTree<T>(Graph<T> graph) where T : notnull
    {
        // We use Kruskal's algorithm:
        //   sort edges by weight
        //   keep adding the next-lightest edge that does not create a cycle
        // A union-find structure makes the cycle test efficient.
        if (graph.Size <= 1)
        {
            return Array.Empty<WeightedEdge<T>>();
        }

        if (!IsConnected(graph))
        {
            throw new InvalidOperationException("minimumSpanningTree: graph is not connected");
        }

        var edges = graph.Edges()
            .OrderBy(static edge => edge.Weight)
            .ThenBy(static edge => NodeOrdering.CanonicalKey(edge.Left), StringComparer.Ordinal)
            .ThenBy(static edge => NodeOrdering.CanonicalKey(edge.Right), StringComparer.Ordinal)
            .ToList();

        var unionFind = new UnionFind<T>(graph.Nodes());
        var result = new List<WeightedEdge<T>>();

        foreach (var edge in edges)
        {
            if (unionFind.Find(edge.Left).Equals(unionFind.Find(edge.Right)))
            {
                continue;
            }

            unionFind.Union(edge.Left, edge.Right);
            result.Add(edge);
            if (result.Count == graph.Size - 1)
            {
                break;
            }
        }

        return result;
    }

    private static IReadOnlyList<T> BfsShortestPath<T>(Graph<T> graph, T start, T end) where T : notnull
    {
        var parents = new Dictionary<T, T>();
        var seen = new HashSet<T> { start };
        var queue = new Queue<T>();
        queue.Enqueue(start);

        while (queue.Count > 0)
        {
            var node = queue.Dequeue();
            if (EqualityComparer<T>.Default.Equals(node, end))
            {
                break;
            }

            foreach (var neighbor in graph.Neighbors(node))
            {
                if (!seen.Add(neighbor))
                {
                    continue;
                }

                parents[neighbor] = node;
                queue.Enqueue(neighbor);
            }
        }

        return parents.ContainsKey(end) ? BuildPath(parents, start, end) : Array.Empty<T>();
    }

    private static IReadOnlyList<T> DijkstraShortestPath<T>(Graph<T> graph, T start, T end) where T : notnull
    {
        var distances = graph.Nodes().ToDictionary(static node => node, static _ => double.PositiveInfinity);
        var parents = new Dictionary<T, T>();
        var queue = new PriorityQueue<T, (double Distance, int Sequence)>();
        var sequence = 0;

        distances[start] = 0.0;
        queue.Enqueue(start, (0.0, sequence));

        while (queue.TryDequeue(out var node, out var priority))
        {
            if (priority.Distance > distances[node])
            {
                continue;
            }

            if (EqualityComparer<T>.Default.Equals(node, end))
            {
                break;
            }

            foreach (var pair in graph.NeighborsWeighted(node)
                         .OrderBy(static item => NodeOrdering.CanonicalKey(item.Key), StringComparer.Ordinal))
            {
                var nextDistance = distances[node] + pair.Value;
                if (nextDistance >= distances.GetValueOrDefault(pair.Key, double.PositiveInfinity))
                {
                    continue;
                }

                distances[pair.Key] = nextDistance;
                parents[pair.Key] = node;
                sequence += 1;
                queue.Enqueue(pair.Key, (nextDistance, sequence));
            }
        }

        return double.IsPositiveInfinity(distances[end]) ? Array.Empty<T>() : BuildPath(parents, start, end);
    }

    private static List<T> BuildPath<T>(IDictionary<T, T> parents, T start, T end) where T : notnull
    {
        var comparer = EqualityComparer<T>.Default;
        var path = new List<T>();
        var current = end;

        while (true)
        {
            path.Add(current);
            if (comparer.Equals(current, start))
            {
                break;
            }

            if (!parents.TryGetValue(current, out var parent))
            {
                return new List<T>();
            }

            current = parent;
        }

        path.Reverse();
        return path;
    }

    private static void EnsureNode<T>(Graph<T> graph, T start) where T : notnull
    {
        if (!graph.HasNode(start))
        {
            throw new KeyNotFoundException($"Node not found: {start}");
        }
    }
}

internal sealed class UnionFind<T> where T : notnull
{
    // Union-find keeps track of which nodes already belong to the same partial
    // tree while Kruskal's algorithm is building the spanning forest.
    private readonly Dictionary<T, T> _parent = new();
    private readonly Dictionary<T, int> _rank = new();

    public UnionFind(IEnumerable<T> nodes)
    {
        foreach (var node in nodes)
        {
            _parent[node] = node;
            _rank[node] = 0;
        }
    }

    public T Find(T node)
    {
        var parent = _parent[node];
        if (!EqualityComparer<T>.Default.Equals(parent, node))
        {
            _parent[node] = Find(parent);
        }

        return _parent[node];
    }

    public void Union(T left, T right)
    {
        var leftRoot = Find(left);
        var rightRoot = Find(right);
        if (EqualityComparer<T>.Default.Equals(leftRoot, rightRoot))
        {
            return;
        }

        var leftRank = _rank[leftRoot];
        var rightRank = _rank[rightRoot];
        if (leftRank < rightRank)
        {
            (leftRoot, rightRoot) = (rightRoot, leftRoot);
            (leftRank, rightRank) = (rightRank, leftRank);
        }

        _parent[rightRoot] = leftRoot;
        if (leftRank == rightRank)
        {
            _rank[leftRoot] = leftRank + 1;
        }
    }
}
