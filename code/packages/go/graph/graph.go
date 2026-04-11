package graph

import (
	"fmt"
	"sort"
)

type GraphRepr string

const (
	AdjacencyList   GraphRepr = "adjacency_list"
	AdjacencyMatrix GraphRepr = "adjacency_matrix"
)

type WeightedEdge struct {
	Left   string
	Right  string
	Weight float64
}

type Graph struct {
	repr      GraphRepr
	adj       map[string]map[string]float64
	nodeList  []string
	nodeIndex map[string]int
	matrix    [][]float64
}

func New(repr GraphRepr) *Graph {
	if repr == "" {
		repr = AdjacencyList
	}
	return &Graph{
		repr:      repr,
		adj:       make(map[string]map[string]float64),
		nodeList:  []string{},
		nodeIndex: make(map[string]int),
		matrix:    [][]float64{},
	}
}

func (g *Graph) Repr() GraphRepr {
	return g.repr
}

func (g *Graph) AddNode(node string) {
	if g.repr == AdjacencyList {
		if _, ok := g.adj[node]; !ok {
			g.adj[node] = make(map[string]float64)
		}
		return
	}

	if _, ok := g.nodeIndex[node]; ok {
		return
	}

	index := len(g.nodeList)
	g.nodeList = append(g.nodeList, node)
	g.nodeIndex[node] = index
	for i := range g.matrix {
		g.matrix[i] = append(g.matrix[i], 0)
	}
	g.matrix = append(g.matrix, make([]float64, index+1))
}

func (g *Graph) RemoveNode(node string) error {
	if g.repr == AdjacencyList {
		neighbors, ok := g.adj[node]
		if !ok {
			return &NodeNotFoundError{Node: node}
		}
		for neighbor := range neighbors {
			delete(g.adj[neighbor], node)
		}
		delete(g.adj, node)
		return nil
	}

	index, ok := g.nodeIndex[node]
	if !ok {
		return &NodeNotFoundError{Node: node}
	}

	delete(g.nodeIndex, node)
	g.nodeList = append(g.nodeList[:index], g.nodeList[index+1:]...)
	g.matrix = append(g.matrix[:index], g.matrix[index+1:]...)
	for i := range g.matrix {
		g.matrix[i] = append(g.matrix[i][:index], g.matrix[i][index+1:]...)
	}
	for i := index; i < len(g.nodeList); i++ {
		g.nodeIndex[g.nodeList[i]] = i
	}
	return nil
}

func (g *Graph) HasNode(node string) bool {
	if g.repr == AdjacencyList {
		_, ok := g.adj[node]
		return ok
	}
	_, ok := g.nodeIndex[node]
	return ok
}

func (g *Graph) Nodes() []string {
	if g.repr == AdjacencyList {
		nodes := make([]string, 0, len(g.adj))
		for node := range g.adj {
			nodes = append(nodes, node)
		}
		sort.Strings(nodes)
		return nodes
	}
	return append([]string(nil), g.nodeList...)
}

func (g *Graph) AddEdge(left, right string, weight float64) {
	if weight == 0 {
		weight = 1
	}
	g.AddNode(left)
	g.AddNode(right)

	if g.repr == AdjacencyList {
		g.adj[left][right] = weight
		g.adj[right][left] = weight
		return
	}

	leftIndex := g.nodeIndex[left]
	rightIndex := g.nodeIndex[right]
	g.matrix[leftIndex][rightIndex] = weight
	g.matrix[rightIndex][leftIndex] = weight
}

func (g *Graph) RemoveEdge(left, right string) error {
	if g.repr == AdjacencyList {
		neighbors, ok := g.adj[left]
		if !ok {
			return &EdgeNotFoundError{Left: left, Right: right}
		}
		if _, ok := neighbors[right]; !ok {
			return &EdgeNotFoundError{Left: left, Right: right}
		}
		delete(g.adj[left], right)
		delete(g.adj[right], left)
		return nil
	}

	leftIndex, okLeft := g.nodeIndex[left]
	rightIndex, okRight := g.nodeIndex[right]
	if !okLeft || !okRight || g.matrix[leftIndex][rightIndex] == 0 {
		return &EdgeNotFoundError{Left: left, Right: right}
	}
	g.matrix[leftIndex][rightIndex] = 0
	g.matrix[rightIndex][leftIndex] = 0
	return nil
}

func (g *Graph) HasEdge(left, right string) bool {
	if g.repr == AdjacencyList {
		if neighbors, ok := g.adj[left]; ok {
			_, ok = neighbors[right]
			return ok
		}
		return false
	}

	leftIndex, okLeft := g.nodeIndex[left]
	rightIndex, okRight := g.nodeIndex[right]
	if !okLeft || !okRight {
		return false
	}
	return g.matrix[leftIndex][rightIndex] != 0
}

func (g *Graph) EdgeWeight(left, right string) (float64, error) {
	if g.repr == AdjacencyList {
		if neighbors, ok := g.adj[left]; ok {
			if weight, ok := neighbors[right]; ok {
				return weight, nil
			}
		}
		return 0, &EdgeNotFoundError{Left: left, Right: right}
	}

	leftIndex, okLeft := g.nodeIndex[left]
	rightIndex, okRight := g.nodeIndex[right]
	if !okLeft || !okRight || g.matrix[leftIndex][rightIndex] == 0 {
		return 0, &EdgeNotFoundError{Left: left, Right: right}
	}
	return g.matrix[leftIndex][rightIndex], nil
}

func (g *Graph) Edges() []WeightedEdge {
	result := make([]WeightedEdge, 0)
	seen := make(map[string]bool)

	if g.repr == AdjacencyList {
		for left, neighbors := range g.adj {
			for right, weight := range neighbors {
				keyLeft, keyRight := canonicalEndpoints(left, right)
				key := keyLeft + "\x00" + keyRight
				if seen[key] {
					continue
				}
				seen[key] = true
				result = append(result, WeightedEdge{Left: keyLeft, Right: keyRight, Weight: weight})
			}
		}
	} else {
		for row := 0; row < len(g.nodeList); row++ {
			for col := row; col < len(g.nodeList); col++ {
				if g.matrix[row][col] != 0 {
					result = append(result, WeightedEdge{
						Left:   g.nodeList[row],
						Right:  g.nodeList[col],
						Weight: g.matrix[row][col],
					})
				}
			}
		}
	}

	sort.Slice(result, func(i, j int) bool {
		if result[i].Weight != result[j].Weight {
			return result[i].Weight < result[j].Weight
		}
		if result[i].Left != result[j].Left {
			return result[i].Left < result[j].Left
		}
		return result[i].Right < result[j].Right
	})
	return result
}

func (g *Graph) Neighbors(node string) ([]string, error) {
	if g.repr == AdjacencyList {
		neighbors, ok := g.adj[node]
		if !ok {
			return nil, &NodeNotFoundError{Node: node}
		}
		result := make([]string, 0, len(neighbors))
		for neighbor := range neighbors {
			result = append(result, neighbor)
		}
		sort.Strings(result)
		return result, nil
	}

	index, ok := g.nodeIndex[node]
	if !ok {
		return nil, &NodeNotFoundError{Node: node}
	}
	result := make([]string, 0)
	for col, weight := range g.matrix[index] {
		if weight != 0 {
			result = append(result, g.nodeList[col])
		}
	}
	sort.Strings(result)
	return result, nil
}

func (g *Graph) NeighborsWeighted(node string) (map[string]float64, error) {
	if g.repr == AdjacencyList {
		neighbors, ok := g.adj[node]
		if !ok {
			return nil, &NodeNotFoundError{Node: node}
		}
		result := make(map[string]float64, len(neighbors))
		for neighbor, weight := range neighbors {
			result[neighbor] = weight
		}
		return result, nil
	}

	index, ok := g.nodeIndex[node]
	if !ok {
		return nil, &NodeNotFoundError{Node: node}
	}
	result := make(map[string]float64)
	for col, weight := range g.matrix[index] {
		if weight != 0 {
			result[g.nodeList[col]] = weight
		}
	}
	return result, nil
}

func (g *Graph) Degree(node string) (int, error) {
	neighbors, err := g.Neighbors(node)
	if err != nil {
		return 0, err
	}
	return len(neighbors), nil
}

func (g *Graph) Size() int {
	if g.repr == AdjacencyList {
		return len(g.adj)
	}
	return len(g.nodeList)
}

func (g *Graph) String() string {
	return fmt.Sprintf("Graph(nodes=%d, edges=%d, repr=%s)", g.Size(), len(g.Edges()), g.repr)
}

func canonicalEndpoints(left, right string) (string, string) {
	if left <= right {
		return left, right
	}
	return right, left
}
