{-# LANGUAGE OverloadedStrings #-}

module PortableConformanceSpec (spec) where

import Data.Aeson (Value (..), eitherDecodeFileStrict', object, (.=))
import qualified Data.Aeson.Key as Key
import qualified Data.Aeson.KeyMap as KeyMap
import qualified Data.ByteString as BS
import Data.Char (digitToInt)
import Data.Foldable (toList)
import Data.Maybe (fromMaybe)
import Data.Scientific (toBoundedInteger)
import qualified Data.Text as T
import Data.Word (Word64)
import Numeric (showHex)
import Test.Hspec
import CodingAdventures.DerAsn1
import qualified CodingAdventures.DerTlv as DT
import CodingAdventures.X509Extension

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
        in if BS.length octets == 1
             then BS.replicate count (BS.head octets)
             else error "repeat_hex must be one octet"

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
caseLimits upstream item = Asn1Limits
  { asn1DerLimits = derLimits (field "der" defaults) (overrides >>= fieldMaybe "der")
  , maxDepth = asWord64 (override defaults overrides "max_depth")
  , maxTotalElements = asWord64 (override defaults overrides "max_total_elements")
  , maxOidArcs = asWord64 (override defaults overrides "max_oid_arcs")
  }
  where
    defaults = field "defaults" upstream
    overrides = fieldMaybe "limits" item

errorAsn1Kind :: X509ExtensionErrorKind -> Maybe Asn1ErrorKind
errorAsn1Kind kind = case kind of
  Structure value -> Just value
  InvalidExtensionId value -> Just value
  InvalidCritical value -> Just value
  InvalidExtensionValue value -> Just value
  _ -> Nothing

attempt :: Asn1Decoder -> Asn1Element -> IO Value
attempt decoder root = do
  result <- decodeX509Extension decoder root
  count <- decoderElementsRead decoder
  pure $ case result of
    Right value -> object
      [ "outcome" .= ("value" :: String)
      , "extension_id_arcs_decimal" .= map show (oidArcs (extensionId value))
      , "critical" .= critical value
      , "extension_value_hex" .= toHex (extensionValue value)
      , "elements_read" .= count
      ]
    Left err -> object (base ++ asn1Fields)
      where
        kind = x509ExtensionErrorKind err
        base =
          [ "outcome" .= ("error" :: String)
          , "error_id" .= x509ExtensionErrorId kind
          , "offset" .= x509ExtensionErrorOffset err
          , "offset_scope" .= ("extension-element" :: String)
          , "elements_read" .= count
          ]
        asn1Fields = case errorAsn1Kind kind of
          Nothing -> []
          Just asn1Kind ->
            ["asn1_error_id" .= asn1ErrorId asn1Kind] ++ case asn1Kind of
              Framing framingKind -> ["framing_error_id" .= DT.errorId framingKind]
              _ -> []

runCase :: Value -> Value -> IO Value
runCase upstream item = do
  let input = materialize (field "input" item)
      operation = asString (field "operation" item)
  decoder <- newAsn1Decoder (caseLimits upstream item)
  rootResult <- decodeExact decoder input
  root <- case rootResult of
    Left err -> expectationFailure (show err) >> fail "fixture root failed"
    Right value -> pure value
  if operation == "extension-script"
    then do
      events <- mapM (const (attempt decoder root)) (asArray (field "actions" item))
      pure (object ["outcome" .= ("script" :: String), "events" .= events])
    else attempt decoder root

spec :: Spec
spec = describe "x509-extension-v1 portable fixture" $
  it "matches all 48 cases and eight stable error categories" $ do
    contract <- fixture "x509-extension-v1"
    upstream <- fixture "der-asn1-v1"
    let cases = asArray (field "cases" contract)
    length cases `shouldBe` 48
    length (asArray (field "error_ids" contract)) `shouldBe` 8
    mapM_ (checkCase upstream) cases
  where
    checkCase upstream item = do
      actual <- runCase upstream item
      actual `shouldBe` field "expected" item
      case fieldMaybe "redacted_input_hex" item of
        Nothing -> pure ()
        Just secret -> show actual `shouldNotContain` asString secret
