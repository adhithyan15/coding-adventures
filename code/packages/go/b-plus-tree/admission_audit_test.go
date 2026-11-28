package bplustree

import (
	"fmt"
	"math/rand"
	"reflect"
	"sort"
	"testing"
)

type bplusAuditReport struct {
	entries []KeyValue[int, string]
	leaves  []*bplusLeaf[int, string]
	count   int
}

type bplusAuditBound struct {
	key   int
	valid bool
}

type bplusMalformedSliceNode []int

func (bplusMalformedSliceNode) isLeaf() bool { return true }

// auditBPlusTree walks the private tree independently of IsValid. Separator
// ranges use upper-bound routing: lower inclusive, upper exclusive.
func auditBPlusTree(tree *BPlusTree[int, string]) (bplusAuditReport, error) {
	var report bplusAuditReport
	if tree == nil {
		return report, fmt.Errorf("nil tree")
	}
	if tree.root == nil {
		if tree.size != 0 || tree.firstLeaf != nil {
			return report, fmt.Errorf("empty root has size or first leaf")
		}
		return report, nil
	}
	if tree.t < 2 || tree.less == nil {
		return report, fmt.Errorf("invalid degree or comparator")
	}
	seen := make(map[bplusNode[int, string]]struct{})
	leafDepth := -1
	if err := auditBPlusNode(tree, tree.root, true, 0, bplusAuditBound{}, bplusAuditBound{}, seen, &leafDepth, &report); err != nil {
		return bplusAuditReport{}, err
	}
	if report.count != tree.size {
		return bplusAuditReport{}, fmt.Errorf("counted %d leaf records, size is %d", report.count, tree.size)
	}
	if len(report.leaves) == 0 || tree.firstLeaf != report.leaves[0] {
		return bplusAuditReport{}, fmt.Errorf("firstLeaf does not name the leftmost reachable leaf")
	}
	leaf := tree.firstLeaf
	for index, want := range report.leaves {
		if leaf == nil || leaf != want {
			return bplusAuditReport{}, fmt.Errorf("leaf chain differs from tree traversal at index %d", index)
		}
		leaf = leaf.next
	}
	if leaf != nil {
		return bplusAuditReport{}, fmt.Errorf("leaf chain has a foreign or cyclic suffix")
	}
	for index := 1; index < len(report.entries); index++ {
		if !tree.less(report.entries[index-1].Key, report.entries[index].Key) {
			return bplusAuditReport{}, fmt.Errorf("global record order is not strict at index %d", index)
		}
	}
	return report, nil
}

func auditBPlusNode(
	tree *BPlusTree[int, string],
	node bplusNode[int, string],
	isRoot bool,
	depth int,
	lower, upper bplusAuditBound,
	seen map[bplusNode[int, string]]struct{},
	leafDepth *int,
	report *bplusAuditReport,
) error {
	if node == nil {
		return fmt.Errorf("nil child at depth %d", depth)
	}
	switch current := node.(type) {
	case *bplusLeaf[int, string]:
		if current == nil {
			return fmt.Errorf("typed nil leaf at depth %d", depth)
		}
	case *bplusInternal[int, string]:
		if current == nil {
			return fmt.Errorf("typed nil internal node at depth %d", depth)
		}
	default:
		return fmt.Errorf("unknown node implementation %T", node)
	}
	if _, duplicate := seen[node]; duplicate {
		return fmt.Errorf("repeated node identity at depth %d", depth)
	}
	seen[node] = struct{}{}

	maxKeys := 2*uint(tree.t) - 1
	switch current := node.(type) {
	case *bplusLeaf[int, string]:
		n := len(current.keys)
		if n != len(current.values) {
			return fmt.Errorf("leaf key/value mismatch at depth %d", depth)
		}
		if isRoot {
			if n < 1 || uint(n) > maxKeys {
				return fmt.Errorf("root leaf occupancy %d outside 1..%d", n, maxKeys)
			}
		} else if n < tree.t-1 || uint(n) > maxKeys {
			return fmt.Errorf("leaf occupancy %d outside %d..%d", n, tree.t-1, maxKeys)
		}
		for index, key := range current.keys {
			if index > 0 && !tree.less(current.keys[index-1], key) {
				return fmt.Errorf("leaf keys are not strictly ordered")
			}
			if lower.valid && tree.less(key, lower.key) {
				return fmt.Errorf("leaf key %d below inclusive lower bound %d", key, lower.key)
			}
			if upper.valid && !tree.less(key, upper.key) {
				return fmt.Errorf("leaf key %d not below exclusive upper bound %d", key, upper.key)
			}
		}
		if *leafDepth < 0 {
			*leafDepth = depth
		} else if *leafDepth != depth {
			return fmt.Errorf("leaf depth %d differs from %d", depth, *leafDepth)
		}
		report.leaves = append(report.leaves, current)
		for index, key := range current.keys {
			report.entries = append(report.entries, KeyValue[int, string]{Key: key, Value: current.values[index]})
		}
		report.count += n
		return nil
	case *bplusInternal[int, string]:
		n := len(current.keys)
		if isRoot {
			if n < 1 || uint(n) > maxKeys {
				return fmt.Errorf("root internal occupancy %d outside 1..%d", n, maxKeys)
			}
		} else if n < tree.t-1 || uint(n) > maxKeys {
			return fmt.Errorf("internal occupancy %d outside %d..%d", n, tree.t-1, maxKeys)
		}
		for index, key := range current.keys {
			if index > 0 && !tree.less(current.keys[index-1], key) {
				return fmt.Errorf("internal separators are not strictly ordered")
			}
			if lower.valid && tree.less(key, lower.key) {
				return fmt.Errorf("separator %d below inclusive lower bound %d", key, lower.key)
			}
			if upper.valid && !tree.less(key, upper.key) {
				return fmt.Errorf("separator %d not below exclusive upper bound %d", key, upper.key)
			}
		}
		if len(current.children) != n+1 {
			return fmt.Errorf("internal child count %d, want %d", len(current.children), n+1)
		}
		for index, child := range current.children {
			childLower, childUpper := lower, upper
			if index > 0 {
				childLower = bplusAuditBound{key: current.keys[index-1], valid: true}
			}
			if index < n {
				childUpper = bplusAuditBound{key: current.keys[index], valid: true}
			}
			if err := auditBPlusNode(tree, child, false, depth+1, childLower, childUpper, seen, leafDepth, report); err != nil {
				return err
			}
		}
		return nil
	default:
		return fmt.Errorf("unknown node implementation %T", node)
	}
}

func bplusExpected(tree *BPlusTree[int, string], model map[int]string) []KeyValue[int, string] {
	keys := make([]int, 0, len(model))
	for key := range model {
		keys = append(keys, key)
	}
	sort.Slice(keys, func(i, j int) bool { return tree.less(keys[i], keys[j]) })
	if len(keys) == 0 {
		return nil
	}
	result := make([]KeyValue[int, string], len(keys))
	for index, key := range keys {
		result[index] = KeyValue[int, string]{Key: key, Value: model[key]}
	}
	return result
}

func compareBPlusWithMap(t *testing.T, tree *BPlusTree[int, string], model map[int]string) {
	t.Helper()
	report, err := auditBPlusTree(tree)
	if err != nil {
		t.Fatalf("independent structural audit: %v", err)
	}
	want := bplusExpected(tree, model)
	if report.count != len(model) || !reflect.DeepEqual(report.entries, want) {
		t.Fatalf("walker records=%v (%d), model=%v", report.entries, report.count, want)
	}
	if !tree.IsValid() || tree.Len() != len(model) {
		t.Fatalf("public IsValid/Len=%t/%d, model count=%d", tree.IsValid(), tree.Len(), len(model))
	}
	if got := tree.FullScan(); !reflect.DeepEqual(got, want) {
		t.Fatalf("FullScan=%v, want %v", got, want)
	}
	for key := -16; key <= 16; key++ {
		got, found := tree.Search(key)
		value, present := model[key]
		if found != present || got != value || tree.Contains(key) != present {
			t.Fatalf("key %d Search/Contains=(%q,%t,%t), want (%q,%t,%t)", key, got, found, tree.Contains(key), value, present, present)
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
	for _, low := range []int{-16, -8, 0, 8, 16} {
		for _, high := range []int{-16, -8, 0, 8, 16} {
			var rangeWant []KeyValue[int, string]
			for _, pair := range want {
				if !tree.less(pair.Key, low) && !tree.less(high, pair.Key) {
					rangeWant = append(rangeWant, pair)
				}
			}
			if got := tree.RangeScan(low, high); !reflect.DeepEqual(got, rangeWant) {
				t.Fatalf("RangeScan(%d,%d)=%v, want %v", low, high, got, rangeWant)
			}
		}
	}
}

func TestIndependentBPlusAuditFiniteHistories(t *testing.T) {
	keys := []int{-1, 0, 1, 2}
	histories, transitions := 0, 0
	for _, descending := range []bool{false, true} {
		less := func(a, b int) bool { return a < b }
		if descending {
			less = func(a, b int) bool { return a > b }
		}
		for encoded := 0; encoded < 8*8*8*8*8; encoded++ {
			state := encoded
			tree := New[int, string](2, less)
			model := make(map[int]string)
			for step := 0; step < 5; step++ {
				command := state % 8
				state /= 8
				key := keys[command%len(keys)]
				if command < 4 {
					value := fmt.Sprintf("history-%d-step-%d", encoded, step)
					tree.Insert(key, value)
					model[key] = value
				} else {
					tree.Delete(key)
					delete(model, key)
				}
				compareBPlusWithMap(t, tree, model)
				transitions++
			}
			histories++
		}
	}
	if histories != 65_536 || transitions != 327_680 {
		t.Fatalf("histories/transitions=%d/%d", histories, transitions)
	}
	t.Logf("audited %d five-command histories and %d transitions", histories, transitions)
}

func TestIndependentBPlusAuditSeededDegreesAndComparators(t *testing.T) {
	transitions := 0
	for _, degree := range []int{2, 3, 5, 11} {
		for direction := 0; direction < 2; direction++ {
			less := func(a, b int) bool { return a < b }
			if direction == 1 {
				less = func(a, b int) bool { return a > b }
			}
			rng := rand.New(rand.NewSource(int64(0xa02d + degree*37 + direction)))
			tree := New[int, string](degree, less)
			model := make(map[int]string)
			for step := 0; step < 4096; step++ {
				key := rng.Intn(41) - 20
				switch rng.Intn(4) {
				case 0:
					tree.Delete(key)
					delete(model, key)
				case 1:
					_, _ = tree.Search(key)
				default:
					value := fmt.Sprintf("seed-%d-%d-%d", degree, key, step)
					tree.Insert(key, value)
					model[key] = value
				}
				compareBPlusWithMap(t, tree, model)
				transitions++
			}
		}
	}
	if transitions != 32_768 {
		t.Fatalf("seeded transitions=%d", transitions)
	}
	t.Logf("audited %d seeded transitions", transitions)
}

func TestIndependentBPlusAuditBorrowAndRootContraction(t *testing.T) {
	for _, test := range []struct {
		name    string
		inserts []int
		deletes []int
	}{
		{name: "redistribute from left", inserts: []int{10, 20, 30, 5, 3}, deletes: []int{30, 20}},
		{name: "redistribute from right", inserts: []int{10, 20, 30, 40, 50, 60}, deletes: []int{10}},
	} {
		t.Run(test.name, func(t *testing.T) {
			tree := New[int, string](2, func(a, b int) bool { return a < b })
			model := make(map[int]string)
			for _, key := range test.inserts {
				value := fmt.Sprintf("v%d", key)
				tree.Insert(key, value)
				model[key] = value
				compareBPlusWithMap(t, tree, model)
			}
			for _, key := range test.deletes {
				tree.Delete(key)
				delete(model, key)
				compareBPlusWithMap(t, tree, model)
			}
		})
	}

	tree := New[int, string](2, func(a, b int) bool { return a < b })
	model := make(map[int]string)
	priorHeight := -1
	for key := 0; key < 96; key++ {
		tree.Insert(key, fmt.Sprintf("v%d", key))
		model[key] = fmt.Sprintf("v%d", key)
		compareBPlusWithMap(t, tree, model)
		priorHeight = tree.Height()
	}
	contracted := false
	for key := 0; key < 96; key++ {
		tree.Delete(key)
		delete(model, key)
		compareBPlusWithMap(t, tree, model)
		if tree.Height() < priorHeight {
			contracted = true
			priorHeight = tree.Height()
		}
	}
	if !contracted || tree.root != nil || tree.firstLeaf != nil || tree.Height() != -1 {
		t.Fatalf("root did not contract to empty, height=%d", tree.Height())
	}
}

func TestIndependentBPlusAuditPreservesSoundHistoricalFence(t *testing.T) {
	tree := New[int, string](2, func(a, b int) bool { return a < b })
	model := make(map[int]string)
	for _, key := range []int{1, 2, 3, 4} {
		tree.Insert(key, fmt.Sprintf("v%d", key))
		model[key] = fmt.Sprintf("v%d", key)
	}
	tree.Delete(3)
	delete(model, 3)
	compareBPlusWithMap(t, tree, model)
	root, ok := tree.root.(*bplusInternal[int, string])
	if !ok || len(root.keys) != 1 || root.keys[0] != 3 {
		t.Fatalf("expected preserved separator 3, root=%#v", tree.root)
	}
	if rightmost := root.children[1].(*bplusLeaf[int, string]); len(rightmost.keys) == 0 || rightmost.keys[0] != 4 {
		t.Fatalf("expected right leaf minimum 4, got %#v", root.children[1])
	}
}

func TestIndependentBPlusAuditUsesOverflowSafeOccupancy(t *testing.T) {
	maxInt := int(^uint(0) >> 1)
	for _, degree := range []int{2, maxInt/2 + 2, maxInt} {
		tree := New[int, string](degree, func(a, b int) bool { return a < b })
		tree.Insert(1, "one")
		report, err := auditBPlusTree(tree)
		if err != nil || report.count != 1 || !reflect.DeepEqual(report.entries, []KeyValue[int, string]{{Key: 1, Value: "one"}}) {
			t.Fatalf("degree %d independent audit=(%+v,%v)", degree, report, err)
		}
	}
}

func TestIndependentBPlusAuditRejectsMalformedPrivateNodes(t *testing.T) {
	build := func() *BPlusTree[int, string] {
		tree := New[int, string](2, func(a, b int) bool { return a < b })
		for key := 1; key <= 32; key++ {
			tree.Insert(key, fmt.Sprintf("v%d", key))
		}
		return tree
	}
	for _, test := range []struct {
		name    string
		corrupt func(*BPlusTree[int, string])
	}{
		{name: "separator violates descendant route", corrupt: func(tree *BPlusTree[int, string]) {
			root := tree.root.(*bplusInternal[int, string])
			node := root.children[0]
			for !node.isLeaf() {
				internal := node.(*bplusInternal[int, string])
				node = internal.children[len(internal.children)-1]
			}
			leaf := node.(*bplusLeaf[int, string])
			leaf.keys[len(leaf.keys)-1] = root.keys[0]
		}},
		{name: "leaf value pairing", corrupt: func(tree *BPlusTree[int, string]) {
			tree.firstLeaf.values = nil
		}},
		{name: "skipped leaf link", corrupt: func(tree *BPlusTree[int, string]) {
			tree.firstLeaf.next = tree.firstLeaf.next.next
		}},
		{name: "foreign leaf link", corrupt: func(tree *BPlusTree[int, string]) {
			tree.firstLeaf.next = &bplusLeaf[int, string]{keys: []int{100}, values: []string{"foreign"}}
		}},
		{name: "leaf link cycle", corrupt: func(tree *BPlusTree[int, string]) {
			leaf := tree.firstLeaf
			for leaf.next != nil {
				leaf = leaf.next
			}
			leaf.next = tree.firstLeaf
		}},
		{name: "nil internal child", corrupt: func(tree *BPlusTree[int, string]) {
			root := tree.root.(*bplusInternal[int, string])
			root.children[0] = nil
		}},
		{name: "typed nil leaf child", corrupt: func(tree *BPlusTree[int, string]) {
			root := tree.root.(*bplusInternal[int, string])
			var nilLeaf *bplusLeaf[int, string]
			root.children[0] = nilLeaf
		}},
		{name: "non-comparable unknown node", corrupt: func(tree *BPlusTree[int, string]) {
			root := tree.root.(*bplusInternal[int, string])
			root.children[0] = bplusMalformedSliceNode{1, 2}
		}},
		{name: "shared child identity", corrupt: func(tree *BPlusTree[int, string]) {
			root := tree.root.(*bplusInternal[int, string])
			root.children[1] = root.children[0]
		}},
		{name: "cyclic child identity", corrupt: func(tree *BPlusTree[int, string]) {
			root := tree.root.(*bplusInternal[int, string])
			root.children[0] = root
		}},
	} {
		t.Run(test.name, func(t *testing.T) {
			tree := build()
			test.corrupt(tree)
			if _, err := auditBPlusTree(tree); err == nil {
				t.Fatal("independent walker accepted malformed tree")
			}
		})
	}
}
