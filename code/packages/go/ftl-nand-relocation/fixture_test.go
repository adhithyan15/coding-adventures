package ftlnandrelocation

import (
	"bytes"
	"encoding/hex"
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
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

const (
	relocationFixtureSchema = "coding-adventures.database-media.ftl-nand-relocation.v1"
	fixtureLimit            = 64 * 1024
	fixtureDepth            = 32
	fixtureCaseLimit        = 16
	fixtureStepLimit        = 64
)

type fixtureDocument struct {
	Schema string        `json:"schema"`
	Cases  []fixtureCase `json:"cases"`
}

type fixtureCase struct {
	Name          string        `json:"name"`
	LogicalCount  *int64        `json:"logical_count"`
	PhysicalCount *int64        `json:"physical_count"`
	PageSize      *int          `json:"page_size"`
	Steps         []fixtureStep `json:"steps"`
}

type fixtureStep struct {
	Op         string             `json:"op"`
	Logical    *int64             `json:"logical,omitempty"`
	PayloadHex *string            `json:"payload_hex,omitempty"`
	Cut        *fixtureCut        `json:"cut,omitempty"`
	Expected   fixtureExpectation `json:"expected"`
}

type fixtureCut struct {
	Kind        *string `json:"kind"`
	PrefixBytes *int    `json:"prefix_bytes,omitempty"`
}

type fixtureExpectation struct {
	Error   *string         `json:"error,omitempty"`
	DataHex *string         `json:"data_hex,omitempty"`
	Present *bool           `json:"present,omitempty"`
	Receipt *fixtureReceipt `json:"receipt,omitempty"`
	State   *fixtureState   `json:"state"`
}

type fixtureReceipt struct {
	Physical     *int64 `json:"physical"`
	Published    *bool  `json:"published"`
	Acknowledged *bool  `json:"acknowledged"`
}

type fixtureState struct {
	Map   *fixtureMap    `json:"map"`
	Pages *[]fixturePage `json:"pages"`
}

type fixtureMap struct {
	LogicalToPhysical *[]int64        `json:"logical_to_physical"`
	PhysicalStatus    *[]string       `json:"physical_status"`
	Reservation       json.RawMessage `json:"reservation"`
}

type fixturePage struct {
	BytesHex   *string `json:"bytes_hex"`
	Programmed *bool   `json:"programmed"`
}

func TestCanonicalRelocationFixtureIsModelCheckedBeforeRuntime(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-nand-relocation.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	document, err := parseRelocationFixture(raw)
	if err != nil {
		t.Fatalf("parse canonical fixture: %v", err)
	}
	if err := validateRelocationFixtureModel(document); err != nil {
		t.Fatalf("fixture does not match independent model: %v", err)
	}
	for _, testCase := range document.Cases {
		device, err := New(*testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize)
		if err != nil {
			t.Fatalf("%s: New: %v", testCase.Name, err)
		}
		for index, step := range testCase.Steps {
			device, err = replayFixtureStep(device, step)
			if err != nil {
				t.Fatalf("%s step %d: %v", testCase.Name, index, err)
			}
		}
	}
}

func TestRelocationFixtureAdapterRejectsMalformedVectors(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-nand-relocation.json"))
	if err != nil {
		t.Fatal(err)
	}
	duplicate := bytes.Replace(raw, []byte(`"schema":`), []byte(`"schema":"duplicate","schema":`), 1)
	escapedDuplicate := bytes.Replace(raw, []byte(`"schema":`), []byte(`"sch\u0065ma":"duplicate","schema":`), 1)
	unknownField := bytes.Replace(raw, []byte(`"schema":`), []byte(`"unknown":true,"schema":`), 1)
	invalidUTF8 := append(append([]byte(nil), raw...), 0xff)
	truncated := raw[:len(raw)-2]
	deep := []byte(strings.Repeat("[", fixtureDepth+1) + strings.Repeat("]", fixtureDepth+1))
	fractional := bytes.Replace(raw, []byte(`"logical": 0`), []byte(`"logical": 0.5`), 1)
	badHex := bytes.Replace(raw, []byte(`"payload_hex": "1020"`), []byte(`"payload_hex": "1z20"`), 1)
	wrongDimensions := bytes.Replace(raw, []byte("\"logical_to_physical\": [\n                  0\n                ]"), []byte(`"logical_to_physical": []`), 1)
	wrongPageWidth := bytes.Replace(raw, []byte(`"bytes_hex": "1020"`), []byte(`"bytes_hex": "10"`), 1)
	for name, vector := range map[string][]byte{
		"duplicate key": duplicate, "escaped duplicate": escapedDuplicate,
		"unknown field": unknownField, "invalid UTF-8": invalidUTF8,
		"truncated": truncated, "trailing data": append(append([]byte(nil), raw...), []byte(`{}`)...),
		"excessive nesting": deep, "fractional address": fractional,
		"malformed hex": badHex, "wrong state dimensions": wrongDimensions,
		"wrong page width": wrongPageWidth, "oversized": bytes.Repeat([]byte{' '}, fixtureLimit+1),
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := parseRelocationFixture(vector); err == nil {
				t.Fatal("malformed vector was accepted")
			}
		})
	}
}

func TestRelocationFixtureRejectsNullLogicalMapSlotAndCaseAlias(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-nand-relocation.json"))
	if err != nil {
		t.Fatal(err)
	}
	variants := map[string][]byte{
		"null logical map slot": bytes.Replace(
			raw,
			[]byte("\"logical_to_physical\": [\n                  0\n                ]"),
			[]byte("\"logical_to_physical\": [\n                  null\n                ]"),
			1,
		),
		"case-aliased Logical key": bytes.Replace(raw, []byte(`"logical": 0`), []byte(`"Logical": 0`), 1),
	}
	for name, variant := range variants {
		t.Run(name, func(t *testing.T) {
			if bytes.Equal(raw, variant) {
				t.Fatal("fixture mutation anchor was absent")
			}
			if _, err := parseRelocationFixture(variant); err == nil {
				t.Fatal("case/token-invalid relocation fixture was accepted")
			}
		})
	}
}

func TestWellTypedCorruptionIsRefusedByIndependentModelBeforeRuntime(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-nand-relocation.json"))
	if err != nil {
		t.Fatal(err)
	}
	corrupted := bytes.Replace(raw, []byte(`"physical": 2`), []byte(`"physical": 1`), 1)
	if bytes.Equal(raw, corrupted) {
		t.Fatal("receipt mutation anchor was absent")
	}
	document, err := parseRelocationFixture(corrupted)
	if err != nil {
		t.Fatalf("mutation should remain well formed: %v", err)
	}
	if err := validateRelocationFixtureModel(document); err == nil {
		t.Fatal("independent model accepted corrupted fixture expectation")
	}
}

func FuzzParseRelocationFixture(f *testing.F) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-nand-relocation.json")
	if raw, err := os.ReadFile(path); err == nil {
		f.Add(raw)
	}
	f.Add([]byte(`{"schema":"invalid","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		document, err := parseRelocationFixture(raw)
		if err != nil || validateRelocationFixtureModel(document) != nil {
			return
		}
		for _, testCase := range document.Cases {
			device, err := New(*testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize)
			if err != nil {
				t.Fatalf("model-accepted geometry rejected: %v", err)
			}
			for _, step := range testCase.Steps {
				device, err = replayFixtureStep(device, step)
				if err != nil {
					t.Fatalf("model-accepted vector disagrees with runtime: %v", err)
				}
			}
		}
	})
}

func FuzzRelocationHistory(f *testing.F) {
	f.Add([]byte{0, 0, 0x10, 0x20, 3, 0, 1, 0xff, 0xff, 1, 0, 0, 0x30, 0x40, 2})
	f.Add([]byte{2, 1, 0xff, 0xff, 0, 0, 0, 0x01, 0x02, 3})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 4096 {
			return
		}
		checkFuzzRelocationHistory(t, raw)
	})
}

func parseRelocationFixture(raw []byte) (fixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return fixtureDocument{}, errors.New("invalid fixture size or UTF-8")
	}
	if err := scanRelocationJSON(raw); err != nil {
		return fixtureDocument{}, err
	}
	if err := validateRelocationJSONKinds(raw); err != nil {
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
	if document.Schema != relocationFixtureSchema || len(document.Cases) == 0 || len(document.Cases) > fixtureCaseLimit {
		return fixtureDocument{}, errors.New("unknown schema or invalid case count")
	}
	for _, testCase := range document.Cases {
		if testCase.Name == "" || testCase.LogicalCount == nil || testCase.PhysicalCount == nil || testCase.PageSize == nil || len(testCase.Steps) == 0 || len(testCase.Steps) > fixtureStepLimit {
			return fixtureDocument{}, errors.New("invalid fixture case shape")
		}
		if *testCase.LogicalCount < 1 || *testCase.LogicalCount > 64 || *testCase.PhysicalCount < 2 || *testCase.PhysicalCount > 128 || *testCase.PhysicalCount <= *testCase.LogicalCount || *testCase.PageSize < 1 || *testCase.PageSize > maxPageSize {
			return fixtureDocument{}, errors.New("geometry outside supported bounds")
		}
		for _, step := range testCase.Steps {
			if step.Expected.State == nil || !validFixtureExpectation(step, *testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize) {
				return fixtureDocument{}, errors.New("invalid expected result shape")
			}
			switch step.Op {
			case "read":
				if step.Logical == nil || step.PayloadHex != nil || step.Cut != nil {
					return fixtureDocument{}, errors.New("invalid read operation fields")
				}
			case "write":
				if step.Logical == nil || step.PayloadHex == nil || step.Cut == nil || !validCutShape(step.Cut) {
					return fixtureDocument{}, errors.New("invalid write operation fields")
				}
				payload, err := decodeFixtureHex(*step.PayloadHex)
				if err != nil || len(payload) > maxPageSize {
					return fixtureDocument{}, errors.New("invalid payload hex or excessive payload")
				}
			default:
				return fixtureDocument{}, errors.New("unknown fixture operation")
			}
		}
	}
	return document, nil
}

// validateRelocationJSONKinds checks case-sensitive keys and JSON token kinds
// before encoding/json can apply its case-insensitive struct-field matching.
func validateRelocationJSONKinds(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return err
	}
	root, ok := value.(map[string]any)
	if !ok || !blockExactKeys(root, []string{"schema", "cases"}, nil) {
		return errors.New("invalid relocation fixture root keys")
	}
	if _, ok := root["schema"].(string); !ok {
		return errors.New("schema must be a string")
	}
	cases, ok := root["cases"].([]any)
	if !ok {
		return errors.New("cases must be an array")
	}
	for _, item := range cases {
		fixtureCase, ok := item.(map[string]any)
		if !ok || !blockExactKeys(fixtureCase, []string{"name", "logical_count", "physical_count", "page_size", "steps"}, nil) {
			return errors.New("invalid relocation case keys")
		}
		if _, ok := fixtureCase["name"].(string); !ok || !blockInteger(fixtureCase["logical_count"]) ||
			!blockInteger(fixtureCase["physical_count"]) || !blockInteger(fixtureCase["page_size"]) {
			return errors.New("invalid relocation case field token kind")
		}
		steps, ok := fixtureCase["steps"].([]any)
		if !ok {
			return errors.New("steps must be an array")
		}
		for _, stepValue := range steps {
			step, ok := stepValue.(map[string]any)
			if !ok {
				return errors.New("step must be an object")
			}
			op, ok := step["op"].(string)
			if !ok {
				return errors.New("op must be a string")
			}
			var required []string
			switch op {
			case "read":
				required = []string{"op", "logical", "expected"}
				if !blockInteger(step["logical"]) {
					return errors.New("read logical address must be an integer")
				}
			case "write":
				required = []string{"op", "logical", "payload_hex", "cut", "expected"}
				if !blockInteger(step["logical"]) {
					return errors.New("write logical address must be an integer")
				}
				if _, ok := step["payload_hex"].(string); !ok {
					return errors.New("payload_hex must be text")
				}
				if err := validateRelocationCutKinds(step["cut"]); err != nil {
					return err
				}
			default:
				return errors.New("unknown relocation operation")
			}
			if !blockExactKeys(step, required, nil) {
				return errors.New("unknown, missing or incorrectly-cased operation key")
			}
			if err := validateRelocationExpectedKinds(step["expected"]); err != nil {
				return err
			}
		}
	}
	return nil
}

func validateRelocationCutKinds(value any) error {
	cut, ok := value.(map[string]any)
	if !ok || !blockExactKeys(cut, []string{"kind"}, []string{"prefix_bytes"}) {
		return errors.New("invalid relocation cut keys")
	}
	if _, ok := cut["kind"].(string); !ok {
		return errors.New("cut kind must be text")
	}
	if prefix, ok := cut["prefix_bytes"]; ok && !blockInteger(prefix) {
		return errors.New("prefix_bytes must be an integer")
	}
	return nil
}

func validateRelocationExpectedKinds(value any) error {
	expected, ok := value.(map[string]any)
	if !ok || !blockExactKeys(expected, []string{"state"}, []string{"error", "data_hex", "present", "receipt"}) {
		return errors.New("invalid relocation expected keys")
	}
	if errorValue, exists := expected["error"]; exists {
		if _, ok := errorValue.(string); !ok {
			return errors.New("expected error must be text")
		}
	}
	if dataValue, exists := expected["data_hex"]; exists {
		if _, ok := dataValue.(string); !ok {
			return errors.New("data_hex must be text")
		}
	}
	if presentValue, exists := expected["present"]; exists {
		if _, ok := presentValue.(bool); !ok {
			return errors.New("present must be boolean")
		}
	}
	if receiptValue, exists := expected["receipt"]; exists {
		receipt, ok := receiptValue.(map[string]any)
		if !ok || !blockExactKeys(receipt, []string{"physical", "published", "acknowledged"}, nil) || !blockInteger(receipt["physical"]) {
			return errors.New("invalid relocation receipt token kinds")
		}
		if _, ok := receipt["published"].(bool); !ok {
			return errors.New("published must be boolean")
		}
		if _, ok := receipt["acknowledged"].(bool); !ok {
			return errors.New("acknowledged must be boolean")
		}
	}
	state, ok := expected["state"].(map[string]any)
	if !ok || !blockExactKeys(state, []string{"map", "pages"}, nil) {
		return errors.New("invalid relocation state keys")
	}
	if err := validateBlockMapKinds(state["map"]); err != nil {
		return err
	}
	pages, ok := state["pages"].([]any)
	if !ok {
		return errors.New("pages must be an array")
	}
	for _, value := range pages {
		page, ok := value.(map[string]any)
		if !ok || !blockExactKeys(page, []string{"bytes_hex", "programmed"}, nil) {
			return errors.New("invalid relocation page keys")
		}
		if _, ok := page["bytes_hex"].(string); !ok {
			return errors.New("bytes_hex must be text")
		}
		if _, ok := page["programmed"].(bool); !ok {
			return errors.New("programmed must be boolean")
		}
	}
	return nil
}

func validFixtureExpectation(step fixtureStep, logicalCount, physicalCount int64, pageSize int) bool {
	expected := step.Expected
	if expected.Error != nil && !knownRelocationError(*expected.Error) {
		return false
	}
	if expected.State.Map == nil || expected.State.Map.LogicalToPhysical == nil || expected.State.Map.PhysicalStatus == nil || expected.State.Pages == nil {
		return false
	}
	if len(expected.State.Map.Reservation) == 0 || string(bytes.TrimSpace(expected.State.Map.Reservation)) != "null" {
		return false
	}
	if len(*expected.State.Map.LogicalToPhysical) != int(logicalCount) || len(*expected.State.Map.PhysicalStatus) != int(physicalCount) || len(*expected.State.Pages) != int(physicalCount) {
		return false
	}
	for _, status := range *expected.State.Map.PhysicalStatus {
		if status != "free" && status != "live" && status != "stale" {
			return false
		}
	}
	for _, page := range *expected.State.Pages {
		if page.BytesHex == nil || page.Programmed == nil {
			return false
		}
		pageBytes, err := decodeFixtureHex(*page.BytesHex)
		if err != nil || len(pageBytes) != pageSize {
			return false
		}
	}
	switch step.Op {
	case "read":
		if expected.DataHex == nil || expected.Present == nil || expected.Receipt != nil {
			return false
		}
		data, err := decodeFixtureHex(*expected.DataHex)
		return err == nil && ((*expected.Present && len(data) == pageSize) || (!*expected.Present && len(data) == 0))
	case "write":
		if expected.DataHex != nil || expected.Present != nil || ((expected.Error == nil) != (expected.Receipt != nil)) {
			return false
		}
		return expected.Receipt == nil || (expected.Receipt.Physical != nil && expected.Receipt.Published != nil && expected.Receipt.Acknowledged != nil)
	default:
		return false
	}
}

func validCutShape(cut *fixtureCut) bool {
	if cut == nil {
		return false
	}
	if cut.Kind == nil {
		return false
	}
	switch *cut.Kind {
	case "before_accept", "after_media", "complete":
		return cut.PrefixBytes == nil
	case "after_prefix":
		return cut.PrefixBytes != nil
	default:
		return cut.PrefixBytes == nil
	}
}

func validateRelocationFixtureModel(document fixtureDocument) error {
	for _, testCase := range document.Cases {
		state := referenceInitial(*testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize)
		for index, step := range testCase.Steps {
			var next referenceState
			var receipt modelReceipt
			var readData []byte
			var readPresent bool
			var expectedError string
			if step.Op == "write" {
				payload, err := decodeFixtureHex(*step.PayloadHex)
				if err != nil {
					return fmt.Errorf("case %q step %d: %w", testCase.Name, index, err)
				}
				next, receipt, expectedError = referenceWrite(state, *step.Logical, payload, fixtureCutToModel(step.Cut))
			} else {
				readData, readPresent, expectedError = referenceRead(state, *step.Logical)
				next = state
			}
			if fixtureError(step.Expected.Error) != expectedError {
				return fmt.Errorf("case %q step %d declares error %v, model predicts %q", testCase.Name, index, step.Expected.Error, expectedError)
			}
			if expectedError == "" && step.Op == "write" {
				declared := step.Expected.Receipt
				if declared == nil || *declared.Physical != receipt.physical || *declared.Published != receipt.published || *declared.Acknowledged != receipt.acknowledged {
					return fmt.Errorf("case %q step %d receipt disagrees with independent model", testCase.Name, index)
				}
			}
			if step.Op == "read" {
				data, err := decodeFixtureHex(*step.Expected.DataHex)
				if err != nil || !bytes.Equal(data, readData) || *step.Expected.Present != readPresent {
					return fmt.Errorf("case %q step %d read declaration disagrees with model", testCase.Name, index)
				}
			}
			if !reflect.DeepEqual(*step.Expected.State, fixtureStateOf(next)) {
				return fmt.Errorf("case %q step %d state disagrees with independent model", testCase.Name, index)
			}
			state = next
		}
	}
	return nil
}

func replayFixtureStep(device Device, step fixtureStep) (Device, error) {
	var next Device
	var receipt Receipt
	var data []byte
	var present bool
	var err error
	if step.Op == "write" {
		payload, decodeErr := decodeFixtureHex(*step.PayloadHex)
		if decodeErr != nil {
			return device, decodeErr
		}
		next, receipt, err = device.Write(*step.Logical, payload, runtimeFixtureCut(step.Cut))
	} else {
		next = device
		data, present, err = device.Read(*step.Logical)
	}
	if errorCode(err) != fixtureError(step.Expected.Error) {
		return device, fmt.Errorf("runtime error %q differs from fixture %q", errorCode(err), fixtureError(step.Expected.Error))
	}
	if step.Op == "write" && err == nil {
		want := step.Expected.Receipt
		if want == nil || receipt != (Receipt{Physical: *want.Physical, Published: *want.Published, Acknowledged: *want.Acknowledged}) {
			return device, errors.New("runtime receipt differs from fixture")
		}
	}
	if step.Op == "read" {
		wantData, decodeErr := decodeFixtureHex(*step.Expected.DataHex)
		if decodeErr != nil || present != *step.Expected.Present || !bytes.Equal(data, wantData) {
			return device, errors.New("runtime read differs from fixture")
		}
	}
	if !reflect.DeepEqual(next.Snapshot(), runtimeFixtureSnapshot(step.Expected.State)) {
		return device, errors.New("runtime state differs from fixture")
	}
	return next, nil
}

func fixtureCutToModel(cut *fixtureCut) modelCut {
	kind := 255
	switch *cut.Kind {
	case "before_accept":
		kind = cutBeforeAccept
	case "after_prefix":
		kind = cutAfterPrefix
	case "after_media":
		kind = cutAfterMedia
	case "complete":
		kind = cutComplete
	}
	prefix := 0
	if cut.PrefixBytes != nil {
		prefix = *cut.PrefixBytes
	}
	return modelCut{kind: kind, prefix: prefix}
}

func runtimeFixtureCut(cut *fixtureCut) Cut {
	model := fixtureCutToModel(cut)
	return runtimeCut(model)
}

func fixtureStateOf(state referenceState) fixtureState {
	statuses := make([]string, len(state.status))
	pages := make([]fixturePage, len(state.pages))
	for index, status := range state.status {
		switch status {
		case modelFree:
			statuses[index] = "free"
		case modelLive:
			statuses[index] = "live"
		case modelStale:
			statuses[index] = "stale"
		}
		bytesHex := hex.EncodeToString(state.pages[index])
		programmed := state.programmed[index]
		pages[index] = fixturePage{BytesHex: &bytesHex, Programmed: &programmed}
	}
	reservation := json.RawMessage("null")
	logicalMap := append([]int64(nil), state.mapping...)
	return fixtureState{
		Map:   &fixtureMap{LogicalToPhysical: &logicalMap, PhysicalStatus: &statuses, Reservation: reservation},
		Pages: &pages,
	}
}

func runtimeFixtureSnapshot(state *fixtureState) Snapshot {
	logicalMap := append([]int64(nil), (*state.Map.LogicalToPhysical)...)
	statuses := make([]ftlpagemap.PhysicalStatus, len(*state.Map.PhysicalStatus))
	for index, status := range *state.Map.PhysicalStatus {
		switch status {
		case "free":
			statuses[index] = ftlpagemap.Free
		case "live":
			statuses[index] = ftlpagemap.Live
		case "stale":
			statuses[index] = ftlpagemap.Stale
		}
	}
	pages := make([]nandpageprogramcuts.PageState, len(*state.Pages))
	for index, page := range *state.Pages {
		pageBytes, _ := hex.DecodeString(*page.BytesHex)
		pages[index] = nandpageprogramcuts.PageState{Bytes: pageBytes, Programmed: *page.Programmed}
	}
	return Snapshot{Map: ftlpagemap.Snapshot{LogicalToPhysical: logicalMap, PhysicalStatus: statuses}, Pages: pages}
}

func decodeFixtureHex(value string) ([]byte, error) {
	if len(value)%2 != 0 || strings.ToLower(value) != value {
		return nil, errors.New("hex bytes must be lowercase and even-width")
	}
	return hex.DecodeString(value)
}

func fixtureError(value *string) string {
	if value == nil {
		return ""
	}
	return *value
}

func knownRelocationError(value string) bool {
	switch value {
	case "invalid_geometry", "invalid_logical_page", "no_free_page", "invalid_page_size", "wrong_payload_length", "invalid_cut", "invalid_page_state", "already_programmed":
		return true
	default:
		return false
	}
}

func scanRelocationJSON(raw []byte) error {
	if err := validateSurrogateEscapes(raw); err != nil {
		return err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	if err := scanRelocationValue(decoder, token, 0); err != nil {
		return err
	}
	if _, err := decoder.Token(); err != io.EOF {
		return errors.New("trailing JSON token")
	}
	return nil
}

func scanRelocationValue(decoder *json.Decoder, token json.Token, depth int) error {
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
				return errors.New("invalid or duplicate decoded object key")
			}
			seen[key] = true
			child, err := decoder.Token()
			if err != nil {
				return err
			}
			if err := scanRelocationValue(decoder, child, depth+1); err != nil {
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
			if err := scanRelocationValue(decoder, child, depth+1); err != nil {
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

func checkFuzzRelocationHistory(t *testing.T, raw []byte) {
	t.Helper()
	state := referenceInitial(2, 5, 2)
	device, _ := New(2, 5, 2)
	for index, operation := 0, 0; index+3 < len(raw) && operation < 128; index, operation = index+4, operation+1 {
		logical := int64(int(raw[index])%4 - 1)
		payload := []byte{raw[index+1], raw[index+2]}
		kind := int(raw[index+3] % 5)
		prefix := int(raw[index+3] / 5 % 4)
		device, state = compareTransition(t, device, state, logical, payload, modelCut{kind: kind, prefix: prefix})
		assertDeviceEquals(t, device, state)
		assertRead(t, device, state, logical)
	}
}
