package discovery

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
	"testing"
)

type sharedDiscoveryFixture struct {
	Workspace struct {
		Files []struct {
			Path        string `json:"path"`
			ContentUTF8 string `json:"content_utf8"`
		} `json:"files"`
	} `json:"workspace"`
	Expected struct {
		Result struct {
			Packages []struct {
				Name     string `json:"name"`
				Language string `json:"language"`
				RelPath  string `json:"rel_path"`
			} `json:"packages"`
		} `json:"result"`
		Diagnostics []struct {
			Code    string `json:"code"`
			Path    string `json:"path"`
			Package string `json:"package"`
			Details struct {
				Paths []string `json:"paths"`
			} `json:"details"`
		} `json:"diagnostics"`
	} `json:"expected"`
}

func loadSharedDiscoveryFixture(t *testing.T, filename string) sharedDiscoveryFixture {
	t.Helper()
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate discovery test source")
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", "..", "..", ".."))
	fixturePath := filepath.Join(repoRoot, "code", "specs", "fixtures", "build-tool-v1", "cases", filename)
	data, err := os.ReadFile(fixturePath)
	if err != nil {
		t.Fatal(err)
	}
	var fixture sharedDiscoveryFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatal(err)
	}
	return fixture
}

func materializeSharedDiscoveryFixture(t *testing.T, fixture sharedDiscoveryFixture) string {
	t.Helper()
	root := t.TempDir()
	for _, file := range fixture.Workspace.Files {
		path := filepath.Join(root, filepath.FromSlash(file.Path))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(file.ContentUTF8), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

func TestDiscoverPackagesConsumesSharedLanguageRegistry(t *testing.T) {
	fixture := loadSharedDiscoveryFixture(t, "discovery-language-registry.json")
	root := materializeSharedDiscoveryFixture(t, fixture)

	packages, err := DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	actual := make([]string, 0, len(packages))
	for _, pkg := range packages {
		relPath, err := filepath.Rel(root, pkg.Path)
		if err != nil {
			t.Fatal(err)
		}
		actual = append(actual, strings.Join([]string{pkg.Name, pkg.Language, filepath.ToSlash(relPath)}, "|"))
	}
	want := make([]string, 0, len(fixture.Expected.Result.Packages))
	for _, pkg := range fixture.Expected.Result.Packages {
		want = append(want, strings.Join([]string{pkg.Name, pkg.Language, pkg.RelPath}, "|"))
	}
	if !slices.Equal(actual, want) {
		t.Fatalf("discovery registry mismatch\n got: %v\nwant: %v", actual, want)
	}
}

func TestDiscoverPackagesFailsClosedOnSharedDuplicateIdentity(t *testing.T) {
	fixture := loadSharedDiscoveryFixture(t, "discovery-duplicate-identity.json")
	root := materializeSharedDiscoveryFixture(t, fixture)

	_, err := DiscoverPackages(filepath.Join(root, "code"))
	var duplicate *DuplicatePackageIdentityError
	if !errors.As(err, &duplicate) {
		t.Fatalf("error = %T %v, want *DuplicatePackageIdentityError", err, err)
	}
	diagnostic := fixture.Expected.Diagnostics[0]
	if duplicate.Code != diagnostic.Code {
		t.Fatalf("code = %q, want %q", duplicate.Code, diagnostic.Code)
	}
	if duplicate.Package != diagnostic.Package {
		t.Fatalf("package = %q, want %q", duplicate.Package, diagnostic.Package)
	}
	if !slices.Equal(duplicate.Paths, diagnostic.Details.Paths) {
		t.Fatalf("paths = %v, want %v", duplicate.Paths, diagnostic.Details.Paths)
	}
	if duplicate.Paths[0] != diagnostic.Path {
		t.Fatalf("primary path = %q, want %q", duplicate.Paths[0], diagnostic.Path)
	}
	wantMessage := diagnostic.Code + ": package=" + diagnostic.Package + " paths=" + strings.Join(diagnostic.Details.Paths, ",")
	if duplicate.Error() != wantMessage {
		t.Fatalf("message = %q, want %q", duplicate.Error(), wantMessage)
	}
	if strings.Contains(duplicate.Error(), root) {
		t.Fatalf("duplicate diagnostic leaked workspace root: %s", duplicate)
	}
}
