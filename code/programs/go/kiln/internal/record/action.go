package record

// The per-action file (KILN04 section 4.8, stage 4b-1, decision P7)
// =================================================================
//
// `prepare` computes, BEFORE any BUILD, what the shard will need to know about
// every package it runs: the weak key (or why it has none), and the lines the
// package is supposed to execute. One small file per action, named by the
// SHA-256 of the action's path so that no path from the tree ever becomes a
// file name:
//
//	pre/actions/9f2c...e1.json
//	  {"schema":"kiln-record-action/1","action":"code/packages/go/parser",
//	   "language":"go","weak":"3b1c...","refusal":"",
//	   "steps":["go build ./...","go test ./... -cover"]}
//
// `kiln record step` (stage 4 PR 2b-2) loads it to know the step it is about to
// supervise; `kiln record collect` loads it for the recipe check. Both read it
// from the sealed `pre/` directory, so a BUILD cannot change what its own steps
// were supposed to be.
//
// Exactly one of `weak` and `refusal` is set, as in a shard record line: a
// package either has a key or says why it does not.

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// ActionSchema is the schema of an action file.
const ActionSchema = "kiln-record-action/1"

// Action is one package the shard runs.
type Action struct {
	Schema   string   `json:"schema"`
	Action   string   `json:"action"`
	Language string   `json:"language"`
	Weak     string   `json:"weak"`
	Refusal  string   `json:"refusal"`
	Steps    []string `json:"steps"`
}

// ActionFile is the file name, relative to `pre/`, of the action's file.
func ActionFile(action string) string {
	sum := sha256.Sum256([]byte(action))
	return "actions/" + hex.EncodeToString(sum[:]) + ".json"
}

func (a *Action) validate() error {
	if a.Schema != ActionSchema {
		return fmt.Errorf("action: schema %q", printable(a.Schema))
	}
	if err := recfmt.CheckPath(a.Action); err != nil {
		return fmt.Errorf("action: %v", err)
	}
	if (a.Weak == "") == (a.Refusal == "") {
		return fmt.Errorf("action %q: exactly one of weak and refusal must be set", printable(a.Action))
	}
	if a.Weak != "" && !hexDigest(a.Weak) {
		return fmt.Errorf("action %q: weak is not a SHA-256 in lowercase hex", printable(a.Action))
	}
	if a.Steps == nil {
		return fmt.Errorf("action %q: steps is null", printable(a.Action))
	}
	return nil
}

func hexDigest(s string) bool {
	if len(s) != 64 {
		return false
	}
	for i := 0; i < len(s); i++ {
		if c := s[i]; !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}

// EncodeAction validates and renders an action file (no trailing newline, no
// HTML escaping, nil steps as an empty list).
func EncodeAction(a Action) ([]byte, error) {
	if a.Steps == nil {
		a.Steps = []string{}
	}
	if err := a.validate(); err != nil {
		return nil, err
	}
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(a); err != nil {
		return nil, err
	}
	return bytes.TrimSuffix(buf.Bytes(), []byte("\n")), nil
}

// DecodeAction reads one action file strictly: no unknown key, no trailing
// data, and the same validation as the encoder.
func DecodeAction(data []byte) (Action, error) {
	var a Action
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&a); err != nil {
		return Action{}, fmt.Errorf("action: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return Action{}, fmt.Errorf("action: unexpected data after the JSON value")
	}
	return a, a.validate()
}
