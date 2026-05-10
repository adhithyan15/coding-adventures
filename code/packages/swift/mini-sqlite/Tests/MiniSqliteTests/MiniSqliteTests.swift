import Testing
@testable import MiniSqlite

@Suite("MiniSqlite")
struct MiniSqliteTests {
    @Test("exposes DB API style constants")
    func exposesConstants() {
        #expect(MiniSqlite.apiLevel == "2.0")
        #expect(MiniSqlite.threadSafety == 1)
        #expect(MiniSqlite.paramStyle == "qmark")
    }

    @Test("creates inserts and selects rows")
    func createsInsertsAndSelectsRows() throws {
        let conn = try MiniSqlite.connect(":memory:")
        try conn.execute("CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)")
        try conn.executeMany("INSERT INTO users VALUES (?, ?, ?)", [
            [.integer(1), .text("Alice"), .bool(true)],
            [.integer(2), .text("Bob"), .bool(false)],
            [.integer(3), .text("Carol"), .bool(true)],
        ])

        let cursor = try conn.execute(
            "SELECT name FROM users WHERE active = ? ORDER BY id ASC",
            [.bool(true)]
        )
        #expect(cursor.description == [Column("name")])
        #expect(try cursor.fetchAll() == [[.text("Alice")], [.text("Carol")]])
    }

    @Test("fetches incrementally")
    func fetchesIncrementally() throws {
        let conn = try MiniSqlite.connect(":memory:")
        try conn.execute("CREATE TABLE nums (n INTEGER)")
        try conn.executeMany("INSERT INTO nums VALUES (?)", [
            [.integer(1)],
            [.integer(2)],
            [.integer(3)],
        ])
        let cursor = try conn.execute("SELECT n FROM nums ORDER BY n ASC")

        #expect(try cursor.fetchOne() == [.integer(1)])
        #expect(try cursor.fetchMany(1) == [[.integer(2)]])
        #expect(try cursor.fetchAll() == [[.integer(3)]])
        #expect(try cursor.fetchOne() == nil)
    }

    @Test("updates and deletes rows")
    func updatesAndDeletesRows() throws {
        let conn = try MiniSqlite.connect(":memory:")
        try conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
        try conn.executeMany("INSERT INTO users VALUES (?, ?)", [
            [.integer(1), .text("Alice")],
            [.integer(2), .text("Bob")],
            [.integer(3), .text("Carol")],
        ])

        let updated = try conn.execute("UPDATE users SET name = ? WHERE id = ?", [.text("Bobby"), .integer(2)])
        #expect(updated.rowCount == 1)

        let deleted = try conn.execute("DELETE FROM users WHERE id IN (?, ?)", [.integer(1), .integer(3)])
        #expect(deleted.rowCount == 2)

        let rows = try conn.execute("SELECT id, name FROM users").fetchAll()
        #expect(rows == [[.integer(2), .text("Bobby")]])
    }

    @Test("rolls back and commits snapshots")
    func rollsBackAndCommitsSnapshots() throws {
        let conn = try MiniSqlite.connect(":memory:")
        try conn.execute("CREATE TABLE users (id INTEGER, name TEXT)")
        try conn.commit()
        try conn.execute("INSERT INTO users VALUES (?, ?)", [.integer(1), .text("Alice")])
        try conn.rollback()
        #expect(try conn.execute("SELECT * FROM users").fetchAll().isEmpty)

        try conn.execute("INSERT INTO users VALUES (?, ?)", [.integer(1), .text("Alice")])
        try conn.commit()
        try conn.rollback()
        #expect(try conn.execute("SELECT * FROM users").fetchAll() == [[.integer(1), .text("Alice")]])
    }

    @Test("supports predicates ordering drop and cursor lifecycle")
    func supportsPredicatesOrderingDropAndCursorLifecycle() throws {
        let conn = try MiniSqlite.connect(":memory:")
        try conn.execute("CREATE TABLE things (id INTEGER, label TEXT, score REAL, enabled BOOLEAN)")
        try conn.execute("INSERT INTO things VALUES (1, NULL, 1.5, TRUE)")
        try conn.execute("INSERT INTO things VALUES (2, 'middle', 2.5, FALSE)")
        try conn.execute("INSERT INTO things VALUES (3, 'tail', 3.5, TRUE)")

        let ordered = try conn
            .execute("SELECT id FROM things WHERE label IS NULL OR score >= 3 ORDER BY id DESC")
            .fetchAll()
        #expect(ordered.map { $0[0] } == [.integer(3), .integer(1)])

        let filtered = try conn
            .execute("SELECT id FROM things WHERE label IS NOT NULL AND id <> 2 ORDER BY id ASC")
            .fetchAll()
        #expect(filtered == [[.integer(3)]])

        let inserted = try conn.execute(
            "INSERT INTO things VALUES (?, 'literal ? with ''quote''', 4, TRUE)",
            [.integer(4)]
        )
        #expect(inserted.lastRowId == .integer(4))
        let literalCursor = try conn.execute("SELECT label FROM things WHERE id = 4")
        #expect(try literalCursor.fetchAll() == [[.text("literal ? with 'quote'")]])
        literalCursor.close()
        expectErrorKind("ProgrammingError") {
            _ = try literalCursor.fetchAll()
        }

        try conn.execute("DROP TABLE things")
        expectErrorKind("OperationalError") {
            _ = try conn.execute("SELECT * FROM things")
        }
    }

    @Test("supports SQL transaction commands and autocommit")
    func supportsSqlTransactionCommandsAndAutocommit() throws {
        let conn = try MiniSqlite.connect(":memory:")
        try conn.execute("CREATE TABLE events (id INTEGER)")
        try conn.commit()
        try conn.execute("BEGIN")
        try conn.execute("INSERT INTO events VALUES (1)")
        try conn.execute("ROLLBACK")
        #expect(try conn.execute("SELECT * FROM events").fetchAll().isEmpty)

        let autocommit = try MiniSqlite.connect(":memory:", options: ConnectionOptions(autocommit: true))
        try autocommit.execute("CREATE TABLE events (id INTEGER)")
        try autocommit.execute("INSERT INTO events VALUES (1)")
        try autocommit.rollback()
        #expect(try autocommit.execute("SELECT * FROM events").fetchAll() == [[.integer(1)]])
    }

    @Test("validates connection strings parameters and closed connections")
    func validatesErrors() throws {
        expectErrorKind("NotSupportedError") {
            _ = try MiniSqlite.connect("app.db")
        }

        let conn = try MiniSqlite.connect(":memory:")
        try conn.execute("CREATE TABLE notes (id INTEGER)")
        expectErrorKind("ProgrammingError") {
            _ = try conn.execute("SELECT * FROM notes WHERE id = ?", [])
        }
        expectErrorKind("ProgrammingError") {
            _ = try conn.execute("SELECT * FROM notes", [.integer(1)])
        }
        expectErrorKind("OperationalError") {
            _ = try conn.execute("PRAGMA user_version")
        }
        conn.close()
        expectErrorKind("ProgrammingError") {
            _ = try conn.execute("SELECT * FROM notes")
        }
    }
}

private func expectErrorKind(_ expected: String, _ operation: () throws -> Void) {
    do {
        try operation()
        Issue.record("expected MiniSqliteError(\(expected))")
    } catch let error as MiniSqliteError {
        #expect(error.kind == expected)
    } catch {
        Issue.record("expected MiniSqliteError(\(expected)), got \(error)")
    }
}
