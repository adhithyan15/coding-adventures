package key

import (
	"encoding/hex"
	"encoding/json"
	"flag"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// The golden vectors are DEFINED here and WRITTEN to
// code/specs/fixtures/kiln/key-vectors-v1.json by
//
//	go test ./internal/key -run TestGenerateVectors -update
//
// Expected preimages/keys come from this implementation, so vector 1 is also
// checked by hand against explicit byte literals in TestMinimalActionByHand.
var update = flag.Bool("update", false, "regenerate the golden vector file")

const vectorPath = "../../../../../specs/fixtures/kiln/key-vectors-v1.json"

func dg(b byte) string   { return strings.Repeat(hex.EncodeToString([]byte{b}), 32) }
func hx(s string) string { return hex.EncodeToString([]byte(s)) }

// base returns a minimal valid action to be modified by each vector.
func base() ActionJSON {
	return ActionJSON{
		OS: "linux", Arch: "x64", Target: "t", Phase: "build", RecipeHex: hx("r"),
		Inputs: []FileJSON{}, Reads: []FileJSON{}, Consumed: []ConsumedJSON{},
		Tools: []ToolJSON{}, Env: []EnvJSON{},
	}
}

func with(f func(a *ActionJSON)) ActionJSON { a := base(); f(&a); return a }

type namedVec struct {
	name, desc string
	action     ActionJSON
}

func validDefs() []namedVec {
	host := "ubuntu-24.04-image-7"
	legacyBuild := []byte("go build ./...\ngo test ./... -cover\n")
	legacyWin := []byte("go build ./...\r\n")
	return []namedVec{
		{"minimal", "Every list empty, no host fingerprint (KILN02 6.1).", base()},
		{"collision-env-a", "Env pair A of the comma collision: name 'a,b' value 'test'. Differs from collision-env-b (6.2).",
			with(func(a *ActionJSON) { a.Env = []EnvJSON{{"a,b", "test", 0}} })},
		{"collision-env-b", "Env pair B: name 'a' value 'b,test'. A comma-joined encoding would equal collision-env-a; length framing must not.",
			with(func(a *ActionJSON) { a.Env = []EnvJSON{{"a", "b,test", 0}} })},
		{"collision-target-a", "Target 'a,b' phase 'test'. Phase is a closed enum so the spec's literal target/phase pair cannot be built; see KILN02 6.2 note in the implementation report.",
			with(func(a *ActionJSON) { a.Target = "a,b"; a.Phase = "test" })},
		{"collision-target-b", "Target 'a' phase 'test': differs from collision-target-a.",
			with(func(a *ActionJSON) { a.Target = "a"; a.Phase = "test" })},
		{"unicode-and-byte-order", "Non-ASCII paths; 'B' (0x42) sorts before 'a' (0x61), which a case-insensitive order would reverse (6.3).",
			with(func(a *ActionJSON) {
				a.Inputs = []FileJSON{
					{"B", 1, 0, dg(1)}, {"a", 1, 0, dg(2)}, {"café/menu.txt", 1, 0, dg(3)}, {"日本/語.txt", 1, 0, dg(4)},
				}
			})},
		{"link-and-exec", "A symbolic link and an executable file among inputs, plus a read (6.4).",
			with(func(a *ActionJSON) {
				a.Inputs = []FileJSON{{"bin/run.sh", 1, 1, dg(5)}, {"lib/current", 2, 0, dg(6)}, {"src/main.go", 1, 0, dg(7)}}
				a.Reads = []FileJSON{{"data/x.txt", 1, 0, dg(8)}}
			})},
		{"tools-every-kind", "One tool of each kind 0..3 (6.5).",
			with(func(a *ActionJSON) {
				a.Tools = []ToolJSON{
					{"absent-tool", "1.0", 0, ""},
					{"go", "1.26.2", 1, dg(9)},
					{"make", "GNU Make 4.3", 2, hx("GNU Make 4.3\nBuilt for x86_64-pc-linux-gnu\n")},
					{"protoc-gen-kiln", "0", 3, dg(10)},
				}
			})},
		{"host-absent", "Same action as host-present but without a host fingerprint (6.6).", base()},
		{"host-present", "Host fingerprint present: must give a different key from host-absent.",
			with(func(a *ActionJSON) { a.HostImageID = &host })},
		{"legacy-build-only", "Legacy recipe with BUILD only (6.7).",
			with(func(a *ActionJSON) { a.RecipeHex = hex.EncodeToString(LegacyRecipe(legacyBuild, nil)) })},
		{"legacy-build-and-windows", "Legacy recipe with BUILD and BUILD_windows (6.7).",
			with(func(a *ActionJSON) { a.RecipeHex = hex.EncodeToString(LegacyRecipe(legacyBuild, legacyWin)) })},
		{"legacy-build-mac-and-linux", "Legacy recipe of the single file BUILD_mac_and_linux, semantics 0 (KILN03 3.4).",
			with(func(a *ActionJSON) { a.RecipeHex = hex.EncodeToString(mustRecipe(legacyBuild, "BUILD_mac_and_linux")) })},
		{"legacy-semantics0-consumed-digests", "Semantics 0: consumed entries are content digests with dir:/pkg:/tool: targets, phase build, sorted bytewise (KILN02 3.5).",
			with(func(a *ActionJSON) {
				a.RecipeHex = hex.EncodeToString(mustRecipe(legacyBuild, "BUILD"))
				a.SemanticsVersion = 0
				a.HostImageID = &host
				a.Consumed = []ConsumedJSON{
					{"dir:code/grammars/ruby", "build", dg(21)},
					{"pkg:rust/lexer", "build", dg(22)},
					{"tool:build-tool", "build", dg(23)},
				}
				a.Env = []EnvJSON{{"CI", "1", 0}}
			})},
		{"legacy-same-text-different-name", "Same contents as legacy-build-only under the name BUILD_linux: the key must differ.",
			with(func(a *ActionJSON) { a.RecipeHex = hex.EncodeToString(mustRecipe(legacyBuild, "BUILD_linux")) })},
		{"consumed-env-semantics", "Consumed entries (sorted by target then phase), env with both origins, non-zero semantics version, macOS/arm64.",
			with(func(a *ActionJSON) {
				a.OS = "macos"
				a.Arch = "arm64"
				a.SemanticsVersion = 2
				a.Consumed = []ConsumedJSON{
					{"code/packages/go/a:lib", "build", dg(11)},
					{"code/packages/go/a:lib", "test", dg(12)},
					{"code/packages/go/b:lib", "build", dg(13)},
				}
				a.Env = []EnvJSON{{"CI", "1", 0}, {"HOME", "", 1}}
			})},
	}
}

func invalidDefs() []struct {
	namedVec
	err string
} {
	type iv = struct {
		namedVec
		err string
	}
	f := func(p string, k, e uint8, d string) FileJSON { return FileJSON{p, k, e, d} }
	return []iv{
		{namedVec{"unsorted-inputs", "inputs out of bytewise order ('b' before 'a')", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("b", 1, 0, dg(1)), f("a", 1, 0, dg(1))}
		})}, "not sorted"},
		{namedVec{"case-insensitive-order", "'a' before 'B' is locale order, not bytewise", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("a", 1, 0, dg(1)), f("B", 1, 0, dg(1))}
		})}, "not sorted"},
		{namedVec{"duplicate-input", "same path twice", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("a", 1, 0, dg(1)), f("a", 1, 0, dg(2))}
		})}, "duplicate"},
		{namedVec{"duplicate-tool", "same tool name twice", with(func(a *ActionJSON) {
			a.Tools = []ToolJSON{{"go", "1", 1, dg(1)}, {"go", "2", 1, dg(2)}}
		})}, "duplicate"},
		{namedVec{"unsorted-tools", "tools out of order", with(func(a *ActionJSON) {
			a.Tools = []ToolJSON{{"z", "1", 1, dg(1)}, {"a", "1", 1, dg(2)}}
		})}, "not sorted"},
		{namedVec{"dotdot-path", "'..' segment", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("a/../b", 1, 0, dg(1))}
		})}, "malformed path"},
		{namedVec{"dot-path", "'.' segment", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("a/./b", 1, 0, dg(1))}
		})}, "malformed path"},
		{namedVec{"leading-slash", "absolute path", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("/etc/passwd", 1, 0, dg(1))}
		})}, "leading slash"},
		{namedVec{"trailing-slash", "directory-like path", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("a/", 1, 0, dg(1))}
		})}, "trailing slash"},
		{namedVec{"empty-segment", "'//' in path", with(func(a *ActionJSON) {
			a.Reads = []FileJSON{f("a//b", 1, 0, dg(1))}
		})}, "empty segment"},
		{namedVec{"empty-path", "empty path", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("", 1, 0, dg(1))}
		})}, "empty"},
		{namedVec{"short-digest", "31-byte digest", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("a", 1, 0, dg(1)[:62])}
		})}, "exactly 32 bytes"},
		{namedVec{"unknown-phase", "phase 'deploy'", with(func(a *ActionJSON) { a.Phase = "deploy" })}, "unknown phase"},
		{namedVec{"unknown-os", "os 'plan9'", with(func(a *ActionJSON) { a.OS = "plan9" })}, "unknown os"},
		{namedVec{"unknown-arch", "arch 'riscv'", with(func(a *ActionJSON) { a.Arch = "riscv" })}, "unknown arch"},
		{namedVec{"unknown-entry-kind", "entry kind 3", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("a", 3, 0, dg(1))}
		})}, "unknown kind"},
		{namedVec{"unknown-tool-kind", "tool kind 9", with(func(a *ActionJSON) {
			a.Tools = []ToolJSON{{"x", "1", 9, ""}}
		})}, "unknown kind"},
		{namedVec{"invalid-utf8-target", "target bytes ff fe", with(func(a *ActionJSON) { a.Target = ""; a.TargetHex = "fffe" })}, "not valid UTF-8"},
		{namedVec{"exec-link", "a symbolic link marked executable", with(func(a *ActionJSON) {
			a.Inputs = []FileJSON{f("l", 2, 1, dg(1))}
		})}, "exec=0"},
		{namedVec{"unsorted-consumed", "consumed out of order", with(func(a *ActionJSON) {
			a.Consumed = []ConsumedJSON{{"b", "build", dg(1)}, {"a", "build", dg(2)}}
		})}, "not sorted"},
		{namedVec{"duplicate-consumed", "same (target, phase) twice", with(func(a *ActionJSON) {
			a.Consumed = []ConsumedJSON{{"a", "build", dg(1)}, {"a", "build", dg(2)}}
		})}, "duplicate"},
		{namedVec{"consumed-phase-order", "same target, 'test' before 'build'", with(func(a *ActionJSON) {
			a.Consumed = []ConsumedJSON{{"a", "test", dg(1)}, {"a", "build", dg(2)}}
		})}, "not sorted"},
		{namedVec{"short-consumed-key", "31-byte consumed key", with(func(a *ActionJSON) {
			a.Consumed = []ConsumedJSON{{"a", "build", dg(1)[:62]}}
		})}, "exactly 32 bytes"},
		{namedVec{"duplicate-env", "same env name twice", with(func(a *ActionJSON) {
			a.Env = []EnvJSON{{"A", "1", 0}, {"A", "2", 0}}
		})}, "duplicate"},
		{namedVec{"bad-env-origin", "origin 2", with(func(a *ActionJSON) {
			a.Env = []EnvJSON{{"A", "1", 2}}
		})}, "unknown origin"},
		{namedVec{"absent-tool-with-identity", "kind 0 must have empty identity", with(func(a *ActionJSON) {
			a.Tools = []ToolJSON{{"x", "1", 0, "00"}}
		})}, "empty identity"},
		{namedVec{"short-managed-identity", "managed tool identity of 31 bytes", with(func(a *ActionJSON) {
			a.Tools = []ToolJSON{{"x", "1", 1, dg(1)[:62]}}
		})}, "exactly 32 bytes"},
	}
}

func buildVectorFile(t *testing.T) VectorFile {
	t.Helper()
	vf := VectorFile{SchemaVersion: SchemaVersion}
	for _, d := range validDefs() {
		a, err := d.action.ToAction()
		if err != nil {
			t.Fatal(err)
		}
		pre, err := Preimage(a)
		if err != nil {
			t.Fatalf("%s: %v", d.name, err)
		}
		k, _ := Key(a)
		vf.Valid = append(vf.Valid, ValidVector{d.name, d.desc, d.action, hex.EncodeToString(pre), Hex(k)})
	}
	for _, d := range tracedValid(t) {
		a, err := d.action.ToAction()
		if err != nil {
			t.Fatal(err)
		}
		pre, err := Preimage(a)
		if err != nil {
			t.Fatalf("%s: %v", d.name, err)
		}
		k, _ := Key(a)
		vf.Valid = append(vf.Valid, ValidVector{d.name, d.desc, d.action, hex.EncodeToString(pre), Hex(k)})
	}
	for _, d := range invalidDefs() {
		vf.Invalid = append(vf.Invalid, InvalidVector{d.name, d.desc, d.action, d.err})
	}
	for _, d := range tracedDigests() {
		got, err := d.Compute()
		if err != nil {
			t.Fatalf("%s: %v", d.Name, err)
		}
		d.DigestHex = hex.EncodeToString(got[:])
		vf.Digests = append(vf.Digests, d)
	}
	vf.StrongConsumed = tracedStrongConsumed(t)
	vf.RefusedEntries = tracedRefused()
	return vf
}

func TestGenerateVectors(t *testing.T) {
	if !*update {
		t.Skip("run with -update to regenerate the golden vector file")
	}
	data, err := json.MarshalIndent(buildVectorFile(t), "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Dir(vectorPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(vectorPath, append(data, '\n'), 0o644); err != nil {
		t.Fatal(err)
	}
}

// mustRecipe builds a one-file legacy recipe, panicking on the impossible.
func mustRecipe(contents []byte, name string) []byte {
	r, err := LegacyRecipeFiles([]NamedFile{{name, contents}})
	if err != nil {
		panic(err)
	}
	return r
}
