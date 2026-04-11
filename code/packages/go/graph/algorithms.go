package graph

import (
	"container/heap"
	"errors"
	"sort"
)

func BFS(g *Graph, start string) ([]string, error) {
	if !g.HasNode(start) {
		return nil, &NodeNotFoundError{Node: start}
	}

	visited := map[string]bool{start: true}
	queue := []string{start}
	result := make([]string, 0, g.Size())

	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		result = append(result, node)

		neighbors, _ := g.Neighbors(node)
		for _, neighbor := range neighbors {
			if !visited[neighbor] {
				visited[neighbor] = true
				queue = append(queue, neighbor)
			}
		}
	}

	return result, nil
}

func DFS(g *Graph, start string) ([]string, error) {
	if !g.HasNode(start) {
		return nil, &NodeNotFoundError{Node: start}
	}

	visited := map[string]bool{}
	stack := []string{start}
	result := make([]string, 0, g.Size())

	for len(stack) > 0 {
		node := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		if visited[node] {
			continue
		}

		visited[node] = true
		result = append(result, node)

		neighbors, _ := g.Neighbors(node)
		for i := len(neighbors) - 1; i >= 0; i-- {
			neighbor := neighbors[i]
			if !visited[neighbor] {
				stack = append(stack, neighbor)
			}
		}
	}

	return result, nil
}

func IsConnected(g *Graph) bool {
	nodes := g.Nodes()
	if len(nodes) == 0 {
		return true
	}
	visited, _ := BFS(g, nodes[0])
	return len(visited) == len(nodes)
}

func ConnectedComponents(g *Graph) [][]string {
	nodes := g.Nodes()
	visited := map[string]bool{}
	components := make([][]string, 0)

	for _, node := range nodes {
		if visited[node] {
			continue
		}

		component, _ := BFS(g, node)
		for _, member := range component {
			visited[member] = true
		}
		components = append(components, component)
	}

	return components
}

func HasCycle(g *Graph) bool {
	visited := map[string]bool{}
	var dfs func(node, parent string) bool
	dfs = func(node, parent string) bool {
		visited[node] = true
		neighbors, _ := g.Neighbors(node)
		for _, neighbor := range neighbors {
			if !visited[neighbor] {
				if dfs(neighbor, node) {
					return true
				}
			} else if neighbor != parent {
				return true
			}
		}
		return false
	}

	for _, node := range g.Nodes() {
		if !visited[node] && dfs(node, "") {
			return true
		}
	}
	return false
}

type shortestPathItem struct {
	node     string
	distance float64
	index    int
}

type shortestPathHeap []*shortestPathItem

func (h shortestPathHeap) Len() int { return len(h) }
func (h shortestPathHeap) Less(i, j int) bool {
	if h[i].distance != h[j].distance {
		return h[i].distance < h[j].distance
	}
	return h[i].node < h[j].node
}
func (h shortestPathHeap) Swap(i, j int) {
	h[i], h[j] = h[j], h[i]
	h[i].index = i
	h[j].index = j
}
func (h *shortestPathHeap) Push(x any) {
	item := x.(*shortestPathItem)
	item.index = len(*h)
	*h = append(*h, item)
}
func (h *shortestPathHeap) Pop() any {
	old := *h
	n := len(old)
	item := old[n-1]
	*h = old[:n-1]
	return item
}

func ShortestPath(g *Graph, start, end string) ([]string, error) {
	if !g.HasNode(start) {
		return nil, &NodeNotFoundError{Node: start}
	}
	if !g.HasNode(end) {
		return nil, &NodeNotFoundError{Node: end}
	}
	if start == end {
		return []string{start}, nil
	}

	dist := make(map[string]float64, g.Size())
	prev := make(map[string]string, g.Size())
	for _, node := range g.Nodes() {
		dist[node] = 1e308
	}
	dist[start] = 0

	pq := shortestPathHeap{&shortestPathItem{node: start, distance: 0}}
	heap.Init(&pq)

	for pq.Len() > 0 {
		item := heap.Pop(&pq).(*shortestPathItem)
		if item.distance > dist[item.node] {
			continue
		}
		if item.node == end {
			break
		}
		neighbors, _ := g.NeighborsWeighted(item.node)
		keys := make([]string, 0, len(neighbors))
		for neighbor := range neighbors {
			keys = append(keys, neighbor)
		}
		sort.Strings(keys)
		for _, neighbor := range keys {
			alt := dist[item.node] + neighbors[neighbor]
			if alt < dist[neighbor] {
				dist[neighbor] = alt
				prev[neighbor] = item.node
				heap.Push(&pq, &shortestPathItem{node: neighbor, distance: alt})
			}
		}
	}

	if dist[end] == 1e308 {
		return []string{}, nil
	}

	path := []string{end}
	for current := end; current != start; {
		parent, ok := prev[current]
		if !ok {
			return []string{}, nil
		}
		path = append(path, parent)
		current = parent
	}

	for i, j := 0, len(path)-1; i < j; i, j = i+1, j-1 {
		path[i], path[j] = path[j], path[i]
	}
	return path, nil
}

type disjointSet struct {
	parent map[string]string
	rank   map[string]int
}

func newDisjointSet(nodes []string) *disjointSet {
	ds := &disjointSet{
		parent: make(map[string]string, len(nodes)),
		rank:   make(map[string]int, len(nodes)),
	}
	for _, node := range nodes {
		ds.parent[node] = node
		ds.rank[node] = 0
	}
	return ds
}

func (ds *disjointSet) find(node string) string {
	if ds.parent[node] != node {
		ds.parent[node] = ds.find(ds.parent[node])
	}
	return ds.parent[node]
}

func (ds *disjointSet) union(left, right string) {
	leftRoot := ds.find(left)
	rightRoot := ds.find(right)
	if leftRoot == rightRoot {
		return
	}
	if ds.rank[leftRoot] < ds.rank[rightRoot] {
		ds.parent[leftRoot] = rightRoot
	} else if ds.rank[leftRoot] > ds.rank[rightRoot] {
		ds.parent[rightRoot] = leftRoot
	} else {
		ds.parent[rightRoot] = leftRoot
		ds.rank[leftRoot]++
	}
}

func MinimumSpanningTree(g *Graph) ([]WeightedEdge, error) {
	if g.Size() == 0 || g.Size() == 1 {
		return []WeightedEdge{}, nil
	}
	if !IsConnected(g) {
		return nil, errors.New("graph is not connected")
	}

	edges := g.Edges()
	ds := newDisjointSet(g.Nodes())
	result := make([]WeightedEdge, 0, g.Size()-1)

	for _, edge := range edges {
		if ds.find(edge.Left) == ds.find(edge.Right) {
			continue
		}
		ds.union(edge.Left, edge.Right)
		result = append(result, edge)
		if len(result) == g.Size()-1 {
			break
		}
	}

	return result, nil
}
