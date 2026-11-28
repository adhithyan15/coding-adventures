package ftlpagemap

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

// Validate exact page-map object keys and token types before encoding/json
// can turn null into scalar zero values or match a differently-cased key.
func validatePageMapJSONKinds(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var root any
	if err := decoder.Decode(&root); err != nil {
		return err
	}
	rootObject, ok := root.(map[string]any)
	if !ok || !exactKeys(rootObject, []string{"schema", "cases"}, nil) {
		return errors.New("invalid page-map root keys")
	}
	if _, ok := rootObject["schema"].(string); !ok {
		return errors.New("page-map schema must be a string")
	}
	cases, ok := rootObject["cases"].([]any)
	if !ok {
		return errors.New("page-map cases must be an array")
	}
	for _, rawCase := range cases {
		caseObject, ok := rawCase.(map[string]any)
		if !ok || !exactKeys(caseObject, []string{"name", "logical_count", "physical_count", "steps"}, nil) {
			return errors.New("invalid page-map case keys")
		}
		if _, ok := caseObject["name"].(string); !ok || !jsonInteger(caseObject["logical_count"]) || !jsonInteger(caseObject["physical_count"]) {
			return errors.New("invalid page-map case field types")
		}
		steps, ok := caseObject["steps"].([]any)
		if !ok {
			return errors.New("page-map steps must be an array")
		}
		for _, rawStep := range steps {
			step, ok := rawStep.(map[string]any)
			if !ok {
				return errors.New("page-map step must be an object")
			}
			op, ok := step["op"].(string)
			if !ok {
				return errors.New("page-map operation must be a string")
			}
			var required []string
			switch op {
			case "lookup", "begin":
				required = []string{"op", "logical", "expected"}
				if !jsonInteger(step["logical"]) {
					return errors.New("page-map logical address must be an integer token")
				}
			case "finish":
				required = []string{"op", "disposition", "expected"}
				if _, ok := step["disposition"].(string); !ok {
					return errors.New("page-map disposition must be a string")
				}
			default:
				return fmt.Errorf("unknown page-map operation %q", op)
			}
			if !exactKeys(step, required, nil) {
				return errors.New("invalid or incorrectly-cased page-map step keys")
			}
			if err := validateReclamationExpectedKinds(step["expected"]); err != nil {
				return fmt.Errorf("invalid page-map expectation: %w", err)
			}
		}
	}
	return nil
}

func TestPageMapScannerRejectsNullMappingAndMalformedReservationFields(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "ftl-page-map.json")
	canonical, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	variants := map[string][]byte{
		"null logical mapping":         bytes.Replace(canonical, []byte(`"logical_to_physical": [-1`), []byte(`"logical_to_physical": [null`), 1),
		"missing reservation physical": bytes.Replace(canonical, []byte(`, "physical": 0}`), []byte(`}`), 1),
		"uppercase logical alias":      bytes.Replace(canonical, []byte(`"logical": 0`), []byte(`"Logical": 0`), 1),
		"uppercase reservation alias":  bytes.Replace(canonical, []byte(`"reservation": {"logical": 0`), []byte(`"reservation": {"Logical": 0`), 1),
		"null reservation logical":     bytes.Replace(canonical, []byte(`"reservation": {"logical": 0`), []byte(`"reservation": {"logical": null`), 1),
		"null reservation physical":    bytes.Replace(canonical, []byte(`"reservation": {"logical": 0, "physical": 0`), []byte(`"reservation": {"logical": 0, "physical": null`), 1),
	}
	for name, raw := range variants {
		t.Run(name, func(t *testing.T) {
			if bytes.Equal(raw, canonical) {
				t.Fatal("malformed variant mutation anchor was absent")
			}
			if _, err := parseFixture(raw); err == nil {
				t.Fatal("malformed fixture was accepted")
			}
		})
	}
}
