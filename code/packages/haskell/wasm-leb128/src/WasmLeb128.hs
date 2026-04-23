module WasmLeb128
    ( description
    , LEB128Error(..)
    , decodeUnsigned
    , decodeSigned
    , encodeUnsigned
    , encodeSigned
    ) where

import qualified Data.Bits as Bits
import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Data.Word (Word8)

description :: String
description = "Haskell LEB128 encoder and decoder for WASM binary integers"

data LEB128Error = LEB128Error
    { leb128ErrorMessage :: String
    , leb128ErrorOffset :: Int
    }
    deriving (Eq, Show)

maxBytes :: Int
maxBytes = 5

decodeUnsigned :: ByteString -> Int -> Either LEB128Error (Integer, Int)
decodeUnsigned bytes offset = go offset offset 0 0 0
  where
    go start current shift consumed result
        | consumed >= maxBytes =
            Left
                LEB128Error
                    { leb128ErrorMessage = "unterminated LEB128 value"
                    , leb128ErrorOffset = start
                    }
        | current >= BS.length bytes =
            Left
                LEB128Error
                    { leb128ErrorMessage = "unterminated LEB128 value"
                    , leb128ErrorOffset = start
                    }
        | otherwise =
            let byte = BS.index bytes current
                payload = toInteger (byte Bits..&. 0x7F)
                nextResult = result + Bits.shiftL payload shift
             in if byte Bits..&. 0x80 == 0
                    then Right (nextResult, consumed + 1)
                    else go start (current + 1) (shift + 7) (consumed + 1) nextResult

decodeSigned :: ByteString -> Int -> Either LEB128Error (Integer, Int)
decodeSigned bytes offset = go offset offset 0 0 0
  where
    go start current shift consumed result
        | consumed >= maxBytes =
            Left
                LEB128Error
                    { leb128ErrorMessage = "unterminated LEB128 value"
                    , leb128ErrorOffset = start
                    }
        | current >= BS.length bytes =
            Left
                LEB128Error
                    { leb128ErrorMessage = "unterminated LEB128 value"
                    , leb128ErrorOffset = start
                    }
        | otherwise =
            let byte = BS.index bytes current
                payload = toInteger (byte Bits..&. 0x7F)
                nextShift = shift + 7
                nextResult = result + Bits.shiftL payload shift
             in if byte Bits..&. 0x80 == 0
                    then
                        let signedResult =
                                if byte Bits..&. 0x40 /= 0
                                    then nextResult - Bits.shiftL 1 nextShift
                                    else nextResult
                         in Right (signedResult, consumed + 1)
                    else go start (current + 1) nextShift (consumed + 1) nextResult

encodeUnsigned :: Integer -> ByteString
encodeUnsigned value
    | value < 0 = error "encodeUnsigned requires a non-negative integer"
    | otherwise = BS.pack (go value)
  where
    go remaining =
        let payload = fromIntegral (remaining Bits..&. 0x7F) :: Word8
            next = Bits.shiftR remaining 7
         in if next == 0
                then [payload]
                else (payload Bits..|. 0x80) : go next

encodeSigned :: Integer -> ByteString
encodeSigned value = BS.pack (go value)
  where
    go remaining =
        let payload = fromIntegral (remaining Bits..&. 0x7F) :: Word8
            next = Bits.shiftR remaining 7
            done =
                (next == 0 && payload Bits..&. 0x40 == 0)
                    || (next == (-1) && payload Bits..&. 0x40 /= 0)
         in if done
                then [payload]
                else (payload Bits..|. 0x80) : go next
