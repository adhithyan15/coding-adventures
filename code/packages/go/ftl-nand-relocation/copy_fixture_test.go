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
	"testing"
	"unicode/utf8"

	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

const copyFixtureSchema = "coding-adventures.database-media.ftl-live-page-copy.v1"

type copyFixtureDocument struct {
	Schema string            `json:"schema"`
	Cases  []copyFixtureCase `json:"cases"`
}
type copyFixtureCase struct {
	Name               string               `json:"name"`
	LogicalCount       *int64               `json:"logical_count"`
	PhysicalCount      *int64               `json:"physical_count"`
	PageSize           *int                 `json:"page_size"`
	PagesPerBlock      *int64               `json:"pages_per_block"`
	MaxCompletedErases *uint64              `json:"max_completed_erases"`
	SetupWrites        *[]copyFixtureWrite  `json:"setup_writes"`
	Block              *int64               `json:"block"`
	ExcludedBlocks     *[]bool              `json:"excluded_blocks"`
	Move               *copyFixtureMove     `json:"move"`
	InterveningWrites  *[]copyFixtureWrite  `json:"intervening_writes"`
	Cut                *fixtureCut          `json:"cut"`
	Expected           *copyFixtureExpected `json:"expected"`
}
type copyFixtureWrite struct {
	Logical    *int64  `json:"logical"`
	PayloadHex *string `json:"payload_hex"`
}
type copyFixtureMove struct {
	Logical             *int64 `json:"logical"`
	SourcePhysical      *int64 `json:"source_physical"`
	DestinationPhysical *int64 `json:"destination_physical"`
}
type copyFixtureExpected struct {
	Error   *string            `json:"error,omitempty"`
	Receipt *fixtureReceipt    `json:"receipt,omitempty"`
	State   *blockFixtureState `json:"state"`
}

func TestCanonicalLivePageCopyFixtureModelFirstThenRuntime(t *testing.T) {
	document, err := parseCopyFixture(readCopyFixture(t))
	if err != nil {
		t.Fatalf("parse canonical copy fixture: %v", err)
	}
	if err := validateCopyFixtureModel(document); err != nil {
		t.Fatalf("independent copy model rejected canonical fixture: %v", err)
	}
	if err := replayCopyFixtureDocument(document); err != nil {
		t.Fatal(err)
	}
}

func replayCopyFixtureDocument(document copyFixtureDocument) error {
	for _, item := range document.Cases {
		device, err := buildRuntimeCopyFixture(item)
		if err != nil {
			return fmt.Errorf("%s: setup: %w", item.Name, err)
		}
		planned, err := ftlevacuationplan.PlanEvacuation(device.table, *item.PagesPerBlock, *item.Block, *item.ExcludedBlocks)
		if err != nil || !containsCopy(planned.Copies, runtimeCopy(item.Move)) {
			return fmt.Errorf("%s: fixture move was not in runtime evacuation plan: %+v %v", item.Name, planned, err)
		}
		for _, write := range *item.InterveningWrites {
			payload, _ := hex.DecodeString(*write.PayloadHex)
			device, _, err = device.Write(*write.Logical, payload, Cut{Kind: nandpageprogramcuts.Complete})
			if err != nil {
				return fmt.Errorf("%s: intervening write: %w", item.Name, err)
			}
		}
		device, receipt, err := device.CopyLivePage(*item.Block, runtimeCopy(item.Move), runtimeFixtureCut(item.Cut))
		if errorCode(err) != copyFixtureError(item.Expected.Error) {
			return fmt.Errorf("%s: error=%s want=%s", item.Name, errorCode(err), copyFixtureError(item.Expected.Error))
		}
		if err != nil {
			if receipt != (Receipt{}) {
				return fmt.Errorf("%s: refused copy returned nonzero receipt %+v", item.Name, receipt)
			}
		} else {
			wantReceipt := Receipt{Physical: *item.Expected.Receipt.Physical, Published: *item.Expected.Receipt.Published, Acknowledged: *item.Expected.Receipt.Acknowledged}
			if receipt != wantReceipt {
				return fmt.Errorf("%s: receipt=%+v want=%+v", item.Name, receipt, wantReceipt)
			}
		}
		if actual := blockFixtureStateOfDevice(device); !reflect.DeepEqual(actual, *item.Expected.State) {
			return fmt.Errorf("%s: runtime snapshot differs from fixture", item.Name)
		}
	}
	return nil
}

func TestCopyFixtureAdapterRejectsMalformedAndWrongDeclarations(t *testing.T) {
	raw := readCopyFixture(t)
	variants := map[string][]byte{
		"null source":                bytes.Replace(raw, []byte(`"source_physical": 0`), []byte(`"source_physical": null`), 1),
		"case-aliased block":         bytes.Replace(raw, []byte(`"block": 0`), []byte(`"Block": 0`), 1),
		"boolean destination":        bytes.Replace(raw, []byte(`"destination_physical": 4`), []byte(`"destination_physical": true`), 1),
		"fractional logical":         bytes.Replace(raw, []byte(`"logical": 0`), []byte(`"logical": 0.5`), 1),
		"nonportable source integer": bytes.Replace(raw, []byte(`"source_physical": 0`), []byte(`"source_physical": 9007199254740992`), 1),
		"duplicate key":              bytes.Replace(raw, []byte(`"schema":`), []byte(`"schema":"other","schema":`), 1),
		"escaped duplicate":          bytes.Replace(raw, []byte(`"schema":`), []byte(`"sch\u0065ma":"other","schema":`), 1),
		"unknown nested field":       bytes.Replace(raw, []byte(`"move": {`), []byte(`"move": {"extra": true,`), 1),
		"bad UTF-8":                  append(append([]byte(nil), raw...), 0xff),
		"trailing value":             append(append([]byte(nil), raw...), []byte(`{}`)...),
		"excessive depth":            []byte(bytes.Repeat([]byte{'['}, fixtureDepth+1)),
		"oversized":                  bytes.Repeat([]byte{' '}, fixtureLimit+1),
	}
	for name, variant := range variants {
		t.Run(name, func(t *testing.T) {
			if bytes.Equal(raw, variant) {
				t.Fatal("fixture mutation anchor was absent")
			}
			if _, err := parseCopyFixture(variant); err == nil {
				t.Fatal("malformed copy fixture accepted")
			}
		})
	}
	wrong := copyFixtureWrongPublication(raw)
	if bytes.Equal(raw, wrong) {
		t.Fatal("receipt mutation anchor was absent")
	}
	document, err := parseCopyFixture(wrong)
	if err != nil {
		t.Fatalf("wrong declaration should remain well typed: %v", err)
	}
	if err := validateCopyFixtureModel(document); err == nil {
		t.Fatal("independent model accepted a wrong publication receipt")
	}
}

func FuzzParseLivePageCopyFixture(f *testing.F) {
	canonical := readCopyFixtureBytes(f)
	f.Add(canonical)
	wrong := copyFixtureWrongPublication(canonical)
	if bytes.Equal(wrong, canonical) {
		f.Fatal("wrong-declaration seed anchor is absent")
	}
	f.Add(wrong)
	f.Add([]byte(`{"schema":"invalid","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		document, err := parseCopyFixture(raw)
		if err != nil {
			return
		}
		if err := validateCopyFixtureModel(document); err != nil {
			return
		}
		if err := replayCopyFixtureDocument(document); err != nil {
			t.Fatalf("model-accepted fixture disagrees with runtime: %v", err)
		}
	})
}

func copyFixtureWrongPublication(raw []byte) []byte {
	return bytes.Replace(raw, []byte(`"published": true`), []byte(`"published": false`), 1)
}

func parseCopyFixture(raw []byte) (copyFixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return copyFixtureDocument{}, errors.New("invalid copy fixture size or UTF-8")
	}
	if err := scanRelocationJSON(raw); err != nil {
		return copyFixtureDocument{}, err
	}
	if err := validateCopyFixtureKinds(raw); err != nil {
		return copyFixtureDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var document copyFixtureDocument
	if err := decoder.Decode(&document); err != nil {
		return copyFixtureDocument{}, err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != io.EOF {
		return copyFixtureDocument{}, errors.New("trailing copy fixture JSON")
	}
	if document.Schema != copyFixtureSchema || len(document.Cases) == 0 || len(document.Cases) > fixtureCaseLimit {
		return copyFixtureDocument{}, errors.New("invalid copy fixture schema or case count")
	}
	for _, item := range document.Cases {
		if item.Name == "" || item.LogicalCount == nil || item.PhysicalCount == nil || item.PageSize == nil ||
			item.PagesPerBlock == nil || item.MaxCompletedErases == nil || item.SetupWrites == nil ||
			item.Block == nil || item.ExcludedBlocks == nil || item.Move == nil || item.InterveningWrites == nil ||
			item.Cut == nil || item.Expected == nil || item.Expected.State == nil {
			return copyFixtureDocument{}, errors.New("missing copy fixture field")
		}
		if *item.LogicalCount < 1 || *item.LogicalCount > 64 || *item.PhysicalCount < 2 || *item.PhysicalCount > 128 ||
			*item.PhysicalCount <= *item.LogicalCount || *item.PageSize < 1 || *item.PageSize > maxPageSize ||
			*item.PagesPerBlock < 1 || *item.PagesPerBlock > *item.PhysicalCount ||
			*item.PhysicalCount%*item.PagesPerBlock != 0 || *item.MaxCompletedErases > maxSafeEraseCount {
			return copyFixtureDocument{}, errors.New("copy fixture geometry outside supported bounds")
		}
		blockCount := *item.PhysicalCount / *item.PagesPerBlock
		if len(*item.ExcludedBlocks) != 0 && int64(len(*item.ExcludedBlocks)) != blockCount {
			return copyFixtureDocument{}, errors.New("copy fixture exclusion mask has wrong length")
		}
		if *item.Block < 0 || *item.Block >= blockCount || !validCopyCutShape(item.Cut) || len(*item.SetupWrites)+len(*item.InterveningWrites) > int(*item.PhysicalCount) {
			return copyFixtureDocument{}, errors.New("invalid copy fixture block, cut or setup operation count")
		}
		for _, writes := range [][]copyFixtureWrite{*item.SetupWrites, *item.InterveningWrites} {
			for _, write := range writes {
				if write.Logical == nil || write.PayloadHex == nil || *write.Logical < 0 || *write.Logical >= *item.LogicalCount {
					return copyFixtureDocument{}, errors.New("invalid copy fixture write address")
				}
				payload, err := decodeFixtureHex(*write.PayloadHex)
				if err != nil || len(payload) != *item.PageSize {
					return copyFixtureDocument{}, errors.New("invalid copy fixture write payload")
				}
			}
		}
		if item.Move.Logical == nil || item.Move.SourcePhysical == nil || item.Move.DestinationPhysical == nil {
			return copyFixtureDocument{}, errors.New("incomplete copy assignment")
		}
		if *item.Move.Logical < 0 || *item.Move.Logical >= *item.LogicalCount ||
			*item.Move.SourcePhysical < 0 || *item.Move.SourcePhysical >= *item.PhysicalCount ||
			*item.Move.DestinationPhysical < 0 || *item.Move.DestinationPhysical >= *item.PhysicalCount {
			return copyFixtureDocument{}, errors.New("copy assignment address outside fixture geometry")
		}
		if !validBlockFixtureState(item.Expected.State, *item.LogicalCount, *item.PhysicalCount, *item.PageSize, *item.PagesPerBlock) {
			return copyFixtureDocument{}, errors.New("invalid copy fixture expected state")
		}
		for _, physical := range *item.Expected.State.Map.LogicalToPhysical {
			if physical < -1 || physical >= *item.PhysicalCount {
				return copyFixtureDocument{}, errors.New("expected mapping address outside fixture geometry")
			}
		}
		if item.Expected.Error != nil {
			if !knownCopyError(*item.Expected.Error) || item.Expected.Receipt != nil {
				return copyFixtureDocument{}, errors.New("invalid copy fixture refusal result")
			}
		} else if item.Expected.Receipt == nil || item.Expected.Receipt.Physical == nil || item.Expected.Receipt.Published == nil || item.Expected.Receipt.Acknowledged == nil {
			return copyFixtureDocument{}, errors.New("missing successful copy receipt")
		} else if *item.Expected.Receipt.Physical < 0 || *item.Expected.Receipt.Physical >= *item.PhysicalCount {
			return copyFixtureDocument{}, errors.New("copy receipt address outside fixture geometry")
		}
	}
	return document, nil
}

func validCopyCutShape(cut *fixtureCut) bool {
	if cut == nil || cut.Kind == nil {
		return false
	}
	switch *cut.Kind {
	case "before_accept", "after_media", "complete":
		return cut.PrefixBytes == nil
	case "after_prefix":
		return cut.PrefixBytes != nil && *cut.PrefixBytes >= 0 && *cut.PrefixBytes <= maxPageSize

	default:
		return false
	}
}

func validateCopyFixtureKinds(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return err
	}
	root, ok := value.(map[string]any)
	if !ok || !blockExactKeys(root, []string{"schema", "cases"}, nil) {
		return errors.New("invalid copy fixture root keys")
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
		if !ok || !blockExactKeys(item, []string{"name", "logical_count", "physical_count", "page_size", "pages_per_block", "max_completed_erases", "setup_writes", "block", "excluded_blocks", "move", "intervening_writes", "cut", "expected"}, nil) {
			return errors.New("invalid copy fixture case keys")
		}
		if _, ok := item["name"].(string); !ok || !blockInteger(item["logical_count"]) || !blockInteger(item["physical_count"]) ||
			!blockInteger(item["page_size"]) || !blockInteger(item["pages_per_block"]) || !blockInteger(item["max_completed_erases"]) || !blockInteger(item["block"]) {
			return errors.New("invalid copy fixture case token kinds")
		}
		for _, field := range []string{"setup_writes", "intervening_writes"} {
			writes, ok := item[field].([]any)
			if !ok {
				return errors.New("copy fixture writes must be arrays")
			}
			for _, value := range writes {
				write, ok := value.(map[string]any)
				if !ok || !blockExactKeys(write, []string{"logical", "payload_hex"}, nil) || !blockInteger(write["logical"]) {
					return errors.New("invalid copy fixture write token kinds")
				}
				if _, ok := write["payload_hex"].(string); !ok {
					return errors.New("write payload must be text")
				}
			}
		}
		mask, ok := item["excluded_blocks"].([]any)
		if !ok {
			return errors.New("excluded_blocks must be an array")
		}
		for _, value := range mask {
			if _, ok := value.(bool); !ok {
				return errors.New("exclusion values must be boolean")
			}
		}
		move, ok := item["move"].(map[string]any)
		if !ok || !blockExactKeys(move, []string{"logical", "source_physical", "destination_physical"}, nil) ||
			!blockInteger(move["logical"]) || !blockInteger(move["source_physical"]) || !blockInteger(move["destination_physical"]) {
			return errors.New("invalid copy assignment token kinds")
		}
		if err := validateProgramCutKinds(item["cut"]); err != nil {
			return err
		}
		if err := validateCopyExpectedKinds(item["expected"]); err != nil {
			return err
		}
	}
	return nil
}

func validateCopyExpectedKinds(value any) error {
	expected, ok := value.(map[string]any)
	if !ok || !blockExactKeys(expected, []string{"state"}, []string{"error", "receipt"}) {
		return errors.New("invalid copy expected keys")
	}
	if errorValue, exists := expected["error"]; exists {
		if _, ok := errorValue.(string); !ok {
			return errors.New("copy error must be text")
		}
	}
	if receiptValue, exists := expected["receipt"]; exists {
		receipt, ok := receiptValue.(map[string]any)
		if !ok || !blockExactKeys(receipt, []string{"physical", "published", "acknowledged"}, nil) || !blockInteger(receipt["physical"]) {
			return errors.New("invalid copy receipt token kinds")
		}
		if _, ok := receipt["published"].(bool); !ok {
			return errors.New("published must be boolean")
		}
		if _, ok := receipt["acknowledged"].(bool); !ok {
			return errors.New("acknowledged must be boolean")
		}
	}
	return validateBlockExpectedKinds(map[string]any{"state": expected["state"]})
}

func validateCopyFixtureModel(document copyFixtureDocument) error {
	for _, item := range document.Cases {
		state := referenceInitialWithEraseBlocks(*item.LogicalCount, *item.PhysicalCount, *item.PageSize, *item.PagesPerBlock, *item.MaxCompletedErases)
		for index, write := range *item.SetupWrites {
			payload, _ := hex.DecodeString(*write.PayloadHex)
			var code string
			state, _, code = referenceWrite(state, *write.Logical, payload, modelCut{kind: cutComplete})
			if code != "" {
				return fmt.Errorf("case %s setup write %d: %s", item.Name, index, code)
			}
		}
		assignments, planError := referenceCopyAssignments(state, *item.PagesPerBlock, *item.Block, *item.ExcludedBlocks)
		if planError != "" || !containsCopy(assignments, runtimeCopy(item.Move)) {
			return fmt.Errorf("case %s move is not an assigned plan copy", item.Name)
		}
		for index, write := range *item.InterveningWrites {
			payload, _ := hex.DecodeString(*write.PayloadHex)
			var code string
			state, _, code = referenceWrite(state, *write.Logical, payload, modelCut{kind: cutComplete})
			if code != "" {
				return fmt.Errorf("case %s intervening write %d: %s", item.Name, index, code)
			}
		}
		next, receipt, code := referenceCopyLive(state, *item.Block, runtimeCopy(item.Move), fixtureCutToModel(item.Cut))
		if copyFixtureError(item.Expected.Error) != code {
			return fmt.Errorf("case %s error=%s model=%s", item.Name, copyFixtureError(item.Expected.Error), code)
		}
		if code == "" {
			if item.Expected.Receipt == nil || *item.Expected.Receipt.Physical != receipt.physical ||
				*item.Expected.Receipt.Published != receipt.published || *item.Expected.Receipt.Acknowledged != receipt.acknowledged {
				return fmt.Errorf("case %s receipt differs from independent model", item.Name)
			}
		}
		if !reflect.DeepEqual(*item.Expected.State, blockFixtureStateOf(next)) {
			return fmt.Errorf("case %s state differs from independent model", item.Name)
		}
	}
	return nil
}

func referenceCopyAssignments(state referenceState, pagesPerBlock, block int64, excluded []bool) ([]ftlevacuationplan.Copy, string) {
	blockCount := state.physicalCount / pagesPerBlock
	if block < 0 || block >= blockCount {
		return nil, "invalid_block_index"
	}
	if len(excluded) != 0 && int64(len(excluded)) != blockCount {
		return nil, "invalid_block_mask"
	}
	if len(excluded) != 0 && excluded[int(block)] {
		return nil, "excluded_block"
	}
	first := block * pagesPerBlock
	copies := make([]ftlevacuationplan.Copy, 0)
	stale := false
	for physical := first; physical < first+pagesPerBlock; physical++ {
		switch state.status[int(physical)] {
		case modelStale:
			stale = true
		case modelLive:
			logical := int64(-1)
			for candidate, mapped := range state.mapping {
				if mapped == physical {
					logical = int64(candidate)
					break
				}
			}
			copies = append(copies, ftlevacuationplan.Copy{Logical: logical, SourcePhysical: physical})
		}
	}
	if !stale {
		return nil, "no_stale_page"
	}
	destinations := make([]int64, 0)
	for physical, status := range state.status {
		candidateBlock := int64(physical) / pagesPerBlock
		if status != modelFree || candidateBlock == block || (len(excluded) != 0 && excluded[int(candidateBlock)]) {
			continue
		}
		destinations = append(destinations, int64(physical))
	}
	if len(destinations) < len(copies) {
		return nil, "insufficient_capacity"
	}
	for index := range copies {
		copies[index].DestinationPhysical = destinations[index]
	}
	return copies, ""
}

func buildRuntimeCopyFixture(item copyFixtureCase) (Device, error) {
	device, err := NewWithEraseBlocks(*item.LogicalCount, *item.PhysicalCount, *item.PageSize, *item.PagesPerBlock, *item.MaxCompletedErases)
	if err != nil {
		return Device{}, err
	}
	for _, write := range *item.SetupWrites {
		payload, _ := hex.DecodeString(*write.PayloadHex)
		device, _, err = device.Write(*write.Logical, payload, Cut{Kind: nandpageprogramcuts.Complete})
		if err != nil {
			return Device{}, err
		}
	}
	return device, nil
}
func runtimeCopy(move *copyFixtureMove) ftlevacuationplan.Copy {
	return ftlevacuationplan.Copy{Logical: *move.Logical, SourcePhysical: *move.SourcePhysical, DestinationPhysical: *move.DestinationPhysical}
}
func containsCopy(copies []ftlevacuationplan.Copy, move ftlevacuationplan.Copy) bool {
	for _, candidate := range copies {
		if candidate == move {
			return true
		}
	}
	return false
}
func copyFixtureError(value *string) string {
	if value == nil {
		return ""
	}
	return *value
}
func knownCopyError(value string) bool {
	switch value {
	case "invalid_geometry", "erase_disabled", "invalid_block_index", "evacuation_block_incomplete", "invalid_logical_page", "invalid_copy_source", "copy_source_changed", "invalid_copy_destination", "copy_destination_unavailable", "invalid_cut", "wrong_payload_length", "already_programmed":
		return true
	default:
		return false
	}
}
func readCopyFixture(t *testing.T) []byte {
	t.Helper()
	return readCopyFixtureBytes(t)
}
func readCopyFixtureBytes(t testing.TB) []byte {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-live-page-copy.json"))
	if err != nil {
		t.Fatal(err)
	}
	return raw
}
