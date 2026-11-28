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

	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
)

const blockFixtureSchema = "coding-adventures.database-media.ftl-block-reclamation.v1"

type blockFixtureDocument struct {
	Schema string             `json:"schema"`
	Cases  []blockFixtureCase `json:"cases"`
}
type blockFixtureCase struct {
	Name               string             `json:"name"`
	LogicalCount       *int64             `json:"logical_count"`
	PhysicalCount      *int64             `json:"physical_count"`
	PageSize           *int               `json:"page_size"`
	PagesPerBlock      *int64             `json:"pages_per_block"`
	MaxCompletedErases *uint64            `json:"max_completed_erases"`
	Steps              []blockFixtureStep `json:"steps"`
}
type blockFixtureStep struct {
	Op         string               `json:"op"`
	Logical    *int64               `json:"logical,omitempty"`
	PayloadHex *string              `json:"payload_hex,omitempty"`
	ProgramCut *fixtureCut          `json:"cut,omitempty"`
	Block      *int64               `json:"block,omitempty"`
	EraseCut   *blockFixtureCut     `json:"erase_cut,omitempty"`
	Expected   blockFixtureExpected `json:"expected"`
}
type blockFixtureCut struct {
	Kind        *string `json:"kind"`
	PrefixPages *int    `json:"prefix_pages,omitempty"`
}
type blockFixtureEraseReceipt struct {
	Block        *int64 `json:"block"`
	Released     *bool  `json:"released"`
	Acknowledged *bool  `json:"acknowledged"`
}
type blockFixtureExpected struct {
	Error        *string                   `json:"error,omitempty"`
	DataHex      *string                   `json:"data_hex,omitempty"`
	Present      *bool                     `json:"present,omitempty"`
	WriteReceipt *fixtureReceipt           `json:"write_receipt,omitempty"`
	EraseReceipt *blockFixtureEraseReceipt `json:"erase_receipt,omitempty"`
	State        *blockFixtureState        `json:"state"`
}
type blockFixtureState struct {
	Map                *fixtureMap                  `json:"map"`
	Pages              *[]fixturePage               `json:"pages"`
	PagesPerBlock      *int64                       `json:"pages_per_block"`
	MaxCompletedErases *uint64                      `json:"max_completed_erases"`
	Blocks             *[]blockFixtureBlockSnapshot `json:"blocks"`
}
type blockFixtureBlockSnapshot struct {
	CompletedErases *uint64 `json:"completed_erases"`
	IncompleteErase *bool   `json:"incomplete_erase"`
}

func TestCanonicalBlockReclamationFixtureModelFirstThenRuntime(t *testing.T) {
	raw := readBlockFixture(t)
	document, err := parseBlockFixture(raw)
	if err != nil {
		t.Fatalf("parse block fixture: %v", err)
	}
	if err := validateBlockFixtureModel(document); err != nil {
		t.Fatalf("model rejected block fixture: %v", err)
	}
	if err := replayBlockFixtureDocument(document); err != nil {
		t.Fatal(err)
	}
}

func replayBlockFixtureDocument(document blockFixtureDocument) error {
	for _, testCase := range document.Cases {
		device, err := NewWithEraseBlocks(*testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize, *testCase.PagesPerBlock, *testCase.MaxCompletedErases)
		if err != nil {
			return fmt.Errorf("%s: constructor: %w", testCase.Name, err)
		}
		for index, step := range testCase.Steps {
			device, err = replayBlockFixtureStep(device, step)
			if err != nil {
				return fmt.Errorf("%s step %d (%s): %w", testCase.Name, index, step.Op, err)
			}
		}
	}
	return nil
}

func TestBlockFixtureModelRejectsMarkerCorruptionExactly(t *testing.T) {
	raw := readBlockFixture(t)
	mutated, err := corruptBlockFixtureMarker(raw)
	if err != nil {
		t.Fatalf("make exact marker corruption: %v", err)
	}
	corrupted, err := parseBlockFixture(mutated)
	if err != nil {
		t.Fatalf("marker corruption should remain well typed: %v", err)
	}
	if err := validateBlockFixtureModel(corrupted); err == nil || !strings.Contains(err.Error(), `case "multiple-incomplete-blocks-and-capacity" step 6 state differs from model`) {
		t.Fatalf("independent model did not reject the exact marker corruption: %v", err)
	}
}

func corruptBlockFixtureMarker(raw []byte) ([]byte, error) {
	document, err := parseBlockFixture(raw)
	if err != nil {
		return nil, err
	}
	var target *blockFixtureStep
	for caseIndex := range document.Cases {
		if document.Cases[caseIndex].Name == "multiple-incomplete-blocks-and-capacity" {
			if len(document.Cases[caseIndex].Steps) <= 6 {
				return nil, errors.New("marker corruption step anchor changed")
			}
			target = &document.Cases[caseIndex].Steps[6]
			break
		}
	}
	if target == nil || target.Op != "erase" || target.Expected.State == nil || target.Expected.State.Pages == nil ||
		len(*target.Expected.State.Pages) <= 2 || (*target.Expected.State.Pages)[2].Programmed == nil || *(*target.Expected.State.Pages)[2].Programmed {
		return nil, errors.New("canonical marker corruption anchor changed")
	}
	programmed := true
	(*target.Expected.State.Pages)[2].Programmed = &programmed
	return json.Marshal(document)
}

func readBlockFixture(t testing.TB) []byte {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-block-reclamation.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func TestBlockFixtureAdapterRejectsMalformedAndWrongDeclarations(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-block-reclamation.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	variants := map[string][]byte{
		"duplicate key":          bytes.Replace(raw, []byte(`"schema":`), []byte(`"schema":"bad","schema":`), 1),
		"escaped duplicate":      bytes.Replace(raw, []byte(`"schema":`), []byte(`"sch\u0065ma":"bad","schema":`), 1),
		"uppercase alias":        bytes.Replace(raw, []byte(`"logical": 0`), []byte(`"Logical": 0`), 1),
		"fractional block":       bytes.Replace(raw, []byte(`"block": 0`), []byte(`"block": 0.5`), 1),
		"boolean prefix":         bytes.Replace(raw, []byte(`"prefix_pages": 1`), []byte(`"prefix_pages": true`), 1),
		"null state block count": bytes.Replace(raw, []byte(`"completed_erases": 0`), []byte(`"completed_erases": null`), 1),
		"unknown field":          bytes.Replace(raw, []byte(`"schema":`), []byte(`"extra":true,"schema":`), 1),
		"bad hex":                bytes.Replace(raw, []byte(`"payload_hex": "41"`), []byte(`"payload_hex": "4z"`), 1),
		"invalid UTF-8":          append(append([]byte(nil), raw...), 0xff),
		"trailing data":          append(append([]byte(nil), raw...), []byte(`{}`)...),
		"oversized":              bytes.Repeat([]byte{' '}, fixtureLimit+1),
	}
	for name, variant := range variants {
		t.Run(name, func(t *testing.T) {
			if bytes.Equal(raw, variant) {
				t.Fatal("malformed vector mutation anchor was absent")
			}
			if _, err := parseBlockFixture(variant); err == nil {
				t.Fatal("malformed block fixture accepted")
			}
		})
	}
	wrong := bytes.Replace(raw, []byte(`"released": true`), []byte(`"released": false`), 1)
	if bytes.Equal(wrong, raw) {
		t.Fatal("receipt mutation anchor was absent")
	}
	document, err := parseBlockFixture(wrong)
	if err != nil {
		t.Fatalf("wrong result must remain well-typed: %v", err)
	}
	if err := validateBlockFixtureModel(document); err == nil {
		t.Fatal("model accepted wrong erase receipt")
	}
}

func parseBlockFixture(raw []byte) (blockFixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return blockFixtureDocument{}, errors.New("invalid block fixture size or UTF-8")
	}
	if err := scanRelocationJSON(raw); err != nil {
		return blockFixtureDocument{}, err
	}
	if err := validateBlockJSONKinds(raw); err != nil {
		return blockFixtureDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var document blockFixtureDocument
	if err := decoder.Decode(&document); err != nil {
		return blockFixtureDocument{}, err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != io.EOF {
		return blockFixtureDocument{}, errors.New("trailing block fixture JSON")
	}
	if document.Schema != blockFixtureSchema || len(document.Cases) == 0 || len(document.Cases) > fixtureCaseLimit {
		return blockFixtureDocument{}, errors.New("invalid block fixture schema or case count")
	}
	for _, testCase := range document.Cases {
		if testCase.Name == "" || testCase.LogicalCount == nil || testCase.PhysicalCount == nil ||
			testCase.PageSize == nil || testCase.PagesPerBlock == nil || testCase.MaxCompletedErases == nil ||
			len(testCase.Steps) == 0 || len(testCase.Steps) > fixtureStepLimit {
			return blockFixtureDocument{}, errors.New("invalid block fixture case shape")
		}
		if *testCase.LogicalCount < 1 || *testCase.LogicalCount > 64 ||
			*testCase.PhysicalCount < 2 || *testCase.PhysicalCount > 128 || *testCase.PhysicalCount <= *testCase.LogicalCount ||
			*testCase.PageSize < 1 || *testCase.PageSize > maxPageSize ||
			*testCase.PagesPerBlock < 1 || *testCase.PagesPerBlock > *testCase.PhysicalCount ||
			*testCase.PhysicalCount%*testCase.PagesPerBlock != 0 || *testCase.MaxCompletedErases > maxSafeEraseCount {
			return blockFixtureDocument{}, errors.New("block fixture geometry outside supported bounds")
		}
		for _, step := range testCase.Steps {
			if step.Expected.State == nil || !validBlockFixtureExpected(step, *testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize, *testCase.PagesPerBlock) {
				return blockFixtureDocument{}, errors.New("invalid block fixture expectation shape")
			}
			switch step.Op {
			case "read":
				if step.Logical == nil || step.PayloadHex != nil || step.ProgramCut != nil || step.Block != nil || step.EraseCut != nil {
					return blockFixtureDocument{}, errors.New("invalid block fixture read fields")
				}
			case "write":
				if step.Logical == nil || step.PayloadHex == nil || step.ProgramCut == nil || !validCutShape(step.ProgramCut) || step.Block != nil || step.EraseCut != nil {
					return blockFixtureDocument{}, errors.New("invalid block fixture write fields")
				}
				payload, err := decodeFixtureHex(*step.PayloadHex)
				if err != nil || len(payload) != *testCase.PageSize {
					return blockFixtureDocument{}, errors.New("invalid block fixture write payload")
				}
			case "erase":
				if step.Logical != nil || step.PayloadHex != nil || step.ProgramCut != nil || step.Block == nil || step.EraseCut == nil || !validEraseCutShape(step.EraseCut) {
					return blockFixtureDocument{}, errors.New("invalid block fixture erase fields")
				}
			default:
				return blockFixtureDocument{}, errors.New("unknown block fixture operation")
			}
		}
	}
	return document, nil
}

func validEraseCutShape(cut *blockFixtureCut) bool {
	if cut == nil || cut.Kind == nil {
		return false
	}
	switch *cut.Kind {
	case "before_accept", "after_media", "complete":
		return cut.PrefixPages == nil
	case "after_prefix":
		return cut.PrefixPages != nil
	default:
		return false
	}
}

func validBlockFixtureExpected(step blockFixtureStep, logicalCount, physicalCount int64, pageSize int, pagesPerBlock int64) bool {
	expected := step.Expected
	if expected.Error != nil && !knownBlockError(*expected.Error) {
		return false
	}
	if !validBlockFixtureState(expected.State, logicalCount, physicalCount, pageSize, pagesPerBlock) {
		return false
	}
	switch step.Op {
	case "read":
		if expected.DataHex == nil || expected.Present == nil || expected.WriteReceipt != nil || expected.EraseReceipt != nil {
			return false
		}
		data, err := decodeFixtureHex(*expected.DataHex)
		return err == nil && ((*expected.Present && len(data) == pageSize) || (!*expected.Present && len(data) == 0))
	case "write":
		return expected.DataHex == nil && expected.Present == nil && expected.EraseReceipt == nil && ((expected.Error == nil) == (expected.WriteReceipt != nil))
	case "erase":
		if expected.DataHex != nil || expected.Present != nil || expected.WriteReceipt != nil || ((expected.Error == nil) != (expected.EraseReceipt != nil)) {
			return false
		}
		return expected.EraseReceipt == nil || (expected.EraseReceipt.Block != nil && expected.EraseReceipt.Released != nil && expected.EraseReceipt.Acknowledged != nil)
	default:
		return false
	}
}

func validBlockFixtureState(state *blockFixtureState, logicalCount, physicalCount int64, pageSize int, pagesPerBlock int64) bool {
	if state == nil || state.Map == nil || state.Map.LogicalToPhysical == nil || state.Map.PhysicalStatus == nil ||
		state.Pages == nil || state.PagesPerBlock == nil || state.MaxCompletedErases == nil || state.Blocks == nil {
		return false
	}
	if *state.PagesPerBlock != pagesPerBlock || len(*state.Blocks) != int(physicalCount/pagesPerBlock) || *state.MaxCompletedErases > maxSafeEraseCount {
		return false
	}
	if len(state.Map.Reservation) == 0 || string(bytes.TrimSpace(state.Map.Reservation)) != "null" ||
		len(*state.Map.LogicalToPhysical) != int(logicalCount) || len(*state.Map.PhysicalStatus) != int(physicalCount) ||
		len(*state.Pages) != int(physicalCount) {
		return false
	}
	for _, status := range *state.Map.PhysicalStatus {
		if status != "free" && status != "live" && status != "stale" {
			return false
		}
	}
	for _, page := range *state.Pages {
		if page.BytesHex == nil || page.Programmed == nil {
			return false
		}
		data, err := decodeFixtureHex(*page.BytesHex)
		if err != nil || len(data) != pageSize {
			return false
		}
	}
	for _, block := range *state.Blocks {
		if block.CompletedErases == nil || block.IncompleteErase == nil || *block.CompletedErases > *state.MaxCompletedErases {
			return false
		}
	}
	return true
}

func validateBlockFixtureModel(document blockFixtureDocument) error {
	for _, testCase := range document.Cases {
		state := referenceInitialWithEraseBlocks(*testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize, *testCase.PagesPerBlock, *testCase.MaxCompletedErases)
		for index, step := range testCase.Steps {
			var next referenceState
			var writeReceipt modelReceipt
			var eraseReceipt EraseReceipt
			var readData []byte
			var present bool
			var expectedErr string
			switch step.Op {
			case "read":
				readData, present, expectedErr = referenceRead(state, *step.Logical)
				next = state
			case "write":
				payload, err := decodeFixtureHex(*step.PayloadHex)
				if err != nil {
					return err
				}
				next, writeReceipt, expectedErr = referenceWrite(state, *step.Logical, payload, fixtureCutToModel(step.ProgramCut))
			case "erase":
				next, eraseReceipt, expectedErr = referenceEraseBlock(state, *step.Block, fixtureEraseCutToModel(step.EraseCut))
			}
			if blockFixtureError(step.Expected.Error) != expectedErr {
				return fmt.Errorf("case %q step %d error declared %v model=%s", testCase.Name, index, step.Expected.Error, expectedErr)
			}
			if expectedErr == "" {
				switch step.Op {
				case "read":
					data, err := decodeFixtureHex(*step.Expected.DataHex)
					if err != nil || !bytes.Equal(data, readData) || *step.Expected.Present != present {
						return fmt.Errorf("case %q step %d read result mismatch", testCase.Name, index)
					}
				case "write":
					receipt := step.Expected.WriteReceipt
					if receipt == nil || *receipt.Physical != writeReceipt.physical || *receipt.Published != writeReceipt.published || *receipt.Acknowledged != writeReceipt.acknowledged {
						return fmt.Errorf("case %q step %d write receipt mismatch", testCase.Name, index)
					}
				case "erase":
					receipt := step.Expected.EraseReceipt
					if receipt == nil || *receipt.Block != eraseReceipt.Block || *receipt.Released != eraseReceipt.Released || *receipt.Acknowledged != eraseReceipt.Acknowledged {
						return fmt.Errorf("case %q step %d erase receipt mismatch", testCase.Name, index)
					}
				}
			}
			if !reflect.DeepEqual(*step.Expected.State, blockFixtureStateOf(next)) {
				return fmt.Errorf("case %q step %d state differs from model", testCase.Name, index)
			}
			state = next
		}
	}
	return nil
}

func replayBlockFixtureStep(device Device, step blockFixtureStep) (Device, error) {
	var err error
	var readData []byte
	var present bool
	var writeReceipt Receipt
	var eraseReceipt EraseReceipt
	switch step.Op {
	case "read":
		readData, present, err = device.Read(*step.Logical)
	case "write":
		payload, decodeErr := decodeFixtureHex(*step.PayloadHex)
		if decodeErr != nil {
			return device, decodeErr
		}
		device, writeReceipt, err = device.Write(*step.Logical, payload, runtimeFixtureCut(step.ProgramCut))
	case "erase":
		device, eraseReceipt, err = device.EraseBlock(*step.Block, runtimeEraseCut(step.EraseCut))
	}
	if blockFixtureError(step.Expected.Error) != errorCode(err) {
		return device, fmt.Errorf("runtime error %s differs from %v", errorCode(err), step.Expected.Error)
	}
	if err == nil {
		switch step.Op {
		case "read":
			data, decodeErr := decodeFixtureHex(*step.Expected.DataHex)
			if decodeErr != nil || !bytes.Equal(readData, data) || present != *step.Expected.Present {
				return device, errors.New("runtime read differs from fixture")
			}
		case "write":
			receipt := step.Expected.WriteReceipt
			if receipt == nil || writeReceipt != (Receipt{Physical: *receipt.Physical, Published: *receipt.Published, Acknowledged: *receipt.Acknowledged}) {
				return device, errors.New("runtime write receipt differs from fixture")
			}
		case "erase":
			receipt := step.Expected.EraseReceipt
			if receipt == nil || eraseReceipt != (EraseReceipt{Block: *receipt.Block, Released: *receipt.Released, Acknowledged: *receipt.Acknowledged}) {
				return device, errors.New("runtime erase receipt differs from fixture")
			}
		}
	}
	actual := blockFixtureStateOfDevice(device)
	if !reflect.DeepEqual(actual, *step.Expected.State) {
		return device, errors.New("runtime complete state differs from fixture")
	}
	return device, nil
}

func blockFixtureStateOf(state referenceState) blockFixtureState {
	base := fixtureStateOf(state)
	blocks := make([]blockFixtureBlockSnapshot, len(state.blocks))
	for index, block := range state.blocks {
		completed, incomplete := block.CompletedErases, block.IncompleteErase
		blocks[index] = blockFixtureBlockSnapshot{CompletedErases: &completed, IncompleteErase: &incomplete}
	}
	pagesPerBlock, maxErases := state.pagesPerBlock, state.maxCompletedErases
	return blockFixtureState{Map: base.Map, Pages: base.Pages, PagesPerBlock: &pagesPerBlock, MaxCompletedErases: &maxErases, Blocks: &blocks}
}

func blockFixtureStateOfDevice(device Device) blockFixtureState {
	snapshot := device.Snapshot()
	reclamation := device.ReclamationSnapshot()
	statuses := make([]string, len(snapshot.Map.PhysicalStatus))
	for index, status := range snapshot.Map.PhysicalStatus {
		statuses[index] = []string{"free", "reserved", "live", "stale"}[status]
	}
	logical := append([]int64(nil), snapshot.Map.LogicalToPhysical...)
	reservation := json.RawMessage("null")
	mapState := &fixtureMap{LogicalToPhysical: &logical, PhysicalStatus: &statuses, Reservation: reservation}
	pages := make([]fixturePage, len(snapshot.Pages))
	for index, page := range snapshot.Pages {
		data := hex.EncodeToString(page.Bytes)
		programmed := page.Programmed
		pages[index] = fixturePage{BytesHex: &data, Programmed: &programmed}
	}
	blocks := make([]blockFixtureBlockSnapshot, len(reclamation.Blocks))
	for index, block := range reclamation.Blocks {
		completed, incomplete := block.CompletedErases, block.IncompleteErase
		blocks[index] = blockFixtureBlockSnapshot{CompletedErases: &completed, IncompleteErase: &incomplete}
	}
	ppb, max := reclamation.PagesPerBlock, reclamation.MaxCompletedErases
	return blockFixtureState{Map: mapState, Pages: &pages, PagesPerBlock: &ppb, MaxCompletedErases: &max, Blocks: &blocks}
}

func fixtureEraseCutToModel(cut *blockFixtureCut) nandblockerasecuts.Cut {
	kind := parseEraseCut(cut.Kind)
	prefix := 0
	if cut.PrefixPages != nil {
		prefix = *cut.PrefixPages
	}
	return nandblockerasecuts.Cut{Kind: kind, PrefixPages: prefix}
}
func runtimeEraseCut(cut *blockFixtureCut) EraseCut { return fixtureEraseCutToModel(cut) }
func parseEraseCut(kind *string) nandblockerasecuts.CutKind {
	switch *kind {
	case "before_accept":
		return nandblockerasecuts.BeforeAccept
	case "after_prefix":
		return nandblockerasecuts.AfterPrefix
	case "after_media":
		return nandblockerasecuts.AfterMedia
	case "complete":
		return nandblockerasecuts.Complete
	default:
		return nandblockerasecuts.CutKind(255)
	}
}
func blockFixtureError(code *string) string {
	if code == nil {
		return ""
	}
	return *code
}
func knownBlockError(value string) bool {
	switch value {
	case "invalid_geometry", "invalid_logical_page", "no_free_page", "invalid_page_size",
		"wrong_payload_length", "invalid_cut", "invalid_page_state", "already_programmed",
		"invalid_block_geometry", "erase_disabled", "invalid_block_index", "ineligible_block",
		"invalid_erase_count", "invalid_erase_cut", "erase_limit", "invalid_erase_block",
		"invalid_extent", "ineligible_extent":
		return true
	default:
		return false
	}
}

func validateBlockJSONKinds(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return err
	}
	root, ok := value.(map[string]any)
	if !ok || !blockExactKeys(root, []string{"schema", "cases"}, nil) {
		return errors.New("invalid block fixture root keys")
	}
	if _, ok := root["schema"].(string); !ok {
		return errors.New("schema must be a string")
	}
	cases, ok := root["cases"].([]any)
	if !ok {
		return errors.New("cases must be an array")
	}
	for _, item := range cases {
		c, ok := item.(map[string]any)
		if !ok || !blockExactKeys(c, []string{"name", "logical_count", "physical_count", "page_size", "pages_per_block", "max_completed_erases", "steps"}, nil) {
			return errors.New("invalid case keys")
		}
		if _, ok := c["name"].(string); !ok || !blockInteger(c["logical_count"]) ||
			!blockInteger(c["physical_count"]) || !blockInteger(c["page_size"]) ||
			!blockInteger(c["pages_per_block"]) || !blockInteger(c["max_completed_erases"]) {
			return errors.New("invalid case field token kind")
		}
		steps, ok := c["steps"].([]any)
		if !ok {
			return errors.New("steps must be an array")
		}
		for _, item := range steps {
			s, ok := item.(map[string]any)
			if !ok {
				return errors.New("step must be an object")
			}
			op, ok := s["op"].(string)
			if !ok {
				return errors.New("op must be a string")
			}
			var required []string
			switch op {
			case "read":
				required = []string{"op", "logical", "expected"}
				if !blockInteger(s["logical"]) {
					return errors.New("logical must be an integer")
				}
			case "write":
				required = []string{"op", "logical", "payload_hex", "cut", "expected"}
				if !blockInteger(s["logical"]) {
					return errors.New("logical must be an integer")
				}
				if _, ok := s["payload_hex"].(string); !ok {
					return errors.New("payload_hex must be text")
				}
				if err := validateProgramCutKinds(s["cut"]); err != nil {
					return err
				}
			case "erase":
				required = []string{"op", "block", "erase_cut", "expected"}
				if !blockInteger(s["block"]) {
					return errors.New("block must be an integer")
				}
				if err := validateEraseCutKinds(s["erase_cut"]); err != nil {
					return err
				}
			default:
				return errors.New("unknown block operation")
			}
			if !blockExactKeys(s, required, nil) {
				return errors.New("unknown, missing or incorrectly-cased operation key")
			}
			if err := validateBlockExpectedKinds(s["expected"]); err != nil {
				return err
			}
		}
	}
	return nil
}

func validateProgramCutKinds(value any) error {
	cut, ok := value.(map[string]any)
	if !ok || !blockExactKeys(cut, []string{"kind"}, []string{"prefix_bytes"}) {
		return errors.New("invalid program cut keys")
	}
	if _, ok := cut["kind"].(string); !ok {
		return errors.New("program cut kind must be text")
	}
	if value, ok := cut["prefix_bytes"]; ok && !blockInteger(value) {
		return errors.New("program prefix must be integer")
	}
	return nil
}
func validateEraseCutKinds(value any) error {
	cut, ok := value.(map[string]any)
	if !ok || !blockExactKeys(cut, []string{"kind"}, []string{"prefix_pages"}) {
		return errors.New("invalid erase cut keys")
	}
	if _, ok := cut["kind"].(string); !ok {
		return errors.New("erase cut kind must be text")
	}
	if value, ok := cut["prefix_pages"]; ok && !blockInteger(value) {
		return errors.New("erase prefix must be integer")
	}
	return nil
}
func validateBlockExpectedKinds(value any) error {
	expected, ok := value.(map[string]any)
	if !ok || !blockExactKeys(expected, []string{"state"}, []string{"error", "data_hex", "present", "write_receipt", "erase_receipt"}) {
		return errors.New("invalid expected keys")
	}
	if value, ok := expected["error"]; ok {
		if _, ok := value.(string); !ok {
			return errors.New("expected error must be text")
		}
	}
	if value, ok := expected["data_hex"]; ok {
		if _, ok := value.(string); !ok {
			return errors.New("data_hex must be text")
		}
	}
	if value, ok := expected["present"]; ok {
		if _, ok := value.(bool); !ok {
			return errors.New("present must be boolean")
		}
	}
	if value, ok := expected["write_receipt"]; ok {
		receipt, ok := value.(map[string]any)
		if !ok || !blockExactKeys(receipt, []string{"physical", "published", "acknowledged"}, nil) || !blockInteger(receipt["physical"]) {
			return errors.New("invalid write receipt kinds")
		}
		if _, ok := receipt["published"].(bool); !ok {
			return errors.New("published must be boolean")
		}
		if _, ok := receipt["acknowledged"].(bool); !ok {
			return errors.New("acknowledged must be boolean")
		}
	}
	if value, ok := expected["erase_receipt"]; ok {
		receipt, ok := value.(map[string]any)
		if !ok || !blockExactKeys(receipt, []string{"block", "released", "acknowledged"}, nil) || !blockInteger(receipt["block"]) {
			return errors.New("invalid erase receipt kinds")
		}
		if _, ok := receipt["released"].(bool); !ok {
			return errors.New("released must be boolean")
		}
		if _, ok := receipt["acknowledged"].(bool); !ok {
			return errors.New("acknowledged must be boolean")
		}
	}
	state, ok := expected["state"].(map[string]any)
	if !ok || !blockExactKeys(state, []string{"map", "pages", "pages_per_block", "max_completed_erases", "blocks"}, nil) {
		return errors.New("invalid expected state keys")
	}
	if !blockInteger(state["pages_per_block"]) || !blockInteger(state["max_completed_erases"]) {
		return errors.New("expected block geometry must be integer tokens")
	}
	if err := validateBlockMapKinds(state["map"]); err != nil {
		return err
	}
	pages, ok := state["pages"].([]any)
	if !ok {
		return errors.New("pages must be array")
	}
	for _, item := range pages {
		page, ok := item.(map[string]any)
		if !ok || !blockExactKeys(page, []string{"bytes_hex", "programmed"}, nil) {
			return errors.New("invalid page keys")
		}
		if _, ok := page["bytes_hex"].(string); !ok {
			return errors.New("bytes_hex must be text")
		}
		if _, ok := page["programmed"].(bool); !ok {
			return errors.New("programmed must be boolean")
		}
	}
	blocks, ok := state["blocks"].([]any)
	if !ok {
		return errors.New("blocks must be array")
	}
	for _, item := range blocks {
		block, ok := item.(map[string]any)
		if !ok || !blockExactKeys(block, []string{"completed_erases", "incomplete_erase"}, nil) || !blockInteger(block["completed_erases"]) {
			return errors.New("invalid block snapshot")
		}
		if _, ok := block["incomplete_erase"].(bool); !ok {
			return errors.New("incomplete_erase must be boolean")
		}
	}
	return nil
}
func validateBlockMapKinds(value any) error {
	m, ok := value.(map[string]any)
	if !ok || !blockExactKeys(m, []string{"logical_to_physical", "physical_status", "reservation"}, nil) {
		return errors.New("invalid map keys")
	}
	logical, ok := m["logical_to_physical"].([]any)
	if !ok {
		return errors.New("logical map must be array")
	}
	for _, item := range logical {
		if !blockInteger(item) {
			return errors.New("logical map items must be integer tokens")
		}
	}
	statuses, ok := m["physical_status"].([]any)
	if !ok {
		return errors.New("physical statuses must be array")
	}
	for _, item := range statuses {
		value, ok := item.(string)
		if !ok || (value != "free" && value != "live" && value != "stale") {
			return errors.New("invalid physical status")
		}
	}
	if m["reservation"] != nil {
		return errors.New("reservation must be null in a committed state")
	}
	return nil
}
func blockInteger(value any) bool {
	number, ok := value.(json.Number)
	if !ok {
		return false
	}
	_, err := number.Int64()
	return err == nil
}
func blockExactKeys(m map[string]any, required, optional []string) bool {
	allowed := map[string]bool{}
	for _, key := range required {
		allowed[key] = true
		if _, ok := m[key]; !ok {
			return false
		}
	}
	for _, key := range optional {
		allowed[key] = true
	}
	for key := range m {
		if !allowed[key] {
			return false
		}
	}
	return true
}

func FuzzParseBlockReclamationFixture(f *testing.F) {
	canonical := readBlockFixture(f)
	f.Add(canonical)
	markerCorruption, err := corruptBlockFixtureMarker(canonical)
	if err != nil {
		f.Fatal(err)
	}
	f.Add(markerCorruption)
	f.Add([]byte(`{"schema":"coding-adventures.database-media.ftl-block-reclamation.v1","cases":[]}`))
	f.Add([]byte(`{"schema":"wrong","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		document, err := parseBlockFixture(raw)
		if err != nil {
			return
		}
		if err := validateBlockFixtureModel(document); err != nil {
			return
		}
		if err := replayBlockFixtureDocument(document); err != nil {
			t.Fatalf("model-accepted fixture disagreed with runtime: %v", err)
		}
	})
}

func FuzzBlockReclamationHistory(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7})
	f.Add([]byte{2, 0, 1, 2, 1, 0, 2, 3, 4})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			commands = commands[:256]
		}
		device, err := NewWithEraseBlocks(1, 6, 1, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(1, 6, 1, 2, 2)
		for index, command := range commands {
			payload := []byte{byte(index*17) ^ command}
			logical := int64(0)
			switch command & 0x18 {
			case 0x08:
				logical = -1
			case 0x10:
				logical = 1
			}
			switch command % 3 {
			case 0:
				kind := modelCut{kind: cutComplete}
				if command&4 != 0 {
					kind = modelCut{kind: cutAfterPrefix, prefix: int(command % 2)}
				}
				want, receipt, wantErr := referenceWrite(model, logical, payload, kind)
				got, actualReceipt, actualErr := device.Write(logical, payload, runtimeCut(kind))
				wantReceipt := Receipt{Physical: receipt.physical, Published: receipt.published, Acknowledged: receipt.acknowledged}
				if errorCode(actualErr) != wantErr || (wantErr == "" && actualReceipt != wantReceipt) {
					t.Fatalf("write command %d: receipt=%+v err=%s want=%+v/%s", index, actualReceipt, errorCode(actualErr), receipt, wantErr)
				}
				device = got
				model = want
			case 1:
				cut := nandblockerasecuts.Cut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: int(command % 3)}
				if command&8 != 0 {
					cut = nandblockerasecuts.Cut{Kind: nandblockerasecuts.Complete}
				}
				block := int64((command>>2)%5) - 1
				want, receipt, wantErr := referenceEraseBlock(model, block, cut)
				got, actualReceipt, actualErr := device.EraseBlock(block, cut)
				if errorCode(actualErr) != wantErr || (wantErr == "" && actualReceipt != receipt) {
					t.Fatalf("erase command %d: receipt=%+v err=%s want=%+v/%s", index, actualReceipt, errorCode(actualErr), receipt, wantErr)
				}
				device = got
				model = want
			case 2:
				wantData, wantPresent, wantErr := referenceRead(model, logical)
				gotData, gotPresent, actualErr := device.Read(logical)
				if errorCode(actualErr) != wantErr || gotPresent != wantPresent || !bytes.Equal(gotData, wantData) {
					t.Fatalf("read command %d differs from model", index)
				}
			}
			assertDeviceEquals(t, device, model)
		}
	})
}
