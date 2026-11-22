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
//	  {"schema":"kiln-record-action/3","action":"code/packages/go/parser",
//	   "language":"go","weak":"3b1c...","refusal":"","gate":"",
//	   "steps":["go build ./...","go test ./... -cover"],"tools":["go","git"]}
//
// `kiln record step` (stage 4 PR 2b-2) loads it to know the step it is about to
// supervise; `kiln record collect` loads it for the recipe check. Both read it
// from the sealed `pre/` directory, so a BUILD cannot change what its own steps
// were supposed to be.
//
// `tools` (schema /2, KILN04 decision P13) is the sorted set of tool names the
// action's weak key pins (WeakKey.Tools; for an action whose key was refused,
// the tools Traced.RecordEnv knows). `kiln record step` hands it to the
// classifier as `traced.Config.Tools`, because a step that classifies cannot
// build the `legacy.Traced` that would otherwise supply it.
//
// `gate` (schema /3, KILN04 13 item 5, stage 4 PR 2b-2c-3) is the reason the
// package must not be recorded as a measurement: the never list, a KILN03 hard
// reason or a toolchain probe that disagreed (Traced.Gate, the plan's own
// computation). Empty when nothing gates it. A gated package still RUNS, and
// its record carries the gate; the merge turns it into `precondition:<gate>`,
// so it is never emitted. The text comes from the tree (a package name), so it
// is printable and clipped to 200 bytes, like every reason a record carries.
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
	"strings"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// ActionSchema is the schema of an action file.
const ActionSchema = "kiln-record-action/3"

// MaxGateBytes is the longest gate an action file may carry (a record's reason
// fields allow 200).
const MaxGateBytes = 200

// Action is one package the shard runs.
type Action struct {
	Schema   string   `json:"schema"`
	Action   string   `json:"action"`
	Language string   `json:"language"`
	Weak     string   `json:"weak"`
	Refusal  string   `json:"refusal"`
	Gate     string   `json:"gate"`
	Steps    []string `json:"steps"`
	Tools    []string `json:"tools"`
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
	if len(a.Gate) > MaxGateBytes || !utf8.ValidString(a.Gate) || traced.Sanitize(a.Gate) != a.Gate {
		return fmt.Errorf("action %q: gate is not printable text of at most %d bytes", printable(a.Action), MaxGateBytes)
	}
	if a.Steps == nil {
		return fmt.Errorf("action %q: steps is null", printable(a.Action))
	}
	if a.Tools == nil {
		return fmt.Errorf("action %q: tools is null", printable(a.Action))
	}
	for i, t := range a.Tools {
		if t == "" || len(t) > 128 || strings.ContainsFunc(t, func(r rune) bool { return r < 0x20 || r == 0x7f }) {
			return fmt.Errorf("action %q: tool %q is not a plain name", printable(a.Action), printable(t))
		}
		if i > 0 && a.Tools[i-1] >= t {
			return fmt.Errorf("action %q: tools are not strictly sorted at %q", printable(a.Action), printable(t))
		}
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
// HTML escaping, nil steps and tools as empty lists).
func EncodeAction(a Action) ([]byte, error) {
	if a.Steps == nil {
		a.Steps = []string{}
	}
	if a.Tools == nil {
		a.Tools = []string{}
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
	if err := a.validate(); err != nil {
		return Action{}, err
	}
	// A key that is missing decodes as its zero value (a gate of "" is the
	// common case), so the bytes must be exactly what EncodeAction writes for
	// what they say: every key present, in order.
	again, err := EncodeAction(a)
	if err != nil {
		return Action{}, err
	}
	if !bytes.Equal(again, data) {
		return Action{}, fmt.Errorf("action: not in the canonical form EncodeAction writes")
	}
	return a, nil
}
