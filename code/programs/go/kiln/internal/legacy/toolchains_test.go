package legacy

import (
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

func mustToolchains(t *testing.T, src string) *Toolchains {
	t.Helper()
	tc, err := ParseToolchains([]byte(src))
	if err != nil {
		t.Fatalf("ParseToolchains: %v", err)
	}
	return tc
}

func TestParseToolchainsRejects(t *testing.T) {
	cases := map[string]string{
		"not an object":      `[]`,
		"empty":              ``,
		"trailing":           `{} {}`,
		"unsorted entries":   `{"b": {"pins": {}}, "a": {"pins": {}}}`,
		"duplicate entry":    `{"a": {"pins": {}}, "a": {"pins": {}}}`,
		"bad entry name":     `{"a:b": {"pins": {}}}`,
		"slash entry name":   `{"a/b": {"pins": {}}}`,
		"empty entry name":   `{"": {"pins": {}}}`,
		"long entry name":    `{"` + strings.Repeat("a", 65) + `": {"pins": {}}}`,
		"entry not object":   `{"a": 1}`,
		"unknown field":      `{"a": {"pins": {}, "extra": 1}}`,
		"unknown os":         `{"a": {"pins": {}, "os": ["plan9"]}}`,
		"unsorted os":        `{"a": {"pins": {}, "os": ["linux", "darwin"]}}`,
		"pins not object":    `{"a": {"pins": 3}}`,
		"pins unsorted":      `{"a": {"pins": {"b": "1", "a": "1"}}}`,
		"pin not string":     `{"a": {"pins": {"b": 1}}}`,
		"bad tool name":      `{"a": {"pins": {"b:c": "1"}}}`,
		"empty pin":          `{"a": {"pins": {"b": ""}}}`,
		"floating pin":       `{"a": {"pins": {"b": "latest"}}}`,
		"range pin":          `{"a": {"pins": {"b": "^1.2"}}}`,
		"spaced pin":         `{"a": {"pins": {"b": "1 2"}}}`,
		"too many probes":    `{"a": {"pins": {}, "probes": [["a"],["a"],["a"],["a"],["a"],["a"],["a"],["a"],["a"]]}}`,
		"empty probe":        `{"a": {"pins": {}, "probes": [[]]}}`,
		"path probe":         `{"a": {"pins": {}, "probes": [["/bin/a"]]}}`,
		"probe empty arg":    `{"a": {"pins": {}, "probes": [["a", ""]]}}`,
		"probe control arg":  `{"a": {"pins": {}, "probes": [["a", "x\u0007"]]}}`,
		"probe long arg":     `{"a": {"pins": {}, "probes": [["a", "` + strings.Repeat("x", 257) + `"]]}}`,
		"probe many args":    `{"a": {"pins": {}, "probes": [["a","1","2","3","4","5","6","7","8"]]}}`,
		"roots not object":   `{"a": {"pins": {}, "roots": 3}}`,
		"roots unsorted":     `{"a": {"pins": {"a":"1","b":"1"}, "roots": {"b": ["/x"], "a": ["/x"]}}}`,
		"roots unpinned":     `{"a": {"pins": {}, "roots": {"b": ["/x"]}}}`,
		"roots not list":     `{"a": {"pins": {"b":"1"}, "roots": {"b": "/x"}}}`,
		"roots empty list":   `{"a": {"pins": {"b":"1"}, "roots": {"b": []}}}`,
		"roots relative":     `{"a": {"pins": {"b":"1"}, "roots": {"b": ["bin"]}}}`,
		"roots dotdot":       `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x/../y"]}}}`,
		"roots dot":          `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x/./y"]}}}`,
		"roots trailing":     `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x/"]}}}`,
		"roots double slash": `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x//y"]}}}`,
		"roots tmpdir":       `{"a": {"pins": {"b":"1"}, "roots": {"b": ["$TMPDIR/x"]}}}`,
		"roots variable":     `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x/$Y"]}}}`,
		"roots wildcard":     `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x/*"]}}}`,
		"roots backslash":    `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x\\y"]}}}`,
		"roots duplicate":    `{"a": {"pins": {"b":"1"}, "roots": {"b": ["/x", "/x"]}}}`,
		"roots too many":     `{"a": {"pins": {"b":"1"}, "roots": {"b": [` + strings.Repeat(`"/x",`, 16) + `"/y"]}}}`,
		"pin conflict":       `{"a": {"pins": {"t": "1"}}, "b": {"pins": {"t": "2"}}}`,
		"roots conflict":     `{"a": {"pins": {"t": "1"}, "roots": {"t": ["/x"]}}, "b": {"pins": {"t": "1"}, "roots": {"t": ["/y"]}}}`,
	}
	for name, src := range cases {
		if _, err := ParseToolchains([]byte(src)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	var many strings.Builder
	many.WriteString("{")
	for i := 0; i <= maxToolchains; i++ {
		if i > 0 {
			many.WriteString(",")
		}
		many.WriteString(`"t` + strings.Repeat("0", 3-len(itoa(i))) + itoa(i) + `": {"pins": {}}`)
	}
	many.WriteString("}")
	if _, err := ParseToolchains([]byte(many.String())); err == nil {
		t.Error("too many entries accepted")
	}
}

func itoa(i int) string {
	if i == 0 {
		return "0"
	}
	var b []byte
	for ; i > 0; i /= 10 {
		b = append([]byte{byte('0' + i%10)}, b...)
	}
	return string(b)
}

const sampleToolchains = `{
  "cc":   {"always": true, "pins": {"cc": "image", "ld": "image"}, "roots": {"cc": ["/usr/bin"]}},
  "go":   {"pins": {"go": "1.26.1"}, "roots": {"go": ["/opt/hostedtoolcache/go/1.26.1/x64/bin"]}},
  "mac":  {"always": true, "os": ["darwin"], "pins": {"xcode-select": "image"}},
  "rust": {"pins": {"cargo": "1.99.1", "rustc": "1.99.1"}, "roots": {"cargo": ["$HOME/.cargo/bin"], "rustc": ["$HOME/.cargo/bin"]}},
  "sh":   {"always": true, "pins": {"sh": "image"}},
  "wasm": {"pins": {"cargo": "1.99.1", "rustc": "1.99.1"}}
}`

func TestToolsFor(t *testing.T) {
	tc := mustToolchains(t, sampleToolchains)
	tools, unknown := tc.ToolsFor([]string{"rust", "go", "go", "nosuch", "alsonot"}, "linux", "img/1")
	if len(unknown) != 2 || unknown[0] != "alsonot" || unknown[1] != "nosuch" {
		t.Fatalf("unknown = %v", unknown)
	}
	want := []key.Tool{
		{Name: "cargo", Version: "1.99.1", Kind: key.ToolHost, Identity: []byte("1.99.1")},
		{Name: "cc", Version: ImagePin, Kind: key.ToolHost, Identity: []byte("img/1")},
		{Name: "go", Version: "1.26.1", Kind: key.ToolHost, Identity: []byte("1.26.1")},
		{Name: "ld", Version: ImagePin, Kind: key.ToolHost, Identity: []byte("img/1")},
		{Name: "rustc", Version: "1.99.1", Kind: key.ToolHost, Identity: []byte("1.99.1")},
		{Name: "sh", Version: ImagePin, Kind: key.ToolHost, Identity: []byte("img/1")},
	}
	if len(tools) != len(want) {
		t.Fatalf("tools = %+v", tools)
	}
	for i := range want {
		g, w := tools[i], want[i]
		if g.Name != w.Name || g.Version != w.Version || g.Kind != w.Kind || string(g.Identity) != string(w.Identity) {
			t.Fatalf("tool %d: got %+v want %+v", i, g, w)
		}
	}
	// The result is a valid KILN02 tools section.
	if _, err := key.ToolsDigest(tools); err != nil {
		t.Fatal(err)
	}
	// Platform-limited always entries come and go with the platform.
	mac, _ := tc.ToolsFor(nil, "darwin", "img/1")
	lin, _ := tc.ToolsFor(nil, "linux", "img/1")
	if len(mac) != len(lin)+1 {
		t.Fatalf("darwin %d tools, linux %d", len(mac), len(lin))
	}
	// An always entry named explicitly is not counted twice; a platform-limited
	// one that does not apply adds nothing.
	if tl, unk := tc.ToolsFor([]string{"sh", "mac"}, "linux", "i"); len(unk) != 0 || len(tl) != len(lin) {
		t.Fatalf("%v %v", tl, unk)
	}
	// A pin bump for one language changes only that language's tools.
	d1, _ := key.ToolsDigest(tools)
	bumped := mustToolchains(t, strings.Replace(sampleToolchains, `"go": "1.26.1"}, "roots"`, `"go": "1.26.2"}, "roots"`, 1))
	tl2, _ := bumped.ToolsFor([]string{"rust", "go"}, "linux", "img/1")
	d2, _ := key.ToolsDigest(tl2)
	if d1 == d2 {
		t.Fatal("a pin bump did not change the tools digest")
	}
	tl3, _ := bumped.ToolsFor([]string{"rust"}, "linux", "img/1")
	tl4, _ := tc.ToolsFor([]string{"rust"}, "linux", "img/1")
	e3, _ := key.ToolsDigest(tl3)
	e4, _ := key.ToolsDigest(tl4)
	if e3 != e4 {
		t.Fatal("an unrelated language's tools changed")
	}
	// The host image is the identity of image-pinned tools only.
	other, _ := tc.ToolsFor([]string{"go"}, "linux", "img/2")
	same, _ := tc.ToolsFor([]string{"go"}, "linux", "img/1")
	od, _ := key.ToolsDigest(other)
	sd, _ := key.ToolsDigest(same)
	if od == sd {
		t.Fatal("image identity missing from image-pinned tools")
	}
}

func TestConstructPATH(t *testing.T) {
	tc := mustToolchains(t, sampleToolchains)
	tools, _ := tc.ToolsFor([]string{"rust", "go"}, "linux", "i")
	got := tc.ConstructPATH(tools, "/home/u")
	want := "/home/u/.cargo/bin:/opt/hostedtoolcache/go/1.26.1/x64/bin:/usr/bin"
	if sep := string(filepath.ListSeparator); sep != ":" {
		want = strings.ReplaceAll(want, ":", sep)
	}
	if got != want {
		t.Fatalf("PATH = %q, want %q", got, want)
	}
	// Exactly the given tools: no rust, no cargo directory.
	goOnly, _ := tc.ToolsFor([]string{"go"}, "linux", "i")
	if p := tc.ConstructPATH(goOnly, "/home/u"); strings.Contains(p, "cargo") {
		t.Fatalf("PATH %q has an unrelated language's directory", p)
	}
	// Order does not depend on the order tools are given in.
	rev := []key.Tool{tools[len(tools)-1], tools[0]}
	fwd := []key.Tool{tools[0], tools[len(tools)-1]}
	if tc.ConstructPATH(rev, "/h") != tc.ConstructPATH(fwd, "/h") {
		t.Fatal("PATH order depends on input order")
	}
	// No home: $HOME roots are left out, not guessed.
	if p := tc.ConstructPATH(tools, ""); strings.Contains(p, "cargo") {
		t.Fatalf("PATH %q keeps a $HOME root without a home", p)
	}
	// Never `.`.
	if p := tc.ConstructPATH(nil, "/h"); p != "" {
		t.Fatalf("PATH for no tools = %q", p)
	}
}

func TestToolchainAccessors(t *testing.T) {
	tc := mustToolchains(t, sampleToolchains)
	if n := tc.Names(); len(n) != 6 || n[0] != "cc" || n[5] != "wasm" {
		t.Fatalf("Names = %v", n)
	}
	n := tc.Names()
	n[0] = "zzz"
	if tc.Names()[0] != "cc" {
		t.Fatal("Names exposes internal state")
	}
	if e, ok := tc.Entry("go"); !ok || e.Pins["go"] != "1.26.1" {
		t.Fatal("Entry")
	}
	if _, ok := tc.Entry("nope"); ok {
		t.Fatal("Entry found a missing entry")
	}
	if !tc.HasTool("rustc") || tc.HasTool("nope") {
		t.Fatal("HasTool")
	}
	if tc.Digest == ([32]byte{}) {
		t.Fatal("digest not set")
	}
}

func TestSystemDirAndValidators(t *testing.T) {
	for d, want := range map[string]bool{"/usr": true, "/usr/bin": true, "/bin": true, "/sbin": true, "/opt/homebrew/bin": true, "/usrx": false, "/opt/x": false, "/home/u": false} {
		if systemDir(d) != want {
			t.Errorf("systemDir(%q) = %v", d, !want)
		}
	}
	for _, v := range []string{"1", "1.2.3", "9.0.319", "5.4.7", "1.2.3-rc.1+b5", ImagePin} {
		if !validPin(v) {
			t.Errorf("pin %q refused", v)
		}
	}
	for _, v := range []string{"", "v1", "1 2", "1/2", strings.Repeat("1", 65)} {
		if validPin(v) {
			t.Errorf("pin %q accepted", v)
		}
	}
	for _, n := range []string{"a", "c++", "xcode-select", "Python", "a.b_c+d"} {
		if !validName(n) {
			t.Errorf("name %q refused", n)
		}
	}
	for _, n := range []string{"", "-a", "a b", "a:b", "a/b", "é"} {
		if validName(n) {
			t.Errorf("name %q accepted", n)
		}
	}
	for _, p := range []string{"", "/", "rel", "/a/", "$HOME/", "$HOMEX/a", "\x00"} {
		if checkPolicyPath(p, true, true) == nil {
			t.Errorf("path %q accepted", p)
		}
	}
	for _, p := range []string{"/a", "$HOME", "$HOME/a/b", "$TMPDIR/x", "/dev/fd/*"} {
		if err := checkPolicyPath(p, true, true); err != nil {
			t.Errorf("path %q refused: %v", p, err)
		}
	}
}
