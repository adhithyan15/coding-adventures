// ============================================================================
// TrieTests.swift — Unit tests for Trie
// ============================================================================

import XCTest
@testable import Trie

final class TrieTests: XCTestCase {

    private func makeTrie(_ words: [String]) -> Trie<Bool> {
        var trie = Trie<Bool>()
        for w in words { trie.insert(w, value: true) }
        return trie
    }

    func testEmptyTrieHasNoKeys() {
        let trie = Trie<Int>()
        XCTAssertEqual(trie.count, 0)
        XCTAssertTrue(trie.isEmpty)
        XCTAssertNil(trie.search("anything"))
        XCTAssertFalse(trie.startsWith("a"))
        XCTAssertTrue(trie.isValid())
    }

    func testInsertAndSearchExactKeys() {
        var trie = Trie<Int>()
        trie.insert("hello", value: 42)
        XCTAssertEqual(trie.search("hello"), 42)
        XCTAssertNil(trie.search("hell"))    // prefix, not a key
        XCTAssertNil(trie.search("hellos"))  // extends past a key
        XCTAssertTrue(trie.containsKey("hello"))
        XCTAssertFalse(trie.containsKey("hell"))
        XCTAssertTrue(trie.startsWith("hell"))
    }

    func testInsertUpdatesExistingKeyWithoutGrowingSize() {
        var trie = Trie<Int>()
        trie.insert("hello", value: 1)
        trie.insert("hello", value: 99)
        XCTAssertEqual(trie.search("hello"), 99)
        XCTAssertEqual(trie.count, 1)
    }

    func testWordsWithPrefixAreLexicographic() {
        let trie = makeTrie(["app", "apple", "apply", "apt"])
        XCTAssertEqual(trie.wordsWithPrefix("app").map { $0.0 }, ["app", "apple", "apply"])
        XCTAssertEqual(trie.wordsWithPrefix("z").map { $0.0 }, [])
    }

    func testDeleteLeafAndSharedPrefixCases() {
        var trie = makeTrie(["app", "apple"])
        XCTAssertTrue(trie.delete("app"))
        XCTAssertFalse(trie.containsKey("app"))
        XCTAssertTrue(trie.containsKey("apple"))
        XCTAssertEqual(trie.count, 1)

        XCTAssertTrue(trie.delete("apple"))
        XCTAssertEqual(trie.count, 0)
        XCTAssertTrue(trie.isEmpty)
        XCTAssertTrue(trie.isValid())
    }

    func testDeleteNonexistentKeyReturnsFalse() {
        var trie = makeTrie(["apple"])
        XCTAssertFalse(trie.delete("xyz"))
        XCTAssertFalse(trie.delete("app")) // prefix, not a stored key
        XCTAssertEqual(trie.count, 1)
    }

    func testLongestPrefixMatchTracksMostSpecificKey() {
        var trie = Trie<Int>()
        trie.insert("a", value: 1)
        trie.insert("ab", value: 2)
        trie.insert("abc", value: 3)
        trie.insert("abcd", value: 4)
        let m = trie.longestPrefixMatch("abcde")
        XCTAssertEqual(m?.0, "abcd"); XCTAssertEqual(m?.1, 4)
        XCTAssertNil(trie.longestPrefixMatch("xyz"))
        let a = trie.longestPrefixMatch("a")
        XCTAssertEqual(a?.0, "a"); XCTAssertEqual(a?.1, 1)
    }

    func testUnicodeScalarAndEmptyStringKeys() {
        var trie = Trie<String>()
        trie.insert("", value: "root")
        trie.insert("cafe", value: "plain")
        trie.insert("cafe\u{301}", value: "accent-combining") // e + combining accent
        trie.insert("caf\u{e9}", value: "accent-single")       // precomposed é
        XCTAssertEqual(trie.search(""), "root")
        XCTAssertTrue(trie.startsWith("caf"))
        XCTAssertEqual(trie.search("caf\u{e9}"), "accent-single")
        // The combining form and the precomposed form are distinct scalar keys.
        XCTAssertEqual(trie.search("cafe\u{301}"), "accent-combining")
        XCTAssertEqual(trie.count, 4)
    }

    func testKeysAndAllWordsAreSorted() {
        let trie = makeTrie(["banana", "app", "apple", "apt"])
        XCTAssertEqual(trie.keys(), ["app", "apple", "apt", "banana"])
        XCTAssertEqual(trie.allWords().count, 4)
    }

    func testDescriptionMentionsSize() {
        let trie = makeTrie(["app", "apple"])
        XCTAssertTrue(trie.description.contains("2 keys"))
    }

    func testValueSemanticsCopyOnAssign() {
        var a = makeTrie(["x"])
        var b = a
        b.insert("y", value: true)
        // Mutating the copy must not affect the original (value type).
        XCTAssertEqual(a.count, 1)
        XCTAssertEqual(b.count, 2)
        a.delete("x")
        XCTAssertEqual(a.count, 0)
        XCTAssertEqual(b.count, 2)
    }
}
