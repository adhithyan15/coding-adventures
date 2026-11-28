package ftlpagemap

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
)

const reclamationFixtureSchema = "coding-adventures.database-media.ftl-page-reclamation.v1"

type reclamationDocument struct {
	Schema string            `json:"schema"`
	Cases  []reclamationCase `json:"cases"`
}
type reclamationCase struct {
	Name          string            `json:"name"`
	LogicalCount  int64             `json:"logical_count"`
	PhysicalCount int64             `json:"physical_count"`
	Steps         []reclamationStep `json:"steps"`
}
type reclamationStep struct {
	Op            string                 `json:"op"`
	Logical       *int64                 `json:"logical,omitempty"`
	Excluded      *[]bool                `json:"excluded,omitempty"`
	FirstPhysical *int64                 `json:"first_physical,omitempty"`
	PageCount     *int64                 `json:"page_count,omitempty"`
	Disposition   *string                `json:"disposition,omitempty"`
	Expected      reclamationExpectation `json:"expected"`
}
type reclamationExpectation struct {
	Error       *string             `json:"error,omitempty"`
	Physical    *int64              `json:"physical,omitempty"`
	Present     *bool               `json:"present,omitempty"`
	Reservation *fixtureReservation `json:"reservation,omitempty"`
	State       *fixtureSnapshot    `json:"state"`
}

func TestCanonicalReclamationFixtureModelFirstThenRuntime(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-reclamation.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	document, err := parseReclamationFixture(raw)
	if err != nil {
		t.Fatalf("parse reclamation fixture: %v", err)
	}
	if err := validateReclamationModel(document); err != nil {
		t.Fatalf("independent model rejected fixture: %v", err)
	}
	for _, testCase := range document.Cases {
		table, err := New(testCase.LogicalCount, testCase.PhysicalCount)
		if err != nil {
			t.Fatal(err)
		}
		for index, step := range testCase.Steps {
			table, err = replayReclamationStep(table, step)
			if err != nil {
				t.Fatalf("%s step %d: %v", testCase.Name, index, err)
			}
		}
	}
}

func TestReclamationFixtureRejectsMalformedAndWellFormedWrongExpectation(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-reclamation.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	for name, bad := range map[string][]byte{
		"duplicate field":              bytes.Replace(raw, []byte(`"schema":`), []byte(`"schema":"wrong","schema":`), 1),
		"escaped duplicate":            bytes.Replace(raw, []byte(`"schema":`), []byte(`"sch\u0065ma":"wrong","schema":`), 1),
		"bad schema":                   bytes.Replace(raw, []byte(reclamationFixtureSchema), []byte("wrong.schema"), 1),
		"trailing bytes":               append(append([]byte(nil), raw...), []byte(` {}`)...),
		"invalid UTF-8":                append(append([]byte(nil), raw...), 0xff),
		"oversized":                    bytes.Repeat([]byte{' '}, fixtureLimit+1),
		"unknown field":                bytes.Replace(raw, []byte(`"schema":`), []byte(`"extra":true,"schema":`), 1),
		"fractional address":           bytes.Replace(raw, []byte(`"logical": 0`), []byte(`"logical": 0.5`), 1),
		"boolean extent":               bytes.Replace(raw, []byte(`"first_physical": 0`), []byte(`"first_physical": true`), 1),
		"null mask entry":              bytes.Replace(raw, []byte("\"excluded\": [\n            false"), []byte("\"excluded\": [\n            null"), 1),
		"null logical mapping":         bytes.Replace(raw, []byte("\"logical_to_physical\": [\n                -1"), []byte("\"logical_to_physical\": [\n                null"), 1),
		"missing reservation physical": bytes.Replace(raw, []byte("\"physical\": 0\n"), []byte("\n"), 1),
		"null operation reservation":   bytes.Replace(raw, []byte("\"expected\": {"), []byte("\"expected\": {\n            \"reservation\": null,"), 1),
		"null reservation logical":     bytes.Replace(raw, []byte("\"reservation\": {\n              \"logical\": 0"), []byte("\"reservation\": {\n              \"logical\": null"), 1),
		"null reservation physical":    bytes.Replace(raw, []byte("\"reservation\": {\n              \"logical\": 0,\n              \"physical\": 0"), []byte("\"reservation\": {\n              \"logical\": 0,\n              \"physical\": null"), 1),
		"excessive depth":              []byte(strings.Repeat("[", fixtureDepth+1) + strings.Repeat("]", fixtureDepth+1)),
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := parseReclamationFixture(bad); err == nil {
				t.Fatal("invalid fixture accepted")
			}
		})
	}
	wrong := bytes.Replace(raw, []byte(`"physical": 2`), []byte(`"physical": 3`), 1)
	if bytes.Equal(raw, wrong) {
		t.Fatal("wrong-result mutation anchor missing")
	}
	document, err := parseReclamationFixture(wrong)
	if err != nil {
		t.Fatalf("mutated fixture must stay well formed: %v", err)
	}
	if err := validateReclamationModel(document); err == nil {
		t.Fatal("model accepted corrupted declared state")
	}
}

func parseReclamationFixture(raw []byte) (reclamationDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return reclamationDocument{}, errors.New("invalid reclamation fixture size or UTF-8")
	}
	if err := validateSurrogateEscapes(raw); err != nil {
		return reclamationDocument{}, err
	}
	if err := scanJSON(raw); err != nil {
		return reclamationDocument{}, err
	}
	if err := validateReclamationJSONKinds(raw); err != nil {
		return reclamationDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var doc reclamationDocument
	if err := decoder.Decode(&doc); err != nil {
		return reclamationDocument{}, err
	}
	var extra any
	if err := decoder.Decode(&extra); err != io.EOF {
		return reclamationDocument{}, errors.New("trailing reclamation JSON")
	}
	if doc.Schema != reclamationFixtureSchema || len(doc.Cases) == 0 || len(doc.Cases) > 32 {
		return reclamationDocument{}, errors.New("invalid reclamation schema or cases")
	}
	for _, testCase := range doc.Cases {
		if testCase.Name == "" || testCase.LogicalCount < 1 || testCase.LogicalCount > 64 || testCase.PhysicalCount < 2 || testCase.PhysicalCount > 128 || testCase.PhysicalCount <= testCase.LogicalCount || len(testCase.Steps) == 0 || len(testCase.Steps) > 64 {
			return reclamationDocument{}, errors.New("invalid reclamation case shape")
		}
		for _, step := range testCase.Steps {
			if step.Expected.State == nil {
				return reclamationDocument{}, errors.New("missing reclamation state")
			}
			switch step.Op {
			case "lookup":
				if step.Logical == nil || step.Excluded != nil || step.FirstPhysical != nil || step.PageCount != nil || step.Disposition != nil || step.Expected.Physical == nil || step.Expected.Present == nil || step.Expected.Reservation != nil {
					return reclamationDocument{}, errors.New("bad lookup shape")
				}
			case "begin", "begin_excluding":
				if step.Logical == nil || step.FirstPhysical != nil || step.PageCount != nil || step.Disposition != nil || step.Expected.Physical != nil || step.Expected.Present != nil || (step.Expected.Error == nil) != (step.Expected.Reservation != nil) || (step.Op == "begin" && step.Excluded != nil) || (step.Op == "begin_excluding" && step.Excluded == nil) {
					return reclamationDocument{}, errors.New("bad begin shape")
				}
			case "finish":
				if step.Logical != nil || step.Excluded != nil || step.FirstPhysical != nil || step.PageCount != nil || step.Disposition == nil || step.Expected.Physical != nil || step.Expected.Present != nil || step.Expected.Reservation != nil {
					return reclamationDocument{}, errors.New("bad finish shape")
				}
			case "release_erased":
				if step.Logical != nil || step.Excluded != nil || step.FirstPhysical == nil || step.PageCount == nil || step.Disposition != nil || step.Expected.Physical != nil || step.Expected.Present != nil || step.Expected.Reservation != nil {
					return reclamationDocument{}, errors.New("bad release shape")
				}
			default:
				return reclamationDocument{}, errors.New("unknown reclamation operation")
			}
			if step.Expected.Error != nil && errorFromReclamationFixture(step.Expected.Error) == nil {
				return reclamationDocument{}, errors.New("unknown reclamation error")
			}
		}
	}
	return doc, nil
}

// encoding/json's struct decoder accepts null into scalar fields and matches
// struct tags without regard to key case. Validate the shared profile's exact
// object keys and token kinds first so null, missing, aliased, or fractional
// values cannot collapse into ordinary zero values.
func validateReclamationJSONKinds(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var root any
	if err := decoder.Decode(&root); err != nil {
		return err
	}
	rootObject, ok := root.(map[string]any)
	if !ok || !exactKeys(rootObject, []string{"schema", "cases"}, nil) {
		return errors.New("invalid reclamation root keys")
	}
	if _, ok := rootObject["schema"].(string); !ok {
		return errors.New("reclamation schema must be a string")
	}
	cases, ok := rootObject["cases"].([]any)
	if !ok {
		return errors.New("reclamation cases must be an array")
	}
	for _, rawCase := range cases {
		caseObject, ok := rawCase.(map[string]any)
		if !ok || !exactKeys(caseObject, []string{"name", "logical_count", "physical_count", "steps"}, nil) {
			return errors.New("invalid reclamation case keys")
		}
		if _, ok := caseObject["name"].(string); !ok || !jsonInteger(caseObject["logical_count"]) || !jsonInteger(caseObject["physical_count"]) {
			return errors.New("invalid reclamation case field types")
		}
		steps, ok := caseObject["steps"].([]any)
		if !ok {
			return errors.New("reclamation steps must be an array")
		}
		for _, rawStep := range steps {
			step, ok := rawStep.(map[string]any)
			if !ok {
				return errors.New("reclamation step must be an object")
			}
			op, ok := step["op"].(string)
			if !ok {
				return errors.New("reclamation operation must be a string")
			}
			allowed := map[string]bool{"op": true, "expected": true}
			switch op {
			case "lookup", "begin":
				allowed["logical"] = true
			case "begin_excluding":
				allowed["logical"], allowed["excluded"] = true, true
			case "finish":
				allowed["disposition"] = true
			case "release_erased":
				allowed["first_physical"], allowed["page_count"] = true, true
			default:
				return errors.New("unknown reclamation operation")
			}
			if _, exists := step["logical"]; exists && !jsonInteger(step["logical"]) {
				return errors.New("reclamation logical address must be an integer token")
			}
			if _, exists := step["first_physical"]; exists && !jsonInteger(step["first_physical"]) {
				return errors.New("first physical address must be an integer token")
			}
			if _, exists := step["page_count"]; exists && !jsonInteger(step["page_count"]) {
				return errors.New("page count must be an integer token")
			}
			if _, exists := step["disposition"]; exists {
				if _, ok := step["disposition"].(string); !ok {
					return errors.New("disposition must be a string")
				}
			}
			if _, exists := step["excluded"]; exists {
				mask, ok := step["excluded"].([]any)
				if !ok {
					return errors.New("allocation mask must be an array")
				}
				for _, bit := range mask {
					if _, ok := bit.(bool); !ok {
						return errors.New("allocation mask entries must be booleans")
					}
				}
			}
			for key := range step {
				if !allowed[key] {
					return errors.New("unknown or incorrectly-cased reclamation step key")
				}
			}
			if _, ok := step["expected"]; !ok {
				return errors.New("missing reclamation expectation")
			}
			if err := validateReclamationExpectedKinds(step["expected"]); err != nil {
				return err
			}
		}
	}
	return nil
}

func validateReclamationExpectedKinds(value any) error {
	expected, ok := value.(map[string]any)
	if !ok || !exactKeys(expected, []string{"state"}, []string{"error", "physical", "present", "reservation"}) {
		return errors.New("invalid reclamation expected-result keys")
	}
	if value, exists := expected["error"]; exists {
		if _, ok := value.(string); !ok {
			return errors.New("expected error must be a string")
		}
	}
	if value, exists := expected["physical"]; exists && !jsonInteger(value) {
		return errors.New("expected physical address must be an integer token")
	}
	if value, exists := expected["present"]; exists {
		if _, ok := value.(bool); !ok {
			return errors.New("expected presence must be boolean")
		}
	}
	if value, exists := expected["reservation"]; exists && value != nil {
		if err := validateReclamationReservation(value); err != nil {
			return err
		}
	} else if _, exists := expected["reservation"]; exists {
		return errors.New("operation reservation must be omitted when absent")
	}
	state, ok := expected["state"].(map[string]any)
	if !ok || !exactKeys(state, []string{"logical_to_physical", "physical_status", "reservation"}, nil) {
		return errors.New("invalid reclamation snapshot keys")
	}
	logicalMap, ok := state["logical_to_physical"].([]any)
	if !ok {
		return errors.New("logical map must be an array")
	}
	for _, address := range logicalMap {
		if !jsonInteger(address) {
			return errors.New("logical map entries must be integer tokens")
		}
	}
	statuses, ok := state["physical_status"].([]any)
	if !ok {
		return errors.New("physical status must be an array")
	}
	for _, value := range statuses {
		status, ok := value.(string)
		if !ok || (status != "free" && status != "reserved" && status != "live" && status != "stale") {
			return errors.New("invalid physical status token")
		}
	}
	if state["reservation"] != nil {
		if err := validateReclamationReservation(state["reservation"]); err != nil {
			return err
		}
	}
	return nil
}

func validateReclamationReservation(value any) error {
	reservation, ok := value.(map[string]any)
	if !ok || !exactKeys(reservation, []string{"logical", "physical"}, nil) || !jsonInteger(reservation["logical"]) || !jsonInteger(reservation["physical"]) {
		return errors.New("reservation requires exact integer logical and physical fields")
	}
	return nil
}

func jsonInteger(value any) bool {
	number, ok := value.(json.Number)
	if !ok {
		return false
	}
	_, err := number.Int64()
	return err == nil
}

func exactKeys(object map[string]any, required, optional []string) bool {
	allowed := make(map[string]bool, len(required)+len(optional))
	for _, key := range required {
		allowed[key] = true
		if _, exists := object[key]; !exists {
			return false
		}
	}
	for _, key := range optional {
		allowed[key] = true
	}
	for key := range object {
		if !allowed[key] {
			return false
		}
	}
	return true
}

func validateReclamationModel(doc reclamationDocument) error {
	for _, testCase := range doc.Cases {
		state := modelInitial(testCase.LogicalCount, testCase.PhysicalCount)
		for index, step := range testCase.Steps {
			next := state
			var physical int64
			var present bool
			var reservation *Reservation
			var expectedErr error
			switch step.Op {
			case "lookup":
				logical := *step.Logical
				if logical < 0 || logical >= state.logicalCount {
					expectedErr = ErrInvalidLogicalPage
				} else if found := state.logicalMap[int(logical)]; found != noMapping {
					physical, present = found, true
				}
			case "begin", "begin_excluding":
				mask := []bool(nil)
				if step.Excluded != nil {
					mask = *step.Excluded
				}
				var accepted bool
				next, accepted = modelBeginExcluding(state, *step.Logical, mask)
				if !accepted {
					switch {
					case *step.Logical < 0 || *step.Logical >= state.logicalCount:
						expectedErr = ErrInvalidLogicalPage
					case state.pending != nil:
						expectedErr = ErrReservationPending
					case len(mask) != 0 && len(mask) != len(state.physical):
						expectedErr = ErrInvalidFence
					default:
						expectedErr = ErrNoFreePage
					}
				} else {
					r := *next.pending
					reservation = &r
				}
			case "finish":
				disposition, ok := parseDisposition(*step.Disposition)
				if !ok {
					if state.pending == nil {
						expectedErr = ErrNoReservation
					} else {
						expectedErr = ErrInvalidDisposition
					}
				} else {
					var accepted bool
					next, accepted = modelFinish(state, disposition)
					if !accepted {
						expectedErr = ErrNoReservation
					}
				}
			case "release_erased":
				var accepted bool
				next, accepted = modelReleaseErased(state, *step.FirstPhysical, *step.PageCount)
				if !accepted {
					first, count := *step.FirstPhysical, *step.PageCount
					if first < 0 || first >= int64(len(state.physical)) || count <= 0 || count > int64(len(state.physical))-first {
						expectedErr = ErrInvalidExtent
					} else {
						expectedErr = ErrIneligibleExtent
					}
				}
			}
			if err := compareReclamationExpectation(step, next, physical, present, reservation, expectedErr); err != nil {
				return fmt.Errorf("case %q step %d: %w", testCase.Name, index, err)
			}
			state = next
		}
	}
	return nil
}

func compareReclamationExpectation(step reclamationStep, state modelState, physical int64, present bool, reservation *Reservation, expectedErr error) error {
	var declared error
	if step.Expected.Error != nil {
		declared = errorFromReclamationFixture(step.Expected.Error)
	}
	if !errors.Is(declared, expectedErr) || (declared == nil) != (expectedErr == nil) {
		return fmt.Errorf("declared error %v want %v", declared, expectedErr)
	}
	if !reflect.DeepEqual(*step.Expected.State, fixtureSnapshotOf(state)) {
		return errors.New("declared complete state differs from independent model")
	}
	if expectedErr == nil {
		if step.Op == "lookup" && (*step.Expected.Physical != physical || *step.Expected.Present != present) {
			return errors.New("declared lookup result differs")
		}
		if (step.Op == "begin" || step.Op == "begin_excluding") && !sameFixtureReservation(step.Expected.Reservation, reservation) {
			return errors.New("declared reservation differs")
		}
	}
	return nil
}

func replayReclamationStep(table Table, step reclamationStep) (Table, error) {
	var err error
	switch step.Op {
	case "lookup":
		physical, present, lookupErr := table.Lookup(*step.Logical)
		err = lookupErr
		if physical != *step.Expected.Physical || present != *step.Expected.Present {
			return table, errors.New("runtime lookup differs")
		}
	case "begin", "begin_excluding":
		mask := []bool(nil)
		if step.Excluded != nil {
			mask = *step.Excluded
		}
		var reservation Reservation
		if step.Op == "begin" {
			table, reservation, err = table.Begin(*step.Logical)
		} else {
			table, reservation, err = table.BeginExcluding(*step.Logical, mask)
		}
		if err == nil && !sameFixtureReservation(step.Expected.Reservation, &reservation) {
			return table, errors.New("runtime reservation differs")
		}
	case "finish":
		disposition, ok := parseDisposition(*step.Disposition)
		if !ok {
			disposition = Disposition(255)
		}
		table, err = table.Finish(disposition)
	case "release_erased":
		table, err = table.ReleaseErased(*step.FirstPhysical, *step.PageCount)
	}
	if !errors.Is(err, errorFromReclamationFixture(step.Expected.Error)) {
		return table, fmt.Errorf("runtime error %v differs from declaration %v", err, step.Expected.Error)
	}
	if !reflect.DeepEqual(table.Snapshot(), runtimeSnapshotFromFixture(step.Expected.State)) {
		return table, errors.New("runtime complete state differs")
	}
	return table, nil
}

func errorFromReclamationFixture(code *string) error {
	if code == nil {
		return nil
	}
	switch *code {
	case "invalid_geometry":
		return ErrInvalidGeometry
	case "invalid_logical_page":
		return ErrInvalidLogicalPage
	case "reservation_pending":
		return ErrReservationPending
	case "no_free_page":
		return ErrNoFreePage
	case "no_reservation":
		return ErrNoReservation
	case "invalid_disposition":
		return ErrInvalidDisposition
	case "invalid_fence":
		return ErrInvalidFence
	case "invalid_extent":
		return ErrInvalidExtent
	case "ineligible_extent":
		return ErrIneligibleExtent
	default:
		return nil
	}
}

func FuzzParseReclamationFixture(f *testing.F) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-reclamation.json")
	if raw, err := os.ReadFile(path); err == nil {
		f.Add(raw)
	}
	f.Add([]byte(`{"schema":"bad","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		document, err := parseReclamationFixture(raw)
		if err != nil || validateReclamationModel(document) != nil {
			return
		}
		for _, testCase := range document.Cases {
			table, err := New(testCase.LogicalCount, testCase.PhysicalCount)
			if err != nil {
				t.Fatalf("model-accepted geometry refused: %v", err)
			}
			for _, step := range testCase.Steps {
				table, err = replayReclamationStep(table, step)
				if err != nil {
					t.Fatalf("model-accepted reclamation step refused: %v", err)
				}
			}
		}
	})
}

func FuzzReclamationHistory(f *testing.F) {
	f.Add([]byte{0, 0, 1, 0, 4, 1, 2, 0, 3, 1, 5, 0, 2, 1, 1, 0})
	f.Add([]byte{1, 1, 3, 7, 0, 2, 4, 0, 3, 2, 1, 1})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 2048 {
			return
		}
		checkReclamationHistory(t, raw)
	})
}

func checkReclamationHistory(t *testing.T, raw []byte) {
	t.Helper()
	model := modelInitial(2, 4)
	table, _ := New(2, 4)
	for offset := 0; offset+3 < len(raw) && offset < 128; offset += 4 {
		logical := int64(int(raw[offset+1])%4 - 1)
		first := int64(raw[offset+1]%6) - 1
		count := int64(raw[offset+2]%6) - 1
		switch raw[offset] % 5 {
		case 0:
			checkLookup(t, model, table, logical)
		case 1:
			table = compareBegin(t, model, table, logical)
			if next, ok := modelBegin(model, logical); ok {
				model = next
			}
		case 2:
			disposition := Disposition(raw[offset+3] % 4)
			table = compareFinish(t, model, table, disposition)
			if next, ok := modelFinish(model, disposition); ok {
				model = next
			}
		case 3:
			width := len(model.physical)
			mask := make([]bool, width)
			for index := range mask {
				mask[index] = raw[offset+3]&(1<<index) != 0
			}
			checkBeginExcluding(t, model, table, logical, mask)
			if next, ok := modelBeginExcluding(model, logical, mask); ok {
				model = next
				table, _, _ = table.BeginExcluding(logical, mask)
			}
		case 4:
			checkReleaseErased(t, model, table, first, count)
			if next, ok := modelReleaseErased(model, first, count); ok {
				model = next
				table, _ = table.ReleaseErased(first, count)
			}
		}
		checkRuntimeInvariants(t, table)
		checkModelInvariants(t, model)
	}
}
