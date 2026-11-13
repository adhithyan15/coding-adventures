package ledger

import (
	"fmt"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// Stage 5a: the wanted set and the cached ids.

func TestLoadWantedIgnoresOtherKeysAndBoundsMemory(t *testing.T) {
	dir := manifestDirOf(t)
	var lines []string
	for i := 0; i < 60; i++ { // 3 weak keys x 20 distinct entry lists
		lines = append(lines, mfLine(t, mf(byte(1+i%3), fmt.Sprint(100+i), t0, fmt.Sprintf("path:f%d", i))))
	}
	writeManifestSeg(t, dir, 1, lines...)
	store := OpenManifests(filepath.Dir(dir), Shared)

	all, allRep, err := store.Load(nowT, nil)
	if err != nil {
		t.Fatal(err)
	}
	if allRep.Retained != 3*ManifestKeep || allRep.Ignored != 0 {
		t.Fatalf("no filter: %+v", allRep)
	}

	want := map[[32]byte]struct{}{key(2): {}}
	set, rep, err := store.Load(nowT, want)
	if err != nil {
		t.Fatal(err)
	}
	if rep.Read != 60 || rep.Ignored != 40 || rep.Retained != ManifestKeep || rep.Superseded != 20-ManifestKeep {
		t.Errorf("filtered report %+v", rep)
	}
	if len(set.byWeak) != 1 || set.Manifests(key(1)) != nil || set.Manifests(key(3)) != nil {
		t.Errorf("an unwanted weak key was retained: %d keys", len(set.byWeak))
	}
	// The wanted key keeps exactly what an unfiltered load would have kept.
	if !reflect.DeepEqual(set.Manifests(key(2)), all.Manifests(key(2))) {
		t.Error("filtering changed the retained manifests of a wanted key")
	}

	// A wanted set that matches nothing keeps nothing: empty is not "no filter".
	none, nrep, err := store.Load(nowT, map[[32]byte]struct{}{})
	if err != nil || len(none.byWeak) != 0 || nrep.Retained != 0 || nrep.Ignored != 60 {
		t.Errorf("empty wanted set: %v %+v", err, nrep)
	}
}

func TestLoadWantedStillJudgesStructureOfIgnoredRecords(t *testing.T) {
	dir := manifestDirOf(t)
	writeManifestSeg(t, dir, 1, mfLine(t, mf(1, "1", t0)),
		strings.Replace(mfLine(t, mf(2, "2", t0)), `"kind"`, `"extra":1,"kind"`, 1))
	store := OpenManifests(filepath.Dir(dir), Shared)
	if set, _, err := store.Load(nowT, map[[32]byte]struct{}{key(1): {}}); err == nil || set != nil {
		t.Fatalf("a corrupt record of an unwanted key must still make the store unusable: %v", err)
	}
}

func TestRetainedCarriesCachedIDs(t *testing.T) {
	dir := manifestDirOf(t)
	writeManifestSeg(t, dir, 1,
		mfLine(t, mf(1, "1", t0, "path:a")),
		mfLine(t, mf(1, "2", t0, "path:a")), // the same entries again: one slot
		mfLine(t, mf(1, "3", t0, "path:b")))
	set, _, err := load(t, dir, nowT)
	if err != nil {
		t.Fatal(err)
	}
	got := set.Retained(key(1))
	if len(got) != 2 || got[0].Manifest.Run != "3" || got[1].Manifest.Run != "2" {
		t.Fatalf("retained %+v", got)
	}
	for _, r := range got {
		if r.ID != r.Manifest.ID() {
			t.Errorf("cached id %x differs from the recomputed one", r.ID)
		}
	}
	got[0].Manifest.Entries[0] = "tampered"
	if set.Retained(key(1))[0].Manifest.Entries[0] != "path:b" {
		t.Error("Retained handed out the store's own entries slice")
	}
	if set.Retained(key(9)) != nil {
		t.Error("an unknown weak key has retained manifests")
	}
}
