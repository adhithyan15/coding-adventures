package binarytree

import (
	"fmt"
	"strings"
)

type Node[T comparable] struct {
	Value T
	Left  *Node[T]
	Right *Node[T]
}

func NewNode[T comparable](value T) *Node[T] {
	return &Node[T]{Value: value}
}

type BinaryTree[T comparable] struct {
	root *Node[T]
}

func New[T comparable]() BinaryTree[T] {
	return BinaryTree[T]{}
}

func WithRoot[T comparable](root *Node[T]) BinaryTree[T] {
	return BinaryTree[T]{root: root}
}

func FromLevelOrder[T comparable](values []*T) BinaryTree[T] {
	return BinaryTree[T]{root: buildFromLevelOrder(values, 0)}
}

func Ptr[T comparable](value T) *T {
	return &value
}

func (tree BinaryTree[T]) Root() *Node[T] {
	return tree.root
}

func (tree BinaryTree[T]) Find(value T) *Node[T] {
	return Find(tree.root, value)
}

func (tree BinaryTree[T]) LeftChild(value T) *Node[T] {
	node := tree.Find(value)
	if node == nil {
		return nil
	}
	return node.Left
}

func (tree BinaryTree[T]) RightChild(value T) *Node[T] {
	node := tree.Find(value)
	if node == nil {
		return nil
	}
	return node.Right
}

func (tree BinaryTree[T]) IsFull() bool {
	return IsFull(tree.root)
}

func (tree BinaryTree[T]) IsComplete() bool {
	return IsComplete(tree.root)
}

func (tree BinaryTree[T]) IsPerfect() bool {
	return IsPerfect(tree.root)
}

func (tree BinaryTree[T]) Height() int {
	return Height(tree.root)
}

func (tree BinaryTree[T]) Size() int {
	return Size(tree.root)
}

func (tree BinaryTree[T]) Inorder() []T {
	out := []T{}
	inorder(tree.root, &out)
	return out
}

func (tree BinaryTree[T]) Preorder() []T {
	out := []T{}
	preorder(tree.root, &out)
	return out
}

func (tree BinaryTree[T]) Postorder() []T {
	out := []T{}
	postorder(tree.root, &out)
	return out
}

func (tree BinaryTree[T]) LevelOrder() []T {
	if tree.root == nil {
		return []T{}
	}

	out := []T{}
	queue := []*Node[T]{tree.root}
	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		out = append(out, node.Value)
		if node.Left != nil {
			queue = append(queue, node.Left)
		}
		if node.Right != nil {
			queue = append(queue, node.Right)
		}
	}
	return out
}

func (tree BinaryTree[T]) ToArray() []*T {
	treeHeight := tree.Height()
	if treeHeight < 0 {
		return []*T{}
	}

	out := make([]*T, (1<<(treeHeight+1))-1)
	fillArray(tree.root, 0, out)
	return out
}

func (tree BinaryTree[T]) ToASCII() string {
	if tree.root == nil {
		return ""
	}

	lines := []string{}
	renderASCII(tree.root, "", true, &lines)
	return strings.Join(lines, "\n")
}

func (tree BinaryTree[T]) String() string {
	if tree.root == nil {
		return "BinaryTree(root=<nil>, size=0)"
	}
	return fmt.Sprintf("BinaryTree(root=%v, size=%d)", tree.root.Value, tree.Size())
}

func Find[T comparable](root *Node[T], value T) *Node[T] {
	if root == nil {
		return nil
	}
	if root.Value == value {
		return root
	}
	if found := Find(root.Left, value); found != nil {
		return found
	}
	return Find(root.Right, value)
}

func IsFull[T comparable](root *Node[T]) bool {
	if root == nil {
		return true
	}
	if root.Left == nil && root.Right == nil {
		return true
	}
	if root.Left == nil || root.Right == nil {
		return false
	}
	return IsFull(root.Left) && IsFull(root.Right)
}

func IsComplete[T comparable](root *Node[T]) bool {
	queue := []*Node[T]{root}
	seenNil := false

	for len(queue) > 0 {
		node := queue[0]
		queue = queue[1:]
		if node == nil {
			seenNil = true
			continue
		}
		if seenNil {
			return false
		}
		queue = append(queue, node.Left, node.Right)
	}

	return true
}

func IsPerfect[T comparable](root *Node[T]) bool {
	treeHeight := Height(root)
	if treeHeight < 0 {
		return Size(root) == 0
	}
	return Size(root) == (1<<(treeHeight+1))-1
}

func Height[T comparable](root *Node[T]) int {
	if root == nil {
		return -1
	}
	left := Height(root.Left)
	right := Height(root.Right)
	if left > right {
		return 1 + left
	}
	return 1 + right
}

func Size[T comparable](root *Node[T]) int {
	if root == nil {
		return 0
	}
	return 1 + Size(root.Left) + Size(root.Right)
}

func buildFromLevelOrder[T comparable](values []*T, index int) *Node[T] {
	if index >= len(values) || values[index] == nil {
		return nil
	}
	return &Node[T]{
		Value: *values[index],
		Left:  buildFromLevelOrder(values, 2*index+1),
		Right: buildFromLevelOrder(values, 2*index+2),
	}
}

func inorder[T comparable](root *Node[T], out *[]T) {
	if root == nil {
		return
	}
	inorder(root.Left, out)
	*out = append(*out, root.Value)
	inorder(root.Right, out)
}

func preorder[T comparable](root *Node[T], out *[]T) {
	if root == nil {
		return
	}
	*out = append(*out, root.Value)
	preorder(root.Left, out)
	preorder(root.Right, out)
}

func postorder[T comparable](root *Node[T], out *[]T) {
	if root == nil {
		return
	}
	postorder(root.Left, out)
	postorder(root.Right, out)
	*out = append(*out, root.Value)
}

func fillArray[T comparable](root *Node[T], index int, out []*T) {
	if root == nil || index >= len(out) {
		return
	}
	out[index] = Ptr(root.Value)
	fillArray(root.Left, 2*index+1, out)
	fillArray(root.Right, 2*index+2, out)
}

func renderASCII[T comparable](node *Node[T], prefix string, isTail bool, lines *[]string) {
	connector := "`-- "
	if !isTail {
		connector = "|-- "
	}
	*lines = append(*lines, fmt.Sprintf("%s%s%v", prefix, connector, node.Value))

	children := []*Node[T]{}
	if node.Left != nil {
		children = append(children, node.Left)
	}
	if node.Right != nil {
		children = append(children, node.Right)
	}

	nextPrefix := prefix + "    "
	if !isTail {
		nextPrefix = prefix + "|   "
	}
	for index, child := range children {
		renderASCII(child, nextPrefix, index+1 == len(children), lines)
	}
}
