package ftlevacuationplan

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

const drainFixtureSchema = "coding-adventures.database-media.ftl-retirement-drain-plan.v1"

func TestCanonicalDrainFixtureModelFirstThenRuntime(t *testing.T) {
	raw := readDrainFixture(t)
	document, err := parseFixtureSchema(raw, drainFixtureSchema)
	if err != nil {
		t.Fatalf("parse canonical drain fixture: %v", err)
	}
	if err := validateDrainFixtureModel(document); err != nil {
		t.Fatalf("independent model rejected drain fixture: %v", err)
	}
	for _, testCase := range document.Cases {
		table := tableFromFixture(t, testCase)
		before := table.Snapshot()
		plan, err := PlanDrain(table, *testCase.PagesPerBlock, *testCase.Block, *testCase.ExcludedBlocks)
		if errorName(err) != fixtureError(testCase.Expected.Error) {
			t.Fatalf("%s: error=%s want=%s", testCase.Name, errorName(err), fixtureError(testCase.Expected.Error))
		}
		if err == nil && !reflect.DeepEqual(plan, runtimePlan(testCase.Expected.Plan)) {
			t.Fatalf("%s: plan=%+v want=%+v", testCase.Name, plan, runtimePlan(testCase.Expected.Plan))
		}
		if !reflect.DeepEqual(table.Snapshot(), before) {
			t.Fatalf("%s: drain planning mutated its input map", testCase.Name)
		}
	}
}

func TestDrainAdapterRejectsMalformedAndWrongDeclarations(t *testing.T) {
	raw := readDrainFixture(t)
	variants := map[string][]byte{
		"wrong schema":     bytes.Replace(raw, []byte(drainFixtureSchema), []byte(fixtureSchema), 1),
		"boolean target":   bytes.Replace(raw, []byte(`"block": 0`), []byte(`"block": false`), 1),
		"duplicate schema": bytes.Replace(raw, []byte(`"schema":`), []byte(`"schema":"bad","schema":`), 1),
		"unknown root key": bytes.Replace(raw, []byte(`"schema":`), []byte(`"extra":true,"schema":`), 1),
		"trailing value":   append(append([]byte(nil), raw...), []byte(`{}`)...),
		"oversized":        bytes.Repeat([]byte{' '}, fixtureLimit+1),
	}
	for name, variant := range variants {
		t.Run(name, func(t *testing.T) {
			if bytes.Equal(raw, variant) {
				t.Fatal("mutation anchor was absent")
			}
			if _, err := parseFixtureSchema(variant, drainFixtureSchema); err == nil {
				t.Fatal("malformed drain fixture accepted")
			}
		})
	}
	wrong := bytes.Replace(raw, []byte(`"destination_physical": 13`), []byte(`"destination_physical": 14`), 1)
	if bytes.Equal(raw, wrong) {
		t.Fatal("wrong-result mutation anchor was absent")
	}
	document, err := parseFixtureSchema(wrong, drainFixtureSchema)
	if err != nil {
		t.Fatalf("wrong declaration must remain well formed: %v", err)
	}
	if err := validateDrainFixtureModel(document); err == nil {
		t.Fatal("independent model accepted a well-typed wrong drain destination")
	}
}

func TestDrainValidationOrderAndZeroPlans(t *testing.T) {
	var zero ftlpagemap.Table
	if got, err := PlanDrain(zero, 0, -1, []bool{true}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || !reflect.DeepEqual(got, Plan{}) {
		t.Fatalf("unconfigured precedence: %+v %v", got, err)
	}
	fresh := mustTable(t, 1, 4)
	if _, err := PlanDrain(fresh, 3, -1, nil); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("invalid width must precede target: %v", err)
	}
	if _, err := PlanDrain(fresh, 2, -1, []bool{true}); !errors.Is(err, ErrInvalidBlockIndex) {
		t.Fatalf("invalid target must precede mask: %v", err)
	}
	for _, block := range []int64{-1 << 63, 1<<63 - 1} {
		if plan, err := PlanDrain(fresh, 2, block, nil); !errors.Is(err, ErrInvalidBlockIndex) || !reflect.DeepEqual(plan, Plan{}) {
			t.Fatalf("signed target %d: plan=%+v err=%v", block, plan, err)
		}
	}
	if _, err := PlanDrain(fresh, 2, 0, []bool{true}); !errors.Is(err, ErrInvalidBlockMask) {
		t.Fatalf("invalid mask precedes pending reservation: %v", err)
	}
	pending, _, err := fresh.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := PlanDrain(pending, 2, 0, []bool{true, false}); !errors.Is(err, ftlpagemap.ErrReservationPending) {
		t.Fatalf("pending reservation must follow mask validation: %v", err)
	}
	got, err := PlanDrain(fresh, 2, 0, []bool{true, false})
	if err != nil || !reflect.DeepEqual(got, Plan{Block: 0, FirstPhysical: 0, PageCount: 2, Copies: []Copy{}}) {
		t.Fatalf("empty fenced target = %+v, %v", got, err)
	}
	if _, err := PlanEvacuation(fresh, 2, 0, []bool{true, false}); !errors.Is(err, ErrExcludedBlock) {
		t.Fatalf("legacy target fence behavior changed: %v", err)
	}
}

func TestDrainCapacityRefusalIsAtomic(t *testing.T) {
	table := tableWithPrograms(t, 3, 9, 0, 1, 2)
	plan, err := PlanDrain(table, 3, 0, []bool{false, true, true})
	if !errors.Is(err, ErrInsufficientCapacity) || !reflect.DeepEqual(plan, Plan{}) {
		t.Fatalf("short reserve returned partial plan %+v / %v", plan, err)
	}
}

func TestEveryDrainRefusalReturnsZeroPlan(t *testing.T) {
	fresh := mustTable(t, 1, 4)
	pending, _, err := fresh.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	short := tableWithPrograms(t, 3, 9, 0, 1, 2)
	cases := []struct {
		name       string
		table      ftlpagemap.Table
		ppb, block int64
		mask       []bool
		want       error
	}{
		{name: "invalid geometry", want: ftlpagemap.ErrInvalidGeometry},
		{name: "invalid target", table: fresh, ppb: 2, block: -1, want: ErrInvalidBlockIndex},
		{name: "invalid mask", table: fresh, ppb: 2, mask: []bool{false}, want: ErrInvalidBlockMask},
		{name: "pending reservation", table: pending, ppb: 2, mask: []bool{false, false}, want: ftlpagemap.ErrReservationPending},
		{name: "insufficient reserve", table: short, ppb: 3, mask: []bool{false, true, true}, want: ErrInsufficientCapacity},
	}
	for _, item := range cases {
		t.Run(item.name, func(t *testing.T) {
			plan, err := PlanDrain(item.table, item.ppb, item.block, item.mask)
			if !errors.Is(err, item.want) || !reflect.DeepEqual(plan, Plan{}) {
				t.Fatalf("refusal returned plan=%+v err=%v, want zero plan and %v", plan, err, item.want)
			}
		})
	}
}

func TestDrainOrderedCopiesAndLegacySharedResult(t *testing.T) {
	allLive := tableWithPrograms(t, 3, 9, 0, 1, 2)
	plan, err := PlanDrain(allLive, 3, 0, nil)
	want := []Copy{{Logical: 0, SourcePhysical: 0, DestinationPhysical: 3}, {Logical: 1, SourcePhysical: 1, DestinationPhysical: 4}, {Logical: 2, SourcePhysical: 2, DestinationPhysical: 5}}
	if err != nil || !reflect.DeepEqual(plan.Copies, want) {
		t.Fatalf("all-live source order: %+v %v", plan, err)
	}
	plan.Copies[0].DestinationPhysical = 99
	again, err := PlanDrain(allLive, 3, 0, nil)
	if err != nil || !reflect.DeepEqual(again.Copies, want) {
		t.Fatalf("drain results are not detached: %+v %v", again, err)
	}

	legacy := tableWithPrograms(t, 4, 9, 0, 1, 2, 1)
	mask := []bool{false, false, true}
	oldPlan, oldErr := PlanEvacuation(legacy, 3, 0, mask)
	drainPlan, drainErr := PlanDrain(legacy, 3, 0, mask)
	if oldErr != nil || drainErr != nil || !reflect.DeepEqual(drainPlan, oldPlan) {
		t.Fatalf("shared eligible source changed legacy result: drain=%+v/%v evacuation=%+v/%v", drainPlan, drainErr, oldPlan, oldErr)
	}
}

func TestAllReachableSmallMapsMasksAndLayoutsCompareDrain(t *testing.T) {
	geometries := []struct{ logical, physical int64 }{{1, 2}, {1, 3}, {1, 4}, {2, 4}, {2, 6}}
	comparisons := 0
	for _, geometry := range geometries {
		for _, table := range reachableTables(t, geometry.logical, geometry.physical) {
			for ppb := int64(1); ppb <= geometry.physical; ppb++ {
				if geometry.physical%ppb != 0 {
					continue
				}
				blockCount := geometry.physical / ppb
				for bits := 0; bits < 1<<blockCount; bits++ {
					mask := make([]bool, int(blockCount))
					for bit := range mask {
						mask[bit] = bits&(1<<bit) != 0
					}
					for block := int64(0); block < blockCount; block++ {
						before := table.Snapshot()
						want, wantErr := modelDrainPlanFor(before, ppb, block, mask)
						got, gotErr := PlanDrain(table, ppb, block, mask)
						if errorName(gotErr) != wantErr || (wantErr == "" && !reflect.DeepEqual(got, planFromModel(want))) {
							t.Fatalf("geometry %d/%d ppb=%d mask=%v block=%d got=%+v/%s want=%+v/%s", geometry.logical, geometry.physical, ppb, mask, block, got, errorName(gotErr), want, wantErr)
						}
						if !reflect.DeepEqual(table.Snapshot(), before) {
							t.Fatal("drain planning mutated a reachable predecessor")
						}
						comparisons++
					}
				}
			}
		}
	}
	t.Logf("exhaustively compared %d drain map/layout/mask/target combinations", comparisons)
}

func FuzzParseDrainFixture(f *testing.F) {
	f.Add(readDrainFixtureBytes(f))
	f.Add([]byte(`{"schema":"invalid","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		document, err := parseFixtureSchema(raw, drainFixtureSchema)
		if err != nil {
			return
		}
		if err := validateDrainFixtureModel(document); err != nil {
			return
		}
	})
}

func readDrainFixture(t *testing.T) []byte {
	t.Helper()
	return readDrainFixtureBytes(t)
}

func readDrainFixtureBytes(t testing.TB) []byte {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-retirement-drain-plan.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func validateDrainFixtureModel(document fixtureDocument) error {
	for _, testCase := range document.Cases {
		state := modelForFixture(testCase)
		wantPlan, wantErr := modelDrainPlanFor(state.snapshot(), *testCase.PagesPerBlock, *testCase.Block, *testCase.ExcludedBlocks)
		if fixtureError(testCase.Expected.Error) != wantErr {
			return fmt.Errorf("case %s error=%s, drain model predicts %s", testCase.Name, fixtureError(testCase.Expected.Error), wantErr)
		}
		if wantErr == "" && !reflect.DeepEqual(*testCase.Expected.Plan, fixturePlanFromModel(wantPlan)) {
			return fmt.Errorf("case %s plan differs from independent drain model", testCase.Name)
		}
		if !reflect.DeepEqual(*testCase.Expected.State, fixtureStateOf(state)) {
			return fmt.Errorf("case %s map state differs from independent drain model", testCase.Name)
		}
	}
	return nil
}
