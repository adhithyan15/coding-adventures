import Testing
@testable import SqlBackend

@Suite("SqlBackend")
struct SqlBackendTests {
    @Test("classifies and compares SQL values")
    func classifiesAndComparesSQLValues() {
        #expect(SQLValues.isSQLValue(nil))
        #expect(SQLValues.isSQLValue(SQLValue.bool(true)))
        #expect(SQLValues.isSQLValue(SQLValue.integer(42)))
        #expect(SQLValues.isSQLValue(SQLValue.real(1.5)))
        #expect(SQLValues.isSQLValue(SQLValue.text("text")))
        #expect(SQLValues.isSQLValue(SQLValues.blob([0x61, 0x62, 0x63])))
        #expect(!SQLValues.isSQLValue(["not": "sql"]))

        #expect(SQLValues.typeName(.null) == "NULL")
        #expect(SQLValues.typeName(.bool(false)) == "BOOLEAN")
        #expect(SQLValues.typeName(.integer(1)) == "INTEGER")
        #expect(SQLValues.typeName(.real(1.0)) == "REAL")
        #expect(SQLValues.typeName(.text("x")) == "TEXT")
        #expect(SQLValues.typeName(.blob([0x78])) == "BLOB")

        #expect(SQLValues.compare(.null, .integer(1)) < 0)
        #expect(SQLValues.compare(.bool(false), .bool(true)) < 0)
        #expect(SQLValues.compare(.integer(1), .real(2.0)) < 0)
        #expect(SQLValues.compare(.text("b"), .text("a")) > 0)
        #expect(SQLValues.compare(.blob([0x61]), .blob([0x61])) == 0)
    }

    @Test("iterators and cursors expose positioned copies")
    func iteratorsAndCursorsExposePositionedCopies() throws {
        let iterator = ListRowIterator([
            row(("id", .integer(1)), ("name", .text("Ada"))),
            row(("id", .integer(2)), ("name", .text("Grace"))),
        ])

        var first = iterator.next()
        #expect(first?["name"] == .text("Ada"))
        first?["name"] = .text("mutated")
        #expect(first?["name"] == .text("mutated"))
        #expect(iterator.next()?["name"] == .text("Grace"))
        #expect(iterator.next() == nil)

        let cursor = try users().openCursor("users")
        let firstRow = cursor.next()
        #expect(firstRow?["name"] == .text("Ada"))

        var current = cursor.currentRow()
        #expect(current?["name"] == .text("Ada"))
        current?["name"] = .text("mutated")
        #expect(cursor.currentRow()?["name"] == .text("Ada"))
    }

    @Test("creates tables, inserts rows, scans rows, and adapts schema")
    func createsTablesInsertsRowsScansRowsAndAdaptsSchema() throws {
        let backend = try users()
        #expect(backend.tables() == ["users"])
        #expect(try backend.columns("USERS").map(\.name) == ["id", "name", "email"])

        let provider = backendAsSchemaProvider(backend)
        #expect(try provider.columns("users") == ["id", "name", "email"])

        let rows = try backend.scan("users").toArray()
        #expect(rows.count == 2)
        #expect(rows[0]["name"] == .text("Ada"))
        #expect(rows[1]["email"] == .null)
    }

    @Test("rejects bad rows with typed constraint errors")
    func rejectsBadRowsWithTypedConstraintErrors() throws {
        let backend = try users()

        expectBackendError("ConstraintViolation") {
            try backend.insert("users", row(("id", .integer(2))))
        }
        expectBackendError("ConstraintViolation") {
            try backend.insert("users", row(("id", .integer(1)), ("name", .text("Ada Again"))))
        }
        expectBackendError("ColumnNotFound") {
            try backend.insert("users", row(("id", .integer(3)), ("name", .text("Lin")), ("missing", .integer(1))))
        }

        try backend.insert("users", row(("id", .integer(3)), ("name", .text("Lin")), ("email", .text("lin@example.test"))))

        expectBackendError("ConstraintViolation") {
            try backend.insert("users", row(("id", .integer(4)), ("name", .text("Other Lin")), ("email", .text("lin@example.test"))))
        }
    }

    @Test("updates and deletes positioned rows")
    func updatesAndDeletesPositionedRows() throws {
        let backend = try users()
        let cursor = try backend.openCursor("users")
        _ = cursor.next()

        try backend.update("users", cursor: cursor, assignments: row(("name", .text("Augusta Ada"))))
        #expect(try backend.scan("users").toArray()[0]["name"] == .text("Augusta Ada"))

        _ = cursor.next()
        try backend.delete("users", cursor: cursor)
        let rows = try backend.scan("users").toArray()
        #expect(rows.count == 1)
        #expect(rows[0]["name"] == .text("Augusta Ada"))
    }

    @Test("creates, alters, and drops tables")
    func createsAltersAndDropsTables() throws {
        let backend = try users()

        expectBackendError("TableAlreadyExists") {
            try backend.createTable("users", [], ifNotExists: false)
        }

        try backend.createTable("users", [], ifNotExists: true)
        try backend.addColumn("users", ColumnDef("active", "BOOLEAN", defaultValue: .bool(true)))
        #expect(try backend.scan("users").toArray()[0]["active"] == .bool(true))

        expectBackendError("ColumnAlreadyExists") {
            try backend.addColumn("users", ColumnDef("ACTIVE", "BOOLEAN"))
        }

        try backend.dropTable("users", ifExists: false)
        expectBackendError("TableNotFound") {
            _ = try backend.columns("users")
        }
        try backend.dropTable("users", ifExists: true)
    }

    @Test("scans indexes, fetches row IDs, and enforces unique indexes")
    func scansIndexesFetchesRowIDsAndEnforcesUniqueIndexes() throws {
        let backend = try users()
        try backend.insert("users", row(("id", .integer(3)), ("name", .text("Lin"))))
        try backend.createIndex(IndexDef("idx_users_name", table: "users", columns: ["name"]))

        let rowIDs = try backend.scanIndex("idx_users_name", lo: [.text("G")], hi: [.text("M")], loInclusive: false, hiInclusive: false)
        let rows = try backend.scanByRowIDs("users", rowIDs).toArray()
        #expect(rows.map { $0["name"] ?? .null } == [.text("Grace"), .text("Lin")])
        #expect(backend.listIndexes("users")[0].name == "idx_users_name")

        expectBackendError("IndexAlreadyExists") {
            try backend.createIndex(IndexDef("idx_users_name", table: "users", columns: ["id"]))
        }

        try backend.dropIndex("idx_users_name")
        #expect(backend.listIndexes().isEmpty)
        try backend.dropIndex("idx_users_name", ifExists: true)
        expectBackendError("IndexNotFound") {
            _ = try backend.scanIndex("missing")
        }

        try backend.createIndex(IndexDef("idx_name_unique", table: "users", columns: ["name"], unique: true))
        expectBackendError("ConstraintViolation") {
            try backend.insert("users", row(("id", .integer(4)), ("name", .text("Lin"))))
        }
    }

    @Test("transactions and savepoints restore snapshots")
    func transactionsAndSavepointsRestoreSnapshots() throws {
        let backend = try users()
        let handle = try backend.beginTransaction()
        try backend.insert("users", row(("id", .integer(3)), ("name", .text("Lin"))))
        #expect(backend.currentTransaction == handle)

        try backend.rollback(handle)
        #expect(try backend.scan("users").toArray().count == 2)

        let second = try backend.beginTransaction()
        try backend.insert("users", row(("id", .integer(3)), ("name", .text("Lin"))))
        try backend.createSavepoint("after_lin")
        try backend.insert("users", row(("id", .integer(4)), ("name", .text("Katherine"))))
        try backend.rollbackToSavepoint("after_lin")
        #expect(try backend.scan("users").toArray().count == 3)
        try backend.rollback(second)
        #expect(try backend.scan("users").toArray().count == 2)

        let third = try backend.beginTransaction()
        try backend.insert("users", row(("id", .integer(3)), ("name", .text("Lin"))))
        try backend.createSavepoint("after_lin")
        try backend.releaseSavepoint("after_lin")
        try backend.commit(third)
        #expect(backend.currentTransaction == nil)
        #expect(try backend.scan("users").toArray().count == 3)
    }

    @Test("stores triggers and version fields")
    func storesTriggersAndVersionFields() throws {
        let backend = try users()
        let initial = backend.schemaVersion
        let trigger = TriggerDef("users_ai", table: "users", timing: "after", event: "insert", body: "SELECT 1")
        try backend.createTrigger(trigger)

        #expect(backend.schemaVersion > initial)
        #expect(backend.listTriggers("users")[0].name == "users_ai")
        #expect(backend.listTriggers("users")[0].timing == "AFTER")

        expectBackendError("TriggerAlreadyExists") {
            try backend.createTrigger(trigger)
        }

        backend.userVersion = 7
        #expect(backend.userVersion == 7)

        try backend.dropTrigger("users_ai")
        #expect(backend.listTriggers("users").isEmpty)
        try backend.dropTrigger("users_ai", ifExists: true)
        expectBackendError("TriggerNotFound") {
            try backend.dropTrigger("users_ai")
        }
    }
}

private func users() throws -> InMemoryBackend {
    let backend = InMemoryBackend()
    try backend.createTable(
        "users",
        [
            ColumnDef("id", "INTEGER", primaryKey: true),
            ColumnDef("name", "TEXT", notNull: true),
            ColumnDef("email", "TEXT", unique: true),
        ]
    )
    try backend.insert("users", row(("id", .integer(1)), ("name", .text("Ada")), ("email", .text("ada@example.test"))))
    try backend.insert("users", row(("id", .integer(2)), ("name", .text("Grace"))))
    return backend
}

private func row(_ pairs: (String, SQLValue)...) -> Row {
    Dictionary(uniqueKeysWithValues: pairs)
}

private func expectBackendError(_ expected: String, _ operation: () throws -> Void) {
    do {
        try operation()
        Issue.record("expected BackendError(\(expected))")
    } catch let error as BackendError {
        #expect(error.kind == expected)
    } catch {
        Issue.record("expected BackendError(\(expected)), got \(error)")
    }
}
