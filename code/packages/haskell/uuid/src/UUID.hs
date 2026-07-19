-- | UUID generation and parsing according to RFC 4122 and RFC 9562.
module UUID
    ( UUID
    , UUIDError (..)
    , uuidFromBytes
    , uuidBytes
    , uuidFromInteger
    , uuidToInteger
    , parse
    , render
    , isValid
    , uuidVersion
    , uuidVariant
    , isNil
    , isMax
    , namespaceDNS
    , namespaceURL
    , namespaceOID
    , namespaceX500
    , nilUUID
    , maxUUID
    , v1
    , v3
    , v4
    , v5
    , v7
    ) where

import Data.Bits ((.&.), (.|.), shiftL, shiftR)
import Data.Char (isHexDigit, isSpace, ord, toLower)
import Data.List (foldl')
import Data.Word (Word8, Word16, Word64)
import qualified Md5
import Numeric (readHex, showHex)
import qualified Sha1
import System.Random (randomIO)
import Data.Time.Clock.POSIX (getPOSIXTime)

-- | A validated 128-bit UUID in network byte order.
newtype UUID = UUID [Word8]
    deriving (Eq, Ord)

-- | Error returned for malformed byte, integer, or text input.
newtype UUIDError = UUIDError {errorMessage :: String}
    deriving (Eq, Show)

instance Show UUID where
    show uuid = "UUID(\"" ++ render uuid ++ "\")"

-- | Construct a UUID from exactly sixteen bytes.
uuidFromBytes :: [Word8] -> Either UUIDError UUID
uuidFromBytes bytes
    | length bytes == 16 = Right (UUID bytes)
    | otherwise = Left (UUIDError ("UUID bytes must be exactly 16, got " ++ show (length bytes)))

-- | Return the UUID's sixteen bytes in network byte order.
uuidBytes :: UUID -> [Word8]
uuidBytes (UUID bytes) = bytes

-- | Construct a UUID from an unsigned 128-bit integer.
uuidFromInteger :: Integer -> Either UUIDError UUID
uuidFromInteger value
    | value < 0 || value >= 2 ^ (128 :: Int) = Left (UUIDError "UUID integer must be in the range 0 through 2^128 - 1")
    | otherwise = Right (UUID [fromIntegral (value `shiftR` shift) | shift <- [120, 112 .. 0]])

-- | Convert a UUID to its unsigned 128-bit integer representation.
uuidToInteger :: UUID -> Integer
uuidToInteger = foldl' (\value byte -> value `shiftL` 8 + fromIntegral byte) 0 . uuidBytes

-- | Parse canonical, compact, braced, or URN UUID text.
parse :: String -> Either UUIDError UUID
parse input = do
    body <- unwrap (trim input)
    digits <- groupedHex body
    bytes <- traverse parsePair (pairs digits)
    uuidFromBytes bytes
  where
    invalid = Left (UUIDError ("Invalid UUID string: '" ++ input ++ "'"))

    unwrap value =
        let withoutUrn =
                if map toLower (take 9 value) == "urn:uuid:"
                    then drop 9 value
                    else value
         in case withoutUrn of
                ('{' : rest)
                    | not (null rest) && last rest == '}' -> Right (init rest)
                    | otherwise -> invalid
                _
                    | '}' `elem` withoutUrn || '{' `elem` withoutUrn -> invalid
                    | otherwise -> Right withoutUrn

    groupedHex value =
        case consumeGroups [8, 4, 4, 4, 12] value of
            Just digits
                | length digits == 32 && all isHexDigit digits -> Right digits
            _ -> invalid

    parsePair pair =
        case readHex pair of
            [(value, "")] -> Right (fromIntegral (value :: Int))
            _ -> invalid

-- | Render lowercase canonical 8-4-4-4-12 UUID text.
render :: UUID -> String
render uuid = joinGroups [8, 4, 4, 4, 12] hex
  where
    hex = concatMap byteHex (uuidBytes uuid)

-- | Return whether text parses as a UUID.
isValid :: String -> Bool
isValid value =
    case parse value of
        Right _ -> True
        Left _ -> False

-- | Read the four-bit version field.
uuidVersion :: UUID -> Int
uuidVersion uuid = fromIntegral ((uuidBytes uuid !! 6 `shiftR` 4) .&. 0x0f)

-- | Classify the UUID variant field.
uuidVariant :: UUID -> String
uuidVariant uuid
    | byte .&. 0x80 == 0 = "ncs"
    | byte .&. 0xc0 == 0x80 = "rfc4122"
    | byte .&. 0xe0 == 0xc0 = "microsoft"
    | otherwise = "reserved"
  where
    byte = uuidBytes uuid !! 8

-- | Return whether all UUID bits are zero.
isNil :: UUID -> Bool
isNil = all (== 0) . uuidBytes

-- | Return whether all UUID bits are one.
isMax :: UUID -> Bool
isMax = all (== 0xff) . uuidBytes

namespaceDNS, namespaceURL, namespaceOID, namespaceX500, nilUUID, maxUUID :: UUID
namespaceDNS = literal "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
namespaceURL = literal "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
namespaceOID = literal "6ba7b812-9dad-11d1-80b4-00c04fd430c8"
namespaceX500 = literal "6ba7b814-9dad-11d1-80b4-00c04fd430c8"
nilUUID = UUID (replicate 16 0)
maxUUID = UUID (replicate 16 0xff)

-- | Generate an RFC 4122 version 1 UUID.
v1 :: IO UUID
v1 = do
    now <- getPOSIXTime
    clockSequence <- ((.&. 0x3fff) <$> (randomIO :: IO Word16))
    node <- randomBytes 6
    let timestamp = fromIntegral (floor (now * 10000000) :: Integer) + gregorianOffset
        timeLow = timestamp .&. 0xffffffff
        timeMid = (timestamp `shiftR` 32) .&. 0xffff
        timeHigh = (timestamp `shiftR` 48) .&. 0x0fff
        nodeWithMulticast = ((head node .|. 0x01) : tail node)
        bytes =
            wordBytes 4 timeLow
                ++ wordBytes 2 timeMid
                ++ wordBytes 2 (timeHigh .|. 0x1000)
                ++ [ 0x80 .|. fromIntegral (clockSequence `shiftR` 8)
                   , fromIntegral clockSequence
                   ]
                ++ nodeWithMulticast
    pure (UUID bytes)

-- | Generate an RFC 4122 version 3 (MD5 name-based) UUID.
v3 :: UUID -> String -> UUID
v3 namespace name = UUID (setVersionVariant 3 (Md5.sumMd5 input))
  where
    input = uuidBytes namespace ++ utf8Bytes name

-- | Generate an RFC 4122 version 4 random UUID.
v4 :: IO UUID
v4 = UUID . setVersionVariant 4 <$> randomBytes 16

-- | Generate an RFC 4122 version 5 (SHA-1 name-based) UUID.
v5 :: UUID -> String -> UUID
v5 namespace name = UUID (setVersionVariant 5 (take 16 (Sha1.sha1 input)))
  where
    input = uuidBytes namespace ++ utf8Bytes name

-- | Generate an RFC 9562 version 7 Unix-millisecond UUID.
v7 :: IO UUID
v7 = do
    now <- getPOSIXTime
    random <- randomBytes 10
    let timestamp = fromIntegral (floor (now * 1000) :: Integer) :: Word64
        bytes =
            wordBytes 6 timestamp
                ++ [0x70 .|. (random !! 0 .&. 0x0f), random !! 1]
                ++ [0x80 .|. (random !! 2 .&. 0x3f)]
                ++ drop 3 random
    pure (UUID bytes)

gregorianOffset :: Word64
gregorianOffset = 122192928000000000

setVersionVariant :: Word8 -> [Word8] -> [Word8]
setVersionVariant version bytes =
    take 6 bytes
        ++ [bytes !! 6 .&. 0x0f .|. (version `shiftL` 4)]
        ++ [bytes !! 7]
        ++ [bytes !! 8 .&. 0x3f .|. 0x80]
        ++ drop 9 bytes

randomBytes :: Int -> IO [Word8]
randomBytes count = sequence (replicate count randomIO)

wordBytes :: Int -> Word64 -> [Word8]
wordBytes count value =
    [fromIntegral (value `shiftR` shift) | shift <- [8 * (count - 1), 8 * (count - 2) .. 0]]

utf8Bytes :: String -> [Word8]
utf8Bytes = concatMap encode
  where
    encode char
        | code <= 0x7f = [fromIntegral code]
        | code <= 0x7ff =
            [ fromIntegral (0xc0 .|. code `shiftR` 6)
            , fromIntegral (0x80 .|. code .&. 0x3f)
            ]
        | code <= 0xffff =
            [ fromIntegral (0xe0 .|. code `shiftR` 12)
            , fromIntegral (0x80 .|. code `shiftR` 6 .&. 0x3f)
            , fromIntegral (0x80 .|. code .&. 0x3f)
            ]
        | otherwise =
            [ fromIntegral (0xf0 .|. code `shiftR` 18)
            , fromIntegral (0x80 .|. code `shiftR` 12 .&. 0x3f)
            , fromIntegral (0x80 .|. code `shiftR` 6 .&. 0x3f)
            , fromIntegral (0x80 .|. code .&. 0x3f)
            ]
      where
        code = ord char

consumeGroups :: [Int] -> String -> Maybe String
consumeGroups [] "" = Just ""
consumeGroups [] _ = Nothing
consumeGroups [size] value
    | length value == size = Just value
    | otherwise = Nothing
consumeGroups (size : remainingSizes) value = do
    let (group, rest) = splitAt size value
    if length group /= size
        then Nothing
        else do
            suffix <- consumeGroups remainingSizes (dropOptionalHyphen rest)
            Just (group ++ suffix)

dropOptionalHyphen :: String -> String
dropOptionalHyphen ('-' : rest) = rest
dropOptionalHyphen value = value

pairs :: [a] -> [[a]]
pairs [] = []
pairs (first : second : rest) = [first, second] : pairs rest
pairs _ = []

trim :: String -> String
trim = reverse . dropWhile isSpace . reverse . dropWhile isSpace

byteHex :: Word8 -> String
byteHex byte =
    let value = showHex byte ""
     in if length value == 1 then '0' : value else value

joinGroups :: [Int] -> String -> String
joinGroups [] _ = ""
joinGroups [size] value = take size value
joinGroups (size : rest) value = take size value ++ "-" ++ joinGroups rest (drop size value)

literal :: String -> UUID
literal value =
    case parse value of
        Right uuid -> uuid
        Left err -> error (errorMessage err)
