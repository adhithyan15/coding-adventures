package ftlerasevictim

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"testing"
	"unicode/utf8"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

const copyFixtureSchema = "coding-adventures.database-media.ftl-copy-aware-victim.v1"

type copyFixtureDocument struct {
	Schema string            `json:"schema"`
	Cases  []copyFixtureCase `json:"cases"`
}

type copyFixtureCase struct {
	Name          string               `json:"name"`
	Status        []string             `json:"status"`
	PagesPerBlock *int64               `json:"pages_per_block"`
	Excluded      *[]bool              `json:"excluded_blocks"`
	Expected      *copyFixtureExpected `json:"expected"`
}

type copyFixtureExpected struct {
	Found            *bool  `json:"found"`
	Block            *int64 `json:"block"`
	FirstPhysical    *int64 `json:"first_physical"`
	PageCount        *int64 `json:"page_count"`
	StalePages       *int64 `json:"stale_pages"`
	LivePages        *int64 `json:"live_pages"`
	FreeDestinations *int64 `json:"free_destinations"`
}

func TestAllSmallCopyAwareLayoutsMatchIndependentEnumerator(t *testing.T) {
	checked := 0
	selected := 0
	for pageCount := 2; pageCount <= 6; pageCount++ {
		vectorCount := 1
		for range pageCount {
			vectorCount *= 4
		}
		for encoded := 0; encoded < vectorCount; encoded++ {
			status := make([]ftlpagemap.PhysicalStatus, pageCount)
			remaining := encoded
			for index := range status {
				status[index] = ftlpagemap.PhysicalStatus(remaining % 4)
				remaining /= 4
			}
			for pagesPerBlock := 1; pagesPerBlock <= pageCount; pagesPerBlock++ {
				if pageCount%pagesPerBlock != 0 {
					continue
				}
				blockCount := pageCount / pagesPerBlock
				for maskBits := 0; maskBits < 1<<blockCount; maskBits++ {
					mask := make([]bool, blockCount)
					for block := range mask {
						mask[block] = maskBits&(1<<block) != 0
					}
					beforeStatus := append([]ftlpagemap.PhysicalStatus(nil), status...)
					beforeMask := append([]bool(nil), mask...)
					want, wantFound, wantErr := referenceCopyAware(rawStatus(status), int64(pagesPerBlock), mask)
					got, gotFound, gotErr := SelectCopyAware(status, int64(pagesPerBlock), mask)
					if !sameErrorClass(gotErr, wantErr) || gotFound != wantFound || got != want {
						t.Fatalf("status=%v width=%d mask=%v => %+v/%t/%v; model %+v/%t/%v", status, pagesPerBlock, mask, got, gotFound, gotErr, want, wantFound, wantErr)
					}
					if gotFound {
						selected++
					}
					if !reflect.DeepEqual(status, beforeStatus) || !reflect.DeepEqual(mask, beforeMask) {
						t.Fatal("selection mutated an input slice")
					}
					checked++
				}
			}
		}
	}
	if checked < 300_000 || selected == 0 {
		t.Fatalf("bounded enumeration unexpectedly small: checked=%d selected=%d", checked, selected)
	}
	t.Logf("copy-aware exhaustive role/mask domain: %d cases, %d selected", checked, selected)
}

func TestCopyAwareRefusalPrecedenceAndZeroResults(t *testing.T) {
	unknown := []ftlpagemap.PhysicalStatus{ftlpagemap.Reserved, 255}
	for _, testCase := range []struct {
		name   string
		status []ftlpagemap.PhysicalStatus
		width  int64
		mask   []bool
		want   error
	}{
		{"geometry before mask and roles", unknown, 0, []bool{true}, ErrInvalidGeometry},
		{"mask before unknown role", unknown, 1, []bool{true}, ErrInvalidBlockMask},
		{"unknown role before reservation", unknown, 1, nil, ErrInvalidStatus},
		{"pending reservation in excluded block", []ftlpagemap.PhysicalStatus{ftlpagemap.Free, ftlpagemap.Reserved}, 1, []bool{false, true}, ErrReservationPending},
		{"unknown role in excluded block", []ftlpagemap.PhysicalStatus{ftlpagemap.Free, 255}, 1, []bool{false, true}, ErrInvalidStatus},
	} {
		t.Run(testCase.name, func(t *testing.T) {
			got, found, err := SelectCopyAware(testCase.status, testCase.width, testCase.mask)
			if !errors.Is(err, testCase.want) || found || got != (CopyAwareVictim{}) {
				t.Fatalf("refusal=%+v/%t/%v, want %v", got, found, err, testCase.want)
			}
		})
	}
}

func TestEmptyMaskAndStaleOnlyAgreement(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Free, ftlpagemap.Free,
		ftlpagemap.Stale, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free,
	}
	copyVictim, copyFound, err := SelectCopyAware(status, 4, nil)
	staleVictim, staleFound, staleErr := Select(status, 4)
	if err != nil || staleErr != nil || !copyFound || !staleFound ||
		copyVictim.Block != staleVictim.Block || copyVictim.StalePages != staleVictim.StalePages {
		t.Fatalf("stale-only agreement: copy=%+v/%t/%v stale=%+v/%t/%v", copyVictim, copyFound, err, staleVictim, staleFound, staleErr)
	}
	maskVictim, maskFound, maskErr := SelectCopyAware(status, 4, []bool{false, false})
	if maskErr != nil || !maskFound || maskVictim != copyVictim {
		t.Fatalf("all-false mask changed result: %+v/%t/%v", maskVictim, maskFound, maskErr)
	}
}

func TestCopyAwareSelectionIsolatesInputSlices(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free}
	mask := []bool{false, false}
	statusBefore := append([]ftlpagemap.PhysicalStatus(nil), status...)
	maskBefore := append([]bool(nil), mask...)
	_, _, _ = SelectCopyAware(status, 2, mask)
	if !reflect.DeepEqual(status, statusBefore) || !reflect.DeepEqual(mask, maskBefore) {
		t.Fatal("copy-aware selection modified caller state")
	}
}

func TestCopyAwareRatioBeatsStaleCount(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Live,
		ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Free, ftlpagemap.Free,
		ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free,
	}
	victim, found, err := SelectCopyAware(status, 4, nil)
	if err != nil || !found || victim.Block != 1 || victim.StalePages != 2 || victim.LivePages != 0 {
		t.Fatalf("copy-aware ratio must beat a larger stale count: %+v/%t/%v", victim, found, err)
	}
}

func TestCopyAwareEqualScoreTiesChooseLargerStaleCount(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free,
		ftlpagemap.Stale, ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Live, ftlpagemap.Live, ftlpagemap.Free,
		ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free,
	}
	victim, found, err := SelectCopyAware(status, 6, nil)
	if err != nil || !found || victim.Block != 1 || victim.StalePages != 2 {
		t.Fatalf("equal ratios must prefer the larger stale count: %+v/%t/%v", victim, found, err)
	}
}

func TestCopyAwareEqualScoreAndStaleCountTieChooseLowestIndex(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free,
		ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free,
		ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free, ftlpagemap.Free,
	}
	victim, found, err := SelectCopyAware(status, 4, nil)
	if err != nil || !found || victim.Block != 0 {
		t.Fatalf("equal ratio and stale count ties choose lowest block index: %+v/%t/%v", victim, found, err)
	}
}

func TestCopyAwareUnexcludedFirstBlockRemainsCandidate(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free, ftlpagemap.Free}
	victim, found, err := SelectCopyAware(status, 2, nil)
	if err != nil || !found || victim.Block != 0 {
		t.Fatalf("unexcluded block zero must remain a candidate: %+v/%t/%v", victim, found, err)
	}
}

func TestCopyAwareRequiresExternalSparePages(t *testing.T) {
	status := []ftlpagemap.PhysicalStatus{
		ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Live, ftlpagemap.Free,
		ftlpagemap.Free, ftlpagemap.Live, ftlpagemap.Live, ftlpagemap.Live,
	}
	victim, found, err := SelectCopyAware(status, 4, nil)
	if err != nil || found || victim != (CopyAwareVictim{}) {
		t.Fatalf("target-block free pages cannot be counted as external copy destinations: %+v/%t/%v", victim, found, err)
	}
}

func TestCopyAwareBoundaryGeometry(t *testing.T) {
	status := make([]ftlpagemap.PhysicalStatus, maxPhysicalPages)
	for index := range status {
		status[index] = ftlpagemap.Free
	}
	status[64], status[65], status[66] = ftlpagemap.Stale, ftlpagemap.Live, ftlpagemap.Free
	status[127] = ftlpagemap.Free
	victim, found, err := SelectCopyAware(status, 32, nil)
	if err != nil || !found || victim != (CopyAwareVictim{Block: 2, FirstPhysical: 64, PageCount: 32, StalePages: 1, LivePages: 1, FreeDestinations: 96}) {
		t.Fatalf("maximum geometry selection=%+v/%t/%v", victim, found, err)
	}
}

func TestCanonicalCopyAwareFixtureModelFirst(t *testing.T) {
	doc := mustReadCopyFixture(t)
	if err := validateCopyFixtureModel(doc); err != nil {
		t.Fatalf("independent copy-aware fixture model: %v", err)
	}
	for _, item := range doc.Cases {
		got, found, err := SelectCopyAware(copyFixtureStatuses(item.Status), *item.PagesPerBlock, *item.Excluded)
		want := copyFixtureVictim(*item.Expected)
		if err != nil || found != *item.Expected.Found || got != want {
			t.Fatalf("%s runtime=%+v/%t/%v want=%+v/%t", item.Name, got, found, err, want, *item.Expected.Found)
		}
	}
}

func TestCopyAwareParserRejectsMalformedAndModelMismatch(t *testing.T) {
	original, err := os.ReadFile(copyFixturePath())
	if err != nil {
		t.Fatal(err)
	}
	bad := map[string][]byte{
		"duplicate key":               bytes.Replace(original, []byte(`"schema":`), []byte(`"schema":"bad","schema":`), 1),
		"escaped duplicate":           bytes.Replace(original, []byte(`"schema":`), []byte(`"sch\u0065ma":"bad","schema":`), 1),
		"unknown field":               bytes.Replace(original, []byte(`"schema":`), []byte(`"unknown":0,"schema":`), 1),
		"truncated":                   original[:len(original)-2],
		"trailing value":              append(append([]byte(nil), original...), []byte(`{}`)...),
		"invalid UTF-8":               append(append([]byte(nil), original...), 0xff),
		"oversized":                   bytes.Repeat([]byte{' '}, fixtureLimit+1),
		"deep":                        []byte(stringsRepeat("[", fixtureDepth+1) + stringsRepeat("]", fixtureDepth+1)),
		"unknown status":              bytes.Replace(original, []byte(`"stale"`), []byte(`"reservedish"`), 1),
		"wrong dimensions":            bytes.Replace(original, []byte(`"pages_per_block": 4`), []byte(`"pages_per_block": 5`), 1),
		"null exclusion entry":        bytes.Replace(original, []byte(`"excluded_blocks": [false, true, false]`), []byte(`"excluded_blocks": [false, null, false]`), 1),
		"case-insensitive schema key": bytes.Replace(original, []byte(`"schema":`), []byte(`"SCHEMA":`), 1),
		"fractional block width":      bytes.Replace(original, []byte(`"pages_per_block": 4`), []byte(`"pages_per_block": 4.0`), 1),
		"unbounded expected integer":  bytes.Replace(original, []byte(`"stale_pages": 2`), []byte(`"stale_pages": 9223372036854775808`), 1),
	}
	for name, raw := range bad {
		t.Run(name, func(t *testing.T) {
			if _, err := parseCopyFixture(raw); err == nil {
				t.Fatal("accepted malformed copy-aware fixture")
			}
		})
	}
	corrupted := bytes.Replace(original, []byte(`"stale_pages": 2`), []byte(`"stale_pages": 1`), 1)
	if bytes.Equal(original, corrupted) {
		t.Fatal("copy-aware expected-value mutation anchor missing")
	}
	doc, err := parseCopyFixture(corrupted)
	if err != nil {
		t.Fatalf("well-typed corruption should parse: %v", err)
	}
	if err := validateCopyFixtureModel(doc); err == nil {
		t.Fatal("independent model accepted corrupted fixture claim")
	}
}

func TestSeededCopyAwareHistories(t *testing.T) {
	const seed = 0x434F5059
	rng := rand.New(rand.NewSource(seed))
	for step := 0; step < 8_000; step++ {
		pageCount := 2 + rng.Intn(31)
		status := make([]ftlpagemap.PhysicalStatus, pageCount)
		for index := range status {
			status[index] = ftlpagemap.PhysicalStatus(rng.Intn(4))
		}
		width := 1 + rng.Intn(pageCount)
		if pageCount%width != 0 {
			width = 1
		}
		mask := make([]bool, pageCount/width)
		for index := range mask {
			mask[index] = rng.Intn(4) == 0
		}
		want, wantFound, wantErr := referenceCopyAware(rawStatus(status), int64(width), mask)
		got, gotFound, gotErr := SelectCopyAware(status, int64(width), mask)
		if !sameErrorClass(gotErr, wantErr) || gotFound != wantFound || got != want {
			t.Fatalf("seed=%d step=%d status=%v width=%d mask=%v got=%+v/%t/%v want=%+v/%t/%v", seed, step, status, width, mask, got, gotFound, gotErr, want, wantFound, wantErr)
		}
	}
}

func FuzzParseCopyAwareFixture(f *testing.F) {
	if raw, err := os.ReadFile(copyFixturePath()); err == nil {
		f.Add(raw)
	}
	f.Add([]byte(`{"schema":"wrong","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureLimit {
			return
		}
		doc, err := parseCopyFixture(raw)
		if err != nil || validateCopyFixtureModel(doc) != nil {
			return
		}
		for _, item := range doc.Cases {
			got, found, err := SelectCopyAware(copyFixtureStatuses(item.Status), *item.PagesPerBlock, *item.Excluded)
			if err != nil || found != *item.Expected.Found || got != copyFixtureVictim(*item.Expected) {
				t.Fatalf("model-accepted fixture disagrees with runtime at %q", item.Name)
			}
		}
	})
}

func FuzzCopyAwareStatusVectors(f *testing.F) {
	f.Add([]byte{2, 3, 2, 0, 0, 3, 0})
	f.Add([]byte{4, 3, 2, 0, 0, 3, 3, 0, 0, 0, 0, 0, 0})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) < 2 || len(raw) > maxPhysicalPages+2 {
			return
		}
		width := int64(raw[0])
		status := make([]ftlpagemap.PhysicalStatus, len(raw)-1)
		for index, value := range raw[1:] {
			status[index] = ftlpagemap.PhysicalStatus(value & 7)
		}
		if width < 1 || width > int64(len(status)) || int64(len(status))%width != 0 {
			got, found, err := SelectCopyAware(status, width, nil)
			if !errors.Is(err, ErrInvalidGeometry) || found || got != (CopyAwareVictim{}) {
				t.Fatalf("invalid geometry refused as %+v/%t/%v", got, found, err)
			}
			return
		}
		blocks := len(status) / int(width)
		mask := make([]bool, blocks)
		for block := range mask {
			mask[block] = raw[(block+1)%(len(raw)-1)]&8 != 0
		}
		want, wantFound, wantErr := referenceCopyAware(rawStatus(status), width, mask)
		got, gotFound, gotErr := SelectCopyAware(status, width, mask)
		if !sameErrorClass(gotErr, wantErr) || gotFound != wantFound || got != want {
			t.Fatalf("status-vector disagreement got=%+v/%t/%v want=%+v/%t/%v", got, gotFound, gotErr, want, wantFound, wantErr)
		}
	})
}

func parseCopyFixture(raw []byte) (copyFixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureLimit || !utf8.Valid(raw) {
		return copyFixtureDocument{}, errors.New("invalid copy-aware fixture size or UTF-8")
	}
	if err := scanJSON(raw); err != nil {
		return copyFixtureDocument{}, err
	}
	if err := validateCopyFixtureTokens(raw); err != nil {
		return copyFixtureDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var doc copyFixtureDocument
	if err := decoder.Decode(&doc); err != nil {
		return copyFixtureDocument{}, err
	}
	return doc, nil
}

func validateCopyFixtureTokens(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return err
	}
	root, ok := value.(map[string]any)
	if !ok || !exactJSONKeys(root, "schema", "cases") {
		return errors.New("copy-aware fixture requires exact document keys")
	}
	schema, ok := root["schema"].(string)
	if !ok || schema != copyFixtureSchema {
		return errors.New("copy-aware fixture schema mismatch")
	}
	cases, ok := root["cases"].([]any)
	if !ok || len(cases) == 0 || len(cases) > 16 {
		return errors.New("invalid copy-aware fixture case array")
	}
	for index, item := range cases {
		caseObject, ok := item.(map[string]any)
		if !ok || !exactJSONKeys(caseObject, "name", "status", "pages_per_block", "excluded_blocks", "expected") {
			return fmt.Errorf("case %d requires exact keys", index)
		}
		if name, ok := caseObject["name"].(string); !ok || name == "" {
			return fmt.Errorf("case %d has invalid name", index)
		}
		width, err := exactJSONInteger(caseObject["pages_per_block"], 1, maxPhysicalPages)
		if err != nil {
			return fmt.Errorf("case %d pages_per_block: %w", index, err)
		}
		roles, ok := caseObject["status"].([]any)
		if !ok || len(roles) < minPhysicalPages || len(roles) > maxPhysicalPages || int64(len(roles))%width != 0 {
			return fmt.Errorf("case %d has invalid status geometry", index)
		}
		for _, role := range roles {
			name, ok := role.(string)
			if !ok || name != "free" && name != "reserved" && name != "live" && name != "stale" {
				return fmt.Errorf("case %d has invalid role token", index)
			}
		}
		mask, ok := caseObject["excluded_blocks"].([]any)
		blocks := len(roles) / int(width)
		if !ok || len(mask) != 0 && len(mask) != blocks {
			return fmt.Errorf("case %d has invalid exclusion mask width", index)
		}
		for _, excluded := range mask {
			if _, ok := excluded.(bool); !ok {
				return fmt.Errorf("case %d exclusion entries must be Boolean tokens", index)
			}
		}
		expected, ok := caseObject["expected"].(map[string]any)
		if !ok || !exactJSONKeys(expected, "found", "block", "first_physical", "page_count", "stale_pages", "live_pages", "free_destinations") {
			return fmt.Errorf("case %d expected object requires exact keys", index)
		}
		if _, ok := expected["found"].(bool); !ok {
			return fmt.Errorf("case %d expected found must be Boolean", index)
		}
		for _, key := range []string{"block", "first_physical", "page_count", "stale_pages", "live_pages", "free_destinations"} {
			if _, err := exactJSONInteger(expected[key], 0, maxPhysicalPages); err != nil {
				return fmt.Errorf("case %d expected %s: %w", index, key, err)
			}
		}
	}
	return nil
}

func exactJSONKeys(object map[string]any, required ...string) bool {
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

func exactJSONInteger(value any, minimum, maximum int64) (int64, error) {
	number, ok := value.(json.Number)
	if !ok {
		return 0, errors.New("value must be an integer token")
	}
	parsed, err := strconv.ParseInt(number.String(), 10, 64)
	if err != nil || parsed < minimum || parsed > maximum {
		return 0, errors.New("integer is outside its bounded field range")
	}
	return parsed, nil
}

func validateCopyFixtureModel(doc copyFixtureDocument) error {
	for _, item := range doc.Cases {
		want, found, err := referenceCopyAware(fixtureRaw(item.Status), *item.PagesPerBlock, *item.Excluded)
		if err != nil || found != *item.Expected.Found || want != copyFixtureVictim(*item.Expected) {
			return fmt.Errorf("case %q differs from independent selector model", item.Name)
		}
	}
	return nil
}

func referenceCopyAware(status []uint8, pagesPerBlock int64, excluded []bool) (CopyAwareVictim, bool, error) {
	count := int64(len(status))
	if count < minPhysicalPages || count > maxPhysicalPages || pagesPerBlock < 1 || pagesPerBlock > count || count%pagesPerBlock != 0 {
		return CopyAwareVictim{}, false, ErrInvalidGeometry
	}
	blocks := int(count / pagesPerBlock)
	if len(excluded) != 0 && len(excluded) != blocks {
		return CopyAwareVictim{}, false, ErrInvalidBlockMask
	}
	pending := false
	for _, role := range status {
		if role > uint8(ftlpagemap.Stale) {
			return CopyAwareVictim{}, false, ErrInvalidStatus
		}
		pending = pending || role == uint8(ftlpagemap.Reserved)
	}
	if pending {
		return CopyAwareVictim{}, false, ErrReservationPending
	}
	var best CopyAwareVictim
	bestScore := -1.0
	found := false
	for block := 0; block < blocks; block++ {
		if len(excluded) != 0 && excluded[block] {
			continue
		}
		first := block * int(pagesPerBlock)
		stale, live := 0, 0
		for _, role := range status[first : first+int(pagesPerBlock)] {
			if role == uint8(ftlpagemap.Stale) {
				stale++
			} else if role == uint8(ftlpagemap.Live) {
				live++
			}
		}
		if stale == 0 {
			continue
		}
		freeOutside := 0
		for other := 0; other < blocks; other++ {
			if other == block || len(excluded) != 0 && excluded[other] {
				continue
			}
			start := other * int(pagesPerBlock)
			for _, role := range status[start : start+int(pagesPerBlock)] {
				if role == uint8(ftlpagemap.Free) {
					freeOutside++
				}
			}
		}
		if freeOutside < live {
			continue
		}
		score := float64(stale) / float64(live+1)
		if !found || score > bestScore || score == bestScore && (int64(stale) > best.StalePages || int64(stale) == best.StalePages && int64(block) < best.Block) {
			found = true
			bestScore = score
			best = CopyAwareVictim{Block: int64(block), FirstPhysical: int64(first), PageCount: pagesPerBlock, StalePages: int64(stale), LivePages: int64(live), FreeDestinations: int64(freeOutside)}
		}
	}
	return best, found, nil
}

func copyFixtureStatuses(status []string) []ftlpagemap.PhysicalStatus {
	result := make([]ftlpagemap.PhysicalStatus, len(status))
	for index, role := range status {
		switch role {
		case "free":
			result[index] = ftlpagemap.Free
		case "reserved":
			result[index] = ftlpagemap.Reserved
		case "live":
			result[index] = ftlpagemap.Live
		case "stale":
			result[index] = ftlpagemap.Stale
		}
	}
	return result
}

func copyFixtureVictim(expected copyFixtureExpected) CopyAwareVictim {
	return CopyAwareVictim{Block: *expected.Block, FirstPhysical: *expected.FirstPhysical, PageCount: *expected.PageCount, StalePages: *expected.StalePages, LivePages: *expected.LivePages, FreeDestinations: *expected.FreeDestinations}
}

func copyFixturePath() string {
	return filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-copy-aware-victim.json")
}

func mustReadCopyFixture(t testing.TB) copyFixtureDocument {
	t.Helper()
	raw, err := os.ReadFile(copyFixturePath())
	if err != nil {
		t.Fatal(err)
	}
	doc, err := parseCopyFixture(raw)
	if err != nil {
		t.Fatal(err)
	}
	return doc
}

func sameErrorClass(actual, expected error) bool {
	if actual == nil || expected == nil {
		return actual == nil && expected == nil
	}
	return errors.Is(actual, expected) && errors.Is(expected, actual)
}

func stringsRepeat(value string, count int) string {
	result := ""
	for range count {
		result += value
	}
	return result
}
