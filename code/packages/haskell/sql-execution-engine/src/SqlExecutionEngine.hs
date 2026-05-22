module SqlExecutionEngine
    ( SqlValue(..)
    , Row
    , DataSource(..)
    , QueryResult(..)
    , ExecutionResult(..)
    , SqlExecutionError(..)
    , InMemoryDataSource
    , emptyInMemoryDataSource
    , addTable
    , inMemoryDataSource
    , execute
    , tryExecute
    ) where

import Data.Char (isAlpha, isAlphaNum, isDigit, isSpace, toLower, toUpper)
import Data.List (dropWhileEnd, isPrefixOf, isSuffixOf, sort, sortBy)
import qualified Data.Map.Strict as Map
import Data.Maybe (mapMaybe)
import Text.Read (readMaybe)

data SqlValue
    = SqlNull
    | SqlBool Bool
    | SqlInteger Integer
    | SqlReal Double
    | SqlText String
    deriving (Eq, Ord, Show)

type Row = Map.Map String SqlValue

data DataSource = DataSource
    { dataSourceSchema :: String -> Either SqlExecutionError [String]
    , dataSourceScan :: String -> Either SqlExecutionError [Row]
    }

data QueryResult = QueryResult
    { resultColumns :: [String]
    , resultRows :: [[SqlValue]]
    } deriving (Eq, Show)

data ExecutionResult = ExecutionResult
    { executionOk :: Bool
    , executionResult :: Maybe QueryResult
    , executionError :: Maybe String
    } deriving (Eq, Show)

data SqlExecutionError = SqlExecutionError
    { sqlExecutionMessage :: String
    } deriving (Eq, Show)

data InMemoryDataSource = InMemoryDataSource
    { memorySchemas :: Map.Map String [String]
    , memoryTables :: Map.Map String [Row]
    } deriving (Eq, Show)

emptyInMemoryDataSource :: InMemoryDataSource
emptyInMemoryDataSource = InMemoryDataSource Map.empty Map.empty

addTable :: String -> [String] -> [Row] -> InMemoryDataSource -> InMemoryDataSource
addTable name columns rows source =
    source
        { memorySchemas = Map.insert name columns (memorySchemas source)
        , memoryTables = Map.insert name rows (memoryTables source)
        }

inMemoryDataSource :: InMemoryDataSource -> DataSource
inMemoryDataSource source =
    DataSource
        { dataSourceSchema = \name ->
            maybe (Left (sqlError ("table not found: " ++ name))) Right (Map.lookup name (memorySchemas source))
        , dataSourceScan = \name ->
            maybe (Left (sqlError ("table not found: " ++ name))) Right (Map.lookup name (memoryTables source))
        }

execute :: String -> DataSource -> Either SqlExecutionError QueryResult
execute sql source = do
    statement <- parseStatementFromSql sql
    executeSelect statement source

tryExecute :: String -> DataSource -> ExecutionResult
tryExecute sql source =
    case execute sql source of
        Left failure -> ExecutionResult False Nothing (Just (sqlExecutionMessage failure))
        Right result -> ExecutionResult True (Just result) Nothing

executeSelect :: SelectStatement -> DataSource -> Either SqlExecutionError QueryResult
executeSelect statement source = do
    initialRows <- scanTable source (tableName (selectFrom statement)) (tableAlias (selectFrom statement))
    joinedRows <- foldl (applyJoinStep source) (Right initialRows) (selectJoins statement)
    let filteredRows =
            case selectWhere statement of
                Nothing -> joinedRows
                Just expression -> filterRows expression joinedRows
    frames <- makeFrames filteredRows statement
    let havingFrames =
            case selectHaving statement of
                Nothing -> frames
                Just expression -> filterFrames expression frames
        orderedFrames =
            if null (selectOrderBy statement)
                then havingFrames
                else sortBy (compareFrameOrder (selectOrderBy statement)) havingFrames
    projected <- project orderedFrames statement
    let distinctRows =
            if selectDistinct statement
                then distinct (resultRows projected)
                else resultRows projected
        from = max 0 (maybe 0 id (selectOffset statement))
        count = maybe (length distinctRows - from) (max 0) (selectLimit statement)
        pagedRows =
            if from >= length distinctRows
                then []
                else take count (drop from distinctRows)
    Right projected { resultRows = pagedRows }
  where
    filterRows expression rows =
        filter (\row -> either (const False) truthy (eval expression (rowValues row) Nothing)) rows
    filterFrames expression frames =
        filter (\frame -> either (const False) truthy (eval expression (rowValues (frameRow frame)) (frameGroupRows frame))) frames

scanTable :: DataSource -> String -> String -> Either SqlExecutionError [RowContext]
scanTable source name alias = do
    columns <- dataSourceSchema source name
    rows <- dataSourceScan source name
    Right (map (materialize columns) rows)
  where
    materialize columns raw =
        RowContext (foldl addColumn Map.empty columns)
      where
        addColumn acc column =
            let value = Map.findWithDefault SqlNull column raw
            in Map.insert column value
                (Map.insert (alias ++ "." ++ column) value
                    (Map.insert (name ++ "." ++ column) value acc))

applyJoinStep :: DataSource -> Either SqlExecutionError [RowContext] -> JoinDef -> Either SqlExecutionError [RowContext]
applyJoinStep _ (Left failure) _ = Left failure
applyJoinStep source (Right leftRows) join = do
    rightRows <- scanTable source (tableName (joinTable join)) (tableAlias (joinTable join))
    Right (applyJoin leftRows rightRows join)

applyJoin :: [RowContext] -> [RowContext] -> JoinDef -> [RowContext]
applyJoin leftRows rightRows join
    | joinType join == "CROSS" = [mergeRows left right | left <- leftRows, right <- rightRows]
    | otherwise = concatMap joinLeft leftRows
  where
    joinLeft left =
        let matches = [merged | right <- rightRows, let merged = mergeRows left right, joinMatches merged]
        in if null matches && joinType join == "LEFT" then [left] else matches

    joinMatches merged =
        case joinOn join of
            Nothing -> True
            Just expression -> either (const False) truthy (eval expression (rowValues merged) Nothing)

makeFrames :: [RowContext] -> SelectStatement -> Either SqlExecutionError [RowFrame]
makeFrames rows statement
    | not grouped && not aggregated = Right (map (\row -> RowFrame row Nothing) rows)
    | not grouped =
        let row = case rows of
                [] -> RowContext Map.empty
                (firstRow:_) -> firstRow
        in Right [RowFrame row (Just rows)]
    | otherwise = Right (map frameGroup (Map.elems groups))
  where
    grouped = not (null (selectGroupBy statement))
    aggregated =
        any (hasAggregate . itemExpression) (selectItems statement) ||
        maybe False hasAggregate (selectHaving statement)
    groups = foldl addGroup Map.empty rows
    addGroup acc row =
        let key = map (\expression -> either (const SqlNull) id (eval expression (rowValues row) Nothing)) (selectGroupBy statement)
        in Map.insertWith (++) key [row] acc
    frameGroup groupRows =
        let row = case groupRows of
                [] -> RowContext Map.empty
                (firstRow:_) -> firstRow
        in RowFrame row (Just groupRows)

project :: [RowFrame] -> SelectStatement -> Either SqlExecutionError QueryResult
project frames statement =
    case selectItems statement of
        [SelectItem Star Nothing] ->
            let columns =
                    case frames of
                        [] -> []
                        (frame:_) -> sort (filter (not . isQualified) (Map.keys (rowValues (frameRow frame))))
                rows = [[Map.findWithDefault SqlNull column (rowValues (frameRow frame)) | column <- columns] | frame <- frames]
            in Right (QueryResult columns rows)
        items -> do
            rows <- mapM (projectFrame items) frames
            let columns = map (\item -> maybe (expressionLabel (itemExpression item)) id (itemAlias item)) items
            Right (QueryResult columns rows)
  where
    isQualified = ('.' `elem`)
    projectFrame items frame =
        mapM (\item -> eval (itemExpression item) (rowValues (frameRow frame)) (frameGroupRows frame)) items

compareFrameOrder :: [OrderItem] -> RowFrame -> RowFrame -> Ordering
compareFrameOrder items left right = firstNonEq comparisons
  where
    comparisons =
        [ let ordering = compareSqlValues
                (either (const SqlNull) id (eval (orderExpression item) (rowValues (frameRow left)) (frameGroupRows left)))
                (either (const SqlNull) id (eval (orderExpression item) (rowValues (frameRow right)) (frameGroupRows right)))
          in if orderDescending item then invert ordering else ordering
        | item <- items
        ]
    invert LT = GT
    invert GT = LT
    invert EQ = EQ
    firstNonEq [] = EQ
    firstNonEq (EQ:rest) = firstNonEq rest
    firstNonEq (ordering:_) = ordering

eval :: Expr -> Row -> Maybe [RowContext] -> Either SqlExecutionError SqlValue
eval expression row groupRows =
    case expression of
        Literal value -> Right value
        NullValue -> Right SqlNull
        Column table column ->
            Right (resolveColumn table column row)
        Star -> Right SqlNull
        Unary operator inner -> do
            value <- eval inner row groupRows
            case operator of
                "NOT" -> Right (if value == SqlNull then SqlNull else SqlBool (not (truthy value)))
                "-" -> if value == SqlNull then Right SqlNull else SqlReal . negate <$> asDouble value
                _ -> Left (sqlError ("unknown unary operator: " ++ operator))
        Binary operator left right -> evalBinary operator left right row groupRows
        IsNull inner negated -> do
            value <- eval inner row groupRows
            let result = value == SqlNull
            Right (SqlBool (if negated then not result else result))
        Between inner lower upperBound negated -> do
            value <- eval inner row groupRows
            lo <- eval lower row groupRows
            hi <- eval upperBound row groupRows
            if value == SqlNull || lo == SqlNull || hi == SqlNull
                then Right SqlNull
                else do
                    let result = compareSqlValues value lo /= LT && compareSqlValues value hi /= GT
                    Right (SqlBool (if negated then not result else result))
        InList inner values negated -> do
            value <- eval inner row groupRows
            if value == SqlNull
                then Right SqlNull
                else do
                    options <- mapM (\item -> eval item row groupRows) values
                    let found = any (\option -> option /= SqlNull && sqlEquals value option) options
                    Right (SqlBool (if negated then not found else found))
        Like inner patternExpr negated -> do
            value <- eval inner row groupRows
            patternValue <- eval patternExpr row groupRows
            if value == SqlNull || patternValue == SqlNull
                then Right SqlNull
                else do
                    let result = like (valueText value) (valueText patternValue)
                    Right (SqlBool (if negated then not result else result))
        Function name args -> evalFunction name args row groupRows

evalBinary :: String -> Expr -> Expr -> Row -> Maybe [RowContext] -> Either SqlExecutionError SqlValue
evalBinary operator leftExpr rightExpr row groupRows
    | operator == "AND" = do
        left <- eval leftExpr row groupRows
        if left /= SqlNull && not (truthy left)
            then Right (SqlBool False)
            else do
                right <- eval rightExpr row groupRows
                if right /= SqlNull && not (truthy right)
                    then Right (SqlBool False)
                    else Right (if left == SqlNull || right == SqlNull then SqlNull else SqlBool True)
    | operator == "OR" = do
        left <- eval leftExpr row groupRows
        if left /= SqlNull && truthy left
            then Right (SqlBool True)
            else do
                right <- eval rightExpr row groupRows
                if right /= SqlNull && truthy right
                    then Right (SqlBool True)
                    else Right (if left == SqlNull || right == SqlNull then SqlNull else SqlBool False)
    | otherwise = do
        left <- eval leftExpr row groupRows
        right <- eval rightExpr row groupRows
        if left == SqlNull || right == SqlNull
            then Right SqlNull
            else evalNonNullBinary operator left right

evalNonNullBinary :: String -> SqlValue -> SqlValue -> Either SqlExecutionError SqlValue
evalNonNullBinary operator left right =
    case operator of
        "+" -> SqlReal <$> numeric2 (+)
        "-" -> SqlReal <$> numeric2 (-)
        "*" -> SqlReal <$> numeric2 (*)
        "/" -> SqlReal <$> numeric2 (/)
        "%" -> do
            leftNumber <- asDouble left
            rightNumber <- asDouble right
            Right (SqlReal (leftNumber - fromIntegral (truncate (leftNumber / rightNumber) :: Integer) * rightNumber))
        "=" -> Right (SqlBool (sqlEquals left right))
        "!=" -> Right (SqlBool (not (sqlEquals left right)))
        "<>" -> Right (SqlBool (not (sqlEquals left right)))
        "<" -> Right (SqlBool (compareSqlValues left right == LT))
        ">" -> Right (SqlBool (compareSqlValues left right == GT))
        "<=" -> Right (SqlBool (compareSqlValues left right /= GT))
        ">=" -> Right (SqlBool (compareSqlValues left right /= LT))
        _ -> Left (sqlError ("unknown operator: " ++ operator))
  where
    numeric2 operation = do
        leftNumber <- asDouble left
        rightNumber <- asDouble right
        Right (operation leftNumber rightNumber)

evalFunction :: String -> [Expr] -> Row -> Maybe [RowContext] -> Either SqlExecutionError SqlValue
evalFunction rawName args row groupRows =
    if upperName `elem` ["COUNT", "SUM", "AVG", "MIN", "MAX"]
        then evalAggregate upperName args groupRows
        else do
            value <-
                case args of
                    [] -> Right SqlNull
                    (firstArg:_) -> eval firstArg row groupRows
            if value == SqlNull
                then Right SqlNull
                else evalScalar upperName value
  where
    upperName = upper rawName

evalAggregate :: String -> [Expr] -> Maybe [RowContext] -> Either SqlExecutionError SqlValue
evalAggregate name args maybeRows =
    case maybeRows of
        Nothing -> Left (sqlError ("aggregate used outside grouped context: " ++ name))
        Just rows ->
            case name of
                "COUNT" ->
                    case args of
                        [Star] -> Right (SqlInteger (toInteger (length rows)))
                        [] -> Right (SqlInteger (toInteger (length rows)))
                        (arg:_) -> do
                            values <- mapM (\row -> eval arg (rowValues row) Nothing) rows
                            Right (SqlInteger (toInteger (length (filter (/= SqlNull) values))))
                "SUM" -> aggregateNumbers rows args sum
                "AVG" -> do
                    values <- aggregateValues rows args
                    if null values
                        then Right SqlNull
                        else do
                            numbers <- mapM asDouble values
                            Right (SqlReal (sum numbers / fromIntegral (length numbers)))
                "MIN" -> aggregateMinMax rows args minimumBySql
                "MAX" -> aggregateMinMax rows args maximumBySql
                _ -> Left (sqlError ("unknown aggregate: " ++ name))

aggregateNumbers :: [RowContext] -> [Expr] -> ([Double] -> Double) -> Either SqlExecutionError SqlValue
aggregateNumbers rows args operation = do
    values <- aggregateValues rows args
    if null values
        then Right SqlNull
        else do
            numbers <- mapM asDouble values
            Right (SqlReal (operation numbers))

aggregateValues :: [RowContext] -> [Expr] -> Either SqlExecutionError [SqlValue]
aggregateValues rows args =
    case args of
        [] -> Left (sqlError "aggregate requires an argument")
        (arg:_) -> do
            values <- mapM (\row -> eval arg (rowValues row) Nothing) rows
            Right (filter (/= SqlNull) values)

aggregateMinMax :: [RowContext] -> [Expr] -> ([SqlValue] -> SqlValue) -> Either SqlExecutionError SqlValue
aggregateMinMax rows args operation = do
    values <- aggregateValues rows args
    Right (if null values then SqlNull else operation values)

minimumBySql :: [SqlValue] -> SqlValue
minimumBySql = foldl1 (\best next -> if compareSqlValues next best == LT then next else best)

maximumBySql :: [SqlValue] -> SqlValue
maximumBySql = foldl1 (\best next -> if compareSqlValues next best == GT then next else best)

evalScalar :: String -> SqlValue -> Either SqlExecutionError SqlValue
evalScalar name value =
    case name of
        "UPPER" -> Right (SqlText (map toUpper (valueText value)))
        "LOWER" -> Right (SqlText (map toLower (valueText value)))
        "LENGTH" -> Right (SqlInteger (toInteger (length (valueText value))))
        _ -> Left (sqlError ("unknown function: " ++ name))

hasAggregate :: Expr -> Bool
hasAggregate expression =
    case expression of
        Function name args -> upper name `elem` ["COUNT", "SUM", "AVG", "MIN", "MAX"] || any hasAggregate args
        Binary _ left right -> hasAggregate left || hasAggregate right
        Unary _ inner -> hasAggregate inner
        IsNull inner _ -> hasAggregate inner
        Between inner lower upperBound _ -> any hasAggregate [inner, lower, upperBound]
        InList inner values _ -> hasAggregate inner || any hasAggregate values
        Like inner patternExpr _ -> hasAggregate inner || hasAggregate patternExpr
        _ -> False

resolveColumn :: Maybe String -> String -> Row -> SqlValue
resolveColumn table column row =
    case table of
        Just tableName -> Map.findWithDefault SqlNull (tableName ++ "." ++ column) row
        Nothing ->
            case Map.lookup column row of
                Just value -> value
                Nothing ->
                    maybe SqlNull snd (firstMatching (\(key, _) -> ("." ++ column) `isSuffixOf` key) (Map.toList row))

truthy :: SqlValue -> Bool
truthy value =
    case value of
        SqlNull -> False
        SqlBool flag -> flag
        SqlInteger number -> number /= 0
        SqlReal number -> number /= 0.0
        SqlText text -> not (null text)

sqlEquals :: SqlValue -> SqlValue -> Bool
sqlEquals SqlNull SqlNull = True
sqlEquals (SqlInteger left) (SqlReal right) = fromInteger left == right
sqlEquals (SqlReal left) (SqlInteger right) = left == fromInteger right
sqlEquals left right = left == right

compareSqlValues :: SqlValue -> SqlValue -> Ordering
compareSqlValues left right =
    case compare (rank left) (rank right) of
        EQ -> compareSameRank left right
        other -> other
  where
    rank SqlNull = (0 :: Int)
    rank (SqlBool _) = 1
    rank (SqlInteger _) = 2
    rank (SqlReal _) = 2
    rank (SqlText _) = 3
    compareSameRank SqlNull SqlNull = EQ
    compareSameRank (SqlBool a) (SqlBool b) = compare a b
    compareSameRank (SqlInteger a) (SqlInteger b) = compare a b
    compareSameRank (SqlInteger a) (SqlReal b) = compare (fromInteger a :: Double) b
    compareSameRank (SqlReal a) (SqlInteger b) = compare a (fromInteger b :: Double)
    compareSameRank (SqlReal a) (SqlReal b) = compare a b
    compareSameRank (SqlText a) (SqlText b) = compare a b
    compareSameRank _ _ = EQ

asDouble :: SqlValue -> Either SqlExecutionError Double
asDouble value =
    case value of
        SqlInteger number -> Right (fromInteger number)
        SqlReal number -> Right number
        SqlText text ->
            maybe (Left (sqlError ("expected numeric value: " ++ text))) Right (readMaybe text)
        SqlBool True -> Right 1.0
        SqlBool False -> Right 0.0
        SqlNull -> Left (sqlError "expected numeric value: NULL")

valueText :: SqlValue -> String
valueText value =
    case value of
        SqlNull -> "null"
        SqlBool True -> "true"
        SqlBool False -> "false"
        SqlInteger number -> show number
        SqlReal number -> show number
        SqlText text -> text

expressionLabel :: Expr -> String
expressionLabel expression =
    case expression of
        Column _ name -> name
        Function name [Star] -> upper name ++ "(*)"
        Function name _ -> upper name ++ "(...)"
        Literal value -> valueText value
        NullValue -> "null"
        _ -> "?"

like :: String -> String -> Bool
like value pattern = match pattern value
  where
    match [] [] = True
    match [] _ = False
    match ('%':restPattern) text =
        match restPattern text ||
            case text of
                [] -> False
                (_:restText) -> match ('%':restPattern) restText
    match ('_':restPattern) (_:restText) = match restPattern restText
    match ('_':_) [] = False
    match (patternChar:restPattern) (textChar:restText) =
        patternChar == textChar && match restPattern restText
    match _ [] = False

distinct :: (Eq a) => [a] -> [a]
distinct = go []
  where
    go _ [] = []
    go seen (value:rest)
        | value `elem` seen = go seen rest
        | otherwise = value : go (value:seen) rest

mergeRows :: RowContext -> RowContext -> RowContext
mergeRows left right = RowContext (Map.union (rowValues right) (rowValues left))

firstMatching :: (a -> Bool) -> [a] -> Maybe a
firstMatching _ [] = Nothing
firstMatching predicate (value:rest)
    | predicate value = Just value
    | otherwise = firstMatching predicate rest

sqlError :: String -> SqlExecutionError
sqlError = SqlExecutionError

data SelectStatement = SelectStatement
    { selectDistinct :: Bool
    , selectItems :: [SelectItem]
    , selectFrom :: TableRef
    , selectJoins :: [JoinDef]
    , selectWhere :: Maybe Expr
    , selectGroupBy :: [Expr]
    , selectHaving :: Maybe Expr
    , selectOrderBy :: [OrderItem]
    , selectLimit :: Maybe Int
    , selectOffset :: Maybe Int
    } deriving (Eq, Show)

data SelectItem = SelectItem
    { itemExpression :: Expr
    , itemAlias :: Maybe String
    } deriving (Eq, Show)

data TableRef = TableRef
    { tableName :: String
    , tableAlias :: String
    } deriving (Eq, Show)

data JoinDef = JoinDef
    { joinType :: String
    , joinTable :: TableRef
    , joinOn :: Maybe Expr
    } deriving (Eq, Show)

data OrderItem = OrderItem
    { orderExpression :: Expr
    , orderDescending :: Bool
    } deriving (Eq, Show)

data RowContext = RowContext
    { rowValues :: Row
    } deriving (Eq, Show)

data RowFrame = RowFrame
    { frameRow :: RowContext
    , frameGroupRows :: Maybe [RowContext]
    } deriving (Eq, Show)

data Expr
    = Literal SqlValue
    | NullValue
    | Column (Maybe String) String
    | Star
    | Unary String Expr
    | Binary String Expr Expr
    | IsNull Expr Bool
    | Between Expr Expr Expr Bool
    | InList Expr [Expr] Bool
    | Like Expr Expr Bool
    | Function String [Expr]
    deriving (Eq, Show)

parseStatementFromSql :: String -> Either SqlExecutionError SelectStatement
parseStatementFromSql sql =
    case runParser parseStatement (ParserState (tokenize sql) 0) of
        Left failure -> Left failure
        Right (statement, _) -> Right statement

newtype Parser a = Parser
    { runParser :: ParserState -> Either SqlExecutionError (a, ParserState)
    }

data ParserState = ParserState
    { parserTokens :: [Token]
    , parserPosition :: Int
    } deriving (Eq, Show)

instance Functor Parser where
    fmap f parser = Parser $ \state ->
        case runParser parser state of
            Left failure -> Left failure
            Right (value, state') -> Right (f value, state')

instance Applicative Parser where
    pure value = Parser $ \state -> Right (value, state)
    fParser <*> valueParser = Parser $ \state ->
        case runParser fParser state of
            Left failure -> Left failure
            Right (f, state') ->
                case runParser valueParser state' of
                    Left failure -> Left failure
                    Right (value, state'') -> Right (f value, state'')

instance Monad Parser where
    parser >>= next = Parser $ \state ->
        case runParser parser state of
            Left failure -> Left failure
            Right (value, state') -> runParser (next value) state'

parseStatement :: Parser SelectStatement
parseStatement = do
    expectKeyword "SELECT"
    distinctFlag <- matchKeyword "DISTINCT"
    _ <- matchKeyword "ALL"
    items <- parseSelectList
    expectKeyword "FROM"
    fromRef <- parseTableRef
    joins <- parseJoins
    whereExpr <- optionalAfterKeyword "WHERE" parseExpression
    groupBy <- do
        matched <- matchKeyword "GROUP"
        if matched
            then expectKeyword "BY" >> parseExpressionList
            else pure []
    havingExpr <- optionalAfterKeyword "HAVING" parseExpression
    orderBy <- do
        matched <- matchKeyword "ORDER"
        if matched
            then expectKeyword "BY" >> parseOrderList
            else pure []
    limitValue <- optionalAfterKeyword "LIMIT" parseNumberAsInt
    offsetValue <- optionalAfterKeyword "OFFSET" parseNumberAsInt
    _ <- matchSymbol ";"
    expectKind End
    pure (SelectStatement distinctFlag items fromRef joins whereExpr groupBy havingExpr orderBy limitValue offsetValue)

parseSelectList :: Parser [SelectItem]
parseSelectList = commaSeparated parseItem
  where
    parseItem = do
        star <- matchSymbol "*"
        if star
            then pure (SelectItem Star Nothing)
            else do
                expression <- parseExpression
                alias <- do
                    asAlias <- matchKeyword "AS"
                    if asAlias
                        then Just <$> expectIdentifier
                        else do
                            token <- peekToken
                            if tokenKind token == Ident
                                then Just <$> (advanceToken >>= pure . tokenValue)
                                else pure Nothing
                pure (SelectItem expression alias)

parseTableRef :: Parser TableRef
parseTableRef = do
    name <- expectIdentifier
    alias <- do
        asAlias <- matchKeyword "AS"
        if asAlias
            then expectIdentifier
            else do
                token <- peekToken
                if tokenKind token == Ident
                    then advanceToken >>= pure . tokenValue
                    else pure name
    pure (TableRef name alias)

parseJoins :: Parser [JoinDef]
parseJoins = do
    maybeJoin <- parseOneJoin
    case maybeJoin of
        Nothing -> pure []
        Just join -> do
            rest <- parseJoins
            pure (join:rest)
  where
    parseOneJoin = do
        joinTypeValue <- do
            inner <- matchKeyword "INNER"
            if inner
                then expectKeyword "JOIN" >> pure (Just "INNER")
                else do
                    left <- matchKeyword "LEFT"
                    if left
                        then do
                            _ <- matchKeyword "OUTER"
                            expectKeyword "JOIN"
                            pure (Just "LEFT")
                        else do
                            cross <- matchKeyword "CROSS"
                            if cross
                                then expectKeyword "JOIN" >> pure (Just "CROSS")
                                else do
                                    joined <- matchKeyword "JOIN"
                                    pure (if joined then Just "INNER" else Nothing)
        case joinTypeValue of
            Nothing -> pure Nothing
            Just kind -> do
                table <- parseTableRef
                onExpr <-
                    if kind == "CROSS"
                        then pure Nothing
                        else expectKeyword "ON" >> Just <$> parseExpression
                pure (Just (JoinDef kind table onExpr))

parseExpressionList :: Parser [Expr]
parseExpressionList = commaSeparated parseExpression

parseOrderList :: Parser [OrderItem]
parseOrderList = commaSeparated parseOrderItem
  where
    parseOrderItem = do
        expression <- parseExpression
        asc <- matchKeyword "ASC"
        descending <- if asc then pure False else matchKeyword "DESC"
        pure (OrderItem expression descending)

parseExpression :: Parser Expr
parseExpression = parseOr

parseOr :: Parser Expr
parseOr = chainKeyword "OR" parseAnd

parseAnd :: Parser Expr
parseAnd = chainKeyword "AND" parseNot

parseNot :: Parser Expr
parseNot = do
    matched <- matchKeyword "NOT"
    if matched then Unary "NOT" <$> parseNot else parseComparison

parseComparison :: Parser Expr
parseComparison = do
    left <- parseAdditive
    isMatched <- matchKeyword "IS"
    if isMatched
        then do
            negated <- matchKeyword "NOT"
            expectKeyword "NULL"
            pure (IsNull left negated)
        else do
            notMatched <- matchKeyword "NOT"
            if notMatched
                then parseNegatedComparison left
                else parsePositiveComparison left

parseNegatedComparison :: Expr -> Parser Expr
parseNegatedComparison left = do
    between <- matchKeyword "BETWEEN"
    if between
        then do
            lower <- parseAdditive
            expectKeyword "AND"
            Between left lower <$> parseAdditive <*> pure True
        else do
            inMatched <- matchKeyword "IN"
            if inMatched
                then InList left <$> parseInValues <*> pure True
                else do
                    likeMatched <- matchKeyword "LIKE"
                    if likeMatched
                        then Like left <$> parseAdditive <*> pure True
                        else failParser "expected BETWEEN, IN, or LIKE after NOT"

parsePositiveComparison :: Expr -> Parser Expr
parsePositiveComparison left = do
    between <- matchKeyword "BETWEEN"
    if between
        then do
            lower <- parseAdditive
            expectKeyword "AND"
            Between left lower <$> parseAdditive <*> pure False
        else do
            inMatched <- matchKeyword "IN"
            if inMatched
                then InList left <$> parseInValues <*> pure False
                else do
                    likeMatched <- matchKeyword "LIKE"
                    if likeMatched
                        then Like left <$> parseAdditive <*> pure False
                        else do
                            token <- peekToken
                            if tokenKind token == Symbol && tokenValue token `elem` ["=", "!=", "<>", "<", ">", "<=", ">="]
                                then do
                                    operator <- tokenValue <$> advanceToken
                                    Binary operator left <$> parseAdditive
                                else pure left

parseInValues :: Parser [Expr]
parseInValues = do
    expectSymbol "("
    values <- parseExpressionList
    expectSymbol ")"
    pure values

parseAdditive :: Parser Expr
parseAdditive = chainSymbol ["+", "-"] parseMultiplicative

parseMultiplicative :: Parser Expr
parseMultiplicative = chainSymbol ["*", "/", "%"] parseUnary

parseUnary :: Parser Expr
parseUnary = do
    negated <- matchSymbol "-"
    if negated then Unary "-" <$> parseUnary else parsePrimary

parsePrimary :: Parser Expr
parsePrimary = do
    parenthesized <- matchSymbol "("
    if parenthesized
        then do
            expression <- parseExpression
            expectSymbol ")"
            pure expression
        else parsePrimaryToken

parsePrimaryToken :: Parser Expr
parsePrimaryToken = do
    token <- peekToken
    case tokenKind token of
        Number -> do
            _ <- advanceToken
            case numberValue (tokenValue token) of
                Nothing -> failParser ("invalid number: " ++ tokenValue token)
                Just value -> pure (Literal value)
        StringToken -> Literal . SqlText . tokenValue <$> advanceToken
        _ -> do
            nullMatched <- matchKeyword "NULL"
            if nullMatched
                then pure NullValue
                else do
                    trueMatched <- matchKeyword "TRUE"
                    if trueMatched
                        then pure (Literal (SqlBool True))
                        else do
                            falseMatched <- matchKeyword "FALSE"
                            if falseMatched
                                then pure (Literal (SqlBool False))
                                else do
                                    starMatched <- matchSymbol "*"
                                    if starMatched
                                        then pure Star
                                        else parseIdentifierPrimary

parseIdentifierPrimary :: Parser Expr
parseIdentifierPrimary = do
    token <- advanceToken
    if tokenKind token == Ident || tokenKind token == Keyword
        then do
            let name = tokenValue token
            functionCall <- matchSymbol "("
            if functionCall
                then do
                    args <- do
                        close <- matchSymbol ")"
                        if close
                            then pure []
                            else do
                                star <- matchSymbol "*"
                                values <- if star then pure [Star] else parseExpressionList
                                expectSymbol ")"
                                pure values
                    pure (Function name args)
                else do
                    qualified <- matchSymbol "."
                    if qualified
                        then Column (Just name) <$> expectIdentifier
                        else pure (Column Nothing name)
        else failParser ("unexpected token: " ++ tokenValue token)

chainKeyword :: String -> Parser Expr -> Parser Expr
chainKeyword keyword operandParser = do
    left <- operandParser
    rest left
  where
    rest left = do
        matched <- matchKeyword keyword
        if matched
            then operandParser >>= rest . Binary keyword left
            else pure left

chainSymbol :: [String] -> Parser Expr -> Parser Expr
chainSymbol operators operandParser = do
    left <- operandParser
    rest left
  where
    rest left = do
        token <- peekToken
        if tokenKind token == Symbol && tokenValue token `elem` operators
            then do
                operator <- tokenValue <$> advanceToken
                operandParser >>= rest . Binary operator left
            else pure left

commaSeparated :: Parser a -> Parser [a]
commaSeparated itemParser = do
    firstItem <- itemParser
    rest <- more
    pure (firstItem:rest)
  where
    more = do
        comma <- matchSymbol ","
        if comma
            then do
                nextItem <- itemParser
                rest <- more
                pure (nextItem:rest)
            else pure []

optionalAfterKeyword :: String -> Parser a -> Parser (Maybe a)
optionalAfterKeyword keyword parser = do
    matched <- matchKeyword keyword
    if matched then Just <$> parser else pure Nothing

parseNumberAsInt :: Parser Int
parseNumberAsInt = do
    token <- advanceToken
    if tokenKind token /= Number
        then failParser "expected number"
        else maybe (failParser ("invalid number: " ++ tokenValue token)) pure (readMaybe (tokenValue token))

numberValue :: String -> Maybe SqlValue
numberValue text =
    if '.' `elem` text
        then SqlReal <$> (readMaybe text :: Maybe Double)
        else SqlInteger <$> (readMaybe text :: Maybe Integer)

peekToken :: Parser Token
peekToken = Parser $ \state ->
    case drop (parserPosition state) (parserTokens state) of
        [] -> Right (Token End "", state)
        (token:_) -> Right (token, state)

advanceToken :: Parser Token
advanceToken = Parser $ \state ->
    case drop (parserPosition state) (parserTokens state) of
        [] -> Right (Token End "", state)
        (token:_) ->
            let nextPosition =
                    if tokenKind token == End
                        then parserPosition state
                        else parserPosition state + 1
            in Right (token, state { parserPosition = nextPosition })

expectIdentifier :: Parser String
expectIdentifier = do
    token <- advanceToken
    if tokenKind token == Ident || tokenKind token == Keyword
        then pure (tokenValue token)
        else failParser "expected identifier"

expectKind :: TokenKind -> Parser ()
expectKind kind = do
    token <- advanceToken
    if tokenKind token == kind
        then pure ()
        else failParser ("expected " ++ show kind ++ ", got " ++ show (tokenKind token))

expectKeyword :: String -> Parser ()
expectKeyword keyword = do
    matched <- matchKeyword keyword
    if matched then pure () else failParser ("expected " ++ keyword)

matchKeyword :: String -> Parser Bool
matchKeyword keyword = do
    token <- peekToken
    if tokenKind token == Keyword && upper (tokenValue token) == upper keyword
        then advanceToken >> pure True
        else pure False

expectSymbol :: String -> Parser ()
expectSymbol symbol = do
    matched <- matchSymbol symbol
    if matched then pure () else failParser ("expected " ++ symbol)

matchSymbol :: String -> Parser Bool
matchSymbol symbol = do
    token <- peekToken
    if tokenKind token == Symbol && tokenValue token == symbol
        then advanceToken >> pure True
        else pure False

failParser :: String -> Parser a
failParser message = Parser $ \state ->
    Left (sqlError (message ++ " near token " ++ show (parserPosition state)))

data TokenKind = Ident | Keyword | Number | StringToken | Symbol | End
    deriving (Eq, Show)

data Token = Token
    { tokenKind :: TokenKind
    , tokenValue :: String
    } deriving (Eq, Show)

keywords :: [String]
keywords =
    [ "SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET"
    , "DISTINCT", "ALL", "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS"
    , "ON", "AS", "AND", "OR", "NOT", "IS", "NULL", "IN", "BETWEEN", "LIKE", "TRUE"
    , "FALSE", "ASC", "DESC", "COUNT", "SUM", "AVG", "MIN", "MAX", "UPPER", "LOWER", "LENGTH"
    ]

tokenize :: String -> [Token]
tokenize sql = go sql
  where
    go [] = [Token End ""]
    go text@(ch:rest)
        | isSpace ch = go rest
        | "--" `isPrefixOf` text = go (dropWhile (/= '\n') (drop 2 text))
        | ch == '\'' =
            let (value, remaining) = readStringLiteral rest
            in Token StringToken value : go remaining
        | isDigit ch || startsDottedNumber ch rest =
            let (numberText, remaining) = span (\c -> isDigit c || c == '.') text
            in Token Number numberText : go remaining
        | isAlpha ch || ch == '_' =
            let (identifier, remaining) = span (\c -> isAlphaNum c || c == '_') text
                kind = if upper identifier `elem` keywords then Keyword else Ident
            in Token kind identifier : go remaining
        | ch == '"' || ch == '`' =
            let (identifier, remaining) = readQuotedIdentifier ch rest
            in Token Ident identifier : go remaining
        | otherwise =
            case twoCharacterSymbol text of
                Just symbol -> Token Symbol symbol : go (drop 2 text)
                Nothing ->
                    if ch `elem` "=<>+-*/%(),.;"
                        then Token Symbol [ch] : go rest
                        else go rest

readStringLiteral :: String -> (String, String)
readStringLiteral = go ""
  where
    go acc [] = (reverse acc, [])
    go acc ('\'':'\'':rest) = go ('\'':acc) rest
    go acc ('\'':rest) = (reverse acc, rest)
    go acc (ch:rest) = go (ch:acc) rest

readQuotedIdentifier :: Char -> String -> (String, String)
readQuotedIdentifier quote = go ""
  where
    go acc [] = (reverse acc, [])
    go acc (ch:rest)
        | ch == quote = (reverse acc, rest)
        | otherwise = go (ch:acc) rest

twoCharacterSymbol :: String -> Maybe String
twoCharacterSymbol text =
    case take 2 text of
        symbol | symbol `elem` ["!=", "<>", "<=", ">="] -> Just symbol
        _ -> Nothing

upper :: String -> String
upper = map toUpper

startsDottedNumber :: Char -> String -> Bool
startsDottedNumber ch rest =
    case rest of
        next:_ -> ch == '.' && isDigit next
        [] -> False
