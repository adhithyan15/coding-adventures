module MiniSqliteSpec (spec) where

import MiniSqlite
import Test.Hspec

spec :: Spec
spec = describe "MiniSqlite" $ do
    it "exposes DB API style constants" $ do
        apiLevel `shouldBe` "2.0"
        threadSafety `shouldBe` 1
        paramStyle `shouldBe` "qmark"

    it "creates inserts and selects rows" $ do
        conn <- expectRight =<< connect ":memory:"
        _ <- expectRight =<< execute conn "CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)" []
        _ <- expectRight =<< executeMany conn "INSERT INTO users VALUES (?, ?, ?)"
            [ [SqlInteger 1, SqlText "Alice", SqlBool True]
            , [SqlInteger 2, SqlText "Bob", SqlBool False]
            , [SqlInteger 3, SqlText "Carol", SqlBool True]
            ]

        cursor <- expectRight =<< execute conn "SELECT name FROM users WHERE active = ? ORDER BY id ASC" [SqlBool True]
        description <- cursorDescription cursor
        description `shouldBe` [Column "name"]
        rows <- expectRight =<< fetchAll cursor
        rows `shouldBe` [[SqlText "Alice"], [SqlText "Carol"]]

    it "fetches incrementally" $ do
        conn <- expectRight =<< connect ":memory:"
        _ <- expectRight =<< execute conn "CREATE TABLE nums (n INTEGER)" []
        _ <- expectRight =<< executeMany conn "INSERT INTO nums VALUES (?)"
            [[SqlInteger 1], [SqlInteger 2], [SqlInteger 3]]
        cursor <- expectRight =<< execute conn "SELECT n FROM nums ORDER BY n ASC" []

        firstRow <- expectRight =<< fetchOne cursor
        firstRow `shouldBe` Just [SqlInteger 1]
        batch <- expectRight =<< fetchMany cursor 1
        batch `shouldBe` [[SqlInteger 2]]
        remaining <- expectRight =<< fetchAll cursor
        remaining `shouldBe` [[SqlInteger 3]]
        end <- expectRight =<< fetchOne cursor
        end `shouldBe` Nothing

    it "updates and deletes rows" $ do
        conn <- expectRight =<< connect ":memory:"
        _ <- expectRight =<< execute conn "CREATE TABLE users (id INTEGER, name TEXT)" []
        _ <- expectRight =<< executeMany conn "INSERT INTO users VALUES (?, ?)"
            [ [SqlInteger 1, SqlText "Alice"]
            , [SqlInteger 2, SqlText "Bob"]
            , [SqlInteger 3, SqlText "Carol"]
            ]

        updated <- expectRight =<< execute conn "UPDATE users SET name = ? WHERE id = ?" [SqlText "Bobby", SqlInteger 2]
        cursorRowCount updated >>= (`shouldBe` 1)
        deleted <- expectRight =<< execute conn "DELETE FROM users WHERE id IN (?, ?)" [SqlInteger 1, SqlInteger 3]
        cursorRowCount deleted >>= (`shouldBe` 2)

        cursor <- expectRight =<< execute conn "SELECT id, name FROM users" []
        rows <- expectRight =<< fetchAll cursor
        rows `shouldBe` [[SqlInteger 2, SqlText "Bobby"]]

    it "rolls back and commits snapshots" $ do
        conn <- expectRight =<< connect ":memory:"
        _ <- expectRight =<< execute conn "CREATE TABLE users (id INTEGER, name TEXT)" []
        expectRightUnit =<< commit conn
        _ <- expectRight =<< execute conn "INSERT INTO users VALUES (?, ?)" [SqlInteger 1, SqlText "Alice"]
        expectRightUnit =<< rollback conn
        emptyCursor <- expectRight =<< execute conn "SELECT * FROM users" []
        emptyRows <- expectRight =<< fetchAll emptyCursor
        emptyRows `shouldBe` []

        _ <- expectRight =<< execute conn "INSERT INTO users VALUES (?, ?)" [SqlInteger 1, SqlText "Alice"]
        expectRightUnit =<< commit conn
        expectRightUnit =<< rollback conn
        committedCursor <- expectRight =<< execute conn "SELECT * FROM users" []
        committedRows <- expectRight =<< fetchAll committedCursor
        committedRows `shouldBe` [[SqlInteger 1, SqlText "Alice"]]

    it "supports predicates ordering drop and cursor lifecycle" $ do
        conn <- expectRight =<< connect ":memory:"
        _ <- expectRight =<< execute conn "CREATE TABLE things (id INTEGER, label TEXT, score REAL, enabled BOOLEAN)" []
        _ <- expectRight =<< execute conn "INSERT INTO things VALUES (1, NULL, 1.5, TRUE)" []
        _ <- expectRight =<< execute conn "INSERT INTO things VALUES (2, 'middle', 2.5, FALSE)" []
        _ <- expectRight =<< execute conn "INSERT INTO things VALUES (3, 'tail', 3.5, TRUE)" []

        cursor <- expectRight =<< execute conn "SELECT id FROM things WHERE label IS NULL OR score >= 3 ORDER BY id DESC" []
        rows <- expectRight =<< fetchAll cursor
        rows `shouldBe` [[SqlInteger 3], [SqlInteger 1]]

        filtered <- expectRight =<< execute conn "SELECT id FROM things WHERE label IS NOT NULL AND id <> 2 ORDER BY id ASC" []
        filteredRows <- expectRight =<< fetchAll filtered
        filteredRows `shouldBe` [[SqlInteger 3]]

        inserted <- expectRight =<< execute conn "INSERT INTO things VALUES (?, 'literal ? with ''quote''', 4, TRUE)" [SqlInteger 4]
        cursorLastRowId inserted >>= (`shouldBe` Just (SqlInteger 4))
        literalCursor <- expectRight =<< execute conn "SELECT label FROM things WHERE id = 4" []
        literalRows <- expectRight =<< fetchAll literalCursor
        literalRows `shouldBe` [[SqlText "literal ? with 'quote'"]]
        closeCursor literalCursor
        closed <- fetchAll literalCursor
        closed `shouldBe` Left (MiniSqliteError "ProgrammingError" "cursor is closed")

        _ <- expectRight =<< execute conn "DROP TABLE things" []
        missing <- execute conn "SELECT * FROM things" []
        errorKindOf missing `shouldBe` Just "OperationalError"

    it "supports SQL transaction commands and autocommit" $ do
        conn <- expectRight =<< connect ":memory:"
        _ <- expectRight =<< execute conn "CREATE TABLE events (id INTEGER)" []
        expectRightUnit =<< commit conn
        _ <- expectRight =<< execute conn "BEGIN" []
        _ <- expectRight =<< execute conn "INSERT INTO events VALUES (1)" []
        _ <- expectRight =<< execute conn "ROLLBACK" []
        emptyCursor <- expectRight =<< execute conn "SELECT * FROM events" []
        emptyRows <- expectRight =<< fetchAll emptyCursor
        emptyRows `shouldBe` []

        autocommitConn <- expectRight =<< connectWith (defaultConnectionOptions { autocommit = True }) ":memory:"
        _ <- expectRight =<< execute autocommitConn "CREATE TABLE events (id INTEGER)" []
        _ <- expectRight =<< execute autocommitConn "INSERT INTO events VALUES (1)" []
        expectRightUnit =<< rollback autocommitConn
        cursor <- expectRight =<< execute autocommitConn "SELECT * FROM events" []
        rows <- expectRight =<< fetchAll cursor
        rows `shouldBe` [[SqlInteger 1]]

    it "validates connection strings and parameters" $ do
        rejected <- connect "app.db"
        errorKindOf rejected `shouldBe` Just "NotSupportedError"

        conn <- expectRight =<< connect ":memory:"
        _ <- expectRight =<< execute conn "CREATE TABLE notes (id INTEGER)" []
        tooFew <- execute conn "SELECT * FROM notes WHERE id = ?" []
        errorKindOf tooFew `shouldBe` Just "ProgrammingError"
        tooMany <- execute conn "SELECT * FROM notes" [SqlInteger 1]
        errorKindOf tooMany `shouldBe` Just "ProgrammingError"
        unsupported <- execute conn "PRAGMA user_version" []
        errorKindOf unsupported `shouldBe` Just "OperationalError"
        close conn
        closed <- execute conn "SELECT * FROM notes" []
        errorKindOf closed `shouldBe` Just "ProgrammingError"

expectRight :: (Show err) => Either err a -> IO a
expectRight result =
    case result of
        Left failure -> expectationFailure ("expected Right, got Left: " ++ show failure) >> error "unreachable"
        Right value -> pure value

expectRightUnit :: (Show err) => Either err () -> IO ()
expectRightUnit result =
    case result of
        Left failure -> expectationFailure ("expected Right (), got Left: " ++ show failure)
        Right () -> pure ()

errorKindOf :: Either MiniSqliteError a -> Maybe String
errorKindOf result =
    case result of
        Left failure -> Just (errorKind failure)
        Right _ -> Nothing
