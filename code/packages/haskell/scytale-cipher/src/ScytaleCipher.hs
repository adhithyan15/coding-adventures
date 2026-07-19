module ScytaleCipher
    ( BruteForceResult (..)
    , encrypt
    , decrypt
    , bruteForce
    ) where

import Data.List (dropWhileEnd)

-- | One candidate plaintext from a brute-force Scytale attack.
data BruteForceResult = BruteForceResult
    { bruteForceKey :: Int
    , bruteForceText :: String
    }
    deriving (Eq, Show)

-- | Encrypt text by writing it row-by-row into a grid with @key@ columns,
-- padding the final row with spaces, and reading the grid column-by-column.
encrypt :: String -> Int -> Either String String
encrypt "" _ = Right ""
encrypt text key = do
    validateKey (length text) key
    let rowCount = ceilingDiv (length text) key
        paddedLength = rowCount * key
        padded = text ++ replicate (paddedLength - length text) ' '
    Right
        [ padded !! (row * key + column)
        | column <- [0 .. key - 1]
        , row <- [0 .. rowCount - 1]
        ]

-- | Decrypt Scytale text with @key@ columns and remove trailing space padding.
-- Uneven ciphertext lengths are supported so brute-force candidates remain
-- well-defined for every key in the search range.
decrypt :: String -> Int -> Either String String
decrypt "" _ = Right ""
decrypt text key = do
    validateKey textLength key
    let rowCount = ceilingDiv textLength key
        remainder = textLength `mod` key
        columnLengths =
            [ if remainder == 0 || column < remainder
                then rowCount
                else rowCount - 1
            | column <- [0 .. key - 1]
            ]
        columnStarts = scanl (+) 0 columnLengths
        plaintext =
            [ text !! (columnStarts !! column + row)
            | row <- [0 .. rowCount - 1]
            , column <- [0 .. key - 1]
            , row < columnLengths !! column
            ]
    Right (dropWhileEnd (== ' ') plaintext)
  where
    textLength = length text

-- | Try every key from 2 through half the ciphertext length.
bruteForce :: String -> [BruteForceResult]
bruteForce text =
    [ BruteForceResult key plaintext
    | key <- [2 .. length text `div` 2]
    , Right plaintext <- [decrypt text key]
    ]

validateKey :: Int -> Int -> Either String ()
validateKey textLength key
    | key < 2 = Left "Key must be >= 2."
    | key > textLength = Left "Key must be <= text length."
    | otherwise = Right ()

ceilingDiv :: Int -> Int -> Int
ceilingDiv dividend divisor = (dividend + divisor - 1) `div` divisor
