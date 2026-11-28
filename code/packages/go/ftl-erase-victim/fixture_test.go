package ftlerasevictim

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

const (
	fixtureSchema = "coding-adventures.database-media.ftl-erase-victim.v1"
	fixtureLimit  = 64 * 1024
	fixtureDepth  = 32
)

type fixtureDocument struct {
	Schema string        `json:"schema"`
	Cases  []fixtureCase `json:"cases"`
}
type fixtureCase struct {
	Name          string           `json:"name"`
	Status        []string         `json:"status"`
	PagesPerBlock *int64           `json:"pages_per_block"`
	Expected      *fixtureExpected `json:"expected"`
}
type fixtureExpected struct {
	Found         *bool  `json:"found"`
	Block         *int64 `json:"block"`
	FirstPhysical *int64 `json:"first_physical"`
	PageCount     *int64 `json:"page_count"`
	StalePages    *int64 `json:"stale_pages"`
}

func TestCanonicalVictimFixtureModelFirst(t *testing.T) {
	doc := mustReadFixture(t)
	assertFixtureModelAndRuntime(t, doc)
}

func TestFixtureParserRejectsCaseAliasesBeforeModelReplay(t *testing.T) {
	original, err := os.ReadFile(fixturePath())
	if err != nil {
		t.Fatal(err)
	}
	for _, trial := range [][2]string{
		{`"schema"`, `"SCHEMA"`},
		{`"pages_per_block"`, `"PAGES_PER_BLOCK"`},
		{`"stale_pages"`, `"STALE_PAGES"`},
	} {
		t.Run(trial[1], func(t *testing.T) {
			changed := replaceFixtureAnchor(t, original, trial[0], trial[1])
			if _, err := parseFixture(changed); err == nil {
				t.Fatalf("case alias %s was accepted by the decoder", trial[1])
			}
		})
	}
	secondResult := replaceFixtureAnchor(t, original, `"stale_pages": 2`, `"stale_pages": 2, "STALE_PAGES": 2`)
	if _, err := parseFixture(secondResult); err == nil {
		t.Fatal("an exact expected result plus a case-variant member was accepted")
	}
}

func TestFixtureParserAllowsOneEscapedDecodedCanonicalKey(t *testing.T) {
	original, err := os.ReadFile(fixturePath())
	if err != nil {
		t.Fatal(err)
	}
	escaped := original
	for _, trial := range [][2]string{
		{`"schema"`, `"sche\u006da"`},
		{`"pages_per_block"`, `"pages_\u0070er_block"`},
		{`"stale_pages"`, `"stale_\u0070ages"`},
	} {
		escaped = replaceFixtureAnchor(t, escaped, trial[0], trial[1])
	}
	doc, err := parseFixture(escaped)
	if err != nil {
		t.Fatalf("one escaped spelling of each decoded key should be accepted: %v", err)
	}
	assertFixtureModelAndRuntime(t, doc)
}

func assertFixtureModelAndRuntime(t *testing.T, doc fixtureDocument) {
	t.Helper()
	if err := validateFixtureModel(doc); err != nil {
		t.Fatalf("independent fixture model: %v", err)
	}
	for _, item := range doc.Cases {
		status := fixtureStatuses(item.Status)
		got, found, err := Select(status, *item.PagesPerBlock)
		want := fixtureVictim(*item.Expected)
		if err != nil || found != *item.Expected.Found || got != want {
			t.Fatalf("%s runtime = %+v/%t/%v; want %+v/%t", item.Name, got, found, err, want, *item.Expected.Found)
		}
	}
}

func replaceFixtureAnchor(t *testing.T, source []byte, old, replacement string) []byte {
	t.Helper()
	if count := bytes.Count(source, []byte(old)); count == 0 {
		t.Fatalf("fixture anchor %q is missing", old)
	}
	changed := bytes.Replace(source, []byte(old), []byte(replacement), 1)
	if bytes.Equal(changed, source) {
		t.Fatalf("fixture replacement for %q had no effect", old)
	}
	return changed
}

func TestFixtureParserRejectsMalformedInputs(t *testing.T) {
	_ = mustReadFixture(t)
	original, err := os.ReadFile(fixturePath())
	if err != nil {
		t.Fatal(err)
	}
	bad := map[string][]byte{
		"duplicate key":     bytes.Replace(original, []byte(`"schema":`), []byte(`"schema":"x","schema":`), 1),
		"escaped duplicate": bytes.Replace(original, []byte(`"schema":`), []byte(`"sch\u0065ma":"x","schema":`), 1),
		"unknown field":     bytes.Replace(original, []byte(`"schema":`), []byte(`"unknown":0,"schema":`), 1),
		"truncated":         original[:len(original)-2],
		"trailing value":    append(append([]byte(nil), original...), []byte(`{}`)...),
		"invalid UTF-8":     append(append([]byte(nil), original...), 0xff),
		"oversized":         bytes.Repeat([]byte{' '}, fixtureLimit+1),
		"deep":              []byte(strings.Repeat("[", fixtureDepth+1) + strings.Repeat("]", fixtureDepth+1)),
		"unknown status":    bytes.Replace(original, []byte(`"stale"`), []byte(`"reservedish"`), 1),
		"fractional width":  replaceFixtureAnchor(t, original, `"pages_per_block": 4`, `"pages_per_block": 4.0`),
		"integer overflow":  replaceFixtureAnchor(t, original, `"stale_pages": 2`, `"stale_pages": 9223372036854775808`),
		"null result flag":  replaceFixtureAnchor(t, original, `"found": true`, `"found": null`),
	}
	for name, vector := range bad {
		t.Run(name, func(t *testing.T) {
			if _, err := parseFixture(vector); err == nil {
				t.Fatal("accepted malformed fixture")
			}
		})
	}
}

func TestWellTypedCorruptionFailsIndependentModel(t *testing.T) {
	raw, err := os.ReadFile(fixturePath())
	if err != nil {
		t.Fatal(err)
	}
	corrupted := bytes.Replace(raw, []byte(`"stale_pages": 2`), []byte(`"stale_pages": 1`), 1)
	if bytes.Equal(raw, corrupted) {
		t.Fatal("expected-value mutation anchor missing")
	}
	doc, err := parseFixture(corrupted)
	if err != nil {
		t.Fatalf("corruption should parse: %v", err)
	}
	if err := validateFixtureModel(doc); err == nil {
		t.Fatal("model accepted corrupted expected victim")
	}
}

func FuzzParseVictimFixture(f *testing.F) {
	path := fixturePath()
	if raw, err := os.ReadFile(path); err == nil {
		f.Add(raw)
	}
	f.Add([]byte(`{"schema":"wrong","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		doc, err := parseFixture(raw)
		if err != nil {
			return
		}
		if err := validateFixtureModel(doc); err != nil {
			return
		}
		for _, item := range doc.Cases {
			got, found, err := Select(fixtureStatuses(item.Status), *item.PagesPerBlock)
			if err != nil || found != *item.Expected.Found || got != fixtureVictim(*item.Expected) {
				t.Fatalf("model-accepted fixture disagrees with runtime: %s", item.Name)
			}
		}
	})
}

func FuzzVictimStatusVectors(f *testing.F) {
	f.Add([]byte{4, 2, 3, 0, 3, 0, 2})
	f.Add([]byte{2, 2, 0, 3, 0, 3})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) < 2 || len(raw) > maxPhysicalPages {
			return
		}
		width := int64(raw[0]%(byte(len(raw)-1)) + 1)
		status := make([]ftlpagemap.PhysicalStatus, len(raw)-1)
		model := make([]uint8, len(status))
		for i, value := range raw[1:] {
			status[i] = ftlpagemap.PhysicalStatus(value)
			model[i] = value
		}
		want, found, _ := referenceSelect(model, width)
		got, gotFound, err := Select(status, width)
		if len(status) < minPhysicalPages || len(status) > maxPhysicalPages || width < 1 || width > int64(len(status)) || len(status)%int(width) != 0 {
			if !errors.Is(err, ErrInvalidGeometry) || gotFound || got != (Victim{}) {
				t.Fatalf("invalid fuzz geometry returned %v", err)
			}
			return
		}
		if err != nil {
			if found || !errors.Is(err, ErrInvalidStatus) {
				t.Fatalf("unexpected refusal %v", err)
			}
			return
		}
		if gotFound != found || got != want {
			t.Fatalf("status vector mismatch: got %+v/%t want %+v/%t", got, gotFound, want, found)
		}
	})
}

func parseFixture(raw []byte) (fixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return fixtureDocument{}, errors.New("invalid fixture size or UTF-8")
	}
	if err := scanJSON(raw); err != nil {
		return fixtureDocument{}, err
	}
	if err := validateFixtureTokens(raw); err != nil {
		return fixtureDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var doc fixtureDocument
	if err := decoder.Decode(&doc); err != nil {
		return fixtureDocument{}, err
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != io.EOF {
		return fixtureDocument{}, errors.New("trailing JSON value")
	}
	return doc, nil
}

func validateFixtureTokens(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return err
	}
	root, ok := value.(map[string]any)
	if !ok || !exactFixtureKeys(root, "schema", "cases") {
		return errors.New("fixture requires exact document keys")
	}
	schema, ok := root["schema"].(string)
	if !ok || schema != fixtureSchema {
		return errors.New("fixture schema mismatch")
	}
	cases, ok := root["cases"].([]any)
	if !ok || len(cases) == 0 || len(cases) > 16 {
		return errors.New("invalid fixture case array")
	}
	for index, rawCase := range cases {
		item, ok := rawCase.(map[string]any)
		if !ok || !exactFixtureKeys(item, "name", "status", "pages_per_block", "expected") {
			return fmt.Errorf("case %d requires exact keys", index)
		}
		if name, ok := item["name"].(string); !ok || name == "" {
			return fmt.Errorf("case %d has invalid name", index)
		}
		width, err := exactFixtureInteger(item["pages_per_block"], 1, maxPhysicalPages)
		if err != nil {
			return fmt.Errorf("case %d pages_per_block: %w", index, err)
		}
		statuses, ok := item["status"].([]any)
		if !ok || len(statuses) < minPhysicalPages || len(statuses) > maxPhysicalPages || int64(len(statuses))%width != 0 {
			return fmt.Errorf("case %d has invalid status geometry", index)
		}
		for _, rawStatus := range statuses {
			status, ok := rawStatus.(string)
			if !ok || status != "free" && status != "reserved" && status != "live" && status != "stale" {
				return fmt.Errorf("case %d has invalid status token", index)
			}
		}
		expected, ok := item["expected"].(map[string]any)
		if !ok || !exactFixtureKeys(expected, "found", "block", "first_physical", "page_count", "stale_pages") {
			return fmt.Errorf("case %d expected result requires exact keys", index)
		}
		if _, ok := expected["found"].(bool); !ok {
			return fmt.Errorf("case %d expected found must be Boolean", index)
		}
		for _, key := range []string{"block", "first_physical", "page_count", "stale_pages"} {
			if _, err := exactFixtureInteger(expected[key], 0, maxPhysicalPages); err != nil {
				return fmt.Errorf("case %d expected %s: %w", index, key, err)
			}
		}
	}
	return nil
}

func exactFixtureKeys(object map[string]any, required ...string) bool {
	if len(object) != len(required) {
		return false
	}
	for _, key := range required {
		if _, found := object[key]; !found {
			return false
		}
	}
	return true
}

func exactFixtureInteger(value any, minimum, maximum int64) (int64, error) {
	number, ok := value.(json.Number)
	if !ok {
		return 0, errors.New("value must be an integer token")
	}
	parsed, err := strconv.ParseInt(number.String(), 10, 64)
	if err != nil || parsed < minimum || parsed > maximum {
		return 0, errors.New("integer token outside bounded field range")
	}
	return parsed, nil
}

func scanJSON(raw []byte) error {
	if err := validatePlanningSurrogateEscapes(raw); err != nil {
		return err
	}
	d := json.NewDecoder(bytes.NewReader(raw))
	d.UseNumber()
	var walk func(int) error
	walk = func(depth int) error {
		if depth > fixtureDepth {
			return errors.New("excessive JSON depth")
		}
		tok, err := d.Token()
		if err != nil {
			return err
		}
		delim, ok := tok.(json.Delim)
		if !ok {
			return nil
		}
		switch delim {
		case '{':
			seen := map[string]bool{}
			for d.More() {
				keyToken, err := d.Token()
				if err != nil {
					return err
				}
				key := keyToken.(string)
				if seen[key] {
					return fmt.Errorf("duplicate key %q", key)
				}
				seen[key] = true
				if err := walk(depth + 1); err != nil {
					return err
				}
			}
			_, err = d.Token()
			return err
		case '[':
			for d.More() {
				if err := walk(depth + 1); err != nil {
					return err
				}
			}
			_, err = d.Token()
			return err
		default:
			return errors.New("unexpected JSON delimiter")
		}
	}
	if err := walk(0); err != nil {
		return err
	}
	if _, err := d.Token(); err != io.EOF {
		return errors.New("trailing JSON value")
	}
	return nil
}

func validateFixtureModel(doc fixtureDocument) error {
	for _, item := range doc.Cases {
		want, found, _ := referenceSelect(fixtureRaw(item.Status), *item.PagesPerBlock)
		if found != *item.Expected.Found || want != fixtureVictim(*item.Expected) {
			return fmt.Errorf("case %q expected selection differs from model", item.Name)
		}
	}
	return nil
}
func fixtureRaw(in []string) []uint8 {
	out := make([]uint8, len(in))
	for i, s := range in {
		switch s {
		case "free":
			out[i] = 0
		case "reserved":
			out[i] = 1
		case "live":
			out[i] = 2
		case "stale":
			out[i] = 3
		}
	}
	return out
}
func fixtureStatuses(in []string) []ftlpagemap.PhysicalStatus {
	out := make([]ftlpagemap.PhysicalStatus, len(in))
	for i, v := range fixtureRaw(in) {
		out[i] = ftlpagemap.PhysicalStatus(v)
	}
	return out
}
func fixtureVictim(e fixtureExpected) Victim {
	return Victim{Block: *e.Block, FirstPhysical: *e.FirstPhysical, PageCount: *e.PageCount, StalePages: *e.StalePages}
}
func fixturePath() string {
	return filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-erase-victim.json")
}
func mustReadFixture(t testing.TB) fixtureDocument {
	t.Helper()
	raw, err := os.ReadFile(fixturePath())
	if err != nil {
		t.Fatal(err)
	}
	doc, err := parseFixture(raw)
	if err != nil {
		t.Fatal(err)
	}
	return doc
}
