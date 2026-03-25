// Comprehensive tests for the capability-cage library.
//
// Test coverage:
//   - Manifest construction (NewManifest, EmptyManifest, immutability)
//   - Capability.Check and Has (allow/deny/error messages)
//   - Glob matching (bare *, literal, *.ext, prefix*, multi-*)
//   - Path normalization (Clean, traversal prevention)
//   - SecureFile wrappers (ReadFile, WriteFile, CreateFile, DeleteFile, ListDir)
//   - SecureNet wrappers (Connect, Listen, DNSLookup)
//   - SecureProc wrappers (Exec, Signal)
//   - SecureEnv wrappers (ReadEnv, WriteEnv)
//   - SecureTime wrappers (Now, Sleep)
//   - SecureStdio wrappers (ReadStdin, WriteStdout)
//   - Backend interface (OpenBackend, WithBackend swap)
//   - Error message format
//   - Integration: full pipeline check→do
package capabilitycage_test

import (
	"errors"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	cage "github.com/adhithyan15/coding-adventures/code/packages/go/capability-cage"
)

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

// mustCheck returns true if m.Check(cat, act, tgt) returns nil.
func mustCheck(m *cage.Manifest, cat, act, tgt string) bool {
	return m.Check(cat, act, tgt) == nil
}

// isViolation returns true if err is a CapabilityViolationError.
func isViolation(err error) bool {
	var cve *cage.CapabilityViolationError
	return errors.As(err, &cve)
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Manifest construction
// ─────────────────────────────────────────────────────────────────────────────

func TestNewManifest_NilCapabilities(t *testing.T) {
	m := cage.NewManifest(nil)
	if m == nil {
		t.Fatal("NewManifest(nil) returned nil")
	}
}

func TestNewManifest_EmptySlice(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{})
	if m == nil {
		t.Fatal("NewManifest([]) returned nil")
	}
}

func TestNewManifest_SingleCapability(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	if !m.Has(cage.CategoryFS, cage.ActionRead, "foo.txt") {
		t.Error("expected Has to return true for declared capability")
	}
}

func TestNewManifest_Immutability(t *testing.T) {
	// Modifying the original slice after NewManifest must not affect the manifest.
	caps := []cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "original"},
	}
	m := cage.NewManifest(caps)
	caps[0].Target = "changed"

	if !m.Has(cage.CategoryFS, cage.ActionRead, "anything") {
		t.Error("manifest was mutated via original slice — immutability violated")
	}
}

func TestEmptyManifest_DeniesAll(t *testing.T) {
	if cage.EmptyManifest.Has(cage.CategoryFS, cage.ActionRead, "*") {
		t.Error("EmptyManifest should deny all operations")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Check and Has
// ─────────────────────────────────────────────────────────────────────────────

func TestCheck_AllowsDeclaredExactTarget(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/verilog.tokens", Justification: "test"},
	})
	if err := m.Check(cage.CategoryFS, cage.ActionRead, "grammars/verilog.tokens"); err != nil {
		t.Errorf("expected nil, got: %v", err)
	}
}

func TestCheck_DeniesUndeclaredCategory(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	err := m.Check(cage.CategoryNet, cage.ActionConnect, "example.com:80")
	if err == nil {
		t.Error("expected violation for undeclared category")
	}
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestCheck_DeniesUndeclaredAction(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	err := m.Check(cage.CategoryFS, cage.ActionWrite, "foo.txt")
	if err == nil {
		t.Error("expected violation for undeclared action")
	}
}

func TestCheck_DeniesUndeclaredTarget(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/*.tokens", Justification: "test"},
	})
	err := m.Check(cage.CategoryFS, cage.ActionRead, "secrets/passwords.txt")
	if err == nil {
		t.Error("expected violation for undeclared target")
	}
}

func TestCheck_AllowsWildcardTarget(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	if err := m.Check(cage.CategoryFS, cage.ActionRead, "/any/path/at/all"); err != nil {
		t.Errorf("expected wildcard to allow any path, got: %v", err)
	}
}

func TestHas_ReturnsFalseForEmptyManifest(t *testing.T) {
	if cage.EmptyManifest.Has(cage.CategoryFS, cage.ActionRead, "anything") {
		t.Error("EmptyManifest.Has should return false")
	}
}

func TestHas_ReturnsTrueForDeclaredCapability(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryNet, Action: cage.ActionConnect,
			Target: "api.example.com:443", Justification: "test"},
	})
	if !m.Has(cage.CategoryNet, cage.ActionConnect, "api.example.com:443") {
		t.Error("Has should return true for declared capability")
	}
}

func TestCapabilities_ReturnsCopy(t *testing.T) {
	original := []cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	}
	m := cage.NewManifest(original)
	caps := m.Capabilities()
	caps[0].Target = "mutated"
	// Manifest should still have original target
	if !m.Has(cage.CategoryFS, cage.ActionRead, "any") {
		t.Error("Capabilities() should return a copy — manifest was mutated")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Glob matching
// ─────────────────────────────────────────────────────────────────────────────

func TestGlob_BareStarMatchesAnything(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	cases := []string{
		"foo.txt", "/etc/passwd", "a/b/c/d/e", "", "just-a-name",
	}
	for _, c := range cases {
		if !m.Has(cage.CategoryFS, cage.ActionRead, c) {
			t.Errorf("bare * should match %q", c)
		}
	}
}

func TestGlob_LiteralRequiresExactMatch(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/verilog.tokens", Justification: "test"},
	})
	if !m.Has(cage.CategoryFS, cage.ActionRead, "grammars/verilog.tokens") {
		t.Error("literal target should match exact string")
	}
	if m.Has(cage.CategoryFS, cage.ActionRead, "grammars/python.tokens") {
		t.Error("literal target should not match different string")
	}
}

func TestGlob_StarInTargetIsLiteralNotWildcard(t *testing.T) {
	// Wildcards are not supported — a "*" in the target is a literal character,
	// not a glob pattern. "grammars/*.tokens" only matches the target string
	// "grammars/*.tokens" literally. It does NOT expand to match real filenames.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/*.tokens", Justification: "test"},
	})
	// Real filenames do NOT match a literal-* target.
	for _, p := range []string{
		"grammars/verilog.tokens",
		"grammars/python.tokens",
		"grammars/sub/verilog.tokens",
	} {
		if m.Has(cage.CategoryFS, cage.ActionRead, p) {
			t.Errorf("* in target is literal, should not match real file %q", p)
		}
	}
}

func TestGlob_BareStarIsSpecialForNonPathCapabilities(t *testing.T) {
	// Bare "*" (alone, no other characters) is the only wildcard form retained.
	// It is intended for non-path capabilities like stdin/stdout/time where
	// the "target" has no meaningful value to restrict.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryStdin, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	if !m.Has(cage.CategoryStdin, cage.ActionRead, "*") {
		t.Error("bare * should match *")
	}
	if !m.Has(cage.CategoryStdin, cage.ActionRead, "stdin") {
		t.Error("bare * should match any target value")
	}
}

func TestGlob_NoMatchOnEmptyCapabilities(t *testing.T) {
	m := cage.EmptyManifest
	if m.Has(cage.CategoryFS, cage.ActionRead, "anything") {
		t.Error("empty manifest should match nothing")
	}
}

func TestGlob_ExactMatchRequired(t *testing.T) {
	// Targets are exact paths — no wildcards. Only the declared path matches.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "code/grammars/verilog.tokens", Justification: "test"},
	})
	if !m.Has(cage.CategoryFS, cage.ActionRead, "code/grammars/verilog.tokens") {
		t.Error("exact path should match itself")
	}
	// A different file in the same directory must not match.
	if m.Has(cage.CategoryFS, cage.ActionRead, "code/grammars/vhdl.tokens") {
		t.Error("exact match must not allow a different file in the same dir")
	}
	// A path that contains the declared path as a substring must not match.
	if m.Has(cage.CategoryFS, cage.ActionRead, "other/code/grammars/verilog.tokens") {
		t.Error("exact match must not allow prefix-extended paths")
	}
}

func TestGlob_NoWildcardPatterns(t *testing.T) {
	// A pattern containing * (other than bare "*") is treated as a literal
	// string, not a glob — it will only match a target that literally contains *.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "code/grammars/*.tokens", Justification: "test"},
	})
	// The literal "code/grammars/*.tokens" does NOT match real filenames.
	if m.Has(cage.CategoryFS, cage.ActionRead, "code/grammars/verilog.tokens") {
		t.Error("* in target is a literal character, not a wildcard")
	}
}

func TestGlob_WindowsBackslashNormalized(t *testing.T) {
	// On Windows, runtime paths use backslashes. The cage normalizes them
	// so that forward-slash logical paths declared in Go source code match.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "code/grammars/verilog.tokens", Justification: "test"},
	})
	if !m.Has(cage.CategoryFS, cage.ActionRead,
		`code\grammars\verilog.tokens`) {
		t.Error("backslash Windows path should match after normalization")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Path normalization
// ─────────────────────────────────────────────────────────────────────────────

func TestPathNormalization_CleansDotDot(t *testing.T) {
	// The declared target is "grammars/verilog.tokens".
	// A caller passing "grammars/../grammars/verilog.tokens" is asking for
	// the same file — should be allowed after normalization.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/verilog.tokens", Justification: "test"},
	})
	if !m.Has(cage.CategoryFS, cage.ActionRead, "grammars/../grammars/verilog.tokens") {
		t.Error("normalized path should match after cleaning ..")
	}
}

func TestPathNormalization_PreventsDotDotEscape(t *testing.T) {
	// The manifest allows "grammars/*.tokens". A caller should not be able
	// to escape to an unrelated path by using "../" traversal.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/*.tokens", Justification: "test"},
	})
	// "grammars/../etc/passwd" normalizes to "etc/passwd" — not in grammars/
	if m.Has(cage.CategoryFS, cage.ActionRead, "grammars/../etc/passwd") {
		t.Error(".. traversal should not escape the grammars/ prefix")
	}
}

func TestPathNormalization_CleansDotSlash(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/verilog.tokens", Justification: "test"},
	})
	if !m.Has(cage.CategoryFS, cage.ActionRead, "./grammars/verilog.tokens") {
		t.Error("./path should match after normalization")
	}
}

func TestPathNormalization_CleansDoubleSlash(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target: "grammars/verilog.tokens", Justification: "test"},
	})
	if !m.Has(cage.CategoryFS, cage.ActionRead, "grammars//verilog.tokens") {
		t.Error("double slash should be cleaned before matching")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. SecureFile wrappers — using a real temp directory
// ─────────────────────────────────────────────────────────────────────────────

func TestReadFileAt_AllowedUsesLogicalKeyNotOSPath(t *testing.T) {
	// ReadFileAt checks the declared logical key against the manifest and reads
	// from the OS path. This decouples the stable capability declaration from
	// the machine-specific absolute OS path.
	tmp := t.TempDir()
	osPath := filepath.Join(tmp, "verilog.tokens")
	_ = os.WriteFile(osPath, []byte("token data"), 0o644)

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target:        "code/grammars/verilog.tokens",
			Justification: "reads the verilog grammar file"},
	})

	data, err := cage.ReadFileAt(m, "code/grammars/verilog.tokens", osPath)
	if err != nil {
		t.Fatalf("ReadFileAt returned error: %v", err)
	}
	if string(data) != "token data" {
		t.Errorf("expected %q, got %q", "token data", string(data))
	}
}

func TestReadFileAt_WrongLogicalKeyDenied(t *testing.T) {
	// If the declared key doesn't match the manifest, ReadFileAt must deny
	// access even if the OS path is valid.
	tmp := t.TempDir()
	osPath := filepath.Join(tmp, "verilog.tokens")
	_ = os.WriteFile(osPath, []byte("token data"), 0o644)

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead,
			Target:        "code/grammars/verilog.tokens",
			Justification: "reads the verilog grammar file"},
	})

	// Trying to read using a different logical key must fail.
	_, err := cage.ReadFileAt(m, "code/grammars/vhdl.tokens", osPath)
	if err == nil {
		t.Fatal("ReadFileAt with wrong logical key should return error")
	}
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestReadFile_AllowedReturnsContents(t *testing.T) {
	tmp := t.TempDir()
	path := filepath.Join(tmp, "test.txt")
	_ = os.WriteFile(path, []byte("hello"), 0o644)

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	data, err := cage.ReadFile(m, path)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if string(data) != "hello" {
		t.Errorf("expected 'hello', got %q", string(data))
	}
}

func TestReadFile_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.ReadFile(m, "anything.txt")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestWriteFile_AllowedWritesFile(t *testing.T) {
	tmp := t.TempDir()
	path := filepath.Join(tmp, "out.txt")

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionWrite, Target: "*",
			Justification: "test"},
	})
	err := cage.WriteFile(m, path, []byte("world"), 0o644)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	data, _ := os.ReadFile(path) //nolint:cap
	if string(data) != "world" {
		t.Errorf("expected 'world', got %q", string(data))
	}
}

func TestWriteFile_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	err := cage.WriteFile(m, "out.txt", []byte("data"), 0o644)
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestCreateFile_AllowedCreatesFile(t *testing.T) {
	tmp := t.TempDir()
	path := filepath.Join(tmp, "new.txt")

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionCreate, Target: "*",
			Justification: "test"},
	})
	err := cage.CreateFile(m, path)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if _, err := os.Stat(path); os.IsNotExist(err) { //nolint:cap
		t.Error("file should exist after CreateFile")
	}
}

func TestCreateFile_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	err := cage.CreateFile(m, "new.txt")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestDeleteFile_AllowedDeletesFile(t *testing.T) {
	tmp := t.TempDir()
	path := filepath.Join(tmp, "todelete.txt")
	_ = os.WriteFile(path, []byte(""), 0o644) //nolint:cap

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionDelete, Target: "*",
			Justification: "test"},
	})
	err := cage.DeleteFile(m, path)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if _, err := os.Stat(path); !os.IsNotExist(err) { //nolint:cap
		t.Error("file should not exist after DeleteFile")
	}
}

func TestDeleteFile_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	err := cage.DeleteFile(m, "file.txt")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestListDir_AllowedListsDirectory(t *testing.T) {
	tmp := t.TempDir()
	_ = os.WriteFile(filepath.Join(tmp, "a.txt"), []byte(""), 0o644) //nolint:cap
	_ = os.WriteFile(filepath.Join(tmp, "b.txt"), []byte(""), 0o644) //nolint:cap

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionList, Target: "*",
			Justification: "test"},
	})
	entries, err := cage.ListDir(m, tmp)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(entries) != 2 {
		t.Errorf("expected 2 entries, got %d: %v", len(entries), entries)
	}
}

func TestListDir_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.ListDir(m, ".")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. SecureNet wrappers — capability check only (no real network calls)
// ─────────────────────────────────────────────────────────────────────────────

func TestConnect_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.Connect(m, "tcp", "example.com:80")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestConnect_AllowedDelegatesToBackend(t *testing.T) {
	// We don't want to make a real network connection in tests.
	// Use WithBackend to inject a fake.
	var dialCalled bool
	restore := cage.WithBackend(&fakeBackend{
		dialFn: func(network, address string) (net.Conn, error) {
			dialCalled = true
			return nil, nil
		},
	})
	defer restore()

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryNet, Action: cage.ActionConnect,
			Target: "example.com:80", Justification: "test"},
	})
	_, _ = cage.Connect(m, "tcp", "example.com:80")
	if !dialCalled {
		t.Error("expected backend.Dial to be called")
	}
}

func TestListen_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.Listen(m, "tcp", ":8080")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestListen_AllowedDelegatesToBackend(t *testing.T) {
	var listenCalled bool
	restore := cage.WithBackend(&fakeBackend{
		listenFn: func(network, address string) (net.Listener, error) {
			listenCalled = true
			return nil, nil
		},
	})
	defer restore()

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryNet, Action: cage.ActionListen,
			Target: ":9999", Justification: "test"},
	})
	_, _ = cage.Listen(m, "tcp", ":9999")
	if !listenCalled {
		t.Error("expected backend.Listen to be called")
	}
}

func TestDNSLookup_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.DNSLookup(m, "example.com")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestDNSLookup_AllowedDelegatesToBackend(t *testing.T) {
	var lookupCalled bool
	restore := cage.WithBackend(&fakeBackend{
		lookupHostFn: func(host string) ([]string, error) {
			lookupCalled = true
			return []string{"1.2.3.4"}, nil
		},
	})
	defer restore()

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryNet, Action: cage.ActionDNS,
			Target: "example.com", Justification: "test"},
	})
	ips, err := cage.DNSLookup(m, "example.com")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !lookupCalled {
		t.Error("expected backend.LookupHost to be called")
	}
	if len(ips) != 1 || ips[0] != "1.2.3.4" {
		t.Errorf("unexpected IPs: %v", ips)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. SecureProc wrappers
// ─────────────────────────────────────────────────────────────────────────────

func TestExec_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.Exec(m, "git", "status")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestExec_AllowedReturnsCmd(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryProc, Action: cage.ActionExec,
			Target: "git", Justification: "test"},
	})
	cmd, err := cage.Exec(m, "git", "version")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if cmd == nil {
		t.Error("expected non-nil Cmd")
	}
}

func TestSignal_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	err := cage.Signal(m, os.Getpid(), os.Interrupt)
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestSignal_AllowedWithWildcardPID(t *testing.T) {
	var killCalled bool
	restore := cage.WithBackend(&fakeBackend{
		killFn: func(pid int, sig os.Signal) error {
			killCalled = true
			return nil
		},
	})
	defer restore()

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryProc, Action: cage.ActionSignal,
			Target: "*", Justification: "test"},
	})
	err := cage.Signal(m, 1234, os.Interrupt)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !killCalled {
		t.Error("expected backend.Kill to be called")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. SecureEnv wrappers
// ─────────────────────────────────────────────────────────────────────────────

func TestReadEnv_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.ReadEnv(m, "HOME")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestReadEnv_AllowedReturnsValue(t *testing.T) {
	t.Setenv("CAGE_TEST_VAR", "testvalue")
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryEnv, Action: cage.ActionRead,
			Target: "CAGE_TEST_VAR", Justification: "test"},
	})
	val, err := cage.ReadEnv(m, "CAGE_TEST_VAR")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != "testvalue" {
		t.Errorf("expected 'testvalue', got %q", val)
	}
}

func TestWriteEnv_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	err := cage.WriteEnv(m, "FOO", "bar")
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestWriteEnv_AllowedSetsValue(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryEnv, Action: cage.ActionWrite,
			Target: "CAGE_WRITE_TEST", Justification: "test"},
	})
	err := cage.WriteEnv(m, "CAGE_WRITE_TEST", "hello")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Verify the env var was actually set.
	if got := os.Getenv("CAGE_WRITE_TEST"); got != "hello" { //nolint:cap
		t.Errorf("expected env var to be 'hello', got %q", got)
	}
	// Clean up.
	_ = os.Unsetenv("CAGE_WRITE_TEST") //nolint:cap
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. SecureTime wrappers
// ─────────────────────────────────────────────────────────────────────────────

func TestNow_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.Now(m)
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestNow_AllowedReturnsTime(t *testing.T) {
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryTime, Action: cage.ActionRead,
			Target: "*", Justification: "test"},
	})
	before := time.Now()
	got, err := cage.Now(m)
	after := time.Now()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if got.Before(before) || got.After(after) {
		t.Errorf("Now() returned time outside expected range: %v", got)
	}
}

func TestSleep_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	err := cage.Sleep(m, time.Millisecond)
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestSleep_AllowedCallsBackend(t *testing.T) {
	var sleptFor time.Duration
	restore := cage.WithBackend(&fakeBackend{
		sleepFn: func(d time.Duration) { sleptFor = d },
	})
	defer restore()

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryTime, Action: cage.ActionSleep,
			Target: "*", Justification: "test"},
	})
	if err := cage.Sleep(m, 42*time.Millisecond); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if sleptFor != 42*time.Millisecond {
		t.Errorf("expected 42ms sleep, got %v", sleptFor)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. SecureStdio wrappers
// ─────────────────────────────────────────────────────────────────────────────

func TestReadStdin_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	buf := make([]byte, 10)
	_, err := cage.ReadStdin(m, buf)
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestReadStdin_AllowedCallsBackend(t *testing.T) {
	restore := cage.WithBackend(&fakeBackend{
		readStdinFn: func(p []byte) (int, error) {
			copy(p, []byte("hi"))
			return 2, nil
		},
	})
	defer restore()

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryStdin, Action: cage.ActionRead,
			Target: "*", Justification: "test"},
	})
	buf := make([]byte, 10)
	n, err := cage.ReadStdin(m, buf)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if n != 2 || string(buf[:n]) != "hi" {
		t.Errorf("unexpected read result: n=%d, data=%q", n, buf[:n])
	}
}

func TestWriteStdout_DeniedReturnsViolation(t *testing.T) {
	m := cage.EmptyManifest
	_, err := cage.WriteStdout(m, []byte("hello"))
	if !isViolation(err) {
		t.Errorf("expected CapabilityViolationError, got %T: %v", err, err)
	}
}

func TestWriteStdout_AllowedCallsBackend(t *testing.T) {
	var written []byte
	restore := cage.WithBackend(&fakeBackend{
		writeStdoutFn: func(p []byte) (int, error) {
			written = append(written, p...)
			return len(p), nil
		},
	})
	defer restore()

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryStdout, Action: cage.ActionWrite,
			Target: "*", Justification: "test"},
	})
	n, err := cage.WriteStdout(m, []byte("hello"))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if n != 5 {
		t.Errorf("expected 5 bytes written, got %d", n)
	}
	if string(written) != "hello" {
		t.Errorf("expected 'hello', got %q", string(written))
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. Error message format
// ─────────────────────────────────────────────────────────────────────────────

func TestViolationError_ContainsCategoryActionTarget(t *testing.T) {
	m := cage.EmptyManifest
	err := m.Check(cage.CategoryFS, cage.ActionRead, "secret.txt")
	if err == nil {
		t.Fatal("expected error")
	}
	msg := err.Error()
	if msg == "" {
		t.Error("error message should not be empty")
	}
	// The message should contain the category, action, and target so the
	// developer knows exactly what to add to required_capabilities.json.
	for _, substr := range []string{"fs", "read", "secret.txt"} {
		if !containsString(msg, substr) {
			t.Errorf("error message %q should contain %q", msg, substr)
		}
	}
}

func TestViolationError_ContainsRemediationHint(t *testing.T) {
	m := cage.EmptyManifest
	err := m.Check(cage.CategoryNet, cage.ActionConnect, "example.com:443")
	msg := err.Error()
	// Should mention required_capabilities.json or gen_capabilities.go
	if !containsString(msg, "required_capabilities.json") &&
		!containsString(msg, "gen_capabilities.go") {
		t.Errorf("error message should mention remediation steps: %q", msg)
	}
}

func TestViolationError_AsWorks(t *testing.T) {
	m := cage.EmptyManifest
	err := m.Check(cage.CategoryFS, cage.ActionRead, "file.txt")

	var cve *cage.CapabilityViolationError
	if !errors.As(err, &cve) {
		t.Fatal("errors.As should unwrap to CapabilityViolationError")
	}
	if cve.Category != cage.CategoryFS {
		t.Errorf("expected Category=%q, got %q", cage.CategoryFS, cve.Category)
	}
	if cve.Action != cage.ActionRead {
		t.Errorf("expected Action=%q, got %q", cage.ActionRead, cve.Action)
	}
	if cve.Target != "file.txt" {
		t.Errorf("expected Target=%q, got %q", "file.txt", cve.Target)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 11b. OpenBackend direct tests
// ─────────────────────────────────────────────────────────────────────────────

func TestOpenBackend_SleepViaSecureWrapper(t *testing.T) {
	// Ensure OpenBackend.Sleep is covered by calling Sleep through the
	// secure wrapper without fakeBackend.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryTime, Action: cage.ActionSleep,
			Target: "*", Justification: "test"},
	})
	// Sleep for 1ns — fast but exercises the real OpenBackend.Sleep path.
	if err := cage.Sleep(m, time.Nanosecond); err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestOpenBackend_WriteStdoutViaSecureWrapper(t *testing.T) {
	// Ensure OpenBackend.WriteStdout is covered. Writing to stdout in tests
	// is harmless (test output is usually captured or discarded).
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryStdout, Action: cage.ActionWrite,
			Target: "*", Justification: "test"},
	})
	n, err := cage.WriteStdout(m, []byte(""))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	_ = n
}

func TestCreateFile_FailsIfExists(t *testing.T) {
	// The CreateFile error path: file already exists.
	tmp := t.TempDir()
	path := filepath.Join(tmp, "existing.txt")
	_ = os.WriteFile(path, []byte(""), 0o644) //nolint:cap

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionCreate, Target: "*",
			Justification: "test"},
	})
	err := cage.CreateFile(m, path)
	if err == nil {
		t.Error("expected error when creating file that already exists")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. Backend swap (WithBackend)
// ─────────────────────────────────────────────────────────────────────────────

func TestWithBackend_RestoresOriginal(t *testing.T) {
	restore := cage.WithBackend(&fakeBackend{})
	restore()
	// After restore, ReadFile should use OpenBackend (real filesystem).
	// We just verify no panic and the function works.
	tmp := t.TempDir()
	path := filepath.Join(tmp, "test.txt")
	_ = os.WriteFile(path, []byte("x"), 0o644) //nolint:cap

	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
	})
	_, err := cage.ReadFile(m, path)
	if err != nil {
		t.Errorf("unexpected error after restore: %v", err)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 13. Integration: full pipeline
// ─────────────────────────────────────────────────────────────────────────────

func TestIntegration_ReadFilePipelineAllowed(t *testing.T) {
	// Simulates what a grammar-based lexer does at startup:
	// 1. It has a compiled-in manifest declaring fs:read:*.
	// 2. It calls cage.ReadFile(Manifest, grammarPath).
	// 3. The cage checks the manifest, then reads the file.
	tmp := t.TempDir()
	grammarFile := filepath.Join(tmp, "verilog.tokens")
	_ = os.WriteFile(grammarFile, []byte("TOKEN_PATTERN"), 0o644) //nolint:cap

	// Simulates gen_capabilities.go
	Manifest := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "Reads grammar files at startup to configure the lexer DFA."},
	})

	data, err := cage.ReadFile(Manifest, grammarFile)
	if err != nil {
		t.Fatalf("integration test failed: %v", err)
	}
	if string(data) != "TOKEN_PATTERN" {
		t.Errorf("unexpected content: %q", string(data))
	}
}

func TestIntegration_ReadFilePipelineDenied(t *testing.T) {
	// Simulates a pure-computation package (EmptyManifest) trying to read
	// a file — should be denied.
	_, err := cage.ReadFile(cage.EmptyManifest, "/any/path")
	if !isViolation(err) {
		t.Errorf("expected violation for EmptyManifest read attempt")
	}
}

func TestIntegration_MultipleCapabilities(t *testing.T) {
	// A manifest with both fs:read and stdout:write should allow both
	// and deny everything else.
	m := cage.NewManifest([]cage.Capability{
		{Category: cage.CategoryFS, Action: cage.ActionRead, Target: "*",
			Justification: "test"},
		{Category: cage.CategoryStdout, Action: cage.ActionWrite, Target: "*",
			Justification: "test"},
	})

	if err := m.Check(cage.CategoryFS, cage.ActionRead, "any.txt"); err != nil {
		t.Errorf("fs:read should be allowed: %v", err)
	}
	if err := m.Check(cage.CategoryStdout, cage.ActionWrite, "*"); err != nil {
		t.Errorf("stdout:write should be allowed: %v", err)
	}
	if err := m.Check(cage.CategoryNet, cage.ActionConnect, "x:80"); err == nil {
		t.Error("net:connect should be denied")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Fake backend for testing
// ─────────────────────────────────────────────────────────────────────────────

// fakeBackend is a test double for the Backend interface. Only the
// functions set via function fields do anything; all others are no-ops.
type fakeBackend struct {
	dialFn        func(network, address string) (net.Conn, error)
	listenFn      func(network, address string) (net.Listener, error)
	lookupHostFn  func(host string) ([]string, error)
	killFn        func(pid int, sig os.Signal) error
	sleepFn       func(d time.Duration)
	writeStdoutFn func(p []byte) (int, error)
	readStdinFn   func(p []byte) (int, error)
}

func (f *fakeBackend) ReadFile(path string) ([]byte, error) { return nil, nil }
func (f *fakeBackend) WriteFile(path string, data []byte, perm os.FileMode) error {
	return nil
}
func (f *fakeBackend) CreateFile(path string) error             { return nil }
func (f *fakeBackend) DeleteFile(path string) error             { return nil }
func (f *fakeBackend) ListDir(path string) ([]string, error)    { return nil, nil }
func (f *fakeBackend) Getenv(key string) string                 { return "" }
func (f *fakeBackend) Setenv(key, value string) error           { return nil }
func (f *fakeBackend) Now() time.Time                           { return time.Time{} }
func (f *fakeBackend) Command(name string, args ...string) *exec.Cmd {
	return exec.Command(name, args...)
}

func (f *fakeBackend) Dial(network, address string) (net.Conn, error) {
	if f.dialFn != nil {
		return f.dialFn(network, address)
	}
	return nil, nil
}

func (f *fakeBackend) Listen(network, address string) (net.Listener, error) {
	if f.listenFn != nil {
		return f.listenFn(network, address)
	}
	return nil, nil
}

func (f *fakeBackend) LookupHost(host string) ([]string, error) {
	if f.lookupHostFn != nil {
		return f.lookupHostFn(host)
	}
	return nil, nil
}

func (f *fakeBackend) Kill(pid int, sig os.Signal) error {
	if f.killFn != nil {
		return f.killFn(pid, sig)
	}
	return nil
}

func (f *fakeBackend) Sleep(d time.Duration) {
	if f.sleepFn != nil {
		f.sleepFn(d)
	}
}

func (f *fakeBackend) ReadStdin(p []byte) (int, error) {
	if f.readStdinFn != nil {
		return f.readStdinFn(p)
	}
	return 0, nil
}

func (f *fakeBackend) WriteStdout(p []byte) (int, error) {
	if f.writeStdoutFn != nil {
		return f.writeStdoutFn(p)
	}
	return len(p), nil
}

// Ensure fakeBackend satisfies Backend.
var _ cage.Backend = (*fakeBackend)(nil)

// ─────────────────────────────────────────────────────────────────────────────
// Utility
// ─────────────────────────────────────────────────────────────────────────────

func containsString(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr ||
		len(substr) == 0 ||
		stringContains(s, substr))
}

func stringContains(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}
