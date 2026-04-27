module NoteFrequency
    ( Note(..)
    , spelling
    , chromaticIndex
    , semitonesFromA4
    , frequency
    , parseNote
    , noteToFrequency
    ) where

import Data.Char (toUpper)
import Text.Read (readMaybe)

data Note = Note
    { letter :: Char
    , accidental :: String
    , octave :: Int
    } deriving (Eq)

instance Show Note where
    show note = spelling note ++ show (octave note)

spelling :: Note -> String
spelling note = [letter note] ++ accidental note

chromaticIndexFor :: String -> Maybe Int
chromaticIndexFor name = case name of
    "C"  -> Just 0
    "C#" -> Just 1
    "Db" -> Just 1
    "D"  -> Just 2
    "D#" -> Just 3
    "Eb" -> Just 3
    "E"  -> Just 4
    "F"  -> Just 5
    "F#" -> Just 6
    "Gb" -> Just 6
    "G"  -> Just 7
    "G#" -> Just 8
    "Ab" -> Just 8
    "A"  -> Just 9
    "A#" -> Just 10
    "Bb" -> Just 10
    "B"  -> Just 11
    _     -> Nothing

createNote :: Char -> String -> Int -> Either String Note
createNote rawLetter accidentalValue octaveValue =
    let canonicalLetter = toUpper rawLetter
        note = Note canonicalLetter accidentalValue octaveValue
    in case chromaticIndexFor (spelling note) of
        Nothing -> Left ("Unsupported note spelling " ++ show (spelling note) ++ ". Only natural notes plus single # or b accidentals are supported.")
        Just _ -> Right note

chromaticIndex :: Note -> Int
chromaticIndex note = case chromaticIndexFor (spelling note) of
    Just indexValue -> indexValue
    Nothing -> error "chromaticIndex called on invalid note"

semitonesFromA4 :: Note -> Int
semitonesFromA4 note = octaveOffset + pitchOffset
  where
    octaveOffset = (octave note - 4) * 12
    pitchOffset = chromaticIndex note - 9

frequency :: Note -> Double
frequency note = 440.0 * 2 ** (fromIntegral (semitonesFromA4 note) / 12.0)

parseNote :: String -> Either String Note
parseNote input = case input of
    [] -> Left invalidShape
    (rawLetter:rest) ->
        let canonicalLetter = toUpper rawLetter
        in if canonicalLetter `notElem` "ABCDEFG"
           then Left invalidShape
           else do
            let (accidentalValue, octaveText) = case rest of
                    '#':xs -> ("#", xs)
                    'b':xs -> ("b", xs)
                    xs     -> ("", xs)
            if not (isCanonicalOctave octaveText)
               then Left invalidShape
               else do
                octaveValue <- maybe (Left invalidShape) Right (readMaybe octaveText)
                createNote canonicalLetter accidentalValue octaveValue
  where
    invalidShape = "Invalid note. Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."

isCanonicalOctave :: String -> Bool
isCanonicalOctave [] = False
isCanonicalOctave ('-':digits) = not (null digits) && all isDigitAscii digits
isCanonicalOctave digits = all isDigitAscii digits

isDigitAscii :: Char -> Bool
isDigitAscii character = character >= '0' && character <= '9'

noteToFrequency :: String -> Either String Double
noteToFrequency input = frequency <$> parseNote input
