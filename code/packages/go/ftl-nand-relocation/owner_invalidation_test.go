package ftlnandrelocation

import (
	"errors"
	"math"
	"math/rand"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func compareOwnerInvalidation(
	t *testing.T,
	device Device,
	model referenceState,
	symbols []uint64,
	observation OwnerObservation,
	observedSymbol uint64,
) (Device, referenceState, bool) {
	t.Helper()
	var wantErr error
	if !referenceConfigured(model) {
		wantErr = ftlpagemap.ErrInvalidGeometry
	} else {
		logical, physical := observation.Owner.Logical, observation.Owner.Physical
		if logical < 0 || logical >= model.logicalCount || physical < 0 || physical >= model.physicalCount ||
			model.mapping[int(logical)] != physical || observedSymbol == 0 || symbols[int(logical)] != observedSymbol {
			wantErr = ErrOwnerObservationStale
		}
	}
	got, err := device.InvalidateOwner(observation)
	if err != wantErr {
		t.Fatalf("InvalidateOwner(%+v) error=%v want=%v", observation.Owner, err, wantErr)
	}
	if wantErr != nil {
		assertDeviceEquals(t, got, model)
		return device, model, false
	}
	want := cloneReference(model)
	want.mapping[int(observation.Owner.Logical)] = -1
	want.status[int(observation.Owner.Physical)] = modelStale
	assertDeviceEquals(t, got, want)
	if !device.MatchesOwner(observation) {
		t.Fatal("invalidation changed the retained predecessor identity")
	}
	return got, want, true
}

func TestInvalidateOwnerRequiresCurrentOpaqueObservation(t *testing.T) {
	var zero Device
	if got, err := zero.InvalidateOwner(OwnerObservation{}); !reflect.DeepEqual(got.Snapshot(), zero.Snapshot()) || !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero-device invalidation=%v", err)
	}
	device, err := New(2, 5, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitial(2, 5, 2)
	device, model = assertWrite(t, device, model, 0, []byte{0x11, 0x22}, modelCut{kind: cutComplete})
	observation, present, err := device.ObserveOwner(0)
	if err != nil || !present || !device.MatchesOwner(observation) {
		t.Fatalf("current observation=%+v/%t/%v", observation.Owner, present, err)
	}
	symbols := []uint64{1, 0}
	invalidated, nextModel, accepted := compareOwnerInvalidation(t, device, model, symbols, observation, 1)
	if !accepted || invalidated.MatchesOwner(observation) {
		t.Fatal("success did not clear exactly the removed owner's identity")
	}
	if invalidated.ownerIdentities[0] != nil {
		t.Fatal("success did not clear exactly the removed owner's identity")
	}
	if _, err := invalidated.InvalidateOwner(observation); err != ErrOwnerObservationStale {
		t.Fatalf("repeated invalidation error=%v; want stale", err)
	}
	if got := invalidated.Snapshot(); !reflect.DeepEqual(got.Map.LogicalToPhysical, nextModel.mapping) {
		t.Fatalf("current map after invalidation=%v want=%v", got.Map.LogicalToPhysical, nextModel.mapping)
	}
	if bytes, programmed := invalidated.Snapshot().Pages[observation.Owner.Physical].Bytes, invalidated.Snapshot().Pages[observation.Owner.Physical].Programmed; !programmed || len(bytes) != 2 || bytes[0] != 0x11 || bytes[1] != 0x22 {
		t.Fatalf("invalidation changed media bytes/marker: %x/%t", bytes, programmed)
	}

	for name, modified := range map[string]OwnerObservation{
		"zero":     {},
		"logical":  {Owner: PageOwner{Logical: 1, Physical: observation.Owner.Physical}, identity: observation.identity},
		"physical": {Owner: PageOwner{Logical: 0, Physical: observation.Owner.Physical + 1}, identity: observation.identity},
	} {
		t.Run(name, func(t *testing.T) {
			got, err := device.InvalidateOwner(modified)
			if err != ErrOwnerObservationStale || !reflect.DeepEqual(got.Snapshot(), device.Snapshot()) {
				t.Fatalf("tampered observation error=%v or changed predecessor", err)
			}
		})
	}

	reusable, err := NewWithEraseBlocks(1, 4, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	reusable, _, err = reusable.Write(0, []byte{0x7a}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	old, _, _ := reusable.ObserveOwner(0)
	reusable, err = reusable.InvalidateOwner(old)
	if err != nil {
		t.Fatal(err)
	}
	reusable, _, err = reusable.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	reusable, _, err = reusable.Write(0, []byte{0x7a}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	newOwner, _, _ := reusable.ObserveOwner(0)
	if newOwner.Owner != old.Owner || reusable.MatchesOwner(old) || !reusable.MatchesOwner(newOwner) {
		t.Fatal("erase and same-coordinate rewrite revived an invalidated identity")
	}
}

func TestInvalidateOwnerPreservesCutAndCopyPublicationRules(t *testing.T) {
	base, err := New(1, 7, 2)
	if err != nil {
		t.Fatal(err)
	}
	base, _, err = base.Write(0, []byte{0x31, 0x32}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	original, _, _ := base.ObserveOwner(0)
	for _, cut := range []Cut{
		{Kind: nandpageprogramcuts.BeforeAccept},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0},
		{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 1},
	} {
		next, _, writeErr := base.Write(0, []byte{0x41, 0x42}, cut)
		if writeErr != nil || !next.MatchesOwner(original) {
			t.Fatalf("interrupted replacement %+v invalidated old identity: %v", cut, writeErr)
		}
		invalidated, err := next.InvalidateOwner(original)
		if err != nil || invalidated.MatchesOwner(original) || !base.MatchesOwner(original) {
			t.Fatalf("old owner invalidation after cut %+v: %v", cut, err)
		}
	}
	for _, cut := range []Cut{{Kind: nandpageprogramcuts.AfterMedia}, {Kind: nandpageprogramcuts.Complete}} {
		next, _, writeErr := base.Write(0, []byte{0x41, 0x42}, cut)
		if writeErr != nil {
			t.Fatal(writeErr)
		}
		if _, err := next.InvalidateOwner(original); err != ErrOwnerObservationStale {
			t.Fatalf("published replacement cut %+v old-observation error=%v", cut, err)
		}
	}

	copyBase, _, plan := copyTestDevice(t)
	move := plan.Copies[0]
	source, _, err := copyBase.ObserveOwner(move.Logical)
	if err != nil {
		t.Fatal(err)
	}
	partial, _, err := copyBase.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if err != nil || !partial.MatchesOwner(source) {
		t.Fatalf("partial copy changed source identity: %v", err)
	}
	partial, err = partial.InvalidateOwner(source)
	if err != nil || partial.MatchesOwner(source) || !copyBase.MatchesOwner(source) {
		t.Fatalf("invalidate after partial copy: %v", err)
	}
	published, _, err := copyBase.CopyLivePage(0, move, Cut{Kind: nandpageprogramcuts.AfterMedia})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := published.InvalidateOwner(source); err != ErrOwnerObservationStale {
		t.Fatalf("old source observation after copy publication=%v", err)
	}
}

func TestInvalidateOwnerSupportsZeroBudgetAndSurvivesEraseCuts(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 6, 1, 2, 0)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 6, 1, 2, 0)
	device, model = assertWrite(t, device, model, 0, []byte{0x55}, modelCut{kind: cutComplete})
	device, model = assertWrite(t, device, model, 1, []byte{0x66}, modelCut{kind: cutComplete})
	removed, _, _ := device.ObserveOwner(0)
	retained, _, _ := device.ObserveOwner(1)
	device, model, accepted := compareOwnerInvalidation(t, device, model, []uint64{1, 2}, removed, 1)
	if !accepted || !device.MatchesOwner(retained) {
		t.Fatal("invalidation of one owner changed its neighbor")
	}
	device, model, accepted = compareOwnerInvalidation(t, device, model, []uint64{1, 2}, retained, 2)
	if !accepted {
		t.Fatal("invalidation of final block owner refused")
	}
	if live, err := device.LivePagesInBlock(0); err != nil || live != 0 {
		t.Fatalf("remaining live count=%d err=%v", live, err)
	}
	if _, _, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete}); !errors.Is(err, nandblockerasecuts.ErrEraseLimit) {
		t.Fatalf("zero-budget erase error=%v", err)
	}
	_ = model

	progress, err := NewWithEraseBlocks(2, 6, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	progress, _, err = progress.Write(0, []byte{0x10}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	progress, _, err = progress.Write(1, []byte{0x20}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	progress, _, err = progress.Write(0, []byte{0x30}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	removed, _, _ = progress.ObserveOwner(0)
	retained, _, _ = progress.ObserveOwner(1)
	progress, err = progress.InvalidateOwner(removed)
	if err != nil {
		t.Fatal(err)
	}
	progress, _, err = progress.EraseBlock(1, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	if err != nil || !progress.MatchesOwner(retained) {
		t.Fatalf("partial erase changed other owner identity: %v", err)
	}
	progress, _, err = progress.EraseBlock(1, EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil || !progress.MatchesOwner(retained) {
		t.Fatalf("completed erase changed other owner identity: %v", err)
	}
}

func TestInvalidateOwnerSeededAndFuzzedAgainstReferenceModel(t *testing.T) {
	random := rand.New(rand.NewSource(0xDB05D2C6B1))
	for history := 0; history < 256; history++ {
		device, err := NewWithEraseBlocks(2, 6, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 2)
		symbols := []uint64{0, 0}
		var old []observedOwnerRecord
		var symbol uint64
		for step := 0; step < 32; step++ {
			logical := int64(random.Intn(2))
			if random.Intn(3) == 0 {
				observation, present, observeErr := device.ObserveOwner(logical)
				if observeErr != nil {
					t.Fatal(observeErr)
				}
				candidateSymbol := uint64(0)
				if present {
					candidateSymbol = symbols[logical]
					old = append(old, observedOwnerRecord{observation: observation, symbol: candidateSymbol})
				}
				if len(old) > 0 && random.Intn(4) == 0 {
					record := old[random.Intn(len(old))]
					observation, candidateSymbol = record.observation, record.symbol
				}
				var accepted bool
				device, model, accepted = compareOwnerInvalidation(t, device, model, symbols, observation, candidateSymbol)
				if accepted {
					symbols[int(observation.Owner.Logical)] = 0
				}
				old = assertObservationModel(t, device, model, symbols, old)
				continue
			}
			payload := []byte{byte(random.Intn(256)), byte(random.Intn(256))}
			cut := symbolicCut(byte(random.Intn(5)))
			_, receipt, writeErr := referenceWrite(model, logical, payload, cut)
			device, model = compareTransition(t, device, model, logical, payload, cut)
			if writeErr == "" && receipt.published {
				symbol++
				symbols[int(logical)] = symbol
			}
			old = assertObservationModel(t, device, model, symbols, old)
		}
	}
}

func TestInvalidateOwnerBoundedBranchHistories(t *testing.T) {
	device, err := New(1, 4, 1)
	if err != nil {
		t.Fatal(err)
	}
	initial := referenceInitial(1, 4, 1)
	cuts := []modelCut{
		{kind: cutBeforeAccept},
		{kind: cutAfterPrefix, prefix: 0},
		{kind: cutAfterMedia},
		{kind: cutComplete},
	}
	var nodes, invalidations int
	var visit func(Device, referenceState, []uint64, []observedOwnerRecord, int)
	visit = func(current Device, state referenceState, symbols []uint64, records []observedOwnerRecord, depth int) {
		nodes++
		records = assertObservationModel(t, current, state, symbols, records)
		if depth == 3 {
			return
		}
		for _, cut := range cuts {
			_, receipt, expectedErr := referenceWrite(state, 0, []byte{0x5a}, cut)
			nextDevice, nextModel := compareTransition(t, current, state, 0, []byte{0x5a}, cut)
			nextSymbols := append([]uint64(nil), symbols...)
			if expectedErr == "" && receipt.published {
				nextSymbols[0]++
			}
			visit(nextDevice, nextModel, nextSymbols, append([]observedOwnerRecord(nil), records...), depth+1)
		}
		observation, present, observeErr := current.ObserveOwner(0)
		if observeErr != nil {
			t.Fatal(observeErr)
		}
		if present {
			invalidations++
			branchRecords := append([]observedOwnerRecord(nil), records...)
			branchRecords = append(branchRecords, observedOwnerRecord{observation: observation, symbol: symbols[0]})
			nextDevice, nextModel, accepted := compareOwnerInvalidation(t, current, state, symbols, observation, symbols[0])
			nextSymbols := append([]uint64(nil), symbols...)
			if accepted {
				nextSymbols[0] = 0
			}
			visit(nextDevice, nextModel, nextSymbols, branchRecords, depth+1)
		}
	}
	visit(device, initial, []uint64{0}, nil, 0)
	if nodes < 80 || invalidations == 0 {
		t.Fatalf("bounded branches did not explore enough paths: nodes=%d invalidations=%d", nodes, invalidations)
	}
	t.Logf("bounded invalidation branches: %d nodes, %d accepted invalidations", nodes, invalidations)
}

func FuzzInvalidateOwnerHistories(f *testing.F) {
	f.Add([]byte{0, 0x11, 0x22, 4, 0, 0, 0, 0, 1, 0x33, 0x44, 3})
	f.Add([]byte{1, 0xff, 0xff, 2, 1, 0, 0, 0})
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 128 {
			t.Skip()
		}
		device, err := NewWithEraseBlocks(2, 6, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 2, 2, 2)
		symbols := []uint64{0, 0}
		var old []observedOwnerRecord
		var symbol uint64
		for offset := 0; offset+3 < len(data); offset += 4 {
			logical := int64(data[offset] % 2)
			if data[offset+3]%3 == 0 {
				observation, present, observeErr := device.ObserveOwner(logical)
				if observeErr != nil {
					t.Fatal(observeErr)
				}
				observedSymbol := uint64(0)
				if present {
					observedSymbol = symbols[logical]
					old = append(old, observedOwnerRecord{observation: observation, symbol: observedSymbol})
				}
				var accepted bool
				device, model, accepted = compareOwnerInvalidation(t, device, model, symbols, observation, observedSymbol)
				if accepted {
					symbols[int(observation.Owner.Logical)] = 0
				}
				continue
			}
			payload := []byte{data[offset+1], data[offset+2]}
			cut := symbolicCut(data[offset+3])
			_, receipt, writeErr := referenceWrite(model, logical, payload, cut)
			device, model = compareTransition(t, device, model, logical, payload, cut)
			if writeErr == "" && receipt.published {
				symbol++
				symbols[int(logical)] = symbol
			}
			old = assertObservationModel(t, device, model, symbols, old)
		}
	})
}

func TestInvalidateOwnerPreservesDetachedMediaAndOtherObservations(t *testing.T) {
	device, err := New(2, 5, 1)
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(0, []byte{0x10}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	device, _, err = device.Write(1, []byte{0x20}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	owner0, _, _ := device.ObserveOwner(0)
	owner1, _, _ := device.ObserveOwner(1)
	before := device.Snapshot()
	invalidated, err := device.InvalidateOwner(owner0)
	if err != nil || invalidated.MatchesOwner(owner1) != true || invalidated.MatchesOwner(owner0) {
		t.Fatalf("owner invalidation=%v", err)
	}
	if invalidated.ownerIdentities[0] != nil || invalidated.ownerIdentities[1] != device.ownerIdentities[1] {
		t.Fatal("invalidation did not clear only the selected owner identity")
	}
	after := invalidated.Snapshot()
	for index := range before.Pages {
		if !reflect.DeepEqual(before.Pages[index], after.Pages[index]) {
			t.Fatalf("physical page %d changed during metadata invalidation", index)
		}
	}
	if after.Map.LogicalToPhysical[0] != -1 || after.Map.PhysicalStatus[owner0.Owner.Physical] != ftlpagemap.Stale || after.Map.LogicalToPhysical[1] != owner1.Owner.Physical {
		t.Fatalf("unexpected map after invalidation: %+v", after.Map)
	}
	before.Map.LogicalToPhysical[1] = -1
	before.Pages[0].Bytes[0] ^= 0xff
	if !device.MatchesOwner(owner0) || !device.MatchesOwner(owner1) {
		t.Fatal("editing a detached snapshot changed predecessor owners")
	}
}

func TestInvalidateOwnerSignedGeometryAndCurrentOwnerPrecedence(t *testing.T) {
	var zero Device
	if got, err := zero.InvalidateOwner(OwnerObservation{Owner: PageOwner{Logical: math.MinInt64, Physical: math.MinInt64}}); !reflect.DeepEqual(got.Snapshot(), zero.Snapshot()) || !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero geometry precedence: %v", err)
	}
	device, err := New(1, 3, 1)
	if err != nil {
		t.Fatal(err)
	}
	for _, observation := range []OwnerObservation{
		{},
		{Owner: PageOwner{Logical: -1, Physical: -1}},
		{Owner: PageOwner{Logical: math.MaxInt64, Physical: math.MaxInt64}},
	} {
		if got, err := device.InvalidateOwner(observation); !reflect.DeepEqual(got.Snapshot(), device.Snapshot()) || err != ErrOwnerObservationStale {
			t.Errorf("unmatched observation %+v: %v", observation.Owner, err)
		}
	}
}

func TestInvalidateOwnerOnAnActiveDevicePreservesEvacuationAndZeroBudget(t *testing.T) {
	device, _, _ := copyTestDevice(t)
	block := int64(0)
	if _, active := device.EvacuationProgress(); !active {
		var err error
		device, err = device.BeginEvacuation(block)
		if err != nil {
			t.Fatal(err)
		}
	}
	observation, present, err := device.ObserveOwner(0)
	if err != nil || !present {
		t.Fatalf("active device owner=%+v/%t/%v", observation.Owner, present, err)
	}
	beforeProgress, beforeActive := device.EvacuationProgress()
	invalidated, err := device.InvalidateOwner(observation)
	if err != nil {
		t.Fatal(err)
	}
	afterProgress, afterActive := invalidated.EvacuationProgress()
	if !beforeActive || !afterActive || afterProgress.Block != beforeProgress.Block || invalidated.MatchesOwner(observation) {
		t.Fatalf("active evacuation metadata changed across invalidation: %+v -> %+v", beforeProgress, afterProgress)
	}
}
