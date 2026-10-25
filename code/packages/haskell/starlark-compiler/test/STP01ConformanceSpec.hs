{-# LANGUAGE OverloadedStrings #-}

module STP01ConformanceSpec (spec) where

import Control.Exception (evaluate)
import Control.Monad (unless)
import Data.Aeson (Value (..), eitherDecodeStrict')
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KeyMap
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as C8
import Data.Char (isDigit)
import Data.List (sortOn)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Scientific (toBoundedInteger)
import qualified Data.Set as Set
import Data.Set (Set)
import qualified Data.Text as Text
import Data.Text (Text)
import qualified Data.Text.Encoding as TextEncoding
import qualified Data.Vector as Vector
import Data.Word (Word8)
import StarlarkCompiler
import System.IO (IOMode (ReadMode), withBinaryFile)
import Test.Hspec

fixturePath :: FilePath
fixturePath = "../../../specs/fixtures/starlark-opcode-provider-v1/cases.json"

maxBytes :: Int
maxBytes = 131072

data Frame = ObjectFrame (Set Text) | ArrayFrame

data OpcodeRow = OpcodeRow
    { rowSymbol :: Text
    , rowByte :: Word8
    , rowCategory :: Text
    }
    deriving (Eq, Show)

data DecodeRow = DecodeRow
    { decodeByte :: Word8
    , decodeSymbol :: Maybe Text
    , decodeCategory :: Maybe Text
    }
    deriving (Eq, Show)

data OperatorRow = OperatorRow
    { operatorText :: Text
    , operatorSymbol :: Text
    }
    deriving (Eq, Show)

data UnknownRow = UnknownRow
    { unknownGroup :: Text
    , unknownOperator :: Text
    }
    deriving (Eq, Show)

data Corpus = Corpus
    { corpusOpcodes :: [OpcodeRow]
    , corpusDecode :: [DecodeRow]
    , corpusOperators :: Map Text [OperatorRow]
    , corpusUnknown :: [UnknownRow]
    }
    deriving (Eq, Show)

data Counts = Counts
    { assignedCount :: Int
    , decodeCount :: Int
    , operatorCount :: Int
    , unknownCount :: Int
    , holeCount :: Int
    }
    deriving (Eq, Show)

spec :: Spec
spec = describe "STP01 shared corpus" $ do
    it "drives every production API through all 359 records" $ do
        raw <- readFixture
        corpus <- either fail pure (decodeCorpus raw)
        validateProvider corpus `shouldBe` Right (Counts 61 256 34 8 195)

    it "rejects hostile or ambiguous transport before comparison" $ do
        raw <- readFixture
        let malformed =
                [ ("duplicate", replaceOnce "\"schema_version\": 1" "\"schema_version\": 1,\"schema_version\":1" raw)
                , ("escaped duplicate", replaceOnce "\"profile\":" "\"pro\\u0066ile\":\"standalone-provider-v1\",\"profile\":" raw)
                , ("float", replaceOnce "\"byte\": 1" "\"byte\": 1.0" raw)
                , ("extra", replaceOnce "\"schema_version\": 1" "\"schema_version\": 1,\"extra\":0" raw)
                , ("wrong type", replaceOnce "\"profile\": \"standalone-provider-v1\"" "\"profile\": true" raw)
                , ("lone surrogate", replaceOnce "LoadConst" "\\ud800" raw)
                , ("invalid UTF-8", BS.cons 0xFF raw)
                , ("too deep", "[[[[[[[[[]]]]]]]]]")
                , ("trailing", raw <> " {}")
                , ("oversize", BS.replicate (maxBytes + 1) 0x20)
                ]
        mapM_ (\(name, candidate) ->
            unless (isLeft (decodeCorpus candidate))
                (expectationFailure ("accepted malformed transport: " <> name))) malformed

    it "rejects coordinated semantic corruption instead of learning it" $ do
        raw <- readFixture
        let changed = replaceAll "\"Add\"" "\"Counterfeit\"" raw
        corpus <- either fail pure (decodeCorpus changed)
        validateProvider corpus `shouldSatisfy` isLeft

    it "keeps the bounded file read independent from lazy IO" $ do
        raw <- readFixture
        BS.length raw `shouldSatisfy` (<= maxBytes)
        evaluate (BS.length raw) `shouldReturn` BS.length raw

readFixture :: IO BS.ByteString
readFixture = withBinaryFile fixturePath ReadMode $ \handle -> do
    -- Read at most one byte beyond the transport ceiling so an oversized
    -- fixture is rejected without first allocating or decoding the full file.
    raw <- BS.hGet handle (maxBytes + 1)
    if BS.length raw > maxBytes
        then expectationFailure "fixture exceeds 131072 bytes" >> pure BS.empty
        else pure raw

decodeCorpus :: BS.ByteString -> Either String Corpus
decodeCorpus raw = do
    preflight raw
    value <- eitherDecodeStrict' raw
    root <- exactObject ["schema_version", "profile", "opcodes", "decode", "operators", "unknown_lookups"] value
    version <- integerAt 1 =<< member "schema_version" root
    if version /= 1 then Left "schema version" else pure ()
    profile <- textAt 64 =<< member "profile" root
    if profile /= "standalone-provider-v1" then Left "profile" else pure ()
    opcodes <- exactArray 61 (member "opcodes" root) >>= traverse opcodeRow
    decodeRows <- exactArray 256 (member "decode" root) >>= traverse decodeRow
    operatorObject <- exactObject ["binary", "compare", "augmented_assign", "unary"] =<< member "operators" root
    operators <- traverseGroups operatorObject
    unknown <- exactArray 8 (member "unknown_lookups" root) >>= traverse unknownRow
    validateRoster opcodes decodeRows operators unknown
    pure (Corpus opcodes decodeRows operators unknown)

preflight :: BS.ByteString -> Either String ()
preflight raw
    | BS.length raw > maxBytes = Left "transport exceeds 131072 bytes"
    | BS.isPrefixOf (BS.pack [0xEF, 0xBB, 0xBF]) raw = Left "byte-order mark"
    | otherwise = do
        case TextEncoding.decodeUtf8' raw of
            Left failure -> Left (show failure)
            Right _ -> Right ()
        scan 0 []
  where
    total = BS.length raw

    scan index frames
        | index >= total = Right ()
        | otherwise =
            case BS.index raw index of
                0x22 -> do
                    end <- stringEnd raw (index + 1)
                    let next = skipSpace raw end
                    updated <-
                        if next < total && BS.index raw next == 0x3A
                            then recordKey (BS.take (end - index) (BS.drop index raw)) frames
                            else Right frames
                    scan end updated
                0x7B -> push (ObjectFrame Set.empty)
                0x5B -> push ArrayFrame
                0x7D -> popObject frames >>= scan (index + 1)
                0x5D -> popArray frames >>= scan (index + 1)
                byte
                    | byte == 0x2D || isDigitByte byte -> do
                        let end = numberEnd raw index
                        if validIntegerToken (C8.unpack (BS.take (end - index) (BS.drop index raw)))
                            then scan end frames
                            else Left "ambiguous numeric form"
                    | otherwise -> scan (index + 1) frames
      where
        push frame
            | length frames >= 8 = Left "container depth exceeds eight"
            | otherwise = scan (index + 1) (frame : frames)

stringEnd :: BS.ByteString -> Int -> Either String Int
stringEnd raw = go
  where
    total = BS.length raw
    go index
        | index >= total = Left "unterminated string"
        | BS.index raw index == 0x22 = Right (index + 1)
        | BS.index raw index /= 0x5C = go (index + 1)
        | index + 1 >= total = Left "truncated escape"
        | BS.index raw (index + 1) /= 0x75 = go (index + 2)
        | otherwise = do
            high <- hexUnit raw (index + 2)
            if high >= 0xD800 && high <= 0xDBFF
                then do
                    if index + 12 > total || BS.take 2 (BS.drop (index + 6) raw) /= "\\u"
                        then Left "unpaired high surrogate"
                        else do
                            low <- hexUnit raw (index + 8)
                            if low >= 0xDC00 && low <= 0xDFFF
                                then go (index + 12)
                                else Left "invalid surrogate pair"
                else
                    if high >= 0xDC00 && high <= 0xDFFF
                        then Left "unpaired low surrogate"
                        else go (index + 6)

hexUnit :: BS.ByteString -> Int -> Either String Int
hexUnit raw index
    | index + 4 > BS.length raw = Left "truncated Unicode escape"
    | otherwise = foldl step (Right 0) (C8.unpack (BS.take 4 (BS.drop index raw)))
  where
    step result char = do
        value <- result
        digit <- case char of
            _ | char >= '0' && char <= '9' -> Right (fromEnum char - fromEnum '0')
            _ | char >= 'a' && char <= 'f' -> Right (10 + fromEnum char - fromEnum 'a')
            _ | char >= 'A' && char <= 'F' -> Right (10 + fromEnum char - fromEnum 'A')
            _ -> Left "invalid Unicode escape"
        pure (value * 16 + digit)

recordKey :: BS.ByteString -> [Frame] -> Either String [Frame]
recordKey token (ObjectFrame keys : rest) = do
    key <- eitherDecodeStrict' token
    if Set.member key keys
        then Left "duplicate decoded object key"
        else Right (ObjectFrame (Set.insert key keys) : rest)
recordKey _ _ = Left "object key outside object"

popObject :: [Frame] -> Either String [Frame]
popObject (ObjectFrame _ : rest) = Right rest
popObject _ = Left "mismatched object close"

popArray :: [Frame] -> Either String [Frame]
popArray (ArrayFrame : rest) = Right rest
popArray _ = Left "mismatched array close"

skipSpace :: BS.ByteString -> Int -> Int
skipSpace raw = go
  where
    go index
        | index < BS.length raw && BS.index raw index `elem` [0x20, 0x09, 0x0A, 0x0D] = go (index + 1)
        | otherwise = index

numberEnd :: BS.ByteString -> Int -> Int
numberEnd raw = go
  where
    go index
        | index >= BS.length raw = index
        | BS.index raw index `elem` [0x20, 0x09, 0x0A, 0x0D, 0x2C, 0x5D, 0x7D] = index
        | otherwise = go (index + 1)

validIntegerToken :: String -> Bool
validIntegerToken ('-' : digits) = validDigits digits
validIntegerToken digits = validDigits digits

validDigits :: String -> Bool
validDigits "0" = True
validDigits (first : rest) = first >= '1' && first <= '9' && all isDigit rest
validDigits [] = False

isDigitByte :: Word8 -> Bool
isDigitByte byte = byte >= 0x30 && byte <= 0x39

exactObject :: [Text] -> Value -> Either String (KeyMap.KeyMap Value)
exactObject fields (Object object)
    | Set.fromList (map Key.toText (KeyMap.keys object)) == Set.fromList fields = Right object
    | otherwise = Left "closed object fields"
exactObject _ _ = Left "object required"

member :: Text -> KeyMap.KeyMap Value -> Either String Value
member name object = maybe (Left ("missing " <> Text.unpack name)) Right (KeyMap.lookup (Key.fromText name) object)

exactArray :: Int -> Either String Value -> Either String [Value]
exactArray count valueResult = do
    value <- valueResult
    case value of
        Array values | Vector.length values == count -> Right (Vector.toList values)
        _ -> Left "exact array length required"

textAt :: Int -> Value -> Either String Text
textAt limit (String text)
    | not (Text.null text) && Text.length text <= limit = Right text
textAt _ _ = Left "bounded non-empty text required"

integerAt :: Int -> Value -> Either String Int
integerAt upperBound (Number number) =
    case toBoundedInteger number :: Maybe Int of
        Just value | value >= 0 && value <= upperBound -> Right value
        _ -> Left "integer out of range"
integerAt _ _ = Left "integer required"

word8At :: Value -> Either String Word8
word8At value = fromIntegral <$> integerAt 255 value

nullableTextAt :: Int -> Value -> Either String (Maybe Text)
nullableTextAt _ Null = Right Nothing
nullableTextAt limit value = Just <$> textAt limit value

opcodeRow :: Value -> Either String OpcodeRow
opcodeRow value = do
    object <- exactObject ["symbol", "byte", "category"] value
    OpcodeRow
        <$> (textAt 64 =<< member "symbol" object)
        <*> (word8At =<< member "byte" object)
        <*> (textAt 64 =<< member "category" object)

decodeRow :: Value -> Either String DecodeRow
decodeRow value = do
    object <- exactObject ["byte", "symbol", "category"] value
    byte <- word8At =<< member "byte" object
    symbol <- nullableTextAt 64 =<< member "symbol" object
    category <- nullableTextAt 64 =<< member "category" object
    if (symbol == Nothing) /= (category == Nothing)
        then Left "decode null pair"
        else Right (DecodeRow byte symbol category)

operatorRow :: Value -> Either String OperatorRow
operatorRow value = do
    object <- exactObject ["operator", "symbol"] value
    OperatorRow
        <$> (textAt 32 =<< member "operator" object)
        <*> (textAt 64 =<< member "symbol" object)

unknownRow :: Value -> Either String UnknownRow
unknownRow value = do
    object <- exactObject ["group", "operator", "symbol"] value
    group <- textAt 64 =<< member "group" object
    operator <- textAt 32 =<< member "operator" object
    symbol <- member "symbol" object
    if symbol == Null then Right (UnknownRow group operator) else Left "unknown symbol must be null"

groupOrder :: [(Text, [Text])]
groupOrder =
    [ ("binary", ["+", "-", "*", "/", "//", "%", "**", "&", "|", "^", "<<", ">>"])
    , ("compare", ["==", "!=", "<", ">", "<=", ">=", "in", "not in"])
    , ("augmented_assign", ["+=", "-=", "*=", "/=", "//=", "%=", "**=", "&=", "|=", "^=", "<<=", ">>="])
    , ("unary", ["-", "~"])
    ]

unknownOrder :: [UnknownRow]
unknownOrder =
    [ UnknownRow "binary" "@", UnknownRow "binary" "++", UnknownRow "binary" "＋"
    , UnknownRow "compare" "IN", UnknownRow "compare" "not  in"
    , UnknownRow "augmented_assign" ":=", UnknownRow "unary" "+", UnknownRow "unary" "not"
    ]

traverseGroups :: KeyMap.KeyMap Value -> Either String (Map Text [OperatorRow])
traverseGroups object = Map.fromList <$> traverse parseGroup groupOrder
  where
    parseGroup (name, spellings) = do
        rows <- exactArray (length spellings) (member name object) >>= traverse operatorRow
        if map operatorText rows == spellings
            then Right (name, rows)
            else Left "operator roster or order"

validateRoster :: [OpcodeRow] -> [DecodeRow] -> Map Text [OperatorRow] -> [UnknownRow] -> Either String ()
validateRoster opcodes decodeRows operators unknown = do
    let bytes = map rowByte opcodes
        symbols = map rowSymbol opcodes
    if bytes /= map rowByte (sortOn rowByte opcodes) || length (Set.fromList bytes) /= 61
        then Left "ascending unique opcode bytes"
        else pure ()
    if length (Set.fromList symbols) /= 61 then Left "unique opcode symbols" else pure ()
    if map decodeByte decodeRows /= [0 .. 255] then Left "complete decode order" else pure ()
    if Map.keysSet operators /= Set.fromList (map fst groupOrder) then Left "operator groups" else pure ()
    if unknown /= unknownOrder then Left "unknown lookup roster" else pure ()

validateProvider :: Corpus -> Either String Counts
validateProvider corpus = do
    if length allOps /= 61 then Left "allOps count" else pure ()
    mapM_ compareAssigned (zip allOps (corpusOpcodes corpus))
    holes <- mapM compareDecode (corpusDecode corpus)
    mapM_ compareGroup groupOrder
    mapM_ compareUnknown (corpusUnknown corpus)
    let counts = Counts 61 256 (sum (map (length . snd) groupOrder)) 8 (length (filter id holes))
    if counts == Counts 61 256 34 8 195 then Right counts else Left "visit counts"
  where
    compareAssigned (actual, expected) = compareOp actual (rowByte expected) (rowSymbol expected) (rowCategory expected)
    compareDecode expected =
        case (decodeSymbol expected, decodeCategory expected) of
            (Nothing, Nothing) ->
                case opFromByte (decodeByte expected) of
                    Nothing -> Right True
                    Just _ -> Left "decode hole resolved"
            (Just symbol, Just category) -> do
                actual <- maybe (Left "assigned byte absent") Right (opFromByte (decodeByte expected))
                compareOp actual (decodeByte expected) symbol category
                Right False
            _ -> Left "partial decode null"
    compareGroup (name, _) = do
        rows <- maybe (Left "missing operator group") Right (Map.lookup name (corpusOperators corpus))
        let (table, lookupOne) = productionGroup name
        if Map.size table /= length rows then Left "operator map size" else pure ()
        mapM_ (compareOperator table lookupOne) rows
    compareOperator table lookupOne row = do
        fromMap <- maybe (Left "operator absent from map") Right (Map.lookup (Text.unpack (operatorText row)) table)
        direct <- maybe (Left "operator absent from direct lookup") Right (lookupOne (Text.unpack (operatorText row)))
        if fromMap == direct && Text.pack (opName direct) == operatorSymbol row
            then Right ()
            else Left "operator mapping mismatch"
    compareUnknown row =
        let (_, lookupOne) = productionGroup (unknownGroup row)
         in if lookupOne (Text.unpack (unknownOperator row)) == Nothing then Right () else Left "unknown lookup resolved"

compareOp :: Op -> Word8 -> Text -> Text -> Either String ()
compareOp actual byte symbol category
    | opByte actual /= byte = Left "opcode byte mismatch"
    | Text.pack (opName actual) /= symbol = Left "opcode name mismatch"
    | categoryName (opCategory actual) /= category = Left "opcode category mismatch"
    | otherwise = Right ()

categoryName :: OpCategory -> Text
categoryName Stack = "stack"
categoryName Variable = "variable"
categoryName Arithmetic = "arithmetic"
categoryName Comparison = "comparison"
categoryName ControlFlow = "control_flow"
categoryName Function = "function"
categoryName Collection = "collection"
categoryName SubscriptAttribute = "subscript_attribute"
categoryName Iteration = "iteration"
categoryName Module = "module"
categoryName InputOutput = "io"
categoryName VMControl = "vm_control"

productionGroup :: Text -> (Map String Op, String -> Maybe Op)
productionGroup "binary" = (binaryOpMap, binaryOpcode)
productionGroup "compare" = (compareOpMap, compareOpcode)
productionGroup "augmented_assign" = (augmentedAssignMap, augmentedAssignOpcode)
productionGroup "unary" = (unaryOpMap, unaryOpcode)
productionGroup _ = (Map.empty, const Nothing)

replaceOnce :: BS.ByteString -> BS.ByteString -> BS.ByteString -> BS.ByteString
replaceOnce needle replacement source =
    let (prefix, suffix) = BS.breakSubstring needle source
     in if BS.null suffix then source else prefix <> replacement <> BS.drop (BS.length needle) suffix

replaceAll :: BS.ByteString -> BS.ByteString -> BS.ByteString -> BS.ByteString
replaceAll needle replacement source =
    let (prefix, suffix) = BS.breakSubstring needle source
     in if BS.null suffix
            then prefix
            else prefix <> replacement <> replaceAll needle replacement (BS.drop (BS.length needle) suffix)

isLeft :: Either a b -> Bool
isLeft (Left _) = True
isLeft (Right _) = False
