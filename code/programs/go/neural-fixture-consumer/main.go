package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"math"
	"os"
	"strings"
)

const (
	laneID       = "go-native"
	maximumBytes = 1 << 20
)

type layer struct {
	Name       string      `json:"name"`
	Weights    [][]float64 `json:"weights"`
	Biases     []float64   `json:"biases"`
	Activation string      `json:"activation"`
}

type datasetRow struct {
	Label  string    `json:"label"`
	Input  []float64 `json:"input"`
	Target []float64 `json:"target"`
}

type forwardExpectation struct {
	Row        string    `json:"row"`
	Prediction []float64 `json:"prediction"`
}

type fixture struct {
	SchemaVersion int      `json:"schema_version"`
	ID            string   `json:"id"`
	Title         string   `json:"title"`
	Stage         string   `json:"stage"`
	Question      string   `json:"question"`
	Concepts      []string `json:"concepts"`
	Model         struct {
		Kind       string  `json:"kind"`
		InputCount int     `json:"input_count"`
		Layers     []layer `json:"layers"`
	} `json:"model"`
	Dataset struct {
		InputLabels  []string     `json:"input_labels"`
		TargetLabels []string     `json:"target_labels"`
		Rows         []datasetRow `json:"rows"`
	} `json:"dataset"`
	Training *json.RawMessage `json:"training"`
	Expected struct {
		AbsoluteTolerance float64              `json:"absolute_tolerance"`
		Forward           []forwardExpectation `json:"forward"`
		FirstStep         *json.RawMessage     `json:"first_step"`
	} `json:"expected"`
}

type receipt struct {
	SchemaVersion        int       `json:"schema_version"`
	LaneID               string    `json:"lane_id"`
	FixtureID            string    `json:"fixture_id"`
	Row                  string    `json:"row"`
	Contributions        []float64 `json:"contributions"`
	Bias                 float64   `json:"bias"`
	Preactivation        float64   `json:"preactivation"`
	Prediction           []float64 `json:"prediction"`
	MaximumAbsoluteError float64   `json:"maximum_absolute_error"`
	Passes               bool      `json:"passes"`
}

func validateObjectKeys(decoder *json.Decoder) error {
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	delimiter, isDelimiter := token.(json.Delim)
	if !isDelimiter {
		return nil
	}
	switch delimiter {
	case '{':
		seen := make(map[string]struct{})
		for decoder.More() {
			keyToken, err := decoder.Token()
			if err != nil {
				return err
			}
			key, ok := keyToken.(string)
			if !ok {
				return errors.New("object key must be a string")
			}
			if key != strings.ToLower(key) {
				return fmt.Errorf("object key %q must use canonical lowercase spelling", key)
			}
			if _, duplicate := seen[key]; duplicate {
				return fmt.Errorf("duplicate object key %q", key)
			}
			seen[key] = struct{}{}
			if err := validateObjectKeys(decoder); err != nil {
				return err
			}
		}
		_, err = decoder.Token()
		return err
	case '[':
		for decoder.More() {
			if err := validateObjectKeys(decoder); err != nil {
				return err
			}
		}
		_, err = decoder.Token()
		return err
	default:
		return errors.New("unexpected JSON delimiter")
	}
}

func validateJSONKeys(payload []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(payload))
	if err := validateObjectKeys(decoder); err != nil {
		return err
	}
	if _, err := decoder.Token(); !errors.Is(err, io.EOF) {
		return errors.New("fixture must contain exactly one JSON document")
	}
	return nil
}

func loadFixture(path string) (fixture, error) {
	file, err := os.Open(path)
	if err != nil {
		return fixture{}, fmt.Errorf("open fixture: %w", err)
	}
	defer file.Close()

	info, err := file.Stat()
	if err != nil {
		return fixture{}, fmt.Errorf("stat fixture: %w", err)
	}
	if !info.Mode().IsRegular() || info.Size() <= 0 || info.Size() > maximumBytes {
		return fixture{}, errors.New("fixture must be a non-empty regular file no larger than 1 MiB")
	}
	payload, err := io.ReadAll(io.LimitReader(file, maximumBytes+1))
	if err != nil {
		return fixture{}, fmt.Errorf("read fixture: %w", err)
	}
	if len(payload) == 0 || len(payload) > maximumBytes {
		return fixture{}, errors.New("fixture must be a non-empty regular file no larger than 1 MiB")
	}
	if err := validateJSONKeys(payload); err != nil {
		return fixture{}, fmt.Errorf("decode fixture: %w", err)
	}

	decoder := json.NewDecoder(bytes.NewReader(payload))
	decoder.DisallowUnknownFields()
	var document fixture
	if err := decoder.Decode(&document); err != nil {
		return fixture{}, fmt.Errorf("decode fixture: %w", err)
	}
	if err := decoder.Decode(&struct{}{}); !errors.Is(err, io.EOF) {
		return fixture{}, errors.New("fixture must contain exactly one JSON document")
	}
	return document, nil
}

func evaluate(document fixture) (receipt, error) {
	if document.SchemaVersion != 1 || document.ID != "weighted-neuron-forward" || document.Stage != "forward" {
		return receipt{}, errors.New("unsupported fixture identity")
	}
	if document.Training != nil || document.Expected.FirstStep != nil {
		return receipt{}, errors.New("forward-only fixture must not contain a training step")
	}
	if document.Model.Kind != "single-neuron" || document.Model.InputCount != 2 || len(document.Model.Layers) != 1 {
		return receipt{}, errors.New("expected one two-input neuron")
	}
	layer := document.Model.Layers[0]
	if layer.Name != "output" || layer.Activation != "identity" || len(layer.Weights) != 2 || len(layer.Biases) != 1 {
		return receipt{}, errors.New("unsupported layer contract")
	}
	if len(document.Dataset.Rows) != 1 || len(document.Expected.Forward) != 1 {
		return receipt{}, errors.New("expected one data row and one forward expectation")
	}
	row := document.Dataset.Rows[0]
	expected := document.Expected.Forward[0]
	if len(row.Input) != 2 || len(expected.Prediction) != 1 || expected.Row != row.Label || document.Expected.AbsoluteTolerance <= 0 {
		return receipt{}, errors.New("invalid row or expectation shape")
	}

	contributions := make([]float64, 2)
	preactivation := layer.Biases[0]
	for index := range row.Input {
		if len(layer.Weights[index]) != 1 {
			return receipt{}, errors.New("each input must have one output weight")
		}
		contributions[index] = row.Input[index] * layer.Weights[index][0]
		preactivation += contributions[index]
	}
	errorValue := math.Abs(preactivation - expected.Prediction[0])
	if math.IsNaN(preactivation) || math.IsInf(preactivation, 0) || math.IsNaN(errorValue) || math.IsInf(errorValue, 0) {
		return receipt{}, errors.New("non-finite arithmetic result")
	}
	return receipt{
		SchemaVersion:        1,
		LaneID:               laneID,
		FixtureID:            document.ID,
		Row:                  row.Label,
		Contributions:        contributions,
		Bias:                 layer.Biases[0],
		Preactivation:        preactivation,
		Prediction:           []float64{preactivation},
		MaximumAbsoluteError: errorValue,
		Passes:               errorValue <= document.Expected.AbsoluteTolerance,
	}, nil
}

func run(arguments []string, stdout io.Writer) error {
	flags := flag.NewFlagSet("neural-fixture-consumer", flag.ContinueOnError)
	flags.SetOutput(io.Discard)
	fixturePath := flags.String("fixture", "", "path to the weighted-neuron fixture")
	if err := flags.Parse(arguments); err != nil || *fixturePath == "" || flags.NArg() != 0 {
		return errors.New("usage: neural-fixture-consumer --fixture PATH")
	}
	document, err := loadFixture(*fixturePath)
	if err != nil {
		return err
	}
	result, err := evaluate(document)
	if err != nil {
		return err
	}
	if !result.Passes {
		return errors.New("prediction exceeded the fixture tolerance")
	}
	encoder := json.NewEncoder(stdout)
	encoder.SetEscapeHTML(false)
	return encoder.Encode(result)
}

func main() {
	if err := run(os.Args[1:], os.Stdout); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
