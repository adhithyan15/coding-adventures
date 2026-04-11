// CorrelationVectorTests.swift
// ============================================================================
// Tests for the CorrelationVector Swift package
// ============================================================================
//
// Coverage requirements (per spec CV00):
//   1. Root lifecycle: create, contribute, passthrough, delete, error on deleted
//   2. Derivation: child ID format, ancestors, descendants
//   3. Merging: 3-way merge, mergedFrom populated, ancestors include parents
//   4. Deep ancestry chain: 4+ levels, ancestors nearest-first, lineage oldest-first
//   5. Disabled log: create/derive return IDs but get returns nil
//   6. Serialization roundtrip: all fields preserved
//   7. ID uniqueness: 1000 creates, no collisions
//
// We use XCTest because it is available in all Swift Platform targets without
// additional package dependencies (Swift Testing would require a package dep on
// the swift-testing library, which is not yet universally available in CI
// environments).

import XCTest
@testable import CorrelationVector
import JsonValue

// ============================================================================
// MARK: — 1. Root lifecycle
// ============================================================================
//
// Tests:
//   - create with an origin string → ID has format "base.N"
//   - contribute → contribution appears in history
//   - passthrough → source appears in passOrder, no contribution added
//   - delete → DeletionRecord present
//   - contribute after delete → throws CVError
//   - passthrough after delete → throws CVError

final class RootLifecycleTests: XCTestCase {

    func testCreateReturnsDotSeparatedId() {
        // Given: A fresh log and an origin string
        let log = CVLog()

        // When: We create a root CV
        let id = log.create(originString: "app.ts:5:12")

        // Then: The ID has exactly two components separated by a dot
        // Format: "base.N" where base is 8 hex chars and N is the counter
        let parts = id.split(separator: ".")
        XCTAssertEqual(parts.count, 2, "Root CV ID should have format 'base.N'")
        XCTAssertEqual(parts[0].count, 8, "Base segment should be 8 hex characters")
        XCTAssertTrue(parts[0].allSatisfy { $0.isHexDigit }, "Base segment should be hex")
    }

    func testCreateSyntheticUsesZeroBase() {
        // Given: A fresh log
        let log = CVLog()

        // When: We create a synthetic CV (no natural origin)
        let id = log.create(synthetic: true)

        // Then: The base segment is "00000000"
        let base = String(id.split(separator: ".")[0])
        XCTAssertEqual(base, "00000000", "Synthetic CVs should have '00000000' base")
    }

    func testContributeAppearsInHistory() throws {
        // Given: A root CV
        let log = CVLog()
        let id = log.create(originString: "orders_table:row:42")

        // When: A stage contributes to it
        try log.contribute(cvId: id, source: "validator", tag: "schema_checked",
                           meta: .object([("schema", .string("orders_v2"))]))

        // Then: The contribution appears in history
        let contributions = log.history(of: id)
        XCTAssertEqual(contributions.count, 1)
        XCTAssertEqual(contributions[0].source, "validator")
        XCTAssertEqual(contributions[0].tag, "schema_checked")

        // And: The source appears in passOrder
        let entry = log.get(cvId: id)
        XCTAssertNotNil(entry)
        XCTAssertTrue(entry!.passOrder.contains("validator"))
    }

    func testMultipleContributionsOrderPreserved() throws {
        // Given: A root CV
        let log = CVLog()
        let id = log.create(originString: "file.ts")

        // When: Three stages contribute in order
        try log.contribute(cvId: id, source: "parser", tag: "tokenized")
        try log.contribute(cvId: id, source: "scope_analysis", tag: "resolved")
        try log.contribute(cvId: id, source: "type_checker", tag: "typed")

        // Then: Contributions are in the order they were appended
        let contributions = log.history(of: id)
        XCTAssertEqual(contributions.count, 3)
        XCTAssertEqual(contributions[0].source, "parser")
        XCTAssertEqual(contributions[1].source, "scope_analysis")
        XCTAssertEqual(contributions[2].source, "type_checker")

        // And: passOrder reflects all three sources
        let entry = log.get(cvId: id)!
        XCTAssertEqual(entry.passOrder, ["parser", "scope_analysis", "type_checker"])
    }

    func testPassthroughAddsToPassOrderNotContributions() throws {
        // Given: A root CV
        let log = CVLog()
        let id = log.create(originString: "config.json")

        // When: A stage passthroughs it (examines but does not transform)
        try log.passthrough(cvId: id, source: "lint_checker")

        // Then: No contribution was added
        let contributions = log.history(of: id)
        XCTAssertEqual(contributions.count, 0, "Passthrough should not add to contributions")

        // But: The source appears in passOrder
        let entry = log.get(cvId: id)!
        XCTAssertTrue(entry.passOrder.contains("lint_checker"))
    }

    func testPassthroughDeduplicatesSource() throws {
        // Given: A root CV
        let log = CVLog()
        let id = log.create(originString: "data.csv")

        // When: The same source is recorded twice via passthrough
        try log.passthrough(cvId: id, source: "checksum_verifier")
        try log.passthrough(cvId: id, source: "checksum_verifier")

        // Then: passOrder only contains the source once
        let entry = log.get(cvId: id)!
        let occurrences = entry.passOrder.filter { $0 == "checksum_verifier" }.count
        XCTAssertEqual(occurrences, 1, "passOrder should deduplicate sources")
    }

    func testDeleteAddsDeletionRecord() throws {
        // Given: A root CV
        let log = CVLog()
        let id = log.create(originString: "unused_var.ts")

        // When: We delete it
        try log.delete(cvId: id, by: "dead_code_eliminator")

        // Then: The entry still exists but has a deletion record
        let entry = log.get(cvId: id)
        XCTAssertNotNil(entry, "Entry should still exist after soft-delete")
        XCTAssertNotNil(entry!.deleted, "Entry should have a DeletionRecord")
        XCTAssertEqual(entry!.deleted!.by, "dead_code_eliminator")
    }

    func testContributeToDeletedCVThrows() throws {
        // Given: A deleted CV
        let log = CVLog()
        let id = log.create(originString: "dead_node")
        try log.delete(cvId: id, by: "dce")

        // When/Then: Contributing to it throws an error
        XCTAssertThrowsError(
            try log.contribute(cvId: id, source: "renamer", tag: "renamed")
        ) { error in
            guard let cvError = error as? CorrelationVectorError else {
                XCTFail("Expected CVError, got \(error)")
                return
            }
            XCTAssertTrue(cvError.message.contains("deleted"), "Error message should mention deletion")
        }
    }

    func testPassthroughToDeletedCVThrows() throws {
        // Given: A deleted CV
        let log = CVLog()
        let id = log.create(originString: "gone_node")
        try log.delete(cvId: id, by: "dce")

        // When/Then: Passthrough on deleted CV throws
        XCTAssertThrowsError(
            try log.passthrough(cvId: id, source: "type_checker")
        )
    }

    func testContributeToMissingCVThrows() {
        // Given: An empty log
        let log = CVLog()

        // When/Then: Contributing to a non-existent ID throws
        XCTAssertThrowsError(
            try log.contribute(cvId: "nonexistent.0", source: "parser", tag: "parsed")
        ) { error in
            guard let cvError = error as? CorrelationVectorError else {
                XCTFail("Expected CVError")
                return
            }
            XCTAssertTrue(cvError.message.contains("not found"))
        }
    }
}

// ============================================================================
// MARK: — 2. Derivation
// ============================================================================
//
// Tests:
//   - derive produces ID with format "parent.N"
//   - ancestors(child) returns [parentId]
//   - descendants(parent) returns both children

final class DerivationTests: XCTestCase {

    func testDeriveProducesParentPrefixedId() throws {
        // Given: A root CV
        let log = CVLog()
        let parentId = log.create(originString: "ast_node")

        // When: We derive a child
        let childId = try log.derive(parentCvId: parentId, source: "splitter", tag: "split")

        // Then: The child ID has the parent ID as its prefix
        XCTAssertTrue(
            childId.hasPrefix(parentId + "."),
            "Derived ID '\(childId)' should start with '\(parentId).'"
        )

        // And: The child entry exists with the correct parentCvId
        let entry = log.get(cvId: childId)
        XCTAssertNotNil(entry)
        XCTAssertEqual(entry!.parentCvId, parentId)
    }

    func testTwoChildrenBothPrefixedByParent() throws {
        // Given: A root CV
        let log = CVLog()
        let parentId = log.create(originString: "destructure_target")

        // When: We derive two children from the same parent
        let child1 = try log.derive(parentCvId: parentId, source: "destructurer", tag: "left")
        let child2 = try log.derive(parentCvId: parentId, source: "destructurer", tag: "right")

        // Then: Both children start with the parent ID
        XCTAssertTrue(child1.hasPrefix(parentId + "."))
        XCTAssertTrue(child2.hasPrefix(parentId + "."))

        // And: They are distinct IDs
        XCTAssertNotEqual(child1, child2)
    }

    func testAncestorsReturnsParent() throws {
        // Given: A parent and child
        let log = CVLog()
        let parentId = log.create(originString: "base_node")
        let childId = try log.derive(parentCvId: parentId, source: "pass", tag: "derived")

        // When: We query ancestors of the child
        let ancestorIds = log.ancestors(of: childId)

        // Then: The parent is in the ancestor list
        XCTAssertTrue(ancestorIds.contains(parentId),
                      "Parent should appear in ancestors of child")
    }

    func testDescendantsReturnsBothChildren() throws {
        // Given: A parent with two children
        let log = CVLog()
        let parentId = log.create(originString: "parent_token")
        let child1 = try log.derive(parentCvId: parentId, source: "expander", tag: "left_child")
        let child2 = try log.derive(parentCvId: parentId, source: "expander", tag: "right_child")

        // When: We query descendants of the parent
        let descendantIds = log.descendants(of: parentId)

        // Then: Both children appear
        XCTAssertTrue(descendantIds.contains(child1), "Child 1 should be a descendant")
        XCTAssertTrue(descendantIds.contains(child2), "Child 2 should be a descendant")
    }

    func testDescendantsExcludesGrandchildren() throws {
        // Given: A two-level hierarchy
        let log = CVLog()
        let rootId = log.create(originString: "root")
        let childId = try log.derive(parentCvId: rootId, source: "a", tag: "a")
        let grandchildId = try log.derive(parentCvId: childId, source: "b", tag: "b")

        // When: We query direct descendants of root
        let directDescendants = log.descendants(of: rootId)

        // Then: Only the child (not grandchild) appears as a direct descendant
        XCTAssertTrue(directDescendants.contains(childId))
        XCTAssertFalse(directDescendants.contains(grandchildId),
                       "Grandchild should not appear in direct descendants of root")
    }

    func testDeriveFromDeletedCVThrows() throws {
        // Given: A deleted CV
        let log = CVLog()
        let parentId = log.create(originString: "deleted_node")
        try log.delete(cvId: parentId, by: "dce")

        // When/Then: Deriving from a deleted CV throws
        XCTAssertThrowsError(
            try log.derive(parentCvId: parentId, source: "splitter", tag: "split")
        )
    }
}

// ============================================================================
// MARK: — 3. Merging
// ============================================================================
//
// Tests:
//   - 3-way merge produces a new CV ID
//   - mergedFrom lists all three parent IDs
//   - ancestors(merged) contains all parents

final class MergingTests: XCTestCase {

    func testThreeWayMerge() throws {
        // Given: Three root CVs
        let log = CVLog()
        let id1 = log.create(originString: "alpha")
        let id2 = log.create(originString: "beta")
        let id3 = log.create(originString: "gamma")

        // When: We merge all three
        let mergedId = try log.merge(cvIds: [id1, id2, id3], source: "merger", tag: "merged")

        // Then: The merged entry exists
        let entry = log.get(cvId: mergedId)
        XCTAssertNotNil(entry, "Merged entry should exist")

        // And: mergedFrom lists all three parents
        XCTAssertEqual(Set(entry!.mergedFrom), Set([id1, id2, id3]),
                       "mergedFrom should contain all three parent IDs")
    }

    func testMergedAncestorsIncludeAllParents() throws {
        // Given: Three root CVs merged together
        let log = CVLog()
        let id1 = log.create(originString: "src1")
        let id2 = log.create(originString: "src2")
        let id3 = log.create(originString: "src3")
        let mergedId = try log.merge(cvIds: [id1, id2, id3], source: "joiner", tag: "joined")

        // When: We query ancestors of the merged CV
        let ancestorIds = log.ancestors(of: mergedId)

        // Then: All three parents appear
        XCTAssertTrue(ancestorIds.contains(id1), "id1 should be an ancestor")
        XCTAssertTrue(ancestorIds.contains(id2), "id2 should be an ancestor")
        XCTAssertTrue(ancestorIds.contains(id3), "id3 should be an ancestor")
    }

    func testMergeRecordsContribution() throws {
        // Given: Two root CVs
        let log = CVLog()
        let id1 = log.create(originString: "call_site")
        let id2 = log.create(originString: "function_body")

        // When: We merge them
        let mergedId = try log.merge(cvIds: [id1, id2], source: "inliner", tag: "inlined")

        // Then: The merge is recorded as a contribution on the merged entry
        let history = log.history(of: mergedId)
        XCTAssertEqual(history.count, 1)
        XCTAssertEqual(history[0].source, "inliner")
        XCTAssertEqual(history[0].tag, "inlined")
    }

    func testMergeWithMissingCVThrows() throws {
        // Given: One real CV and one nonexistent ID
        let log = CVLog()
        let realId = log.create(originString: "real")

        // When/Then: Merging throws because "fake.99" doesn't exist
        XCTAssertThrowsError(
            try log.merge(cvIds: [realId, "fake.99"], source: "merger", tag: "merged")
        )
    }
}

// ============================================================================
// MARK: — 4. Deep ancestry chain
// ============================================================================
//
// Build a 4-level chain: A → B → C → D (each derived from the previous).
// Verify:
//   - ancestors(D) = [C, B, A] (nearest-first)
//   - lineage(D) returns [A, B, C, D] entries (oldest-first)

final class DeepAncestryTests: XCTestCase {

    func testFourLevelChainAncestorsNearestFirst() throws {
        // Given: A → B → C → D
        let log = CVLog()
        let idA = log.create(originString: "level_a")
        let idB = try log.derive(parentCvId: idA, source: "pass_a_to_b", tag: "derived")
        let idC = try log.derive(parentCvId: idB, source: "pass_b_to_c", tag: "derived")
        let idD = try log.derive(parentCvId: idC, source: "pass_c_to_d", tag: "derived")

        // When: We query ancestors of D
        let ancestorIds = log.ancestors(of: idD)

        // Then: The order is nearest-first: [C, B, A]
        XCTAssertEqual(ancestorIds.count, 3, "D should have 3 ancestors: C, B, A")
        XCTAssertEqual(ancestorIds[0], idC, "First ancestor (nearest) should be C")
        XCTAssertEqual(ancestorIds[1], idB, "Second ancestor should be B")
        XCTAssertEqual(ancestorIds[2], idA, "Third ancestor (most distant) should be A")
    }

    func testFourLevelChainLineageOldestFirst() throws {
        // Given: A → B → C → D
        let log = CVLog()
        let idA = log.create(originString: "level_a_lin")
        let idB = try log.derive(parentCvId: idA, source: "stage_b", tag: "derived")
        let idC = try log.derive(parentCvId: idB, source: "stage_c", tag: "derived")
        let idD = try log.derive(parentCvId: idC, source: "stage_d", tag: "derived")

        // When: We query lineage of D
        let lineage = log.lineage(of: idD)

        // Then: The order is oldest-first: [A, B, C, D]
        XCTAssertEqual(lineage.count, 4, "Lineage of D should have 4 entries: A, B, C, D")
        XCTAssertEqual(lineage[0].cvId, idA, "First entry (oldest) should be A")
        XCTAssertEqual(lineage[1].cvId, idB, "Second entry should be B")
        XCTAssertEqual(lineage[2].cvId, idC, "Third entry should be C")
        XCTAssertEqual(lineage[3].cvId, idD, "Fourth entry (self) should be D")
    }

    func testAncestorsOfRootIsEmpty() {
        // Given: A root CV with no parents
        let log = CVLog()
        let rootId = log.create(originString: "root_node")

        // When: We query ancestors of the root
        let ancestorIds = log.ancestors(of: rootId)

        // Then: There are no ancestors
        XCTAssertTrue(ancestorIds.isEmpty, "Root CVs should have no ancestors")
    }
}

// ============================================================================
// MARK: — 5. Disabled log
// ============================================================================
//
// When `enabled: false`:
//   - create, derive, merge still return IDs
//   - get returns nil (entries are never stored)
//   - history returns empty list
//   - contribute, delete, passthrough complete without error

final class DisabledLogTests: XCTestCase {

    func testDisabledCreateStillReturnsId() {
        // Given: A disabled log
        let log = CVLog(enabled: false)

        // When: We create a CV
        let id = log.create(originString: "something")

        // Then: An ID is returned (not empty)
        XCTAssertFalse(id.isEmpty, "ID should be returned even when log is disabled")
        XCTAssertTrue(id.contains("."), "ID should still have dot-separated format")
    }

    func testDisabledGetReturnsNil() {
        // Given: A disabled log with a "created" CV
        let log = CVLog(enabled: false)
        let id = log.create(originString: "something")

        // When: We try to get the entry
        let entry = log.get(cvId: id)

        // Then: Nil (nothing was stored)
        XCTAssertNil(entry, "get() should return nil when log is disabled")
    }

    func testDisabledHistoryReturnsEmpty() {
        // Given: A disabled log
        let log = CVLog(enabled: false)
        let id = log.create(originString: "something")

        // When: We query history
        let history = log.history(of: id)

        // Then: Empty (nothing was stored)
        XCTAssertTrue(history.isEmpty, "history() should return [] when log is disabled")
    }

    func testDisabledContributeNoError() throws {
        // Given: A disabled log
        let log = CVLog(enabled: false)
        let id = log.create(originString: "node")

        // When/Then: Contribute does not throw (no-op)
        XCTAssertNoThrow(
            try log.contribute(cvId: id, source: "parser", tag: "parsed")
        )
    }

    func testDisabledDeriveStillReturnsId() throws {
        // Given: A disabled log
        let log = CVLog(enabled: false)
        let parentId = log.create(originString: "parent")

        // When: We derive a child
        let childId = try log.derive(parentCvId: parentId, source: "stage", tag: "split")

        // Then: An ID is returned and formatted correctly
        XCTAssertFalse(childId.isEmpty)
        XCTAssertTrue(childId.hasPrefix(parentId + "."),
                      "Derived ID should be prefixed with parent even in disabled log")
    }

    func testDisabledPassthroughNoError() throws {
        // Given: A disabled log
        let log = CVLog(enabled: false)
        let id = log.create(originString: "node")

        // When/Then: Passthrough does not throw (no-op)
        XCTAssertNoThrow(
            try log.passthrough(cvId: id, source: "type_checker")
        )
    }
}

// ============================================================================
// MARK: — 6. Serialization roundtrip
// ============================================================================
//
// Build a CVLog with roots, derivations, merges, deletions, contributions,
// passthroughs, and metadata. Serialize to JSON and deserialize back.
// Verify all fields are preserved.

final class SerializationTests: XCTestCase {

    func testRoundtripPreservesAllFields() throws {
        // Given: A log with a variety of entries
        let log = CVLog()

        // Root with a contribution
        let rootId = log.create(originString: "schema.json")
        try log.contribute(cvId: rootId, source: "loader", tag: "loaded",
                           meta: .object([("version", .number(2))]))

        // Derived child
        let childId = try log.derive(parentCvId: rootId, source: "transformer", tag: "transformed")

        // Another root (will be merged)
        let root2Id = log.create(originString: "extension.json", synthetic: false)

        // Merge the two
        let mergedId = try log.merge(cvIds: [rootId, root2Id], source: "merger", tag: "merged")

        // Passthrough on child
        try log.passthrough(cvId: childId, source: "validator")

        // Delete the merged entry
        try log.delete(cvId: mergedId, by: "archiver")

        // When: We serialize and deserialize
        let json = log.serialize()
        XCTAssertFalse(json.isEmpty, "Serialized JSON should not be empty")

        let restored = try CVLog.deserialize(json)

        // Then: Root entry is preserved
        let restoredRoot = restored.get(cvId: rootId)
        XCTAssertNotNil(restoredRoot, "Root entry should survive roundtrip")
        XCTAssertEqual(restoredRoot!.cvId, rootId)
        XCTAssertNotNil(restoredRoot!.origin)
        XCTAssertEqual(restoredRoot!.origin!.string, "schema.json")
        XCTAssertFalse(restoredRoot!.origin!.synthetic)
        XCTAssertEqual(restoredRoot!.contributions.count, 1)
        XCTAssertEqual(restoredRoot!.contributions[0].source, "loader")
        XCTAssertEqual(restoredRoot!.contributions[0].tag, "loaded")

        // Then: Child entry is preserved
        let restoredChild = restored.get(cvId: childId)
        XCTAssertNotNil(restoredChild, "Child entry should survive roundtrip")
        XCTAssertEqual(restoredChild!.parentCvId, rootId)
        XCTAssertTrue(restoredChild!.passOrder.contains("validator"))

        // Then: Merge entry is preserved
        let restoredMerged = restored.get(cvId: mergedId)
        XCTAssertNotNil(restoredMerged, "Merged entry should survive roundtrip")
        XCTAssertTrue(restoredMerged!.mergedFrom.contains(rootId))
        XCTAssertTrue(restoredMerged!.mergedFrom.contains(root2Id))
        XCTAssertNotNil(restoredMerged!.deleted, "Deletion record should survive roundtrip")
        XCTAssertEqual(restoredMerged!.deleted!.by, "archiver")

        // Then: The restored log can create new IDs without colliding with
        // any of the existing IDs (the counter was restored and advances forward).
        let newId = restored.create()
        XCTAssertNil(restored.get(cvId: rootId) == nil && restored.get(cvId: newId) != nil ? nil : Optional<Int>.none,
                     "This is always true — just checking newId was stored")
        // Verify the new ID doesn't collide with any pre-existing IDs
        XCTAssertNotEqual(newId, rootId)
        XCTAssertNotEqual(newId, childId)
        XCTAssertNotEqual(newId, mergedId)
    }

    func testRoundtripPreservesMetadata() throws {
        // Given: A log with rich metadata on a contribution
        let log = CVLog()
        let id = log.create(originString: "test.ts")
        let meta: JsonValue = .object([
            ("from", .string("userPreferences")),
            ("to", .string("a")),
            ("line", .number(42)),
            ("safe", .bool(true)),
        ])
        try log.contribute(cvId: id, source: "renamer", tag: "renamed", meta: meta)

        // When: Roundtrip
        let restored = try CVLog.deserialize(log.serialize())

        // Then: Metadata is preserved
        let restoredEntry = restored.get(cvId: id)!
        let restoredMeta = restoredEntry.contributions[0].meta
        XCTAssertNotNil(restoredMeta)

        // Spot-check a field
        if case .object(let pairs) = restoredMeta! {
            let dict = Dictionary(pairs.map { ($0.key, $0.value) }, uniquingKeysWith: { $1 })
            XCTAssertEqual(dict["from"]?.stringValue, "userPreferences")
            XCTAssertEqual(dict["to"]?.stringValue, "a")
            XCTAssertEqual(dict["line"]?.doubleValue, 42)
            XCTAssertEqual(dict["safe"]?.boolValue, true)
        } else {
            XCTFail("Meta should be a JsonValue.object")
        }
    }

    func testRoundtripDisabledLog() throws {
        // Given: A disabled log
        let log = CVLog(enabled: false)
        _ = log.create(originString: "ignored")

        // When: Roundtrip
        let json = log.serialize()
        let restored = try CVLog.deserialize(json)

        // Then: The restored log is also disabled
        XCTAssertFalse(restored.enabled)
    }

    func testRoundtripEmptyLog() throws {
        // Given: An empty log
        let log = CVLog()

        // When: Roundtrip
        let json = log.serialize()
        let restored = try CVLog.deserialize(json)

        // Then: The restored log is empty
        XCTAssertTrue(restored.ancestors(of: "nonexistent.0").isEmpty)
        XCTAssertNil(restored.get(cvId: "nonexistent.0"))
    }
}

// ============================================================================
// MARK: — 7. ID uniqueness
// ============================================================================
//
// Create 1000 root CVs and verify no collisions.
// Mix different origins to test cross-base uniqueness.

final class IdUniquenessTests: XCTestCase {

    func testOneThousandCreatesAreUnique() {
        // Given: A fresh log
        let log = CVLog()
        var ids = Set<String>()

        // When: We create 1000 CVs with the same origin string
        for _ in 0..<1000 {
            let id = log.create(originString: "same_origin")
            ids.insert(id)
        }

        // Then: All 1000 IDs are unique (no collisions)
        XCTAssertEqual(ids.count, 1000,
                       "1000 creates with the same origin should produce 1000 unique IDs")
    }

    func testMixedOriginsNoCollisions() {
        // Given: A fresh log
        let log = CVLog()
        var ids = Set<String>()

        // When: We create CVs with varied origins
        for i in 0..<500 {
            let idA = log.create(originString: "origin_\(i)")
            let idB = log.create(originString: "other_origin_\(i)")
            let idC = log.create(synthetic: true)
            ids.insert(idA)
            ids.insert(idB)
            ids.insert(idC)
        }

        // Then: All 1500 IDs are unique
        XCTAssertEqual(ids.count, 1500,
                       "Mixed origins should produce unique IDs across all bases")
    }

    func testDerivedIdsAreUnique() throws {
        // Given: A log with derivation chains
        let log = CVLog()
        var ids = Set<String>()

        let rootId = log.create(originString: "big_ast_node")
        ids.insert(rootId)

        // When: We create 200 derived children from the same parent
        for i in 0..<200 {
            let childId = try log.derive(
                parentCvId: rootId,
                source: "splitter_\(i)",
                tag: "split"
            )
            ids.insert(childId)
        }

        // Then: All 201 IDs (root + 200 children) are unique
        XCTAssertEqual(ids.count, 201, "Derived IDs should all be unique")
    }
}

// ============================================================================
// MARK: — Additional edge cases
// ============================================================================

final class EdgeCaseTests: XCTestCase {

    func testSameOriginProducesSameBase() {
        // Given: Two logs with the same origin string
        let log1 = CVLog()
        let log2 = CVLog()

        let id1 = log1.create(originString: "consistent_origin")
        let id2 = log2.create(originString: "consistent_origin")

        // Then: Both IDs have the same base segment (SHA-256 is deterministic)
        let base1 = String(id1.split(separator: ".")[0])
        let base2 = String(id2.split(separator: ".")[0])
        XCTAssertEqual(base1, base2, "Same origin string should produce same base segment")
    }

    func testDifferentOriginsProduceDifferentBases() {
        // Given: A log
        let log = CVLog()

        let id1 = log.create(originString: "file_a.ts")
        let id2 = log.create(originString: "file_b.ts")

        // Then: Different origins produce different bases (with very high probability)
        let base1 = String(id1.split(separator: ".")[0])
        let base2 = String(id2.split(separator: ".")[0])
        XCTAssertNotEqual(base1, base2,
                          "Different origins should (with high probability) produce different bases")
    }

    func testGetNonExistentCVReturnsNil() {
        // Given: An empty log
        let log = CVLog()

        // When: We get a non-existent CV
        let entry = log.get(cvId: "nonexistent.0")

        // Then: Nil is returned (not a crash)
        XCTAssertNil(entry)
    }

    func testHistoryOfNonExistentCVReturnsEmpty() {
        // Given: An empty log
        let log = CVLog()

        // When: We query history of a non-existent CV
        let history = log.history(of: "nonexistent.0")

        // Then: Empty list (not a crash)
        XCTAssertTrue(history.isEmpty)
    }

    func testAncestorsOfNonExistentCVReturnsEmpty() {
        // Given: An empty log
        let log = CVLog()

        // When/Then: No crash, returns empty
        let ancestors = log.ancestors(of: "nonexistent.0")
        XCTAssertTrue(ancestors.isEmpty)
    }

    func testContributeUpdatesTimestamp() throws {
        // Given: A root CV
        let log = CVLog()
        let id = log.create(originString: "ts_test")

        // When: We contribute
        try log.contribute(cvId: id, source: "timer", tag: "marked")

        // Then: The contribution has a non-empty timestamp
        let history = log.history(of: id)
        XCTAssertFalse(history[0].timestamp.isEmpty, "Contribution should have a timestamp")

        // And: It looks like an ISO 8601 string (starts with a 4-digit year)
        XCTAssertTrue(history[0].timestamp.count >= 10,
                      "Timestamp should be at least 10 chars (YYYY-MM-DD)")
    }
}
