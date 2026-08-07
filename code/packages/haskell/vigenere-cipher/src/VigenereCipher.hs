-- | The Vigenere cipher and classical statistical cryptanalysis.
module VigenereCipher
    ( BreakResult (..)
    , encrypt
    , decrypt
    , englishFrequencies
    , findKeyLength
    , findKeyLengthWithLimit
    , findKey
    , breakCipher
    ) where

import Data.Char (chr, isAsciiLower, isAsciiUpper, ord, toUpper)
import Data.List (foldl')

-- | Result of automatic Vigenere cryptanalysis.
data BreakResult = BreakResult
    { recoveredKey :: String
    , recoveredPlaintext :: String
    }
    deriving (Eq, Show)

-- | Expected English letter frequencies from A through Z.
englishFrequencies :: [Double]
englishFrequencies =
    [ 0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228
    , 0.02015, 0.06094, 0.06966, 0.00153, 0.00772, 0.04025
    , 0.02406, 0.06749, 0.07507, 0.01929, 0.00095, 0.05987
    , 0.06327, 0.09056, 0.02758, 0.00978, 0.02360, 0.00150
    , 0.01974, 0.00074
    ]

-- | Encrypt ASCII letters with a repeating alphabetic key.
encrypt :: String -> String -> Either String String
encrypt plaintext key = do
    shifts <- keyShifts key
    Right (applyCipher 1 shifts plaintext)

-- | Decrypt text encrypted with the given Vigenere key.
decrypt :: String -> String -> Either String String
decrypt ciphertext key = do
    shifts <- keyShifts key
    Right (applyCipher (-1) shifts ciphertext)

-- | Estimate the key length, considering candidates up to 20 letters.
findKeyLength :: String -> Int
findKeyLength ciphertext = findKeyLengthWithLimit ciphertext 20

-- | Estimate the key length using average index of coincidence.
findKeyLengthWithLimit :: String -> Int -> Int
findKeyLengthWithLimit ciphertext maximumLength
    | letterCount < 2 || limit < 2 = 1
    | bestIc <= 0.0 = 1
    | otherwise = fst (head candidates)
  where
    letters = extractAlphaUpper ciphertext
    letterCount = length letters
    limit = min maximumLength (letterCount `div` 2)
    scores = [(keyLength, averageIc letters keyLength) | keyLength <- [2 .. limit]]
    bestIc = maximum (map snd scores)
    threshold = bestIc * 0.9
    candidates = filter ((>= threshold) . snd) scores

-- | Recover an uppercase key of the requested length with chi-squared scoring.
findKey :: String -> Int -> String
findKey ciphertext keyLength
    | keyLength <= 0 = ""
    | otherwise = map recoverPosition [0 .. keyLength - 1]
  where
    letters = extractAlphaUpper ciphertext
    recoverPosition position =
        case positionGroup letters keyLength position of
            [] -> 'A'
            group -> chr (ord 'A' + bestShift group)

-- | Estimate the key, decrypt the ciphertext, and return both results.
breakCipher :: String -> BreakResult
breakCipher ciphertext =
    BreakResult key (applyCipher (-1) shifts ciphertext)
  where
    key = findKey ciphertext (findKeyLength ciphertext)
    shifts = map ((subtractCode 'A') . toUpper) key

keyShifts :: String -> Either String [Int]
keyShifts "" = Left "key must not be empty"
keyShifts key
    | all isAsciiLetter key = Right (map ((subtractCode 'A') . toUpper) key)
    | otherwise = Left "key must contain only ASCII letters"

applyCipher :: Int -> [Int] -> String -> String
applyCipher direction shifts = go 0
  where
    keyLength = length shifts
    go _ [] = []
    go keyIndex (char : remaining)
        | isAsciiUpper char || isAsciiLower char =
            shiftAscii direction (shifts !! (keyIndex `mod` keyLength)) char
                : go (keyIndex + 1) remaining
        | otherwise = char : go keyIndex remaining

shiftAscii :: Int -> Int -> Char -> Char
shiftAscii direction amount char
    | isAsciiUpper char = shiftFrom 'A'
    | otherwise = shiftFrom 'a'
  where
    shiftFrom base =
        chr
            ( ord base
                + (subtractCode base char + direction * amount) `mod` 26
            )

extractAlphaUpper :: String -> String
extractAlphaUpper = map toUpper . filter isAsciiLetter

isAsciiLetter :: Char -> Bool
isAsciiLetter char = isAsciiUpper char || isAsciiLower char

averageIc :: String -> Int -> Double
averageIc letters keyLength
    | null usableGroups = 0.0
    | otherwise = sum (map indexOfCoincidence usableGroups) / fromIntegral (length usableGroups)
  where
    usableGroups = filter ((> 1) . length) groups
    groups = [positionGroup letters keyLength position | position <- [0 .. keyLength - 1]]

positionGroup :: String -> Int -> Int -> String
positionGroup letters keyLength position =
    [ letter
    | (index, letter) <- zip [0 ..] letters
    , index `mod` keyLength == position
    ]

indexOfCoincidence :: String -> Double
indexOfCoincidence letters
    | letterCount < 2 = 0.0
    | otherwise = fromIntegral numerator / fromIntegral denominator
  where
    letterCount = length letters
    counts = letterCounts letters
    numerator = sum [count * (count - 1) | count <- counts]
    denominator = letterCount * (letterCount - 1)

bestShift :: String -> Int
bestShift group = fst (foldl' choose firstCandidate remainingCandidates)
  where
    candidates = [(shift, shiftScore group shift) | shift <- [0 .. 25]]
    firstCandidate = head candidates
    remainingCandidates = tail candidates
    choose best@(_, bestScore) candidate@(_, candidateScore)
        | candidateScore < bestScore = candidate
        | otherwise = best

shiftScore :: String -> Int -> Double
shiftScore group shift = chiSquared counts
  where
    decrypted =
        [ chr (ord 'A' + (subtractCode 'A' letter - shift) `mod` 26)
        | letter <- group
        ]
    counts = letterCounts decrypted

letterCounts :: String -> [Int]
letterCounts letters =
    [ length [() | letter <- letters, subtractCode 'A' letter == index]
    | index <- [0 .. 25]
    ]

chiSquared :: [Int] -> Double
chiSquared counts
    | total == 0 = 1 / 0
    | otherwise = sum (zipWith contribution counts englishFrequencies)
  where
    total = sum counts
    totalDouble = fromIntegral total
    contribution observed frequency = difference * difference / expected
      where
        expected = totalDouble * frequency
        difference = fromIntegral observed - expected

subtractCode :: Char -> Char -> Int
subtractCode base char = ord char - ord base
