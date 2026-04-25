package avltree

type Ordered interface {
	~int | ~int8 | ~int16 | ~int32 | ~int64 |
		~uint | ~uint8 | ~uint16 | ~uint32 | ~uint64 | ~uintptr |
		~float32 | ~float64 | ~string
}

type Node[T Ordered] struct {
	Value  T
	Left   *Node[T]
	Right  *Node[T]
	Height int
	size   int
}

func NewNode[T Ordered](value T) *Node[T] {
	return &Node[T]{Value: value, Height: 0, size: 1}
}

func (node *Node[T]) Size() int {
	if node == nil {
		return 0
	}
	return node.size
}

type AVLTree[T Ordered] struct {
	root *Node[T]
}

func Empty[T Ordered]() AVLTree[T] {
	return AVLTree[T]{}
}

func FromValues[T Ordered](values []T) AVLTree[T] {
	tree := Empty[T]()
	for _, value := range values {
		tree = tree.Insert(value)
	}
	return tree
}

func (tree AVLTree[T]) Root() *Node[T] {
	return tree.root
}

func (tree AVLTree[T]) Insert(value T) AVLTree[T] {
	return AVLTree[T]{root: insert(tree.root, value)}
}

func (tree AVLTree[T]) Delete(value T) AVLTree[T] {
	return AVLTree[T]{root: deleteNode(tree.root, value)}
}

func (tree AVLTree[T]) Search(value T) *Node[T] {
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

func (tree AVLTree[T]) Contains(value T) bool {
	return tree.Search(value) != nil
}

func (tree AVLTree[T]) MinValue() (T, bool) {
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

func (tree AVLTree[T]) MaxValue() (T, bool) {
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

func (tree AVLTree[T]) Predecessor(value T) (T, bool) {
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

func (tree AVLTree[T]) Successor(value T) (T, bool) {
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

func (tree AVLTree[T]) KthSmallest(k int) (T, bool) {
	return kth(tree.root, k)
}

func (tree AVLTree[T]) Rank(value T) int {
	return rank(tree.root, value)
}

func (tree AVLTree[T]) ToSortedArray() []T {
	out := []T{}
	inorder(tree.root, &out)
	return out
}

func (tree AVLTree[T]) IsValidBST() bool {
	return validateBST(tree.root, nil, nil)
}

func (tree AVLTree[T]) IsValidAVL() bool {
	_, _, ok := validateAVL(tree.root, nil, nil)
	return ok
}

func (tree AVLTree[T]) BalanceFactor(node *Node[T]) int {
	if node == nil {
		return 0
	}
	return height(node.Left) - height(node.Right)
}

func (tree AVLTree[T]) Height() int {
	return height(tree.root)
}

func (tree AVLTree[T]) Size() int {
	return size(tree.root)
}

func insert[T Ordered](root *Node[T], value T) *Node[T] {
	if root == nil {
		return NewNode(value)
	}
	if value < root.Value {
		return rebalance(newNode(root.Value, insert(root.Left, value), root.Right))
	}
	if value > root.Value {
		return rebalance(newNode(root.Value, root.Left, insert(root.Right, value)))
	}
	return root
}

func deleteNode[T Ordered](root *Node[T], value T) *Node[T] {
	if root == nil {
		return nil
	}
	if value < root.Value {
		return rebalance(newNode(root.Value, deleteNode(root.Left, value), root.Right))
	}
	if value > root.Value {
		return rebalance(newNode(root.Value, root.Left, deleteNode(root.Right, value)))
	}
	if root.Left == nil {
		return root.Right
	}
	if root.Right == nil {
		return root.Left
	}
	newRight, successor := extractMin(root.Right)
	return rebalance(newNode(successor, root.Left, newRight))
}

func extractMin[T Ordered](root *Node[T]) (*Node[T], T) {
	if root.Left == nil {
		return root.Right, root.Value
	}
	newLeft, minimum := extractMin(root.Left)
	return rebalance(newNode(root.Value, newLeft, root.Right)), minimum
}

func rebalance[T Ordered](root *Node[T]) *Node[T] {
	bf := balanceFactor(root)
	if bf > 1 {
		left := root.Left
		if left != nil && balanceFactor(left) < 0 {
			left = rotateLeft(left)
		}
		return rotateRight(newNode(root.Value, left, root.Right))
	}
	if bf < -1 {
		right := root.Right
		if right != nil && balanceFactor(right) > 0 {
			right = rotateRight(right)
		}
		return rotateLeft(newNode(root.Value, root.Left, right))
	}
	return root
}

func rotateLeft[T Ordered](root *Node[T]) *Node[T] {
	if root.Right == nil {
		return root
	}
	newLeft := newNode(root.Value, root.Left, root.Right.Left)
	return newNode(root.Right.Value, newLeft, root.Right.Right)
}

func rotateRight[T Ordered](root *Node[T]) *Node[T] {
	if root.Left == nil {
		return root
	}
	newRight := newNode(root.Value, root.Left.Right, root.Right)
	return newNode(root.Left.Value, root.Left.Left, newRight)
}

func balanceFactor[T Ordered](root *Node[T]) int {
	return height(root.Left) - height(root.Right)
}

func kth[T Ordered](root *Node[T], k int) (T, bool) {
	if root == nil || k <= 0 {
		var zero T
		return zero, false
	}
	leftSize := size(root.Left)
	if k == leftSize+1 {
		return root.Value, true
	}
	if k <= leftSize {
		return kth(root.Left, k)
	}
	return kth(root.Right, k-leftSize-1)
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

func validateBST[T Ordered](root *Node[T], min *T, max *T) bool {
	if root == nil {
		return true
	}
	if min != nil && root.Value <= *min {
		return false
	}
	if max != nil && root.Value >= *max {
		return false
	}
	return validateBST(root.Left, min, &root.Value) && validateBST(root.Right, &root.Value, max)
}

func validateAVL[T Ordered](root *Node[T], min *T, max *T) (int, int, bool) {
	if root == nil {
		return -1, 0, true
	}
	if min != nil && root.Value <= *min {
		return 0, 0, false
	}
	if max != nil && root.Value >= *max {
		return 0, 0, false
	}
	leftHeight, leftSize, ok := validateAVL(root.Left, min, &root.Value)
	if !ok {
		return 0, 0, false
	}
	rightHeight, rightSize, ok := validateAVL(root.Right, &root.Value, max)
	if !ok {
		return 0, 0, false
	}
	nodeHeight := 1 + maxInt(leftHeight, rightHeight)
	nodeSize := 1 + leftSize + rightSize
	if root.Height != nodeHeight || root.size != nodeSize || absInt(leftHeight-rightHeight) > 1 {
		return 0, 0, false
	}
	return nodeHeight, nodeSize, true
}

func height[T Ordered](root *Node[T]) int {
	if root == nil {
		return -1
	}
	return root.Height
}

func size[T Ordered](root *Node[T]) int {
	if root == nil {
		return 0
	}
	return root.size
}

func newNode[T Ordered](value T, left *Node[T], right *Node[T]) *Node[T] {
	return &Node[T]{
		Value:  value,
		Left:   left,
		Right:  right,
		Height: 1 + maxInt(height(left), height(right)),
		size:   1 + size(left) + size(right),
	}
}

func maxInt(left int, right int) int {
	if left > right {
		return left
	}
	return right
}

func absInt(value int) int {
	if value < 0 {
		return -value
	}
	return value
}
