module HashFunctions
    ( description
    , hashBytes64
    , hashString64
    ) where

import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC
import Data.Bits (xor)
import Data.Word (Word64)

description :: String
description = "Haskell hashing helpers for DT data structures"

fnvOffset64 :: Word64
fnvOffset64 = 14695981039346656037

fnvPrime64 :: Word64
fnvPrime64 = 1099511628211

hashBytes64 :: BS.ByteString -> Word64
hashBytes64 =
    BS.foldl'
        (\acc byte -> (acc `xor` fromIntegral byte) * fnvPrime64)
        fnvOffset64

hashString64 :: String -> Word64
hashString64 = hashBytes64 . BC.pack
