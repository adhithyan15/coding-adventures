//go:build unix

package record

import (
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
)

func TestInventoryListsFifosAndDoesNotOpenThem(t *testing.T) {
	dir := newRepo(t, baseFiles)
	if err := syscall.Mkfifo(filepath.Join(dir, "src", "pipe"), 0o644); err != nil {
		t.Skipf("cannot create a FIFO: %v", err)
	}
	if got := inventoryOf(t, dir); len(got) != 1 || got[0] != "src/pipe" {
		t.Fatalf("got %v", got)
	}
}

func TestInventoryUnreadableDirectoryIsAnError(t *testing.T) {
	if os.Getuid() == 0 {
		t.Skip("root can read everything")
	}
	dir := newRepo(t, baseFiles)
	locked := filepath.Join(dir, "src")
	if err := os.Chmod(locked, 0); err != nil {
		t.Fatal(err)
	}
	defer os.Chmod(locked, 0o755)
	if _, err := Inventory(dir); err == nil {
		t.Fatal("an unreadable tracked directory must fail the inventory")
	}
}

func TestInventoryRefusesANameTheFormatsRefuse(t *testing.T) {
	dir := newRepo(t, baseFiles)
	// Not valid UTF-8: the JSON record would carry U+FFFD instead of the
	// real name, so the merge could not match it.
	bad := filepath.Join(dir, "bad\xffname")
	if err := os.WriteFile(bad, []byte("x"), 0o644); err != nil {
		t.Skipf("this filesystem refuses invalid UTF-8 names: %v", err)
	}
	_, err := Inventory(dir)
	if err == nil || !strings.Contains(err.Error(), "cannot be recorded") {
		t.Fatalf("got %v", err)
	}
}
