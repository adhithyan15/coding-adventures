package ftleraseroleevidence

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

func TestQualifiesIndistinguishableInterruptedHistories(t *testing.T) {
	geometry := Geometry{BlockCount: 2, PagesPerBlock: 3, MaxCompletedErases: 4}
	observed := BlockObservation{
		Block: 0, CompletedErases: 0, IncompleteErase: true,
		Programmed: []bool{false, false, false},
	}
	firstIntent := RetainedIntent{Sequence: 7, Block: 0, CompletedErases: 0, Stale: []bool{true, false, false}}
	secondIntent := RetainedIntent{Sequence: 8, Block: 0, CompletedErases: 0, Stale: []bool{false, true, false}}
	first, err := Qualify(geometry, 7, firstIntent, observed)
	if err != nil || !reflect.DeepEqual(first, []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Free, ftlpagemap.Free}) {
		t.Fatalf("first indistinguishable history roles=%v err=%v", first, err)
	}
	second, err := Qualify(geometry, 8, secondIntent, observed)
	if err != nil || !reflect.DeepEqual(second, []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Stale, ftlpagemap.Free}) {
		t.Fatalf("second indistinguishable history roles=%v err=%v", second, err)
	}
	if reflect.DeepEqual(first, second) {
		t.Fatal("different retained stale masks produced identical role evidence")
	}
}

func TestQualifyPrecedenceAndZeroResults(t *testing.T) {
	geometry := Geometry{BlockCount: 1, PagesPerBlock: 3, MaxCompletedErases: 2}
	intent := RetainedIntent{Sequence: 11, Block: 0, CompletedErases: 1, Stale: []bool{true, false, false}}
	observed := BlockObservation{Block: 0, CompletedErases: 1, IncompleteErase: true, Programmed: []bool{false, false, false}}
	tests := []struct {
		name     string
		geometry Geometry
		sequence uint64
		intent   RetainedIntent
		observed BlockObservation
		want     error
	}{
		{"geometry before all evidence", Geometry{}, 0, intent, observed, ErrInvalidGeometry},
		{"one physical page is unsupported", Geometry{BlockCount: 1, PagesPerBlock: 1, MaxCompletedErases: 2}, 11, intent, observed, ErrInvalidGeometry},
		{"safe ceiling before block ordinal", Geometry{BlockCount: 1, PagesPerBlock: 3, MaxCompletedErases: maxSafeEraseCount + 1}, 11, intent, observed, ErrInvalidEraseCount},
		{"block ordinal before incomplete flag", geometry, 11, intent, BlockObservation{Block: 1}, ErrInvalidBlock},
		{"incomplete flag before budget", geometry, 11, intent, BlockObservation{Block: 0, CompletedErases: 2}, ErrBlockNotIncomplete},
		{"remaining budget before evidence shape", geometry, 11, intent, BlockObservation{Block: 0, CompletedErases: 2, IncompleteErase: true}, ErrEraseBudgetExhausted},
		{"observed count above ceiling", geometry, 11, intent, BlockObservation{Block: 0, CompletedErases: 3, IncompleteErase: true}, ErrInvalidEraseCount},
		{"exact arrays before identity", geometry, 99, intent, BlockObservation{Block: 0, CompletedErases: 1, IncompleteErase: true}, ErrInvalidEvidenceShape},
		{"sequence before block", geometry, 12, RetainedIntent{Sequence: 11, Block: 9, Stale: []bool{true, false, false}}, observed, ErrIntentSequenceMismatch},
		{"block before generation", geometry, 11, RetainedIntent{Sequence: 11, Block: 9, CompletedErases: 0, Stale: []bool{true, false, false}}, observed, ErrIntentBlockMismatch},
		{"generation before stale roles", geometry, 11, RetainedIntent{Sequence: 11, Block: 0, CompletedErases: 0, Stale: []bool{false, false, false}}, observed, ErrIntentGenerationMismatch},
		{"stale required before marker implication", geometry, 11, RetainedIntent{Sequence: 11, Block: 0, CompletedErases: 1, Stale: []bool{false, false, false}}, BlockObservation{Block: 0, CompletedErases: 1, IncompleteErase: true, Programmed: []bool{true, false, false}}, ErrNoStalePages},
		{"set marker requires retained stale flag", geometry, 11, intent, BlockObservation{Block: 0, CompletedErases: 1, IncompleteErase: true, Programmed: []bool{false, true, false}}, ErrProgrammedFreePage},
		{"nonzero matching sequence required", geometry, 0, intent, observed, ErrIntentSequenceMismatch},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			got, err := Qualify(test.geometry, test.sequence, test.intent, test.observed)
			if !errors.Is(err, test.want) || got != nil {
				t.Fatalf("Qualify() = %v, %v; want nil roles and %v", got, err, test.want)
			}
		})
	}
}

func TestQualifyExhaustiveSmallMarkerAndStaleMasks(t *testing.T) {
	checked := 0
	accepted := 0
	for pages := 1; pages <= 5; pages++ {
		geometry := Geometry{BlockCount: 2, PagesPerBlock: int64(pages), MaxCompletedErases: 3}
		for staleMask := 0; staleMask < 1<<pages; staleMask++ {
			stale := make([]bool, pages)
			for page := range stale {
				stale[page] = staleMask&(1<<page) != 0
			}
			intent := RetainedIntent{Sequence: 1, Block: 1, CompletedErases: 1, Stale: stale}
			for markerMask := 0; markerMask < 1<<pages; markerMask++ {
				markers := make([]bool, pages)
				for page := range markers {
					markers[page] = markerMask&(1<<page) != 0
				}
				observed := BlockObservation{Block: 1, CompletedErases: 1, IncompleteErase: true, Programmed: markers}
				want, wantErr := modelQualify(geometry, 1, intent, observed)
				got, err := Qualify(geometry, 1, intent, observed)
				if !sameError(err, wantErr) || !reflect.DeepEqual(got, want) {
					t.Fatalf("pages=%d stale=%05b markers=%05b roles=%v/%v error=%v/%v", pages, staleMask, markerMask, got, want, err, wantErr)
				}
				if err == nil {
					accepted++
				}
				checked++
			}
		}
	}
	if checked != 1364 || accepted == 0 {
		t.Fatalf("checked=%d accepted=%d; want 1,364 combinations and successes", checked, accepted)
	}
	t.Logf("checked %d marker/stale pairs; %d accepted", checked, accepted)
}

func TestQualifyDetachedResultsAcrossCalls(t *testing.T) {
	geometry := Geometry{BlockCount: 2, PagesPerBlock: 3, MaxCompletedErases: 4}
	observed := BlockObservation{Block: 1, IncompleteErase: true, Programmed: []bool{false, false, false}}
	firstIntent := RetainedIntent{Sequence: 1, Block: 1, Stale: []bool{true, false, false}}
	first, err := Qualify(geometry, 1, firstIntent, observed)
	if err != nil {
		t.Fatal(err)
	}
	secondIntent := RetainedIntent{Sequence: 2, Block: 1, Stale: []bool{false, true, false}}
	second, err := Qualify(geometry, 2, secondIntent, observed)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(first, []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Free, ftlpagemap.Free}) {
		t.Fatalf("later call changed earlier returned roles: %v", first)
	}
	first[0] = ftlpagemap.Free
	if !firstIntent.Stale[0] || second[1] != ftlpagemap.Stale {
		t.Fatal("returned role changes aliased intent or another result")
	}
}

func TestQualifyMaximumGeometryAndDetachedInputs(t *testing.T) {
	geometry := Geometry{BlockCount: 2, PagesPerBlock: 64, MaxCompletedErases: maxSafeEraseCount}
	intent := RetainedIntent{Sequence: 9, Block: 1, CompletedErases: maxSafeEraseCount - 1, Stale: make([]bool, 64)}
	markers := make([]bool, 64)
	intent.Stale[63] = true
	observed := BlockObservation{Block: 1, CompletedErases: maxSafeEraseCount - 1, IncompleteErase: true, Programmed: markers}
	roles, err := Qualify(geometry, 9, intent, observed)
	if err != nil || len(roles) != 64 || roles[63] != ftlpagemap.Stale || roles[0] != ftlpagemap.Free {
		t.Fatalf("maximum supported geometry roles=%v err=%v", roles, err)
	}
	intent.Stale[63] = false
	markers[0] = true
	if roles[63] != ftlpagemap.Stale || roles[0] != ftlpagemap.Free {
		t.Fatal("result retained caller arrays")
	}
}

func TestFiveThousandSeededEvidenceInputsMatchModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xe3b3d))
	accepted := 0
	refused := 0
	for trial := 0; trial < 5000; trial++ {
		geometry, sequence, intent, observed := randomEvidence(random)
		want, wantErr := modelQualify(geometry, sequence, intent, observed)
		got, err := Qualify(geometry, sequence, intent, observed)
		if !sameError(err, wantErr) || !reflect.DeepEqual(got, want) {
			t.Fatalf("trial=%d got=%v/%v want=%v/%v", trial, got, err, want, wantErr)
		}
		if err != nil {
			refused++
		} else {
			accepted++
			intent.Stale[0] = !intent.Stale[0]
			if got[0] != want[0] {
				t.Fatalf("trial=%d result aliased caller intent", trial)
			}
		}
	}
	if accepted == 0 || refused == 0 {
		t.Fatalf("seeded evidence accepted=%d refused=%d", accepted, refused)
	}
	t.Logf("seed 0xe3b3d checked 5,000 evidence tuples: %d accepted, %d refused", accepted, refused)
}

func FuzzQualifyEvidence(f *testing.F) {
	for _, seed := range [][]byte{{}, {0}, {1, 1, 0, 0}, {0xff, 0x55, 0xaa, 3}, {9, 7, 3, 0xff}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 4 {
			raw = raw[:4]
		}
		random := rand.New(rand.NewSource(seedFor(raw)))
		geometry, sequence, intent, observed := randomEvidence(random)
		want, wantErr := modelQualify(geometry, sequence, intent, observed)
		got, err := Qualify(geometry, sequence, intent, observed)
		if !sameError(err, wantErr) || !reflect.DeepEqual(got, want) {
			t.Fatalf("raw=%x got=%v/%v want=%v/%v", raw, got, err, want, wantErr)
		}
	})
}

func TestValidateGeometryExhaustiveDimensionsAndCeilings(t *testing.T) {
	const portableCeiling = uint64(9007199254740991)
	dimensions := make([]int64, 0, 140)
	for value := int64(-2); value <= 130; value++ {
		dimensions = append(dimensions, value)
	}
	dimensions = append(dimensions, -1<<63, 1<<63-1)
	ceilings := []uint64{0, 1, portableCeiling, portableCeiling + 1, ^uint64(0)}
	checked, accepted, refusedGeometry, refusedCount := 0, 0, 0, 0
	for _, blocks := range dimensions {
		for _, pages := range dimensions {
			for _, ceiling := range ceilings {
				geometry := Geometry{BlockCount: blocks, PagesPerBlock: pages, MaxCompletedErases: ceiling}
				want := modelGeometryError(geometry)
				got := ValidateGeometry(geometry)
				if got != want {
					t.Fatalf("ValidateGeometry(%+v)=%v; want %v", geometry, got, want)
				}
				checked++
				switch got {
				case nil:
					accepted++
				case ErrInvalidGeometry:
					refusedGeometry++
				case ErrInvalidEraseCount:
					refusedCount++
				}
			}
		}
	}
	if checked < 90000 || accepted == 0 || refusedGeometry == 0 || refusedCount == 0 {
		t.Fatalf("checked=%d accepted=%d invalidGeometry=%d invalidCount=%d", checked, accepted, refusedGeometry, refusedCount)
	}
	t.Logf("validated %d dimension/ceiling combinations (%d accepted, %d geometry refusals, %d ceiling refusals)", checked, accepted, refusedGeometry, refusedCount)
}

func TestValidateGeometryMaximumProductsAndPrecedence(t *testing.T) {
	const portableCeiling = uint64(9007199254740991)
	for _, pair := range [][2]int64{{1, 2}, {1, 128}, {2, 64}, {4, 32}, {8, 16}, {16, 8}, {32, 4}, {64, 2}, {128, 1}} {
		geometry := Geometry{BlockCount: pair[0], PagesPerBlock: pair[1], MaxCompletedErases: portableCeiling}
		if err := ValidateGeometry(geometry); err != nil {
			t.Fatalf("maximum-product geometry %+v refused: %v", geometry, err)
		}
	}
	if err := ValidateGeometry(Geometry{BlockCount: 1, PagesPerBlock: 1}); err != ErrInvalidGeometry {
		t.Fatalf("single-cell geometry error=%v; want %v", err, ErrInvalidGeometry)
	}
	if err := ValidateGeometry(Geometry{BlockCount: 129, PagesPerBlock: 1, MaxCompletedErases: portableCeiling + 1}); err != ErrInvalidGeometry {
		t.Fatalf("bad dimensions must precede excessive ceiling, got %v", err)
	}
	if err := ValidateGeometry(Geometry{BlockCount: 2, PagesPerBlock: 1, MaxCompletedErases: portableCeiling + 1}); err != ErrInvalidEraseCount {
		t.Fatalf("valid dimensions with excessive ceiling error=%v", err)
	}
	if err := ValidateGeometry(Geometry{BlockCount: 2, PagesPerBlock: 1, MaxCompletedErases: 0}); err != nil {
		t.Fatalf("zero erase budget is valid geometry: %v", err)
	}
}

func modelGeometryError(geometry Geometry) error {
	blocks, pages := geometry.BlockCount, geometry.PagesPerBlock
	if blocks < 1 || pages < 1 || blocks > 128 || pages > 128 {
		return ErrInvalidGeometry
	}
	// Bounds above make this multiplication safe; the oracle uses the literal
	// product domain rather than the runtime's division-based overflow guard.
	product := blocks * pages
	if product < 2 || product > 128 {
		return ErrInvalidGeometry
	}
	if geometry.MaxCompletedErases > 9007199254740991 {
		return ErrInvalidEraseCount
	}
	return nil
}

func seedFor(raw []byte) int64 {
	var seed uint64 = 1469598103934665603
	for _, value := range raw {
		seed = (seed ^ uint64(value)) * 1099511628211
	}
	return int64(seed)
}

func randomEvidence(random *rand.Rand) (Geometry, uint64, RetainedIntent, BlockObservation) {
	const pages = 4
	geometry := Geometry{BlockCount: int64(1 + random.Intn(3)), PagesPerBlock: pages, MaxCompletedErases: uint64(1 + random.Intn(5))}
	sequence := uint64(1 + random.Intn(1000))
	block := int64(random.Intn(int(geometry.BlockCount)))
	completed := uint64(random.Intn(int(geometry.MaxCompletedErases)))
	intent := RetainedIntent{
		Sequence: sequence, Block: block, CompletedErases: completed, Stale: make([]bool, pages),
	}
	observed := BlockObservation{
		Block: block, CompletedErases: completed, IncompleteErase: true,
		Programmed: make([]bool, pages),
	}
	hasStale := false
	for page := 0; page < pages; page++ {
		intent.Stale[page] = random.Intn(2) == 0
		hasStale = hasStale || intent.Stale[page]
		observed.Programmed[page] = intent.Stale[page] && random.Intn(2) == 0
	}
	if !hasStale {
		intent.Stale[random.Intn(pages)] = true
	}
	switch random.Intn(6) {
	case 1:
		observed.IncompleteErase = false
	case 2:
		observed.CompletedErases = geometry.MaxCompletedErases
	case 3:
		sequence++
	case 4:
		intent.Block = geometry.BlockCount
	case 5:
		intent.CompletedErases++
	}
	return geometry, sequence, intent, observed
}

func modelQualify(geometry Geometry, sequence uint64, intent RetainedIntent, observed BlockObservation) ([]ftlpagemap.PhysicalStatus, error) {
	// The oracle spells out bounds and ordered checks separately from Qualify.
	if geometry.BlockCount <= 0 || geometry.PagesPerBlock <= 0 ||
		geometry.BlockCount > 128 || geometry.PagesPerBlock > 128 ||
		geometry.BlockCount > 128/geometry.PagesPerBlock {
		return nil, ErrInvalidGeometry
	}
	if geometry.BlockCount*geometry.PagesPerBlock < 2 {
		return nil, ErrInvalidGeometry
	}
	if geometry.MaxCompletedErases > 9007199254740991 {
		return nil, ErrInvalidEraseCount
	}
	if observed.Block < 0 || observed.Block >= geometry.BlockCount {
		return nil, ErrInvalidBlock
	}
	if observed.IncompleteErase == false {
		return nil, ErrBlockNotIncomplete
	}
	if observed.CompletedErases > geometry.MaxCompletedErases {
		return nil, ErrInvalidEraseCount
	}
	if observed.CompletedErases == geometry.MaxCompletedErases {
		return nil, ErrEraseBudgetExhausted
	}
	width := int(geometry.PagesPerBlock)
	if len(intent.Stale) != width || len(observed.Programmed) != width {
		return nil, ErrInvalidEvidenceShape
	}
	if sequence == 0 || intent.Sequence != sequence {
		return nil, ErrIntentSequenceMismatch
	}
	if intent.Block != observed.Block || intent.Block < 0 || intent.Block >= geometry.BlockCount {
		return nil, ErrIntentBlockMismatch
	}
	if intent.CompletedErases != observed.CompletedErases {
		return nil, ErrIntentGenerationMismatch
	}
	roles := make([]ftlpagemap.PhysicalStatus, width)
	anyStale := false
	for i := range roles {
		if intent.Stale[i] {
			anyStale = true
			roles[i] = ftlpagemap.Stale
		}
	}
	if !anyStale {
		return nil, ErrNoStalePages
	}
	for i, marker := range observed.Programmed {
		if marker && roles[i] != ftlpagemap.Stale {
			return nil, ErrProgrammedFreePage
		}
	}
	return roles, nil
}

func sameError(left, right error) bool {
	if left == nil || right == nil {
		return left == right
	}
	return errors.Is(left, right) && errors.Is(right, left)
}
