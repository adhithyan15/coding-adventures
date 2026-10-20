package irtojvmclassfile

import (
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// ── Write-safety tests, run under every walk ───────────────────────────────
//
// Each test below runs once per strategy.  On POSIX that means the openat
// descriptor walk ("auto") *and* the Windows path-check walk ("path-checks"),
// so the Windows code is exercised on every CI job rather than only on the
// Windows one.  On Windows both strategies are the path-check walk.

var writeStrategies = []struct {
	name     string
	strategy classFileWriteStrategy
}{
	{"auto", writeStrategyAuto},
	{"path-checks", writeStrategyPathChecks},
}

func forEachWriteStrategy(t *testing.T, test func(t *testing.T, write func(*JVMClassArtifact, string) (string, error))) {
	t.Helper()
	for _, entry := range writeStrategies {
		strategy := entry.strategy
		t.Run(entry.name, func(t *testing.T) {
			test(t, func(artifact *JVMClassArtifact, outputDir string) (string, error) {
				return writeClassFileWithStrategy(artifact, outputDir, strategy)
			})
		})
	}
}

func demoArtifact(t *testing.T) *JVMClassArtifact {
	t.Helper()
	artifact, err := LowerIRToJvmClassFile(simpleProgram(), JvmBackendConfig{
		ClassName:       "demo.Example",
		EmitMainWrapper: true,
	})
	if err != nil {
		t.Fatalf("lower failed: %v", err)
	}
	return artifact
}

// makeDirectoryLink plants a directory link at link pointing to target.  On
// Windows it makes a junction, which needs no privilege (a directory symlink
// needs Developer Mode or admin) and is the link an attacker would plant.
func makeDirectoryLink(t *testing.T, target string, link string) {
	t.Helper()
	if runtime.GOOS == "windows" {
		output, err := exec.Command("cmd", "/c", "mklink", "/J", link, target).CombinedOutput()
		if err != nil {
			t.Fatalf("mklink /J failed: %v\n%s", err, output)
		}
		return
	}
	if err := os.Symlink(target, link); err != nil {
		t.Fatalf("symlink failed: %v", err)
	}
}

// canonical resolves path to an absolute, link-free form.
func canonical(t *testing.T, path string) string {
	t.Helper()
	absolute, err := filepath.Abs(path)
	if err != nil {
		t.Fatalf("absolutize failed: %v", err)
	}
	resolved, err := filepath.EvalSymlinks(absolute)
	if err != nil {
		t.Fatalf("canonicalize failed: %v", err)
	}
	return resolved
}

func requireErrorContaining(t *testing.T, err error, fragment string) {
	t.Helper()
	if err == nil || !strings.Contains(err.Error(), fragment) {
		t.Fatalf("expected error containing %q, got %v", fragment, err)
	}
}

func requireNoTempFiles(t *testing.T, dir string) {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("read dir failed: %v", err)
	}
	for _, entry := range entries {
		if strings.HasSuffix(entry.Name(), ".tmp") {
			t.Fatalf("temporary file left behind: %s", entry.Name())
		}
	}
}

func TestWriteClassFileUsesClasspathLayout(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		outputDir := t.TempDir()
		target, err := write(artifact, outputDir)
		if err != nil {
			t.Fatalf("write failed: %v", err)
		}
		expected := filepath.Join(canonical(t, outputDir), "demo", "Example.class")
		if target != expected {
			t.Fatalf("expected %q, got %q", expected, target)
		}
		data, err := os.ReadFile(target)
		if err != nil {
			t.Fatalf("read failed: %v", err)
		}
		if string(data) != string(artifact.ClassBytes) {
			t.Fatal("written class file does not match artifact bytes")
		}
		requireNoTempFiles(t, filepath.Dir(target))
	})
}

func TestWriteClassFileCreatesMissingRootDirectories(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		// Canonical base: the Unix descriptor walk creates a missing root by
		// walking down from "/" with O_NOFOLLOW, so it refuses ordinary
		// system links above the root (macOS's /var -> /private/var).
		base := canonical(t, t.TempDir())
		target, err := write(artifact, filepath.Join(base, "new", "deeper"))
		if err != nil {
			t.Fatalf("write failed: %v", err)
		}
		expected := filepath.Join(base, "new", "deeper", "demo", "Example.class")
		if target != expected {
			t.Fatalf("expected %q, got %q", expected, target)
		}
		if _, err := os.Stat(target); err != nil {
			t.Fatalf("class file missing: %v", err)
		}
	})
}

func TestWriteClassFileAcceptsRelativeOutputRoot(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		base := canonical(t, t.TempDir()) // see CreatesMissingRootDirectories
		t.Chdir(base)
		target, err := write(artifact, filepath.Join("out", "nested"))
		if err != nil {
			t.Fatalf("write failed: %v", err)
		}
		expected := filepath.Join(base, "out", "nested", "demo", "Example.class")
		if target != expected {
			t.Fatalf("expected %q, got %q", expected, target)
		}
	})
}

func TestWriteClassFileDefaultsToCurrentDirectory(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		base := t.TempDir()
		t.Chdir(base)
		target, err := write(artifact, "")
		if err != nil {
			t.Fatalf("write failed: %v", err)
		}
		// The descriptor walk reports "demo/Example.class" relative to the
		// working directory; the path-check walk reports it absolute.  Both
		// name the same file.
		expected := filepath.Join(canonical(t, base), "demo", "Example.class")
		if canonical(t, target) != expected {
			t.Fatalf("expected %q, got %q", expected, target)
		}
	})
}

func TestWriteClassFileOverwritesExistingClassFile(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		outputDir := t.TempDir()
		packageDir := filepath.Join(outputDir, "demo")
		if err := os.Mkdir(packageDir, 0o755); err != nil {
			t.Fatalf("mkdir failed: %v", err)
		}
		stale := strings.Repeat("stale bytes that are longer than the class ", 200)
		if err := os.WriteFile(filepath.Join(packageDir, "Example.class"), []byte(stale), 0o644); err != nil {
			t.Fatalf("seed failed: %v", err)
		}
		target, err := write(artifact, outputDir)
		if err != nil {
			t.Fatalf("write failed: %v", err)
		}
		data, err := os.ReadFile(target)
		if err != nil {
			t.Fatalf("read failed: %v", err)
		}
		if string(data) != string(artifact.ClassBytes) {
			t.Fatal("existing class file was not fully replaced")
		}
		requireNoTempFiles(t, packageDir)
	})
}

func TestWriteClassFileRejectsNilArtifact(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		_, err := write(nil, t.TempDir())
		requireErrorContaining(t, err, "artifact must not be nil")
	})
}

func TestWriteClassFileRejectsEscapingArtifact(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := &JVMClassArtifact{
			ClassName:  ".Escape",
			ClassBytes: []byte("class-bytes"),
		}
		_, err := write(artifact, t.TempDir())
		requireErrorContaining(t, err, "escapes the requested")
	})
}

func TestValidatedOutputRelativePath(t *testing.T) {
	cases := []struct {
		classFilename string
		want          string // "" means the filename must be rejected
	}{
		{"demo/Example.class", filepath.Join("demo", "Example.class")},
		{"Example.class", "Example.class"},
		{"demo//Example.class", filepath.Join("demo", "Example.class")},
		{"", ""},
		{"/Example.class", ""},
		{"../Example.class", ""},
		{"demo/../../Example.class", ""},
		{"/.class", ""},
		{"demo\\..\\..\\x.class", ""},
		{"..\\x.class", ""},
		{"C:x.class", ""},
		{"demo/Example.class:stream", ""},
		{"demo/Ex\x00ample.class", ""},
	}
	for _, testCase := range cases {
		got, err := validatedOutputRelativePath(testCase.classFilename)
		if testCase.want == "" {
			if err == nil {
				t.Errorf("%q: expected rejection, got %q", testCase.classFilename, got)
			}
			continue
		}
		if err != nil || got != testCase.want {
			t.Errorf("%q: expected %q, got %q (%v)", testCase.classFilename, testCase.want, got, err)
		}
	}
}

func TestWriteClassFileRejectsSymlinkedParentDirectory(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		root := t.TempDir()
		sink := t.TempDir()
		makeDirectoryLink(t, sink, filepath.Join(root, "demo"))
		_, err := write(artifact, root)
		requireErrorContaining(t, err, "symlinked or invalid directory component")
		if _, statErr := os.Stat(filepath.Join(sink, "Example.class")); !os.IsNotExist(statErr) {
			t.Fatalf("class file escaped into the link target: %v", statErr)
		}
	})
}

func TestWriteClassFileRejectsOutputRootWithSymlinkedAncestor(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		root := t.TempDir()
		sink := t.TempDir()
		makeDirectoryLink(t, sink, filepath.Join(root, "linked-root"))
		_, err := write(artifact, filepath.Join(root, "linked-root", "nested"))
		requireErrorContaining(t, err, "symlinked or invalid path component")
		if _, statErr := os.Stat(filepath.Join(sink, "nested")); !os.IsNotExist(statErr) {
			t.Fatalf("directory created through the link: %v", statErr)
		}
	})
}

func TestWriteClassFileRejectsOutputRootThatIsALink(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		root := t.TempDir()
		sink := t.TempDir()
		link := filepath.Join(root, "classes")
		makeDirectoryLink(t, sink, link)
		_, err := write(artifact, link)
		requireErrorContaining(t, err, "symlinked or invalid path component")
	})
}

func TestWriteClassFileRejectsOutputRootThatIsAFile(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		rootFile := filepath.Join(t.TempDir(), "classes")
		if err := os.WriteFile(rootFile, []byte("not a directory"), 0o644); err != nil {
			t.Fatalf("seed failed: %v", err)
		}
		_, err := write(artifact, rootFile)
		requireErrorContaining(t, err, "symlinked or invalid path component")
	})
}

func TestWriteClassFileRejectsPackageDirectoryThatIsAFile(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		root := t.TempDir()
		if err := os.WriteFile(filepath.Join(root, "demo"), []byte("not a directory"), 0o644); err != nil {
			t.Fatalf("seed failed: %v", err)
		}
		_, err := write(artifact, root)
		requireErrorContaining(t, err, "symlinked or invalid directory component")
	})
}

func TestWriteClassFileRejectsOutputPathThatIsADirectory(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		root := t.TempDir()
		if err := os.MkdirAll(filepath.Join(root, "demo", "Example.class"), 0o755); err != nil {
			t.Fatalf("seed failed: %v", err)
		}
		_, err := write(artifact, root)
		requireErrorContaining(t, err, "symlinked or invalid output file")
	})
}

func TestWriteClassFileRejectsSymlinkedOutputFile(t *testing.T) {
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		root := t.TempDir()
		sentinel := filepath.Join(t.TempDir(), "sentinel.txt")
		if err := os.WriteFile(sentinel, []byte("keep me"), 0o644); err != nil {
			t.Fatalf("seed failed: %v", err)
		}
		if err := os.Mkdir(filepath.Join(root, "demo"), 0o755); err != nil {
			t.Fatalf("mkdir failed: %v", err)
		}
		if err := os.Symlink(sentinel, filepath.Join(root, "demo", "Example.class")); err != nil {
			if runtime.GOOS == "windows" {
				t.Skipf("file symlinks need Developer Mode or admin on Windows: %v", err)
			}
			t.Fatalf("symlink failed: %v", err)
		}
		_, err := write(artifact, root)
		requireErrorContaining(t, err, "symlinked or invalid output file")
		data, readErr := os.ReadFile(sentinel)
		if readErr != nil || string(data) != "keep me" {
			t.Fatalf("link target was modified: %q (%v)", data, readErr)
		}
	})
}

func TestWriteClassFileReportsUncreatablePackageDirectory(t *testing.T) {
	if runtime.GOOS == "windows" || os.Geteuid() == 0 {
		t.Skip("needs POSIX permission bits enforced against a non-root user")
	}
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		root := t.TempDir()
		if err := os.Chmod(root, 0o555); err != nil {
			t.Fatalf("chmod failed: %v", err)
		}
		t.Cleanup(func() { _ = os.Chmod(root, 0o755) })
		_, err := write(artifact, root)
		requireErrorContaining(t, err, "failed to create output directory")
	})
}

func TestReplaceFileAtomicallyRemovesTempFileWhenRenameFails(t *testing.T) {
	dir := t.TempDir()
	// A non-empty directory at the target name makes the final rename fail on
	// every platform.  (writeClassFileViaPathChecks refuses this case before
	// it gets here; the test drives the helper directly to reach the cleanup.)
	if err := os.MkdirAll(filepath.Join(dir, "Example.class", "occupied"), 0o755); err != nil {
		t.Fatalf("seed failed: %v", err)
	}
	err := replaceFileAtomically(dir, "Example.class", []byte("class-bytes"))
	requireErrorContaining(t, err, "failed to move class file into place")
	requireNoTempFiles(t, dir)
}

func TestReplaceFileAtomicallyReportsUnwritableDirectory(t *testing.T) {
	err := replaceFileAtomically(filepath.Join(t.TempDir(), "missing"), "Example.class", []byte("class-bytes"))
	requireErrorContaining(t, err, "failed to create temporary class file")
}

func TestPathChecksRefuseRootWithNoExistingAncestor(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("only Windows has volume roots that can be missing")
	}
	// Find a drive letter with nothing mounted on it.
	for letter := 'Z'; letter >= 'D'; letter-- {
		volume := string(letter) + `:\`
		if _, err := os.Lstat(volume); err == nil {
			continue
		}
		_, err := writeClassFileWithStrategy(demoArtifact(t), volume+`out`, writeStrategyPathChecks)
		requireErrorContaining(t, err, "failed to inspect output root")
		return
	}
	t.Skip("every drive letter is mapped")
}

func TestWriteClassFileReportsUncreatableRootComponent(t *testing.T) {
	if runtime.GOOS == "windows" || os.Geteuid() == 0 {
		t.Skip("needs POSIX permission bits enforced against a non-root user")
	}
	forEachWriteStrategy(t, func(t *testing.T, write func(*JVMClassArtifact, string) (string, error)) {
		artifact := demoArtifact(t)
		base := canonical(t, t.TempDir())
		if err := os.Chmod(base, 0o555); err != nil {
			t.Fatalf("chmod failed: %v", err)
		}
		t.Cleanup(func() { _ = os.Chmod(base, 0o755) })
		_, err := write(artifact, filepath.Join(base, "missing"))
		requireErrorContaining(t, err, "failed to create output root component")
	})
}

// The public entry point is the auto strategy; check it is wired up.
func TestWriteClassFileUsesPlatformWalk(t *testing.T) {
	artifact := demoArtifact(t)
	outputDir := t.TempDir()
	target, err := WriteClassFile(artifact, outputDir)
	if err != nil {
		t.Fatalf("write failed: %v", err)
	}
	if target != filepath.Join(canonical(t, outputDir), "demo", "Example.class") {
		t.Fatalf("unexpected target %q", target)
	}
}
