package ftlnandrelocation

import (
	"bytes"
	"errors"
	"reflect"
	"testing"

	ftlerasevictim "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-erase-victim"
	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestCopyAwareVictimComposesWithPlannerAndController(t *testing.T) {
	base, model := copyAwareComparisonWorkload(t)
	initialSnapshot := base.Snapshot()
	initialFree := countFree(initialSnapshot.Map.PhysicalStatus)
	logicalBytes, present, err := base.Read(0)
	if err != nil || !present {
		t.Fatalf("initial live read: %x/%t/%v", logicalBytes, present, err)
	}

	// Both choices come from the same reachable workload: the stale-only policy
	// reclaims a one-slot dead block while copy-aware policy moves one live page
	// to reclaim three stale slots in the mixed block.
	staleVictim, staleFound, staleErr := ftlerasevictim.Select(initialSnapshot.Map.PhysicalStatus, 4)
	if staleErr != nil || !staleFound || staleVictim.Block != 1 || staleVictim.StalePages != 1 {
		t.Fatalf("stale-only workload selection=%+v/%t/%v", staleVictim, staleFound, staleErr)
	}
	copyVictim, copyFound, copyErr := ftlerasevictim.SelectCopyAware(initialSnapshot.Map.PhysicalStatus, 4, nil)
	if copyErr != nil || !copyFound || copyVictim != (ftlerasevictim.CopyAwareVictim{
		Block: 0, FirstPhysical: 0, PageCount: 4, StalePages: 3, LivePages: 1, FreeDestinations: 7,
	}) {
		t.Fatalf("copy-aware workload selection=%+v/%t/%v", copyVictim, copyFound, copyErr)
	}
	plan, err := ftlevacuationplan.PlanEvacuation(base.table, 4, copyVictim.Block, nil)
	if err != nil || len(plan.Copies) != 1 || plan.Copies[0] != (ftlevacuationplan.Copy{Logical: 0, SourcePhysical: 3, DestinationPhysical: 5}) {
		t.Fatalf("copy-aware plan=%+v/%v", plan, err)
	}

	staleDevice, staleModel := compareEvacuationBegin(t, base, cloneReference(model), staleVictim.Block)
	if progress, active := staleDevice.EvacuationProgress(); !active || progress.RemainingLivePages != 0 {
		t.Fatalf("stale-only block should need no live copies: %+v/%t", progress, active)
	}
	staleDevice, staleReceipt, staleErr := staleDevice.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.Complete})
	staleModel, wantStaleReceipt, staleCode := referenceFinishEvacuation(staleModel, nandblockerasecuts.Cut{Kind: nandblockerasecuts.Complete})
	assertError(t, staleErr, staleCode)
	if staleErr != nil || staleReceipt != wantStaleReceipt || !staleReceipt.Released {
		t.Fatalf("stale-only reclamation=%+v/%v", staleReceipt, staleErr)
	}
	assertEvacuationState(t, staleDevice, staleModel)
	staleGain := countFree(staleDevice.Snapshot().Map.PhysicalStatus) - initialFree
	if staleGain != 1 {
		t.Fatalf("stale-only net free slots=%d, want one", staleGain)
	}

	device, model := compareEvacuationBegin(t, base, cloneReference(model), copyVictim.Block)
	device, model = compareEvacuationCopy(t, device, model, nandpageprogramcuts.Cut{Kind: nandpageprogramcuts.Complete})
	device, receipt, err := device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.Complete})
	wantModel, wantReceipt, wantCode := referenceFinishEvacuation(model, nandblockerasecuts.Cut{Kind: nandblockerasecuts.Complete})
	assertError(t, err, wantCode)
	if err != nil || receipt != wantReceipt || !receipt.Released || !receipt.Acknowledged {
		t.Fatalf("copy-aware completion=%+v/%v, want %+v", receipt, err, wantReceipt)
	}
	assertEvacuationState(t, device, wantModel)
	finalSnapshot := device.Snapshot()
	for name, result := range map[string]Device{"initial": base, "stale-only": staleDevice, "copy-aware": device} {
		data, present, readErr := result.Read(0)
		if readErr != nil || !present || !bytes.Equal(logicalBytes, data) {
			t.Fatalf("%s policy changed logical bytes: %x/%t/%v want %x", name, data, present, readErr, logicalBytes)
		}
	}
	for physical := 0; physical < 4; physical++ {
		if finalSnapshot.Map.PhysicalStatus[physical] != ftlpagemap.Free {
			t.Fatalf("selected copy-aware block did not release slot %d", physical)
		}
	}
	copyGain := countFree(finalSnapshot.Map.PhysicalStatus) - initialFree
	if copyGain != 3 {
		t.Fatalf("copy-aware net free slots=%d, want three", copyGain)
	}
	if !reflect.DeepEqual(base.Snapshot(), initialSnapshot) {
		t.Fatal("both policy transitions must preserve the common predecessor")
	}
	completedCopies := len(plan.Copies)
	t.Logf("common policy workload: stale-only copies=0 spent=0 refused=0 recovered=%d; copy-aware copies=%d spent=0 refused=0 recovered=%d", staleGain, completedCopies, copyGain)

	t.Run("incomplete extent is excluded and spent destination replans", func(t *testing.T) {
		base, state := copyAwareIncompleteWorkload(t)
		old, oldFound, oldErr := ftlerasevictim.Select(base.Snapshot().Map.PhysicalStatus, 4)
		if oldErr != nil || !oldFound || old.Block != 0 || old.StalePages != 4 {
			t.Fatalf("legacy selector sees incomplete extent=%+v/%t/%v", old, oldFound, oldErr)
		}
		if _, err := base.BeginEvacuation(old.Block); !errors.Is(err, ErrEvacuationBlockIncomplete) {
			t.Fatalf("controller must refuse the incomplete legacy choice: %v", err)
		}
		mask := []bool{true, false, false, false}
		selected, found, selectErr := ftlerasevictim.SelectCopyAware(base.Snapshot().Map.PhysicalStatus, 4, mask)
		if selectErr != nil || !found || selected.Block != 1 {
			t.Fatalf("selection around incomplete block=%+v/%t/%v", selected, found, selectErr)
		}
		firstPlan, planErr := ftlevacuationplan.PlanEvacuation(base.table, 4, selected.Block, mask)
		if planErr != nil || len(firstPlan.Copies) != 1 {
			t.Fatalf("initial plan=%+v/%v", firstPlan, planErr)
		}
		device, state := compareCopyLive(t, base, state, selected.Block, firstPlan.Copies[0], Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
		modelBeforeRetry := cloneReference(state)
		beforeRetry := device.Snapshot()
		refused, copyReceipt, copyErr := device.CopyLivePage(selected.Block, firstPlan.Copies[0], Cut{Kind: nandpageprogramcuts.Complete})
		if !errors.Is(copyErr, ErrCopyDestinationUnavailable) || copyReceipt != (Receipt{}) {
			t.Fatalf("spent destination retry=%+v/%v", copyReceipt, copyErr)
		}
		assertDeviceEquals(t, refused, modelBeforeRetry)
		active, state := compareEvacuationBegin(t, device, state, selected.Block)
		active, state = compareEvacuationCopy(t, active, state, Cut{Kind: nandpageprogramcuts.Complete})
		active, eraseReceipt, eraseErr := active.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.Complete})
		wantState, wantErase, wantCode := referenceFinishEvacuation(state, nandblockerasecuts.Cut{Kind: nandblockerasecuts.Complete})
		assertError(t, eraseErr, wantCode)
		if eraseErr != nil || eraseReceipt != wantErase || !eraseReceipt.Released {
			t.Fatalf("post-replan erase=%+v/%v", eraseReceipt, eraseErr)
		}
		assertEvacuationState(t, active, wantState)
		if !beforeRetry.Pages[int(firstPlan.Copies[0].DestinationPhysical)].Programmed ||
			active.Snapshot().Map.PhysicalStatus[int(firstPlan.Copies[0].DestinationPhysical)] != ftlpagemap.Stale {
			t.Fatal("spent destination did not remain spent after successful replan")
		}
		t.Log("interrupted copy policy: copies completed=1, spent destinations=1, refused retries=1, then replanned and released block")
	})
}

func copyAwareComparisonWorkload(t *testing.T) (Device, referenceState) {
	t.Helper()
	device, err := NewWithEraseBlocks(2, 12, 2, 4, 1)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(2, 12, 2, 4, 1)
	for value := byte(0); value < 4; value++ {
		device, model = compareTransition(t, device, model, 0, []byte{value, 0x80}, modelCut{kind: cutComplete})
	}
	device, model = compareTransition(t, device, model, 1, []byte{7, 6}, modelCut{kind: cutAfterPrefix, prefix: 0})
	return device, model
}

func copyAwareIncompleteWorkload(t *testing.T) (Device, referenceState) {
	t.Helper()
	device, err := NewWithEraseBlocks(4, 16, 2, 4, 3)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(4, 16, 2, 4, 3)
	for logical := int64(0); logical < 4; logical++ {
		payload := []byte{byte(0x10 + logical), byte(0x20 + logical)}
		device, model = compareTransition(t, device, model, logical, payload, modelCut{kind: cutComplete})
	}
	for logical := int64(0); logical < 4; logical++ {
		payload := []byte{byte(0x30 + logical), byte(0x40 + logical)}
		device, model = compareTransition(t, device, model, logical, payload, modelCut{kind: cutComplete})
	}
	for logical := int64(0); logical < 3; logical++ {
		payload := []byte{byte(0x50 + logical), byte(0x60 + logical)}
		device, model = compareTransition(t, device, model, logical, payload, modelCut{kind: cutComplete})
	}
	device, model = compareEraseBlock(t, device, model, 0, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 3})
	return device, model
}

func countFree(status []ftlpagemap.PhysicalStatus) int {
	count := 0
	for _, item := range status {
		if item == ftlpagemap.Free {
			count++
		}
	}
	return count
}
