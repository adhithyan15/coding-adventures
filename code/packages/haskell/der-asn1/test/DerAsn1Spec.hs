module DerAsn1Spec (spec) where

import qualified Data.ByteString as BS
import Control.Concurrent (forkIO, newEmptyMVar, putMVar, takeMVar)
import Control.Monad (replicateM)
import Data.Either (isLeft)
import Test.Hspec
import CodingAdventures.DerAsn1
import qualified CodingAdventures.DerTlv as DT

decode :: [Word] -> IO (Asn1Decoder, Asn1Element)
decode octets = do
  decoder <- newAsn1Decoder defaultAsn1Limits
  result <- decodeExact decoder (BS.pack (map fromIntegral octets))
  element <- unwrap result
  pure (decoder, element)

unwrap :: Show error => Either error value -> IO value
unwrap result = case result of
  Left err -> expectationFailure (show err) >> fail "unexpected Left"
  Right value -> pure value

shouldHaveKind :: Either Asn1Error value -> Asn1ErrorKind -> Expectation
shouldHaveKind result expected =
  case result of
    Left err -> asn1ErrorKind err `shouldBe` expected
    Right _ -> expectationFailure "operation unexpectedly succeeded"

spec :: Spec
spec = describe "CodingAdventures.DerAsn1" $ do
  it "decodes canonical primitive values" $ do
    (_, boolean) <- decode [0x01, 0x01, 0xff]
    unwrap (decodeBoolean boolean) `shouldReturn` True
    (_, integerElement) <- decode [0x02, 0x02, 0x00, 0x80]
    integer <- unwrap (decodeInteger integerElement)
    unwrap (integerToWord64 integer) `shouldReturn` 128
    (_, bitsElement) <- decode [0x03, 0x02, 0x03, 0xa8]
    bits <- unwrap (decodeBitString bitsElement)
    bitStringLength bits `shouldBe` 5

  it "rejects non-canonical primitive values" $ do
    (_, boolean) <- decode [0x01, 0x01, 0x01]
    decodeBoolean boolean `shouldSatisfy` isLeft
    (_, integer) <- decode [0x02, 0x02, 0x00, 0x7f]
    decodeInteger integer `shouldHaveKind` NonMinimalInteger
    (_, bits) <- decode [0x03, 0x02, 0x03, 0xab]
    decodeBitString bits `shouldHaveKind` NonZeroBitPadding

  it "binds cursors to the exact originating decoder" $ do
    (decoder, root) <- decode [0x30, 0x02, 0x05, 0x00]
    cursor <- unwrap (sequenceCursor decoder root)
    other <- newAsn1Decoder defaultAsn1Limits
    result <- readAsn1Cursor other cursor
    result `shouldHaveKind` DecoderLimitMismatch
    case result of
      Left err -> do
        show err `shouldContain` "DecoderLimitMismatch"
        show err `shouldContain` "at byte 0"
      Right _ -> expectationFailure "foreign decoder accepted"

  it "retains exact ownership through nested cursors" $ do
    (decoder, root) <- decode [0x30, 0x04, 0x30, 0x02, 0x05, 0x00]
    outer <- unwrap (sequenceCursor decoder root)
    outerRead <- readAsn1Cursor decoder outer
    (child, _) <- unwrap outerRead
    inner <- unwrap (sequenceCursor decoder (fromMaybeElement child))
    innerRead <- readAsn1Cursor decoder inner
    case innerRead of
      Right (Just _, _) -> pure ()
      _ -> expectationFailure "nested cursor did not preserve owner"

  it "shares the total element budget transactionally" $ do
    let limits = defaultAsn1Limits {maxTotalElements = 2}
    decoder <- newAsn1Decoder limits
    rootResult <- decodeExact decoder (BS.pack [0x30, 0x06, 0x05, 0x00, 0x05, 0x00, 0x05, 0x00])
    root <- unwrap rootResult
    cursor <- unwrap (sequenceCursor decoder root)
    first <- readAsn1Cursor decoder cursor
    (_, next) <- unwrap first
    countBefore <- decoderElementsRead decoder
    failed <- readAsn1Cursor decoder next
    failed `shouldHaveKind` ElementLimitExceeded
    decoderElementsRead decoder `shouldReturn` countBefore

  it "keeps empty, malformed, and unfinished cursors transactional" $ do
    (emptyDecoder, emptyRoot) <- decode [0x30, 0x00]
    emptyCursor <- unwrap (sequenceCursor emptyDecoder emptyRoot)
    emptyRead <- readAsn1Cursor emptyDecoder emptyCursor
    case emptyRead of
      Right (Nothing, _) -> pure ()
      _ -> expectationFailure "empty cursor did not end"
    (unfinishedDecoder, unfinishedRoot) <- decode [0x30, 0x02, 0x05, 0x00]
    unfinished <- unwrap (sequenceCursor unfinishedDecoder unfinishedRoot)
    finishAsn1Cursor unfinished `shouldHaveKind` Framing DT.TrailingData
    (malformedDecoder, malformedRoot) <- decode [0x30, 0x02, 0x02, 0x01]
    malformed <- unwrap (sequenceCursor malformedDecoder malformedRoot)
    beforeMalformed <- decoderElementsRead malformedDecoder
    malformedRead <- readAsn1Cursor malformedDecoder malformed
    malformedRead `shouldHaveKind` Framing DT.TruncatedValue
    decoderElementsRead malformedDecoder `shouldReturn` beforeMalformed

  it "applies explicit work limits before child framing" $ do
    let limits = defaultAsn1Limits {maxTotalElements = 1}
    decoder <- newAsn1Decoder limits
    root <- decodeExact decoder (BS.pack [0xa3, 0x03, 0x02, 0x01, 0x01]) >>= unwrap
    result <- decodeExplicit decoder root 3
    result `shouldHaveKind` ElementLimitExceeded

  it "enforces the shared budget atomically across concurrent callers" $ do
    decoder <- newAsn1Decoder (defaultAsn1Limits {maxTotalElements = 1})
    outputs <- replicateM 24 newEmptyMVar
    mapM_ (\output -> forkIO (decodeExact decoder (BS.pack [0x05, 0x00]) >>= putMVar output)) outputs
    results <- mapM takeMVar outputs
    length [() | Right _ <- results] `shouldBe` 1
    decoderElementsRead decoder `shouldReturn` 1

  it "exposes immutable value projections and stable diagnostics" $ do
    version `shouldBe` "0.1.0"
    (defaultAsn1Limits == defaultAsn1Limits) `shouldBe` True
    (defaultAsn1Limits /= defaultAsn1Limits {maxDepth = 1}) `shouldBe` True
    let kinds =
          [ Framing DT.EmptyInput, UnexpectedTag, DecoderLimitMismatch
          , DepthLimitExceeded, ElementLimitExceeded, InvalidBooleanLength
          , InvalidBooleanValue, EmptyInteger, NonMinimalInteger
          , NegativeInteger, IntegerOverflow, MissingUnusedBitCount
          , InvalidUnusedBitCount, NonZeroBitPadding, BitLengthOverflow
          , NonEmptyNull, NonAsciiIa5String, EmptyObjectIdentifier
          , UnterminatedObjectIdentifier, NonMinimalObjectIdentifier
          , ObjectIdentifierOverflow, OidArcLimitExceeded
          ]
    map asn1ErrorId kinds `shouldSatisfy` all (not . null)
    map show kinds `shouldSatisfy` all (not . null)
    and (zipWith (==) kinds kinds) `shouldBe` True
    UnexpectedTag /= DecoderLimitMismatch `shouldBe` True
    show kinds `shouldContain` "UnexpectedTag"
    showsPrec 1 UnexpectedTag "" `shouldBe` "UnexpectedTag"
    (_, integerElement) <- decode [0x02, 0x01, 0x01]
    integer <- unwrap (decodeInteger integerElement)
    integerSignedBytes integer `shouldBe` BS.pack [1]
    (_, bitsElement) <- decode [0x03, 0x02, 0x00, 0xa5]
    bits <- unwrap (decodeBitString bitsElement)
    bitStringBytes bits `shouldBe` BS.pack [0xa5]
    (_, oidElement) <- decode [0x06, 0x03, 0x2a, 0x03, 0x04]
    oid <- unwrap (decodeObjectIdentifier oidElement defaultAsn1Limits)
    oidEquals oid [1, 2, 3, 4] `shouldBe` True
    oidArcCount oid `shouldBe` 4
    (_, zeroFirstElement) <- decode [0x06, 0x01, 0x03]
    zeroFirst <- unwrap (decodeObjectIdentifier zeroFirstElement defaultAsn1Limits)
    oidArcs zeroFirst `shouldBe` [0, 3]
    decodeObjectIdentifier zeroFirstElement (defaultAsn1Limits {maxOidArcs = 1})
      `shouldHaveKind` OidArcLimitExceeded

fromMaybeElement :: Maybe Asn1Element -> Asn1Element
fromMaybeElement value = case value of
  Just element -> element
  Nothing -> error "expected element"
