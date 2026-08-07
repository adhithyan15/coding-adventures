-- | LZ78 explicit-dictionary compression from CMP01.
--
-- The encoder and decoder independently build the same dictionary. The wire
-- representation stores the original length, a token count, and fixed-width
-- four-byte token records.
module Lz78
    ( Token (..)
    , Lz78Error (..)
    , TrieCursor
    , defaultMaxDictionarySize
    , emptyCursor
    , stepCursor
    , insertCursor
    , resetCursor
    , cursorDictId
    , cursorAtRoot
    , encode
    , decode
    , serialiseTokens
    , deserialiseTokens
    , compress
    , compressDefault
    , decompress
    ) where

import Data.Bits ((.|.), shiftL, shiftR)
import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import qualified Data.IntMap.Strict as IntMap
import Data.IntMap.Strict (IntMap)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Word (Word16, Word32, Word8)

-- | One LZ78 token. Index zero denotes the empty dictionary prefix.
data Token = Token
    { dictIndex :: Word16
    , nextChar :: Word8
    }
    deriving (Eq, Show)

-- | Structural failures detected while reading or decoding a token stream.
data Lz78Error
    = HeaderTooShort Int
    | WireLengthMismatch Int Int
    | NonZeroReservedByte Int Word8
    | InvalidDictionaryIndex Word16 Int
    | InvalidOriginalLength Int
    | DecodedLengthMismatch Int Int
    deriving (Eq, Show)

data TrieNode = TrieNode
    { nodeDictId :: Word16
    , nodeChildren :: Map Word8 Int
    }
    deriving (Eq, Show)

-- | Immutable cursor over an arena-backed byte trie.
data TrieCursor = TrieCursor
    { cursorNodes :: IntMap TrieNode
    , cursorCurrent :: Int
    , cursorNextNode :: Int
    }
    deriving (Eq, Show)

-- | Maximum dictionary size supported by the 16-bit wire index.
defaultMaxDictionarySize :: Int
defaultMaxDictionarySize = 65536

-- | An empty trie whose root represents dictionary index zero.
emptyCursor :: TrieCursor
emptyCursor =
    TrieCursor
        { cursorNodes = IntMap.singleton 0 (TrieNode 0 Map.empty)
        , cursorCurrent = 0
        , cursorNextNode = 1
        }

-- | Follow one byte edge, returning 'Nothing' when the edge is absent.
stepCursor :: Word8 -> TrieCursor -> Maybe TrieCursor
stepCursor byte cursor = do
    node <- IntMap.lookup (cursorCurrent cursor) (cursorNodes cursor)
    child <- Map.lookup byte (nodeChildren node)
    pure cursor{cursorCurrent = child}

-- | Add a byte edge at the current node. The cursor does not advance.
insertCursor :: Word8 -> Word16 -> TrieCursor -> TrieCursor
insertCursor byte newDictId cursor =
    cursor
        { cursorNodes =
            IntMap.insert newNodeIndex newNode
                (IntMap.adjust addChild (cursorCurrent cursor) (cursorNodes cursor))
        , cursorNextNode = newNodeIndex + 1
        }
  where
    newNodeIndex = cursorNextNode cursor
    newNode = TrieNode newDictId Map.empty
    addChild node = node{nodeChildren = Map.insert byte newNodeIndex (nodeChildren node)}

-- | Return a cursor to the trie root without changing its dictionary.
resetCursor :: TrieCursor -> TrieCursor
resetCursor cursor = cursor{cursorCurrent = 0}

-- | Dictionary ID at the current cursor position.
cursorDictId :: TrieCursor -> Word16
cursorDictId cursor = maybe 0 nodeDictId (IntMap.lookup (cursorCurrent cursor) (cursorNodes cursor))

-- | Whether the cursor currently points at the trie root.
cursorAtRoot :: TrieCursor -> Bool
cursorAtRoot cursor = cursorCurrent cursor == 0

-- | Encode bytes as LZ78 tokens, capping the dictionary at the requested size.
encode :: ByteString -> Int -> [Token]
encode input maxDictionarySize = reverse (finish finalCursor reversedTokens)
  where
    cappedSize = max 1 (min defaultMaxDictionarySize maxDictionarySize)
    (finalCursor, _, reversedTokens) = BS.foldl' encodeByte (emptyCursor, 1, []) input

    encodeByte (cursor, nextId, tokens) byte =
        case stepCursor byte cursor of
            Just advanced -> (advanced, nextId, tokens)
            Nothing ->
                let token = Token (cursorDictId cursor) byte
                    canInsert = nextId < cappedSize
                    inserted =
                        if canInsert
                            then insertCursor byte (fromIntegral nextId) cursor
                            else cursor
                    followingId = if canInsert then nextId + 1 else nextId
                 in (resetCursor inserted, followingId, token : tokens)

    finish cursor tokens
        | cursorAtRoot cursor = tokens
        | otherwise = Token (cursorDictId cursor) 0 : tokens

-- | Decode tokens, optionally enforcing and trimming to an original length.
decode :: [Token] -> Maybe Int -> Either Lz78Error ByteString
decode tokens originalLength = do
    validateLength originalLength
    (outputReversed, _, _) <- foldTokens tokens
    let output = BS.pack (reverse outputReversed)
        trimmed = maybe output (`BS.take` output) originalLength
        actualLength = BS.length trimmed
    case originalLength of
        Just expected
            | actualLength /= expected -> Left (DecodedLengthMismatch expected actualLength)
        _ -> Right trimmed
  where
    validateLength (Just value)
        | value < 0 = Left (InvalidOriginalLength value)
    validateLength _ = Right ()

    foldTokens = go (IntMap.singleton 0 (0, 0)) 1 [] 0

    go table _ output outputLength [] = Right (output, outputLength, table)
    go table nextId output outputLength (token : rest) = do
        prefix <- reconstruct table nextId (dictIndex token)
        let withPrefix = reverse prefix ++ output
            prefixLength = outputLength + length prefix
            includeNext = maybe True (prefixLength <) originalLength
            withNext = if includeNext then nextChar token : withPrefix else withPrefix
            nextLength = if includeNext then prefixLength + 1 else prefixLength
            nextTable = IntMap.insert nextId (dictIndex token, nextChar token) table
        go nextTable (nextId + 1) withNext nextLength rest

-- | Rebuild one dictionary entry by following its parent chain.
reconstruct :: IntMap (Word16, Word8) -> Int -> Word16 -> Either Lz78Error [Word8]
reconstruct table nextId index = walk index []
  where
    walk 0 bytes = Right bytes
    walk current bytes =
        case IntMap.lookup (fromIntegral current) table of
            Nothing -> Left (InvalidDictionaryIndex current nextId)
            Just (parent, byte) -> walk parent (byte : bytes)

-- | Serialize tokens using the CMP01 big-endian fixed-width wire format.
serialiseTokens :: [Token] -> Int -> ByteString
serialiseTokens tokens originalLength =
    BS.pack
        ( word32Bytes (fromIntegral originalLength)
            ++ word32Bytes (fromIntegral (length tokens))
            ++ concatMap tokenBytes tokens
        )
  where
    tokenBytes token = word16Bytes (dictIndex token) ++ [nextChar token, 0]

-- | Parse and validate a complete CMP01 wire stream.
deserialiseTokens :: ByteString -> Either Lz78Error ([Token], Int)
deserialiseTokens input
    | actualLength < 8 = Left (HeaderTooShort actualLength)
    | actualLength /= expectedLength = Left (WireLengthMismatch expectedLength actualLength)
    | otherwise = do
        tokens <- traverse readToken [0 .. tokenCount - 1]
        Right (tokens, originalLength)
  where
    actualLength = BS.length input
    originalLength = fromIntegral (readWord32 input 0)
    tokenCount = fromIntegral (readWord32 input 4)
    expectedLength = 8 + tokenCount * 4

    readToken tokenNumber =
        let offset = 8 + tokenNumber * 4
            reserved = BS.index input (offset + 3)
         in if reserved /= 0
                then Left (NonZeroReservedByte tokenNumber reserved)
                else
                    Right
                        Token
                            { dictIndex = readWord16 input offset
                            , nextChar = BS.index input (offset + 2)
                            }

-- | Encode and serialize bytes with an explicit dictionary cap.
compress :: ByteString -> Int -> ByteString
compress input maxDictionarySize =
    serialiseTokens (encode input maxDictionarySize) (BS.length input)

-- | Encode and serialize bytes with the 65,536-entry default dictionary.
compressDefault :: ByteString -> ByteString
compressDefault input = compress input defaultMaxDictionarySize

-- | Parse and decode a complete CMP01 wire stream.
decompress :: ByteString -> Either Lz78Error ByteString
decompress input = do
    (tokens, originalLength) <- deserialiseTokens input
    decode tokens (Just originalLength)

word16Bytes :: Word16 -> [Word8]
word16Bytes value =
    [ fromIntegral (value `shiftR` 8)
    , fromIntegral value
    ]

word32Bytes :: Word32 -> [Word8]
word32Bytes value =
    [ fromIntegral (value `shiftR` 24)
    , fromIntegral (value `shiftR` 16)
    , fromIntegral (value `shiftR` 8)
    , fromIntegral value
    ]

readWord16 :: ByteString -> Int -> Word16
readWord16 input offset =
    (fromIntegral (BS.index input offset) `shiftL` 8)
        .|. fromIntegral (BS.index input (offset + 1))

readWord32 :: ByteString -> Int -> Word32
readWord32 input offset =
    (fromIntegral (BS.index input offset) `shiftL` 24)
        .|. (fromIntegral (BS.index input (offset + 1)) `shiftL` 16)
        .|. (fromIntegral (BS.index input (offset + 2)) `shiftL` 8)
        .|. fromIntegral (BS.index input (offset + 3))
