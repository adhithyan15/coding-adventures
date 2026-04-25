package binarysearchtree

import "fmt"

type Ordered interface {
	~int | ~int8 | ~int16 | ~int32 | ~int64 |
		~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64 | ~uintptr |
		~float32 | ~float64 | ~string
}

type Node[T Ordered] struct {
	Value T
	Left  *Node[T]
	Right *Node[T]
	size  int
}

func NewNode[T Ordered](value T) *Node[T] {
	return &Node[T]{Value: value, size: 1}
}

func (node *Node[T]) Size() int {
	if node == nil {
		return 0
	}
	return node.size
}

type BST[T Ordered] struct {
	root *Node[T]
}

func Empty[T Ordered]() BST[T] {
	return BST[T]{}
}

func FromSortedArray[T Ordered](values []T) BST[T] {
	return BST[T]{root: buildBalanced(values)}
}

func (tree BST[T]) Root() *Node[T] {
	return tree.root
}

func (tree BST[T]) Insert(value T) BST[T] {
	return BST[T]{root: insert(tree.root, value)}
}

func (tree BST[T]) Delete(value T) BST[T] {
	return BST[T]{root: deleteNode(tree.root, value)}
}

func (tree BST[T]) Search(value T) *Node[T] {
	current := tree.root
	for current != nil {
		switch {
		case value < current.Value:
			current = current.Left
		case value > current.Value:
			current = current.Right
		default:
			return current
		}
	}
	return nil
}

func (tree BST[T]) Contains(value T) bool {
	return tree.Search(value) != nil
}

func (tree BST[T]) MinValue() (T, bool) {
	current := tree.root
	for current != nil && current.Left != nil {
		current = current.Left
	}
	if current == nil {
		var zero T
		return zero, false
	}
	return current.Value, true
}

func (tree BST[T]) MaxValue() (T, bool) {
	current := tree.root
	for current != nil && current.Right != nil {
		current = current.Right
	}
	if current == nil {
		var zero T
		return zero, false
	}
	return current.Value, true
}

func (tree BST[T]) Predecessor(value T) (T, bool) {
	current := tree.root
	var best T
	found := false
	for current != nil {
		if value <= current.Value {
			current = current.Left
		} else {
			best = current.Value
			found = true
			current = current.Right
		}
	}
	return best, found
}

func (tree BST[T]) Successor(value T) (T, bool) {
	current := tree.root
	var best T
	found := false
	for current != nil {
		if value >= current.Value {
			current = current.Right
		} else {
			best = current.Value
			found = true
			current = current.Left
		}
	}
	return best, found
}

func (tree BST[T]) KthSmallest(k int) (T, bool) {
	return kthSmallest(tree.root, k)
}

func (tree BST[T]) Rank(value T) int {
	return rank(tree.root, value)
}

func (tree BST[T]) ToSortedArray() []T {
	out := []T{}
	inorder(tree.root, &out)
	return out
}

func (tree BST[T]) IsValid() bool {
	_, _, ok := validate(tree.root, nil, nil)
	return ok
}

func (tree BST[T]) Height() int {
	return height(tree.root)
}

func (tree BST[T]) Size() int {
	return size(tree.root)
}

func (tree BST[T]) String() string {
	if tree.root == nil {
		return "BinarySearchTree(root=<nil>, size=0)"
	}
	return fmt.Sprintf("BinarySearchTree(root=%v, size=%d)", tree.root.Value, tree.Size())
}

func insert[T Ordered](root *Node[T], value T) *Node[T] {
	if root == nil {
		return NewNode(value)
	}
	if value < root.Value {
		return withChildren(root, insert(root.Left, value), root.Right)
	}
	if value > root.Value {
		return withChildren(root, root.Left, insert(root.Right, value))
	}
	return root
}

func deleteNode[T Ordered](root *Node[T], value T) *Node[T] {
	if root == nil {
		return nil
	}
	if value < root.Value {
		return withChildren(root, deleteNode(root.Left, value), root.Right)
	}
	if value > root.Value {
		return withChildren(root, root.Left, deleteNode(root.Right, value))
	}
	if root.Left == nil {
		return root.Right
	}
	if root.Right == nil {
		return root.Left
	}
	newRight, successor := extractMin(root.Right)
	return newNode(successor, root.Left, newRight)
}

func extractMin[T Ordered](root *Node[T]) (*Node[T], T) {
	if root.Left == nil {
		return root.Right, root.Value
	}
	newLeft, minimum := extractMin(root.Left)
	return withChildren(root, newLeft, root.Right), minimum
}

func kthSmallest[T Ordered](root *Node[T], k int) (T, bool) {
	if root == nil || k <= 0 {
		var zero T
		return zero, false
	}
	leftSize := size(root.Left)
	if k == leftSize+1 {
		return root.Value, true
	}
	if k <= leftSize {
		return kthSmallest(root.Left, k)
	}
	return kthSmallest(root.Right, k-leftSize-1)
}

func rank[T Ordered](root *Node[T], value T) int {
	if root == nil {
		return 0
	}
	if value < root.Value {
		return rank(root.Left, value)
	}
	if value > root.Value {
		return size(root.Left) + 1 + rank(root.Right, value)
	}
	return size(root.Left)
}

func inorder[T Ordered](root *Node[T], out *[]T) {
	if root == nil {
		return
	}
	inorder(root.Left, out)
	*out = append(*out, root.Value)
	inorder(root.Right, out)
}

func validate[T Ordered](root *Node[T], min *T, max *T) (int, int, bool) {
	if root == nil {
		return -1, 0, true
	}
	if min != nil && root.Value <= *min {
		return 0, 0, false
	}
	if max != nil && root.Value >= *max {
		return 0, 0, false
	}
	leftHeight, leftSize, ok := validate(root.Left, min, &root.Value)
	if !ok {
		return 0, 0, false
	}
	rightHeight, rightSize, ok := validate(root.Right, &root.Value, max)
	if !ok {
		return 0, 0, false
	}
	nodeHeight := 1 + maxInt(leftHeight, rightHeight)
	nodeSize := 1 + leftSize + rightSize
	if root.size != nodeSize {
		return 0, 0, false
	}
	return nodeHeight, nodeSize, true
}

func height[T Ordered](root *Node[T]) int {
	if root == nil {
		return -1
	}
	return 1 + maxInt(height(root.Left), height(root.Right))
}

func size[T Ordered](root *Node[T]) int {
	if root == nil {
		return 0
	}
	return root.size
}

func buildBalanced[T Ordered](values []T) *Node[T] {
	if len(values) == 0 {
		return nil
	}
	mid := len(values) / 2
	return newNode(values[mid], buildBalanced(values[:mid]), buildBalanced(values[mid+1:]))
}

func withChildren[T Ordered](root *Node[T], left *Node[T], right *Node[T]) *Node[T] {
	return newNode(root.Value, left, right)
}

func newNode[T Ordered](value T, left *Node[T], right *Node[T]) *Node[T] {
	return &Node[T]{
		Value: value,
		Left:  left,
		Right: right,
		size:  1 + size(left) + size(right),
	}
}

func maxInt(left int, right int) int {
	if left > right {
		return left
	}
	return right
}
