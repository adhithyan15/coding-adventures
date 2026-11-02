package main

import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"testing"
)

const haskellITFWindowsFront = `powershell -NoProfile -Command "if (Get-Command cabal -ErrorAction SilentlyContinue) { & cabal test all --builddir=dist; exit $LASTEXITCODE } else { Write-Output 'cabal not found -- skipping'; exit 0 }"`

func haskellITFFrontPaths(t *testing.T) []string {
	t.Helper()
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("locate test source")
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", ".."))
	return []string{
		filepath.Join(repoRoot, "code", "packages", "haskell", "barcode-layout-1d", "BUILD_windows"),
		filepath.Join(repoRoot, "code", "packages", "haskell", "itf", "BUILD_windows"),
	}
}

func TestHaskellITFWindowsFrontIsOneFailClosedInvocation(t *testing.T) {
	for _, path := range haskellITFFrontPaths(t) {
		data, err := os.ReadFile(path)
		if err != nil {
			t.Fatal(err)
		}
		if got := strings.TrimSpace(string(data)); got != haskellITFWindowsFront {
			t.Fatalf("%s must be the reviewed single-process Windows front\ngot:  %s\nwant: %s", path, got, haskellITFWindowsFront)
		}
	}
}

func TestHaskellITFWindowsFrontExitContract(t *testing.T) {
	if runtime.GOOS != "windows" {
		t.Skip("PowerShell and cabal.cmd exit propagation are exercised on Windows CI")
	}
	powershell, err := exec.LookPath("powershell")
	if err != nil {
		t.Fatal(err)
	}
	script := strings.TrimSuffix(strings.TrimPrefix(haskellITFWindowsFront, `powershell -NoProfile -Command "`), `"`)
	toolDir := t.TempDir()

	run := func(t *testing.T, want int) {
		t.Helper()
		cmd := exec.Command(powershell, "-NoProfile", "-Command", script)
		cmd.Env = []string{"PATH=" + toolDir}
		for _, item := range os.Environ() {
			name, _, _ := strings.Cut(item, "=")
			if !strings.EqualFold(name, "PATH") {
				cmd.Env = append(cmd.Env, item)
			}
		}
		err := cmd.Run()
		got := 0
		if err != nil {
			var exitErr *exec.ExitError
			if !errors.As(err, &exitErr) {
				t.Fatal(err)
			}
			got = exitErr.ExitCode()
		}
		if got != want {
			t.Fatalf("exit code %d, want %d", got, want)
		}
	}

	t.Run("absent tool skips successfully", func(t *testing.T) { run(t, 0) })
	for _, tc := range []struct {
		name string
		code int
	}{
		{name: "successful tool", code: 0},
		{name: "failing tool", code: 23},
	} {
		t.Run(tc.name, func(t *testing.T) {
			content := []byte("@exit /b " + strconv.Itoa(tc.code) + "\r\n")
			if err := os.WriteFile(filepath.Join(toolDir, "cabal.cmd"), content, 0o755); err != nil {
				t.Fatal(err)
			}
			run(t, tc.code)
		})
	}
}
