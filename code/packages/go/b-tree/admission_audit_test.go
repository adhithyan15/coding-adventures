package btree

import (
	"fmt"
	"math/rand"
	"reflect"
	"sort"
	"testing"
)

type auditReport struct {
	entries []KeyValue[int, string]
	keys    int
}

type auditBound struct {
	key   int
	valid bool
}

// auditPrivateTree checks the private node graph independently of IsValid.
// Bounds are inherited from every ancestor separator, not reconstructed only
// from the current node. The identity set rejects both cycles and shared
// children before recursion can revisit them.
func auditPrivateTree(tree *BTree[int, string]) (auditReport, error) {
	var report auditReport
	if tree == nil {
		return report, fmt.Errorf("nil tree")
	}
	if tree.root == nil {
		if tree.size != 0 {
			return report, fmt.Errorf("nil root with size %d", tree.size)
		}
		return report, nil
	}
	if tree.t < 2 || tree.less == nil {
		return report, fmt.Errorf("invalid degree/comparator")
	}
	seen := make(map[*bTreeNode[int, string]]struct{})
	leafDepth := -1
	if err := auditNode(tree, tree.root, true, 0, auditBound{}, auditBound{}, seen, &leafDepth, &report); err != nil {
		return auditReport{}, err
	}
	if report.keys != tree.size {
		return auditReport{}, fmt.Errorf("counted %d keys, size is %d", report.keys, tree.size)
	}
	for index := 1; index < len(report.entries); index++ {
		if !tree.less(report.entries[index-1].Key, report.entries[index].Key) {
			return auditReport{}, fmt.Errorf("global traversal order is not strict at %d", index)
		}
	}
	return report, nil
}

func auditNode(
	tree *BTree[int, string],
	node *bTreeNode[int, string],
	isRoot bool,
	depth int,
	lower, upper auditBound,
	seen map[*bTreeNode[int, string]]struct{},
	leafDepth *int,
	report *auditReport,
) error {
	if depth > 128 {
		return fmt.Errorf("tree exceeds audit depth bound")
	}
	if node == nil {
		return fmt.Errorf("nil child at depth %d", depth)
	}
	if _, duplicate := seen[node]; duplicate {
		return fmt.Errorf("repeated node identity at depth %d", depth)
	}
	seen[node] = struct{}{}
	n := len(node.keys)
	maxKeys := 2*uint(tree.t) - 1
	if n != len(node.values) {
		return fmt.Errorf("key/value width mismatch at depth %d: %d/%d", depth, n, len(node.values))
	}
	if isRoot {
		if n < 1 || uint(n) > maxKeys {
			return fmt.Errorf("root occupancy %d outside 1..%d", n, maxKeys)
		}
	} else if n < tree.t-1 || uint(n) > maxKeys {
		return fmt.Errorf("node occupancy %d outside %d..%d", n, tree.t-1, maxKeys)
	}
	for index, key := range node.keys {
		if index > 0 && !tree.less(node.keys[index-1], key) {
			return fmt.Errorf("node keys are not strictly ordered at depth %d", depth)
		}
		if lower.valid && !tree.less(lower.key, key) {
			return fmt.Errorf("key %d violates inherited lower bound %d at depth %d", key, lower.key, depth)
		}
		if upper.valid && !tree.less(key, upper.key) {
			return fmt.Errorf("key %d violates inherited upper bound %d at depth %d", key, upper.key, depth)
		}
	}
	if node.isLeaf {
		if len(node.children) != 0 {
			return fmt.Errorf("leaf has %d children", len(node.children))
		}
		if *leafDepth == -1 {
			*leafDepth = depth
		} else if *leafDepth != depth {
			return fmt.Errorf("leaf depth %d differs from %d", depth, *leafDepth)
		}
		for index, key := range node.keys {
			report.entries = append(report.entries, KeyValue[int, string]{Key: key, Value: node.values[index]})
		}
		report.keys += n
		return nil
	}
	if len(node.children) != n+1 {
		return fmt.Errorf("internal child count %d, want %d", len(node.children), n+1)
	}
	for index := 0; index <= n; index++ {
		childLower := lower
		childUpper := upper
		if index > 0 {
			childLower = auditBound{key: node.keys[index-1], valid: true}
		}
		if index < n {
			childUpper = auditBound{key: node.keys[index], valid: true}
		}
		if err := auditNode(tree, node.children[index], false, depth+1, childLower, childUpper, seen, leafDepth, report); err != nil {
			return err
		}
		if index < n {
			report.entries = append(report.entries, KeyValue[int, string]{Key: node.keys[index], Value: node.values[index]})
			report.keys++
		}
	}
	return nil
}

func auditExpected(tree *BTree[int, string], model map[int]string) []KeyValue[int, string] {
	keys := make([]int, 0, len(model))
	for key := range model {
		keys = append(keys, key)
	}
	sort.Slice(keys, func(i, j int) bool { return tree.less(keys[i], keys[j]) })
	result := make([]KeyValue[int, string], len(keys))
	for index, key := range keys {
		result[index] = KeyValue[int, string]{Key: key, Value: model[key]}
	}
	return result
}

func compareTreeWithMap(t *testing.T, tree *BTree[int, string], model map[int]string) {
	t.Helper()
	walked, err := auditPrivateTree(tree)
	if err != nil {
		t.Fatalf("independent structural audit: %v", err)
	}
	want := auditExpected(tree, model)
	if len(want) == 0 {
		want = nil
	}
	if walked.keys != len(model) || !reflect.DeepEqual(walked.entries, want) {
		t.Fatalf("private traversal=%v (%d keys), model=%v", walked.entries, walked.keys, want)
	}
	if tree.Len() != len(model) || !tree.IsValid() {
		t.Fatalf("public Len/IsValid = %d/%t; want %d/true", tree.Len(), tree.IsValid(), len(model))
	}
	if got := tree.Inorder(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Inorder=%v, want %v", got, want)
	}
	for key := -12; key <= 12; key++ {
		got, present := tree.Search(key)
		wantValue, expected := model[key]
		if present != expected || got != wantValue || tree.Contains(key) != expected {
			t.Fatalf("key %d Search/Contains=(%q,%t,%t), want (%q,%t,%t)", key, got, present, tree.Contains(key), wantValue, expected, expected)
		}
	}
	if len(want) == 0 {
		if _, err := tree.MinKey(); err == nil {
			t.Fatal("MinKey succeeded on empty model")
		}
		if _, err := tree.MaxKey(); err == nil {
			t.Fatal("MaxKey succeeded on empty model")
		}
	} else {
		if got, err := tree.MinKey(); err != nil || got != want[0].Key {
			t.Fatalf("MinKey=(%d,%v), want %d", got, err, want[0].Key)
		}
		if got, err := tree.MaxKey(); err != nil || got != want[len(want)-1].Key {
			t.Fatalf("MaxKey=(%d,%v), want %d", got, err, want[len(want)-1].Key)
		}
	}
	for _, low := range []int{-12, -4, 0, 4, 12} {
		for _, high := range []int{-12, -4, 0, 4, 12} {
			var expected []KeyValue[int, string]
			for _, pair := range want {
				if !tree.less(pair.Key, low) && !tree.less(high, pair.Key) {
					expected = append(expected, pair)
				}
			}
			if got := tree.RangeQuery(low, high); !reflect.DeepEqual(got, expected) {
				t.Fatalf("RangeQuery(%d,%d)=%v, want %v", low, high, got, expected)
			}
		}
	}
}

func compareTreeStructureWithMap(t *testing.T, tree *BTree[int, string], model map[int]string) {
	t.Helper()
	walked, err := auditPrivateTree(tree)
	if err != nil {
		t.Fatalf("independent structural audit: %v", err)
	}
	want := auditExpected(tree, model)
	if len(want) == 0 {
		want = nil
	}
	if walked.keys != len(model) || !reflect.DeepEqual(walked.entries, want) {
		t.Fatalf("private traversal=%v (%d keys), model=%v", walked.entries, walked.keys, want)
	}
	if tree.Len() != len(model) || !tree.IsValid() {
		t.Fatalf("public Len/IsValid = %d/%t; want %d/true", tree.Len(), tree.IsValid(), len(model))
	}
	if got := tree.Inorder(); !reflect.DeepEqual(got, want) {
		t.Fatalf("Inorder=%v, want %v", got, want)
	}
}

func TestIndependentAuditAcrossBoundedExhaustiveHistories(t *testing.T) {
	commands := []struct {
		insert bool
		key    int
		value  string
	}{
		{true, -1, "minus-first"}, {true, 0, "zero-first"}, {true, 1, "plus-first"},
		{false, -1, ""}, {false, 0, ""}, {false, 1, ""},
	}
	histories := 0
	transitions := 0
	for encoded := 0; encoded < 6*6*6*6; encoded++ {
		state := encoded
		tree := newIntTree(2)
		model := make(map[int]string)
		for step := 0; step < 4; step++ {
			command := commands[state%len(commands)]
			state /= len(commands)
			if command.insert {
				tree.Insert(command.key, command.value)
				model[command.key] = command.value
			} else {
				tree.Delete(command.key)
				delete(model, command.key)
			}
			compareTreeWithMap(t, tree, model)
			transitions++
		}
		histories++
	}
	if histories != 1296 || transitions != 5184 {
		t.Fatalf("bounded histories/transitions=%d/%d", histories, transitions)
	}
	t.Logf("audited %d degree-two histories and %d public transitions", histories, transitions)
}

func TestIndependentAuditSeededDegreesAndComparatorDirections(t *testing.T) {
	const seed = int64(0xa02c)
	transitions := 0
	for _, degree := range []int{2, 3, 5, 11} {
		for _, descending := range []bool{false, true} {
			less := func(a, b int) bool { return a < b }
			if descending {
				less = func(a, b int) bool { return a > b }
			}
			rng := rand.New(rand.NewSource(seed + int64(degree)*31))
			tree := New[int, string](degree, less)
			model := make(map[int]string)
			for step := 0; step < 2048; step++ {
				key := rng.Intn(25) - 12
				if rng.Intn(3) != 0 {
					value := fmt.Sprintf("d%d-x%d-step%d", degree, key, step)
					tree.Insert(key, value)
					model[key] = value
				} else {
					tree.Delete(key)
					delete(model, key)
				}
				compareTreeWithMap(t, tree, model)
				transitions++
			}
		}
	}
	if transitions != 16_384 {
		t.Fatalf("seeded transition count=%d, want 16384", transitions)
	}
	t.Logf("audited %d seeded transitions across degrees 2,3,5,11 and both comparator directions", transitions)
}

func TestIndependentAuditRootContractionAndReturnedEntryOwnership(t *testing.T) {
	tree := newIntTree(2)
	model := make(map[int]string)
	for key := 0; key < 96; key++ {
		value := fmt.Sprintf("value-%d", key)
		tree.Insert(key, value)
		model[key] = value
	}
	compareTreeWithMap(t, tree, model)
	previousHeight := tree.Height()
	contracted := false
	for key := 0; key < 96; key++ {
		tree.Delete(key)
		delete(model, key)
		compareTreeWithMap(t, tree, model)
		if tree.Height() < previousHeight {
			contracted = true
			previousHeight = tree.Height()
		}
	}
	if !contracted || tree.root != nil || tree.Height() != -1 {
		t.Fatalf("root failed to contract to empty: contracted=%t height=%d root=%p", contracted, tree.Height(), tree.root)
	}

	owned := newIntTree(2)
	owned.Insert(4, "four")
	entries := owned.Inorder()
	entries[0].Key = 99
	entries[0].Value = "changed returned copy"
	if got, ok := owned.Search(4); !ok || got != "four" {
		t.Fatalf("mutating returned entry changed stored value: %q/%t", got, ok)
	}
}

func TestIndependentAuditBorrowFromBothSiblingDirections(t *testing.T) {
	node := func(keys ...int) *bTreeNode[int, string] {
		values := make([]string, len(keys))
		for index, key := range keys {
			values[index] = fmt.Sprintf("v%d", key)
		}
		return &bTreeNode[int, string]{keys: append([]int(nil), keys...), values: values, isLeaf: true}
	}
	for _, test := range []struct {
		name     string
		parent   *bTreeNode[int, string]
		childIdx int
		want     []KeyValue[int, string]
	}{
		{
			name:     "borrow from left",
			parent:   &bTreeNode[int, string]{keys: []int{10}, values: []string{"v10"}, children: []*bTreeNode[int, string]{node(1, 5), node(12)}, isLeaf: false},
			childIdx: 1,
			want:     []KeyValue[int, string]{{1, "v1"}, {5, "v5"}, {10, "v10"}, {12, "v12"}},
		},
		{
			name:     "borrow from right",
			parent:   &bTreeNode[int, string]{keys: []int{10}, values: []string{"v10"}, children: []*bTreeNode[int, string]{node(8), node(12, 15)}, isLeaf: false},
			childIdx: 0,
			want:     []KeyValue[int, string]{{8, "v8"}, {10, "v10"}, {12, "v12"}, {15, "v15"}},
		},
	} {
		t.Run(test.name, func(t *testing.T) {
			tree := newIntTree(2)
			tree.root = test.parent
			tree.size = len(test.want)
			if _, err := auditPrivateTree(tree); err != nil {
				t.Fatalf("precondition tree invalid: %v", err)
			}
			tree.fill(tree.root, test.childIdx)
			report, err := auditPrivateTree(tree)
			if err != nil || !reflect.DeepEqual(report.entries, test.want) || !tree.IsValid() {
				t.Fatalf("post-borrow audit=%+v err=%v IsValid=%t", report, err, tree.IsValid())
			}
		})
	}
}

func TestIndependentAuditRejectsCorruptBoundsLengthsAndIdentity(t *testing.T) {
	t.Run("ancestor separator violation", func(t *testing.T) {
		tree := newIntTree(2)
		for _, key := range []int{1, 2, 3, 4} {
			tree.Insert(key, fmt.Sprintf("v%d", key))
		}
		if tree.root == nil || len(tree.root.keys) != 1 || len(tree.root.children[0].keys) != 1 {
			t.Fatalf("unexpected smallest witness shape: root=%+v", tree.root)
		}
		tree.root.children[0].keys[0] = 5
		if _, err := auditPrivateTree(tree); err == nil {
			t.Fatal("independent walker accepted key above inherited separator")
		}
	})

	t.Run("ancestor bound reaches nested leaf", func(t *testing.T) {
		tree := newIntTree(2)
		for key := 0; key < 1024; key++ {
			tree.Insert(key, fmt.Sprintf("v%d", key))
		}
		root := tree.root
		if root == nil || root.isLeaf || root.children[0].isLeaf {
			t.Fatalf("expected at least three levels, height=%d", tree.Height())
		}
		leftSubtree := root.children[0]
		leaf := leftSubtree
		for !leaf.isLeaf {
			leaf = leaf.children[len(leaf.children)-1]
		}
		if len(leaf.keys) == 0 {
			t.Fatal("expected populated rightmost leaf under root's left subtree")
		}
		leaf.keys[len(leaf.keys)-1] = root.keys[0]
		if _, err := auditPrivateTree(tree); err == nil {
			t.Fatal("walker failed to propagate the root upper bound to a grandchild")
		}
	})

	t.Run("key/value lengths", func(t *testing.T) {
		tree := newIntTree(2)
		tree.Insert(7, "seven")
		tree.root.values = nil
		if _, err := auditPrivateTree(tree); err == nil {
			t.Fatal("walker accepted mismatched key/value lengths")
		}
	})

	t.Run("shared child identity", func(t *testing.T) {
		tree := newIntTree(2)
		for _, key := range []int{1, 2, 3, 4} {
			tree.Insert(key, fmt.Sprintf("v%d", key))
		}
		tree.root.children[1] = tree.root.children[0]
		if _, err := auditPrivateTree(tree); err == nil {
			t.Fatal("walker accepted a shared child pointer")
		}
	})

	t.Run("cycle identity", func(t *testing.T) {
		tree := newIntTree(2)
		for _, key := range []int{1, 2, 3, 4} {
			tree.Insert(key, fmt.Sprintf("v%d", key))
		}
		tree.root.children[0] = tree.root
		// Do not call public IsValid: this deliberate cycle would recurse forever
		// in its unguarded recursive checker. The independent walker must stop.
		if _, err := auditPrivateTree(tree); err == nil {
			t.Fatal("walker accepted a cyclic child pointer")
		}
	})
}

func TestIsValidRejectsMalformedPrivateStructures(t *testing.T) {
	leaf := func(keys ...int) *bTreeNode[int, string] {
		values := make([]string, len(keys))
		for i, key := range keys {
			values[i] = fmt.Sprintf("v%d", key)
		}
		return &bTreeNode[int, string]{keys: append([]int(nil), keys...), values: values, isLeaf: true}
	}
	valid := func() *BTree[int, string] {
		tree := newIntTree(2)
		for _, key := range []int{1, 2, 3, 4} {
			tree.Insert(key, fmt.Sprintf("v%d", key))
		}
		return tree
	}

	tests := []struct {
		name    string
		corrupt func(*BTree[int, string])
	}{
		{
			name: "ancestor separator bound",
			corrupt: func(tree *BTree[int, string]) {
				tree.root.children[0].keys[0] = tree.root.keys[0] + 1
			},
		},
		{
			name: "mismatched key and value lengths",
			corrupt: func(tree *BTree[int, string]) {
				tree.root.values = nil
			},
		},
		{
			name: "nil child",
			corrupt: func(tree *BTree[int, string]) {
				tree.root.children[0] = nil
			},
		},
		{
			name: "shared child",
			corrupt: func(tree *BTree[int, string]) {
				tree.root.children[1] = tree.root.children[0]
			},
		},
		{
			name: "cycle",
			corrupt: func(tree *BTree[int, string]) {
				tree.root.children[0] = tree.root
			},
		},
		{
			name: "leaf with child",
			corrupt: func(tree *BTree[int, string]) {
				tree.root = leaf(7)
				tree.root.children = []*bTreeNode[int, string]{leaf(8)}
				tree.size = 1
			},
		},
		{
			name: "different leaf depths",
			corrupt: func(tree *BTree[int, string]) {
				tree.root = &bTreeNode[int, string]{
					keys: []int{10}, values: []string{"v10"}, isLeaf: false,
					children: []*bTreeNode[int, string]{
						{keys: []int{5}, values: []string{"v5"}, isLeaf: false, children: []*bTreeNode[int, string]{leaf(2), leaf(7)}},
						leaf(12),
					},
				}
				tree.size = 5
			},
		},
		{
			name: "empty occupied root",
			corrupt: func(tree *BTree[int, string]) {
				tree.root = leaf()
				tree.size = 0
			},
		},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			tree := valid()
			test.corrupt(tree)
			if _, err := auditPrivateTree(tree); err == nil {
				t.Fatal("independent audit accepted malformed tree")
			}
			if tree.IsValid() {
				t.Fatal("IsValid accepted malformed private structure")
			}
		})
	}
}

func TestIsValidSupportsNonComparableKeys(t *testing.T) {
	less := func(a, b []int) bool {
		limit := len(a)
		if len(b) < limit {
			limit = len(b)
		}
		for i := 0; i < limit; i++ {
			if a[i] != b[i] {
				return a[i] < b[i]
			}
		}
		return len(a) < len(b)
	}
	tree := New[[]int, string](2, less)
	for i, key := range [][]int{{2}, {1}, {4}, {3}, {5}, {0}} {
		tree.Insert(key, fmt.Sprintf("value-%d", i))
	}
	if !tree.IsValid() {
		t.Fatal("IsValid rejected a valid tree with non-comparable slice keys")
	}
}

func TestIsValidHandlesLargeMinimumDegreesWithoutOverflow(t *testing.T) {
	maxInt := int(^uint(0) >> 1)
	for _, degree := range []int{2, maxInt/2 + 1, maxInt/2 + 2, maxInt} {
		tree := newIntTree(degree)
		tree.Insert(7, "seven")
		if !tree.IsValid() {
			t.Errorf("IsValid rejected a one-entry tree with degree %d", degree)
		}
		if _, err := auditPrivateTree(tree); err != nil {
			t.Errorf("independent walker rejected a one-entry tree with degree %d: %v", degree, err)
		}
	}
}

func FuzzIsValidAgainstIndependentAudit(f *testing.F) {
	f.Add([]byte{0, 4, 8, 12, 1, 5, 9, 13, 2, 6, 10, 14})
	f.Add([]byte{1, 5, 9, 13, 17, 21, 25, 29})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 64 {
			input = input[:64]
		}
		tree := newIntTree(2)
		model := make(map[int]string)
		for step, command := range input {
			key := int(command>>2)%31 - 15
			if command%3 != 0 {
				value := fmt.Sprintf("fuzz-%d-%d", key, step)
				tree.Insert(key, value)
				model[key] = value
			} else {
				tree.Delete(key)
				delete(model, key)
			}
			compareTreeStructureWithMap(t, tree, model)
		}
		compareTreeWithMap(t, tree, model)
	})
}
