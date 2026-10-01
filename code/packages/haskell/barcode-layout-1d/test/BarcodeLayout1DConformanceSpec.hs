{-# LANGUAGE OverloadedStrings #-}

module BarcodeLayout1DConformanceSpec (spec) where

import CodingAdventures.BarcodeLayout1D
import CodingAdventures.PaintInstructions (PaintInstruction (..), PaintScene (..))
import Control.Exception (IOException, evaluate, try)
import Data.Aeson
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KeyMap
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as LBS
import qualified Data.ByteString.Lazy.Char8 as LBS8
import Data.Either (isLeft)
import Data.Foldable (toList)
import qualified Data.Map.Strict as Map
import Data.Maybe (fromMaybe)
import Data.Scientific (floatingOrInteger)
import qualified Data.Set as Set
import qualified Data.Text as Text
import System.Directory (doesFileExist, findExecutable, getCurrentDirectory, getTemporaryDirectory, removeFile)
import System.Exit (ExitCode (..))
import System.FilePath ((</>), takeDirectory)
import System.IO (IOMode (ReadMode), hClose, openBinaryTempFile, withBinaryFile)
import System.Process (readProcessWithExitCode)
import Test.Hspec

spec :: Spec
spec = describe "barcode-layout-1d v1 conformance" $ do
  (document, schemaEncoded, documentEncoded) <- runIO loadFixture
  let cases = arrayField "cases" (asObject document)
      counts name = length (filter ((== name) . textField "operation" . asObject) cases)
  it "executes all 56 language-neutral cases" $ do
    length cases `shouldBe` 56
    counts "expand-binary" `shouldBe` 12
    counts "expand-width" `shouldBe` 12
    counts "compute-layout" `shouldBe` 19
    counts "project-scene" `shouldBe` 13
    mapM_ runCase cases

  it "rejects an on-disk fixture larger than the byte ceiling" $ do
    temporaryDirectory <- getTemporaryDirectory
    (path, handle) <- openBinaryTempFile temporaryDirectory "barcode-layout-1d-oversized.json"
    BS.hPut handle (BS.replicate (maxFixtureBytes + 1) 0x20)
    hClose handle
    result <- try (readFixtureFile path) :: IO (Either IOException LBS.ByteString)
    removeFile path
    case result of
      Left exception -> show exception `shouldContain` "fixture-size-limit"
      Right _ -> expectationFailure "oversized fixture was accepted"

  it "text-value-fails-before-native-resolution" $ do
    let options = defaultV1Options { v1HumanReadableText = Just "123" }
    either (Left . barcode1DV1ErrorId) (const (Right ()))
      (projectBarcode1DSceneV1 [Barcode1DRun Bar 0 "A" 0 Data] 0 options)
      `shouldBe` Left "human-readable-text-unsupported"

  it "text-enabled-fails-before-native-resolution" $ do
    let config = defaultV1Config
          { v1ModuleWidth = 0, v1IncludeHumanReadableText = True }
        options = defaultV1Options { v1RenderConfig = config }
    either (Left . barcode1DV1ErrorId) (const (Right ()))
      (projectBarcode1DSceneV1 [Barcode1DRun Bar 0 "A" 0 Data] 0 options)
      `shouldBe` Left "human-readable-text-unsupported"

  it "returns fresh immutable values on repeated calls" $ do
    let runs = [Barcode1DRun Bar 1 "A" 0 Data]
    projectBarcode1DSceneV1 runs 1 defaultV1Options
      `shouldBe` projectBarcode1DSceneV1 runs 1 defaultV1Options

  it "keeps portable failures closed and payload-blind" $ do
    let options = defaultBinaryPatternOptions "source" 0 Data
    runsFromBinaryPatternV1 "2" options `shouldBe` Left V1InvalidBinaryToken
    runsFromBinaryPatternV1 "x" options `shouldBe` Left V1InvalidBinaryToken
    barcode1DV1ErrorId V1InvalidBinaryToken `shouldBe` "invalid-binary-token"

  it "rejects hostile fixture envelopes before dispatch" $ do
    loadFixtureDocument schemaEncoded "{\"x\":1,\"x\":2}"
      `shouldSatisfy` isLeft
    loadFixtureDocument schemaEncoded (LBS8.pack (replicate 9 '[' ++ "0" ++ replicate 9 ']'))
      `shouldSatisfy` isLeft
    loadFixtureDocument schemaEncoded "[]" `shouldSatisfy` isLeft
    loadFixtureDocument schemaEncoded (LBS8.replicate 131073 'x')
      `shouldSatisfy` isLeft
    let badScalar = LBS8.pack (replaceOnce "\"layout-v1-binary-basic\""
          "\"\\ud800\"" (LBS8.unpack documentEncoded))
    loadFixtureDocument schemaEncoded badScalar `shouldSatisfy` isLeft

  it "checks repeat counts before materialization" $ do
    let tooLargePattern = asObject (object ["repeat" .= object
          ["token" .= ("1" :: String), "count" .= (65570 :: Int)]])
        tooManyRuns = asObject (object ["repeatRuns" .= object
          ["count" .= (40981 :: Int)]])
        tooManySymbols = asObject (object ["repeatSymbols" .= object
          ["count" .= (40981 :: Int)]])
    evaluate (length (inputPattern tooLargePattern)) `shouldThrow` anyErrorCall
    evaluate (length (inputRuns tooManyRuns)) `shouldThrow` anyErrorCall
    evaluate (length (fromMaybe [] (inputSymbols tooManySymbols)))
      `shouldThrow` anyErrorCall

  it "has an explicitly pure production call graph with no native resolver" $ do
    source <- findProductionSource
    body <- readFile source
    mapM_ (\forbidden -> body `shouldNotContain` forbidden)
      ["unsafePerformIO", "foreign import", "System.IO", "System.Process"
      , "System.Environment", "CodingAdventures.Font", "Text.Shaping"]

runCase :: Value -> Expectation
runCase value = do
  let caseObject = asObject value
      expected = asObject (valueField "expected" caseObject)
      identifier = textField "id" caseObject
  case KeyMap.lookup "error" expected of
    Just wanted -> case executeCase caseObject of
      Left actual -> actual `shouldBe` asText wanted
      Right _ -> expectationFailure (identifier ++ ": expected failure")
    Nothing -> case executeCase caseObject of
      Left actual -> expectationFailure (identifier ++ ": unexpected " ++ actual)
      Right actual -> case KeyMap.lookup "runDigest" expected of
        Just digestValue -> checkRunDigest actual (asObject digestValue)
        Nothing -> actual `shouldBe` expectedValue expected

executeCase :: Object -> Either String Value
executeCase caseObject =
  let operation = textField "operation" caseObject
      input = asObject (valueField "input" caseObject)
      project = either (Left . barcode1DV1ErrorId) Right
  in case operation of
      "expand-binary" -> runProjection <$> project
        (runsFromBinaryPatternV1 (inputPattern input) (binaryOptions input))
      "expand-width" -> runProjection <$> project
        (runsFromWidthPatternV1 (inputPattern input) (widthOptions input))
      "compute-layout" -> layoutProjection <$> project
        (computeBarcode1DLayoutV1 (inputRuns input) (intField "quietZoneModules" input)
          (inputSymbols input))
      "project-scene" -> sceneProjection <$> project
        (projectBarcode1DSceneV1 (inputRuns input) (intField "quietZoneModules" input)
          (sceneOptions input))
      _ -> Left ("unsupported operation " ++ operation)

inputPattern :: Object -> String
inputPattern input = case KeyMap.lookup "pattern" input of
  Just value -> asText value
  Nothing ->
    let repeated = asObject (valueField "repeat" input)
        count = boundedRepeatCount 65569 (intField "count" repeated)
        token = textField "token" repeated
        suffix = maybe "" asText (KeyMap.lookup "suffix" repeated)
    in if null token || length token > 2 || length suffix > 1
      then error "repeat pattern shape is outside fixture bounds"
      else concat (replicate count token) ++ suffix

binaryOptions :: Object -> RunsFromBinaryPatternOptions
binaryOptions input = defaultBinaryPatternOptions
  (textField "sourceLabel" input) (intField "sourceIndex" input)
  (roleValue (textField "role" input))

widthOptions :: Object -> RunsFromWidthPatternOptions
widthOptions input = (defaultWidthPatternOptions
  (textField "sourceLabel" input) (intField "sourceIndex" input)
  (roleValue (textField "role" input)))
    { widthNarrowModules = optionalInt "narrowModules" 1 input
    , widthWideModules = optionalInt "wideModules" 3 input
    , widthNarrowMarker = head (optionalText "narrowMarker" "N" input)
    , widthWideMarker = head (optionalText "wideMarker" "W" input)
    , widthStartingColor = colorValue (optionalText "startingColor" "bar" input)
    }

inputRuns :: Object -> [Barcode1DRun]
inputRuns input = case KeyMap.lookup "runs" input of
  Just (Array values) -> map valueRun (toList values)
  Just _ -> error "runs must be an array"
  Nothing ->
    let repeated = asObject (valueField "repeatRuns" input)
        first = colorValue (textField "firstColor" repeated)
        modules = intField "modules" repeated
        label = textField "sourceLabel" repeated
        sourceIndex = intField "sourceIndex" repeated
        role = roleValue (textField "role" repeated)
        count = boundedRepeatCount 40980 (intField "count" repeated)
    in [Barcode1DRun (if even index then first else flipColor first)
          modules label sourceIndex role
       | index <- [0 .. count - 1]]

valueRun :: Value -> Barcode1DRun
valueRun value = let item = asObject value in Barcode1DRun
  (colorValue (textField "color" item)) (intField "modules" item)
  (textField "sourceLabel" item) (intField "sourceIndex" item)
  (roleValue (textField "role" item))

inputSymbols :: Object -> Maybe [Barcode1DSymbolDescriptor]
inputSymbols input = case KeyMap.lookup "symbols" input of
  Just (Array values) -> Just (map valueSymbol (toList values))
  Just _ -> error "symbols must be an array"
  Nothing -> case KeyMap.lookup "repeatSymbols" input of
    Nothing -> Nothing
    Just value ->
      let repeated = asObject value
          count = boundedRepeatCount 40980 (intField "count" repeated)
      in Just [Barcode1DSymbolDescriptor (textField "label" repeated)
          (intField "modules" repeated) index
          (symbolRoleValue (textField "role" repeated))
        | index <- [0 .. count - 1]]

boundedRepeatCount :: Int -> Int -> Int
boundedRepeatCount maximumValue value
  | value < 0 || value > maximumValue = error "repeat count is outside fixture bounds"
  | otherwise = value

valueSymbol :: Value -> Barcode1DSymbolDescriptor
valueSymbol value = let item = asObject value in Barcode1DSymbolDescriptor
  (textField "label" item) (intField "modules" item)
  (intField "sourceIndex" item) (symbolRoleValue (textField "role" item))

sceneOptions :: Object -> Barcode1DV1Options
sceneOptions input = defaultV1Options
  { v1RenderConfig = defaultV1Config
      { v1ModuleWidth = optionalInt "moduleWidth" 4 render
      , v1BarHeight = optionalInt "barHeight" 120 render
      , v1Foreground = optionalText "foreground" "#000000" render
      , v1Background = optionalText "background" "#ffffff" render
      , v1IncludeHumanReadableText = optionalBool "includeHumanReadableText" False render
      }
  , v1Label = optionalText "label" "1D barcode" input
  , v1Metadata = case KeyMap.lookup "metadata" input of
      Nothing -> Map.empty
      Just value -> Map.fromList [(Key.toString key, asText item)
        | (key, item) <- KeyMap.toList (asObject value)]
  , v1HumanReadableText = case KeyMap.lookup "humanReadableText" input of
      Just Null -> Nothing
      Just value -> Just (asText value)
      Nothing -> Nothing
  , v1Symbols = inputSymbols input
  }
  where
    render = maybe KeyMap.empty asObject (KeyMap.lookup "renderConfig" input)

defaultV1Config :: Barcode1DV1RenderConfig
defaultV1Config = Barcode1DV1RenderConfig 4 120 "#000000" "#ffffff" False

defaultV1Options :: Barcode1DV1Options
defaultV1Options = Barcode1DV1Options defaultV1Config "1D barcode" Map.empty Nothing Nothing

runProjection :: [Barcode1DRun] -> Value
runProjection = toJSON . map runObject

runObject :: Barcode1DRun -> Value
runObject run = object
  [ "color" .= colorName (runColor run), "modules" .= runModules run
  , "sourceLabel" .= runSourceLabel run, "sourceIndex" .= runSourceIndex run
  , "role" .= roleName (runRole run)
  ]

layoutProjection :: Barcode1DLayout -> Value
layoutProjection layout = object
  [ "leftQuietZoneModules" .= layoutLeftQuietZoneModules layout
  , "rightQuietZoneModules" .= layoutRightQuietZoneModules layout
  , "contentModules" .= layoutContentModules layout
  , "totalModules" .= layoutTotalModules layout
  , "symbolLayouts" .= map symbolObject (layoutSymbolLayouts layout)
  ]

symbolObject :: Barcode1DSymbolLayout -> Value
symbolObject symbol = object
  [ "label" .= symbolLayoutLabel symbol
  , "startModule" .= symbolLayoutStartModule symbol
  , "endModule" .= symbolLayoutEndModule symbol
  , "sourceIndex" .= symbolLayoutSourceIndex symbol
  , "role" .= symbolRoleName (symbolLayoutRole symbol)
  ]

sceneProjection :: PaintScene -> Value
sceneProjection scene = object
  [ "width" .= (round (psWidth scene) :: Int)
  , "height" .= (round (psHeight scene) :: Int)
  , "background" .= psBg scene
  , "rectangles" .= map rectangleObject (psInstructions scene)
  , "metadata" .= psMeta scene
  ]

rectangleObject :: PaintInstruction -> Value
rectangleObject rectangle = case rectangle of
  PaintRect x y width height fill _ _ metadata -> object
    [ "x" .= (round x :: Int), "y" .= (round y :: Int)
    , "width" .= (round width :: Int), "height" .= (round height :: Int)
    , "fill" .= fill, "metadata" .= metadata
    ]
  _ -> error "v1 projection emitted a non-rectangle instruction"

checkRunDigest :: Value -> Object -> Expectation
checkRunDigest actual digest = do
  let values = case actual of Array items -> toList items; _ -> error "runs must be array"
      canonical = "[" ++ joinComma (map canonicalRun values) ++ "]"
  length values `shouldBe` intField "runCount" digest
  sum [intField "modules" (asObject value) | value <- values]
    `shouldBe` intField "contentModules" digest
  head values `shouldBe` valueField "firstRun" digest
  last values `shouldBe` valueField "lastRun" digest
  actualDigest <- sha256 canonical
  actualDigest `shouldBe` textField "runsSha256" digest

canonicalRun :: Value -> String
canonicalRun value =
  let item = asObject value
  in "{\"color\":" ++ jsonString (textField "color" item)
      ++ ",\"modules\":" ++ show (intField "modules" item)
      ++ ",\"role\":" ++ jsonString (textField "role" item)
      ++ ",\"sourceIndex\":" ++ show (intField "sourceIndex" item)
      ++ ",\"sourceLabel\":" ++ jsonString (textField "sourceLabel" item) ++ "}"

jsonString :: String -> String
jsonString = LBS8.unpack . encode

joinComma :: [String] -> String
joinComma [] = ""
joinComma (value : rest) = value ++ concatMap (',' :) rest

sha256 :: String -> IO String
sha256 input = do
  executable <- findExecutable "sha256sum"
  path <- maybe (fail "sha256sum is required for the conformance digest") pure executable
  outcome <- try (readProcessWithExitCode path [] input)
    :: IO (Either IOException (ExitCode, String, String))
  case outcome of
    Right (ExitSuccess, output, _) -> pure (take 64 output)
    _ -> fail "sha256sum failed"

expectedValue :: Object -> Value
expectedValue expected = fromMaybe (error "missing expected projection")
  (KeyMap.lookup "runs" expected
    `orElse` KeyMap.lookup "layout" expected
    `orElse` KeyMap.lookup "scene" expected)

orElse :: Maybe a -> Maybe a -> Maybe a
orElse (Just value) _ = Just value
orElse Nothing value = value

asObject :: Value -> Object
asObject (Object value) = value
asObject _ = error "expected object"

asText :: Value -> String
asText value = case fromJSON value of
  Success result -> result
  Error message -> error message

-- Text's Show instance quotes, so use JSON parsing to keep this module's
-- dependencies limited to Aeson and base.
textField :: Key -> Object -> String
textField key objectValue = case fromJSON (valueField key objectValue) of
  Success value -> value
  Error message -> error message

intField :: Key -> Object -> Int
intField key objectValue = case fromJSON (valueField key objectValue) of
  Success value -> value
  Error message -> error message

arrayField :: Key -> Object -> [Value]
arrayField key objectValue = case valueField key objectValue of
  Array values -> toList values
  _ -> error "expected array"

valueField :: Key -> Object -> Value
valueField key objectValue = fromMaybe (error ("missing field " ++ show key))
  (KeyMap.lookup key objectValue)

optionalText :: Key -> String -> Object -> String
optionalText key fallback objectValue = maybe fallback asText (KeyMap.lookup key objectValue)

optionalInt :: Key -> Int -> Object -> Int
optionalInt key fallback objectValue = maybe fallback convert (KeyMap.lookup key objectValue)
  where convert value = case fromJSON value of Success result -> result; Error message -> error message

optionalBool :: Key -> Bool -> Object -> Bool
optionalBool key fallback objectValue = maybe fallback convert (KeyMap.lookup key objectValue)
  where convert value = case fromJSON value of Success result -> result; Error message -> error message

colorValue :: String -> Barcode1DRunColor
colorValue "bar" = Bar
colorValue "space" = Space
colorValue value = error ("bad color " ++ value)

flipColor :: Barcode1DRunColor -> Barcode1DRunColor
flipColor Bar = Space
flipColor Space = Bar

colorName :: Barcode1DRunColor -> String
colorName Bar = "bar"
colorName Space = "space"

roleValue :: String -> Barcode1DRunRole
roleValue value = case value of
  "data" -> Data; "start" -> Start; "stop" -> Stop; "guard" -> Guard
  "check" -> Check; "inter-character-gap" -> InterCharacterGap
  _ -> error ("bad role " ++ value)

roleName :: Barcode1DRunRole -> String
roleName value = case value of
  Data -> "data"; Start -> "start"; Stop -> "stop"; Guard -> "guard"
  Check -> "check"; InterCharacterGap -> "inter-character-gap"

symbolRoleValue :: String -> Barcode1DSymbolRole
symbolRoleValue value = case value of
  "data" -> SymbolData; "start" -> SymbolStart; "stop" -> SymbolStop
  "guard" -> SymbolGuard; "check" -> SymbolCheck
  _ -> error ("bad symbol role " ++ value)

symbolRoleName :: Barcode1DSymbolRole -> String
symbolRoleName value = case value of
  SymbolData -> "data"; SymbolStart -> "start"; SymbolStop -> "stop"
  SymbolGuard -> "guard"; SymbolCheck -> "check"

data JsonFrame = JsonObject (Set.Set String) Bool | JsonArray

maxFixtureBytes :: Int
maxFixtureBytes = 131072

readFixtureFile :: FilePath -> IO LBS.ByteString
readFixtureFile path = withBinaryFile path ReadMode $ \handle -> do
  encoded <- BS.hGet handle (maxFixtureBytes + 1)
  if BS.length encoded > maxFixtureBytes
    then fail "fixture-size-limit"
    else pure (LBS.fromStrict encoded)

loadFixture :: IO (Value, LBS8.ByteString, LBS8.ByteString)
loadFixture = do
  cwd <- getCurrentDirectory
  path <- findFixture cwd 8
  let schemaPath = takeDirectory path </> "schema.json"
  documentEncoded <- readFixtureFile path
  schemaEncoded <- readFixtureFile schemaPath
  document <- either fail pure (loadFixtureDocument schemaEncoded documentEncoded)
  pure (document, schemaEncoded, documentEncoded)

loadFixtureDocument :: LBS8.ByteString -> LBS8.ByteString -> Either String Value
loadFixtureDocument schemaEncoded documentEncoded = do
  if max (LBS8.length schemaEncoded) (LBS8.length documentEncoded) > 131072
    then Left "fixture-size-limit" else Right ()
  preflightJson 24 schemaEncoded
  preflightJson 8 documentEncoded
  schema <- eitherDecode schemaEncoded
  document <- eitherDecode documentEncoded
  validateJsonTree 24 0 schema
  validateJsonTree 8 0 document
  validateFixtureSchema schema
  validateFixtureDocument document
  pure document

preflightJson :: Int -> LBS8.ByteString -> Either String ()
preflightJson limit encoded = scan [] (LBS8.unpack encoded)
  where
    scan _ [] = Right ()
    scan frames ('"' : rest) = do
      (token, remaining) <- takeJsonString ['"'] False rest
      updated <- registerKey frames token
      scan updated remaining
    scan frames ('{' : rest) = push (JsonObject Set.empty True : frames) rest
    scan frames ('[' : rest) = push (JsonArray : frames) rest
    scan (JsonObject keys _ : frames) (',' : rest) =
      scan (JsonObject keys True : frames) rest
    scan (_ : frames) ('}' : rest) = scan frames rest
    scan (_ : frames) (']' : rest) = scan frames rest
    scan frames (_ : rest) = scan frames rest
    push frames rest
      | length frames > limit = Left "fixture-depth-limit"
      | otherwise = scan frames rest

takeJsonString :: String -> Bool -> String -> Either String (String, String)
takeJsonString _ _ [] = Left "fixture-invalid-json"
takeJsonString reversed escaped (character : rest)
  | escaped = takeJsonString (character : reversed) False rest
  | character == '\\' = takeJsonString (character : reversed) True rest
  | character == '"' = Right (reverse ('"' : reversed), rest)
  | otherwise = takeJsonString (character : reversed) False rest

registerKey :: [JsonFrame] -> String -> Either String [JsonFrame]
registerKey (JsonObject keys True : frames) token = do
  key <- eitherDecode (LBS8.pack token)
  if Set.member key keys
    then Left "fixture-duplicate-key"
    else Right (JsonObject (Set.insert key keys) False : frames)
registerKey frames _ = Right frames

validateJsonTree :: Int -> Int -> Value -> Either String ()
validateJsonTree limit depth value
  | depth > limit = Left "fixture-depth-limit"
  | otherwise = case value of
      String text -> validateScalar text
      Number number -> case (floatingOrInteger number :: Either Double Integer) of
        Left _ -> Left "fixture-invalid-type"
        Right _ -> Right ()
      Array values -> mapM_ (validateJsonTree limit (depth + 1)) (toList values)
      Object values -> mapM_ validateEntry (KeyMap.toList values)
      Bool _ -> Right ()
      Null -> Right ()
  where
    validateEntry (key, item) = do
      validateScalar (Key.toText key)
      validateJsonTree limit (depth + 1) item
    validateScalar text
      | Text.any (\character -> let point = fromEnum character
          in point >= 0xd800 && point <= 0xdfff) text =
            Left "fixture-invalid-scalar"
      | otherwise = Right ()

validateFixtureSchema :: Value -> Either String ()
validateFixtureSchema (Object schema) = do
  identifier <- fieldEither "$id" schema >>= stringEither
  if identifier == "https://coding-adventures.dev/schemas/barcode-layout-1d-v1.json"
    then validateLocalRefs (Object schema)
    else Left "fixture-schema-invalid"
validateFixtureSchema _ = Left "fixture-schema-invalid"

validateLocalRefs :: Value -> Either String ()
validateLocalRefs value = case value of
  Array values -> mapM_ validateLocalRefs (toList values)
  Object values -> mapM_ check (KeyMap.toList values)
  _ -> Right ()
  where
    check (key, item)
      | Key.toString key `elem` ["$ref", "$dynamicRef"] = do
          reference <- stringEither item
          if take 2 reference == "#/" then Right () else Left "fixture-schema-invalid"
      | otherwise = validateLocalRefs item

validateFixtureDocument :: Value -> Either String ()
validateFixtureDocument (Object document) = do
  exactObjectKeys ["schema_version", "profile", "limits", "error_ids", "cases"] document
  schemaVersion <- fieldEither "schema_version" document >>= intEither
  profile <- fieldEither "profile" document >>= stringEither
  limits <- fieldEither "limits" document >>= objectEither
  errors <- fieldEither "error_ids" document >>= arrayEither >>= mapM stringEither
  cases <- fieldEither "cases" document >>= arrayEither
  if schemaVersion /= 1 || profile /= "barcode-layout-1d-v1"
      || errors /= fixtureErrorIds || length cases < 48 || length cases > 64
    then Left "fixture-schema-invalid"
    else validateLimits limits >> validateCases Set.empty cases
  where
    validateCases _ [] = Right ()
    validateCases seen (item : rest) = do
      value <- objectEither item
      exactObjectKeys ["id", "operation", "input", "expected"] value
      identifier <- fieldEither "id" value >>= stringEither
      operation <- fieldEither "operation" value >>= stringEither
      input <- fieldEither "input" value >>= objectEither
      expected <- fieldEither "expected" value >>= objectEither
      if Set.member identifier seen || take 10 identifier /= "layout-v1-"
        then Left "fixture-schema-invalid" else Right ()
      validateOperation operation input expected
      validateCases (Set.insert identifier seen) rest
validateFixtureDocument _ = Left "fixture-schema-invalid"

validateLimits :: Object -> Either String ()
validateLimits limits = do
  exactObjectKeys (map fst fixtureLimits) limits
  mapM_ check fixtureLimits
  where
    check (name, expected) = do
      actual <- fieldEither (Key.fromString name) limits >>= intEither
      if actual == expected then Right () else Left "fixture-schema-invalid"

fixtureLimits :: [(String, Int)]
fixtureLimits =
  [ ("max_pattern_scalars", 65567), ("max_runs", 40979)
  , ("max_content_modules", 65567), ("max_quiet_zone_modules", 4096)
  , ("max_symbols", 40979), ("max_label_scalars", 4096)
  , ("max_metadata_entries", 64), ("max_metadata_key_scalars", 128)
  , ("max_metadata_value_scalars", 4096), ("max_metadata_utf8_bytes", 65536)
  , ("max_module_width", 8192), ("max_bar_height", 8192)
  , ("max_color_scalars", 128), ("max_cases", 64)
  , ("max_fixture_bytes", 131072), ("max_fixture_depth", 8)
  ]

fixtureErrorIds :: [String]
fixtureErrorIds =
  [ "pattern-too-long", "empty-pattern", "invalid-binary-token"
  , "invalid-width-token", "invalid-marker-configuration", "invalid-module-count"
  , "too-many-runs", "content-too-wide", "non-alternating-runs"
  , "invalid-quiet-zone", "too-many-symbols", "symbol-width-mismatch"
  , "invalid-render-config", "metadata-too-large"
  , "human-readable-text-unsupported", "invalid-source-attribution"
  ]

validateOperation :: String -> Object -> Object -> Either String ()
validateOperation operation input expected = do
  let expectedKeys = map Key.toString (KeyMap.keys expected)
      exclusive left right = KeyMap.member left input /= KeyMap.member right input
      actualInputKeys = Set.fromList (map Key.toString (KeyMap.keys input))
  if actualInputKeys `Set.isSubsetOf` fixtureInputKeys
    then Right () else Left "fixture-schema-invalid"
  validateRepeatInput input
  if length expectedKeys /= 1 then Left "fixture-schema-invalid" else Right ()
  validateExpectedType expected
  case operation of
    "expand-binary" -> expand expectedKeys exclusive
    "expand-width" -> expand expectedKeys exclusive
    "compute-layout" -> layout expectedKeys exclusive
    "project-scene" -> scene expectedKeys exclusive
    _ -> Left "fixture-schema-invalid"
  where
    required keys = if all (`KeyMap.member` input) keys
      then Right () else Left "fixture-schema-invalid"
    expand keys exclusive = do
      required ["sourceLabel", "sourceIndex", "role"]
      if exclusive "pattern" "repeat"
          && head keys `elem` ["runs", "runDigest", "error"]
        then Right () else Left "fixture-schema-invalid"
    layout keys exclusive = do
      required ["quietZoneModules"]
      if exclusive "runs" "repeatRuns" && head keys `elem` ["layout", "error"]
        then Right () else Left "fixture-schema-invalid"
    scene keys exclusive = do
      required ["quietZoneModules"]
      if exclusive "runs" "repeatRuns" && head keys `elem` ["scene", "error"]
        then Right () else Left "fixture-schema-invalid"

fixtureInputKeys :: Set.Set String
fixtureInputKeys = Set.fromList
  [ "pattern", "repeat", "sourceLabel", "sourceIndex", "role"
  , "narrowMarker", "wideMarker", "narrowModules", "wideModules"
  , "startingColor", "runs", "repeatRuns", "quietZoneModules"
  , "symbols", "repeatSymbols", "renderConfig", "label", "metadata"
  , "humanReadableText"
  ]

validateRepeatInput :: Object -> Either String ()
validateRepeatInput input = do
  check "repeat" 65569
  check "repeatRuns" 40980
  check "repeatSymbols" 40980
  where
    check key maximumValue = case KeyMap.lookup key input of
      Nothing -> Right ()
      Just value -> do
        repeated <- objectEither value
        count <- fieldEither "count" repeated >>= intEither
        if count >= 0 && count <= maximumValue
          then Right () else Left "fixture-schema-invalid"

validateExpectedType :: Object -> Either String ()
validateExpectedType expected = case KeyMap.toList expected of
  [(key, value)] -> case Key.toString key of
    "error" -> stringEither value >> Right ()
    "runs" -> arrayEither value >> Right ()
    "runDigest" -> objectEither value >> Right ()
    "layout" -> objectEither value >> Right ()
    "scene" -> objectEither value >> Right ()
    _ -> Left "fixture-schema-invalid"
  _ -> Left "fixture-schema-invalid"

exactObjectKeys :: [String] -> Object -> Either String ()
exactObjectKeys wanted actual =
  if Set.fromList wanted == Set.fromList (map Key.toString (KeyMap.keys actual))
    then Right () else Left "fixture-schema-invalid"

fieldEither :: Key -> Object -> Either String Value
fieldEither key value = maybe (Left "fixture-schema-invalid") Right (KeyMap.lookup key value)

objectEither :: Value -> Either String Object
objectEither (Object value) = Right value
objectEither _ = Left "fixture-schema-invalid"

arrayEither :: Value -> Either String [Value]
arrayEither (Array value) = Right (toList value)
arrayEither _ = Left "fixture-schema-invalid"

stringEither :: Value -> Either String String
stringEither value = case fromJSON value of
  Success result -> Right result
  Error _ -> Left "fixture-schema-invalid"

intEither :: Value -> Either String Int
intEither value = case fromJSON value of
  Success result -> Right result
  Error _ -> Left "fixture-schema-invalid"

replaceOnce :: String -> String -> String -> String
replaceOnce needle replacement haystack = go haystack
  where
    go [] = []
    go remaining
      | take (length needle) remaining == needle =
          replacement ++ drop (length needle) remaining
      | otherwise = head remaining : go (tail remaining)

findProductionSource :: IO FilePath
findProductionSource = do
  cwd <- getCurrentDirectory
  fixture <- findFixture cwd 8
  let root = takeDirectory (takeDirectory (takeDirectory (takeDirectory fixture)))
      source = root </> "packages" </> "haskell" </> "barcode-layout-1d"
        </> "src" </> "CodingAdventures" </> "BarcodeLayout1D.hs"
  exists <- doesFileExist source
  if exists then pure source else fail "Haskell production source not found"

findFixture :: FilePath -> Int -> IO FilePath
findFixture start remaining = do
  let candidate = start </> "code" </> "specs" </> "fixtures"
        </> "barcode-layout-1d-v1" </> "cases.json"
      fromCode = start </> "specs" </> "fixtures"
        </> "barcode-layout-1d-v1" </> "cases.json"
  direct <- doesFileExist candidate
  codeRelative <- doesFileExist fromCode
  if direct then pure candidate else if codeRelative then pure fromCode
    else if remaining <= 0 || takeDirectory start == start
      then fail "barcode-layout-1d-v1 fixture not found"
      else findFixture (takeDirectory start) (remaining - 1)
