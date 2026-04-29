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

type PropertyValue any
type PropertyBag map[string]PropertyValue

type Graph struct {
	repr            GraphRepr
	adj             map[string]map[string]float64
	nodeList        []string
	nodeIndex       map[string]int
	matrix          [][]float64
	graphProperties PropertyBag
	nodeProperties  map[string]PropertyBag
	edgeProperties  map[string]PropertyBag
}

func New(repr GraphRepr) *Graph {
	if repr == "" {
		repr = AdjacencyList
	}
	return &Graph{
		repr:            repr,
		adj:             make(map[string]map[string]float64),
		nodeList:        []string{},
		nodeIndex:       make(map[string]int),
		matrix:          [][]float64{},
		graphProperties: make(PropertyBag),
		nodeProperties:  make(map[string]PropertyBag),
		edgeProperties:  make(map[string]PropertyBag),
	}
}

func (g *Graph) Repr() GraphRepr {
	return g.repr
}

func (g *Graph) AddNode(node string, properties ...PropertyBag) {
	if g.repr == AdjacencyList {
		if _, ok := g.adj[node]; !ok {
			g.adj[node] = make(map[string]float64)
			g.nodeProperties[node] = make(PropertyBag)
		}
		g.mergeNodeProperties(node, properties...)
		return
	}

	if _, ok := g.nodeIndex[node]; ok {
		g.mergeNodeProperties(node, properties...)
		return
	}

	index := len(g.nodeList)
	g.nodeList = append(g.nodeList, node)
	g.nodeIndex[node] = index
	g.nodeProperties[node] = make(PropertyBag)
	g.mergeNodeProperties(node, properties...)
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
			delete(g.edgeProperties, edgeKey(node, neighbor))
		}
		delete(g.adj, node)
		delete(g.nodeProperties, node)
		return nil
	}

	index, ok := g.nodeIndex[node]
	if !ok {
		return &NodeNotFoundError{Node: node}
	}
	for _, other := range g.nodeList {
		delete(g.edgeProperties, edgeKey(node, other))
	}

	delete(g.nodeIndex, node)
	delete(g.nodeProperties, node)
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

func (g *Graph) AddEdge(left, right string, weight float64, properties ...PropertyBag) {
	if weight == 0 {
		weight = 1
	}
	g.AddNode(left)
	g.AddNode(right)

	if g.repr == AdjacencyList {
		g.adj[left][right] = weight
		g.adj[right][left] = weight
		g.mergeEdgeProperties(left, right, weight, properties...)
		return
	}

	leftIndex := g.nodeIndex[left]
	rightIndex := g.nodeIndex[right]
	g.matrix[leftIndex][rightIndex] = weight
	g.matrix[rightIndex][leftIndex] = weight
	g.mergeEdgeProperties(left, right, weight, properties...)
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
		delete(g.edgeProperties, edgeKey(left, right))
		return nil
	}

	leftIndex, okLeft := g.nodeIndex[left]
	rightIndex, okRight := g.nodeIndex[right]
	if !okLeft || !okRight || g.matrix[leftIndex][rightIndex] == 0 {
		return &EdgeNotFoundError{Left: left, Right: right}
	}
	g.matrix[leftIndex][rightIndex] = 0
	g.matrix[rightIndex][leftIndex] = 0
	delete(g.edgeProperties, edgeKey(left, right))
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

func (g *Graph) GraphProperties() PropertyBag {
	return copyPropertyBag(g.graphProperties)
}

func (g *Graph) SetGraphProperty(key string, value PropertyValue) {
	g.graphProperties[key] = value
}

func (g *Graph) RemoveGraphProperty(key string) {
	delete(g.graphProperties, key)
}

func (g *Graph) NodeProperties(node string) (PropertyBag, error) {
	if !g.HasNode(node) {
		return nil, &NodeNotFoundError{Node: node}
	}
	return copyPropertyBag(g.nodeProperties[node]), nil
}

func (g *Graph) SetNodeProperty(node, key string, value PropertyValue) error {
	if !g.HasNode(node) {
		return &NodeNotFoundError{Node: node}
	}
	g.nodeProperties[node][key] = value
	return nil
}

func (g *Graph) RemoveNodeProperty(node, key string) error {
	if !g.HasNode(node) {
		return &NodeNotFoundError{Node: node}
	}
	delete(g.nodeProperties[node], key)
	return nil
}

func (g *Graph) EdgeProperties(left, right string) (PropertyBag, error) {
	if !g.HasEdge(left, right) {
		return nil, &EdgeNotFoundError{Left: left, Right: right}
	}
	properties := copyPropertyBag(g.edgeProperties[edgeKey(left, right)])
	weight, err := g.EdgeWeight(left, right)
	if err != nil {
		return nil, err
	}
	properties["weight"] = weight
	return properties, nil
}

func (g *Graph) SetEdgeProperty(left, right, propertyKey string, value PropertyValue) error {
	if !g.HasEdge(left, right) {
		return &EdgeNotFoundError{Left: left, Right: right}
	}
	if propertyKey == "weight" {
		weight, ok := numericValue(value)
		if !ok {
			return fmt.Errorf("edge property %q must be numeric", propertyKey)
		}
		g.setEdgeWeight(left, right, weight)
	}
	edgeID := edgeKey(left, right)
	if _, ok := g.edgeProperties[edgeID]; !ok {
		g.edgeProperties[edgeID] = make(PropertyBag)
	}
	g.edgeProperties[edgeID][propertyKey] = value
	return nil
}

func (g *Graph) RemoveEdgeProperty(left, right, key string) error {
	if !g.HasEdge(left, right) {
		return &EdgeNotFoundError{Left: left, Right: right}
	}
	if key == "weight" {
		g.setEdgeWeight(left, right, 1)
		edgeID := edgeKey(left, right)
		if _, ok := g.edgeProperties[edgeID]; !ok {
			g.edgeProperties[edgeID] = make(PropertyBag)
		}
		g.edgeProperties[edgeID]["weight"] = float64(1)
		return nil
	}
	delete(g.edgeProperties[edgeKey(left, right)], key)
	return nil
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

func edgeKey(left, right string) string {
	keyLeft, keyRight := canonicalEndpoints(left, right)
	return keyLeft + "\x00" + keyRight
}

func copyPropertyBag(properties PropertyBag) PropertyBag {
	result := make(PropertyBag, len(properties))
	for key, value := range properties {
		result[key] = value
	}
	return result
}

func (g *Graph) mergeNodeProperties(node string, properties ...PropertyBag) {
	for _, propertyBag := range properties {
		for key, value := range propertyBag {
			g.nodeProperties[node][key] = value
		}
	}
}

func (g *Graph) mergeEdgeProperties(left, right string, weight float64, properties ...PropertyBag) {
	edgeID := edgeKey(left, right)
	if _, ok := g.edgeProperties[edgeID]; !ok {
		g.edgeProperties[edgeID] = make(PropertyBag)
	}
	for _, propertyBag := range properties {
		for propertyKey, value := range propertyBag {
			g.edgeProperties[edgeID][propertyKey] = value
		}
	}
	g.edgeProperties[edgeID]["weight"] = weight
}

func (g *Graph) setEdgeWeight(left, right string, weight float64) {
	if weight == 0 {
		weight = 1
	}
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

func numericValue(value PropertyValue) (float64, bool) {
	switch typed := value.(type) {
	case int:
		return float64(typed), true
	case int8:
		return float64(typed), true
	case int16:
		return float64(typed), true
	case int32:
		return float64(typed), true
	case int64:
		return float64(typed), true
	case uint:
		return float64(typed), true
	case uint8:
		return float64(typed), true
	case uint16:
		return float64(typed), true
	case uint32:
		return float64(typed), true
	case uint64:
		return float64(typed), true
	case float32:
		return float64(typed), true
	case float64:
		return typed, true
	default:
		return 0, false
	}
}
