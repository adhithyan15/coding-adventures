package recfmt

// The binding's language field (KILN04 7.3): a default binding keeps its exact
// bytes, so every binding id (the SHA-256 of those bytes, named by every record
// line) is the one it always was.

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"strings"
	"testing"
)

func sha256Hex(b []byte) string { d := sha256.Sum256(b); return hex.EncodeToString(d[:]) }

// legacyBinding is Binding exactly as it was before the language field: the
// same fields, tags and order. Encoding it is what a pre-selector prepare
// wrote, so equality with EncodeBinding is the byte-identity proof.
type legacyBinding struct {
	Schema              string `json:"schema"`
	Repository          string `json:"repository"`
	Run                 string `json:"run"`
	Attempt             int    `json:"attempt"`
	Revision            string `json:"revision"`
	ShardIndex          int    `json:"shard_index"`
	ShardCount          int    `json:"shard_count"`
	GraphSHA256         string `json:"graph_sha256"`
	StepsSHA256         string `json:"steps_sha256"`
	PartitionSHA256     string `json:"partition_sha256"`
	HostFactsSHA256     string `json:"host_facts_sha256"`
	InventorySHA256     string `json:"inventory_sha256"`
	SandboxConfigSHA256 string `json:"sandbox_config_sha256"`
	KilnSHA256          string `json:"kiln_sha256"`
	BuildToolSHA256     string `json:"build_tool_sha256"`
	StraceSHA256        string `json:"strace_sha256"`
	GitSHA256           string `json:"git_sha256"`
	Recorder            int    `json:"recorder"`
	StraceVersion       string `json:"strace_version"`
	Kernel              string `json:"kernel"`
	JobStart            int64  `json:"job_start"`
	PreparedAt          string `json:"prepared_at"`
	PreparedTicks       int64  `json:"prepared_ticks"`
	ExecHead            bool   `json:"exec_head,omitempty"`
}

func TestDefaultBindingBytesAndIDAreUnchanged(t *testing.T) {
	for _, exec := range []bool{false, true} {
		b := goodBinding()
		b.ExecHead = exec
		got, err := EncodeBinding(b)
		if err != nil {
			t.Fatal(err)
		}
		old := legacyBinding{b.Schema, b.Repository, b.Run, b.Attempt, b.Revision, b.ShardIndex, b.ShardCount,
			b.GraphSHA256, b.StepsSHA256, b.PartitionSHA256, b.HostFactsSHA256, b.InventorySHA256,
			b.SandboxConfigSHA256, b.KilnSHA256, b.BuildToolSHA256, b.StraceSHA256, b.GitSHA256, b.Recorder,
			b.StraceVersion, b.Kernel, b.JobStart, b.PreparedAt, b.PreparedTicks, b.ExecHead}
		want, err := marshal(old)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(got, want) {
			t.Fatalf("exec_head=%v: the default binding changed\n got %s\nwant %s", exec, got, want)
		}
		if bytes.Contains(got, []byte("language")) {
			t.Fatalf("the default binding names a language: %s", got)
		}
		if sha256Hex(got) != sha256Hex(want) {
			t.Fatal("the binding id changed")
		}
	}
}

func TestBindingWithALanguageSealsItAndHasAnotherID(t *testing.T) {
	b := goodBinding()
	def, _ := EncodeBinding(b)
	b.Language = "go"
	got, err := EncodeBinding(b)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(got, []byte(`"language":"go"`)) || sha256Hex(got) == sha256Hex(def) {
		t.Fatalf("%s", got)
	}
}

func TestCheckLanguage(t *testing.T) {
	for _, ok := range []string{"", "go", "csharp", "c-2", "a0"} {
		if err := CheckLanguage(ok); err != nil {
			t.Errorf("%q: %v", ok, err)
		}
	}
	// "all" is the default and is spelled by leaving the field out, so there is
	// one encoding (and one id) of every selection.
	for _, bad := range []string{"all", "Go", "0go", "-x", "a b", "a/b", "go\n", strings.Repeat("a", MaxLanguageBytes+1)} {
		if err := CheckLanguage(bad); err == nil {
			t.Errorf("%q accepted", bad)
		}
		b := goodBinding()
		b.Language = bad
		if _, err := EncodeBinding(b); err == nil {
			t.Errorf("a binding with language %q encoded", bad)
		}
	}
}
