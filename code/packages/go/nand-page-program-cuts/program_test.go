package nandpageprogramcuts

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math/rand"
	"os"
	"reflect"
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

const (
	fixtureMaxBytes    = 16 * 1024
	fixtureMaxCases    = 32
	fixtureMaxDepth    = 16
	pageSizeForTests   = 4
	maxHistoryBytes    = 4096
	maxHistoryCommands = 32
)

type expectedError uint8

const (
	noExpectedError expectedError = iota
	invalidPageSizeError
	invalidPageStateError
	wrongPayloadLengthError
	invalidCutError
	alreadyProgrammedError
)

type fixtureCase struct {
	name     string
	pageSize int
	before   PageState
	payload  []byte
	cut      Cut
	want     Outcome
	errCode  expectedError
}

type fixtureDisposition uint8

const (
	fixtureRefused fixtureDisposition = iota
	fixtureAcceptedAndMatched
)

func TestEverySmallPagePayloadAndCutAndRetryMatchesModel(t *testing.T) {
	alphabet := []byte{0x00, 0xff, 0xa5, 0x5a}
	combinations := 0
	apiCalls := 0
	for pageSize := 1; pageSize <= 4; pageSize++ {
		cutCases := []Cut{{Kind: BeforeAccept}, {Kind: AfterMedia}, {Kind: Complete}}
		for prefix := 0; prefix < pageSize; prefix++ {
			cutCases = append(cutCases, Cut{Kind: AfterPrefix, PrefixBytes: prefix})
		}
		payloadCount := integerPower(len(alphabet), pageSize)
		for encoded := 0; encoded < payloadCount; encoded++ {
			payload := make([]byte, pageSize)
			value := encoded
			for index := range payload {
				payload[index] = alphabet[value%len(alphabet)]
				value /= len(alphabet)
			}
			for _, cut := range cutCases {
				before := erasedPage(pageSize)
				want, wantErr := referenceApply(pageSize, before, payload, cut)
				got, gotErr := Apply(pageSize, before, payload, cut)
				assertTransition(t, got, gotErr, want, wantErr)
				apiCalls++
				combinations++

				// A retry with all ones distinguishes a clean pre-accept refusal
				// from a zero-prefix program whose bytes still all equal 0xff.
				retryPayload := erasedBytes(pageSize)
				retryCut := Cut{Kind: Complete}
				retryWant, retryWantErr := referenceApply(pageSize, got.State, retryPayload, retryCut)
				retryGot, retryGotErr := Apply(pageSize, got.State, retryPayload, retryCut)
				assertTransition(t, retryGot, retryGotErr, retryWant, retryWantErr)
				apiCalls++
			}
		}
	}
	if combinations != 2272 || apiCalls != 4544 {
		t.Fatalf("enumerated %d cut cases and %d API transitions, want 2272 and 4544", combinations, apiCalls)
	}
}

func TestPublicLimitsPrecedenceAtomicRefusalsAndOwnership(t *testing.T) {
	for _, pageSize := range []int{-1, 0, maxPageSize + 1, int(^uint(0) >> 1)} {
		before := PageState{}
		got, err := Apply(pageSize, before, nil, Cut{Kind: CutKind(255), PrefixBytes: -1})
		assertRefusal(t, got, err, ErrInvalidPageSize)
	}

	valid := erasedPage(2)
	badState := PageState{Bytes: []byte{0x00, 0xff}}
	got, err := Apply(2, badState, nil, Cut{Kind: CutKind(255)})
	assertRefusal(t, got, err, ErrInvalidPageState)
	got, err = Apply(2, PageState{Bytes: []byte{0xff}}, nil, Cut{Kind: CutKind(255)})
	assertRefusal(t, got, err, ErrInvalidPageState)

	got, err = Apply(2, valid, []byte{0x00}, Cut{Kind: CutKind(255)})
	assertRefusal(t, got, err, ErrWrongPayloadLength)
	got, err = Apply(2, valid, []byte{0x00, 0x11}, Cut{Kind: AfterPrefix, PrefixBytes: 2})
	assertRefusal(t, got, err, ErrInvalidCut)
	got, err = Apply(2, valid, []byte{0x00, 0x11}, Cut{Kind: BeforeAccept, PrefixBytes: 1})
	assertRefusal(t, got, err, ErrInvalidCut)

	consumed := PageState{Bytes: []byte{0xff, 0xff}, Programmed: true}
	got, err = Apply(2, consumed, []byte{0x00, 0x11}, Cut{Kind: CutKind(255)})
	assertRefusal(t, got, err, ErrInvalidCut)
	got, err = Apply(2, consumed, []byte{0x00, 0x11}, Cut{Kind: Complete})
	assertRefusal(t, got, err, ErrAlreadyProgrammed)

	for _, cut := range []Cut{
		{Kind: BeforeAccept},
		{Kind: AfterPrefix, PrefixBytes: 0},
		{Kind: AfterPrefix, PrefixBytes: 1},
		{Kind: AfterMedia},
		{Kind: Complete},
	} {
		before := erasedPage(2)
		payload := []byte{0x12, 0x34}
		out, applyErr := Apply(2, before, payload, cut)
		if applyErr != nil {
			t.Fatalf("Apply(%+v) refused valid input: %v", cut, applyErr)
		}
		out.State.Bytes[0] ^= 0xff
		payload[1] ^= 0xff
		if before.Bytes[0] != 0xff || before.Bytes[1] != 0xff {
			t.Fatalf("successful call mutated caller's before page for cut %+v", cut)
		}
		if out.State.Bytes[1] == payload[1] && cut.Kind != BeforeAccept {
			t.Fatalf("outcome aliased payload for cut %+v", cut)
		}
	}

	allOnes, err := Apply(1, erasedPage(1), []byte{0xff}, Cut{Kind: AfterPrefix, PrefixBytes: 0})
	if err != nil || !allOnes.State.Programmed || allOnes.State.Bytes[0] != 0xff || allOnes.Acknowledged {
		t.Fatalf("zero-prefix all-ones outcome = %+v/%v", allOnes, err)
	}
	if _, err := Apply(1, allOnes.State, []byte{0x00}, Cut{Kind: Complete}); !errors.Is(err, ErrAlreadyProgrammed) {
		t.Fatalf("all-ones marked page retry error = %v; want already programmed", err)
	}
}

func TestDomainErrorMessagesAreStable(t *testing.T) {
	for _, tc := range []struct {
		err  error
		text string
	}{
		{ErrInvalidPageSize, "page size outside supported range"},
		{ErrInvalidPageState, "invalid NAND page state"},
		{ErrWrongPayloadLength, "payload length must equal page size"},
		{ErrInvalidCut, "invalid NAND program cut"},
		{ErrAlreadyProgrammed, "page has already been programmed since erase"},
	} {
		if tc.err.Error() != tc.text {
			t.Errorf("error text %q; want %q", tc.err.Error(), tc.text)
		}
	}
}

func TestMaximumPageSizeCompleteAndDetached(t *testing.T) {
	before := erasedPage(maxPageSize)
	payload := make([]byte, maxPageSize)
	for i := range payload {
		payload[i] = byte(i * 37)
	}
	out, err := Apply(maxPageSize, before, payload, Cut{Kind: Complete})
	if err != nil || !out.Acknowledged || !out.State.Programmed || !bytes.Equal(out.State.Bytes, payload) {
		t.Fatalf("maximum page complete outcome mismatched: ack=%t marker=%t err=%v", out.Acknowledged, out.State.Programmed, err)
	}
	out.State.Bytes[0] ^= 0xff
	if before.Bytes[0] != 0xff || payload[0] == out.State.Bytes[0] {
		t.Fatal("maximum page result was not detached")
	}
}

func TestAllCutKindsHaveExactByteAndAcknowledgementResults(t *testing.T) {
	before := PageState{Bytes: []byte{0xff, 0xff, 0xff, 0xff}}
	payload := []byte{0x10, 0x20, 0x30, 0x40}
	tests := []struct {
		cut   Cut
		bytes []byte
		mark  bool
		ack   bool
	}{
		{Cut{Kind: BeforeAccept}, []byte{0xff, 0xff, 0xff, 0xff}, false, false},
		{Cut{Kind: AfterPrefix, PrefixBytes: 0}, []byte{0xff, 0xff, 0xff, 0xff}, true, false},
		{Cut{Kind: AfterPrefix, PrefixBytes: 2}, []byte{0x10, 0x20, 0xff, 0xff}, true, false},
		{Cut{Kind: AfterPrefix, PrefixBytes: 3}, []byte{0x10, 0x20, 0x30, 0xff}, true, false},
		{Cut{Kind: AfterMedia}, payload, true, false},
		{Cut{Kind: Complete}, payload, true, true},
	}
	for _, tc := range tests {
		got, err := Apply(4, before, payload, tc.cut)
		if err != nil || !bytes.Equal(got.State.Bytes, tc.bytes) || got.State.Programmed != tc.mark || got.Acknowledged != tc.ack {
			t.Errorf("Apply(%+v) = %+v/%v; want bytes %x marker %t ack %t", tc.cut, got, err, tc.bytes, tc.mark, tc.ack)
		}
	}
}

func TestPrefixCutDoesNotProgramSuffix(t *testing.T) {
	out, err := Apply(4, erasedPage(4), []byte{0x10, 0x20, 0x30, 0x40}, Cut{Kind: AfterPrefix, PrefixBytes: 2})
	if err != nil || !bytes.Equal(out.State.Bytes, []byte{0x10, 0x20, 0xff, 0xff}) {
		t.Fatalf("prefix-two persisted bytes=%x err=%v", out.State.Bytes, err)
	}
}

func TestPrefixCutConsumesProgramOpportunity(t *testing.T) {
	out, err := Apply(4, erasedPage(4), []byte{0xff, 0xff, 0xff, 0xff}, Cut{Kind: AfterPrefix, PrefixBytes: 0})
	if err != nil || !out.State.Programmed || out.Acknowledged {
		t.Fatalf("zero-prefix result=%+v err=%v; want marked and unacknowledged", out, err)
	}
	retry, retryErr := Apply(4, out.State, []byte{0, 0, 0, 0}, Cut{Kind: Complete})
	assertRefusal(t, retry, retryErr, ErrAlreadyProgrammed)
}

func TestUnacknowledgedCutsRemainUnacknowledged(t *testing.T) {
	for _, cut := range []Cut{
		{Kind: BeforeAccept},
		{Kind: AfterPrefix, PrefixBytes: 2},
		{Kind: AfterMedia},
	} {
		out, err := Apply(4, erasedPage(4), []byte{0x11, 0x22, 0x33, 0x44}, cut)
		if err != nil || out.Acknowledged {
			t.Errorf("Apply(%+v) ack=%t err=%v; want no acknowledgement", cut, out.Acknowledged, err)
		}
	}
}

func TestFiveThousandSeededBoundedCommandsMatchIndependentModel(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB04B1))
	const pageCount = 256
	states := make([]PageState, pageCount)
	for i := range states {
		states[i] = erasedPage(8)
	}
	var successes, refusals int
	for index := 0; index < 5000; index++ {
		page := rng.Intn(pageCount)
		state := states[page]
		payload := make([]byte, 8)
		if index%7 == 0 {
			for j := range payload {
				payload[j] = 0xff
			}
		} else {
			_, _ = rng.Read(payload)
		}
		kind := CutKind(rng.Intn(7))
		prefix := rng.Intn(10) - 2
		if index%11 == 0 {
			kind = BeforeAccept
			prefix = 0
		}
		cut := Cut{Kind: kind, PrefixBytes: prefix}
		want, wantErr := referenceApply(8, clonePage(state), append([]byte(nil), payload...), cut)
		got, gotErr := Apply(8, state, payload, cut)
		assertTransition(t, got, gotErr, want, wantErr)
		if gotErr == nil {
			successes++
			states[page] = got.State
		} else {
			refusals++
			if !reflect.DeepEqual(got, Outcome{}) {
				t.Fatalf("command %d refusal returned nonzero outcome: %+v", index, got)
			}
		}
	}
	if successes == 0 || refusals == 0 {
		t.Fatalf("seeded trace covered successes=%d refusals=%d", successes, refusals)
	}
	t.Logf("seed 0xDB04B1 checked 5000 commands: %d accepted, %d refused", successes, refusals)
}

func TestFixtureHasValidIndependentOracleAndRuntimeReplay(t *testing.T) {
	data, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		t.Fatal(err)
	}
	fixture, err := parseFixture(data)
	if err != nil {
		t.Fatal(err)
	}
	if len(fixture) < 10 || len(fixture) > fixtureMaxCases {
		t.Fatalf("fixture has %d cases outside expected bounds", len(fixture))
	}
	if got, err := checkFixtureBytes(data); err != nil || got != fixtureAcceptedAndMatched {
		t.Fatalf("fixture adapter disposition=%v err=%v", got, err)
	}
}

func TestMalformedFixtureShapesRefuseBeforeReplay(t *testing.T) {
	valid, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		t.Fatal(err)
	}
	tooManyCases := fixtureWithRepeatedCases(t, valid, fixtureMaxCases+1)
	for name, data := range map[string][]byte{
		"oversize":           bytes.Repeat([]byte{' '}, fixtureMaxBytes+1),
		"invalid-utf8":       append([]byte{'\xff'}, valid...),
		"trailing":           append(append([]byte(nil), valid...), []byte(" {}")...),
		"duplicate-key":      bytes.Replace(valid, []byte(`"schema":`), []byte(`"schema":"duplicate","schema":`), 1),
		"wrong-schema":       bytes.Replace(valid, []byte("coding-adventures.database-media.nand-page-program-cuts.v1"), []byte("other.schema"), 1),
		"deep":               []byte(strings.Repeat("[", fixtureMaxDepth+1) + "0" + strings.Repeat("]", fixtureMaxDepth+1)),
		"unknown-case-field": bytes.Replace(valid, []byte(`"name": "before-accept"`), []byte(`"unexpected": 1, "name": "before-accept"`), 1),
		"uppercase-hex":      bytes.Replace(valid, []byte(`"bytes": "ffffffff"`), []byte(`"bytes": "FFFFFFFF"`), 1),
		"float-integer":      bytes.Replace(valid, []byte(`"page_size": 4`), []byte(`"page_size": 4.0`), 1),
		"boolean-integer":    bytes.Replace(valid, []byte(`"kind": 0`), []byte(`"kind": false`), 1),
		"integer-overflow":   bytes.Replace(valid, []byte(`"page_size": 4`), []byte(`"page_size": 2147483648`), 1),
		"unknown-error":      bytes.Replace(valid, []byte(`"invalid_cut"`), []byte(`"not_a_domain_error"`), 1),
		"too-many-cases":     tooManyCases,
	} {
		t.Run(name, func(t *testing.T) {
			got, fuzzErr := checkFixtureBytes(data)
			if fuzzErr != nil || got != fixtureRefused {
				t.Fatalf("malformed vector disposition=%v err=%v; want refusal", got, fuzzErr)
			}
		})
	}
}

func fixtureWithRepeatedCases(t *testing.T, data []byte, count int) []byte {
	t.Helper()
	var document struct {
		Schema string            `json:"schema"`
		Cases  []json.RawMessage `json:"cases"`
	}
	if err := json.Unmarshal(data, &document); err != nil || len(document.Cases) == 0 {
		t.Fatalf("decode canonical fixture for case cap: %v", err)
	}
	cases := make([]json.RawMessage, count)
	for index := range cases {
		cases[index] = document.Cases[0]
	}
	encoded, err := json.Marshal(struct {
		Schema string            `json:"schema"`
		Cases  []json.RawMessage `json:"cases"`
	}{Schema: document.Schema, Cases: cases})
	if err != nil {
		t.Fatal(err)
	}
	return encoded
}

func TestFixtureAdapterTreatsCorruptedExpectedOutputAsRefusal(t *testing.T) {
	data, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		t.Fatal(err)
	}
	mutated := bytes.Replace(data, []byte(`"acknowledged": true`), []byte(`"acknowledged": false`), 1)
	if bytes.Equal(mutated, data) {
		t.Fatal("fixture corruption anchor did not match")
	}
	got, fuzzErr := checkFixtureBytes(mutated)
	if fuzzErr != nil || got != fixtureRefused {
		t.Fatalf("corrupted expected output disposition=%v err=%v; want refusal", got, fuzzErr)
	}
}

func TestLateWrongDeclarationIsRejectedBeforeAnyRuntimeReplay(t *testing.T) {
	data, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		t.Fatal(err)
	}
	mutated := bytes.Replace(data, []byte(`"page_size": 0`), []byte(`"page_size": 1`), 1)
	if bytes.Equal(mutated, data) {
		t.Fatal("late fixture corruption anchor did not match")
	}
	calls := 0
	countedApply := func(size int, before PageState, payload []byte, cut Cut) (Outcome, error) {
		calls++
		return Apply(size, before, payload, cut)
	}
	got, checkErr := checkFixtureBytesWithApply(mutated, countedApply)
	if checkErr != nil || got != fixtureRefused {
		t.Fatalf("late wrong declaration disposition=%v err=%v; want model refusal", got, checkErr)
	}
	if calls != 0 {
		t.Fatalf("model-inconsistent late declaration allowed %d runtime calls before refusal", calls)
	}
}

func TestFixtureRejectsMalformedEscapedUnicodeScalars(t *testing.T) {
	canonical, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		t.Fatal(err)
	}
	const oldName = `"name": "before-accept"`
	cases := map[string]string{
		"isolated high surrogate":   `"name": "before-\ud800"`,
		"isolated low surrogate":    `"name": "before-\udc00"`,
		"reversed surrogate pair":   `"name": "before-\udc00\ud800"`,
		"incomplete unicode escape": `"name": "before-\ud80"`,
	}
	for name, replacement := range cases {
		t.Run(name, func(t *testing.T) {
			mutated := bytes.Replace(canonical, []byte(oldName), []byte(replacement), 1)
			if bytes.Equal(mutated, canonical) {
				t.Fatal("fixture name mutation anchor did not match")
			}
			got, checkErr := checkFixtureBytes(mutated)
			if checkErr != nil || got != fixtureRefused {
				t.Fatalf("malformed scalar disposition=%v err=%v; want refusal", got, checkErr)
			}
		})
	}
}

func TestFixtureAcceptsUnicodeScalarsAndEscapedBackslash(t *testing.T) {
	canonical, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		t.Fatal(err)
	}
	const oldName = `"name": "before-accept"`
	positive := map[string]string{
		"valid surrogate pair":       `"name": "before-\ud83d\ude80"`,
		"literal replacement scalar": `"name": "before-�"`,
		"escaped replacement scalar": `"name": "before-\ufffd"`,
		"escaped backslash":          `"name": "before-\\ud800"`,
	}
	for name, replacement := range positive {
		t.Run(name, func(t *testing.T) {
			mutated := bytes.Replace(canonical, []byte(oldName), []byte(replacement), 1)
			if bytes.Equal(mutated, canonical) {
				t.Fatal("fixture name mutation anchor did not match")
			}
			got, checkErr := checkFixtureBytes(mutated)
			if checkErr != nil || got != fixtureAcceptedAndMatched {
				t.Fatalf("well-formed scalar disposition=%v err=%v; want accepted", got, checkErr)
			}
		})
	}
}

func TestSurrogateAdmissionControlReproducesEncodingJSONNormalization(t *testing.T) {
	canonical, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		t.Fatal(err)
	}
	for _, escaped := range []string{`\ud800`, `\udc00`} {
		mutated := bytes.Replace(canonical, []byte(`"name": "before-accept"`),
			[]byte(`"name": "before-`+escaped+`"`), 1)
		if bytes.Equal(mutated, canonical) {
			t.Fatalf("isolated surrogate control anchor did not match: %s", escaped)
		}
		cases, parseErr := parseFixtureJSON(mutated)
		if parseErr != nil {
			t.Fatalf("decoder bypass did not reproduce legacy acceptance for %s: %v", escaped, parseErr)
		}
		if len(cases) == 0 || cases[0].name != "before-�" {
			t.Fatalf("decoder bypass did not normalize %s to replacement scalar: %+v", escaped, cases)
		}
		got, replayErr := compareFixtureCases(cases, Apply)
		if replayErr != nil || got != fixtureAcceptedAndMatched {
			t.Fatalf("decoder bypass should show old acceptance for %s: disposition=%v err=%v", escaped, got, replayErr)
		}
	}
}

func TestBoundedCommandHistoriesCompareEveryPageAndRetry(t *testing.T) {
	const pages = 8
	states := make([]PageState, pages)
	for i := range states {
		states[i] = erasedPage(pageSizeForTests)
	}
	inputs := []byte{
		0, byte(BeforeAccept), 0, 0, 0x01, 0x02, 0x03, 0x04,
		1, byte(AfterPrefix), 0, 0, 0xff, 0xff, 0xff, 0xff,
		1, byte(Complete), 0, 0, 0, 0, 0, 0,
		2, byte(AfterPrefix), 3, 0, 0xaa, 0xbb, 0xcc, 0xdd,
		2, byte(Complete), 0, 0, 0xff, 0xff, 0xff, 0xff,
	}
	checkCommandHistory(t, inputs, states)
}

func FuzzParseFixture(f *testing.F) {
	canonical, err := os.ReadFile("../../../specs/fixtures/database-media-v1/nand-page-program-cuts.json")
	if err != nil {
		f.Fatal(err)
	}
	f.Add(canonical)
	f.Add([]byte(`{"schema":`))
	f.Add([]byte("[]"))
	f.Add(bytes.Repeat([]byte{' '}, fixtureMaxBytes+1))
	f.Fuzz(func(t *testing.T, data []byte) {
		disposition, fuzzErr := checkFixtureBytes(data)
		if fuzzErr != nil {
			t.Fatalf("valid model/runtime disagreement: %v", fuzzErr)
		}
		if disposition != fixtureRefused && disposition != fixtureAcceptedAndMatched {
			t.Fatalf("unknown fixture disposition %d", disposition)
		}
	})
}

func FuzzCommandHistory(f *testing.F) {
	f.Add([]byte{
		0, byte(Complete), 0, 0, 0x11, 0x22, 0x33, 0x44,
		1, byte(AfterPrefix), 2, 0, 0xff, 0x00, 0xa5, 0x5a,
		0, byte(Complete), 0, 0, 0xff, 0xff, 0xff, 0xff,
	})
	f.Add([]byte{0xff, 0xff, 0xff, 1, 0, 0, 0, 0})
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > maxHistoryBytes {
			data = data[:maxHistoryBytes]
		}
		states := make([]PageState, 8)
		for i := range states {
			states[i] = erasedPage(pageSizeForTests)
		}
		checkCommandHistory(t, data, states)
	})
}

func checkCommandHistory(t *testing.T, input []byte, states []PageState) {
	t.Helper()
	bounded := input
	if len(bounded) > maxHistoryBytes {
		bounded = bounded[:maxHistoryBytes]
	}
	count := len(bounded) / 8
	if count > maxHistoryCommands {
		count = maxHistoryCommands
	}
	for index := 0; index < count; index++ {
		command := bounded[index*8 : index*8+8]
		page := int(command[0]) % len(states)
		kind := CutKind(command[1])
		prefix := int(int8(command[2]))
		payload := append([]byte(nil), command[4:8]...)
		cut := Cut{Kind: kind, PrefixBytes: prefix}
		before := clonePage(states[page])
		want, wantErr := referenceApply(pageSizeForTests, before, payload, cut)
		got, gotErr := Apply(pageSizeForTests, states[page], payload, cut)
		assertTransition(t, got, gotErr, want, wantErr)
		if gotErr == nil {
			states[page] = got.State
		}
	}
}

func parseFixture(data []byte) ([]fixtureCase, error) {
	if len(data) == 0 || len(data) > fixtureMaxBytes || !utf8.Valid(data) {
		return nil, fmt.Errorf("fixture outside byte or UTF-8 limit")
	}
	if err := validateSurrogateEscapes(data); err != nil {
		return nil, err
	}
	return parseFixtureJSON(data)
}

// parseFixtureJSON is the structural decoder after the normal entry point's
// byte, UTF-8, and scalar checks. The only bypass caller is a test supplying
// the known bounded UTF-8 canonical document to demonstrate old decoder behavior.
func parseFixtureJSON(data []byte) ([]fixtureCase, error) {
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	root, err := parseJSONValue(decoder, 0)
	if err != nil {
		return nil, err
	}
	if _, err := decoder.Token(); err != io.EOF {
		return nil, fmt.Errorf("trailing JSON data")
	}
	rootObject, ok := root.(map[string]any)
	if !ok || !exactKeys(rootObject, "schema", "cases") {
		return nil, fmt.Errorf("invalid fixture root")
	}
	schema, ok := rootObject["schema"].(string)
	if !ok || schema != "coding-adventures.database-media.nand-page-program-cuts.v1" {
		return nil, fmt.Errorf("unsupported fixture schema")
	}
	rawCases, ok := rootObject["cases"].([]any)
	if !ok || len(rawCases) == 0 || len(rawCases) > fixtureMaxCases {
		return nil, fmt.Errorf("invalid fixture case list")
	}
	cases := make([]fixtureCase, 0, len(rawCases))
	for index, rawCase := range rawCases {
		parsed, err := parseFixtureCase(rawCase)
		if err != nil {
			return nil, fmt.Errorf("case %d: %w", index, err)
		}
		cases = append(cases, parsed)
	}
	return cases, nil
}

func parseFixtureCase(raw any) (fixtureCase, error) {
	object, ok := raw.(map[string]any)
	if !ok {
		return fixtureCase{}, fmt.Errorf("case must be an object")
	}
	if !exactKeys(object, "name", "page_size", "before", "payload", "cut", "result") &&
		!exactKeys(object, "name", "page_size", "before", "payload", "cut", "error") {
		return fixtureCase{}, fmt.Errorf("case keys do not match the schema")
	}
	name, ok := object["name"].(string)
	if !ok || strings.TrimSpace(name) == "" {
		return fixtureCase{}, fmt.Errorf("case name must be nonempty text")
	}
	pageSize, ok := signedInt32(object["page_size"])
	if !ok {
		return fixtureCase{}, fmt.Errorf("page_size must be a signed 32-bit integer token")
	}
	before, err := parseState(object["before"])
	if err != nil {
		return fixtureCase{}, fmt.Errorf("before state: %w", err)
	}
	payloadHex, ok := object["payload"].(string)
	if !ok {
		return fixtureCase{}, fmt.Errorf("payload must be lowercase hex text")
	}
	payload, err := decodeLowerHex(payloadHex)
	if err != nil {
		return fixtureCase{}, fmt.Errorf("payload: %w", err)
	}
	cut, err := parseCut(object["cut"])
	if err != nil {
		return fixtureCase{}, fmt.Errorf("cut: %w", err)
	}
	parsed := fixtureCase{name: name, pageSize: int(pageSize), before: before, payload: payload, cut: cut}
	if rawResult, exists := object["result"]; exists {
		parsed.want, err = parseOutcome(rawResult)
		if err != nil {
			return fixtureCase{}, fmt.Errorf("result: %w", err)
		}
	} else {
		code, ok := object["error"].(string)
		if !ok {
			return fixtureCase{}, fmt.Errorf("error code must be text")
		}
		parsed.errCode, ok = errorCode(code)
		if !ok {
			return fixtureCase{}, fmt.Errorf("unknown error code")
		}
	}
	return parsed, nil
}

func parseState(raw any) (PageState, error) {
	object, ok := raw.(map[string]any)
	if !ok || !exactKeys(object, "bytes", "programmed") {
		return PageState{}, fmt.Errorf("state keys do not match the schema")
	}
	hexText, ok := object["bytes"].(string)
	if !ok {
		return PageState{}, fmt.Errorf("state bytes must be lowercase hex text")
	}
	data, err := decodeLowerHex(hexText)
	if err != nil {
		return PageState{}, err
	}
	programmed, ok := object["programmed"].(bool)
	if !ok {
		return PageState{}, fmt.Errorf("programmed must be boolean")
	}
	return PageState{Bytes: data, Programmed: programmed}, nil
}

func parseCut(raw any) (Cut, error) {
	object, ok := raw.(map[string]any)
	if !ok || !exactKeys(object, "kind", "prefix_bytes") {
		return Cut{}, fmt.Errorf("cut keys do not match the schema")
	}
	kind, ok := signedInt32(object["kind"])
	if !ok || kind < 0 || kind > 255 {
		return Cut{}, fmt.Errorf("cut kind must be an integer byte")
	}
	prefix, ok := signedInt32(object["prefix_bytes"])
	if !ok {
		return Cut{}, fmt.Errorf("prefix_bytes must be a signed 32-bit integer token")
	}
	return Cut{Kind: CutKind(kind), PrefixBytes: int(prefix)}, nil
}

func parseOutcome(raw any) (Outcome, error) {
	object, ok := raw.(map[string]any)
	if !ok || !exactKeys(object, "state", "acknowledged") {
		return Outcome{}, fmt.Errorf("outcome keys do not match the schema")
	}
	state, err := parseState(object["state"])
	if err != nil {
		return Outcome{}, err
	}
	acknowledged, ok := object["acknowledged"].(bool)
	if !ok {
		return Outcome{}, fmt.Errorf("acknowledged must be boolean")
	}
	return Outcome{State: state, Acknowledged: acknowledged}, nil
}

func parseJSONValue(decoder *json.Decoder, depth int) (any, error) {
	if depth > fixtureMaxDepth {
		return nil, fmt.Errorf("JSON nesting exceeds limit")
	}
	token, err := decoder.Token()
	if err != nil {
		return nil, err
	}
	switch value := token.(type) {
	case json.Delim:
		switch value {
		case '{':
			object := make(map[string]any)
			for decoder.More() {
				keyToken, err := decoder.Token()
				if err != nil {
					return nil, err
				}
				key, ok := keyToken.(string)
				if !ok {
					return nil, fmt.Errorf("object key is not text")
				}
				if _, exists := object[key]; exists {
					return nil, fmt.Errorf("duplicate object key")
				}
				child, err := parseJSONValue(decoder, depth+1)
				if err != nil {
					return nil, err
				}
				object[key] = child
			}
			closing, err := decoder.Token()
			if err != nil || closing != json.Delim('}') {
				return nil, fmt.Errorf("unterminated JSON object")
			}
			return object, nil
		case '[':
			array := make([]any, 0)
			for decoder.More() {
				child, err := parseJSONValue(decoder, depth+1)
				if err != nil {
					return nil, err
				}
				array = append(array, child)
			}
			closing, err := decoder.Token()
			if err != nil || closing != json.Delim(']') {
				return nil, fmt.Errorf("unterminated JSON array")
			}
			return array, nil
		default:
			return nil, fmt.Errorf("unexpected JSON delimiter")
		}
	case nil, bool, string, json.Number:
		return value, nil
	default:
		return nil, fmt.Errorf("unsupported JSON token")
	}
}

func exactKeys(object map[string]any, keys ...string) bool {
	if len(object) != len(keys) {
		return false
	}
	for _, key := range keys {
		if _, ok := object[key]; !ok {
			return false
		}
	}
	return true
}

func signedInt32(raw any) (int32, bool) {
	number, ok := raw.(json.Number)
	if !ok || number.String() == "" {
		return 0, false
	}
	parsed, err := strconv.ParseInt(number.String(), 10, 32)
	if err != nil {
		return 0, false
	}
	return int32(parsed), true
}

func decodeLowerHex(text string) ([]byte, error) {
	if text != strings.ToLower(text) || len(text)%2 != 0 {
		return nil, fmt.Errorf("hex must be lowercase and even-length")
	}
	data, err := hex.DecodeString(text)
	if err != nil {
		return nil, fmt.Errorf("invalid hex text")
	}
	return data, nil
}

func errorCode(text string) (expectedError, bool) {
	switch text {
	case "invalid_page_size":
		return invalidPageSizeError, true
	case "invalid_page_state":
		return invalidPageStateError, true
	case "wrong_payload_length":
		return wrongPayloadLengthError, true
	case "invalid_cut":
		return invalidCutError, true
	case "already_programmed":
		return alreadyProgrammedError, true
	default:
		return noExpectedError, false
	}
}

func expectedCode(err error) expectedError {
	switch {
	case err == nil:
		return noExpectedError
	case errors.Is(err, ErrInvalidPageSize):
		return invalidPageSizeError
	case errors.Is(err, ErrInvalidPageState):
		return invalidPageStateError
	case errors.Is(err, ErrWrongPayloadLength):
		return wrongPayloadLengthError
	case errors.Is(err, ErrInvalidCut):
		return invalidCutError
	case errors.Is(err, ErrAlreadyProgrammed):
		return alreadyProgrammedError
	default:
		return 255
	}
}

func checkFixtureBytes(data []byte) (fixtureDisposition, error) {
	return checkFixtureBytesWithApply(data, Apply)
}

func checkFixtureBytesWithApply(data []byte, apply func(int, PageState, []byte, Cut) (Outcome, error)) (fixtureDisposition, error) {
	cases, err := parseFixture(data)
	if err != nil {
		return fixtureRefused, nil
	}
	return compareFixtureCases(cases, apply)
}

func compareFixtureCases(cases []fixtureCase, apply func(int, PageState, []byte, Cut) (Outcome, error)) (fixtureDisposition, error) {
	for _, row := range cases {
		modelOutcome, modelErr := referenceApply(row.pageSize, row.before, row.payload, row.cut)
		if row.errCode != noExpectedError {
			if expectedCode(modelErr) != row.errCode {
				return fixtureRefused, nil
			}
			continue
		}
		if modelErr != nil || !sameOutcome(modelOutcome, row.want) {
			return fixtureRefused, nil
		}
	}
	for _, row := range cases {
		runtimeOutcome, runtimeErr := apply(row.pageSize, row.before, row.payload, row.cut)
		if row.errCode != noExpectedError {
			if expectedCode(runtimeErr) != row.errCode || !reflect.DeepEqual(runtimeOutcome, Outcome{}) {
				return fixtureRefused, fmt.Errorf("accepted fixture error disagreed with runtime: %s", row.name)
			}
			continue
		}
		if runtimeErr != nil || !sameOutcome(runtimeOutcome, row.want) {
			return fixtureRefused, fmt.Errorf("accepted fixture result disagreed with runtime: %s", row.name)
		}
	}
	return fixtureAcceptedAndMatched, nil
}

// encoding/json normalizes unpaired UTF-16 surrogate escapes to U+FFFD.
// Reject non-scalar escapes before token decoding so malformed fixture text
// cannot silently become a different, well-formed name.
func validateSurrogateEscapes(raw []byte) error {
	for index := 0; index < len(raw); {
		if raw[index] != '"' {
			index++
			continue
		}
		index++
		for index < len(raw) && raw[index] != '"' {
			if raw[index] != '\\' {
				index++
				continue
			}
			if index+1 >= len(raw) {
				return errors.New("truncated JSON string escape")
			}
			if raw[index+1] != 'u' {
				index += 2
				continue
			}
			unit, ok := jsonHexUnit(raw[index+2:])
			if !ok {
				return errors.New("invalid JSON Unicode escape")
			}
			if unit >= 0xdc00 && unit <= 0xdfff {
				return errors.New("unpaired low surrogate in JSON string")
			}
			if unit >= 0xd800 && unit <= 0xdbff {
				if index+11 >= len(raw) || raw[index+6] != '\\' || raw[index+7] != 'u' {
					return errors.New("unpaired high surrogate in JSON string")
				}
				low, lowOK := jsonHexUnit(raw[index+8:])
				if !lowOK || low < 0xdc00 || low > 0xdfff {
					return errors.New("unpaired high surrogate in JSON string")
				}
				index += 12
				continue
			}
			index += 6
		}
		if index >= len(raw) {
			return errors.New("unterminated JSON string")
		}
		index++
	}
	return nil
}

func jsonHexUnit(raw []byte) (uint16, bool) {
	if len(raw) < 4 {
		return 0, false
	}
	var result uint16
	for _, digit := range raw[:4] {
		result <<= 4
		switch {
		case digit >= '0' && digit <= '9':
			result |= uint16(digit - '0')
		case digit >= 'a' && digit <= 'f':
			result |= uint16(digit-'a') + 10
		case digit >= 'A' && digit <= 'F':
			result |= uint16(digit-'A') + 10
		default:
			return 0, false
		}
	}
	return result, true
}

func referenceApply(size int, before PageState, payload []byte, cut Cut) (Outcome, error) {
	if size < 1 || size > maxPageSize {
		return Outcome{}, ErrInvalidPageSize
	}
	if len(before.Bytes) != size {
		return Outcome{}, ErrInvalidPageState
	}
	if !before.Programmed {
		for _, byteValue := range before.Bytes {
			if byteValue != 0xff {
				return Outcome{}, ErrInvalidPageState
			}
		}
	}
	if len(payload) != size {
		return Outcome{}, ErrWrongPayloadLength
	}
	if cut.Kind > Complete {
		return Outcome{}, ErrInvalidCut
	}
	if cut.Kind != AfterPrefix && cut.PrefixBytes != 0 {
		return Outcome{}, ErrInvalidCut
	}
	if cut.Kind == AfterPrefix && (cut.PrefixBytes < 0 || cut.PrefixBytes >= size) {
		return Outcome{}, ErrInvalidCut
	}
	if before.Programmed {
		return Outcome{}, ErrAlreadyProgrammed
	}

	result := append([]byte(nil), before.Bytes...)
	mark := false
	ack := false
	switch cut.Kind {
	case BeforeAccept:
	case AfterPrefix:
		for i := 0; i < cut.PrefixBytes; i++ {
			result[i] = payload[i]
		}
		mark = true
	case AfterMedia:
		copy(result, payload)
		mark = true
	case Complete:
		copy(result, payload)
		mark, ack = true, true
	}
	return Outcome{State: PageState{Bytes: result, Programmed: mark}, Acknowledged: ack}, nil
}

func assertTransition(t *testing.T, got Outcome, gotErr error, want Outcome, wantErr error) {
	t.Helper()
	if expectedCode(gotErr) != expectedCode(wantErr) || !sameOutcome(got, want) {
		t.Fatalf("transition = %+v/%v; want %+v/%v", got, gotErr, want, wantErr)
	}
}

func assertRefusal(t *testing.T, got Outcome, gotErr, wantErr error) {
	t.Helper()
	if !errors.Is(gotErr, wantErr) || !reflect.DeepEqual(got, Outcome{}) {
		t.Fatalf("refusal = %+v/%v; want zero outcome / %v", got, gotErr, wantErr)
	}
}

func sameOutcome(left, right Outcome) bool {
	return left.Acknowledged == right.Acknowledged &&
		left.State.Programmed == right.State.Programmed &&
		bytes.Equal(left.State.Bytes, right.State.Bytes)
}

func erasedPage(size int) PageState {
	return PageState{Bytes: erasedBytes(size)}
}

func erasedBytes(size int) []byte {
	data := make([]byte, size)
	for i := range data {
		data[i] = 0xff
	}
	return data
}

func clonePage(page PageState) PageState {
	return PageState{Bytes: append([]byte(nil), page.Bytes...), Programmed: page.Programmed}
}

func integerPower(base, exponent int) int {
	result := 1
	for range exponent {
		result *= base
	}
	return result
}
