package main

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

// #16929: with the starting snapshot taken, an unchanged tree reports no
// change, so the watcher no longer reloads every browser at startup. An edit
// and a deletion are each still reported, once.
func TestPollFilesReportsChangesOnlyAfterTheStartingSnapshot(t *testing.T) {
	root := t.TempDir()
	layout := filepath.Join(root, "Card.mll")
	mustWrite(t, layout, "Box [ card ] ( )")
	mustWrite(t, filepath.Join(root, "notes.txt"), "not watched")
	server := &Server{root: root}

	mtimes := make(map[string]time.Time)
	server.pollFiles(mtimes) // the starting snapshot
	if server.pollFiles(mtimes) {
		t.Fatal("an unchanged tree reported a change after the starting snapshot")
	}

	later := time.Now().Add(2 * time.Second)
	if err := os.Chtimes(layout, later, later); err != nil {
		t.Fatalf("chtimes: %v", err)
	}
	if !server.pollFiles(mtimes) {
		t.Fatal("an edited .mll was not reported")
	}
	if server.pollFiles(mtimes) {
		t.Fatal("the same edit was reported twice")
	}

	if err := os.Remove(layout); err != nil {
		t.Fatalf("remove: %v", err)
	}
	if !server.pollFiles(mtimes) {
		t.Fatal("a deleted .mll was not reported")
	}
}
