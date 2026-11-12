package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const vectors = "../../../specs/fixtures/kiln/key-vectors-v1.json"

func do(args ...string) (int, string, string) {
	var o, e bytes.Buffer
	c := run(args, &o, &e)
	return c, o.String(), e.String()
}

func TestVersion(t *testing.T) {
	c, o, _ := do("version")
	if c != 0 || o != "kiln 0.1.0 (key schema 1)\n" {
		t.Errorf("%d %q", c, o)
	}
}

func TestUsage(t *testing.T) {
	for _, args := range [][]string{{}, {"verify-vectors"}, {"verify-vectors", "a", "b"}, {"bogus"}} {
		if c, _, e := do(args...); c != 2 || e == "" {
			t.Errorf("%v: %d %q", args, c, e)
		}
	}
}

func TestVerifyVectorsOK(t *testing.T) {
	c, o, e := do("verify-vectors", vectors)
	if c != 0 || !strings.HasPrefix(o, "ok:") {
		t.Errorf("%d %q %q", c, o, e)
	}
}

func TestVerifyVectorsFailures(t *testing.T) {
	dir := t.TempDir()
	if c, _, _ := do("verify-vectors", filepath.Join(dir, "missing.json")); c != 1 {
		t.Errorf("missing file: %d", c)
	}
	bad := filepath.Join(dir, "bad.json")
	os.WriteFile(bad, []byte("{"), 0o644)
	if c, _, _ := do("verify-vectors", bad); c != 1 {
		t.Errorf("bad json: %d", c)
	}
	data, _ := os.ReadFile(vectors)
	i := strings.Index(string(data), `"key_hex": "`) + len(`"key_hex": "`)
	mut := []byte(string(data))
	if mut[i] == '0' {
		mut[i] = '1'
	} else {
		mut[i] = '0'
	}
	wrong := filepath.Join(dir, "wrong.json")
	os.WriteFile(wrong, mut, 0o644)
	if c, _, e := do("verify-vectors", wrong); c != 1 || !strings.Contains(e, "FAIL") {
		t.Errorf("mismatch: %d %q", c, e)
	}
}
