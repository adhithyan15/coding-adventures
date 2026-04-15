using System.Text.Json;

namespace CodingAdventures.Graph;

public enum GraphRepr
{
    AdjacencyList,
    AdjacencyMatrix,
}

public readonly record struct WeightedEdge<T>(T Left, T Right, double Weight) where T : notnull;

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

public sealed class Graph<T> where T : notnull
{
    private readonly GraphRepr _repr;
    private readonly Dictionary<T, Dictionary<T, double>> _adj = new();
    private readonly List<T> _nodeList = new();
    private readonly Dictionary<T, int> _nodeIndex = new();
    private readonly List<List<double?>> _matrix = new();

    public Graph(GraphRepr repr = GraphRepr.AdjacencyList)
    {
        _repr = repr;
    }

    public GraphRepr Representation => _repr;

    public int Size => _repr == GraphRepr.AdjacencyList ? _adj.Count : _nodeList.Count;

    public void AddNode(T node)
    {
        if (_repr == GraphRepr.AdjacencyList)
        {
            if (!_adj.ContainsKey(node))
            {
                _adj[node] = new Dictionary<T, double>();
            }

            return;
        }

        if (_nodeIndex.ContainsKey(node))
        {
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
    }

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
            }

            _adj.Remove(node);
            return;
        }

        if (!_nodeIndex.TryGetValue(node, out var index))
        {
            throw new KeyNotFoundException($"Node not found: {node}");
        }

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

    public bool HasNode(T node) =>
        _repr == GraphRepr.AdjacencyList ? _adj.ContainsKey(node) : _nodeIndex.ContainsKey(node);

    public IReadOnlyCollection<T> Nodes() =>
        _repr == GraphRepr.AdjacencyList
            ? new HashSet<T>(_adj.Keys)
            : new HashSet<T>(_nodeList);

    public void AddEdge(T left, T right, double weight = 1.0)
    {
        AddNode(left);
        AddNode(right);

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
    }

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

    public IReadOnlyList<WeightedEdge<T>> Edges()
    {
        var result = new List<WeightedEdge<T>>();

        if (_repr == GraphRepr.AdjacencyList)
        {
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

    public int Degree(T node) => Neighbors(node).Count;

    public override string ToString() =>
        $"Graph(nodes={Size}, edges={Edges().Count}, repr={Representation})";
}

public static class GraphAlgorithms
{
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

    public static IReadOnlyList<T> Dfs<T>(Graph<T> graph, T start) where T : notnull
    {
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

    public static bool IsConnected<T>(Graph<T> graph) where T : notnull
    {
        if (graph.Size == 0)
        {
            return true;
        }

        var start = NodeOrdering.Sort(graph.Nodes()).First();
        return Bfs(graph, start).Count == graph.Size;
    }

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

    public static IReadOnlyList<T> ShortestPath<T>(Graph<T> graph, T start, T end) where T : notnull
    {
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

    public static IReadOnlyList<WeightedEdge<T>> MinimumSpanningTree<T>(Graph<T> graph) where T : notnull
    {
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
