package heap

import (
	"errors"
	"fmt"
	"math/rand"
	"reflect"
	"testing"
)

func minIntLess(a, b int) bool { return a < b }
func maxIntLess(a, b int) bool { return a > b }

func TestBoundedConstructionAndErrorPrecedence(t *testing.T) {
	if _, err := NewBoundedHeap[int](-1, minIntLess); !errors.Is(err, ErrNegativeLimit) {
		t.Fatalf("negative limit error = %v", err)
	}
	if _, err := NewBoundedHeap[int](0, nil); !errors.Is(err, ErrNilComparator) {
		t.Fatalf("nil comparator error = %v", err)
	}
	if _, err := NewBoundedHeapFromSlice([]int{1}, -1, nil); !errors.Is(err, ErrNegativeLimit) {
		t.Fatalf("constructor should check negative limit first, got %v", err)
	}
	if _, err := NewBoundedHeapFromSlice([]int{1}, 0, nil); !errors.Is(err, ErrNilComparator) {
		t.Fatalf("constructor should check comparator before length, got %v", err)
	}
	if _, err := NewBoundedHeapFromSlice([]int{1}, 0, minIntLess); !errors.Is(err, ErrInputExceedsLimit) {
		t.Fatalf("oversized input error = %v", err)
	}

	queue, err := NewBoundedHeap[int](0, minIntLess)
	if err != nil {
		t.Fatal(err)
	}
	if queue.Limit() != 0 || queue.Len() != 0 {
		t.Fatalf("zero-capacity queue reports limit=%d len=%d", queue.Limit(), queue.Len())
	}
	if err := queue.Push(1); !errors.Is(err, ErrQueueFull) {
		t.Fatalf("zero-capacity push error = %v", err)
	}
	if got, ok := queue.Peek(); ok || got != 0 {
		t.Fatalf("empty peek = %d, %v", got, ok)
	}
	if got, ok := queue.Pop(); ok || got != 0 {
		t.Fatalf("empty pop = %d, %v", got, ok)
	}

	var nilQueue *BoundedHeap[int]
	if err := nilQueue.Push(1); !errors.Is(err, ErrUninitialized) {
		t.Fatalf("nil receiver push error = %v", err)
	}
	var zeroQueue BoundedHeap[int]
	if err := zeroQueue.Push(1); !errors.Is(err, ErrUninitialized) {
		t.Fatalf("zero-value push error = %v", err)
	}
	if zeroQueue.Limit() != 0 || zeroQueue.Len() != 0 {
		t.Fatalf("zero value reports limit=%d len=%d", zeroQueue.Limit(), zeroQueue.Len())
	}
	zeroQueue.Clear()
	nilQueue.Clear()
}

func TestBoundedPushFullClearAndReuse(t *testing.T) {
	queue, err := NewBoundedHeap[int](3, minIntLess)
	if err != nil {
		t.Fatal(err)
	}
	model := []int{}
	for _, value := range []int{7, -2, 4} {
		if err := queue.Push(value); err != nil {
			t.Fatal(err)
		}
		model = append(model, value)
	}
	before := append([]int(nil), queue.items...)
	if err := queue.Push(99); !errors.Is(err, ErrQueueFull) {
		t.Fatalf("full queue push error = %v", err)
	}
	if !reflect.DeepEqual(queue.items, before) {
		t.Fatalf("full rejection mutated items: got %v, want %v", queue.items, before)
	}
	if err := checkHeapState(queue, model, minIntLess); err != nil {
		t.Fatal(err)
	}
	queue.Clear()
	if queue.Len() != 0 || queue.Limit() != 3 {
		t.Fatalf("clear changed len/limit: %d/%d", queue.Len(), queue.Limit())
	}
	if err := queue.Push(11); err != nil {
		t.Fatalf("push after clear: %v", err)
	}
	if top, ok := queue.Peek(); !ok || top != 11 {
		t.Fatalf("peek after reuse = %d, %v", top, ok)
	}
}

func TestFloydConstructorOwnsStorageAndOrdersEveryValue(t *testing.T) {
	input := []int{5, -1, 8, 8, 0, int(^uint(0) >> 1), -int(^uint(0)>>1) - 1}
	original := append([]int(nil), input...)
	queue, err := NewBoundedHeapFromSlice(input, len(input), minIntLess)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(input, original) {
		t.Fatalf("constructor reordered caller input: got %v, want %v", input, original)
	}
	input[0] = 123456
	if err := checkHeapState(queue, original, minIntLess); err != nil {
		t.Fatalf("queue aliased caller slice: %v", err)
	}
	for index, want := range sortedByModel(original, minIntLess) {
		got, ok := queue.Pop()
		if !ok || got != want {
			t.Fatalf("pop %d = %d, %v; want %d", index, got, ok, want)
		}
	}
	if _, ok := queue.Pop(); ok {
		t.Fatal("pop on empty queue succeeded")
	}
}

func TestPopAndClearReleasePointerReferences(t *testing.T) {
	values := []*int{}
	for _, value := range []int{4, 1, 3} {
		copy := value
		values = append(values, &copy)
	}
	less := func(a, b *int) bool { return *a < *b }
	queue, err := NewBoundedHeapFromSlice(values, len(values), less)
	if err != nil {
		t.Fatal(err)
	}
	backing := queue.items[:cap(queue.items)]
	last := len(queue.items) - 1
	if _, ok := queue.Pop(); !ok {
		t.Fatal("pop failed")
	}
	if backing[last] != nil {
		t.Fatal("pop retained the vacated pointer")
	}
	queue.Clear()
	for index, value := range backing {
		if value != nil {
			t.Fatalf("clear retained pointer at backing slot %d", index)
		}
	}
}

func TestExistingMinHeapClearsSlotsAndKeepsItsAPI(t *testing.T) {
	first, second := 2, 1
	h := NewMinHeap(func(a, b *int) bool { return *a < *b })
	h.Push(&first)
	h.Push(&second)
	backing := h.items[:cap(h.items)]
	if got, ok := h.Pop(); !ok || got != &second {
		t.Fatalf("pop = %p, %v; want %p", got, ok, &second)
	}
	if backing[1] != nil {
		t.Fatal("MinHeap.Pop retained the vacated pointer")
	}
	h.Clear()
	if h.Len() != 0 {
		t.Fatalf("clear left length %d", h.Len())
	}
}

func TestNilAndZeroValueCompatibilitySemantics(t *testing.T) {
	var legacy *MinHeap[int]
	if legacy.Len() != 0 || legacy.Clone() != nil {
		t.Fatal("nil MinHeap Len/Clone semantics changed")
	}
	if _, ok := legacy.Peek(); ok {
		t.Fatal("nil MinHeap.Peek succeeded")
	}
	if _, ok := legacy.Pop(); ok {
		t.Fatal("nil MinHeap.Pop succeeded")
	}
	legacy.Clear()
	if NewMinHeap[int](minIntLess).Clone() == nil {
		t.Fatal("Clone of an initialized heap returned nil")
	}
	if NewMinHeap[int](nil).Len() != 0 {
		t.Fatal("empty heap Len changed")
	}
	func() {
		defer func() {
			if recover() == nil {
				t.Fatal("legacy Push without comparator stopped panicking")
			}
		}()
		NewMinHeap[int](nil).Push(1)
	}()

	var checked *BoundedHeap[int]
	if checked.Len() != 0 || checked.Limit() != 0 {
		t.Fatal("nil checked heap Len/Limit must be zero")
	}
	if _, ok := checked.Peek(); ok {
		t.Fatal("nil checked Peek succeeded")
	}
	if _, ok := checked.Pop(); ok {
		t.Fatal("nil checked Pop succeeded")
	}
	checked.Clear()
	if err := checked.Push(1); !errors.Is(err, ErrUninitialized) {
		t.Fatalf("nil checked Push = %v, want ErrUninitialized", err)
	}
	var zero BoundedHeap[int]
	if zero.Len() != 0 || zero.Limit() != 0 {
		t.Fatal("zero-value checked heap Len/Limit must be zero")
	}
	if err := zero.Push(1); !errors.Is(err, ErrUninitialized) {
		t.Fatalf("zero-value checked Push = %v, want ErrUninitialized", err)
	}
}

func TestMinHeapCloneOwnsSliceStorage(t *testing.T) {
	original := NewMinHeap(minIntLess)
	for _, value := range []int{8, 3, 5} {
		original.Push(value)
	}
	clone := original.Clone()
	clone.Push(1)
	if got, ok := original.Peek(); !ok || got != 3 {
		t.Fatalf("original changed after clone push: %d,%v", got, ok)
	}
	if got, ok := clone.Peek(); !ok || got != 1 {
		t.Fatalf("clone root = %d,%v, want 1", got, ok)
	}
}

func TestMinHeapPopsSingletonAndChoosesChildren(t *testing.T) {
	singleton := NewMinHeap(minIntLess)
	singleton.Push(7)
	if got, ok := singleton.Pop(); !ok || got != 7 {
		t.Fatalf("singleton pop = %d,%v", got, ok)
	}
	if singleton.Len() != 0 {
		t.Fatalf("singleton pop left length %d", singleton.Len())
	}

	queue := NewMinHeap(minIntLess)
	for _, value := range []int{4, 8, 6, 9, 10, 7, 11} {
		queue.Push(value)
	}
	want := []int{4, 6, 7, 8, 9, 10, 11}
	for index, expected := range want {
		if got, ok := queue.Pop(); !ok || got != expected {
			t.Fatalf("pop %d = %d,%v, want %d", index, got, ok, expected)
		}
	}
}

func TestChildIndexGuardRunsBeforeMultiplication(t *testing.T) {
	min := NewMinHeap(minIntLess)
	min.Push(1)
	min.down(int(^uint(0) >> 1))
	bounded, err := NewBoundedHeap[int](2, minIntLess)
	if err != nil {
		t.Fatal(err)
	}
	if err := bounded.Push(1); err != nil {
		t.Fatal(err)
	}
	bounded.down(int(^uint(0) >> 1))
}

type historyAction struct {
	kind  byte // 0..2 push a small value, 3 peek, 4 pop, 5 clear
	value int
}

func applyHistoryAction(queue *BoundedHeap[int], model *[]int, limit int, less func(int, int) bool, action historyAction) error {
	switch action.kind {
	case 0, 1, 2:
		before := append([]int(nil), queue.items...)
		err := queue.Push(action.value)
		if len(*model) == limit {
			if !errors.Is(err, ErrQueueFull) {
				return fmt.Errorf("push at limit returned %v, want ErrQueueFull", err)
			}
			if !equalIntSlices(queue.items, before) {
				return fmt.Errorf("full push changed heap from %v to %v", before, queue.items)
			}
		} else {
			if err != nil {
				return fmt.Errorf("push returned %w", err)
			}
			*model = append(*model, action.value)
		}
	case 3:
		want, wantOK := modelTop(*model, less)
		got, gotOK := queue.Peek()
		if gotOK != wantOK || gotOK && got != want {
			return fmt.Errorf("peek = %d,%v; want %d,%v", got, gotOK, want, wantOK)
		}
	case 4:
		want, wantOK := modelPop(model, less)
		got, gotOK := queue.Pop()
		if gotOK != wantOK || gotOK && got != want {
			return fmt.Errorf("pop = %d,%v; want %d,%v", got, gotOK, want, wantOK)
		}
	case 5:
		queue.Clear()
		*model = nil
	default:
		return fmt.Errorf("unknown action kind %d", action.kind)
	}
	return checkHeapState(queue, *model, less)
}

func equalIntSlices(left, right []int) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if left[index] != right[index] {
			return false
		}
	}
	return true
}

func enumerateHistories(t *testing.T, queue *BoundedHeap[int], model []int, less func(int, int) bool, remaining int, path []historyAction) {
	t.Helper()
	if remaining == 0 {
		return
	}
	// A six-action alphabet and maximum history length five give
	// 1+6+6^2+...+6^5 = 9,331 histories for each comparator.
	actions := []historyAction{
		{kind: 0, value: -1}, {kind: 1, value: 0}, {kind: 2, value: 1},
		{kind: 3}, {kind: 4}, {kind: 5},
	}
	for _, action := range actions {
		child := *queue
		child.items = append([]int(nil), queue.items...)
		childModel := append([]int(nil), model...)
		childPath := append(append([]historyAction(nil), path...), action)
		if err := applyHistoryAction(&child, &childModel, child.limit, less, action); err != nil {
			t.Fatalf("history %v: %v", childPath, err)
		}
		enumerateHistories(t, &child, childModel, less, remaining-1, childPath)
	}
}

func TestFiniteOperationHistoriesAgainstIndependentModel(t *testing.T) {
	for _, less := range []func(int, int) bool{minIntLess, maxIntLess} {
		queue, err := NewBoundedHeap[int](3, less)
		if err != nil {
			t.Fatal(err)
		}
		enumerateHistories(t, queue, nil, less, 5, nil)
	}
}

func TestSeededLongHistoriesAgainstIndependentModel(t *testing.T) {
	maxInt := int(^uint(0) >> 1)
	minInt := -maxInt - 1
	extremes := []int{minInt, -2, -1, 0, 1, 2, maxInt}
	for _, less := range []func(int, int) bool{minIntLess, maxIntLess} {
		for seed := int64(0); seed < 32; seed++ {
			random := rand.New(rand.NewSource(seed))
			limit := 1 + random.Intn(32)
			queue, err := NewBoundedHeap[int](limit, less)
			if err != nil {
				t.Fatal(err)
			}
			model := []int{}
			for step := 0; step < 512; step++ {
				action := historyAction{kind: byte(random.Intn(6))}
				if action.kind < 3 {
					if random.Intn(4) == 0 {
						action.value = extremes[random.Intn(len(extremes))]
					} else {
						action.value = random.Intn(101) - 50
					}
				}
				if err := applyHistoryAction(queue, &model, limit, less, action); err != nil {
					t.Fatalf("seed=%d step=%d action=%+v: %v", seed, step, action, err)
				}
			}
		}
	}
}

func TestIndependentModelRejectsSelectedAdapterControls(t *testing.T) {
	// These are synthetic heap observations, not mutations of the implementation.
	if err := validateSnapshot(heapSnapshot{items: []int{1, 2, 3}, limit: 2}, []int{1, 2, 3}, minIntLess); err == nil {
		t.Fatal("model accepted an adapter that admitted an over-capacity item")
	}
	if err := validateSnapshot(heapSnapshot{items: []int{3, 1, 2}, limit: 3}, []int{1, 2, 3}, minIntLess); err == nil {
		t.Fatal("model accepted an adapter snapshot after returning the wrong minimum")
	}
}

func FuzzBoundedHeapHistories(f *testing.F) {
	f.Add([]byte{3, 0, 255, 4, 2, 1, 5, 3, 3, 4})
	f.Add([]byte{1, 2, 254, 0, 7, 0, 1, 0, 5, 4})
	f.Add([]byte("B000")) // Empty zero-limit full-push no-op regression.
	f.Fuzz(func(t *testing.T, data []byte) {
		// Bound each generated history so fuzz inputs cannot turn this model
		// oracle into an unbounded allocation or runtime request.
		if len(data) > 1+3*512 {
			data = data[:1+3*512]
		}
		limit := 0
		if len(data) > 0 {
			limit = int(data[0] % 33)
		}
		var operations []byte
		if len(data) > 1 {
			operations = data[1:]
		}
		for _, less := range []func(int, int) bool{minIntLess, maxIntLess} {
			queue, err := NewBoundedHeap[int](limit, less)
			if err != nil {
				t.Fatal(err)
			}
			model := []int{}
			for offset := 0; offset+2 < len(operations); offset += 3 {
				action := historyAction{kind: operations[offset] % 6}
				value := int(int8(operations[offset+1]))
				if action.kind < 3 {
					action.value = value
				}
				if err := applyHistoryAction(queue, &model, limit, less, action); err != nil {
					t.Fatalf("byte offset %d: %v", offset, err)
				}
			}
		}
	})
}

type heapSnapshot struct {
	items []int
	limit int
}

func checkHeapState(queue *BoundedHeap[int], model []int, less func(int, int) bool) error {
	if queue == nil {
		return fmt.Errorf("queue is nil")
	}
	if err := validateSnapshot(heapSnapshot{items: queue.items, limit: queue.limit}, model, less); err != nil {
		return err
	}
	if queue.Len() != len(model) {
		return fmt.Errorf("Len() = %d, model has %d", queue.Len(), len(model))
	}
	want, wantOK := modelTop(model, less)
	got, gotOK := queue.Peek()
	if gotOK != wantOK || gotOK && got != want {
		return fmt.Errorf("Peek() = %d,%v; model = %d,%v", got, gotOK, want, wantOK)
	}
	return nil
}

func validateSnapshot(snapshot heapSnapshot, model []int, less func(int, int) bool) error {
	if snapshot.limit < 0 || len(snapshot.items) > snapshot.limit {
		return fmt.Errorf("snapshot length %d exceeds limit %d", len(snapshot.items), snapshot.limit)
	}
	if len(snapshot.items) != len(model) {
		return fmt.Errorf("snapshot length %d differs from model length %d", len(snapshot.items), len(model))
	}
	for child := 1; child < len(snapshot.items); child++ {
		parent := (child - 1) / 2
		if less(snapshot.items[child], snapshot.items[parent]) {
			return fmt.Errorf("heap order violated between parent %d and child %d", parent, child)
		}
	}
	counts := make(map[int]int, len(model))
	for _, value := range model {
		counts[value]++
	}
	for _, value := range snapshot.items {
		if counts[value] == 0 {
			return fmt.Errorf("snapshot contains unexpected value %d", value)
		}
		counts[value]--
	}
	for value, count := range counts {
		if count != 0 {
			return fmt.Errorf("snapshot is missing %d copies of %d", count, value)
		}
	}
	return nil
}

func modelTop(values []int, less func(int, int) bool) (int, bool) {
	if len(values) == 0 {
		return 0, false
	}
	best := values[0]
	for _, value := range values[1:] {
		if less(value, best) {
			best = value
		}
	}
	return best, true
}

func modelPop(values *[]int, less func(int, int) bool) (int, bool) {
	items := *values
	if len(items) == 0 {
		return 0, false
	}
	bestIndex := 0
	for index := 1; index < len(items); index++ {
		if less(items[index], items[bestIndex]) {
			bestIndex = index
		}
	}
	best := items[bestIndex]
	copy(items[bestIndex:], items[bestIndex+1:])
	*values = items[:len(items)-1]
	return best, true
}

func sortedByModel(values []int, less func(int, int) bool) []int {
	copyOfValues := append([]int(nil), values...)
	sorted := make([]int, 0, len(values))
	for len(copyOfValues) > 0 {
		value, _ := modelPop(&copyOfValues, less)
		sorted = append(sorted, value)
	}
	return sorted
}
