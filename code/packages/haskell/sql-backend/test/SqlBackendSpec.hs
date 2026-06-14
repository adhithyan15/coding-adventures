module SqlBackendSpec (spec) where

import qualified Data.Map.Strict as Map
import SqlBackend
import Test.Hspec

spec :: Spec
spec = describe "SqlBackend" $ do
    it "classifies and compares SQL values" $ do
        isSqlValue SqlNull `shouldBe` True
        isSqlValue (SqlBool True) `shouldBe` True
        isSqlValue (SqlInteger 42) `shouldBe` True
        isSqlValue (SqlReal 1.5) `shouldBe` True
        isSqlValue (SqlText "text") `shouldBe` True
        isSqlValue (SqlBlob [0x61, 0x62, 0x63]) `shouldBe` True

        sqlValueTypeName SqlNull `shouldBe` "NULL"
        sqlValueTypeName (SqlBool False) `shouldBe` "BOOLEAN"
        sqlValueTypeName (SqlInteger 1) `shouldBe` "INTEGER"
        sqlValueTypeName (SqlReal 1.0) `shouldBe` "REAL"
        sqlValueTypeName (SqlText "x") `shouldBe` "TEXT"
        sqlValueTypeName (SqlBlob [0x78]) `shouldBe` "BLOB"

        compareSqlValues SqlNull (SqlInteger 1) `shouldBe` LT
        compareSqlValues (SqlBool False) (SqlBool True) `shouldBe` LT
        compareSqlValues (SqlInteger 1) (SqlReal 2.0) `shouldBe` LT
        compareSqlValues (SqlText "b") (SqlText "a") `shouldBe` GT
        compareSqlValues (SqlBlob [0x61]) (SqlBlob [0x61]) `shouldBe` EQ

    it "iterators and cursors expose positioned rows" $ do
        let iterator = listRowIterator [row [("id", SqlInteger 1), ("name", SqlText "Ada")], row [("id", SqlInteger 2), ("name", SqlText "Grace")]]
            (first, iterator') = iteratorNext iterator
            (second, iterator'') = iteratorNext iterator'
        Map.lookup "name" <$> first `shouldBe` Just (Just (SqlText "Ada"))
        Map.lookup "name" <$> second `shouldBe` Just (Just (SqlText "Grace"))
        fst (iteratorNext iterator'') `shouldBe` Nothing

        let backend = expectRight (users)
            cursor = expectRight (openCursor backend "users")
            (firstRow, cursor') = cursorNext cursor
        Map.lookup "name" <$> firstRow `shouldBe` Just (Just (SqlText "Ada"))
        Map.lookup "name" <$> cursorCurrentRow cursor' `shouldBe` Just (Just (SqlText "Ada"))

    it "creates tables, inserts rows, scans rows, and adapts schema" $ do
        let backend = expectRight users
        tables backend `shouldBe` ["users"]
        map columnName (expectRight (columns backend "USERS")) `shouldBe` ["id", "name", "email"]
        schemaProviderColumns (backendAsSchemaProvider backend) "users" `shouldBe` Right ["id", "name", "email"]

        let rows = iteratorToList (expectRight (scan backend "users"))
        length rows `shouldBe` 2
        Map.lookup "name" (expectFirst rows) `shouldBe` Just (SqlText "Ada")
        Map.lookup "email" (rows !! 1) `shouldBe` Just SqlNull

    it "rejects bad rows with typed constraint errors" $ do
        let backend = expectRight users
        errorKindOf (insert backend "users" (row [("id", SqlInteger 2)])) `shouldBe` Just "ConstraintViolation"
        errorKindOf (insert backend "users" (row [("id", SqlInteger 1), ("name", SqlText "Ada Again")])) `shouldBe` Just "ConstraintViolation"
        errorKindOf (insert backend "users" (row [("id", SqlInteger 3), ("name", SqlText "Lin"), ("missing", SqlInteger 1)])) `shouldBe` Just "ColumnNotFound"

        let backend' = expectRight (insert backend "users" (row [("id", SqlInteger 3), ("name", SqlText "Lin"), ("email", SqlText "lin@example.test")]))
        errorKindOf (insert backend' "users" (row [("id", SqlInteger 4), ("name", SqlText "Other Lin"), ("email", SqlText "lin@example.test")])) `shouldBe` Just "ConstraintViolation"

    it "updates and deletes positioned rows" $ do
        let backend = expectRight users
            cursor = expectRight (openCursor backend "users")
            (_, cursor') = cursorNext cursor
            backend' = expectRight (update backend "users" cursor' (row [("name", SqlText "Augusta Ada")]))
        Map.lookup "name" (expectFirst (iteratorToList (expectRight (scan backend' "users")))) `shouldBe` Just (SqlText "Augusta Ada")

        let (_, cursor'') = cursorNext cursor'
            backend'' = expectRight (delete backend' "users" cursor'')
            rows = iteratorToList (expectRight (scan backend'' "users"))
        length rows `shouldBe` 1
        Map.lookup "name" (expectFirst rows) `shouldBe` Just (SqlText "Augusta Ada")

    it "creates, alters, and drops tables" $ do
        let backend = expectRight users
        errorKindOf (createTable backend "users" [] False) `shouldBe` Just "TableAlreadyExists"

        let backend' = expectRight (createTable backend "users" [] True)
            activeColumn = (defaultColumnDef "active" "BOOLEAN") { columnDefaultValue = SqlBool True, columnHasDefault = True }
            backend'' = expectRight (addColumn backend' "users" activeColumn)
        Map.lookup "active" (expectFirst (iteratorToList (expectRight (scan backend'' "users")))) `shouldBe` Just (SqlBool True)
        errorKindOf (addColumn backend'' "users" (defaultColumnDef "ACTIVE" "BOOLEAN")) `shouldBe` Just "ColumnAlreadyExists"

        let backend''' = expectRight (dropTable backend'' "users" False)
        errorKindOf (columns backend''' "users") `shouldBe` Just "TableNotFound"
        dropTable backend''' "users" True `shouldBe` Right backend'''

    it "scans indexes, fetches row IDs, and enforces unique indexes" $ do
        let backend = expectRight users
            backend' = expectRight (insert backend "users" (row [("id", SqlInteger 3), ("name", SqlText "Lin")]))
            backend'' = expectRight (createIndex backend' (IndexDef "idx_users_name" "users" ["name"] False False))
            rowIds = expectRight (scanIndex backend'' "idx_users_name" (Just [SqlText "G"]) (Just [SqlText "M"]) False False)
            rows = iteratorToList (expectRight (scanByRowIds backend'' "users" rowIds))
        map (Map.findWithDefault SqlNull "name") rows `shouldBe` [SqlText "Grace", SqlText "Lin"]
        indexName (expectFirst (listIndexes backend'' (Just "users"))) `shouldBe` "idx_users_name"
        errorKindOf (createIndex backend'' (IndexDef "idx_users_name" "users" ["id"] False False)) `shouldBe` Just "IndexAlreadyExists"

        let backend''' = expectRight (dropIndex backend'' "idx_users_name" False)
        listIndexes backend''' Nothing `shouldBe` []
        dropIndex backend''' "idx_users_name" True `shouldBe` Right backend'''
        errorKindOf (scanIndex backend''' "missing" Nothing Nothing True True) `shouldBe` Just "IndexNotFound"

        let backendUnique = expectRight (createIndex backend''' (IndexDef "idx_name_unique" "users" ["name"] True False))
        errorKindOf (insert backendUnique "users" (row [("id", SqlInteger 4), ("name", SqlText "Lin")])) `shouldBe` Just "ConstraintViolation"

    it "transactions and savepoints restore snapshots" $ do
        let backend = expectRight users
            (backend', handle) = expectRight (beginTransaction backend)
            backend'' = expectRight (insert backend' "users" (row [("id", SqlInteger 3), ("name", SqlText "Lin")]))
        backendCurrentTransaction backend'' `shouldBe` Just handle

        let rolledBack = expectRight (rollback backend'' handle)
        length (iteratorToList (expectRight (scan rolledBack "users"))) `shouldBe` 2

        let (txBackend, second) = expectRight (beginTransaction rolledBack)
            withLin = expectRight (insert txBackend "users" (row [("id", SqlInteger 3), ("name", SqlText "Lin")]))
            saved = expectRight (createSavepoint withLin "after_lin")
            withKatherine = expectRight (insert saved "users" (row [("id", SqlInteger 4), ("name", SqlText "Katherine")]))
            savepointRolledBack = expectRight (rollbackToSavepoint withKatherine "after_lin")
        length (iteratorToList (expectRight (scan savepointRolledBack "users"))) `shouldBe` 3
        length (iteratorToList (expectRight (scan (expectRight (rollback savepointRolledBack second)) "users"))) `shouldBe` 2

        let (commitBackend, third) = expectRight (beginTransaction rolledBack)
            committed =
                expectRight (commit (expectRight (releaseSavepoint (expectRight (createSavepoint (expectRight (insert commitBackend "users" (row [("id", SqlInteger 3), ("name", SqlText "Lin")]))) "after_lin")) "after_lin")) third)
        backendCurrentTransaction committed `shouldBe` Nothing
        length (iteratorToList (expectRight (scan committed "users"))) `shouldBe` 3

    it "stores triggers and version fields" $ do
        let backend = expectRight users
            initial = backendSchemaVersion backend
            trigger = triggerDef "users_ai" "users" "after" "insert" "SELECT 1"
            backend' = expectRight (createTrigger backend trigger)
        backendSchemaVersion backend' `shouldSatisfy` (> initial)
        triggerName (expectFirst (listTriggers backend' "users")) `shouldBe` "users_ai"
        triggerTiming (expectFirst (listTriggers backend' "users")) `shouldBe` "AFTER"
        errorKindOf (createTrigger backend' trigger) `shouldBe` Just "TriggerAlreadyExists"

        let backend'' = backend' { backendUserVersion = 7 }
        backendUserVersion backend'' `shouldBe` 7

        let backend''' = expectRight (dropTrigger backend'' "users_ai" False)
        listTriggers backend''' "users" `shouldBe` []
        dropTrigger backend''' "users_ai" True `shouldBe` Right backend'''
        errorKindOf (dropTrigger backend''' "users_ai" False) `shouldBe` Just "TriggerNotFound"

users :: Either BackendError InMemoryBackend
users = do
    backend <- createTable newBackend "users"
        [ (defaultColumnDef "id" "INTEGER") { columnPrimaryKey = True }
        , (defaultColumnDef "name" "TEXT") { columnNotNull = True }
        , (defaultColumnDef "email" "TEXT") { columnUnique = True }
        ]
        False
    backend' <- insert backend "users" (row [("id", SqlInteger 1), ("name", SqlText "Ada"), ("email", SqlText "ada@example.test")])
    insert backend' "users" (row [("id", SqlInteger 2), ("name", SqlText "Grace")])

row :: [(String, SqlValue)] -> Row
row = Map.fromList

expectRight :: (Show err) => Either err value -> value
expectRight result =
    case result of
        Left failure -> error ("expected Right, got Left: " ++ show failure)
        Right value -> value

expectFirst :: [value] -> value
expectFirst values =
    case values of
        [] -> error "expected a non-empty list"
        value:_ -> value

errorKindOf :: Either BackendError value -> Maybe String
errorKindOf result =
    case result of
        Left failure -> Just (errorKind failure)
        Right _ -> Nothing
