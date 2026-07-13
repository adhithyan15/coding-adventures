-- | The Caesar cipher, brute-force enumeration, and frequency analysis.
module CaesarCipher
    ( encrypt
    , decrypt
    , rot13
    , BruteForceResult (..)
    , bruteForce
    , englishFrequencies
    , frequencyAnalysis
    ) where

import Data.Char (chr, isAsciiLower, isAsciiUpper, ord, toUpper)
import Data.List (foldl')

-- | Encrypt ASCII letters with a Caesar shift while preserving case.
-- Non-ASCII and non-letter characters pass through unchanged.
encrypt :: String -> Int -> String
encrypt text shift = map (shiftChar normalizedShift) text
  where
    normalizedShift = shift `mod` 26

-- | Reverse an encryption performed with the given shift.
decrypt :: String -> Int -> String
decrypt text shift = encrypt text (-shift)

-- | Apply the self-inverse Caesar shift of 13.
rot13 :: String -> String
rot13 text = encrypt text 13

shiftChar :: Int -> Char -> Char
shiftChar shift char
    | isAsciiUpper char = shiftFrom 'A'
    | isAsciiLower char = shiftFrom 'a'
    | otherwise = char
  where
    shiftFrom base = chr (ord base + (ord char - ord base + shift) `mod` 26)

-- | One candidate plaintext from a brute-force attack.
data BruteForceResult = BruteForceResult
    { bruteForceShift :: Int
    , bruteForcePlaintext :: String
    }
    deriving (Eq, Show)

-- | Try all 25 non-identity shifts in ascending order.
bruteForce :: String -> [BruteForceResult]
bruteForce ciphertext =
    [ BruteForceResult shift (decrypt ciphertext shift)
    | shift <- [1 .. 25]
    ]

-- | Expected English letter frequencies from A through Z.
englishFrequencies :: [Double]
englishFrequencies =
    [ 0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228
    , 0.02015, 0.06094, 0.06966, 0.00153, 0.00772, 0.04025
    , 0.02406, 0.06749, 0.07507, 0.01929, 0.00095, 0.05987
    , 0.06327, 0.09056, 0.02758, 0.00978, 0.02360, 0.00150
    , 0.01974, 0.00074
    ]

-- | Select the shift whose plaintext has the lowest chi-squared distance
-- from expected English letter frequencies. With no alphabetic signal the
-- first candidate, shift 1, is returned.
frequencyAnalysis :: String -> (Int, String)
frequencyAnalysis ciphertext =
    let firstPlaintext = decrypt ciphertext 1
        first = (1, firstPlaintext, chiSquared firstPlaintext)
        (bestShift, bestPlaintext, _) = foldl' chooseBetter first candidates
     in (bestShift, bestPlaintext)
  where
    candidates =
        [ (shift, plaintext, chiSquared plaintext)
        | shift <- [2 .. 25]
        , let plaintext = decrypt ciphertext shift
        ]
    chooseBetter best@(_, _, bestScore) candidate@(_, _, score)
        | score < bestScore = candidate
        | otherwise = best

letterCounts :: String -> [Int]
letterCounts text =
    [ length
        [ ()
        | char <- text
        , (isAsciiUpper char || isAsciiLower char)
        , toUpper char == chr (ord 'A' + index)
        ]
    | index <- [0 .. 25]
    ]

chiSquared :: String -> Double
chiSquared text
    | total == 0 = 1 / 0
    | otherwise = sum (zipWith contribution counts englishFrequencies)
  where
    counts = letterCounts text
    total = sum counts
    totalDouble = fromIntegral total
    contribution observed frequency
        | expected < 1e-10 = 0.0
        | otherwise = difference * difference / expected
      where
        expected = totalDouble * frequency
        difference = fromIntegral observed - expected
