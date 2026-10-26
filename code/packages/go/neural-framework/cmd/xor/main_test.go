package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"math"
	"testing"
)

type failingIO struct{}

func (failingIO) Read([]byte) (int, error)  { return 0, errors.New("read failed") }
func (failingIO) Write([]byte) (int, error) { return 0, errors.New("write failed") }
func TestNativeReceiptResume(t *testing.T) {
	var first bytes.Buffer
	if e := run(nil, bytes.NewReader(nil), &first); e != nil {
		t.Fatal(e)
	}
	var a receipt
	if e := json.Unmarshal(first.Bytes(), &a); e != nil {
		t.Fatal(e)
	}
	if a.Checkpoint.Step != 3000 || a.FinalLoss >= 0.01 {
		t.Fatal("not trained")
	}
	cp, _ := json.Marshal(a.Checkpoint)
	var second bytes.Buffer
	if e := run([]string{"--resume"}, bytes.NewReader(cp), &second); e != nil {
		t.Fatal(e)
	}
	var b receipt
	if e := json.Unmarshal(second.Bytes(), &b); e != nil {
		t.Fatal(e)
	}
	if b.Checkpoint.Step != 3001 || math.Abs(b.InitialLoss-a.FinalLoss) > 1e-9 {
		t.Fatal("resume")
	}
	for i, want := range []float64{0, 1, 1, 0} {
		if math.Abs(b.Predictions[i][0]-want) > 0.1 {
			t.Fatal("xor prediction")
		}
	}
}
func TestBoundaries(t *testing.T) {
	for _, args := range [][]string{{"bad"}, {"--resume", "extra"}} {
		if run(args, nil, io.Discard) == nil {
			t.Fatal("args")
		}
	}
	if run([]string{"--resume"}, failingIO{}, io.Discard) == nil {
		t.Fatal("read")
	}
	if run([]string{"--resume"}, bytes.NewBufferString("{}"), io.Discard) == nil {
		t.Fatal("checkpoint")
	}
	if run([]string{"--resume"}, bytes.NewReader(bytes.Repeat([]byte(" "), 4*1024*1024+1)), io.Discard) == nil {
		t.Fatal("bound")
	}
	// A well-formed but exhausted model must fail training before output.
	cp := `{"format":"coding-adventures-dense","version":1,"step":9007199254740991,"layers":[{"activation":"linear","weights":[[0],[0]],"biases":[0]}]}`
	if run([]string{"--resume"}, bytes.NewBufferString(cp), io.Discard) == nil {
		t.Fatal("step")
	}
	if run(nil, nil, failingIO{}) == nil {
		t.Fatal("write")
	}
}
