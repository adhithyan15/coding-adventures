package itf

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

type fixtureCase struct {
	ID        string          `json:"id"`
	Symbology string          `json:"symbology"`
	Input     fixtureInput    `json:"input"`
	Expected  fixtureExpected `json:"expected"`
}

type fixtureInput struct {
	Text   *string        `json:"text"`
	Repeat *fixtureRepeat `json:"repeat"`
}

type fixtureRepeat struct {
	Text  string `json:"text"`
	Count int    `json:"count"`
}

type fixtureExpected struct {
	Normalized       *string `json:"normalized"`
	Modules          *string `json:"modules"`
	RunLengths       []int   `json:"run_lengths"`
	NormalizedSHA256 string  `json:"normalized_sha256"`
	ModuleCount      int     `json:"module_count"`
	ModuleSHA256     string  `json:"module_sha256"`
	RunCount         int     `json:"run_count"`
	RunLengthsSHA256 string  `json:"run_lengths_sha256"`
	Error            string  `json:"error"`
}

func loadITFFixtureCases(t *testing.T) []fixtureCase {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "barcode-symbologies-v1", "cases.json")
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var corpus struct {
		Cases []fixtureCase `json:"cases"`
	}
	if err := json.Unmarshal(raw, &corpus); err != nil {
		t.Fatal(err)
	}
	result := make([]fixtureCase, 0, 10)
	for _, testCase := range corpus.Cases {
		if testCase.Symbology == "itf" {
			result = append(result, testCase)
		}
	}
	if len(result) != 10 {
		t.Fatalf("expected 10 ITF cases, got %d", len(result))
	}
	return result
}

func fixtureInputValue(testCase fixtureCase) string {
	if testCase.Input.Text != nil {
		return *testCase.Input.Text
	}
	return strings.Repeat(testCase.Input.Repeat.Text, testCase.Input.Repeat.Count)
}

func fixtureModules(t *testing.T, data string) string {
	t.Helper()
	pairs, err := EncodeITF(data)
	if err != nil {
		t.Fatal(err)
	}
	var result strings.Builder
	result.WriteString("1010")
	for _, pair := range pairs {
		result.WriteString(pair.BinaryPattern)
	}
	result.WriteString("11101")
	return result.String()
}

func fixtureRuns(bits string) []int {
	runs := make([]int, 0)
	for index, bit := range bits {
		if index == 0 || byte(bit) != bits[index-1] {
			runs = append(runs, 1)
		} else {
			runs[len(runs)-1]++
		}
	}
	return runs
}

func fixtureSHA256(value []byte) string {
	digest := sha256.Sum256(value)
	return hex.EncodeToString(digest[:])
}

func TestITFV1Conformance(t *testing.T) {
	for _, testCase := range loadITFFixtureCases(t) {
		t.Run(testCase.ID, func(t *testing.T) {
			data := fixtureInputValue(testCase)
			if testCase.Expected.Error != "" {
				_, err := NormalizeITF(data)
				var inputError *InputError
				if !errors.As(err, &inputError) || inputError.Code != testCase.Expected.Error {
					t.Fatalf("expected %q, got %#v", testCase.Expected.Error, err)
				}
				return
			}

			normalized, err := NormalizeITF(data)
			if err != nil {
				t.Fatal(err)
			}
			modules := fixtureModules(t, data)
			runs := fixtureRuns(modules)
			if testCase.Expected.Normalized != nil {
				if normalized != *testCase.Expected.Normalized || modules != *testCase.Expected.Modules {
					t.Fatalf("unexpected projection for %s", testCase.ID)
				}
				expectedRuns, _ := json.Marshal(testCase.Expected.RunLengths)
				actualRuns, _ := json.Marshal(runs)
				if string(actualRuns) != string(expectedRuns) {
					t.Fatalf("unexpected runs for %s", testCase.ID)
				}
				return
			}

			compactRuns, _ := json.Marshal(runs)
			if fixtureSHA256([]byte(normalized)) != testCase.Expected.NormalizedSHA256 ||
				len(modules) != testCase.Expected.ModuleCount ||
				fixtureSHA256([]byte(modules)) != testCase.Expected.ModuleSHA256 ||
				len(runs) != testCase.Expected.RunCount ||
				fixtureSHA256(compactRuns) != testCase.Expected.RunLengthsSHA256 {
				t.Fatalf("unexpected boundary projection for %s", testCase.ID)
			}
		})
	}
}
