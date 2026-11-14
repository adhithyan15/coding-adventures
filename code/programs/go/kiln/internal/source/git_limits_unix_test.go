//go:build unix

package source

// The deadline and the output cap of every git call (KILN04 4.8: the recorder
// runs git over a checkout a BUILD has just had its way with).

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func fakeGit(t *testing.T, body string) *Git {
	t.Helper()
	d := t.TempDir()
	bin := filepath.Join(d, "git")
	if err := os.WriteFile(bin, []byte("#!/bin/sh\n"+body+"\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	return NewGitAt(d, bin, []string{"HOME=" + d, "PATH=/usr/bin:/bin"})
}

func TestGitCallTimesOut(t *testing.T) {
	g := fakeGit(t, "sleep 30")
	g.SetTimeout(300 * time.Millisecond)
	start := time.Now()
	_, err := g.Output("fsck")
	if err == nil || !strings.Contains(err.Error(), "timed out") {
		t.Fatalf("%v", err)
	}
	if time.Since(start) > 15*time.Second {
		t.Fatalf("the deadline did not stop git: %v", time.Since(start))
	}
}

// A grandchild that keeps the pipes open does not hold the call past its deadline.
func TestGitCallTimeoutWithAGrandchildHoldingThePipes(t *testing.T) {
	g := fakeGit(t, "sleep 30 &\nsleep 30")
	g.SetTimeout(300 * time.Millisecond)
	start := time.Now()
	if _, err := g.Output("fsck"); err == nil {
		t.Fatal("no error")
	}
	if time.Since(start) > 20*time.Second {
		t.Fatalf("%v", time.Since(start))
	}
}

func TestGitCallKeepsItsOutputWhole(t *testing.T) {
	g := fakeGit(t, "printf 'one\\ntwo\\n'")
	out, err := g.Output("ls-files")
	if err != nil || string(out) != "one\ntwo\n" {
		t.Fatalf("%q %v", out, err)
	}
}

func TestGitFailureCarriesBoundedStderr(t *testing.T) {
	g := fakeGit(t, "head -c 3000000 /dev/zero | tr '\\0' x >&2; exit 3")
	_, err := g.Output("fsck")
	if err == nil || !strings.Contains(err.Error(), "[output truncated]") || len(err.Error()) > 1<<21 {
		n := 0
		if err != nil {
			n = len(err.Error())
		}
		t.Fatalf("%d bytes: %v", n, err != nil)
	}
}

func TestLimitedBuffer(t *testing.T) {
	strict := &limitedBuffer{max: 4, strict: true}
	if n, err := strict.Write([]byte("abcd")); n != 4 || err != nil {
		t.Fatal(n, err)
	}
	if _, err := strict.Write([]byte("e")); err == nil {
		t.Fatal("a strict buffer took a byte over its cap")
	}
	if strict.String() != "abcd" {
		t.Fatalf("%q", strict.String())
	}
	soft := &limitedBuffer{max: 4}
	if n, err := soft.Write([]byte("abcdef")); n != 6 || err != nil {
		t.Fatal(n, err)
	}
	if soft.String() != "abcd [output truncated]" {
		t.Fatalf("%q", soft.String())
	}
}
