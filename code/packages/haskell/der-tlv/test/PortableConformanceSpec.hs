{-# LANGUAGE OverloadedStrings #-}

module PortableConformanceSpec (spec) where

import CodingAdventures.DerTlv
import Control.Exception (throwIO)
import Control.Monad (foldM)
import Data.Aeson
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.Char (digitToInt)
import Data.Foldable (toList)
import Data.Scientific (toBoundedInteger)
import qualified Data.Text as Text
import qualified Data.Text.Encoding as TextEncoding
import Data.Word (Word64)
import System.Directory (doesFileExist)
import System.FilePath ((</>))
import Test.Hspec

spec :: Spec
spec = describe "portable DER TLV conformance" $ do
  fixture <- runIO loadFixture
  it "matches all 54 language-neutral cases" $ do
    field "schema_version" fixture `shouldReturn` Number 1
    field "profile" fixture `shouldReturn` String "x690-der-tlv-framing-v1"
    defaults <- expectObject =<< field "defaults" fixture
    field "max_input_len" defaults `shouldReturn` Number 1048576
    field "max_value_len" defaults `shouldReturn` Number 1048576
    field "max_elements" defaults `shouldReturn` Number 4096
    field "max_tag_number" defaults `shouldReturn` Number 4294967295
    errors <- expectArray =<< field "error_ids" fixture
    mapM expectText errors `shouldReturn`
      [ "empty-input", "truncated-high-tag", "truncated-length", "truncated-value"
      , "end-of-contents", "non-minimal-tag", "tag-overflow", "indefinite-length"
      , "reserved-length", "non-minimal-length", "length-too-wide", "length-host-overflow"
      , "input-limit-exceeded", "value-limit-exceeded", "element-limit-exceeded"
      , "tag-limit-exceeded", "trailing-data"
      ]
    cases <- expectArray =<< field "cases" fixture
    length cases `shouldBe` 54
    mapM_ (runCase fixture) cases

runCase :: Object -> Value -> Expectation
runCase fixture caseValue = do
  testCase <- expectObject caseValue
  input <- materialize =<< (expectArray =<< field "input" testCase)
  operation <- expectText =<< field "operation" testCase
  limits <- caseLimits fixture testCase
  actual <- case operation of
    "cursor" -> runCursor testCase input limits
    "decode-one" -> runDecode False input limits
    "decode-exact" -> runDecode True input limits
    _ -> throwIO (userError ("unknown operation " ++ operation))
  expected <- field "expected" testCase
  actual `shouldBe` expected
  case KM.lookup "redacted_input_hex" testCase of
    Just (String hostile) -> BS.isInfixOf (TextEncoding.encodeUtf8 hostile) (BL.toStrict (encode actual)) `shouldBe` False
    _ -> pure ()

runDecode :: Bool -> BS.ByteString -> DerLimits -> IO Value
runDecode exact input limits =
  case if exact then fmap (\element -> (element, BS.empty)) (decodeExact limits input) else decodeOne limits input of
    Left problem -> pure (errorValue problem)
    Right (element, remainder) -> do
      validateViews input element remainder
      pure (elementValueProjection 0 element)

runCursor :: Object -> BS.ByteString -> DerLimits -> IO Value
runCursor testCase input limits = do
  actions <- expectArray =<< field "actions" testCase
  cursor <- either (throwIO . userError . show) pure (newCursor limits input)
  (events, finalCursor) <- foldM step ([], cursor) actions
  pure $ object
    [ "events" .= events
    , "elements_read" .= cursorElementsRead finalCursor
    , "remaining_offset" .= (BS.length input - BS.length (cursorRemaining finalCursor))
    ]
  where
    step (events, cursor) (String "finish") =
      case finishCursor cursor of
        Left problem -> pure (events ++ [errorValue problem], cursor)
        Right () -> pure (events ++ [object ["outcome" .= String "finished"]], cursor)
    step (events, cursor) (String "read") =
      let offset = BS.length input - BS.length (cursorRemaining cursor)
       in case readCursor cursor of
            Left problem -> pure (events ++ [errorValue problem], cursor)
            Right (Nothing, next) -> pure (events ++ [object ["outcome" .= String "end"]], next)
            Right (Just element, next) -> do
              validateViews (cursorRemaining cursor) element (cursorRemaining next)
              pure (events ++ [elementValueProjection offset element], next)
    step _ value = throwIO (userError ("unknown cursor action " ++ show value))

validateViews :: BS.ByteString -> DerElement -> BS.ByteString -> Expectation
validateViews input element remainder = do
  let encodedLength = BS.length (elementEncoded element)
      headerLength = BS.length (elementHeader element)
  elementEncoded element `shouldBe` BS.take encodedLength input
  elementHeader element `shouldBe` BS.take headerLength input
  elementValue element `shouldBe` BS.take (encodedLength - headerLength) (BS.drop headerLength input)
  remainder `shouldBe` BS.drop encodedLength input

elementValueProjection :: Int -> DerElement -> Value
elementValueProjection offset element = object
  [ "outcome" .= String "element"
  , "element_offset" .= offset
  , "tag" .= object
      [ "class" .= className (tagClass tag)
      , "constructed" .= tagConstructed tag
      , "number" .= tagNumber tag
      ]
  , "header_len" .= BS.length (elementHeader element)
  , "encoded_len" .= BS.length (elementEncoded element)
  , "remainder_offset" .= (offset + BS.length (elementEncoded element))
  ]
  where tag = elementTag element

className :: TagClass -> String
className Universal = "universal"
className Application = "application"
className ContextSpecific = "context-specific"
className Private = "private"

errorValue :: DerError -> Value
errorValue problem = object
  [ "outcome" .= String "error"
  , "error_id" .= errorId (derErrorKind problem)
  , "offset" .= derErrorOffset problem
  ]

caseLimits :: Object -> Object -> IO DerLimits
caseLimits fixture testCase = do
  defaults <- expectObject =<< field "defaults" fixture
  let overrides = case KM.lookup "limits" testCase of Just (Object value) -> value; _ -> KM.empty
      get name = maybe (field name defaults) pure (KM.lookup (Key.fromString name) overrides)
  inputLimit <- number =<< get "max_input_len"
  valueNode <- get "max_value_len"
  valueLimit <- case valueNode of String "host-max" -> pure (fromIntegral (maxBound :: Int)); _ -> number valueNode
  elementLimit <- number =<< get "max_elements"
  tagLimit <- number =<< get "max_tag_number"
  pure (DerLimits inputLimit valueLimit elementLimit (fromIntegral tagLimit))

materialize :: [Value] -> IO BS.ByteString
materialize segments = BS.concat <$> mapM segment segments
  where
    segment value = do
      objectValue <- expectObject value
      case KM.lookup "hex" objectValue of
        Just textValue -> hexBytes <$> expectText textValue
        Nothing -> do
          bytes <- hexBytes <$> (expectText =<< field "repeat_hex" objectValue)
          count <- fromIntegral <$> (number =<< field "count" objectValue)
          pure $ if BS.length bytes == 1 then BS.replicate count (BS.head bytes) else BS.concat (replicate count bytes)

hexBytes :: String -> BS.ByteString
hexBytes [] = BS.empty
hexBytes (a:b:rest) = BS.cons (fromIntegral (digitToInt a * 16 + digitToInt b)) (hexBytes rest)
hexBytes _ = error "odd hex"

loadFixture :: IO Object
loadFixture = do
  let relative = "specs" </> "fixtures" </> "der-tlv-v1" </> "cases.json"
      candidates = [".." </> ".." </> ".." </> relative, ".." </> ".." </> relative]
  existing <- filterM doesFileExist candidates
  path <- case existing of first:_ -> pure first; [] -> throwIO (userError "fixture not found")
  bytes <- BS.readFile path
  value <- either (throwIO . userError) pure (eitherDecodeStrict' bytes)
  expectObject value

field :: String -> Object -> IO Value
field name objectValue = maybe (throwIO (userError ("missing " ++ name))) pure (KM.lookup (Key.fromString name) objectValue)

expectObject :: Value -> IO Object
expectObject (Object value) = pure value
expectObject _ = throwIO (userError "expected object")

expectArray :: Value -> IO [Value]
expectArray (Array value) = pure (toList value)
expectArray _ = throwIO (userError "expected array")

expectText :: Value -> IO String
expectText (String value) = pure (Text.unpack value)
expectText _ = throwIO (userError "expected text")

number :: Value -> IO Word64
number (Number value) = maybe (throwIO (userError "bad number")) pure (toBoundedInteger value)
number _ = throwIO (userError "expected number")

filterM :: Monad m => (a -> m Bool) -> [a] -> m [a]
filterM _ [] = pure []
filterM predicate (value:rest) = do
  keep <- predicate value
  values <- filterM predicate rest
  pure (if keep then value : values else values)
