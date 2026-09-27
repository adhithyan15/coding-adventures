module X509ExtensionSpec (spec) where

import qualified Data.ByteString as BS
import Test.Hspec
import CodingAdventures.DerAsn1
import CodingAdventures.X509Extension

decodeInput :: [Int] -> IO (Asn1Decoder, Asn1Element)
decodeInput octets = do
  decoder <- newAsn1Decoder defaultAsn1Limits
  root <- decodeExact decoder (BS.pack (map fromIntegral octets))
  case root of
    Left err -> expectationFailure (show err) >> fail "unexpected DER error"
    Right value -> pure (decoder, value)

spec :: Spec
spec = describe "X509 Extension" $ do
  it "keeps validated values immutable and values opaque" $ do
    (decoder, root) <- decodeInput [0x30,0x09,0x06,0x03,0x55,0x1d,0x11,0x04,0x02,0x30,0x00]
    result <- decodeX509Extension decoder root
    case result of
      Left err -> expectationFailure (show err)
      Right value -> do
        map show (oidArcs (extensionId value)) `shouldBe` ["2","5","29","17"]
        critical value `shouldBe` False
        extensionValue value `shouldBe` BS.pack [0x30,0x00]

  it "uses stable error ids and payload-blind rendering" $ do
    map x509ExtensionErrorId
      [ Structure UnexpectedTag
      , MissingExtensionId
      , InvalidExtensionId UnexpectedTag
      , InvalidCritical UnexpectedTag
      , EncodedDefaultCritical
      , MissingExtensionValue
      , InvalidExtensionValue UnexpectedTag
      , TrailingElement
      ] `shouldBe`
      [ "structure"
      , "missing-extension-id"
      , "invalid-extension-id"
      , "invalid-critical"
      , "encoded-default-critical"
      , "missing-extension-value"
      , "invalid-extension-value"
      , "trailing-element"
      ]
    (decoder, root) <- decodeInput [0x30,0x08,0x06,0x01,0x80,0x04,0x03,0xde,0xad,0xbe]
    result <- decodeX509Extension decoder root
    case result of
      Right _ -> expectationFailure "expected invalid OID"
      Left err -> do
        x509ExtensionErrorKind err `shouldBe` InvalidExtensionId NonMinimalObjectIdentifier
        x509ExtensionErrorOffset err `shouldBe` 4
        show err `shouldNotContain` "deadbe"
