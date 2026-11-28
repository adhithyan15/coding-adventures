package ftlnandrelocation

import (
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftlerasevictim "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-victim"
	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestObserveReclamationValidatesGeometryBeforeBlockProfile(t *testing.T) {
	var zero Device
	if got, err := zero.ObserveReclamation(); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || !reflect.DeepEqual(got, ReclamationObservation{}) {
		t.Fatalf("zero ObserveReclamation=%+v/%v, want zero/invalid geometry", got, err)
	}
	legacy, err := New(1, 4, 4)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := legacy.ObserveReclamation(); !errors.Is(err, ErrEraseDisabled) || !reflect.DeepEqual(got, ReclamationObservation{}) {
		t.Fatalf("legacy ObserveReclamation=%+v/%v, want zero/erase disabled", got, err)
	}
	zeroBudget, err := NewWithEraseBlocks(1, 4, 2, 2, 0)
	if err != nil {
		t.Fatal(err)
	}
	got, err := zeroBudget.ObserveReclamation()
	if err != nil || got.MaxCompletedErases != 0 || got.PagesPerBlock != 2 || len(got.Blocks) != 2 {
		t.Fatalf("zero-budget observation=%+v/%v", got, err)
	}
}

func TestObserveReclamationMatchesRolesAndEveryLifecycleState(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 4, 1, 2, 3)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 4, 1, 2, 3)
	assertObserveReclamationMatchesModel(t, device, model)
	assertObservedRoles(t, device, []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free})

	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x10}, Cut{Kind: nandpageprogramcuts.Complete})
	assertObserveReclamationMatchesModel(t, device, model)
	assertObservedRoles(t, device, []ftlpagemap.PhysicalStatus{ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free})

	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x20}, Cut{Kind: nandpageprogramcuts.Complete})
	assertObserveReclamationMatchesModel(t, device, model)
	assertObservedRoles(t, device, []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free})

	active, model := compareEvacuationBegin(t, device, model, 0)
	activeObservation, err := active.ObserveReclamation()
	if err != nil || !activeObservation.Active || activeObservation.ActiveBlock != 0 {
		t.Fatalf("active block zero lost its explicit active bit: %+v/%v", activeObservation, err)
	}
	assertObserveReclamationMatchesModel(t, active, model)

	// Caller edits to both returned slices must not affect the device or its
	// next scalar observation.
	activeObservation.PhysicalStatus[0] = ftlpagemap.Free
	activeObservation.Blocks[0].IncompleteErase = true
	activeObservation.Blocks[0].CompletedErases = 99
	retained, err := active.ObserveReclamation()
	if err != nil || retained.PhysicalStatus[0] != ftlpagemap.Stale || retained.Blocks[0].IncompleteErase || retained.Blocks[0].CompletedErases != 0 {
		t.Fatalf("mutating returned role/progress slices changed the device: %+v/%v", retained, err)
	}
	assertObserveReclamationMatchesModel(t, active, model)

	canceled, model := compareEvacuationCancel(t, active, model)
	assertObserveReclamationMatchesModel(t, canceled, model)
	if got, err := canceled.ObserveReclamation(); err != nil || got.Active || got.ActiveBlock != 0 {
		t.Fatalf("canceled target leaked active ordinal: %+v/%v", got, err)
	}

	active, model = compareEvacuationBegin(t, canceled, model, 0)
	active, model = compareEvacuationCopy(t, active, model, Cut{Kind: nandpageprogramcuts.Complete})
	assertObserveReclamationMatchesModel(t, active, model)
	partial, model := compareFinishEvacuation(t, active, model, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	partialObservation, err := partial.ObserveReclamation()
	if err != nil || !partialObservation.Active || partialObservation.ActiveBlock != 0 || !partialObservation.Blocks[0].IncompleteErase {
		t.Fatalf("active incomplete erase was not captured in both fields: %+v/%v", partialObservation, err)
	}
	assertObserveReclamationMatchesModel(t, partial, model)

	resumed, model := compareFinishEvacuation(t, partial, model, EraseCut{Kind: nandblockerasecuts.AfterMedia})
	completedObservation, err := resumed.ObserveReclamation()
	if err != nil || completedObservation.Active || completedObservation.ActiveBlock != 0 || completedObservation.Blocks[0].IncompleteErase || completedObservation.Blocks[0].CompletedErases != 1 {
		t.Fatalf("lost acknowledgement media completion=%+v/%v", completedObservation, err)
	}
	assertObserveReclamationMatchesModel(t, resumed, model)
}

func TestObserveReclamationReportsNonzeroActiveOrdinalAndRetainsOldValue(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 6, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 6, 1, 2, 2)
	for value := byte(1); value <= 4; value++ {
		device, model = compareEvacuationWrite(t, device, model, 0, []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
	}
	active, model := compareEvacuationBegin(t, device, model, 1)
	retained, err := active.ObserveReclamation()
	if err != nil || !retained.Active || retained.ActiveBlock != 1 {
		t.Fatalf("nonzero active ordinal=%+v/%v", retained, err)
	}
	assertObserveReclamationMatchesModel(t, active, model)

	canceled, model := compareEvacuationCancel(t, active, model)
	if !retained.Active || retained.ActiveBlock != 1 {
		t.Fatal("later lifecycle call modified a retained observation value")
	}
	assertObserveReclamationMatchesModel(t, canceled, model)
}

func TestObserveReclamationMaximumGeometryStaysCompact(t *testing.T) {
	device, err := NewWithEraseBlocks(64, 128, maxPageSize, 2, maxSafeEraseCount)
	if err != nil {
		t.Fatal(err)
	}
	got, err := device.ObserveReclamation()
	if err != nil || len(got.Blocks) != 64 || len(got.PhysicalStatus) != 128 || got.PagesPerBlock != 2 || got.MaxCompletedErases != maxSafeEraseCount {
		t.Fatalf("maximum geometry observation sizes blocks=%d statuses=%d metadata=%+v err=%v", len(got.Blocks), len(got.PhysicalStatus), got, err)
	}

	// A full media copy would allocate at least the 8 MiB payload extent. This
	// scalar capture must remain well below that even at the geometry ceiling.
	benchmark := testing.Benchmark(func(b *testing.B) {
		b.ReportAllocs()
		for range b.N {
			if _, callErr := device.ObserveReclamation(); callErr != nil {
				b.Fatal(callErr)
			}
		}
	})
	if benchmark.AllocedBytesPerOp() >= int64(maxPageSize*len(device.pages)) {
		t.Fatalf("compact observation allocated %d bytes/op, reaching page payload size %d", benchmark.AllocedBytesPerOp(), maxPageSize*len(device.pages))
	}
}

func TestObservedRolesFeedSelectionButDoNotReplaceProtectedExecution(t *testing.T) {
	base, err := NewWithEraseBlocks(2, 6, 2, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	metadataPageFence := []bool{false, false, true, true, false, false}
	metadataBlockFence := []bool{false, true, false}
	for _, write := range []struct {
		logical int64
		data    []byte
	}{{0, []byte{0x10, 0x11}}, {1, []byte{0x20, 0x21}}, {0, []byte{0x30, 0x31}}} {
		var receipt Receipt
		base, receipt, err = base.WriteExcluding(write.logical, write.data, Cut{Kind: nandpageprogramcuts.Complete}, metadataPageFence)
		if err != nil || !receipt.Published || !receipt.Acknowledged {
			t.Fatalf("protected setup write logical=%d: %+v/%v", write.logical, receipt, err)
		}
	}

	observation, err := base.ObserveReclamation()
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(observation.PhysicalStatus, []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free,
		ftlpagemap.Free, ftlpagemap.Live, ftlpagemap.Free,
	}) {
		t.Fatalf("observed protected roles=%v", observation.PhysicalStatus)
	}
	victim, found, err := ftlerasevictim.SelectCopyAware(observation.PhysicalStatus, observation.PagesPerBlock, metadataBlockFence)
	if err != nil || !found || victim.Block != 0 || victim.StalePages != 1 || victim.LivePages != 1 || victim.FreeDestinations != 1 {
		t.Fatalf("observation-driven victim=%+v/%t/%v", victim, found, err)
	}
	protectedPlan, err := ftlevacuationplan.PlanEvacuation(base.table, observation.PagesPerBlock, victim.Block, metadataBlockFence)
	if err != nil || len(protectedPlan.Copies) != 1 || protectedPlan.Copies[0] != (ftlevacuationplan.Copy{
		Logical: 1, SourcePhysical: 1, DestinationPhysical: 5,
	}) {
		t.Fatalf("metadata-fenced plan=%+v/%v", protectedPlan, err)
	}
	protected, receipt, err := base.CopyLivePage(victim.Block, protectedPlan.Copies[0], Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || receipt.Physical != 5 || !receipt.Published || !receipt.Acknowledged {
		t.Fatalf("protected execution=%+v/%v", receipt, err)
	}
	if physical, present, lookupErr := protected.Lookup(1); lookupErr != nil || !present || physical != 5 {
		t.Fatalf("protected owner published at %d/%t/%v, want page five", physical, present, lookupErr)
	}
	protected, err = protected.BeginEvacuation(victim.Block)
	if err != nil {
		t.Fatal(err)
	}
	protected, eraseReceipt, err := protected.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil || !eraseReceipt.Released || !eraseReceipt.Acknowledged {
		t.Fatalf("protected erase=%+v/%v", eraseReceipt, err)
	}

	// The raw lifecycle API has no metadata mask. Replanning without the fence
	// picks page two, and the raw copy accepts that move; callers must carry the
	// same policy through both selection/planning and execution.
	rawPlan, err := ftlevacuationplan.PlanEvacuation(base.table, 2, victim.Block, nil)
	if err != nil || len(rawPlan.Copies) != 1 || rawPlan.Copies[0].DestinationPhysical != 2 {
		t.Fatalf("unfenced raw plan=%+v/%v", rawPlan, err)
	}
	raw, err := base.BeginEvacuation(victim.Block)
	if err != nil {
		t.Fatal(err)
	}
	raw, evacuationReceipt, err := raw.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || evacuationReceipt.Move != rawPlan.Copies[0] || evacuationReceipt.Receipt.Physical != 2 || !evacuationReceipt.Receipt.Published || !evacuationReceipt.Receipt.Acknowledged {
		t.Fatalf("unfenced raw execution=%+v/%v", evacuationReceipt, err)
	}
	if physical, present, lookupErr := raw.Lookup(1); lookupErr != nil || !present || physical != 2 {
		t.Fatalf("raw owner at %d/%t/%v, want reserved page two", physical, present, lookupErr)
	}
}

func TestSeededObserveReclamationHistoriesMatchTheIndependentModel(t *testing.T) {
	for seed := int64(0); seed < 24; seed++ {
		rng := rand.New(rand.NewSource(seed))
		device, err := NewWithEraseBlocks(2, 8, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 8, 2, 2, 2)
		for step := 0; step < 96; step++ {
			switch rng.Intn(6) {
			case 0, 1:
				logical := int64(rng.Intn(2))
				payload := []byte{byte(rng.Intn(256)), byte(rng.Intn(256))}
				cut := Cut{Kind: nandpageprogramcuts.CutKind(rng.Intn(4))}
				if cut.Kind == nandpageprogramcuts.AfterPrefix {
					cut.PrefixBytes = rng.Intn(3)
				}
				device, model = compareEvacuationWrite(t, device, model, logical, payload, cut)
			case 2:
				block := int64(rng.Intn(4))
				cut := EraseCut{Kind: nandblockerasecuts.CutKind(rng.Intn(4))}
				if cut.Kind == nandblockerasecuts.AfterPrefix {
					cut.PrefixPages = rng.Intn(3)
				}
				device, model = compareEraseBlock(t, device, model, block, cut)
			case 3:
				device, model = compareEvacuationBegin(t, device, model, int64(rng.Intn(4)))
			case 4:
				cut := Cut{Kind: nandpageprogramcuts.CutKind(rng.Intn(4))}
				if cut.Kind == nandpageprogramcuts.AfterPrefix {
					cut.PrefixBytes = rng.Intn(3)
				}
				device, model = compareEvacuationCopy(t, device, model, cut)
			case 5:
				if rng.Intn(2) == 0 {
					device, model = compareEvacuationCancel(t, device, model)
				} else {
					cut := EraseCut{Kind: nandblockerasecuts.CutKind(rng.Intn(4))}
					if cut.Kind == nandblockerasecuts.AfterPrefix {
						cut.PrefixPages = rng.Intn(3)
					}
					device, model = compareFinishEvacuation(t, device, model, cut)
				}
			}
			assertObserveReclamationMatchesModel(t, device, model)
		}
	}
}

func FuzzObserveReclamationHistories(f *testing.F) {
	f.Add([]byte{0, 0, 0, 0, 0, 0})
	f.Add([]byte{0xff, 0x80, 0x20, 0x10, 0x09})
	f.Add([]byte{0, 0, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 512 {
			commands = commands[:512]
		}
		device, err := NewWithEraseBlocks(2, 8, 2, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 8, 2, 2, 2)
		for _, command := range commands {
			switch command % 6 {
			case 0:
				cut := Cut{Kind: nandpageprogramcuts.CutKind(command >> 2 % 4)}
				if cut.Kind == nandpageprogramcuts.AfterPrefix {
					cut.PrefixBytes = int(command>>4) % 3
				}
				device, model = compareEvacuationWrite(t, device, model, int64((command>>1)%2), []byte{command, command ^ 0xff}, cut)
			case 1:
				cut := EraseCut{Kind: nandblockerasecuts.CutKind(command >> 2 % 4)}
				if cut.Kind == nandblockerasecuts.AfterPrefix {
					cut.PrefixPages = int(command>>4) % 3
				}
				device, model = compareEraseBlock(t, device, model, int64((command>>1)%4), cut)
			case 2:
				device, model = compareEvacuationBegin(t, device, model, int64((command>>1)%4))
			case 3:
				cut := Cut{Kind: nandpageprogramcuts.CutKind(command >> 2 % 4)}
				if cut.Kind == nandpageprogramcuts.AfterPrefix {
					cut.PrefixBytes = int(command>>4) % 3
				}
				device, model = compareEvacuationCopy(t, device, model, cut)
			case 4:
				cut := EraseCut{Kind: nandblockerasecuts.CutKind(command >> 2 % 4)}
				if cut.Kind == nandblockerasecuts.AfterPrefix {
					cut.PrefixPages = int(command>>4) % 3
				}
				device, model = compareFinishEvacuation(t, device, model, cut)
			case 5:
				device, model = compareEvacuationCancel(t, device, model)
			}
			assertObserveReclamationMatchesModel(t, device, model)
		}
	})
}

func assertObserveReclamationMatchesModel(t *testing.T, device Device, model referenceState) {
	t.Helper()
	got, err := device.ObserveReclamation()
	if err != nil {
		t.Fatal(err)
	}
	want := ReclamationObservation{
		Blocks:             append([]BlockSnapshot(nil), model.blocks...),
		PhysicalStatus:     make([]ftlpagemap.PhysicalStatus, len(model.status)),
		PagesPerBlock:      model.pagesPerBlock,
		MaxCompletedErases: model.maxCompletedErases,
	}
	for index, slot := range model.status {
		switch slot {
		case modelFree:
			want.PhysicalStatus[index] = ftlpagemap.Free
		case modelLive:
			want.PhysicalStatus[index] = ftlpagemap.Live
		case modelStale:
			want.PhysicalStatus[index] = ftlpagemap.Stale
		}
	}
	if model.evacuationActive {
		want.Active = true
		want.ActiveBlock = model.evacuationBlock
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("ObserveReclamation()=%+v, want reference model %+v", got, want)
	}

	// Compare the new compact result field-by-field with the older public views.
	oldMap := device.Snapshot().Map
	if !reflect.DeepEqual(got.PhysicalStatus, oldMap.PhysicalStatus) {
		t.Fatalf("compact physical roles=%v, old map roles=%v", got.PhysicalStatus, oldMap.PhysicalStatus)
	}
	progress := device.ReclamationSnapshot()
	if got.PagesPerBlock != progress.PagesPerBlock || got.MaxCompletedErases != progress.MaxCompletedErases || !reflect.DeepEqual(got.Blocks, progress.Blocks) {
		t.Fatalf("compact erase progress=%+v differs from old view %+v", got, progress)
	}
	active, isActive := device.EvacuationProgress()
	if got.Active != isActive || got.ActiveBlock != active.Block {
		t.Fatalf("compact active=%t/%d differs from old view %+v/%t", got.Active, got.ActiveBlock, active, isActive)
	}
}

func assertObservedRoles(t *testing.T, device Device, want []ftlpagemap.PhysicalStatus) {
	t.Helper()
	got, err := device.ObserveReclamation()
	if err != nil || !reflect.DeepEqual(got.PhysicalStatus, want) {
		t.Fatalf("physical role vector=%v/%v, want %v", got.PhysicalStatus, err, want)
	}
}
