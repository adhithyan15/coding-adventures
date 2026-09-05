-- | Arithmetic invariants shared by the public context and boundary tests.
--
-- This module is internal and may change without notice.
module Sha256.Internal
    ( maxSha256MessageBytes
    , checkedAdvanceBytes
    ) where

import Data.Word (Word64)

-- | Largest whole-byte message whose bit length is smaller than @2^64@.
maxSha256MessageBytes :: Word64
maxSha256MessageBytes = maxBound `div` 8

-- | Advance a byte count without leaving the FIPS 180-4 message domain.
checkedAdvanceBytes :: Word64 -> Word64 -> Maybe Word64
checkedAdvanceBytes current additional
    | current > maxSha256MessageBytes = Nothing
    | additional > maxSha256MessageBytes - current = Nothing
    | otherwise = Just (current + additional)
