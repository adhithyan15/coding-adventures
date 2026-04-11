// Package correlationvector provides lightweight, append-only provenance
// tracking for any data pipeline.
//
// # What Is a Correlation Vector?
//
// A Correlation Vector (CV) is a stable identifier assigned to a piece of
// data at birth. Every system, stage, or function that processes the data
// appends a Contribution to the CV log. At any point you can ask: "where
// did this data come from, and what happened to it?" — and get a complete,
// ordered answer.
//
// The concept originated in distributed systems tracing, where a request
// flows through dozens of microservices and you need to reconstruct what
// happened across all of them. This implementation generalises the idea to
// any pipeline: compilers, ETL, build systems, ML preprocessing, document
// transformations, and anywhere that data flows through a sequence of steps.
//
// This is a generic, domain-agnostic library. It knows nothing about
// compilers, databases, or networks. Consumers attach semantic meaning via
// the source and tag fields of Contributions, and via arbitrary metadata.
//
// # Core Concepts
//
// Every tracked entity is assigned a CV ID (a string like "a3f1b2c4.1") at
// birth. The ID never changes — even if the entity is renamed, transformed,
// or deleted — so you can always pull the thread and reconstruct history.
//
// CV IDs use a dot-extension scheme:
//
//	base.N       — root entity, born without a parent
//	base.N.M     — derived from base.N
//	base.N.M.K   — derived from base.N.M
//
// The base is an 8-character hex string derived from a SHA-256 of the
// entity's origin (source + location). For synthetic entities with no
// natural origin, the base is "00000000".
//
// # Enabled Flag
//
// The CVLog carries an Enabled flag. When false, all mutating operations
// (Contribute, Derive, Merge, Delete, Passthrough) are no-ops — they return
// immediately without allocating or writing anything. Create and Derive still
// generate and return IDs (callers need those IDs to tag their data
// structures), but no entries are stored in the log. This gives production
// code essentially zero overhead when tracing is off.
//
// # Example: Compiler Pipeline
//
//	log := NewCVLog(true)
//	cvID := log.Create(&Origin{Source: "app.ts", Location: "5:12"})
//	log.Contribute(cvID, "scope_analysis", "resolved",
//	    map[string]any{"binding": "local:count:fn_main"})
//	log.Contribute(cvID, "variable_renamer", "renamed",
//	    map[string]any{"from": "count", "to": "a"})
//	log.Delete(cvID, "dead_code_eliminator", "unreachable from entry", nil)
//
// # Example: ETL Pipeline
//
//	log := NewCVLog(true)
//	cvID := log.Create(&Origin{Source: "orders_table", Location: "row_id:8472"})
//	log.Contribute(cvID, "date_normalizer", "converted",
//	    map[string]any{"from_format": "MM/DD/YYYY", "to_format": "ISO8601"})
//	log.Passthrough(cvID, "deduplicator")
//
// # Serialization
//
// The CVLog can be serialised to a JSON string and reconstructed:
//
//	jsonStr, _ := log.ToJSONString()
//	restored, _ := DeserializeFromJSON(jsonStr)
package correlationvector

import (
	"encoding/json"
	"fmt"
	"strings"

	sha256 "github.com/adhithyan15/coding-adventures/code/packages/go/sha256"
	jsonserializer "github.com/coding-adventures/json-serializer"
	jsonvalue "github.com/coding-adventures/json-value"
)

// ── Public Types ─────────────────────────────────────────────────────────────

// Origin describes where and when an entity was born.
//
// Source identifies the system, file, or table that produced the entity
// (e.g., "app.ts", "orders_table", "stdin"). Location gives a position
// within that source (e.g., "5:12" for line 5 col 12, "row_id:8472" for a
// database primary key). Timestamp is optional — use it when time of birth
// is meaningful (ISO 8601 format). Meta holds any additional context the
// consumer wants to attach at creation time.
type Origin struct {
	Source    string         `json:"source"`
	Location  string         `json:"location"`
	Timestamp string         `json:"timestamp,omitempty"`
	Meta      map[string]any `json:"meta"`
}

// Contribution records that a stage processed an entity.
//
// Source identifies who or what acted (stage name, service name, pass name).
// Tag classifies what happened (a domain-defined label — "resolved",
// "renamed", "converted", "compiled", etc.). Meta holds arbitrary key-value
// detail, also domain-defined.
//
// Contributions are appended in call order. The ordering is semantically
// meaningful: it is the sequence in which stages processed the entity.
type Contribution struct {
	Source string         `json:"source"`
	Tag    string         `json:"tag"`
	Meta   map[string]any `json:"meta"`
}

// DeletionRecord captures the intentional removal of an entity.
//
// Entities are never truly erased from the log — their CVEntry remains
// forever with this non-nil DeletionRecord, so you can always answer "why
// did this disappear?" long after the fact. Source is who deleted it, Reason
// is why, Meta is any additional context.
type DeletionRecord struct {
	Source string         `json:"source"`
	Reason string         `json:"reason"`
	Meta   map[string]any `json:"meta"`
}

// CVEntry is the full provenance record for a single entity.
//
//   - ID: the stable, globally unique CV identifier (never changes)
//   - ParentIDs: empty for root entities; one or more for derived/merged ones
//   - Origin: where/when the entity was born (nil for synthetic entities)
//   - Contributions: append-only ordered history of every stage that touched it
//   - Deleted: non-nil if the entity was intentionally removed
type CVEntry struct {
	ID            string          `json:"id"`
	ParentIDs     []string        `json:"parent_ids"`
	Origin        *Origin         `json:"origin"`
	Contributions []Contribution  `json:"contributions"`
	Deleted       *DeletionRecord `json:"deleted"`
}

// CVLog is the central data structure that holds all CV entries for a
// pipeline run. It travels alongside the data being processed, accumulating
// the provenance history of every entity.
//
//   - Entries: the map from CV ID to CVEntry
//   - PassOrder: ordered list of sources (stage names) that have contributed
//   - Enabled: the tracing switch — when false, all writes are no-ops
//
// The unexported baseCounters and childCounters implement the dot-extension
// ID scheme: baseCounters[base] gives the next N for base.N IDs, and
// childCounters[parentID] gives the next M for parentID.M IDs.
type CVLog struct {
	Entries       map[string]*CVEntry `json:"entries"`
	PassOrder     []string            `json:"pass_order"`
	Enabled       bool                `json:"enabled"`
	baseCounters  map[string]int      // unexported: base → next sequence number
	childCounters map[string]int      // unexported: parentID → next child sequence number
}

// ── Construction ──────────────────────────────────────────────────────────────

// NewCVLog creates a fresh, empty CVLog.
//
// Pass enabled=true to activate full provenance tracking. Pass enabled=false
// for production use where you want zero overhead — Create and Derive still
// return IDs (callers need them), but nothing is stored in the log.
//
// Example:
//
//	log := NewCVLog(true)   // full tracing
//	log := NewCVLog(false)  // production mode, near-zero overhead
func NewCVLog(enabled bool) *CVLog {
	return &CVLog{
		Entries:       make(map[string]*CVEntry),
		PassOrder:     make([]string, 0),
		Enabled:       enabled,
		baseCounters:  make(map[string]int),
		childCounters: make(map[string]int),
	}
}

// ── ID Generation ─────────────────────────────────────────────────────────────

// originBase computes an 8-character hex base from an Origin.
//
// The base is the first 4 bytes of the SHA-256 of "source:location", formatted
// as an 8-character lowercase hex string. If origin is nil, we return the
// synthetic base "00000000".
//
// Using SHA-256 rather than a simpler hash provides collision resistance:
// two different (source, location) pairs will almost never produce the same
// base, even for large pipelines with thousands of entities.
//
// We only use 4 bytes (32 bits) of the hash. This gives 4 billion possible
// bases — more than enough for any realistic pipeline, while keeping the
// CV IDs short and readable.
func originBase(origin *Origin) string {
	if origin == nil {
		return "00000000"
	}
	// Combine source and location with a separator that is unlikely to appear
	// in either field, ensuring "a:bc" != "ab:c".
	input := origin.Source + ":" + origin.Location
	sum := sha256.Sum256([]byte(input))
	return fmt.Sprintf("%08x", sum[:4])
}

// nextRootID allocates the next root ID for the given base.
//
// The scheme is base.N where N starts at 1 and increments per base.
// This means the first entity born from "app.ts:5:12" gets "a3f1b2c4.1",
// the second gets "a3f1b2c4.2", and so on.
func (l *CVLog) nextRootID(base string) string {
	l.baseCounters[base]++
	return fmt.Sprintf("%s.%d", base, l.baseCounters[base])
}

// nextChildID allocates the next child ID for the given parent ID.
//
// The scheme is parentID.M where M starts at 1 and increments per parent.
// Deriving two children from "a3f1.1" gives "a3f1.1.1" then "a3f1.1.2".
func (l *CVLog) nextChildID(parentID string) string {
	l.childCounters[parentID]++
	return fmt.Sprintf("%s.%d", parentID, l.childCounters[parentID])
}

// ── Core Operations ───────────────────────────────────────────────────────────

// Create born a new root CV and returns its ID.
//
// The entity has no parents — it was created from nothing or from an
// external source. If origin is non-nil, the base is derived from its
// source and location via SHA-256. If origin is nil, the base is "00000000".
//
// When Enabled is false, this still allocates and returns an ID — the ID
// is needed by the entity regardless — but no entry is stored in the log.
//
// Example (compiler parser creating a token's CV):
//
//	cvID := log.Create(&Origin{Source: "app.ts", Location: "5:12"})
//	// cvID might be "a3f1b2c4.1"
//
// Example (synthetic entity with no natural origin):
//
//	cvID := log.Create(nil)
//	// cvID might be "00000000.1"
func (l *CVLog) Create(origin *Origin) string {
	base := originBase(origin)
	id := l.nextRootID(base)

	if !l.Enabled {
		return id
	}

	entry := &CVEntry{
		ID:            id,
		ParentIDs:     make([]string, 0),
		Origin:        origin,
		Contributions: make([]Contribution, 0),
		Deleted:       nil,
	}
	l.Entries[id] = entry
	return id
}

// Contribute appends a stage's processing record to a CV entry.
//
// source identifies who acted (e.g., "scope_analysis", "date_normalizer").
// tag classifies what happened (e.g., "resolved", "converted"). meta holds
// arbitrary key-value detail specific to the domain.
//
// Contributions are appended in call order — order is semantically
// meaningful as the sequence of stages.
//
// Returns an error if the CV entry has already been deleted: contributing
// to a deleted entity is a programming error, not a normal condition.
// (Deriving or merging from a deleted entity is allowed — you might need
// to create a tombstone or successor record from a deleted ancestor.)
//
// When Enabled is false, this is a no-op and returns nil.
//
// Example:
//
//	err := log.Contribute(cvID, "scope_analysis", "resolved",
//	    map[string]any{"binding": "local:count:fn_main"})
func (l *CVLog) Contribute(cvID, source, tag string, meta map[string]any) error {
	if !l.Enabled {
		return nil
	}

	entry, ok := l.Entries[cvID]
	if !ok {
		// Entry doesn't exist (e.g., log was disabled when it was created).
		// Silently ignore rather than returning an error, since this is the
		// expected outcome when tracing was previously disabled.
		return nil
	}

	if entry.Deleted != nil {
		// Returning an error rather than panicking gives the caller a chance
		// to handle the case (e.g., log a warning and skip) without crashing.
		return fmt.Errorf("CV %s is deleted", cvID)
	}

	if meta == nil {
		meta = make(map[string]any)
	}

	entry.Contributions = append(entry.Contributions, Contribution{
		Source: source,
		Tag:    tag,
		Meta:   meta,
	})

	// Track the ordered set of sources that have contributed to this log.
	// We use a linear scan because pass_order is typically small (tens of
	// stages) and we preserve insertion order rather than sorting.
	l.recordSource(source)
	return nil
}

// Derive creates a new CV that descends from an existing one.
//
// Use Derive when one entity is split into multiple outputs, or when a
// transformation produces a new entity that is conceptually "the same thing"
// expressed differently (e.g., destructuring {a, b} = x produces two
// separate bindings, each derived from x's CV).
//
// The derived CV's ID is the parent ID with a new numeric suffix:
// deriving from "a3f1.1" gives "a3f1.1.1", then "a3f1.1.2", etc.
//
// When Enabled is false, this still generates and returns an ID but stores
// no entry.
//
// Example:
//
//	// Destructuring {a, b} = x into two separate bindings
//	cvA := log.Derive(xCvID, nil)
//	cvB := log.Derive(xCvID, nil)
func (l *CVLog) Derive(parentCvID string, origin *Origin) string {
	id := l.nextChildID(parentCvID)

	if !l.Enabled {
		return id
	}

	entry := &CVEntry{
		ID:            id,
		ParentIDs:     []string{parentCvID},
		Origin:        origin,
		Contributions: make([]Contribution, 0),
		Deleted:       nil,
	}
	l.Entries[id] = entry
	return id
}

// Merge creates a new CV descended from multiple existing CVs.
//
// Use Merge when multiple entities are combined into one output (e.g.,
// inlining a function merges the call site and function body; joining two
// database tables produces one result row from two source rows).
//
// The merged CV's parent_ids lists all parents. Its ID uses the base derived
// from origin (if provided) or "00000000" (for synthetic merges), with a new
// sequence number.
//
// When Enabled is false, this still generates and returns an ID but stores
// no entry.
//
// Example:
//
//	// Inlining a function: call site and function body merge into one
//	mergedCV := log.Merge([]string{callSiteCvID, functionBodyCvID}, nil)
//
//	// Joining two database tables
//	rowCV := log.Merge([]string{ordersCvID, customersCvID},
//	    &Origin{Source: "join_stage", Location: "orders.customer_id=customers.id"})
func (l *CVLog) Merge(parentCvIDs []string, origin *Origin) string {
	base := originBase(origin)
	id := l.nextRootID(base)

	if !l.Enabled {
		return id
	}

	// Copy the parent IDs slice so we own it and callers cannot mutate it.
	parents := make([]string, len(parentCvIDs))
	copy(parents, parentCvIDs)

	entry := &CVEntry{
		ID:            id,
		ParentIDs:     parents,
		Origin:        origin,
		Contributions: make([]Contribution, 0),
		Deleted:       nil,
	}
	l.Entries[id] = entry
	return id
}

// Delete records the intentional removal of an entity.
//
// The CVEntry is NOT removed from the log — it remains permanently with a
// non-nil DeletionRecord. This is how you can answer "why did this
// disappear?" long after the fact.
//
// After Delete, calling Contribute on the same CV ID will return an error.
// Calling Derive or Merge with a deleted CV as a parent is allowed (you
// might need to create a tombstone or successor from a deleted ancestor).
//
// When Enabled is false, this is a no-op.
//
// Example:
//
//	log.Delete(cvID, "dead_code_eliminator",
//	    "unreachable from entry point",
//	    map[string]any{"entry_point_cv": mainCvID})
func (l *CVLog) Delete(cvID, source, reason string, meta map[string]any) {
	if !l.Enabled {
		return
	}

	entry, ok := l.Entries[cvID]
	if !ok {
		return
	}

	if meta == nil {
		meta = make(map[string]any)
	}

	entry.Deleted = &DeletionRecord{
		Source: source,
		Reason: reason,
		Meta:   meta,
	}
}

// Passthrough records that a stage examined an entity but made no changes.
//
// This is the identity contribution — it tells you which stages an entity
// passed through even when nothing was transformed. Without it, a stage
// that leaves data unchanged is completely invisible in the history.
//
// In performance-sensitive pipelines, Passthrough may be omitted for
// known-clean stages to reduce log size. The tradeoff is that the stage
// will be invisible in the history for unaffected entities.
//
// When Enabled is false, this is a no-op.
//
// Example:
//
//	log.Passthrough(cvID, "type_checker")
func (l *CVLog) Passthrough(cvID, source string) {
	if !l.Enabled {
		return
	}

	entry, ok := l.Entries[cvID]
	if !ok {
		return
	}

	entry.Contributions = append(entry.Contributions, Contribution{
		Source: source,
		Tag:    "passthrough",
		Meta:   make(map[string]any),
	})
	l.recordSource(source)
}

// recordSource appends source to PassOrder if it has not been seen before.
//
// We preserve insertion order (first-seen order) rather than sorting, so the
// pass order reflects the real execution sequence of the pipeline.
func (l *CVLog) recordSource(source string) {
	for _, s := range l.PassOrder {
		if s == source {
			return
		}
	}
	l.PassOrder = append(l.PassOrder, source)
}

// ── Query Operations ──────────────────────────────────────────────────────────

// Get returns the full CVEntry for a CV ID, or nil if not found.
//
// Returns nil both when the CV ID was never created and when the log was
// disabled (Enabled=false) at creation time — in both cases there is no
// stored entry.
//
// Example:
//
//	entry := log.Get(cvID)
//	if entry != nil {
//	    fmt.Println(entry.ID, len(entry.Contributions))
//	}
func (l *CVLog) Get(cvID string) *CVEntry {
	return l.Entries[cvID]
}

// Ancestors walks the parent_ids chain recursively and returns all ancestor
// CV IDs, ordered from nearest ancestor to most distant.
//
// For a deep derivation chain A → B → C → D:
//
//	Ancestors("D") returns ["C", "B", "A"]
//
// Cycles are impossible by construction (a CV cannot be its own ancestor —
// IDs always grow longer with each derivation), but we guard against
// pathological inputs via a visited set.
//
// Returns an empty slice if the CV ID is not found or has no parents.
//
// Example:
//
//	ancestors := log.Ancestors(cvID)
//	// For "a3f1.1.2.1": ["a3f1.1.2", "a3f1.1", "a3f1.1's parents..."]
func (l *CVLog) Ancestors(cvID string) []string {
	result := make([]string, 0)
	visited := make(map[string]bool)

	// BFS-like traversal using a queue, but we append in nearest-first order.
	// We process each generation's parents before moving to the next, which
	// naturally produces nearest-first ordering.
	queue := []string{cvID}
	visited[cvID] = true

	for len(queue) > 0 {
		current := queue[0]
		queue = queue[1:]

		entry, ok := l.Entries[current]
		if !ok {
			continue
		}

		for _, parentID := range entry.ParentIDs {
			if visited[parentID] {
				continue
			}
			visited[parentID] = true
			result = append(result, parentID)
			queue = append(queue, parentID)
		}
	}

	return result
}

// Descendants returns all CV IDs that have this CV ID in their ancestor chain.
//
// This is the inverse of Ancestors. It is computed by scanning all entries
// and checking their ParentIDs — on large logs with millions of entries,
// consider building an index. For the typical use case (pipeline runs with
// thousands of entities), the linear scan is fast enough.
//
// Returns an empty slice if no entries have the given CV ID as a parent.
//
// Example:
//
//	children := log.Descendants(rootCvID)
//	// All IDs derived from rootCvID, directly or transitively
func (l *CVLog) Descendants(cvID string) []string {
	result := make([]string, 0)

	// Use BFS: start with the direct children, then their children, etc.
	// This produces all descendants, not just direct children.
	visited := make(map[string]bool)
	visited[cvID] = true
	queue := []string{cvID}

	for len(queue) > 0 {
		current := queue[0]
		queue = queue[1:]

		// Scan all entries to find those that list current as a parent.
		// We build a sorted list of IDs first so the scan is deterministic.
		for _, entry := range l.Entries {
			if visited[entry.ID] {
				continue
			}
			for _, parentID := range entry.ParentIDs {
				if parentID == current {
					visited[entry.ID] = true
					result = append(result, entry.ID)
					queue = append(queue, entry.ID)
					break
				}
			}
		}
	}

	return result
}

// History returns the contributions for a CV in order.
//
// If the entity has been deleted, the deletion record is represented as a
// final synthetic contribution with tag "deleted" appended to the list. This
// gives callers a unified view of everything that happened to the entity,
// including its final removal, without requiring them to check Deleted
// separately.
//
// Returns an empty slice if the CV ID is not found.
//
// Example:
//
//	history := log.History(cvID)
//	for _, c := range history {
//	    fmt.Printf("%s: %s\n", c.Source, c.Tag)
//	}
func (l *CVLog) History(cvID string) []Contribution {
	entry, ok := l.Entries[cvID]
	if !ok {
		return make([]Contribution, 0)
	}

	// Return a copy so callers cannot mutate the internal slice.
	result := make([]Contribution, len(entry.Contributions))
	copy(result, entry.Contributions)

	// If the entity was deleted, synthesise a final contribution that
	// represents the deletion event. This gives a complete, linear narrative.
	if entry.Deleted != nil {
		deletionMeta := make(map[string]any)
		for k, v := range entry.Deleted.Meta {
			deletionMeta[k] = v
		}
		deletionMeta["reason"] = entry.Deleted.Reason
		result = append(result, Contribution{
			Source: entry.Deleted.Source,
			Tag:    "deleted",
			Meta:   deletionMeta,
		})
	}

	return result
}

// Lineage returns the full CVEntry for the entity and all its ancestors,
// ordered from oldest ancestor to the entity itself.
//
// For a derivation chain A → B → C → D:
//
//	Lineage("D") returns [entry(A), entry(B), entry(C), entry(D)]
//
// This is the complete provenance chain — everything you need to reconstruct
// the full history of an entity from birth to the present moment.
//
// Returns an empty slice if the CV ID is not found or has no stored entry.
//
// Example:
//
//	lineage := log.Lineage(cvID)
//	fmt.Printf("Entity born at: %s\n", lineage[0].Origin.Source)
func (l *CVLog) Lineage(cvID string) []*CVEntry {
	entry := l.Entries[cvID]
	if entry == nil {
		return make([]*CVEntry, 0)
	}

	// Ancestors returns nearest-first; we need oldest-first for lineage.
	ancestorIDs := l.Ancestors(cvID)

	// Reverse to get oldest-first.
	for i, j := 0, len(ancestorIDs)-1; i < j; i, j = i+1, j-1 {
		ancestorIDs[i], ancestorIDs[j] = ancestorIDs[j], ancestorIDs[i]
	}

	result := make([]*CVEntry, 0, len(ancestorIDs)+1)
	for _, id := range ancestorIDs {
		if e := l.Entries[id]; e != nil {
			result = append(result, e)
		}
	}
	result = append(result, entry)
	return result
}

// ── Serialization ─────────────────────────────────────────────────────────────

// Serialize converts the CVLog to a map[string]any suitable for JSON
// marshalling. This is the canonical interchange format between language
// implementations.
//
// The output structure matches the spec:
//
//	{
//	  "entries": { "a3f1.1": { ... }, ... },
//	  "pass_order": ["parser", "scope_analysis"],
//	  "enabled": true
//	}
//
// All non-exported counters (baseCounters, childCounters) are NOT included
// in the serialised form — they cannot be recovered from the entries map
// alone. After deserialization, the counters are reconstructed from the
// existing IDs to ensure new IDs remain unique (see DeserializeFromJSON).
func (l *CVLog) Serialize() map[string]any {
	entries := make(map[string]any)
	for id, entry := range l.Entries {
		entries[id] = serializeEntry(entry)
	}

	passOrder := make([]any, len(l.PassOrder))
	for i, s := range l.PassOrder {
		passOrder[i] = s
	}

	return map[string]any{
		"entries":    entries,
		"pass_order": passOrder,
		"enabled":    l.Enabled,
	}
}

// serializeEntry converts a CVEntry to map[string]any.
//
// We explicitly include all fields — even nil/empty ones — so that the JSON
// output is predictable and compatible across language implementations.
func serializeEntry(e *CVEntry) map[string]any {
	parentIDs := make([]any, len(e.ParentIDs))
	for i, id := range e.ParentIDs {
		parentIDs[i] = id
	}

	contributions := make([]any, len(e.Contributions))
	for i, c := range e.Contributions {
		meta := make(map[string]any)
		for k, v := range c.Meta {
			meta[k] = v
		}
		contributions[i] = map[string]any{
			"source": c.Source,
			"tag":    c.Tag,
			"meta":   meta,
		}
	}

	var originMap any
	if e.Origin != nil {
		ometa := make(map[string]any)
		for k, v := range e.Origin.Meta {
			ometa[k] = v
		}
		om := map[string]any{
			"source":   e.Origin.Source,
			"location": e.Origin.Location,
			"meta":     ometa,
		}
		if e.Origin.Timestamp != "" {
			om["timestamp"] = e.Origin.Timestamp
		}
		originMap = om
	}

	var deletedMap any
	if e.Deleted != nil {
		dmeta := make(map[string]any)
		for k, v := range e.Deleted.Meta {
			dmeta[k] = v
		}
		deletedMap = map[string]any{
			"source": e.Deleted.Source,
			"reason": e.Deleted.Reason,
			"meta":   dmeta,
		}
	}

	return map[string]any{
		"id":            e.ID,
		"parent_ids":    parentIDs,
		"origin":        originMap,
		"contributions": contributions,
		"deleted":       deletedMap,
	}
}

// ToJSONString serialises the CVLog to a compact JSON string.
//
// This uses the repo's json-serializer package, which exercises the full
// json-value → json-serializer pipeline. The output is canonical and
// interoperable with other language implementations of this spec.
//
// Example:
//
//	jsonStr, err := log.ToJSONString()
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Println(jsonStr)
func (l *CVLog) ToJSONString() (string, error) {
	data := l.Serialize()
	jv, err := jsonvalue.FromNative(data)
	if err != nil {
		return "", fmt.Errorf("correlation-vector: serialization failed: %w", err)
	}
	result, err := jsonserializer.Serialize(jv)
	if err != nil {
		return "", fmt.Errorf("correlation-vector: serialization failed: %w", err)
	}
	return result, nil
}

// DeserializeFromJSON reconstructs a CVLog from its JSON representation.
//
// This uses the repo's json-value package (which includes a full JSON parser)
// to parse the JSON string, then walks the resulting value tree to populate
// the CVLog fields.
//
// After deserialization, the ID counters (baseCounters and childCounters) are
// reconstructed from the existing entry IDs to ensure that new IDs created
// after deserialization remain unique and do not collide with existing ones.
//
// Returns an error if the JSON is malformed or has an unexpected structure.
//
// Example:
//
//	restored, err := DeserializeFromJSON(jsonStr)
//	if err != nil {
//	    log.Fatal(err)
//	}
func DeserializeFromJSON(jsonStr string) (*CVLog, error) {
	// Use the standard library for deserialization since we're parsing into
	// a flexible map[string]any and then reconstructing typed structs.
	// The json-value package is used for serialization (ToJSONString); here
	// we use encoding/json for the roundtrip to keep things simple and avoid
	// re-implementing the full deserialization walker.
	var raw map[string]any
	if err := json.Unmarshal([]byte(jsonStr), &raw); err != nil {
		return nil, fmt.Errorf("correlation-vector: failed to parse JSON: %w", err)
	}

	log := NewCVLog(false) // we will set Enabled from the JSON

	// Parse "enabled"
	if enabled, ok := raw["enabled"].(bool); ok {
		log.Enabled = enabled
	}

	// Parse "pass_order"
	if passOrderRaw, ok := raw["pass_order"].([]any); ok {
		for _, s := range passOrderRaw {
			if str, ok := s.(string); ok {
				log.PassOrder = append(log.PassOrder, str)
			}
		}
	}

	// Parse "entries"
	if entriesRaw, ok := raw["entries"].(map[string]any); ok {
		for id, entryRaw := range entriesRaw {
			entryMap, ok := entryRaw.(map[string]any)
			if !ok {
				continue
			}
			entry, err := deserializeEntry(id, entryMap)
			if err != nil {
				return nil, fmt.Errorf("correlation-vector: entry %q: %w", id, err)
			}
			log.Entries[id] = entry
		}
	}

	// Reconstruct the ID counters from existing entries so that new IDs
	// created after deserialization remain unique.
	log.rebuildCounters()
	return log, nil
}

// deserializeEntry converts a raw map to a CVEntry.
func deserializeEntry(id string, m map[string]any) (*CVEntry, error) {
	entry := &CVEntry{
		ID:            id,
		ParentIDs:     make([]string, 0),
		Contributions: make([]Contribution, 0),
	}

	// ParentIDs
	if parentIDsRaw, ok := m["parent_ids"].([]any); ok {
		for _, pid := range parentIDsRaw {
			if str, ok := pid.(string); ok {
				entry.ParentIDs = append(entry.ParentIDs, str)
			}
		}
	}

	// Origin
	if originRaw, ok := m["origin"]; ok && originRaw != nil {
		if originMap, ok := originRaw.(map[string]any); ok {
			origin := &Origin{
				Meta: make(map[string]any),
			}
			if s, ok := originMap["source"].(string); ok {
				origin.Source = s
			}
			if loc, ok := originMap["location"].(string); ok {
				origin.Location = loc
			}
			if ts, ok := originMap["timestamp"].(string); ok {
				origin.Timestamp = ts
			}
			if metaRaw, ok := originMap["meta"].(map[string]any); ok {
				for k, v := range metaRaw {
					origin.Meta[k] = v
				}
			}
			entry.Origin = origin
		}
	}

	// Contributions
	if contribsRaw, ok := m["contributions"].([]any); ok {
		for _, cRaw := range contribsRaw {
			cMap, ok := cRaw.(map[string]any)
			if !ok {
				continue
			}
			contrib := Contribution{
				Meta: make(map[string]any),
			}
			if s, ok := cMap["source"].(string); ok {
				contrib.Source = s
			}
			if t, ok := cMap["tag"].(string); ok {
				contrib.Tag = t
			}
			if metaRaw, ok := cMap["meta"].(map[string]any); ok {
				for k, v := range metaRaw {
					contrib.Meta[k] = v
				}
			}
			entry.Contributions = append(entry.Contributions, contrib)
		}
	}

	// Deleted
	if deletedRaw, ok := m["deleted"]; ok && deletedRaw != nil {
		if deletedMap, ok := deletedRaw.(map[string]any); ok {
			dr := &DeletionRecord{
				Meta: make(map[string]any),
			}
			if s, ok := deletedMap["source"].(string); ok {
				dr.Source = s
			}
			if r, ok := deletedMap["reason"].(string); ok {
				dr.Reason = r
			}
			if metaRaw, ok := deletedMap["meta"].(map[string]any); ok {
				for k, v := range metaRaw {
					dr.Meta[k] = v
				}
			}
			entry.Deleted = dr
		}
	}

	return entry, nil
}

// rebuildCounters restores the ID counters from the set of existing entry IDs.
//
// A CV ID has one of two forms:
//
//	base.N       → a root ID; we update baseCounters[base] = max(N, current)
//	base.N.M     → a child ID; we update childCounters[base.N] = max(M, current)
//	base.N.M.K   → a grandchild; we update childCounters[base.N.M] = max(K, current)
//
// In general, the counter key is everything before the last dot, and the
// sequence number is everything after the last dot. This works for arbitrary
// depth derivation chains.
func (l *CVLog) rebuildCounters() {
	for id := range l.Entries {
		lastDot := strings.LastIndex(id, ".")
		if lastDot < 0 {
			continue
		}
		prefix := id[:lastDot]
		seqStr := id[lastDot+1:]

		var seq int
		fmt.Sscanf(seqStr, "%d", &seq)

		// Determine whether this is a root ID (prefix has no dot, so it's just
		// the base hex) or a child ID (prefix itself contains a dot).
		if strings.ContainsRune(prefix, '.') {
			// Child ID: prefix is a parent CV ID.
			if l.childCounters[prefix] < seq {
				l.childCounters[prefix] = seq
			}
		} else {
			// Root ID: prefix is the 8-char hex base.
			if l.baseCounters[prefix] < seq {
				l.baseCounters[prefix] = seq
			}
		}
	}
}

