module AtbashCipher
    ( encrypt
    , decrypt
    ) where

import Data.Char (chr, isAsciiLower, isAsciiUpper, ord)

-- | Encrypt text with the Atbash substitution. ASCII letters are mirrored
-- within their alphabet while case and all non-letter characters are kept.
encrypt :: String -> String
encrypt = map transform

-- | Decrypt Atbash text. Atbash is an involution, so decryption is encryption.
decrypt :: String -> String
decrypt = encrypt

transform :: Char -> Char
transform character
    | isAsciiUpper character = mirrorFrom 'A'
    | isAsciiLower character = mirrorFrom 'a'
    | otherwise = character
  where
    mirrorFrom alphabetStart =
        chr (ord alphabetStart + 25 - (ord character - ord alphabetStart))
