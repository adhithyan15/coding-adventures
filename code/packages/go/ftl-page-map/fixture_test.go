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

const (
	fixtureSchema = "coding-adventures.database-media.ftl-page-map.v1"
	fixtureLimit  = 64 * 1024
	fixtureDepth  = 32
	fixtureCases  = 32
	fixtureSteps  = 64
)

type fixtureDocument struct {
	Schema string        `json:"schema"`
	Cases  []fixtureCase `json:"cases"`
}

type fixtureCase struct {
	Name          string        `json:"name"`
	LogicalCount  *int64        `json:"logical_count"`
	PhysicalCount *int64        `json:"physical_count"`
	Steps         []fixtureStep `json:"steps"`
}

type fixtureStep struct {
	Op          string             `json:"op"`
	Logical     *int64             `json:"logical,omitempty"`
	Disposition *string            `json:"disposition,omitempty"`
	Expected    fixtureExpectation `json:"expected"`
}

type fixtureExpectation struct {
	Error       *string             `json:"error,omitempty"`
	Physical    *int64              `json:"physical,omitempty"`
	Present     *bool               `json:"present,omitempty"`
	Reservation *fixtureReservation `json:"reservation,omitempty"`
	State       *fixtureSnapshot    `json:"state"`
}

type fixtureReservation struct {
	Logical  int64 `json:"logical"`
	Physical int64 `json:"physical"`
}

type fixtureSnapshot struct {
	LogicalToPhysical []int64             `json:"logical_to_physical"`
	PhysicalStatus    []string            `json:"physical_status"`
	Reservation       *fixtureReservation `json:"reservation"`
}

func TestCanonicalPageMapFixtureIsModelCheckedBeforeRuntime(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-map.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	document, err := parseFixture(raw)
	if err != nil {
		t.Fatalf("parse canonical fixture: %v", err)
	}
	if err := validateFixtureModel(document); err != nil {
		t.Fatalf("fixture is not accepted by independent model: %v", err)
	}
	for _, testCase := range document.Cases {
		table, err := New(*testCase.LogicalCount, *testCase.PhysicalCount)
		if err != nil {
			t.Fatalf("%s: New: %v", testCase.Name, err)
		}
		for index, step := range testCase.Steps {
			table, err = replayStep(table, step)
			if err != nil {
				t.Fatalf("%s step %d (%s): %v", testCase.Name, index, step.Op, err)
			}
		}
	}
}

func TestFixtureAdapterRejectsMalformedAndOversizedJSON(t *testing.T) {
	canonical, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-map.json"))
	if err != nil {
		t.Fatal(err)
	}
	duplicate := bytes.Replace(canonical, []byte(`"schema":`), []byte(`"schema":"duplicate","schema":`), 1)
	escapedDuplicate := bytes.Replace(canonical, []byte(`"schema":`), []byte(`"sch\u0065ma":"duplicate","schema":`), 1)
	invalidUTF8 := append(append([]byte(nil), canonical...), 0xff)
	truncated := canonical[:len(canonical)-2]
	tooDeep := []byte(strings.Repeat("[", fixtureDepth+1) + strings.Repeat("]", fixtureDepth+1))
	unknownField := bytes.Replace(canonical, []byte(`"schema":`), []byte(`"extra":true,"schema":`), 1)
	fractionalAddress := bytes.Replace(canonical, []byte(`"logical": 0`), []byte(`"logical": 0.5`), 1)
	unknownError := bytes.Replace(canonical, []byte(`"no_free_page"`), []byte(`"made_up_error"`), 1)
	for name, raw := range map[string][]byte{
		"duplicate key":       duplicate,
		"escaped duplicate":   escapedDuplicate,
		"invalid UTF-8":       invalidUTF8,
		"truncated":           truncated,
		"trailing value":      append(append([]byte(nil), canonical...), []byte(`{}`)...),
		"excessive depth":     tooDeep,
		"oversized":           bytes.Repeat([]byte{' '}, fixtureLimit+1),
		"unknown field":       unknownField,
		"fractional address":  fractionalAddress,
		"unknown error class": unknownError,
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := parseFixture(raw); err == nil {
				t.Fatal("malformed fixture was accepted")
			}
		})
	}
}

func TestWellFormedCorruptedExpectationFailsBeforeRuntime(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-map.json"))
	if err != nil {
		t.Fatal(err)
	}
	corrupted := bytes.Replace(raw, []byte(`"physical": 1`), []byte(`"physical": 0`), 1)
	if bytes.Equal(raw, corrupted) {
		t.Fatal("fixture mutation anchor was not found")
	}
	document, err := parseFixture(corrupted)
	if err != nil {
		t.Fatalf("corrupted fixture should remain syntactically valid: %v", err)
	}
	if err := validateFixtureModel(document); err == nil {
		t.Fatal("independent model accepted a corrupted expected result")
	}
}

func FuzzParseFixture(f *testing.F) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-map.json")
	if raw, err := os.ReadFile(path); err == nil {
		f.Add(raw)
	}
	f.Add([]byte(`{"schema":"bad","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		document, err := parseFixture(raw)
		if err != nil {
			return
		}
		if err := validateFixtureModel(document); err != nil {
			return
		}
		for _, testCase := range document.Cases {
			table, err := New(*testCase.LogicalCount, *testCase.PhysicalCount)
			if err != nil {
				t.Fatalf("model-accepted geometry rejected by runtime: %v", err)
			}
			for _, step := range testCase.Steps {
				table, err = replayStep(table, step)
				if err != nil {
					t.Fatalf("model-accepted fixture rejected at runtime: %v", err)
				}
			}
		}
	})
}

func FuzzPageMapHistory(f *testing.F) {
	f.Add([]byte{0, 0, 1, 0, 2, 0, 0, 1, 0, 2, 0, 1})
	f.Add([]byte{1, 1, 1, 2, 0, 0, 0, 3, 2, 1, 0})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 4096 {
			return
		}
		checkFuzzHistory(t, raw)
	})
}

func parseFixture(raw []byte) (fixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return fixtureDocument{}, errors.New("invalid fixture size or UTF-8")
	}
	if err := scanJSON(raw); err != nil {
		return fixtureDocument{}, err
	}
	if err := validatePageMapJSONKinds(raw); err != nil {
		return fixtureDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var document fixtureDocument
	if err := decoder.Decode(&document); err != nil {
		return fixtureDocument{}, err
	}
	var extra any
	if err := decoder.Decode(&extra); err != io.EOF {
		return fixtureDocument{}, errors.New("trailing JSON data")
	}
	if document.Schema != fixtureSchema || len(document.Cases) == 0 || len(document.Cases) > fixtureCases {
		return fixtureDocument{}, errors.New("invalid fixture schema or case count")
	}
	for _, testCase := range document.Cases {
		if testCase.Name == "" || testCase.LogicalCount == nil || testCase.PhysicalCount == nil || len(testCase.Steps) == 0 || len(testCase.Steps) > fixtureSteps {
			return fixtureDocument{}, errors.New("invalid fixture case shape")
		}
		if *testCase.LogicalCount < 1 || *testCase.LogicalCount > 64 || *testCase.PhysicalCount < 2 || *testCase.PhysicalCount > 128 || *testCase.PhysicalCount <= *testCase.LogicalCount {
			return fixtureDocument{}, errors.New("fixture geometry outside supported bounds")
		}
		for _, step := range testCase.Steps {
			if step.Expected.State == nil {
				return fixtureDocument{}, errors.New("fixture step lacks required state")
			}
			if !validExpectationShape(step) {
				return fixtureDocument{}, errors.New("fixture step has an invalid expected-result shape")
			}
			switch step.Op {
			case "lookup", "begin":
				if step.Logical == nil || step.Disposition != nil {
					return fixtureDocument{}, errors.New("fixture address operation has invalid fields")
				}
			case "finish":
				if step.Disposition == nil || step.Logical != nil {
					return fixtureDocument{}, errors.New("fixture finish has invalid fields")
				}
			default:
				return fixtureDocument{}, errors.New("unknown fixture operation")
			}
		}
	}
	return document, nil
}

func validateFixtureModel(document fixtureDocument) error {
	for _, testCase := range document.Cases {
		state := modelInitial(*testCase.LogicalCount, *testCase.PhysicalCount)
		for index, step := range testCase.Steps {
			var next modelState
			var resultPhysical int64
			var resultPresent bool
			var resultReservation *Reservation
			var expectedError error
			switch step.Op {
			case "lookup":
				logical := *step.Logical
				if logical < 0 || logical >= state.logicalCount {
					expectedError = ErrInvalidLogicalPage
				} else if physical := state.logicalMap[int(logical)]; physical != noMapping {
					resultPhysical, resultPresent = physical, true
				}
				next = state
			case "begin":
				logical := *step.Logical
				var accepted bool
				next, accepted = modelBegin(state, logical)
				if !accepted {
					switch {
					case logical < 0 || logical >= state.logicalCount:
						expectedError = ErrInvalidLogicalPage
					case state.pending != nil:
						expectedError = ErrReservationPending
					default:
						expectedError = ErrNoFreePage
					}
				} else {
					reservation := *next.pending
					resultReservation = &reservation
				}
			case "finish":
				disposition, ok := parseDisposition(*step.Disposition)
				if !ok {
					if state.pending == nil {
						expectedError = ErrNoReservation
					} else {
						expectedError = ErrInvalidDisposition
					}
					next = state
				} else {
					var accepted bool
					next, accepted = modelFinish(state, disposition)
					if !accepted {
						expectedError = ErrNoReservation
					}
				}
			}
			if err := compareFixtureExpected(step.Expected, next, resultPhysical, resultPresent, resultReservation, expectedError); err != nil {
				return fmt.Errorf("case %q step %d: %w", testCase.Name, index, err)
			}
			state = next
		}
	}
	return nil
}

func compareFixtureExpected(expected fixtureExpectation, state modelState, physical int64, present bool, reservation *Reservation, expectedError error) error {
	if err := compareError(expected.Error, expectedError); err != nil {
		return err
	}
	if expected.State == nil || !reflect.DeepEqual(*expected.State, fixtureSnapshotOf(state)) {
		return errors.New("declared state does not match independent model")
	}
	if expectedError == nil {
		if expected.Physical != nil && *expected.Physical != physical {
			return errors.New("declared lookup address mismatch")
		}
		if expected.Present != nil && *expected.Present != present {
			return errors.New("declared lookup presence mismatch")
		}
		if !sameFixtureReservation(expected.Reservation, reservation) {
			return errors.New("declared reservation mismatch")
		}
	}
	return nil
}

func replayStep(table Table, step fixtureStep) (Table, error) {
	var actual Snapshot
	var gotPhysical int64
	var gotPresent bool
	var gotReservation *Reservation
	var err error
	switch step.Op {
	case "lookup":
		gotPhysical, gotPresent, err = table.Lookup(*step.Logical)
		actual = table.Snapshot()
	case "begin":
		var reservation Reservation
		table, reservation, err = table.Begin(*step.Logical)
		if err == nil {
			gotReservation = &reservation
		}
		actual = table.Snapshot()
	case "finish":
		disposition, _ := parseDisposition(*step.Disposition)
		table, err = table.Finish(disposition)
		actual = table.Snapshot()
	}
	if !errors.Is(err, errorFromFixture(step.Expected.Error)) {
		return table, fmt.Errorf("runtime error %v differs from fixture %v", err, step.Expected.Error)
	}
	if step.Op == "lookup" && (gotPhysical != *step.Expected.Physical || gotPresent != *step.Expected.Present) {
		return table, errors.New("runtime lookup result differs from fixture")
	}
	if step.Op == "begin" && err == nil && !sameFixtureReservation(step.Expected.Reservation, gotReservation) {
		return table, errors.New("runtime reservation differs from fixture")
	}
	if !reflect.DeepEqual(actual, runtimeSnapshotFromFixture(step.Expected.State)) {
		return table, errors.New("runtime state differs from fixture")
	}
	return table, nil
}

func validExpectationShape(step fixtureStep) bool {
	expected := step.Expected
	if expected.Error != nil {
		declared := errorFromFixture(expected.Error)
		if errorCode(declared) == "unknown" {
			return false
		}
	}
	switch step.Op {
	case "lookup":
		return expected.Physical != nil && expected.Present != nil && expected.Reservation == nil
	case "begin":
		if expected.Physical != nil || expected.Present != nil {
			return false
		}
		return (expected.Error == nil) == (expected.Reservation != nil)
	case "finish":
		return expected.Physical == nil && expected.Present == nil && expected.Reservation == nil
	default:
		return false
	}
}

func parseDisposition(value string) (Disposition, bool) {
	switch value {
	case "media_ready":
		return MediaReady, true
	case "abort_fresh":
		return AbortFresh, true
	case "abort_spent":
		return AbortSpent, true
	default:
		return Disposition(255), false
	}
}

func compareError(declared *string, expected error) error {
	if expected == nil {
		if declared != nil {
			return fmt.Errorf("unexpected declared error %q", *declared)
		}
		return nil
	}
	if declared == nil || *declared != errorCode(expected) {
		return fmt.Errorf("declared error %v, want %q", declared, errorCode(expected))
	}
	return nil
}

func errorFromFixture(code *string) error {
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
	default:
		return errors.New("unknown fixture error")
	}
}

func errorCode(err error) string {
	switch {
	case errors.Is(err, ErrInvalidGeometry):
		return "invalid_geometry"
	case errors.Is(err, ErrInvalidLogicalPage):
		return "invalid_logical_page"
	case errors.Is(err, ErrReservationPending):
		return "reservation_pending"
	case errors.Is(err, ErrNoFreePage):
		return "no_free_page"
	case errors.Is(err, ErrNoReservation):
		return "no_reservation"
	case errors.Is(err, ErrInvalidDisposition):
		return "invalid_disposition"
	default:
		return "unknown"
	}
}

func fixtureSnapshotOf(state modelState) fixtureSnapshot {
	statuses := make([]string, len(state.physical))
	for index, status := range state.physical {
		statuses[index] = []string{"free", "reserved", "live", "stale"}[status]
	}
	snapshot := fixtureSnapshot{LogicalToPhysical: append([]int64(nil), state.logicalMap...), PhysicalStatus: statuses}
	if state.pending != nil {
		reservation := fixtureReservation{Logical: state.pending.Logical, Physical: state.pending.Physical}
		snapshot.Reservation = &reservation
	}
	return snapshot
}

func runtimeSnapshotFromFixture(snapshot *fixtureSnapshot) Snapshot {
	if snapshot == nil {
		return Snapshot{}
	}
	statuses := make([]PhysicalStatus, len(snapshot.PhysicalStatus))
	for index, status := range snapshot.PhysicalStatus {
		switch status {
		case "free":
			statuses[index] = Free
		case "reserved":
			statuses[index] = Reserved
		case "live":
			statuses[index] = Live
		case "stale":
			statuses[index] = Stale
		}
	}
	result := Snapshot{LogicalToPhysical: append([]int64(nil), snapshot.LogicalToPhysical...), PhysicalStatus: statuses}
	if snapshot.Reservation != nil {
		reservation := Reservation{Logical: snapshot.Reservation.Logical, Physical: snapshot.Reservation.Physical}
		result.Reservation = &reservation
	}
	return result
}

func sameFixtureReservation(expected *fixtureReservation, actual *Reservation) bool {
	if expected == nil || actual == nil {
		return expected == nil && actual == nil
	}
	return expected.Logical == actual.Logical && expected.Physical == actual.Physical
}

func scanJSON(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
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
	delimiter, ok := token.(json.Delim)
	if !ok {
		return nil
	}
	switch delimiter {
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
			child, err := decoder.Token()
			if err != nil {
				return err
			}
			if err := scanValue(decoder, child, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim('}') {
			return errors.New("unterminated object")
		}
	case '[':
		for decoder.More() {
			child, err := decoder.Token()
			if err != nil {
				return err
			}
			if err := scanValue(decoder, child, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim(']') {
			return errors.New("unterminated array")
		}
	default:
		return errors.New("unexpected JSON delimiter")
	}
	return nil
}

func checkFuzzHistory(t *testing.T, raw []byte) {
	t.Helper()
	model := modelInitial(2, 4)
	table, _ := New(2, 4)
	for index := 0; index+1 < len(raw) && index < 128; index += 2 {
		logical := int64(int(raw[index])%4 - 1)
		switch raw[index+1] % 5 {
		case 0:
			checkLookup(t, model, table, logical)
		case 1:
			table = compareBegin(t, model, table, logical)
			if next, ok := modelBegin(model, logical); ok {
				model = next
			}
		default:
			disposition := Disposition((raw[index+1] - 2) % 5)
			table = compareFinish(t, model, table, disposition)
			if next, ok := modelFinish(model, disposition); ok {
				model = next
			}
		}
		checkRuntimeInvariants(t, table)
		checkModelInvariants(t, model)
	}
}
