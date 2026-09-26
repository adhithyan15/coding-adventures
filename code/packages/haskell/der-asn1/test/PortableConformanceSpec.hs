{-# LANGUAGE OverloadedStrings #-}

module PortableConformanceSpec (spec) where

import Control.Monad (foldM)
import Data.Aeson (Value (..), eitherDecodeFileStrict', object, (.=))
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KeyMap
import qualified Data.ByteString as BS
import Data.Char (digitToInt)
import Data.Foldable (toList)
import Data.Maybe (fromMaybe)
import Data.Scientific (toBoundedInteger)
import qualified Data.Text as T
import Data.Word (Word32, Word64)
import Numeric (showHex)
import Test.Hspec
import CodingAdventures.DerAsn1
import qualified CodingAdventures.DerTlv as DT

fixture :: FilePath -> IO Value
fixture name = do
  result <- eitherDecodeFileStrict' ("../../../specs/fixtures/" ++ name ++ "/cases.json")
  case result of
    Left err -> expectationFailure err >> fail "fixture parse failed"
    Right value -> pure value

fieldMaybe :: String -> Value -> Maybe Value
fieldMaybe name (Object values) = KeyMap.lookup (Key.fromString name) values
fieldMaybe _ _ = Nothing

field :: String -> Value -> Value
field name value = fromMaybe (error ("missing fixture field " ++ name)) (fieldMaybe name value)

asString :: Value -> String
asString (String value) = T.unpack value
asString value = error ("expected string, got " ++ show value)

asWord64 :: Value -> Word64
asWord64 (Number value) = fromMaybe (error "fixture integer out of range") (toBoundedInteger value)
asWord64 value = error ("expected integer, got " ++ show value)

asArray :: Value -> [Value]
asArray (Array values) = toList values
asArray value = error ("expected array, got " ++ show value)

hexBytes :: String -> BS.ByteString
hexBytes [] = BS.empty
hexBytes (a:b:rest) = BS.cons (fromIntegral (digitToInt a * 16 + digitToInt b)) (hexBytes rest)
hexBytes _ = error "fixture hex must contain complete octets"

toHex :: BS.ByteString -> String
toHex = concatMap render . BS.unpack
  where
    render byte = let value = showHex byte "" in if length value == 1 then '0' : value else value

materialize :: Value -> BS.ByteString
materialize = BS.concat . map segment . asArray
  where
    segment value = case fieldMaybe "hex" value of
      Just encoded -> hexBytes (asString encoded)
      Nothing ->
        let octets = hexBytes (asString (field "repeat_hex" value))
            count = fromIntegral (asWord64 (field "count" value))
        in if BS.length octets == 1 then BS.replicate count (BS.head octets) else error "repeat_hex must be one octet"

override :: Value -> Maybe Value -> String -> Value
override defaults overrides name = fromMaybe (field name defaults) (overrides >>= fieldMaybe name)

derLimits :: Value -> Maybe Value -> DT.DerLimits
derLimits defaults overrides = DT.DerLimits
  { DT.maxInputLength = asWord64 (override defaults overrides "max_input_len")
  , DT.maxValueLength = let value = override defaults overrides "max_value_len"
      in if value == String "host-max" then fromIntegral (maxBound :: Int) else asWord64 value
  , DT.maxElements = asWord64 (override defaults overrides "max_elements")
  , DT.maxTagNumber = fromIntegral (asWord64 (override defaults overrides "max_tag_number"))
  }

caseLimits :: Value -> Value -> Asn1Limits
caseLimits document item = Asn1Limits
  { asn1DerLimits = derLimits (field "der" defaults) (overrides >>= fieldMaybe "der")
  , maxDepth = asWord64 (override defaults overrides "max_depth")
  , maxTotalElements = asWord64 (override defaults overrides "max_total_elements")
  , maxOidArcs = asWord64 (override defaults overrides "max_oid_arcs")
  }
  where
    defaults = field "defaults" document
    overrides = fieldMaybe "limits" item

className :: DT.TagClass -> String
className value = case value of
  DT.Universal -> "universal"
  DT.Application -> "application"
  DT.ContextSpecific -> "context-specific"
  DT.Private -> "private"

tagProjection :: Asn1Element -> Value
tagProjection element = object
  [ "class" .= className (DT.tagClass tag)
  , "constructed" .= DT.tagConstructed tag
  , "number" .= DT.tagNumber tag
  ]
  where tag = asn1ElementTag element

failure :: Asn1Error -> String -> Value
failure err scope = object (base ++ framingField)
  where
    base =
      [ "outcome" .= ("error" :: String)
      , "error_id" .= asn1ErrorId (asn1ErrorKind err)
      , "offset" .= asn1ErrorOffset err
      , "offset_scope" .= scope
      ]
    framingField = case asn1ErrorKind err of
      Framing kind -> ["framing_error_id" .= DT.errorId kind]
      _ -> []

verifyUpstream :: Value -> Value -> IO Value
verifyUpstream upstream item = do
  let wanted = asString (field "der_tlv_case_id" item)
      referenced = fromMaybe (error ("missing upstream case " ++ wanted))
        (findCase wanted (asArray (field "cases" upstream)))
      input = materialize (field "input" referenced)
      configured = defaultAsn1Limits
        { asn1DerLimits = derLimits (field "defaults" upstream) (fieldMaybe "limits" referenced) }
      expected = field "expected" referenced
  decoder <- newAsn1Decoder configured
  actual <- decodeExact decoder input
  case actual of
    Right element -> do
      field "outcome" expected `shouldBe` String "element"
      tagProjection element `shouldBe` field "tag" expected
      fromIntegral (BS.length (asn1ElementHeader element)) `shouldBe` asWord64 (field "header_len" expected)
      fromIntegral (BS.length (asn1ElementEncoded element)) `shouldBe` asWord64 (field "encoded_len" expected)
      fromIntegral (BS.length input) `shouldBe` asWord64 (field "remainder_offset" expected)
    Left err -> do
      field "outcome" expected `shouldBe` String "error"
      case asn1ErrorKind err of
        Framing kind -> DT.errorId kind `shouldBe` asString (field "error_id" expected)
        other -> expectationFailure ("expected framing error, got " ++ show other)
      fromIntegral (asn1ErrorOffset err) `shouldBe` asWord64 (field "offset" expected)
  pure (object ["outcome" .= ("upstream" :: String)])
  where
    findCase _ [] = Nothing
    findCase wanted (candidate:rest)
      | asString (field "id" candidate) == wanted = Just candidate
      | otherwise = findCase wanted rest

primitive :: String -> Asn1Element -> Asn1Limits -> Maybe Word32 -> Either Asn1Error Value
primitive operation element limits tagNumber = case operation of
  "decode-boolean" -> do
    value <- decodeBoolean element
    pure (object ["outcome" .= ("value" :: String), "boolean" .= value, "elements_read" .= (1 :: Word64)])
  "decode-integer" -> integerValue False
  "integer-to-u64" -> integerValue True
  "decode-bit-string" -> do
    bits <- decodeBitString element
    pure (object
      [ "outcome" .= ("value" :: String)
      , "bytes_hex" .= toHex (bitStringBytes bits)
      , "unused_bits" .= bitStringUnusedBits bits
      , "bit_length" .= bitStringLength bits
      ])
  "decode-octet-string" -> byteValue (decodeOctetString element)
  "decode-implicit-octet-string" -> byteValue (decodeImplicitOctetString element (needTag tagNumber))
  "decode-ia5-string" -> textValue (decodeIa5String element)
  "decode-implicit-ia5-string" -> textValue (decodeImplicitIa5String element (needTag tagNumber))
  "decode-null" -> decodeNull element >> pure (object ["outcome" .= ("value" :: String)])
  "decode-object-identifier" -> oidValue (decodeObjectIdentifier element limits)
  "decode-implicit-object-identifier" -> oidValue (decodeImplicitObjectIdentifier element (needTag tagNumber) limits)
  _ -> error ("unsupported primitive operation " ++ operation)
  where
    needTag = fromMaybe (error "missing tag_number")
    byteValue result = do
      bytes <- result
      pure (object ["outcome" .= ("value" :: String), "bytes_hex" .= toHex bytes])
    textValue result = do
      value <- result
      pure (object ["outcome" .= ("value" :: String), "text" .= value])
    integerValue includeUnsigned = do
      integer <- decodeInteger element
      unsigned <- if includeUnsigned then Just <$> integerToWord64 integer else Right Nothing
      pure (object (
        [ "outcome" .= ("value" :: String)
        , "signed_hex" .= toHex (integerSignedBytes integer)
        , "negative" .= integerIsNegative integer
        ] ++ maybe [] (\value -> ["u64_decimal" .= show value]) unsigned))
    oidValue result = do
      oid <- result
      pure (object
        [ "outcome" .= ("value" :: String)
        , "bytes_hex" .= toHex (oidEncoded oid)
        , "arcs_decimal" .= map show (oidArcs oid)
        , "arc_count" .= oidArcCount oid
        ])

cursorCase :: Value -> Asn1Decoder -> Asn1Element -> IO (Either Asn1Error Value)
cursorCase item decoder root = case sequenceCursor decoder root of
  Left err -> pure (Left err)
  Right start -> do
    let total = BS.length (asn1CursorRemaining start)
    (current, events) <- foldM action (Just start, []) (map asString (asArray (field "actions" item)))
    count <- decoderElementsRead decoder
    let remaining = maybe 0 (BS.length . asn1CursorRemaining) current
    pure (Right (object
      [ "outcome" .= ("value" :: String)
      , "elements_read" .= count
      , "remaining_offset" .= (total - remaining)
      , "events" .= events
      ]))
  where
    action (current, events) name = case (name, current) of
      ("finish", Just cursor) ->
        let event = either (\err -> failure err "container-value") (const (object ["outcome" .= ("finished" :: String)])) (finishAsn1Cursor cursor)
        in pure (Nothing, events ++ [event])
      ("read-with-different-limits", Just cursor) -> do
        let mismatch = (decoderLimits decoder) {maxTotalElements = maxTotalElements (decoderLimits decoder) + 1}
        other <- newAsn1Decoder mismatch
        result <- readAsn1Cursor other cursor
        let event = readEvent result
            next = either (const cursor) snd result
        pure (Just next, events ++ [event])
      ("read", Just cursor) -> do
        result <- readAsn1Cursor decoder cursor
        let event = readEvent result
            next = either (const cursor) snd result
        pure (Just next, events ++ [event])
      ("read-nested-sequence", Just cursor) -> do
        outer <- readAsn1Cursor decoder cursor
        case outer of
          Left err -> pure (Just cursor, events ++ [failure err "container-value"])
          Right (Nothing, next) -> pure (Just next, events ++ [object ["outcome" .= ("end" :: String)]])
          Right (Just child, next) -> do
            event <- case sequenceCursor decoder child of
              Left err -> pure (failure err "container-value")
              Right nested -> do
                inner <- readAsn1Cursor decoder nested
                pure $ case inner of
                  Left err -> failure err "container-value"
                  Right (Nothing, _) -> object ["outcome" .= ("end" :: String)]
                  Right (Just grandchild, finished) -> case finishAsn1Cursor finished of
                    Left err -> failure err "container-value"
                    Right () -> object ["outcome" .= ("value" :: String), "tag" .= tagProjection grandchild, "depth" .= asn1ElementDepth grandchild]
            pure (Just next, events ++ [event])
      _ -> error ("invalid cursor action/state " ++ name)
    readEvent result = case result of
      Left err -> failure err "container-value"
      Right (Nothing, _) -> object ["outcome" .= ("end" :: String)]
      Right (Just element, _) -> object
        [ "outcome" .= ("value" :: String)
        , "tag" .= tagProjection element
        , "depth" .= asn1ElementDepth element
        ]

runCase :: Value -> Value -> Value -> IO Value
runCase document upstream item = case fieldMaybe "der_tlv_case_id" item of
  Just _ -> verifyUpstream upstream item
  Nothing -> do
    let configured = caseLimits document item
        input = materialize (field "input" item)
        operation = asString (field "operation" item)
        tagNumber = fromIntegral . asWord64 <$> fieldMaybe "tag_number" item
    decoder <- newAsn1Decoder configured
    rootResult <- decodeExact decoder input
    result <- case rootResult of
      Left err -> pure (Left err)
      Right root -> case operation of
        "decode-exact" -> do
          count <- decoderElementsRead decoder
          pure (Right (object
            [ "outcome" .= ("value" :: String)
            , "tag" .= tagProjection root
            , "header_hex" .= toHex (asn1ElementHeader root)
            , "value_hex" .= toHex (asn1ElementValue root)
            , "encoded_hex" .= toHex (asn1ElementEncoded root)
            , "depth" .= asn1ElementDepth root
            , "elements_read" .= count
            ]))
        "cursor-script" -> cursorCase item decoder root
        "sequence" -> containerProjection decoder root sequenceCursor
        "set" -> containerProjection decoder root setCursor
        "explicit" -> do
          decoded <- decodeExplicit decoder root (fromMaybe (error "missing tag_number") tagNumber)
          case decoded of
            Left err -> pure (Left err)
            Right child -> do
              count <- decoderElementsRead decoder
              pure (Right (object
                [ "outcome" .= ("value" :: String)
                , "tag" .= tagProjection child
                , "value_hex" .= toHex (asn1ElementValue child)
                , "depth" .= asn1ElementDepth child
                , "elements_read" .= count
                ]))
        _ -> pure (primitive operation root configured tagNumber)
    pure $ case result of
      Right value -> value
      Left err -> failure err (if operation == "explicit" && isFraming err then "container-value" else "operation-input")
  where
    isFraming err = case asn1ErrorKind err of Framing _ -> True; _ -> False
    containerProjection decoder root opener = case opener decoder root of
      Left err -> pure (Left err)
      Right cursor -> do
        count <- decoderElementsRead decoder
        pure (Right (object
          [ "outcome" .= ("value" :: String)
          , "elements_read" .= count
          , "remaining_offset" .= (BS.length (asn1ElementValue root) - BS.length (asn1CursorRemaining cursor))
          ]))

spec :: Spec
spec = describe "der-asn1-v1 portable fixture" $
  it "matches all 122 closed cases and 46 delegated framing references" $ do
    document <- fixture "der-asn1-v1"
    upstream <- fixture "der-tlv-v1"
    let cases = asArray (field "cases" document)
    length cases `shouldBe` 122
    length (asArray (field "error_ids" document)) `shouldBe` 22
    mapM_ (checkCase document upstream) cases
  where
    checkCase document upstream item = do
      actual <- runCase document upstream item
      actual `shouldBe` field "expected" item
      case fieldMaybe "redacted_input_hex" item of
        Nothing -> pure ()
        Just secret -> show actual `shouldNotContain` asString secret
