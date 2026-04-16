namespace CodingAdventures.DirectedGraph;

public sealed class CycleError : Exception
{
    public CycleError(string message, IReadOnlyList<string> cycle) : base(message)
    {
        Cycle = cycle;
    }

    public IReadOnlyList<string> Cycle { get; }
}

public sealed class NodeNotFoundError : Exception
{
    public NodeNotFoundError(string node) : base($"Node not found: \"{node}\"")
    {
        Node = node;
    }

    public string Node { get; }
}

public sealed class EdgeNotFoundError : Exception
{
    public EdgeNotFoundError(string fromNode, string toNode) : base($"Edge not found: \"{fromNode}\" -> \"{toNode}\"")
    {
        FromNode = fromNode;
        ToNode = toNode;
    }

    public string FromNode { get; }
    public string ToNode { get; }
}

public sealed class Graph
{
    private readonly Dictionary<string, HashSet<string>> _forward = new(StringComparer.Ordinal);
    private readonly Dictionary<string, HashSet<string>> _reverse = new(StringComparer.Ordinal);

    public Graph(bool allowSelfLoops = false)
    {
        AllowSelfLoops = allowSelfLoops;
    }

    public bool AllowSelfLoops { get; }
    public int Size => _forward.Count;

    public void AddNode(string node)
    {
        _forward.TryAdd(node, new HashSet<string>(StringComparer.Ordinal));
        _reverse.TryAdd(node, new HashSet<string>(StringComparer.Ordinal));
    }

    public void RemoveNode(string node)
    {
        if (!_forward.ContainsKey(node))
        {
            throw new NodeNotFoundError(node);
        }

        foreach (var successor in _forward[node].ToArray())
        {
            _reverse[successor].Remove(node);
        }

        foreach (var predecessor in _reverse[node].ToArray())
        {
            _forward[predecessor].Remove(node);
        }

        _forward.Remove(node);
        _reverse.Remove(node);
    }

    public bool HasNode(string node) => _forward.ContainsKey(node);
    public IReadOnlyList<string> Nodes() => _forward.Keys.ToList();

    public void AddEdge(string fromNode, string toNode)
    {
        if (fromNode == toNode && !AllowSelfLoops)
        {
            throw new ArgumentException($"Self-loops are not allowed: \"{fromNode}\" -> \"{toNode}\"");
        }

        AddNode(fromNode);
        AddNode(toNode);
        _forward[fromNode].Add(toNode);
        _reverse[toNode].Add(fromNode);
    }

    public void RemoveEdge(string fromNode, string toNode)
    {
        if (!HasNode(fromNode))
        {
            throw new NodeNotFoundError(fromNode);
        }

        if (!HasNode(toNode) || !_forward[fromNode].Contains(toNode))
        {
            throw new EdgeNotFoundError(fromNode, toNode);
        }

        _forward[fromNode].Remove(toNode);
        _reverse[toNode].Remove(fromNode);
    }

    public bool HasEdge(string fromNode, string toNode) => HasNode(fromNode) && _forward[fromNode].Contains(toNode);

    public IReadOnlyList<string> Successors(string node)
    {
        if (!HasNode(node))
        {
            throw new NodeNotFoundError(node);
        }

        return _forward[node].ToList();
    }

    public IReadOnlyList<string> Predecessors(string node)
    {
        if (!HasNode(node))
        {
            throw new NodeNotFoundError(node);
        }

        return _reverse[node].ToList();
    }

    public IReadOnlyList<(string From, string To)> Edges() =>
        _forward.SelectMany(pair => pair.Value.Select(target => (pair.Key, target))).ToList();

    public ISet<string> TransitiveClosure(string startNode) => Reach(startNode, Successors);
    public ISet<string> TransitiveDependents(string startNode) => Reach(startNode, Predecessors);

    public IReadOnlyList<string> TopologicalSort()
    {
        var indegree = _forward.Keys.ToDictionary(node => node, node => _reverse[node].Count, StringComparer.Ordinal);
        var queue = new Queue<string>(indegree.Where(pair => pair.Value == 0).Select(pair => pair.Key));
        var order = new List<string>();

        while (queue.Count > 0)
        {
            var node = queue.Dequeue();
            order.Add(node);
            foreach (var successor in _forward[node])
            {
                indegree[successor]--;
                if (indegree[successor] == 0)
                {
                    queue.Enqueue(successor);
                }
            }
        }

        if (order.Count != _forward.Count)
        {
            throw new CycleError("Graph contains a cycle", FindCycle());
        }

        return order;
    }

    public IReadOnlyList<IReadOnlyList<string>> IndependentGroups()
    {
        var indegree = _forward.Keys.ToDictionary(node => node, node => _reverse[node].Count, StringComparer.Ordinal);
        var remaining = new HashSet<string>(_forward.Keys, StringComparer.Ordinal);
        var result = new List<IReadOnlyList<string>>();

        while (remaining.Count > 0)
        {
            var layer = remaining.Where(node => indegree[node] == 0).OrderBy(node => node, StringComparer.Ordinal).ToList();
            if (layer.Count == 0)
            {
                throw new CycleError("Graph contains a cycle", FindCycle());
            }

            result.Add(layer);
            foreach (var node in layer)
            {
                remaining.Remove(node);
                foreach (var successor in _forward[node])
                {
                    indegree[successor]--;
                }
            }
        }

        return result;
    }

    public IReadOnlyList<string> AffectedNodes(IEnumerable<string> changedNodes)
    {
        var affected = new HashSet<string>(StringComparer.Ordinal);
        foreach (var node in changedNodes)
        {
            if (!HasNode(node))
            {
                continue;
            }

            affected.Add(node);
            affected.UnionWith(TransitiveClosure(node));
        }

        return TopologicalSort().Where(affected.Contains).ToList();
    }

    private HashSet<string> Reach(string startNode, Func<string, IReadOnlyList<string>> next)
    {
        if (!HasNode(startNode))
        {
            throw new NodeNotFoundError(startNode);
        }

        var visited = new HashSet<string>(StringComparer.Ordinal);
        var stack = new Stack<string>();
        stack.Push(startNode);
        while (stack.Count > 0)
        {
            var node = stack.Pop();
            foreach (var neighbor in next(node))
            {
                if (visited.Add(neighbor))
                {
                    stack.Push(neighbor);
                }
            }
        }

        return visited;
    }

    private List<string> FindCycle()
    {
        var visited = new HashSet<string>(StringComparer.Ordinal);
        var onStack = new HashSet<string>(StringComparer.Ordinal);
        var parent = new Dictionary<string, string?>(StringComparer.Ordinal);

        foreach (var node in _forward.Keys)
        {
            var cycle = FindCycleFrom(node, visited, onStack, parent);
            if (cycle.Count > 0)
            {
                return cycle;
            }
        }

        return [];
    }

    private List<string> FindCycleFrom(
        string node,
        HashSet<string> visited,
        HashSet<string> onStack,
        Dictionary<string, string?> parent)
    {
        if (onStack.Contains(node))
        {
            var cycle = new List<string> { node };
            var current = parent.GetValueOrDefault(node);
            while (current is not null && current != node)
            {
                cycle.Insert(0, current);
                current = parent.GetValueOrDefault(current);
            }

            cycle.Add(node);
            return cycle;
        }

        if (!visited.Add(node))
        {
            return [];
        }

        onStack.Add(node);
        foreach (var successor in _forward[node])
        {
            parent[successor] = node;
            var cycle = FindCycleFrom(successor, visited, onStack, parent);
            if (cycle.Count > 0)
            {
                return cycle;
            }
        }

        onStack.Remove(node);
        return [];
    }
}

public sealed class LabeledDirectedGraph
{
    private readonly Dictionary<string, Dictionary<string, HashSet<string>>> _labels = new(StringComparer.Ordinal);
    private readonly Graph _graph;

    public LabeledDirectedGraph(bool allowSelfLoops = false)
    {
        _graph = new Graph(allowSelfLoops);
    }

    public void AddNode(string node)
    {
        _graph.AddNode(node);
        _labels.TryAdd(node, new Dictionary<string, HashSet<string>>(StringComparer.Ordinal));
    }

    public void AddEdge(string fromNode, string toNode, string label)
    {
        AddNode(fromNode);
        AddNode(toNode);
        _graph.AddEdge(fromNode, toNode);
        if (!_labels[fromNode].TryGetValue(toNode, out var labels))
        {
            labels = new HashSet<string>(StringComparer.Ordinal);
            _labels[fromNode][toNode] = labels;
        }

        labels.Add(label);
    }

    public IReadOnlyList<string> Labels(string fromNode, string toNode)
    {
        if (_labels.TryGetValue(fromNode, out var targets) && targets.TryGetValue(toNode, out var labels))
        {
            return labels.ToList();
        }

        return [];
    }

    public IReadOnlyList<string> TopologicalSort() => _graph.TopologicalSort();
    public ISet<string> TransitiveClosure(string startNode) => _graph.TransitiveClosure(startNode);
}
