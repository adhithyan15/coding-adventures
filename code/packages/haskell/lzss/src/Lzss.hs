-- | LZSS lossless compression algorithm (1982) — CMP02.
--
-- LZSS (Lempel-Ziv-Storer-Szymanski) is a refinement of LZ77 that replaces
-- the mandatory @next_char@ byte after every token with a __flag-bit scheme__:
--
-- * @Literal@ → 1 byte  (flag bit = 0)
-- * @Match@   → 3 bytes (flag bit = 1: offset u16 BE + length u8)
--
-- Tokens are grouped in blocks of 8. Each block is preceded by a 1-byte flag
-- word whose bits describe the 8 tokens that follow (LSB = first token).
--
-- == Wire Format (CMP02)
--
-- @
-- Bytes 0–3:  original_length  (big-endian u32)
-- Bytes 4–7:  block_count      (big-endian u32)
-- Bytes 8+:   blocks
--   Each block: [1-byte flag] [1 or 3 bytes per symbol]
-- @
--
-- == Series
--
-- @
-- CMP00 (LZ77,    1977) — Sliding-window back-references.
-- CMP01 (LZ78,    1978) — Explicit dictionary (trie).
-- CMP02 (LZSS,    1982) — LZ77 + flag bits. ← this package
-- CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
-- CMP04 (Huffman, 1952) — Entropy coding.
-- CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
-- @
--
-- == Example
--
-- @
-- import Lzss (compress, decompress)
-- import qualified Data.ByteString.Char8 as BC
--
-- let bs = BC.pack "hello hello hello"
-- decompress (compress bs) == bs  -- True
-- @

module Lzss
    ( -- * Token type
      Token(..)
      -- * Low-level API
    , encode
    , decode
      -- * High-level API (CMP02 wire format)
    , compress
    , decompress
      -- * Default parameters
    , defaultWindowSize
    , defaultMaxMatch
    , defaultMinMatch
    ) where

import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Word (Word8, Word16, Word32)
import Data.Bits ((.&.), (.|.), shiftL, shiftR, testBit, setBit)

-- ─── Token type ───────────────────────────────────────────────────────────────

-- | A single LZSS token.
--
-- During encoding we produce a list of these tokens that together represent the
-- original data. There are only two possibilities:
--
-- * 'Literal' — the byte had no useful match in the look-back window, so we
--   store it verbatim. Costs 1 byte in the wire format.
-- * 'Match' — a previous occurrence of the same byte run was found. Instead of
--   repeating the bytes, we store how far back to look ('offset') and how many
--   bytes to copy ('matchLength'). Costs 3 bytes (u16 offset + u8 length).
data Token
    = Literal !Word8
      -- ^ A raw byte with no match in the sliding window.
    | Match { offset :: !Int, matchLength :: !Int }
      -- ^ A back-reference: copy 'matchLength' bytes from 'offset' positions
      --   back in the already-decoded output.
    deriving (Show, Eq)

-- ─── Default parameters ───────────────────────────────────────────────────────

-- | The default sliding-window size (4096 bytes).
--
-- Larger windows allow longer back-references but require the decoder to keep
-- more context in memory. 4096 is the traditional LZSS default that balances
-- compression ratio against memory use.
defaultWindowSize :: Int
defaultWindowSize = 4096

-- | Maximum match length (255, fitting in a single @u8@).
--
-- We store the match length as a single byte in the wire format, so 255 is the
-- highest value we can represent directly.
defaultMaxMatch :: Int
defaultMaxMatch = 255

-- | Minimum match length (3 bytes) — the break-even point.
--
-- A 'Match' costs 3 bytes in the wire format (2-byte offset + 1-byte length).
-- A run of 3 literals also costs 3 bytes. So a match of length 3 breaks even;
-- length 4+ saves space. We traditionally use 3 as the threshold, following
-- the original paper's convention.
defaultMinMatch :: Int
defaultMinMatch = 3

-- ─── Encoder ─────────────────────────────────────────────────────────────────

-- | Scan the look-back window for the longest match at position @cursor@.
--
-- We compare @data[cursor..]@ against every position in @data[winStart..cursor-1]@.
-- The comparison can run past @cursor@ (overlapping match), which is intentional:
-- a self-referential copy like @Match(offset=1, length=6)@ encodes a run of
-- identical bytes.
--
-- Returns @(bestOffset, bestLength)@ where @bestOffset@ is the distance back
-- from @cursor@ to the start of the best match. Returns @(0, 0)@ if no match
-- was found.
findLongestMatch
    :: ByteString  -- ^ Full input data
    -> Int         -- ^ Current cursor position
    -> Int         -- ^ Start of the sliding window (= max 0 (cursor - windowSize))
    -> Int         -- ^ Maximum match length to consider
    -> (Int, Int)  -- ^ (offset, length)
findLongestMatch !bytes !cursor !winStart !maxMatch =
    go winStart 0 0
  where
    !n            = BS.length bytes
    !lookaheadEnd = min (cursor + maxMatch) n

    -- Try every window position from winStart up to cursor-1 and track the
    -- longest match found so far.
    go !pos !bestLen !bestOff
        | pos >= cursor = (bestOff, bestLen)
        | otherwise     =
            let len = matchLen pos 0
            in if len > bestLen
               then go (pos + 1) len (cursor - pos)
               else go (pos + 1) bestLen bestOff

    -- Count how many bytes match starting at window position @wpos@ and
    -- cursor + @k@. The window position can read bytes that were just appended
    -- to the window (overlapping), which is correct: the decoder mirrors this
    -- byte-by-byte copy.
    matchLen !wpos !k
        | cursor + k >= lookaheadEnd       = k
        | BS.index bytes (wpos + k) /=
          BS.index bytes (cursor + k)      = k
        | otherwise                        = matchLen wpos (k + 1)

-- | Encode a 'ByteString' into a list of 'Token's using a sliding-window
-- greedy match strategy.
--
-- For each byte position @cursor@ in the input we:
--
-- 1. Search @data[max 0 (cursor - windowSize) .. cursor - 1]@ for the longest
--    run of bytes that matches @data[cursor..]@.
-- 2. If the longest match is at least @minMatch@ bytes long, emit a 'Match'
--    token and advance @cursor@ by the match length.
-- 3. Otherwise emit a 'Literal' token for the current byte and advance by 1.
--
-- The choice of @minMatch = 3@ means a match must cover at least 3 bytes to
-- be worth encoding as a 3-byte back-reference instead of 3 literal bytes.
encode
    :: Int        -- ^ Window size (look-back distance)
    -> Int        -- ^ Maximum match length
    -> Int        -- ^ Minimum match length (matches shorter than this become literals)
    -> ByteString -- ^ Input data
    -> [Token]
encode windowSize maxMatch minMatch bytes =
    go 0
  where
    !n = BS.length bytes

    go !cursor
        | cursor >= n = []
        | otherwise   =
            let !winStart       = max 0 (cursor - windowSize)
                !(off, len)     = findLongestMatch bytes cursor winStart maxMatch
            in if len >= minMatch
               then Match off len : go (cursor + len)
               else Literal (BS.index bytes cursor) : go (cursor + 1)

-- ─── Decoder ─────────────────────────────────────────────────────────────────

-- | Reconstruct bytes from a list of 'Token's.
--
-- This is the inverse of 'encode':
--
-- * 'Literal' b → append @b@ to the output buffer.
-- * 'Match' off len → copy @len@ bytes starting @off@ positions back in the
--   output buffer. The copy is done __byte-by-byte__ so overlapping matches
--   (where @off < len@) work correctly — each newly-appended byte may itself be
--   the source for the next byte in the copy.
--
-- For example, @Match(offset=1, length=6)@ applied to a buffer containing
-- just @[\'A\']@ produces @[\'A\',\'A\',\'A\',\'A\',\'A\',\'A\',\'A\']@ because each byte
-- copy sees the previous copy result.
decode :: [Token] -> ByteString
decode tokens =
    BS.pack (go tokens [])
  where
    -- Accumulate output as a reversed list for O(1) append, then we reverse
    -- once at the end.  We pass the normal-order prefix for back-reference
    -- lookups to avoid a reverse on every Match.
    go [] acc = reverse acc
    go (tok:rest) acc =
        case tok of
            Literal b   -> go rest (b : acc)
            Match off ml ->
                let outLen = length acc          -- length of accumulated output so far
                    start  = outLen - off        -- index in 0-based forward output
                    -- Build the copy byte-by-byte into a list, then prepend
                    -- to acc in reverse order.
                    copied = copyBytes start ml acc
                in go rest (reverse copied ++ acc)

    -- Copy @len@ bytes starting at position @start@ in the forward-order
    -- output (which is the reverse of @acc@).  We do this by indexing into
    -- the reversed-acc list which is the forward output.
    copyBytes _     0   _   = []
    copyBytes start len acc =
        let forwardOut = reverse acc
            b          = forwardOut !! start
        in b : copyBytes (start + 1) (len - 1) (b : acc)

-- ─── Wire format helpers ──────────────────────────────────────────────────────

-- | Serialise a big-endian 32-bit word into 4 bytes.
beWord32 :: Word32 -> [Word8]
beWord32 w =
    [ fromIntegral (w `shiftR` 24)
    , fromIntegral (w `shiftR` 16)
    , fromIntegral (w `shiftR`  8)
    , fromIntegral  w
    ]

-- | Serialise a big-endian 16-bit word into 2 bytes.
beWord16 :: Word16 -> [Word8]
beWord16 w =
    [ fromIntegral (w `shiftR` 8)
    , fromIntegral  w
    ]

-- | Parse a big-endian 32-bit word from 4 bytes starting at index @i@.
readBeWord32 :: ByteString -> Int -> Word32
readBeWord32 bs i =
    (fromIntegral (BS.index bs  i    ) `shiftL` 24)
    .|. (fromIntegral (BS.index bs (i+1)) `shiftL` 16)
    .|. (fromIntegral (BS.index bs (i+2)) `shiftL`  8)
    .|.  fromIntegral (BS.index bs (i+3))

-- | Parse a big-endian 16-bit word from 2 bytes starting at index @i@.
readBeWord16 :: ByteString -> Int -> Word16
readBeWord16 bs i =
    (fromIntegral (BS.index bs  i    ) `shiftL` 8)
    .|. fromIntegral (BS.index bs (i+1))

-- ─── Serialise tokens to CMP02 wire format ────────────────────────────────────

-- | Serialise a token list to the CMP02 binary wire format.
--
-- Layout:
--
-- @
-- [0..3]  original_length  — BE uint32
-- [4..7]  block_count      — BE uint32
-- [8..]   blocks           — each block: flag byte + symbol data
-- @
--
-- Each block holds up to 8 tokens. The flag byte describes the token kinds:
-- bit @i@ is 0 for a 'Literal' and 1 for a 'Match'. Bit 0 describes the first
-- token in the block, bit 7 the eighth.
serialiseTokens :: [Token] -> Int -> ByteString
serialiseTokens tokens origLen =
    BS.pack (header ++ concatMap serialiseBlock (chunksOf 8 tokens))
  where
    blockCount = (length tokens + 7) `div` 8

    header =
        beWord32 (fromIntegral origLen)
        ++ beWord32 (fromIntegral blockCount)

    serialiseBlock chunk =
        let (flag, symData) = foldl encodeSlot (0, []) (zip [0..] chunk)
        in flag : reverse symData   -- symData accumulated in reverse

    encodeSlot (flag, acc) (bitPos, tok) =
        case tok of
            Literal b ->
                (flag, b : acc)
            Match off ml ->
                let flag' = setBit flag bitPos
                    [hi, lo] = beWord16 (fromIntegral off :: Word16)
                    len8 = fromIntegral ml :: Word8
                    -- Prepend in reverse: len8 first (innermost = last in output)
                in (flag', len8 : lo : hi : acc)

-- ─── Deserialise tokens from CMP02 wire format ────────────────────────────────

-- | Deserialise a CMP02 binary stream into a token list and the original length.
--
-- Security: the @block_count@ field is capped to the number of bytes actually
-- available after the 8-byte header, preventing a crafted header from causing
-- an out-of-memory or infinite-loop situation.
deserialiseTokens :: ByteString -> ([Token], Int)
deserialiseTokens bs
    | BS.length bs < 8 = ([], 0)
    | otherwise =
        let origLen    = fromIntegral (readBeWord32 bs 0)
            rawBlocks  = fromIntegral (readBeWord32 bs 4)
            maxBlocks  = BS.length bs - 8   -- 1 byte minimum per block
            blockCount = min rawBlocks maxBlocks
            (tokens, _) = parseBlocks blockCount 8 []
        in (reverse tokens, origLen)
  where
    !bsLen = BS.length bs

    -- Parse @remaining@ blocks starting at byte offset @pos@, accumulating
    -- tokens in reverse order.
    parseBlocks 0    _   acc = (acc, 0)
    parseBlocks remaining pos acc
        | pos >= bsLen = (acc, pos)
        | otherwise    =
            let flag = BS.index bs pos
                (acc', pos') = parseSymbols flag 0 (pos + 1) acc
            in parseBlocks (remaining - 1) pos' acc'

    -- Parse up to 8 symbols for the current block, guided by @flag@.
    parseSymbols _    8   pos acc = (acc, pos)
    parseSymbols flag bit pos acc
        | pos >= bsLen = (acc, pos)
        | testBit flag bit =
            -- Match token: 3 bytes (u16 offset BE + u8 length)
            if pos + 3 > bsLen
            then (acc, pos)
            else
                let off = fromIntegral (readBeWord16 bs pos) :: Int
                    len = fromIntegral (BS.index bs (pos + 2)) :: Int
                    tok = Match off len
                in parseSymbols flag (bit + 1) (pos + 3) (tok : acc)
        | otherwise =
            -- Literal token: 1 byte
            let tok = Literal (BS.index bs pos)
            in parseSymbols flag (bit + 1) (pos + 1) (tok : acc)

-- ─── Utility ─────────────────────────────────────────────────────────────────

-- | Split a list into chunks of at most @n@ elements.
--
-- @chunksOf 3 [1..7] == [[1,2,3],[4,5,6],[7]]@
chunksOf :: Int -> [a] -> [[a]]
chunksOf _ [] = []
chunksOf n xs =
    let (h, t) = splitAt n xs
    in h : chunksOf n t

-- ─── Public one-shot API ──────────────────────────────────────────────────────

-- | Compress a 'ByteString' to the CMP02 wire format using default parameters.
--
-- * Window size: 4096 bytes ('defaultWindowSize')
-- * Maximum match: 255 bytes ('defaultMaxMatch')
-- * Minimum match: 3 bytes ('defaultMinMatch')
--
-- The first 8 bytes of the output are a fixed header containing the original
-- length (so 'decompress' can restore exactly the right number of bytes even if
-- the last block contains padding).
--
-- Example:
--
-- @
-- compress (Data.ByteString.Char8.pack "ABABAB")
-- @
compress :: ByteString -> ByteString
compress bs =
    let tokens = encode defaultWindowSize defaultMaxMatch defaultMinMatch bs
    in serialiseTokens tokens (BS.length bs)

-- | Decompress a 'ByteString' that was produced by 'compress'.
--
-- Reads the CMP02 header for the original length, reconstructs the token
-- list from the blocks, then runs 'decode' and truncates to the recorded
-- original length.
--
-- Invariant: @decompress (compress x) == x@ for all @x@.
decompress :: ByteString -> ByteString
decompress bs =
    let (tokens, origLen) = deserialiseTokens bs
        raw               = decode tokens
    in BS.take origLen raw
