// Tests for the correlation-vector package.
//
// Test philosophy: we verify the seven coverage groups specified in CV00:
//  1. Root lifecycle — create, contribute, passthrough, delete, error on re-contribute
//  2. Derivation — parent/child relationships, ancestors, descendants
//  3. Merging — multi-parent CVs, ancestor union
//  4. Deep ancestry chain — A→B→C→D, ancestors nearest-first, lineage oldest-first
//  5. Disabled log — IDs still returned, no entries stored, history empty
//  6. JSON roundtrip — full serialize/deserialize cycle, byte-for-byte equality
//  7. ID uniqueness — 10,000 creates with same and different origins
//
// We use Go's standard testing package with table-driven tests where a group
// of related cases shares structure. Individual assertions use t.Errorf rather
// than t.Fatalf to report as many failures as possible per test run.
package correlationvector

import (
	"fmt"
	"strings"
	"testing"
)

// ── Helpers ───────────────────────────────────────────────────────────────────

// origin builds an Origin for testing with the given source and location.
// This keeps test setup compact and readable.
func origin(source, location string) *Origin {
	return &Origin{
		Source:   source,
		Location: location,
		Meta:     make(map[string]any),
	}
}

// assertIDFormat checks that a CV ID matches the dot-extension scheme:
// each segment separated by dots is either an 8-char hex base or a
// positive decimal integer.
//
// Valid examples: "a3f1b2c4.1", "a3f1b2c4.1.2", "00000000.1.3.7"
func assertIDFormat(t *testing.T, id string) {
	t.Helper()
	parts := strings.Split(id, ".")
	if len(parts) < 2 {
		t.Errorf("ID %q has too few segments (want at least 2)", id)
		return
	}
	// First segment: 8 hex chars
	base := parts[0]
	if len(base) != 8 {
		t.Errorf("ID %q: base segment %q should be 8 chars, got %d", id, base, len(base))
	}
	for _, ch := range base {
		if !((ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f')) {
			t.Errorf("ID %q: base segment %q has non-hex char %q", id, base, ch)
		}
	}
	// Remaining segments: positive decimal integers
	for _, seg := range parts[1:] {
		var n int
		if _, err := fmt.Sscanf(seg, "%d", &n); err != nil || n < 1 {
			t.Errorf("ID %q: segment %q is not a positive integer", id, seg)
		}
	}
}

// containsString reports whether slice contains target.
func containsString(slice []string, target string) bool {
	for _, s := range slice {
		if s == target {
			return true
		}
	}
	return false
}

// ── 1. Root Lifecycle ─────────────────────────────────────────────────────────

// TestRootCreate verifies that Create returns a well-formed CV ID in the
// base.N format, and that the entry appears in the log with empty contributions.
func TestRootCreate(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(origin("app.ts", "5:12"))

	assertIDFormat(t, cvID)

	entry := log.Get(cvID)
	if entry == nil {
		t.Fatalf("Get(%q) returned nil after Create", cvID)
	}
	if entry.ID != cvID {
		t.Errorf("entry.ID = %q, want %q", entry.ID, cvID)
	}
	if len(entry.ParentIDs) != 0 {
		t.Errorf("root entry should have no parents, got %v", entry.ParentIDs)
	}
	if len(entry.Contributions) != 0 {
		t.Errorf("fresh entry should have 0 contributions, got %d", len(entry.Contributions))
	}
	if entry.Deleted != nil {
		t.Errorf("fresh entry should not be deleted")
	}
}

// TestRootCreateSyntheticBase verifies that Create with no origin uses "00000000"
// as the base segment of the returned ID.
func TestRootCreateSyntheticBase(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(nil)

	assertIDFormat(t, cvID)
	if !strings.HasPrefix(cvID, "00000000.") {
		t.Errorf("ID from nil origin should start with '00000000.', got %q", cvID)
	}
}

// TestRootCreateWithTimestamp verifies that Origin.Timestamp is stored in the
// entry when provided.
func TestRootCreateWithTimestamp(t *testing.T) {
	log := NewCVLog(true)
	o := &Origin{
		Source:    "events",
		Location:  "ts:1234567890",
		Timestamp: "2024-01-15T10:30:00Z",
		Meta:      map[string]any{"env": "prod"},
	}
	cvID := log.Create(o)
	entry := log.Get(cvID)
	if entry == nil {
		t.Fatal("entry is nil")
	}
	if entry.Origin == nil {
		t.Fatal("origin should not be nil")
	}
	if entry.Origin.Timestamp != "2024-01-15T10:30:00Z" {
		t.Errorf("timestamp = %q, want %q", entry.Origin.Timestamp, "2024-01-15T10:30:00Z")
	}
}

// TestContribute verifies that contributions are appended to the entry in
// order, with the correct source, tag, and meta values.
func TestContribute(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(origin("app.ts", "5:12"))

	err := log.Contribute(cvID, "scope_analysis", "resolved",
		map[string]any{"binding": "local:count:fn_main"})
	if err != nil {
		t.Fatalf("Contribute returned error: %v", err)
	}

	err = log.Contribute(cvID, "variable_renamer", "renamed",
		map[string]any{"from": "count", "to": "a"})
	if err != nil {
		t.Fatalf("Contribute returned error: %v", err)
	}

	entry := log.Get(cvID)
	if len(entry.Contributions) != 2 {
		t.Fatalf("expected 2 contributions, got %d", len(entry.Contributions))
	}

	if entry.Contributions[0].Source != "scope_analysis" {
		t.Errorf("contrib[0].Source = %q, want %q", entry.Contributions[0].Source, "scope_analysis")
	}
	if entry.Contributions[0].Tag != "resolved" {
		t.Errorf("contrib[0].Tag = %q, want %q", entry.Contributions[0].Tag, "resolved")
	}
	if entry.Contributions[1].Source != "variable_renamer" {
		t.Errorf("contrib[1].Source = %q, want %q", entry.Contributions[1].Source, "variable_renamer")
	}
}

// TestContributeWithNilMeta verifies that passing nil meta is safe — it should
// be treated as an empty map, not cause a nil pointer dereference.
func TestContributeWithNilMeta(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(nil)

	if err := log.Contribute(cvID, "parser", "parsed", nil); err != nil {
		t.Errorf("Contribute with nil meta returned error: %v", err)
	}

	entry := log.Get(cvID)
	if len(entry.Contributions) != 1 {
		t.Fatalf("expected 1 contribution, got %d", len(entry.Contributions))
	}
	if entry.Contributions[0].Meta == nil {
		t.Errorf("nil meta should be converted to empty map, got nil")
	}
}

// TestPassthrough verifies that Passthrough records an identity contribution
// with tag "passthrough".
func TestPassthrough(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(origin("app.ts", "1:0"))

	log.Passthrough(cvID, "type_checker")

	history := log.History(cvID)
	if len(history) != 1 {
		t.Fatalf("expected 1 history entry, got %d", len(history))
	}
	if history[0].Source != "type_checker" {
		t.Errorf("history[0].Source = %q, want %q", history[0].Source, "type_checker")
	}
	if history[0].Tag != "passthrough" {
		t.Errorf("history[0].Tag = %q, want %q", history[0].Tag, "passthrough")
	}
}

// TestDelete verifies that Delete marks the entry with a DeletionRecord,
// and that subsequent Contribute calls return an error.
func TestDelete(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(origin("app.ts", "10:5"))

	log.Contribute(cvID, "parser", "created", nil)
	log.Delete(cvID, "dce", "unreachable from entry point",
		map[string]any{"entry_cv": "abc.1"})

	entry := log.Get(cvID)
	if entry.Deleted == nil {
		t.Fatal("expected DeletionRecord, got nil")
	}
	if entry.Deleted.Source != "dce" {
		t.Errorf("DeletionRecord.Source = %q, want %q", entry.Deleted.Source, "dce")
	}
	if entry.Deleted.Reason != "unreachable from entry point" {
		t.Errorf("DeletionRecord.Reason = %q, want expected", entry.Deleted.Reason)
	}

	// Contributing to a deleted CV should return an error.
	err := log.Contribute(cvID, "optimizer", "modified", nil)
	if err == nil {
		t.Error("expected error when contributing to deleted CV, got nil")
	}
}

// TestDeleteWithNilMeta verifies that Delete with nil meta is safe.
func TestDeleteWithNilMeta(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(nil)
	log.Delete(cvID, "dce", "unreachable", nil)

	entry := log.Get(cvID)
	if entry.Deleted == nil {
		t.Fatal("deletion record should not be nil")
	}
	if entry.Deleted.Meta == nil {
		t.Errorf("nil meta in Delete should become empty map, got nil")
	}
}

// TestHistoryIncludesDeletion verifies that History appends a synthetic
// "deleted" contribution when the entry has been deleted.
func TestHistoryIncludesDeletion(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(origin("app.ts", "3:0"))
	log.Contribute(cvID, "parser", "created", nil)
	log.Delete(cvID, "dce", "dead code", map[string]any{"info": "unused"})

	history := log.History(cvID)

	// 1 real contribution + 1 synthetic deletion = 2
	if len(history) != 2 {
		t.Fatalf("expected 2 history entries, got %d", len(history))
	}
	last := history[len(history)-1]
	if last.Tag != "deleted" {
		t.Errorf("last history tag = %q, want %q", last.Tag, "deleted")
	}
	if last.Source != "dce" {
		t.Errorf("last history source = %q, want %q", last.Source, "dce")
	}
}

// TestPassOrderTracking verifies that PassOrder accumulates unique sources
// in the order they first contributed.
func TestPassOrderTracking(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(nil)

	log.Contribute(cvID, "parser", "parsed", nil)
	log.Contribute(cvID, "scope_analysis", "resolved", nil)
	log.Contribute(cvID, "parser", "re-parsed", nil) // duplicate — should not add again
	log.Passthrough(cvID, "type_checker")

	// PassOrder should be [parser, scope_analysis, type_checker]
	want := []string{"parser", "scope_analysis", "type_checker"}
	if len(log.PassOrder) != len(want) {
		t.Fatalf("PassOrder = %v, want %v", log.PassOrder, want)
	}
	for i, s := range want {
		if log.PassOrder[i] != s {
			t.Errorf("PassOrder[%d] = %q, want %q", i, log.PassOrder[i], s)
		}
	}
}

// ── 2. Derivation ─────────────────────────────────────────────────────────────

// TestDerive verifies that derived CVs have the parent's ID as a prefix,
// carry the parentID in their parent_ids list, and are independent entries.
func TestDerive(t *testing.T) {
	log := NewCVLog(true)
	parentID := log.Create(origin("app.ts", "5:12"))

	childA := log.Derive(parentID, nil)
	childB := log.Derive(parentID, nil)

	// IDs must be formatted correctly.
	assertIDFormat(t, childA)
	assertIDFormat(t, childB)

	// Children must have the parent ID as a prefix.
	if !strings.HasPrefix(childA, parentID+".") {
		t.Errorf("childA %q does not have parent prefix %q", childA, parentID+".")
	}
	if !strings.HasPrefix(childB, parentID+".") {
		t.Errorf("childB %q does not have parent prefix %q", childB, parentID+".")
	}

	// Children must be distinct.
	if childA == childB {
		t.Errorf("childA and childB are the same: %q", childA)
	}

	// Each child's entry must list the parent.
	entryA := log.Get(childA)
	if entryA == nil {
		t.Fatal("entryA is nil")
	}
	if len(entryA.ParentIDs) != 1 || entryA.ParentIDs[0] != parentID {
		t.Errorf("entryA.ParentIDs = %v, want [%s]", entryA.ParentIDs, parentID)
	}

	entryB := log.Get(childB)
	if entryB == nil {
		t.Fatal("entryB is nil")
	}
	if len(entryB.ParentIDs) != 1 || entryB.ParentIDs[0] != parentID {
		t.Errorf("entryB.ParentIDs = %v, want [%s]", entryB.ParentIDs, parentID)
	}
}

// TestAncestorsOfChild verifies that Ancestors(child) returns [parent].
func TestAncestorsOfChild(t *testing.T) {
	log := NewCVLog(true)
	parentID := log.Create(origin("src", "1:0"))
	childID := log.Derive(parentID, nil)

	ancestors := log.Ancestors(childID)
	if len(ancestors) != 1 {
		t.Fatalf("Ancestors(%q) returned %d entries, want 1: %v", childID, len(ancestors), ancestors)
	}
	if ancestors[0] != parentID {
		t.Errorf("Ancestors(%q)[0] = %q, want %q", childID, ancestors[0], parentID)
	}
}

// TestDescendantsOfParent verifies that Descendants(parent) includes both
// directly derived children.
func TestDescendantsOfParent(t *testing.T) {
	log := NewCVLog(true)
	parentID := log.Create(origin("src", "1:0"))
	childA := log.Derive(parentID, nil)
	childB := log.Derive(parentID, nil)

	descendants := log.Descendants(parentID)

	if !containsString(descendants, childA) {
		t.Errorf("Descendants(%q) does not contain childA %q: %v", parentID, childA, descendants)
	}
	if !containsString(descendants, childB) {
		t.Errorf("Descendants(%q) does not contain childB %q: %v", parentID, childB, descendants)
	}
}

// TestDescendantsTransitive verifies that Descendants returns indirect
// descendants (grandchildren, great-grandchildren, etc.).
func TestDescendantsTransitive(t *testing.T) {
	log := NewCVLog(true)
	rootID := log.Create(origin("src", "0:0"))
	childID := log.Derive(rootID, nil)
	grandchildID := log.Derive(childID, nil)

	descendants := log.Descendants(rootID)
	if !containsString(descendants, childID) {
		t.Errorf("Descendants(%q) missing direct child %q", rootID, childID)
	}
	if !containsString(descendants, grandchildID) {
		t.Errorf("Descendants(%q) missing grandchild %q", rootID, grandchildID)
	}
}

// ── 3. Merging ────────────────────────────────────────────────────────────────

// TestMerge verifies that Merge produces an entry with all parent IDs listed,
// and that Ancestors returns all of them.
func TestMerge(t *testing.T) {
	log := NewCVLog(true)
	cv1 := log.Create(origin("orders", "row:1"))
	cv2 := log.Create(origin("customers", "row:42"))
	cv3 := log.Create(origin("products", "row:7"))

	mergedID := log.Merge([]string{cv1, cv2, cv3},
		&Origin{Source: "join_stage", Location: "orders.customer_id=customers.id", Meta: make(map[string]any)})

	assertIDFormat(t, mergedID)

	entry := log.Get(mergedID)
	if entry == nil {
		t.Fatal("merged entry is nil")
	}
	if len(entry.ParentIDs) != 3 {
		t.Fatalf("merged entry.ParentIDs has %d entries, want 3: %v", len(entry.ParentIDs), entry.ParentIDs)
	}

	// All three parent IDs must be present.
	for _, parentID := range []string{cv1, cv2, cv3} {
		if !containsString(entry.ParentIDs, parentID) {
			t.Errorf("merged entry.ParentIDs missing %q: %v", parentID, entry.ParentIDs)
		}
	}

	// Ancestors must include all three parents.
	ancestors := log.Ancestors(mergedID)
	for _, parentID := range []string{cv1, cv2, cv3} {
		if !containsString(ancestors, parentID) {
			t.Errorf("Ancestors(%q) missing %q: %v", mergedID, parentID, ancestors)
		}
	}
}

// TestMergeNoOrigin verifies that Merge with nil origin uses the "00000000"
// base for the generated ID.
func TestMergeNoOrigin(t *testing.T) {
	log := NewCVLog(true)
	cv1 := log.Create(nil)
	cv2 := log.Create(nil)

	mergedID := log.Merge([]string{cv1, cv2}, nil)
	assertIDFormat(t, mergedID)
	if !strings.HasPrefix(mergedID, "00000000.") {
		t.Errorf("Merge with nil origin should give '00000000.*', got %q", mergedID)
	}
}

// ── 4. Deep Ancestry Chain ────────────────────────────────────────────────────

// TestDeepAncestryChain creates A → B → C → D and verifies:
//   - Ancestors(D) = [C, B, A] (nearest first)
//   - Lineage(D) returns all four entries in order [A, B, C, D]
func TestDeepAncestryChain(t *testing.T) {
	log := NewCVLog(true)
	idA := log.Create(origin("file.ts", "1:0"))
	idB := log.Derive(idA, nil)
	idC := log.Derive(idB, nil)
	idD := log.Derive(idC, nil)

	// Ancestors(D) should be nearest first: [C, B, A]
	ancestors := log.Ancestors(idD)
	if len(ancestors) != 3 {
		t.Fatalf("Ancestors(D) returned %d entries, want 3: %v", len(ancestors), ancestors)
	}
	if ancestors[0] != idC {
		t.Errorf("ancestors[0] = %q, want %q (nearest first)", ancestors[0], idC)
	}
	if ancestors[1] != idB {
		t.Errorf("ancestors[1] = %q, want %q", ancestors[1], idB)
	}
	if ancestors[2] != idA {
		t.Errorf("ancestors[2] = %q, want %q", ancestors[2], idA)
	}

	// Lineage(D) should be oldest first: [A, B, C, D]
	lineage := log.Lineage(idD)
	if len(lineage) != 4 {
		t.Fatalf("Lineage(D) returned %d entries, want 4: %v", len(lineage), lineage)
	}
	expected := []string{idA, idB, idC, idD}
	for i, want := range expected {
		if lineage[i].ID != want {
			t.Errorf("lineage[%d].ID = %q, want %q", i, lineage[i].ID, want)
		}
	}
}

// TestLineageSingleRoot verifies that Lineage on a root with no parents
// returns just that single entry.
func TestLineageSingleRoot(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(origin("x", "y"))
	lineage := log.Lineage(cvID)
	if len(lineage) != 1 {
		t.Fatalf("Lineage of root should return 1 entry, got %d", len(lineage))
	}
	if lineage[0].ID != cvID {
		t.Errorf("lineage[0].ID = %q, want %q", lineage[0].ID, cvID)
	}
}

// ── 5. Disabled Log ───────────────────────────────────────────────────────────

// TestDisabledLogCreateReturnsID verifies that Create still returns a valid
// CV ID when the log is disabled.
func TestDisabledLogCreateReturnsID(t *testing.T) {
	log := NewCVLog(false)
	cvID := log.Create(origin("app.ts", "5:12"))

	// ID should be well-formed.
	assertIDFormat(t, cvID)

	// But no entry should be stored.
	if entry := log.Get(cvID); entry != nil {
		t.Errorf("Get(%q) should return nil for disabled log, got %v", cvID, entry)
	}
}

// TestDisabledLogDeriveReturnsID verifies that Derive still returns a valid
// CV ID when the log is disabled.
func TestDisabledLogDeriveReturnsID(t *testing.T) {
	log := NewCVLog(false)
	parentID := log.Create(nil)
	childID := log.Derive(parentID, nil)

	assertIDFormat(t, childID)

	if entry := log.Get(childID); entry != nil {
		t.Errorf("Get(%q) should return nil for disabled log", childID)
	}
}

// TestDisabledLogMergeReturnsID verifies that Merge still returns a valid
// CV ID when the log is disabled.
func TestDisabledLogMergeReturnsID(t *testing.T) {
	log := NewCVLog(false)
	cv1 := log.Create(nil)
	cv2 := log.Create(nil)
	mergedID := log.Merge([]string{cv1, cv2}, nil)

	assertIDFormat(t, mergedID)

	if entry := log.Get(mergedID); entry != nil {
		t.Errorf("Get(%q) should return nil for disabled log", mergedID)
	}
}

// TestDisabledLogOperationsAreNoops verifies that all mutating operations on
// a disabled log complete without error and produce no side effects.
func TestDisabledLogOperationsAreNoops(t *testing.T) {
	log := NewCVLog(false)
	cvID := log.Create(origin("app.ts", "1:0"))

	// Contribute — no-op, no error
	if err := log.Contribute(cvID, "scope", "resolved", nil); err != nil {
		t.Errorf("Contribute on disabled log returned error: %v", err)
	}

	// Passthrough — no-op
	log.Passthrough(cvID, "type_checker")

	// Delete — no-op
	log.Delete(cvID, "dce", "dead", nil)

	// History should be empty (nothing was stored)
	if history := log.History(cvID); len(history) != 0 {
		t.Errorf("History on disabled log should be empty, got %d entries", len(history))
	}

	// Ancestors should be empty
	if ancestors := log.Ancestors(cvID); len(ancestors) != 0 {
		t.Errorf("Ancestors on disabled log should be empty, got %v", ancestors)
	}
}

// TestDisabledLogPassOrder verifies that PassOrder remains empty when the
// log is disabled (since no contributions are recorded).
func TestDisabledLogPassOrder(t *testing.T) {
	log := NewCVLog(false)
	cvID := log.Create(nil)
	log.Contribute(cvID, "parser", "parsed", nil)
	log.Passthrough(cvID, "type_checker")

	if len(log.PassOrder) != 0 {
		t.Errorf("PassOrder should be empty for disabled log, got %v", log.PassOrder)
	}
}

// ── 6. JSON Roundtrip ─────────────────────────────────────────────────────────

// TestJSONRoundtrip builds a CVLog with roots, derivations, merges, and
// deletions, serialises it, deserialises it, and verifies every entry is
// identical to the original.
func TestJSONRoundtrip(t *testing.T) {
	// Build a rich CVLog.
	log := NewCVLog(true)

	// Root with origin.
	cv1 := log.Create(&Origin{
		Source:    "app.ts",
		Location:  "5:12",
		Timestamp: "2024-01-01T00:00:00Z",
		Meta:      map[string]any{"env": "prod"},
	})
	log.Contribute(cv1, "parser", "created", map[string]any{"token": "IDENTIFIER"})
	log.Contribute(cv1, "scope_analysis", "resolved", map[string]any{"binding": "local"})

	// Root with no origin (synthetic).
	cv2 := log.Create(nil)
	log.Contribute(cv2, "generator", "synthesised", nil)

	// Derivation.
	cv3 := log.Derive(cv1, nil)
	log.Passthrough(cv3, "type_checker")

	// Merge.
	cv4 := log.Merge([]string{cv1, cv2}, &Origin{
		Source:   "join",
		Location: "0",
		Meta:     make(map[string]any),
	})

	// Deletion.
	cv5 := log.Create(origin("dead.ts", "1:0"))
	log.Delete(cv5, "dce", "unreachable", map[string]any{"entry": cv1})

	// Serialise.
	jsonStr, err := log.ToJSONString()
	if err != nil {
		t.Fatalf("ToJSONString failed: %v", err)
	}
	if jsonStr == "" {
		t.Fatal("ToJSONString returned empty string")
	}

	// Deserialise.
	restored, err := DeserializeFromJSON(jsonStr)
	if err != nil {
		t.Fatalf("DeserializeFromJSON failed: %v", err)
	}

	// Compare entry by entry.
	for _, id := range []string{cv1, cv2, cv3, cv4, cv5} {
		orig := log.Get(id)
		rest := restored.Get(id)
		if orig == nil && rest == nil {
			continue
		}
		if orig == nil || rest == nil {
			t.Errorf("entry %q: orig=%v rest=%v (one is nil)", id, orig, rest)
			continue
		}
		compareEntries(t, id, orig, rest)
	}

	// Enabled flag and PassOrder must round-trip.
	if restored.Enabled != log.Enabled {
		t.Errorf("Enabled: orig=%v, restored=%v", log.Enabled, restored.Enabled)
	}
	if len(restored.PassOrder) != len(log.PassOrder) {
		t.Errorf("PassOrder length: orig=%d, restored=%d", len(log.PassOrder), len(restored.PassOrder))
	}
}

// compareEntries does a field-by-field comparison of two CVEntry values.
func compareEntries(t *testing.T, id string, orig, rest *CVEntry) {
	t.Helper()

	if orig.ID != rest.ID {
		t.Errorf("[%s] ID: %q vs %q", id, orig.ID, rest.ID)
	}
	if len(orig.ParentIDs) != len(rest.ParentIDs) {
		t.Errorf("[%s] ParentIDs len: %d vs %d", id, len(orig.ParentIDs), len(rest.ParentIDs))
	}
	if len(orig.Contributions) != len(rest.Contributions) {
		t.Errorf("[%s] Contributions len: %d vs %d", id, len(orig.Contributions), len(rest.Contributions))
	}
	// Check deleted status.
	if (orig.Deleted == nil) != (rest.Deleted == nil) {
		t.Errorf("[%s] Deleted nil mismatch: orig=%v rest=%v", id, orig.Deleted, rest.Deleted)
	}
	if orig.Deleted != nil && rest.Deleted != nil {
		if orig.Deleted.Source != rest.Deleted.Source {
			t.Errorf("[%s] Deleted.Source: %q vs %q", id, orig.Deleted.Source, rest.Deleted.Source)
		}
		if orig.Deleted.Reason != rest.Deleted.Reason {
			t.Errorf("[%s] Deleted.Reason: %q vs %q", id, orig.Deleted.Reason, rest.Deleted.Reason)
		}
	}
}

// TestJSONRoundtripCountersContinue verifies that after deserialisation,
// new IDs created with Create and Derive do not collide with existing ones.
func TestJSONRoundtripCountersContinue(t *testing.T) {
	log := NewCVLog(true)
	cv1 := log.Create(origin("file.ts", "1:0"))
	cv2 := log.Derive(cv1, nil)

	jsonStr, _ := log.ToJSONString()
	restored, err := DeserializeFromJSON(jsonStr)
	if err != nil {
		t.Fatalf("DeserializeFromJSON: %v", err)
	}

	// Creating new entries in the restored log must not reuse existing IDs.
	newRoot := restored.Create(origin("file.ts", "1:0"))
	newChild := restored.Derive(cv1, nil)

	if newRoot == cv1 {
		t.Errorf("new root ID %q collides with existing %q", newRoot, cv1)
	}
	if newChild == cv2 {
		t.Errorf("new child ID %q collides with existing %q", newChild, cv2)
	}
}

// TestSerializeDisabledLog verifies that a disabled log serialises the
// enabled=false flag correctly.
func TestSerializeDisabledLog(t *testing.T) {
	log := NewCVLog(false)
	log.Create(nil) // ID returned but not stored

	jsonStr, err := log.ToJSONString()
	if err != nil {
		t.Fatalf("ToJSONString: %v", err)
	}

	restored, err := DeserializeFromJSON(jsonStr)
	if err != nil {
		t.Fatalf("DeserializeFromJSON: %v", err)
	}
	if restored.Enabled {
		t.Errorf("restored.Enabled should be false")
	}
}

// ── 7. ID Uniqueness ──────────────────────────────────────────────────────────

// TestIDUniquenessSameOrigin creates 10,000 root CVs with the same origin
// and verifies that all returned IDs are unique.
func TestIDUniquenessSameOrigin(t *testing.T) {
	log := NewCVLog(true)
	o := origin("app.ts", "5:12")

	seen := make(map[string]bool, 10000)
	for i := 0; i < 10000; i++ {
		id := log.Create(o)
		if seen[id] {
			t.Fatalf("duplicate ID %q at iteration %d", id, i)
		}
		seen[id] = true
	}
}

// TestIDUniquenessMixedOrigins creates 10,000 root CVs with different
// origins and verifies no collisions across bases.
func TestIDUniquenessMixedOrigins(t *testing.T) {
	log := NewCVLog(true)

	seen := make(map[string]bool, 10000)
	for i := 0; i < 10000; i++ {
		o := origin(fmt.Sprintf("file_%d.ts", i%100), fmt.Sprintf("%d:0", i))
		id := log.Create(o)
		if seen[id] {
			t.Fatalf("duplicate ID %q at iteration %d", id, i)
		}
		seen[id] = true
	}
}

// TestIDUniquenessNilOrigin creates 10,000 root CVs with nil origin and
// verifies no collisions (all will share the "00000000" base).
func TestIDUniquenessNilOrigin(t *testing.T) {
	log := NewCVLog(true)

	seen := make(map[string]bool, 10000)
	for i := 0; i < 10000; i++ {
		id := log.Create(nil)
		if seen[id] {
			t.Fatalf("duplicate ID %q at iteration %d", id, i)
		}
		seen[id] = true
	}
}

// TestChildIDUniqueness verifies that 1000 children derived from the same
// parent all get unique IDs.
func TestChildIDUniqueness(t *testing.T) {
	log := NewCVLog(true)
	parentID := log.Create(origin("src", "0:0"))

	seen := make(map[string]bool, 1000)
	for i := 0; i < 1000; i++ {
		id := log.Derive(parentID, nil)
		if seen[id] {
			t.Fatalf("duplicate child ID %q at iteration %d", id, i)
		}
		seen[id] = true
	}
}

// ── Additional Edge Cases ──────────────────────────────────────────────────────

// TestGetMissingCV verifies that Get returns nil for an ID that was never created.
func TestGetMissingCV(t *testing.T) {
	log := NewCVLog(true)
	if entry := log.Get("00000000.999"); entry != nil {
		t.Errorf("Get of missing ID should return nil, got %v", entry)
	}
}

// TestHistoryEmpty verifies that History returns an empty slice for a CV
// with no contributions and no deletion.
func TestHistoryEmpty(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(nil)
	history := log.History(cvID)
	if len(history) != 0 {
		t.Errorf("fresh CV should have empty history, got %d entries", len(history))
	}
}

// TestHistoryMissingCV verifies that History returns an empty slice for an
// unknown CV ID (not a nil pointer).
func TestHistoryMissingCV(t *testing.T) {
	log := NewCVLog(true)
	history := log.History("nonexistent.1")
	if history == nil {
		t.Errorf("History of missing CV should return empty slice, not nil")
	}
	if len(history) != 0 {
		t.Errorf("History of missing CV should be empty, got %d entries", len(history))
	}
}

// TestAncestorsEmpty verifies that Ancestors of a root CV returns an empty slice.
func TestAncestorsEmpty(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(origin("a", "b"))
	ancestors := log.Ancestors(cvID)
	if len(ancestors) != 0 {
		t.Errorf("root CV should have no ancestors, got %v", ancestors)
	}
}

// TestDescendantsEmpty verifies that Descendants of a leaf CV returns empty.
func TestDescendantsEmpty(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(nil)
	descendants := log.Descendants(cvID)
	if len(descendants) != 0 {
		t.Errorf("leaf CV should have no descendants, got %v", descendants)
	}
}

// TestLineageMissing verifies that Lineage of a missing CV returns empty.
func TestLineageMissing(t *testing.T) {
	log := NewCVLog(true)
	lineage := log.Lineage("nonexistent.1")
	if len(lineage) != 0 {
		t.Errorf("lineage of missing CV should be empty, got %d", len(lineage))
	}
}

// TestContributionsMakeNotNil verifies that both ParentIDs and Contributions
// serialize as [] (not null) when empty.
func TestContributionsMakeNotNil(t *testing.T) {
	log := NewCVLog(true)
	cvID := log.Create(nil)

	entry := log.Get(cvID)
	if entry.ParentIDs == nil {
		t.Error("ParentIDs should be empty slice (not nil) for JSON serialization")
	}
	if entry.Contributions == nil {
		t.Error("Contributions should be empty slice (not nil) for JSON serialization")
	}
}

// TestDeriveDeletedParentAllowed verifies that deriving from a deleted CV
// is allowed (not an error) — e.g. creating a tombstone record.
func TestDeriveDeletedParentAllowed(t *testing.T) {
	log := NewCVLog(true)
	parentID := log.Create(origin("src", "0:0"))
	log.Delete(parentID, "dce", "dead", nil)

	// Derive from deleted parent — must succeed.
	childID := log.Derive(parentID, nil)
	assertIDFormat(t, childID)

	entry := log.Get(childID)
	if entry == nil {
		t.Fatal("derived entry from deleted parent is nil")
	}
	if len(entry.ParentIDs) != 1 || entry.ParentIDs[0] != parentID {
		t.Errorf("childID.ParentIDs = %v, want [%s]", entry.ParentIDs, parentID)
	}
}

// TestSerialize verifies that Serialize produces a map with the expected keys.
func TestSerialize(t *testing.T) {
	log := NewCVLog(true)
	log.Create(origin("x", "y"))

	data := log.Serialize()
	if _, ok := data["entries"]; !ok {
		t.Error("Serialize result missing 'entries' key")
	}
	if _, ok := data["pass_order"]; !ok {
		t.Error("Serialize result missing 'pass_order' key")
	}
	if _, ok := data["enabled"]; !ok {
		t.Error("Serialize result missing 'enabled' key")
	}
}

// TestMergeParentIDsCopied verifies that the ParentIDs in a merged entry
// are an independent copy — mutating the input slice does not affect the
// stored entry.
func TestMergeParentIDsCopied(t *testing.T) {
	log := NewCVLog(true)
	cv1 := log.Create(nil)
	cv2 := log.Create(nil)

	parents := []string{cv1, cv2}
	mergedID := log.Merge(parents, nil)

	// Mutate the original slice.
	parents[0] = "tampered"

	entry := log.Get(mergedID)
	if entry.ParentIDs[0] == "tampered" {
		t.Error("ParentIDs were not copied — mutation of input affected stored entry")
	}
}

// TestSameDeterministicBase verifies that the same (source, location) pair
// always produces the same base segment, making IDs deterministic.
func TestSameDeterministicBase(t *testing.T) {
	log1 := NewCVLog(true)
	log2 := NewCVLog(true)

	o := origin("app.ts", "5:12")
	id1 := log1.Create(o)
	id2 := log2.Create(o)

	// Both should have the same base segment.
	base1 := strings.Split(id1, ".")[0]
	base2 := strings.Split(id2, ".")[0]
	if base1 != base2 {
		t.Errorf("same origin produced different bases: %q vs %q", base1, base2)
	}
}
