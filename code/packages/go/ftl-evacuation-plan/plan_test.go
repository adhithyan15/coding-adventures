package ftlevacuationplan

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"unicode/utf8"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

const (
	fixtureSchema = "coding-adventures.database-media.ftl-evacuation-plan.v1"
	fixtureLimit  = 64 * 1024
	fixtureDepth  = 32
)

type fixtureDocument struct {
	Schema string        `json:"schema"`
	Cases  []fixtureCase `json:"cases"`
}
type fixtureCase struct {
	Name            string           `json:"name"`
	LogicalCount    *int64           `json:"logical_count"`
	PhysicalCount   *int64           `json:"physical_count"`
	PagesPerBlock   *int64           `json:"pages_per_block"`
	ProgramLogicals *[]int64         `json:"program_logicals"`
	Block           *int64           `json:"block"`
	ExcludedBlocks  *[]bool          `json:"excluded_blocks"`
	Expected        *fixtureExpected `json:"expected"`
}
type fixtureExpected struct {
	Error *string       `json:"error,omitempty"`
	Plan  *fixturePlan  `json:"plan,omitempty"`
	State *fixtureState `json:"state"`
}
type fixturePlan struct {
	Block         *int64         `json:"block"`
	FirstPhysical *int64         `json:"first_physical"`
	PageCount     *int64         `json:"page_count"`
	Copies        *[]fixtureCopy `json:"copies"`
}
type fixtureCopy struct {
	Logical             *int64 `json:"logical"`
	SourcePhysical      *int64 `json:"source_physical"`
	DestinationPhysical *int64 `json:"destination_physical"`
}
type fixtureState struct {
	LogicalToPhysical *[]int64  `json:"logical_to_physical"`
	PhysicalStatus    *[]string `json:"physical_status"`
}
type model struct {
	logicalCount  int64
	physicalCount int64
	mapping       []int64
	status        []ftlpagemap.PhysicalStatus
}
type modelPlan struct {
	block, first, count int64
	copies              []Copy
}

func TestCanonicalEvacuationFixtureModelFirstThenRuntime(t *testing.T) {
	raw := readFixture(t)
	document, err := parseFixture(raw)
	if err != nil {
		t.Fatalf("parse canonical fixture: %v", err)
	}
	if err := validateFixtureModel(document); err != nil {
		t.Fatalf("independent model rejected fixture: %v", err)
	}
	for _, testCase := range document.Cases {
		table := tableFromFixture(t, testCase)
		before := table.Snapshot()
		plan, err := PlanEvacuation(table, *testCase.PagesPerBlock, *testCase.Block, *testCase.ExcludedBlocks)
		if errorName(err) != fixtureError(testCase.Expected.Error) {
			t.Fatalf("%s: error=%s want=%s", testCase.Name, errorName(err), fixtureError(testCase.Expected.Error))
		}
		if err == nil && !reflect.DeepEqual(plan, runtimePlan(testCase.Expected.Plan)) {
			t.Fatalf("%s: plan=%+v want=%+v", testCase.Name, plan, runtimePlan(testCase.Expected.Plan))
		}
		if !reflect.DeepEqual(table.Snapshot(), before) {
			t.Fatalf("%s: planning mutated its input table", testCase.Name)
		}
	}
}

func TestFixtureAdapterRejectsKindsDuplicatesAndWrongDeclaration(t *testing.T) {
	raw := readFixture(t)
	variants := map[string][]byte{
		"null logical map slot": bytes.Replace(
			raw,
			[]byte("\"logical_to_physical\": [\n            0,"),
			[]byte("\"logical_to_physical\": [\n            null,"),
			1,
		),
		"case-aliased block":         bytes.Replace(raw, []byte(`"block": 0`), []byte(`"Block": 0`), 1),
		"boolean block":              bytes.Replace(raw, []byte(`"block": 0`), []byte(`"block": false`), 1),
		"fractional pages per block": bytes.Replace(raw, []byte(`"pages_per_block": 3`), []byte(`"pages_per_block": 3.5`), 1),
		"duplicate schema":           bytes.Replace(raw, []byte(`"schema":`), []byte(`"schema":"bad","schema":`), 1),
		"escaped duplicate":          bytes.Replace(raw, []byte(`"schema":`), []byte(`"sch\u0065ma":"bad","schema":`), 1),
		"unknown root key":           bytes.Replace(raw, []byte(`"schema":`), []byte(`"extra":true,"schema":`), 1),
		"invalid UTF-8":              append(append([]byte(nil), raw...), 0xff),
		"trailing value":             append(append([]byte(nil), raw...), []byte(`{}`)...),
		"excessive depth":            []byte(strings.Repeat("[", fixtureDepth+1) + strings.Repeat("]", fixtureDepth+1)),
		"oversized":                  bytes.Repeat([]byte{' '}, fixtureLimit+1),
	}
	for name, variant := range variants {
		t.Run(name, func(t *testing.T) {
			if bytes.Equal(raw, variant) {
				t.Fatal("mutation anchor was absent")
			}
			if _, err := parseFixture(variant); err == nil {
				t.Fatal("malformed fixture accepted")
			}
		})
	}
	wrong := bytes.Replace(raw, []byte(`"destination_physical": 4`), []byte(`"destination_physical": 5`), 1)
	if bytes.Equal(raw, wrong) {
		t.Fatal("wrong-result mutation anchor was absent")
	}
	document, err := parseFixture(wrong)
	if err != nil {
		t.Fatalf("wrong declaration must remain well formed: %v", err)
	}
	if err := validateFixtureModel(document); err == nil {
		t.Fatal("independent model accepted a well-typed wrong destination")
	}
}

func TestValidationOrderAndZeroPlans(t *testing.T) {
	var zero ftlpagemap.Table
	if got, err := PlanEvacuation(zero, 0, -1, []bool{true}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || !reflect.DeepEqual(got, Plan{}) {
		t.Fatalf("unconfigured precedence: %+v %v", got, err)
	}
	table := mustTable(t, 1, 4)
	for _, pagesPerBlock := range []int64{-1, 0, 3} {
		if _, err := PlanEvacuation(table, pagesPerBlock, -1, nil); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
			t.Fatalf("divisor %d precedes target index: %v", pagesPerBlock, err)
		}
	}
	for _, block := range []int64{-1, 2} {
		if _, err := PlanEvacuation(table, 2, block, []bool{true}); !errors.Is(err, ErrInvalidBlockIndex) {
			t.Fatalf("target index %d precedes mask: %v", block, err)
		}
	}
	if _, err := PlanEvacuation(table, 2, 0, []bool{true}); !errors.Is(err, ErrInvalidBlockMask) {
		t.Fatalf("mask size precedes reservation: %v", err)
	}
	pending, _, err := table.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := PlanEvacuation(pending, 2, 0, []bool{false, false, false}); !errors.Is(err, ErrInvalidBlockMask) {
		t.Fatalf("mask size must precede pending reservation: %v", err)
	}
	if _, err := PlanEvacuation(pending, 2, 0, []bool{true, false}); !errors.Is(err, ftlpagemap.ErrReservationPending) {
		t.Fatalf("pending reservation precedes target exclusion: %v", err)
	}
	if _, err := PlanEvacuation(table, 2, 0, []bool{true, false}); !errors.Is(err, ErrExcludedBlock) {
		t.Fatalf("excluded target precedes stale check: %v", err)
	}
	if _, err := PlanEvacuation(table, 2, 0, nil); !errors.Is(err, ErrNoStalePage) {
		t.Fatalf("fresh target requires stale page: %v", err)
	}
}

func TestEveryRefusalReturnsZeroPlan(t *testing.T) {
	fresh := mustTable(t, 1, 4)
	pending, _, err := fresh.Begin(0)
	if err != nil {
		t.Fatal(err)
	}
	stale := tableWithPrograms(t, 1, 4, 0, 0)
	short := tableWithPrograms(t, 3, 6, 0, 1, 2, 1, 1)
	cases := []struct {
		name    string
		table   ftlpagemap.Table
		ppb     int64
		block   int64
		mask    []bool
		wantErr error
	}{
		{name: "invalid geometry", wantErr: ftlpagemap.ErrInvalidGeometry},
		{name: "invalid block geometry", table: fresh, ppb: 3, block: 0, wantErr: ftlpagemap.ErrInvalidGeometry},
		{name: "invalid block index", table: fresh, ppb: 2, block: -1, wantErr: ErrInvalidBlockIndex},
		{name: "invalid block mask", table: fresh, ppb: 2, block: 0, mask: []bool{false}, wantErr: ErrInvalidBlockMask},
		{name: "reservation pending", table: pending, ppb: 2, block: 0, mask: []bool{false, false}, wantErr: ftlpagemap.ErrReservationPending},
		{name: "target excluded", table: stale, ppb: 2, block: 0, mask: []bool{true, false}, wantErr: ErrExcludedBlock},
		{name: "no stale page", table: fresh, ppb: 2, block: 0, wantErr: ErrNoStalePage},
		{name: "insufficient capacity", table: short, ppb: 3, block: 0, wantErr: ErrInsufficientCapacity},
	}
	for _, item := range cases {
		t.Run(item.name, func(t *testing.T) {
			plan, err := PlanEvacuation(item.table, item.ppb, item.block, item.mask)
			if !errors.Is(err, item.wantErr) || !reflect.DeepEqual(plan, Plan{}) {
				t.Fatalf("refusal returned plan=%+v err=%v; want zero plan and %v", plan, err, item.wantErr)
			}
		})
	}
}

func TestPlansOwnCopiesAndAcceptEmptyPlan(t *testing.T) {
	table := tableWithPrograms(t, 2, 8, 0, 1, 0, 1)
	plan, err := PlanEvacuation(table, 2, 0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(plan.Copies) != 0 || plan.Block != 0 || plan.FirstPhysical != 0 || plan.PageCount != 2 {
		t.Fatalf("stale/free no-copy block should return a named empty plan: %+v", plan)
	}
	if plan.Copies == nil {
		t.Fatal("successful empty plan must own an empty copy slice")
	}

	table = tableWithPrograms(t, 4, 9, 0, 1, 2, 1)
	mask := []bool{false, false, true}
	before := table.Snapshot()
	plan, err = PlanEvacuation(table, 3, 0, mask)
	if err != nil || len(plan.Copies) != 2 || plan.Copies[0] != (Copy{Logical: 0, SourcePhysical: 0, DestinationPhysical: 4}) || plan.Copies[1] != (Copy{Logical: 2, SourcePhysical: 2, DestinationPhysical: 5}) {
		t.Fatalf("ordered evacuation plan = %+v %v", plan, err)
	}
	plan.Copies[0].DestinationPhysical = 99
	if !reflect.DeepEqual(mask, []bool{false, false, true}) || !reflect.DeepEqual(table.Snapshot(), before) {
		t.Fatal("planner changed its mask or input table")
	}
	again, err := PlanEvacuation(table, 3, 0, mask)
	if err != nil || again.Copies[0].DestinationPhysical != 4 {
		t.Fatalf("returned copy records were not detached: %+v %v", again, err)
	}
}

func TestTargetAndExcludedFreeSlotsDoNotCount(t *testing.T) {
	table := tableWithPrograms(t, 1, 6, 0, 0)
	plan, err := PlanEvacuation(table, 3, 0, []bool{false, true})
	if !errors.Is(err, ErrInsufficientCapacity) || !reflect.DeepEqual(plan, Plan{}) {
		t.Fatalf("target or excluded free slots counted as spare capacity: %+v %v", plan, err)
	}
}

func TestDestinationsMustBeFree(t *testing.T) {
	table := tableWithPrograms(t, 2, 8, 0, 1, 0, 1, 0)
	plan, err := PlanEvacuation(table, 2, 1, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(plan.Copies) != 1 || plan.Copies[0] != (Copy{Logical: 1, SourcePhysical: 3, DestinationPhysical: 5}) {
		t.Fatalf("planner selected a non-free destination: %+v", plan)
	}
}

func TestInsufficientCapacityReturnsZeroPlan(t *testing.T) {
	table := tableWithPrograms(t, 3, 6, 0, 1, 2, 1, 1)
	plan, err := PlanEvacuation(table, 3, 0, nil)
	if !errors.Is(err, ErrInsufficientCapacity) || !reflect.DeepEqual(plan, Plan{}) {
		t.Fatalf("insufficient capacity returned a partial plan: %+v %v", plan, err)
	}
}

func TestMaximumPageMapGeometry(t *testing.T) {
	table := mustTable(t, 64, 128)
	for logical := int64(0); logical < 8; logical++ {
		table = publish(t, table, logical)
	}
	table = publish(t, table, 0)
	plan, err := PlanEvacuation(table, 8, 0, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(plan.Copies) != 7 || plan.Copies[0] != (Copy{Logical: 1, SourcePhysical: 1, DestinationPhysical: 9}) || plan.Copies[6] != (Copy{Logical: 7, SourcePhysical: 7, DestinationPhysical: 15}) {
		t.Fatalf("maximum geometry plan = %+v", plan)
	}
}

func TestAllReachableSmallMapsMasksAndLayouts(t *testing.T) {
	geometries := []struct{ logical, physical int64 }{{1, 2}, {1, 3}, {1, 4}, {2, 4}, {2, 6}}
	comparisons := 0
	for _, geometry := range geometries {
		states := reachableTables(t, geometry.logical, geometry.physical)
		for _, table := range states {
			status := table.Snapshot().PhysicalStatus
			for pagesPerBlock := int64(1); pagesPerBlock <= geometry.physical; pagesPerBlock++ {
				if geometry.physical%pagesPerBlock != 0 {
					continue
				}
				blockCount := geometry.physical / pagesPerBlock
				for maskBits := 0; maskBits < 1<<blockCount; maskBits++ {
					mask := make([]bool, int(blockCount))
					for bit := range mask {
						mask[bit] = maskBits&(1<<bit) != 0
					}
					for block := int64(0); block < blockCount; block++ {
						want, wantErr := modelPlanFor(table.Snapshot(), pagesPerBlock, block, mask)
						got, gotErr := PlanEvacuation(table, pagesPerBlock, block, mask)
						if errorName(gotErr) != wantErr || (wantErr == "" && !reflect.DeepEqual(got, planFromModel(want))) {
							t.Fatalf("geometry %d/%d statuses=%v ppb=%d mask=%v block=%d got=%+v/%s want=%+v/%s", geometry.logical, geometry.physical, status, pagesPerBlock, mask, block, got, errorName(gotErr), want, wantErr)
						}
						comparisons++
					}
				}
			}
		}
	}
	t.Logf("exhaustively compared %d reachable map/layout/mask/target combinations", comparisons)
}

func FuzzParseEvacuationFixture(f *testing.F) {
	f.Add(readFixtureBytes(f))
	f.Add([]byte(`{"schema":"invalid","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		document, err := parseFixture(raw)
		if err != nil {
			return
		}
		// Mutated declarations that disagree with the oracle are malformed
		// fixture inputs and are rejected before any runtime replay.
		if err := validateFixtureModel(document); err != nil {
			return
		}
	})
}

func FuzzPlanHistory(f *testing.F) {
	f.Add([]byte{0, 1, 0, 1, 0, 1, 3, 2})
	f.Add([]byte{1, 0, 1, 1, 2, 3, 0, 2, 1})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			commands = commands[:256]
		}
		table, modelState := mustTable(t, 2, 6), modelInitial(2, 6)
		for index, command := range commands {
			logical := int64(int(command>>3)%4 - 1)
			if command&1 == 0 {
				var err error
				table, modelState, err = applyModeledProgram(table, modelState, logical)
				if err != nil && !errors.Is(err, ftlpagemap.ErrInvalidLogicalPage) && !errors.Is(err, ftlpagemap.ErrNoFreePage) {
					t.Fatalf("history program %d: %v", index, err)
				}
			}
			pagesPerBlock := int64(2)
			mask := []bool{command&2 != 0, command&4 != 0, false}
			block := int64(command>>5)%5 - 1
			want, wantErr := modelPlanFor(modelState.snapshot(), pagesPerBlock, block, mask)
			got, gotErr := PlanEvacuation(table, pagesPerBlock, block, mask)
			if errorName(gotErr) != wantErr || (wantErr == "" && !reflect.DeepEqual(got, planFromModel(want))) {
				t.Fatalf("history plan %d: got=%+v/%s want=%+v/%s", index, got, errorName(gotErr), want, wantErr)
			}
			if !reflect.DeepEqual(table.Snapshot(), modelState.snapshot()) {
				t.Fatalf("history planning mutated map at %d", index)
			}
			drainWant, drainWantErr := modelDrainPlanFor(modelState.snapshot(), pagesPerBlock, block, mask)
			drainGot, drainGotErr := PlanDrain(table, pagesPerBlock, block, mask)
			if errorName(drainGotErr) != drainWantErr || (drainWantErr == "" && !reflect.DeepEqual(drainGot, planFromModel(drainWant))) {
				t.Fatalf("history drain plan %d: got=%+v/%s want=%+v/%s", index, drainGot, errorName(drainGotErr), drainWant, drainWantErr)
			}
			if !reflect.DeepEqual(table.Snapshot(), modelState.snapshot()) {
				t.Fatalf("history drain planning mutated map at %d", index)
			}
		}
	})
}

func readFixture(t *testing.T) []byte {
	t.Helper()
	return readFixtureBytes(t)
}
func readFixtureBytes(t testing.TB) []byte {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-evacuation-plan.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func parseFixture(raw []byte) (fixtureDocument, error) {
	return parseFixtureSchema(raw, fixtureSchema)
}

func parseFixtureSchema(raw []byte, expectedSchema string) (fixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return fixtureDocument{}, errors.New("invalid fixture size or UTF-8")
	}
	if err := scanJSON(raw); err != nil {
		return fixtureDocument{}, err
	}
	if err := validateFixtureKinds(raw); err != nil {
		return fixtureDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var document fixtureDocument
	if err := decoder.Decode(&document); err != nil {
		return fixtureDocument{}, err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != io.EOF {
		return fixtureDocument{}, errors.New("trailing JSON value")
	}
	if document.Schema != expectedSchema || len(document.Cases) == 0 || len(document.Cases) > 32 {
		return fixtureDocument{}, errors.New("invalid fixture schema or case count")
	}
	for _, item := range document.Cases {
		if item.Name == "" || item.LogicalCount == nil || item.PhysicalCount == nil || item.PagesPerBlock == nil || item.ProgramLogicals == nil || item.Block == nil || item.ExcludedBlocks == nil || item.Expected == nil || item.Expected.State == nil {
			return fixtureDocument{}, errors.New("invalid fixture case fields")
		}
		if *item.LogicalCount < 1 || *item.LogicalCount > 64 || *item.PhysicalCount < 2 || *item.PhysicalCount > 128 || *item.PhysicalCount <= *item.LogicalCount || *item.PagesPerBlock < 1 || *item.PagesPerBlock > *item.PhysicalCount || *item.PhysicalCount%*item.PagesPerBlock != 0 {
			return fixtureDocument{}, errors.New("fixture geometry outside supported bounds")
		}
		blockCount := *item.PhysicalCount / *item.PagesPerBlock
		if len(*item.ExcludedBlocks) != 0 && int64(len(*item.ExcludedBlocks)) != blockCount {
			return fixtureDocument{}, errors.New("invalid fixture mask length")
		}
		if len(*item.ProgramLogicals) > 128 || len(*item.ExcludedBlocks) > 128 {
			return fixtureDocument{}, errors.New("fixture array exceeds portable bound")
		}
		if int64(len(*item.ProgramLogicals)) > *item.PhysicalCount {
			return fixtureDocument{}, errors.New("fixture setup exceeds free physical capacity")
		}
		for _, logical := range *item.ProgramLogicals {
			if logical < 0 || logical >= *item.LogicalCount {
				return fixtureDocument{}, errors.New("fixture program address outside geometry")
			}
		}
		if err := validateFixtureExpected(item); err != nil {
			return fixtureDocument{}, err
		}
	}
	return document, nil
}

func validateFixtureKinds(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return err
	}
	root, ok := value.(map[string]any)
	if !ok || !exactKeys(root, []string{"schema", "cases"}, nil) {
		return errors.New("invalid fixture root keys")
	}
	if _, ok := root["schema"].(string); !ok {
		return errors.New("schema must be text")
	}
	cases, ok := root["cases"].([]any)
	if !ok {
		return errors.New("cases must be an array")
	}
	for _, value := range cases {
		item, ok := value.(map[string]any)
		if !ok || !exactKeys(item, []string{"name", "logical_count", "physical_count", "pages_per_block", "program_logicals", "block", "excluded_blocks", "expected"}, nil) {
			return errors.New("invalid case keys")
		}
		if _, ok := item["name"].(string); !ok || !integerToken(item["logical_count"]) || !integerToken(item["physical_count"]) || !integerToken(item["pages_per_block"]) || !integerToken(item["block"]) {
			return errors.New("invalid case token kinds")
		}
		programs, ok := item["program_logicals"].([]any)
		if !ok {
			return errors.New("program_logicals must be an array")
		}
		for _, logical := range programs {
			if !integerToken(logical) {
				return errors.New("program logical must be an integer")
			}
		}
		mask, ok := item["excluded_blocks"].([]any)
		if !ok {
			return errors.New("excluded_blocks must be an array")
		}
		for _, excluded := range mask {
			if _, ok := excluded.(bool); !ok {
				return errors.New("excluded block item must be boolean")
			}
		}
		if err := validateExpectedKinds(item["expected"]); err != nil {
			return err
		}
	}
	return nil
}

func validateExpectedKinds(value any) error {
	expected, ok := value.(map[string]any)
	if !ok || !exactKeys(expected, []string{"state"}, []string{"plan", "error"}) {
		return errors.New("invalid expected keys")
	}
	if value, exists := expected["error"]; exists {
		if _, ok := value.(string); !ok {
			return errors.New("expected error must be text")
		}
	}
	if value, exists := expected["plan"]; exists {
		plan, ok := value.(map[string]any)
		if !ok || !exactKeys(plan, []string{"block", "first_physical", "page_count", "copies"}, nil) ||
			!integerToken(plan["block"]) || !integerToken(plan["first_physical"]) || !integerToken(plan["page_count"]) {
			return errors.New("invalid plan token kinds")
		}
		copies, ok := plan["copies"].([]any)
		if !ok {
			return errors.New("copies must be an array")
		}
		for _, value := range copies {
			copyRecord, ok := value.(map[string]any)
			if !ok || !exactKeys(copyRecord, []string{"logical", "source_physical", "destination_physical"}, nil) || !integerToken(copyRecord["logical"]) || !integerToken(copyRecord["source_physical"]) || !integerToken(copyRecord["destination_physical"]) {
				return errors.New("invalid copy token kinds")
			}
		}
	}
	state, ok := expected["state"].(map[string]any)
	if !ok || !exactKeys(state, []string{"logical_to_physical", "physical_status"}, nil) {
		return errors.New("invalid state keys")
	}
	logical, ok := state["logical_to_physical"].([]any)
	if !ok {
		return errors.New("logical_to_physical must be an array")
	}
	for _, value := range logical {
		if !integerToken(value) {
			return errors.New("logical map value must be integer")
		}
	}
	statuses, ok := state["physical_status"].([]any)
	if !ok {
		return errors.New("physical_status must be an array")
	}
	for _, value := range statuses {
		status, ok := value.(string)
		if !ok || (status != "free" && status != "live" && status != "stale") {
			return errors.New("invalid physical status")
		}
	}
	return nil
}

func validateFixtureExpected(testCase fixtureCase) error {
	expected := testCase.Expected
	if expected.State == nil || expected.State.LogicalToPhysical == nil || expected.State.PhysicalStatus == nil ||
		len(*expected.State.LogicalToPhysical) != int(*testCase.LogicalCount) || len(*expected.State.PhysicalStatus) != int(*testCase.PhysicalCount) {
		return errors.New("invalid expected map dimensions")
	}
	for _, physical := range *expected.State.LogicalToPhysical {
		if physical < -1 || physical >= *testCase.PhysicalCount {
			return errors.New("expected mapping address outside geometry")
		}
	}
	for _, status := range *expected.State.PhysicalStatus {
		if status != "free" && status != "live" && status != "stale" {
			return errors.New("invalid expected status")
		}
	}
	if expected.Error != nil {
		if expected.Plan != nil || !knownError(*expected.Error) {
			return errors.New("invalid expected refusal shape")
		}
	} else if expected.Plan == nil || expected.Plan.Block == nil || expected.Plan.FirstPhysical == nil || expected.Plan.PageCount == nil || expected.Plan.Copies == nil {
		return errors.New("invalid expected plan shape")
	}
	if expected.Plan != nil {
		if expected.Plan.Block == nil || expected.Plan.FirstPhysical == nil || expected.Plan.PageCount == nil || expected.Plan.Copies == nil {
			return errors.New("incomplete expected plan")
		}
		for _, item := range *expected.Plan.Copies {
			if item.Logical == nil || item.SourcePhysical == nil || item.DestinationPhysical == nil {
				return errors.New("incomplete expected copy")
			}
		}
	}
	return nil
}

func scanJSON(raw []byte) error {
	if err := validatePlanningSurrogateEscapes(raw); err != nil {
		return err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	if err := scanValue(decoder, token, 0); err != nil {
		return err
	}
	if _, err := decoder.Token(); err != io.EOF {
		return errors.New("trailing JSON token")
	}
	return nil
}
func scanValue(decoder *json.Decoder, token json.Token, depth int) error {
	if depth > fixtureDepth {
		return errors.New("fixture nesting exceeds limit")
	}
	delim, ok := token.(json.Delim)
	if !ok {
		return nil
	}
	switch delim {
	case '{':
		seen := make(map[string]bool)
		for decoder.More() {
			keyToken, err := decoder.Token()
			if err != nil {
				return err
			}
			key, ok := keyToken.(string)
			if !ok || seen[key] {
				return errors.New("invalid or duplicate object key")
			}
			seen[key] = true
			value, err := decoder.Token()
			if err != nil {
				return err
			}
			if err := scanValue(decoder, value, depth+1); err != nil {
				return err
			}
		}
		_, err := decoder.Token()
		return err
	case '[':
		for decoder.More() {
			value, err := decoder.Token()
			if err != nil {
				return err
			}
			if err := scanValue(decoder, value, depth+1); err != nil {
				return err
			}
		}
		_, err := decoder.Token()
		return err
	default:
		return errors.New("unexpected JSON delimiter")
	}
}
func exactKeys(value map[string]any, required, optional []string) bool {
	allowed := make(map[string]bool, len(required)+len(optional))
	for _, key := range required {
		allowed[key] = true
		if _, ok := value[key]; !ok {
			return false
		}
	}
	for _, key := range optional {
		allowed[key] = true
	}
	for key := range value {
		if !allowed[key] {
			return false
		}
	}
	return true
}
func integerToken(value any) bool {
	number, ok := value.(json.Number)
	if !ok {
		return false
	}
	_, err := number.Int64()
	return err == nil
}

func validateFixtureModel(document fixtureDocument) error {
	for _, testCase := range document.Cases {
		state := modelForFixture(testCase)
		wantPlan, wantErr := modelPlanFor(state.snapshot(), *testCase.PagesPerBlock, *testCase.Block, *testCase.ExcludedBlocks)
		if fixtureError(testCase.Expected.Error) != wantErr {
			return fmt.Errorf("case %s error=%s, model predicts %s", testCase.Name, fixtureError(testCase.Expected.Error), wantErr)
		}
		if wantErr == "" && !reflect.DeepEqual(*testCase.Expected.Plan, fixturePlanFromModel(wantPlan)) {
			return fmt.Errorf("case %s plan differs from independent model", testCase.Name)
		}
		if !reflect.DeepEqual(*testCase.Expected.State, fixtureStateOf(state)) {
			return fmt.Errorf("case %s map state differs from independent model", testCase.Name)
		}
	}
	return nil
}
func modelForFixture(testCase fixtureCase) model {
	state := modelInitial(*testCase.LogicalCount, *testCase.PhysicalCount)
	for _, logical := range *testCase.ProgramLogicals {
		state = modelProgram(state, logical)
	}
	return state
}
func modelInitial(logicalCount, physicalCount int64) model {
	mapping := make([]int64, int(logicalCount))
	for index := range mapping {
		mapping[index] = -1
	}
	return model{logicalCount: logicalCount, physicalCount: physicalCount, mapping: mapping, status: make([]ftlpagemap.PhysicalStatus, int(physicalCount))}
}
func modelProgram(before model, logical int64) model {
	next := cloneModel(before)
	physical := -1
	for index, status := range next.status {
		if status == ftlpagemap.Free {
			physical = index
			break
		}
	}
	if physical < 0 || logical < 0 || logical >= before.logicalCount {
		return before
	}
	old := next.mapping[int(logical)]
	if old >= 0 {
		next.status[int(old)] = ftlpagemap.Stale
	}
	next.mapping[int(logical)] = int64(physical)
	next.status[physical] = ftlpagemap.Live
	return next
}
func cloneModel(before model) model {
	return model{logicalCount: before.logicalCount, physicalCount: before.physicalCount, mapping: append([]int64(nil), before.mapping...), status: append([]ftlpagemap.PhysicalStatus(nil), before.status...)}
}
func modelPlanFor(snapshot ftlpagemap.Snapshot, pagesPerBlock, block int64, mask []bool) (modelPlan, string) {
	return modelPlanForMode(snapshot, pagesPerBlock, block, mask, true)
}

func modelDrainPlanFor(snapshot ftlpagemap.Snapshot, pagesPerBlock, block int64, mask []bool) (modelPlan, string) {
	return modelPlanForMode(snapshot, pagesPerBlock, block, mask, false)
}

func modelPlanForMode(snapshot ftlpagemap.Snapshot, pagesPerBlock, block int64, mask []bool, requireStale bool) (modelPlan, string) {
	logicalCount, physicalCount := int64(len(snapshot.LogicalToPhysical)), int64(len(snapshot.PhysicalStatus))
	if logicalCount < 1 || logicalCount > 64 || physicalCount < 2 || physicalCount > 128 || physicalCount <= logicalCount || pagesPerBlock < 1 || pagesPerBlock > physicalCount || physicalCount%pagesPerBlock != 0 {
		return modelPlan{}, "invalid_geometry"
	}
	blockCount := physicalCount / pagesPerBlock
	if block < 0 || block >= blockCount {
		return modelPlan{}, "invalid_block_index"
	}
	if len(mask) != 0 && int64(len(mask)) != blockCount {
		return modelPlan{}, "invalid_block_mask"
	}
	if snapshot.Reservation != nil {
		return modelPlan{}, "reservation_pending"
	}
	if requireStale && len(mask) != 0 && mask[int(block)] {
		return modelPlan{}, "excluded_block"
	}
	logicalAt := make([]int64, len(snapshot.PhysicalStatus))
	for index := range logicalAt {
		logicalAt[index] = -1
	}
	for logical, physical := range snapshot.LogicalToPhysical {
		if physical >= 0 && physical < physicalCount {
			logicalAt[int(physical)] = int64(logical)
		}
	}
	first := block * pagesPerBlock
	plan := modelPlan{block: block, first: first, count: pagesPerBlock, copies: make([]Copy, 0)}
	stale := false
	for physical := first; physical < first+pagesPerBlock; physical++ {
		switch snapshot.PhysicalStatus[int(physical)] {
		case ftlpagemap.Stale:
			stale = true
		case ftlpagemap.Live:
			plan.copies = append(plan.copies, Copy{Logical: logicalAt[int(physical)], SourcePhysical: physical})
		}
	}
	if requireStale && !stale {
		return modelPlan{}, "no_stale_page"
	}
	var destinations []int64
	for physical, status := range snapshot.PhysicalStatus {
		candidateBlock := int64(physical) / pagesPerBlock
		if status != ftlpagemap.Free || candidateBlock == block || (len(mask) != 0 && mask[int(candidateBlock)]) {
			continue
		}
		destinations = append(destinations, int64(physical))
	}
	if len(destinations) < len(plan.copies) {
		return modelPlan{}, "insufficient_capacity"
	}
	for index := range plan.copies {
		plan.copies[index].DestinationPhysical = destinations[index]
	}
	return plan, ""
}
func planFromModel(plan modelPlan) Plan {
	copies := make([]Copy, len(plan.copies))
	copy(copies, plan.copies)
	return Plan{Block: plan.block, FirstPhysical: plan.first, PageCount: plan.count, Copies: copies}
}
func fixturePlanFromModel(plan modelPlan) fixturePlan {
	block, first, count := plan.block, plan.first, plan.count
	copies := make([]fixtureCopy, len(plan.copies))
	for index, item := range plan.copies {
		logical, source, destination := item.Logical, item.SourcePhysical, item.DestinationPhysical
		copies[index] = fixtureCopy{Logical: &logical, SourcePhysical: &source, DestinationPhysical: &destination}
	}
	return fixturePlan{Block: &block, FirstPhysical: &first, PageCount: &count, Copies: &copies}
}
func runtimePlan(plan *fixturePlan) Plan {
	copies := make([]Copy, len(*plan.Copies))
	for index, item := range *plan.Copies {
		copies[index] = Copy{Logical: *item.Logical, SourcePhysical: *item.SourcePhysical, DestinationPhysical: *item.DestinationPhysical}
	}
	return Plan{Block: *plan.Block, FirstPhysical: *plan.FirstPhysical, PageCount: *plan.PageCount, Copies: copies}
}
func fixtureStateOf(state model) fixtureState {
	mapping := append([]int64(nil), state.mapping...)
	statuses := make([]string, len(state.status))
	for index, status := range state.status {
		switch status {
		case ftlpagemap.Free:
			statuses[index] = "free"
		case ftlpagemap.Live:
			statuses[index] = "live"
		case ftlpagemap.Stale:
			statuses[index] = "stale"
		}
	}
	return fixtureState{LogicalToPhysical: &mapping, PhysicalStatus: &statuses}
}
func (state model) snapshot() ftlpagemap.Snapshot {
	return ftlpagemap.Snapshot{LogicalToPhysical: append([]int64(nil), state.mapping...), PhysicalStatus: append([]ftlpagemap.PhysicalStatus(nil), state.status...)}
}
func fixtureError(value *string) string {
	if value == nil {
		return ""
	}
	return *value
}
func knownError(value string) bool {
	switch value {
	case "invalid_geometry", "reservation_pending", "invalid_block_index", "invalid_block_mask", "excluded_block", "no_stale_page", "insufficient_capacity":
		return true
	default:
		return false
	}
}
func errorName(err error) string {
	switch {
	case err == nil:
		return ""
	case errors.Is(err, ftlpagemap.ErrInvalidGeometry):
		return "invalid_geometry"
	case errors.Is(err, ftlpagemap.ErrReservationPending):
		return "reservation_pending"
	case errors.Is(err, ErrInvalidBlockIndex):
		return "invalid_block_index"
	case errors.Is(err, ErrInvalidBlockMask):
		return "invalid_block_mask"
	case errors.Is(err, ErrExcludedBlock):
		return "excluded_block"
	case errors.Is(err, ErrNoStalePage):
		return "no_stale_page"
	case errors.Is(err, ErrInsufficientCapacity):
		return "insufficient_capacity"
	default:
		return "unknown"
	}
}
func tableFromFixture(t *testing.T, testCase fixtureCase) ftlpagemap.Table {
	t.Helper()
	table := mustTable(t, *testCase.LogicalCount, *testCase.PhysicalCount)
	for _, logical := range *testCase.ProgramLogicals {
		table = publish(t, table, logical)
	}
	return table
}
func tableWithPrograms(t *testing.T, logical, physical int64, programs ...int64) ftlpagemap.Table {
	t.Helper()
	table := mustTable(t, logical, physical)
	for _, address := range programs {
		table = publish(t, table, address)
	}
	return table
}
func mustTable(t *testing.T, logical, physical int64) ftlpagemap.Table {
	t.Helper()
	table, err := ftlpagemap.New(logical, physical)
	if err != nil {
		t.Fatal(err)
	}
	return table
}
func publish(t *testing.T, table ftlpagemap.Table, logical int64) ftlpagemap.Table {
	t.Helper()
	table, _, err := table.Begin(logical)
	if err != nil {
		t.Fatal(err)
	}
	table, err = table.Finish(ftlpagemap.MediaReady)
	if err != nil {
		t.Fatal(err)
	}
	return table
}
func applyModeledProgram(table ftlpagemap.Table, state model, logical int64) (ftlpagemap.Table, model, error) {
	if logical < 0 || logical >= state.logicalCount {
		return table, state, ftlpagemap.ErrInvalidLogicalPage
	}
	next, _, err := table.Begin(logical)
	if err != nil {
		return table, state, err
	}
	next, err = next.Finish(ftlpagemap.MediaReady)
	if err != nil {
		return table, state, err
	}
	return next, modelProgram(state, logical), nil
}
func reachableTables(t *testing.T, logical, physical int64) []ftlpagemap.Table {
	t.Helper()
	initial := mustTable(t, logical, physical)
	queue := []ftlpagemap.Table{initial}
	seen := map[string]bool{mapKey(initial.Snapshot()): true}
	for head := 0; head < len(queue); head++ {
		current := queue[head]
		for address := int64(0); address < logical; address++ {
			next, _, err := current.Begin(address)
			if err != nil {
				continue
			}
			next, err = next.Finish(ftlpagemap.MediaReady)
			if err != nil {
				t.Fatal(err)
			}
			key := mapKey(next.Snapshot())
			if !seen[key] {
				seen[key] = true
				queue = append(queue, next)
			}
		}
	}
	return queue
}
func mapKey(snapshot ftlpagemap.Snapshot) string {
	return fmt.Sprintf("%v/%v", snapshot.LogicalToPhysical, snapshot.PhysicalStatus)
}
