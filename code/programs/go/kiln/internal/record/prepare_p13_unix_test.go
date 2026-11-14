//go:build unix

package record

// What `prepare` seals for the classifier (KILN04 section 4.8, decision P13):
// pre/tree, pre/classify.json and each action's tools, and that VerifyPre holds
// the new files to the same standard as the old ones.

import (
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// preFile reads a sealed file of the prepared directory.
func (p *prep) preFile(rel string) []byte {
	p.t.Helper()
	data, err := os.ReadFile(filepath.Join(p.dir, "pre", filepath.FromSlash(rel)))
	if err != nil {
		p.t.Fatal(err)
	}
	return data
}

func TestPrepareSealsTheTrackedTree(t *testing.T) {
	p := newPrep(t)
	p.links = map[string]string{"code/packages/x/a/lnk": "src.txt", "code/packages/x/a/sp ace": "../b/BUILD"}
	p.files["code/packages/x/a/run.sh"] = "#!/bin/sh\n"
	p.commit()
	// A tracked executable: the test repository writes 0644, so set the bit and
	// commit it.
	if err := os.Chmod(filepath.Join(p.root, "code/packages/x/a/run.sh"), 0o755); err != nil {
		t.Fatal(err)
	}
	runGit(t, p.root, "add", "-A")
	runGit(t, p.root, "commit", "-q", "-m", "exec")
	p.sha = runGit(t, p.root, "rev-parse", "HEAD")
	p.getenv["GITHUB_SHA"] = p.sha
	p.mustPrepare()

	data := p.preFile("tree")
	if !strings.HasPrefix(string(data), "kiln-record-tree/1\n") || data[len(data)-1] != '\n' {
		t.Fatalf("%q", data[:40])
	}
	tf, err := DecodeTree(data)
	if err != nil {
		t.Fatal(err)
	}
	// Byte for byte what git says about the commit: `ls-tree -r`.
	want := map[string][2]string{}
	for _, line := range strings.Split(strings.TrimSpace(runGit(t, p.root, "ls-tree", "-r", "--full-tree", p.sha)), "\n") {
		meta, path, _ := strings.Cut(line, "\t")
		f := strings.Fields(meta)
		want[path] = [2]string{f[0], f[2]}
	}
	if tf.Len() != len(want) {
		t.Fatalf("%d entries, git has %d", tf.Len(), len(want))
	}
	for path, w := range want {
		mode, id, _, ok := tf.Entry(path)
		if !ok || mode != w[0] || id != w[1] {
			t.Errorf("%s: %s %s, git says %s %s", path, mode, id, w[0], w[1])
		}
	}
	// A link's text is listed, spaces escaped; a file's mode is its own.
	if _, _, text, _ := tf.Entry("code/packages/x/a/lnk"); text != "src.txt" {
		t.Fatalf("link text %q", text)
	}
	if mode, _, _, _ := tf.Entry("code/packages/x/a/run.sh"); mode != modeExec {
		t.Fatalf("mode %s", mode)
	}
	if !strings.Contains(string(data), " code/packages/x/a/sp%20ace ../b/BUILD\n") {
		t.Fatalf("an escaped path with a link text is missing:\n%s", data)
	}
	// The pre/ that VerifyPre accepts lists it, sealed like every file.
	pre, err := VerifyPre(p.dir)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := pre.Sums["tree"]; !ok {
		t.Fatal("tree is not sealed")
	}
	if got, err := pre.Tree(); err != nil || got.Len() != tf.Len() {
		t.Fatalf("%v", err)
	}
}

// Same inputs, same bytes: a second prepare of the same commit into another
// directory writes the same tree and the same classify.json.
func TestPrepareTreeAndClassifyAreDeterministic(t *testing.T) {
	p := newPrep(t)
	p.commit()
	p.mustPrepare()
	tree1, classify1 := p.preFile("tree"), p.preFile("classify.json")
	p.writable()
	p.dir = filepath.Join(p.base, "record2")
	p.mustPrepare()
	if string(tree1) != string(p.preFile("tree")) || string(classify1) != string(p.preFile("classify.json")) {
		t.Fatal("not deterministic")
	}
}

func TestPrepareSealsTheClassifierPolicy(t *testing.T) {
	p := newPrep(t)
	p.commit()
	p.mustPrepare()
	c, err := DecodeClassify(p.preFile("classify.json"))
	if err != nil {
		t.Fatal(err)
	}
	if c.Schema != ClassifySchema || !reflect.DeepEqual(c.Pinned, []string{}) {
		t.Fatalf("%+v", c)
	}
	var paths []string
	for _, r := range c.HostRoots.Roots {
		paths = append(paths, r.Path+":"+r.Kind)
	}
	if !reflect.DeepEqual(paths, []string{"/bin:image", "/sbin:image"}) || !reflect.DeepEqual(c.HostRoots.Ephemeral, []string{"/dev/null"}) || len(c.HostRoots.AbsentOK) != 0 {
		t.Fatalf("%+v", c.HostRoots)
	}
	// A package whose key is refused still gets its tools.
	pre, err := VerifyPre(p.dir)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(pre.Classify, c) {
		t.Fatal("VerifyPre decoded something else")
	}
}

func TestPrepareWritesEachActionsTools(t *testing.T) {
	p := newPrep(t)
	p.commit()
	p.mustPrepare()
	pre, err := VerifyPre(p.dir)
	if err != nil {
		t.Fatal(err)
	}
	for _, name := range pre.Actions() {
		a, _ := pre.Action(name)
		if a.Schema != "kiln-record-action/2" || a.Tools == nil || !sort.StringsAreSorted(a.Tools) {
			t.Errorf("%s: %+v", name, a)
		}
		// The always-on tools (cc and sh, in the fixture) are in every action.
		if !containsStr(a.Tools, "cc") || !containsStr(a.Tools, "sh") {
			t.Errorf("%s: tools %v", name, a.Tools)
		}
		// And exactly the weak key's tool names.
		if a.Weak != "" && a.Refusal != "" {
			t.Errorf("%s: both", name)
		}
	}
}

// An action with a refusal and no tools is still a valid file (a refused key
// whose closure knows no tool): the encoder writes [] and the decoder, which
// refuses null, accepts it. (That a refused key's tools come from RecordEnv is
// covered by legacy's TestRecordEnvForARefusedPackage.)
func TestAnActionWithARefusalAndNoToolsEncodes(t *testing.T) {
	data, err := EncodeAction(Action{Schema: ActionSchema, Action: "code/p", Language: "x", Refusal: "unknown-toolchain:y"})
	if err != nil || !strings.Contains(string(data), `"tools":[]`) {
		t.Fatalf("%s %v", data, err)
	}
	if _, err := DecodeAction(data); err != nil {
		t.Fatal(err)
	}
}

// --- VerifyPre holds the new files to the same standard ---------------------

func TestVerifyPreCatchesTamperingWithTheNewFiles(t *testing.T) {
	for _, rel := range []string{"tree", "classify.json"} {
		t.Run(rel+" modified", func(t *testing.T) {
			f := newStepFix(t)
			f.p.writable()
			path := filepath.Join(f.p.dir, "pre", rel)
			data, _ := os.ReadFile(path)
			if err := os.WriteFile(path, append(data, ' '), 0o644); err != nil {
				t.Fatal(err)
			}
			if _, err := VerifyPre(f.p.dir); err == nil || !strings.Contains(err.Error(), "was modified") {
				t.Fatalf("%v", err)
			}
		})
		t.Run(rel+" removed", func(t *testing.T) {
			f := newStepFix(t)
			f.p.writable()
			os.Remove(filepath.Join(f.p.dir, "pre", rel))
			if _, err := VerifyPre(f.p.dir); err == nil || !strings.Contains(err.Error(), "missing") {
				t.Fatalf("%v", err)
			}
		})
		t.Run(rel+" unsealed", func(t *testing.T) {
			f := newStepFix(t)
			f.reseal(nil, false)
			sealed, _ := os.ReadFile(filepath.Join(f.p.dir, "pre", "SEALED"))
			var keep []string
			for _, l := range strings.Split(strings.TrimSuffix(string(sealed), "\n"), "\n") {
				if !strings.HasSuffix(l, "  "+rel) {
					keep = append(keep, l)
				}
			}
			os.WriteFile(filepath.Join(f.p.dir, "pre", "SEALED"), []byte(strings.Join(keep, "\n")+"\n"), 0o644)
			if _, err := VerifyPre(f.p.dir); err == nil {
				t.Fatal("accepted a SEALED without " + rel)
			}
		})
	}
}

func TestVerifyPreDecodesTheNewFilesStrictly(t *testing.T) {
	t.Run("classify.json that does not decode", func(t *testing.T) {
		for name, body := range map[string]string{
			"garbage":        "{",
			"another schema": `{"schema":"kiln-record-classify/2","host_roots":{"roots":[],"ephemeral":[],"absent-ok":[]},"pinned":[]}`,
			"null pinned":    `{"schema":"kiln-record-classify/1","host_roots":{"roots":[],"ephemeral":[],"absent-ok":[]},"pinned":null}`,
			"an unknown key": `{"schema":"kiln-record-classify/1","host_roots":{"roots":[],"ephemeral":[],"absent-ok":[]},"pinned":[],"extra":1}`,
			"unsorted roots": `{"schema":"kiln-record-classify/1","host_roots":{"roots":[{"path":"/b","kind":"image"},{"path":"/a","kind":"image"}],"ephemeral":[],"absent-ok":[]},"pinned":[]}`,
		} {
			f := newStepFix(t)
			f.reseal(map[string][]byte{"classify.json": []byte(body)}, true)
			if _, err := VerifyPre(f.p.dir); err == nil || !strings.Contains(err.Error(), "classify") {
				t.Errorf("%s: %v", name, err)
			}
		}
	})
	t.Run("a malformed tree is found when it is read", func(t *testing.T) {
		f := newStepFix(t)
		f.reseal(map[string][]byte{"tree": []byte("kiln-record-tree/1\nnot a line\n")}, true)
		pre, err := VerifyPre(f.p.dir) // hashed here, decoded on use
		if err != nil {
			t.Fatal(err)
		}
		if _, err := pre.Tree(); err == nil || !strings.Contains(err.Error(), "tree line 1") {
			t.Fatalf("%v", err)
		}
	})
	t.Run("an action of the old schema", func(t *testing.T) {
		f := newStepFix(t)
		rel := ActionFile(actZ)
		data, _ := os.ReadFile(filepath.Join(f.p.dir, "pre", rel))
		old := strings.Replace(string(data), "action/2", "action/1", 1)
		old = strings.Replace(old, `,"tools":`, `,"x":`, 1)
		f.reseal(map[string][]byte{rel: []byte(old)}, true)
		if _, err := VerifyPre(f.p.dir); err == nil {
			t.Fatal("an action/1 file was accepted")
		}
	})
}

func TestPreReadSealedRefusesAFileChangedSinceVerification(t *testing.T) {
	f := newStepFix(t)
	if _, err := f.pre.ReadSealed("graph.json"); err != nil {
		t.Fatal(err)
	}
	if _, err := f.pre.ReadSealed("not-sealed"); err == nil {
		t.Fatal("read a file SEALED does not list")
	}
	f.p.writable()
	os.WriteFile(filepath.Join(f.p.dir, "pre", "graph.json"), []byte("{}"), 0o644)
	if _, err := f.pre.ReadSealed("graph.json"); err == nil || !strings.Contains(err.Error(), "was modified") {
		t.Fatalf("%v", err)
	}
	if _, err := f.pre.Tree(); err != nil {
		t.Fatal(err)
	}
	os.Remove(filepath.Join(f.p.dir, "pre", "tree"))
	if _, err := f.pre.Tree(); err == nil {
		t.Fatal("a missing tree was read")
	}
}

// --- the tree of a real git repository, links included ----------------------

func TestEncodeTreeFromAVerifiedRepositoryWithLinks(t *testing.T) {
	root := newRepo(t, map[string]string{"a.txt": "a\n", "d/b.txt": "b\n"})
	symlink(t, root, "d/link", "../a.txt")
	runGit(t, root, "add", "-A")
	runGit(t, root, "commit", "-q", "-m", "link")
	res, err := VerifyTree(root, "")
	if err != nil {
		t.Fatal(err)
	}
	if len(res.entries) != 3 {
		t.Fatalf("%v", res.entries)
	}
	data, err := EncodeTree(res.entries, func(p string) (string, error) { return os.Readlink(filepath.Join(root, p)) })
	if err != nil {
		t.Fatal(err)
	}
	tf, err := DecodeTree(data)
	if err != nil {
		t.Fatal(err)
	}
	if n := tf.Stat("d/link"); n.Kind != traced.KindLink || n.Target != "../a.txt" {
		t.Fatalf("%+v", n)
	}
	// A link whose disk text is not what the commit holds is never written.
	os.Remove(filepath.Join(root, "d/link"))
	os.Symlink("../b.txt", filepath.Join(root, "d/link"))
	if _, err := EncodeTree(res.entries, func(p string) (string, error) { return os.Readlink(filepath.Join(root, p)) }); err == nil || !strings.Contains(err.Error(), "does not hash") {
		t.Fatalf("%v", err)
	}
}

// --- cost -------------------------------------------------------------------

// A tree of the real repository's size is hashed on every step and decoded once
// per action: both must stay cheap. This does not assert a wall-clock bound
// (machines differ); it prints the figures the report quotes.
func TestTreeCostAtTheSizeOfTheRealRepository(t *testing.T) {
	if testing.Short() {
		t.Skip("short mode")
	}
	const n = 394000
	tree := make(map[string]entry, n)
	for i := 0; i < n; i++ {
		path := "code/packages/lang/package-" + string(rune('a'+i%26)) + "/src/dir" + itoa(i/26%500) + "/file" + itoa(i) + ".go"
		tree[path] = entry{modeFile, blobID(path)}
	}
	start := time.Now()
	data, err := EncodeTree(tree, nil)
	if err != nil {
		t.Fatal(err)
	}
	enc := time.Since(start)
	start = time.Now()
	tf, err := DecodeTree(data)
	if err != nil || tf.Len() != n {
		t.Fatal(err)
	}
	t.Logf("%d entries: %d bytes; encode %v, strict decode %v", n, len(data), enc, time.Since(start))
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
