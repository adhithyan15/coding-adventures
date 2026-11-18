package pkgmeta

import (
	"reflect"
	"sort"
	"testing"
)

func TestIsBuildFront(t *testing.T) {
	for _, tc := range []struct {
		path string
		want bool
	}{
		{"BUILD", true},
		{"BUILD_linux", true},
		{"BUILD_mac", true},
		{"BUILD_mac_and_linux", true},
		{"BUILD_windows", true},
		{"nested/BUILD", false},
		{"BUILD.bak", false},
		{"BUILD_debug", false},
		{"BUILD_Windows", false},
		{"build", false},
		{"BUILD/", false},
		{"BUILD\\x", false},
		{"", false},
		{"BUILD-EXEMPTIONS", false},
		{"BUILD_mac_and_linux2", false},
	} {
		if got := IsBuildFront(tc.path); got != tc.want {
			t.Errorf("IsBuildFront(%q) = %t, want %t", tc.path, got, tc.want)
		}
	}
}

func TestIsPackageLocalIsTheUnion(t *testing.T) {
	for _, path := range []string{"CHANGELOG.md", "evidence/a.json", "BUILD", "BUILD_windows", "README.md", "src/lib.rs", "nested/BUILD", ""} {
		if got, want := IsPackageLocal(path), IsMetadata(path) || IsBuildFront(path); got != want {
			t.Errorf("IsPackageLocal(%q) = %t, want %t", path, got, want)
		}
	}
	if !IsPackageLocal("BUILD") || IsMetadata("BUILD") {
		t.Error("BUILD is package-local but not metadata")
	}
}

func TestBuildFrontLocalFor(t *testing.T) {
	for language, want := range map[string]bool{
		"rust": true, "typescript": false, "lua": false, "wasm": false, "go": false, "": false, "Rust": false,
	} {
		if got := BuildFrontLocalFor(language); got != want {
			t.Errorf("BuildFrontLocalFor(%q) = %t, want %t", language, got, want)
		}
	}
}

func TestScanBuildFrontReferences(t *testing.T) {
	pkg := func(name, dir, front, content string) PackageFronts {
		return PackageFronts{Name: name, Dir: dir, Fronts: map[string]string{front: content}}
	}
	for _, tc := range []struct {
		name           string
		owner          PackageFronts
		others         []PackageFronts
		wantExecuted   []string
		wantUnresolved int
	}{
		{
			name:         "cd and sh BUILD",
			owner:        pkg("lua/nib", "nib", "BUILD", "(cd ../compiler_ir && sh BUILD)\n"),
			others:       []PackageFronts{pkg("lua/compiler_ir", "compiler_ir", "BUILD", "luarocks make\n")},
			wantExecuted: []string{"lua/compiler_ir"},
		},
		{
			name:         "windows call BUILD_windows",
			owner:        pkg("lua/wasm_runtime", "wasm_runtime", "BUILD_windows", "luarocks show x 1>nul 2>nul || (cd ..\\wasm_execution && call BUILD_windows)\n"),
			others:       []PackageFronts{pkg("lua/wasm_execution", "wasm_execution", "BUILD", "x\n")},
			wantExecuted: []string{"lua/wasm_execution"},
		},
		{
			name:         "REPO_ROOT path with bash BUILD",
			owner:        pkg("lua/barcode_1d", "barcode_1d", "BUILD", "(cd \"$REPO_ROOT/code/packages/lua/paint_vm_metal_native\" && bash BUILD)\n"),
			others:       []PackageFronts{pkg("lua/paint_vm_metal_native", "paint_vm_metal_native", "BUILD", "x\n")},
			wantExecuted: []string{"lua/paint_vm_metal_native"},
		},
		{
			name:  "same basename in two languages: both executed",
			owner: pkg("python/conduit-hello", "conduit-hello", "BUILD", "(cd ../conduit && bash BUILD)\n"),
			others: []PackageFronts{
				pkg("python/conduit", "conduit", "BUILD", "x\n"),
				pkg("rust/conduit", "conduit", "BUILD", "x\n"),
			},
			wantExecuted: []string{"python/conduit", "rust/conduit"},
		},
		{
			name:   "shell comment",
			owner:  pkg("a/x", "x", "BUILD", "# (cd ../y && sh BUILD)\n"),
			others: []PackageFronts{pkg("a/y", "y", "BUILD", "x\n")},
		},
		{
			name:   "REM comment",
			owner:  pkg("a/x", "x", "BUILD_windows", "@REM Mirror BUILD for y\nrem BUILD y\n"),
			others: []PackageFronts{pkg("a/y", "y", "BUILD", "x\n")},
		},
		{
			name:   "double-colon comment",
			owner:  pkg("a/x", "x", "BUILD_windows", ":: BUILD y\n"),
			others: []PackageFronts{pkg("a/y", "y", "BUILD", "x\n")},
		},
		{
			name:   "echo message mentioning BUILD is not an invocation",
			owner:  pkg("a/x", "x", "BUILD", "echo \"x BUILD requires Python 3\"\n"),
			others: []PackageFronts{pkg("a/y", "y", "BUILD", "x\n")},
		},
		{
			name:   "unsupported marker is not a mention",
			owner:  pkg("a/x", "x", "BUILD_windows", "echo BUILD_TOOL_UNSUPPORTED:NO_METAL -- skipped\n"),
			others: []PackageFronts{pkg("a/y", "y", "BUILD", "x\n")},
		},
		{
			name:           "glob loop is unresolved",
			owner:          pkg("a/x", "x", "BUILD", "for d in ../*/; do (cd $d && sh BUILD); done\n"),
			others:         []PackageFronts{pkg("a/y", "y", "BUILD", "x\n")},
			wantUnresolved: 1,
		},
		{
			name:           "self invocation is unresolved and never marks itself",
			owner:          pkg("a/x", "x", "BUILD", "sh BUILD\n"),
			wantUnresolved: 1,
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			executed, unresolved := ScanBuildFrontReferences(append([]PackageFronts{tc.owner}, tc.others...))
			var got []string
			for name := range executed {
				got = append(got, name)
			}
			sort.Strings(got)
			if !reflect.DeepEqual(got, tc.wantExecuted) && !(len(got) == 0 && len(tc.wantExecuted) == 0) {
				t.Fatalf("executed = %v, want %v", got, tc.wantExecuted)
			}
			if len(unresolved) != tc.wantUnresolved {
				t.Fatalf("unresolved = %v, want %d", unresolved, tc.wantUnresolved)
			}
			if executed[tc.owner.Name] {
				t.Fatal("a package must never mark itself executed")
			}
		})
	}
	if IsCommentLine("cargo test") || !IsCommentLine("   ") || !IsCommentLine("  # x") {
		t.Fatal("IsCommentLine misclassifies")
	}
	if !IsFrontInvocation("./BUILD") || !IsFrontInvocation("source ../x/BUILD_mac") || IsFrontInvocation("cargo test") {
		t.Fatal("IsFrontInvocation misclassifies")
	}
}
