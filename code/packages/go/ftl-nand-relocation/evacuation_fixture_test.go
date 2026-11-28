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
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

const evacuationFixtureSchema = "coding-adventures.database-media.ftl-evacuation.v1"

type evacuationFixture struct {
	Schema string                  `json:"schema"`
	Cases  []evacuationFixtureCase `json:"cases"`
}
type evacuationFixtureCase struct {
	Name               string                  `json:"name"`
	LogicalCount       int64                   `json:"logical_count"`
	PhysicalCount      int64                   `json:"physical_count"`
	PageSize           int                     `json:"page_size"`
	PagesPerBlock      int64                   `json:"pages_per_block"`
	MaxCompletedErases uint64                  `json:"max_completed_erases"`
	Steps              []evacuationFixtureStep `json:"steps"`
}
type evacuationFixtureStep struct {
	Op         string                    `json:"op"`
	Logical    *int64                    `json:"logical,omitempty"`
	Block      *int64                    `json:"block,omitempty"`
	PayloadHex *string                   `json:"payload_hex,omitempty"`
	Cut        *fixtureCut               `json:"cut,omitempty"`
	EraseCut   *blockFixtureCut          `json:"erase_cut,omitempty"`
	Expected   evacuationFixtureExpected `json:"expected"`
}
type evacuationFixtureExpected struct {
	Error        string                        `json:"error"`
	State        *blockFixtureState            `json:"state"`
	Active       bool                          `json:"active"`
	Progress     evacuationFixtureProgress     `json:"progress"`
	Read         *evacuationFixtureRead        `json:"read,omitempty"`
	WriteReceipt *fixtureReceipt               `json:"write_receipt,omitempty"`
	CopyReceipt  *evacuationFixtureCopyReceipt `json:"copy_receipt,omitempty"`
	EraseReceipt *blockFixtureEraseReceipt     `json:"erase_receipt,omitempty"`
}

type evacuationFixtureProgress struct {
	Block              int64 `json:"block"`
	RemainingLivePages int64 `json:"remaining_live_pages"`
	IncompleteErase    bool  `json:"incomplete_erase"`
}
type evacuationFixtureRead struct {
	DataHex string `json:"data_hex"`
	Present bool   `json:"present"`
}
type evacuationFixtureCopyReceipt struct {
	Move    evacuationFixtureMove `json:"move"`
	Receipt fixtureReceipt        `json:"receipt"`
}
type evacuationFixtureMove struct {
	Logical             int64 `json:"logical"`
	SourcePhysical      int64 `json:"source_physical"`
	DestinationPhysical int64 `json:"destination_physical"`
}

func TestCanonicalEvacuationFixtureModelFirstThenRuntime(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-evacuation.json"))
	if err != nil {
		t.Fatal(err)
	}
	document, err := parseEvacuationFixture(raw)
	if err != nil {
		t.Fatalf("parse evacuation fixture: %v", err)
	}
	models, err := validateEvacuationFixtureModel(document)
	if err != nil {
		t.Fatalf("independent evacuation model rejected fixture: %v", err)
	}
	for i, item := range document.Cases {
		device, err := NewWithEraseBlocks(item.LogicalCount, item.PhysicalCount, item.PageSize, item.PagesPerBlock, item.MaxCompletedErases)
		if err != nil {
			t.Fatal(err)
		}
		for j, step := range item.Steps {
			nextDevice, actualOutcome, err := applyEvacuationRuntimeObserved(device, step)
			code := errorCode(err)
			if code != step.Expected.Error {
				t.Fatalf("%s step %d runtime error %q, want %q", item.Name, j, errorCode(err), step.Expected.Error)
			}
			if step.Expected.State == nil || !reflect.DeepEqual(blockFixtureStateOfDevice(nextDevice), *step.Expected.State) {
				t.Fatalf("%s step %d runtime state differs from fixture", item.Name, j)
			}
			if !reflect.DeepEqual(actualOutcome, evacuationFixtureExpected{Read: step.Expected.Read, WriteReceipt: step.Expected.WriteReceipt, CopyReceipt: step.Expected.CopyReceipt, EraseReceipt: step.Expected.EraseReceipt}) {
				t.Fatalf("%s step %d runtime command result differs from fixture", item.Name, j)
			}
			assertDeviceEquals(t, nextDevice, models[i][j])
			assertEvacuationProgress(t, nextDevice, models[i][j])
			device = nextDevice
		}
	}
}

func validateEvacuationFixtureModel(document evacuationFixture) ([][]referenceState, error) {
	models := make([][]referenceState, len(document.Cases))
	for i, item := range document.Cases {
		state := referenceInitialWithEraseBlocks(item.LogicalCount, item.PhysicalCount, item.PageSize, item.PagesPerBlock, item.MaxCompletedErases)
		for j, step := range item.Steps {
			next, code, _, _ := applyEvacuationModel(state, step)
			if code != step.Expected.Error {
				return nil, fmt.Errorf("%s step %d model error %q differs from declaration %q", item.Name, j, code, step.Expected.Error)
			}
			if step.Expected.State == nil || !reflect.DeepEqual(*step.Expected.State, blockFixtureStateOf(next)) {
				return nil, fmt.Errorf("%s step %d declared state differs from model", item.Name, j)
			}
			progress, active := referenceEvacuationProgress(next)
			if progress != fixtureProgressValue(step.Expected.Progress) || active != step.Expected.Active {
				return nil, fmt.Errorf("%s step %d declared progress differs from model", item.Name, j)
			}
			want := evacuationModelOutcome(state, step, code)
			if !reflect.DeepEqual(want, evacuationFixtureExpected{Read: step.Expected.Read, WriteReceipt: step.Expected.WriteReceipt, CopyReceipt: step.Expected.CopyReceipt, EraseReceipt: step.Expected.EraseReceipt}) {
				return nil, fmt.Errorf("%s step %d declared command result differs from model", item.Name, j)
			}
			state = next
			models[i] = append(models[i], state)
		}
	}
	return models, nil
}

func replayEvacuationFixture(document evacuationFixture) error {
	for _, item := range document.Cases {
		device, err := NewWithEraseBlocks(item.LogicalCount, item.PhysicalCount, item.PageSize, item.PagesPerBlock, item.MaxCompletedErases)
		if err != nil {
			return err
		}
		for j, step := range item.Steps {
			next, observed, err := applyEvacuationRuntimeObserved(device, step)
			if errorCode(err) != step.Expected.Error {
				return fmt.Errorf("%s step %d runtime refusal differs: %s", item.Name, j, errorCode(err))
			}
			if step.Expected.State == nil || !reflect.DeepEqual(blockFixtureStateOfDevice(next), *step.Expected.State) {
				return fmt.Errorf("%s step %d runtime state differs", item.Name, j)
			}
			if !reflect.DeepEqual(observed, evacuationFixtureExpected{Read: step.Expected.Read, WriteReceipt: step.Expected.WriteReceipt, CopyReceipt: step.Expected.CopyReceipt, EraseReceipt: step.Expected.EraseReceipt}) {
				return fmt.Errorf("%s step %d runtime output differs", item.Name, j)
			}
			progress, active := next.EvacuationProgress()
			if progress != fixtureProgressValue(step.Expected.Progress) || active != step.Expected.Active {
				return fmt.Errorf("%s step %d runtime progress differs", item.Name, j)
			}
			device = next
		}
	}
	return nil
}

func parseEvacuationFixture(raw []byte) (evacuationFixture, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return evacuationFixture{}, errors.New("invalid evacuation fixture size or UTF-8")
	}
	if err := scanRelocationJSON(raw); err != nil {
		return evacuationFixture{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var doc evacuationFixture
	if err := decoder.Decode(&doc); err != nil {
		return evacuationFixture{}, err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != io.EOF {
		return evacuationFixture{}, errors.New("trailing evacuation JSON")
	}
	if doc.Schema != evacuationFixtureSchema || len(doc.Cases) == 0 || len(doc.Cases) > fixtureCaseLimit {
		return evacuationFixture{}, errors.New("invalid evacuation schema or case count")
	}
	// A generic token pass prevents encoding/json's permissive bool/integer
	// coercions and checks exact casing and required operation keys.
	if err := validateEvacuationFixtureKinds(raw); err != nil {
		return evacuationFixture{}, err
	}
	for _, c := range doc.Cases {
		if c.Name == "" || c.LogicalCount < 1 || c.LogicalCount > 64 || c.PhysicalCount <= c.LogicalCount || c.PhysicalCount > 128 || c.PageSize < 1 || c.PageSize > maxPageSize || c.PagesPerBlock < 1 || c.PagesPerBlock > c.PhysicalCount || c.PhysicalCount%c.PagesPerBlock != 0 || c.MaxCompletedErases > maxSafeEraseCount || len(c.Steps) == 0 || len(c.Steps) > fixtureStepLimit {
			return evacuationFixture{}, errors.New("invalid evacuation case geometry or steps")
		}
		for _, s := range c.Steps {
			if !validEvacuationStep(s, c) {
				return evacuationFixture{}, fmt.Errorf("invalid %s step shape", s.Op)
			}
		}
	}
	return doc, nil
}

func validateEvacuationFixtureKinds(raw []byte) error {
	d := json.NewDecoder(bytes.NewReader(raw))
	d.UseNumber()
	var root any
	if err := d.Decode(&root); err != nil {
		return err
	}
	r, ok := root.(map[string]any)
	if !ok || !blockExactKeys(r, []string{"schema", "cases"}, nil) {
		return errors.New("invalid evacuation root keys")
	}
	if err := validateEvacuationPortableNumbers(root); err != nil {
		return err
	}
	cases, ok := r["cases"].([]any)
	if !ok {
		return errors.New("cases must be an array")
	}
	for _, v := range cases {
		c, ok := v.(map[string]any)
		if !ok || !blockExactKeys(c, []string{"name", "logical_count", "physical_count", "page_size", "pages_per_block", "max_completed_erases", "steps"}, nil) {
			return errors.New("invalid evacuation case keys")
		}
		for _, k := range []string{"logical_count", "physical_count", "page_size", "pages_per_block", "max_completed_erases"} {
			if !blockInteger(c[k]) {
				return fmt.Errorf("%s must be an integer", k)
			}
		}
		steps, ok := c["steps"].([]any)
		if !ok {
			return errors.New("steps must be array")
		}
		for _, sv := range steps {
			s, ok := sv.(map[string]any)
			if !ok {
				return errors.New("step must be object")
			}
			op, ok := s["op"].(string)
			if !ok {
				return errors.New("op must be text")
			}
			required := []string{"op", "expected"}
			optional := []string{}
			switch op {
			case "write":
				required = append(required, "logical", "payload_hex", "cut")
				if !blockInteger(s["logical"]) {
					return errors.New("write logical must be integer")
				}
				if _, ok := s["payload_hex"].(string); !ok {
					return errors.New("payload must be text")
				}
				if err := validateProgramCutKinds(s["cut"]); err != nil {
					return err
				}
			case "begin", "erase_other":
				required = append(required, "block")
				if !blockInteger(s["block"]) {
					return errors.New("block must be integer")
				}
				if op == "erase_other" {
					required = append(required, "erase_cut")
					if err := validateEraseCutKinds(s["erase_cut"]); err != nil {
						return err
					}
				}
			case "copy":
				required = append(required, "cut")
				if err := validateProgramCutKinds(s["cut"]); err != nil {
					return err
				}
			case "finish":
				required = append(required, "erase_cut")
				if err := validateEraseCutKinds(s["erase_cut"]); err != nil {
					return err
				}
			case "cancel":
			case "read":
				required = append(required, "logical")
				if !blockInteger(s["logical"]) {
					return errors.New("read logical must be integer")
				}
			default:
				return errors.New("unknown evacuation operation")
			}
			if !blockExactKeys(s, required, optional) {
				return errors.New("unknown, missing or case-aliased evacuation step key")
			}
			exp, ok := s["expected"].(map[string]any)
			if !ok || !blockExactKeys(exp, []string{"error", "state", "active", "progress"}, []string{"read", "write_receipt", "copy_receipt", "erase_receipt"}) {
				return errors.New("invalid expected evacuation result")
			}
			if _, ok := exp["error"].(string); !ok {
				return errors.New("expected error must be text")
			}
			if err := validateBlockExpectedKinds(map[string]any{"state": exp["state"]}); err != nil {
				return err
			}
			if _, ok := exp["active"].(bool); !ok {
				return errors.New("active must be bool")
			}
			p, ok := exp["progress"].(map[string]any)
			if !ok || !blockExactKeys(p, []string{"block", "remaining_live_pages", "incomplete_erase"}, nil) || !blockInteger(p["block"]) || !blockInteger(p["remaining_live_pages"]) {
				return errors.New("invalid progress token kinds")
			}
			if _, ok := p["incomplete_erase"].(bool); !ok {
				return errors.New("progress incomplete must be bool")
			}
			if v, exists := exp["read"]; exists {
				r, ok := v.(map[string]any)
				if !ok || !blockExactKeys(r, []string{"data_hex", "present"}, nil) {
					return errors.New("invalid read result keys")
				}
				if _, ok := r["data_hex"].(string); !ok {
					return errors.New("read data must be text")
				}
				if _, ok := r["present"].(bool); !ok {
					return errors.New("read presence must be bool")
				}
			}
			if v, exists := exp["write_receipt"]; exists {
				r, ok := v.(map[string]any)
				if !ok || !blockExactKeys(r, []string{"physical", "published", "acknowledged"}, nil) || !blockInteger(r["physical"]) {
					return errors.New("invalid write receipt")
				}
				if _, ok := r["published"].(bool); !ok {
					return errors.New("write publication must be bool")
				}
				if _, ok := r["acknowledged"].(bool); !ok {
					return errors.New("write acknowledgement must be bool")
				}
			}
			if v, exists := exp["erase_receipt"]; exists {
				r, ok := v.(map[string]any)
				if !ok || !blockExactKeys(r, []string{"block", "released", "acknowledged"}, nil) || !blockInteger(r["block"]) {
					return errors.New("invalid erase receipt")
				}
				if _, ok := r["released"].(bool); !ok {
					return errors.New("erase release must be bool")
				}
				if _, ok := r["acknowledged"].(bool); !ok {
					return errors.New("erase acknowledgement must be bool")
				}
			}
			if v, exists := exp["copy_receipt"]; exists {
				r, ok := v.(map[string]any)
				if !ok || !blockExactKeys(r, []string{"move", "receipt"}, nil) {
					return errors.New("invalid copy receipt")
				}
				m, ok := r["move"].(map[string]any)
				if !ok || !blockExactKeys(m, []string{"logical", "source_physical", "destination_physical"}, nil) || !blockInteger(m["logical"]) || !blockInteger(m["source_physical"]) || !blockInteger(m["destination_physical"]) {
					return errors.New("invalid copy assignment")
				}
				rr, ok := r["receipt"].(map[string]any)
				if !ok || !blockExactKeys(rr, []string{"physical", "published", "acknowledged"}, nil) || !blockInteger(rr["physical"]) {
					return errors.New("invalid copy receipt fields")
				}
				if _, ok := rr["published"].(bool); !ok {
					return errors.New("copy publication must be bool")
				}
				if _, ok := rr["acknowledged"].(bool); !ok {
					return errors.New("copy acknowledgement must be bool")
				}
			}
		}
	}
	return nil
}

func validateEvacuationPortableNumbers(value any) error {
	switch item := value.(type) {
	case json.Number:
		n, err := strconv.ParseInt(item.String(), 10, 64)
		if err != nil || n < -int64(maxSafeEraseCount) || n > int64(maxSafeEraseCount) {
			return errors.New("evacuation integer is outside portable exact range")
		}
	case []any:
		for _, child := range item {
			if err := validateEvacuationPortableNumbers(child); err != nil {
				return err
			}
		}
	case map[string]any:
		for _, child := range item {
			if err := validateEvacuationPortableNumbers(child); err != nil {
				return err
			}
		}
	}
	return nil
}

func validEvacuationStep(s evacuationFixtureStep, c evacuationFixtureCase) bool {
	if s.Expected.State == nil || !validBlockFixtureState(s.Expected.State, c.LogicalCount, c.PhysicalCount, c.PageSize, c.PagesPerBlock) {
		return false
	}
	if s.Expected.Error != "" && (s.Expected.Read != nil || s.Expected.WriteReceipt != nil || s.Expected.CopyReceipt != nil || s.Expected.EraseReceipt != nil) {
		return false
	}
	if s.Expected.Error == "" {
		switch s.Op {
		case "write":
			if s.Expected.WriteReceipt == nil || s.Expected.Read != nil || s.Expected.CopyReceipt != nil || s.Expected.EraseReceipt != nil {
				return false
			}
		case "copy":
			if s.Expected.CopyReceipt == nil || s.Expected.Read != nil || s.Expected.WriteReceipt != nil || s.Expected.EraseReceipt != nil {
				return false
			}
		case "finish", "erase_other":
			if s.Expected.EraseReceipt == nil || s.Expected.Read != nil || s.Expected.WriteReceipt != nil || s.Expected.CopyReceipt != nil {
				return false
			}
		case "read":
			if s.Expected.Read == nil || s.Expected.WriteReceipt != nil || s.Expected.CopyReceipt != nil || s.Expected.EraseReceipt != nil {
				return false
			}
		default:
			if s.Expected.Read != nil || s.Expected.WriteReceipt != nil || s.Expected.CopyReceipt != nil || s.Expected.EraseReceipt != nil {
				return false
			}
		}
	}
	if !s.Expected.Active && (s.Expected.Progress != (evacuationFixtureProgress{})) {
		return false
	}
	if s.Expected.Active && (s.Expected.Progress.Block < 0 || s.Expected.Progress.Block >= c.PhysicalCount/c.PagesPerBlock || s.Expected.Progress.RemainingLivePages < 0 || s.Expected.Progress.RemainingLivePages > c.PagesPerBlock) {
		return false
	}
	switch s.Op {
	case "write":
		if s.Logical == nil || *s.Logical < 0 || *s.Logical >= c.LogicalCount || s.PayloadHex == nil || s.Cut == nil {
			return false
		}
		p, e := decodeFixtureHex(*s.PayloadHex)
		return e == nil && len(p) == c.PageSize && validCopyCutShape(s.Cut)
	case "begin", "erase_other":
		if s.Block == nil || *s.Block < 0 || *s.Block >= c.PhysicalCount/c.PagesPerBlock {
			return false
		}
		return s.Op != "erase_other" || s.EraseCut != nil
	case "copy":
		return s.Cut != nil && validCopyCutShape(s.Cut)
	case "finish":
		return s.EraseCut != nil
	case "cancel":
		return true
	case "read":
		return s.Logical != nil && *s.Logical >= 0 && *s.Logical < c.LogicalCount
	}
	return false
}

func applyEvacuationModel(before referenceState, s evacuationFixtureStep) (referenceState, string, []byte, bool) {
	switch s.Op {
	case "write":
		p, _ := decodeFixtureHex(*s.PayloadHex)
		n, _, e := referenceWrite(before, *s.Logical, p, fixtureCutToModel(s.Cut))
		return n, e, nil, false
	case "begin":
		n, e := referenceBeginEvacuation(before, *s.Block)
		return n, e, nil, false
	case "copy":
		n, _, e := referenceCopyNext(before, fixtureCutToModel(s.Cut))
		return n, e, nil, false
	case "finish":
		n, _, e := referenceFinishEvacuation(before, fixtureEraseCutToModel(s.EraseCut))
		return n, e, nil, false
	case "cancel":
		n, e := referenceCancelEvacuation(before)
		return n, e, nil, false
	case "erase_other":
		n, _, e := referenceEraseBlock(before, *s.Block, fixtureEraseCutToModel(s.EraseCut))
		return n, e, nil, false
	case "read":
		data, present, e := referenceRead(before, *s.Logical)
		return before, e, data, present
	}
	return before, "invalid_command", nil, false
}

func fixtureWriteReceipt(r modelReceipt) *fixtureReceipt {
	p := r.physical
	pub, ack := r.published, r.acknowledged
	return &fixtureReceipt{Physical: &p, Published: &pub, Acknowledged: &ack}
}
func fixtureCopyReceipt(r EvacuationCopyReceipt) *evacuationFixtureCopyReceipt {
	logical, src, dst := r.Move.Logical, r.Move.SourcePhysical, r.Move.DestinationPhysical
	p := r.Receipt.Physical
	pub, ack := r.Receipt.Published, r.Receipt.Acknowledged
	return &evacuationFixtureCopyReceipt{Move: evacuationFixtureMove{Logical: logical, SourcePhysical: src, DestinationPhysical: dst}, Receipt: fixtureReceipt{Physical: &p, Published: &pub, Acknowledged: &ack}}
}
func fixtureEraseReceipt(r EraseReceipt) *blockFixtureEraseReceipt {
	b := r.Block
	released, ack := r.Released, r.Acknowledged
	return &blockFixtureEraseReceipt{Block: &b, Released: &released, Acknowledged: &ack}
}

func evacuationModelOutcome(before referenceState, step evacuationFixtureStep, code string) evacuationFixtureExpected {
	out := evacuationFixtureExpected{}
	if code != "" {
		return out
	}
	switch step.Op {
	case "write":
		p, _ := decodeFixtureHex(*step.PayloadHex)
		_, r, _ := referenceWrite(before, *step.Logical, p, fixtureCutToModel(step.Cut))
		out.WriteReceipt = fixtureWriteReceipt(r)
	case "copy":
		_, r, _ := referenceCopyNext(before, fixtureCutToModel(step.Cut))
		out.CopyReceipt = fixtureCopyReceipt(r)
	case "finish":
		_, r, _ := referenceFinishEvacuation(before, fixtureEraseCutToModel(step.EraseCut))
		out.EraseReceipt = fixtureEraseReceipt(r)
	case "erase_other":
		_, r, _ := referenceEraseBlock(before, *step.Block, fixtureEraseCutToModel(step.EraseCut))
		out.EraseReceipt = fixtureEraseReceipt(r)
	case "read":
		d, p, _ := referenceRead(before, *step.Logical)
		out.Read = &evacuationFixtureRead{DataHex: hex.EncodeToString(d), Present: p}
	}
	return out
}

func applyEvacuationRuntimeObserved(before Device, s evacuationFixtureStep) (Device, evacuationFixtureExpected, error) {
	var err error
	var observed evacuationFixtureExpected
	switch s.Op {
	case "write":
		p, _ := decodeFixtureHex(*s.PayloadHex)
		var receipt Receipt
		before, receipt, err = before.Write(*s.Logical, p, runtimeFixtureCut(s.Cut))
		if err == nil {
			observed.WriteReceipt = fixtureWriteReceipt(modelReceipt{physical: receipt.Physical, published: receipt.Published, acknowledged: receipt.Acknowledged})
		}
	case "begin":
		before, err = before.BeginEvacuation(*s.Block)
	case "copy":
		var receipt EvacuationCopyReceipt
		before, receipt, err = before.CopyNextEvacuationPage(runtimeFixtureCut(s.Cut))
		if err == nil {
			observed.CopyReceipt = fixtureCopyReceipt(receipt)
		}
	case "finish":
		var receipt EraseReceipt
		before, receipt, err = before.FinishEvacuation(runtimeEraseCut(s.EraseCut))
		if err == nil {
			observed.EraseReceipt = fixtureEraseReceipt(receipt)
		}
	case "cancel":
		before, err = before.CancelEvacuation()
	case "erase_other":
		var receipt EraseReceipt
		before, receipt, err = before.EraseBlock(*s.Block, runtimeEraseCut(s.EraseCut))
		if err == nil {
			observed.EraseReceipt = fixtureEraseReceipt(receipt)
		}
	case "read":
		var data []byte
		var present bool
		data, present, err = before.Read(*s.Logical)
		if err == nil {
			observed.Read = &evacuationFixtureRead{DataHex: hex.EncodeToString(data), Present: present}
		}
	}
	if err != nil {
		observed = evacuationFixtureExpected{}
	}
	return before, observed, err
}

func fixtureProgressValue(p evacuationFixtureProgress) EvacuationProgress {
	return EvacuationProgress{Block: p.Block, RemainingLivePages: p.RemainingLivePages, IncompleteErase: p.IncompleteErase}
}

func TestEvacuationFixtureMalformedInputs(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-evacuation.json"))
	if err != nil {
		t.Fatal(err)
	}
	variants := [][]byte{
		bytes.Replace(raw, []byte(`"logical": 0`), []byte(`"logical": true`), 1),
		bytes.Replace(raw, []byte(`"active": false`), []byte(`"active": null`), 1),
		bytes.Replace(raw, []byte(`"programmed": true`), []byte(`"programmed": null`), 1),
		bytes.Replace(raw, []byte(`"remaining_live_pages": 1`), []byte(`"remaining_live_pages": 1.0`), 1),
		bytes.Replace(raw, []byte(`"schema":`), []byte(`"schema":"bad","schema":`), 1),
		bytes.Replace(raw, []byte(`"schema":`), []byte(`"sch\u0065ma":"bad","schema":`), 1),
		bytes.Replace(raw, []byte(`"logical": 0`), []byte(`"Logical": 0`), 1),
		bytes.Replace(raw, []byte(`"logical_count": 1`), []byte(`"logical_count": 9007199254740992`), 1),
		append(append([]byte(nil), raw...), 0xff), append(append([]byte(nil), raw...), []byte(`{}`)...),
		[]byte(strings.Repeat("[", fixtureDepth+1) + "0" + strings.Repeat("]", fixtureDepth+1)), bytes.Repeat([]byte{' '}, fixtureLimit+1),
	}
	for i, v := range variants {
		if bytes.Equal(v, raw) {
			t.Fatalf("mutation %d anchor absent", i)
		}
		if _, err := parseEvacuationFixture(v); err == nil {
			t.Fatalf("malformed fixture %d accepted", i)
		}
	}
	wrong := bytes.Replace(raw, []byte(`"bytes_hex": "11"`), []byte(`"bytes_hex": "12"`), 1)
	if bytes.Equal(wrong, raw) {
		t.Fatal("well-typed state corruption anchor absent")
	}
	doc, err := parseEvacuationFixture(wrong)
	if err != nil {
		t.Fatalf("state corruption must remain well typed: %v", err)
	}
	state := referenceInitialWithEraseBlocks(doc.Cases[0].LogicalCount, doc.Cases[0].PhysicalCount, doc.Cases[0].PageSize, doc.Cases[0].PagesPerBlock, doc.Cases[0].MaxCompletedErases)
	if next, _, _, _ := applyEvacuationModel(state, doc.Cases[0].Steps[0]); reflect.DeepEqual(blockFixtureStateOf(next), *doc.Cases[0].Steps[0].Expected.State) {
		t.Fatal("independent model accepted altered complete state")
	}
}

func FuzzParseEvacuationFixture(f *testing.F) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-evacuation.json"))
	if err == nil {
		f.Add(raw)
	}
	f.Add([]byte(`{"schema":"bad","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		doc, err := parseEvacuationFixture(raw)
		if err != nil {
			return
		}
		if _, err := validateEvacuationFixtureModel(doc); err != nil {
			return
		}
		if err := replayEvacuationFixture(doc); err != nil {
			t.Fatalf("accepted fixture disagrees with runtime: %v", err)
		}
	})
}
