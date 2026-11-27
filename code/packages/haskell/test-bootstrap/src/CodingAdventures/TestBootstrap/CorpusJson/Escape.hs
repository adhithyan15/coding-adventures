-- | Escaping of text that reaches a log line or a report.
--
-- A witness, a case id or an exception message is attacker-influenced data
-- that is about to be printed next to trusted text.  A raw newline forges a
-- log line; a bidirectional override (U+202E) reorders the text a reader sees;
-- a zero-width character hides a difference.  So every such character is
-- written as an escape.  The set is copied EXACTLY from the Elixir lane's
-- @Canon@ (VER05m 3); it is not invented here.
module CodingAdventures.TestBootstrap.CorpusJson.Escape
  ( needsEscape
  , escapeChar
  , escapeText
  , escapeBounded
  , clipText
  , utf8Len
  ) where

import Data.Char (ord)
import Data.Text (Text)
import qualified Data.Text as T
import Numeric (showHex)

-- | Characters that are never printed raw.
--
-- > U+0000..U+001F, U+007F, U+0080..U+009F      C0, DEL, C1 controls
-- > U+2028, U+2029                               line / paragraph separator
-- > U+061C, U+200E, U+200F, U+202A..U+202E,
-- > U+2066..U+2069                               bidirectional controls
-- > U+00AD, U+034F, U+180E, U+200B..U+200D,
-- > U+2060..U+2064, U+FEFF, U+FFF9..U+FFFB,
-- > U+E0000..U+E007F                             invisible formatting, tag block
needsEscape :: Char -> Bool
needsEscape ch =
  c < 0x20
    || c == 0x7F
    || (c >= 0x80 && c <= 0x9F)
    || c == 0x2028
    || c == 0x2029
    || c == 0x061C
    || c == 0x200E
    || c == 0x200F
    || (c >= 0x202A && c <= 0x202E)
    || (c >= 0x2066 && c <= 0x2069)
    || c == 0xAD
    || c == 0x34F
    || c == 0x180E
    || c == 0xFEFF
    || (c >= 0x200B && c <= 0x200D)
    || (c >= 0x2060 && c <= 0x2064)
    || (c >= 0xFFF9 && c <= 0xFFFB)
    || (c >= 0xE0000 && c <= 0xE007F)
  where
    c = ord ch

-- | One character's escaped form.  @quote@ additionally escapes @\\@ and @"@
-- (inside a quoted text); @\\n@, @\\r@, @\\t@ always have short forms; every
-- other escaped character is @\\uXXXX@ (four lowercase hex digits) or
-- @\\u{XXXXX}@ above U+FFFF.
escapeChar :: Bool -> Char -> String
escapeChar quote ch = case ch of
  '\n' -> "\\n"
  '\r' -> "\\r"
  '\t' -> "\\t"
  '\\' | quote -> "\\\\"
  '"' | quote -> "\\\""
  _
    | needsEscape ch ->
        let h = showHex (ord ch) ""
         in if ord ch > 0xFFFF
              then "\\u{" ++ h ++ "}"
              else "\\u" ++ replicate (4 - length h) '0' ++ h
    | otherwise -> [ch]

escapeText :: Bool -> Text -> Text
escapeText quote = T.concatMap (T.pack . escapeChar quote)

-- | Escapes at most @budget@ BYTES of output and says whether it stopped
-- early.  The input is clipped to @budget@ characters first, and the loop stops
-- as soon as the budget is spent, so a 10 MiB text costs O(budget), not O(10
-- MiB x 8): the cap is enforced BEFORE the allocation.
escapeBounded :: Bool -> Int -> Text -> (Text, Bool)
escapeBounded quote budget t = go 0 [] (T.unpack (T.take (budget + 1) t))
  where
    go :: Int -> [String] -> String -> (Text, Bool)
    go _ acc [] = (T.pack (concat (reverse acc)), False)
    go used acc (c : cs)
      | used >= budget = (T.pack (concat (reverse acc)), True)
      | otherwise =
          let e = escapeChar quote c
           in go (used + sum (map utf8CharLen e)) (e : acc) cs

utf8CharLen :: Char -> Int
utf8CharLen ch
  | c < 0x80 = 1
  | c < 0x800 = 2
  | c < 0x10000 = 3
  | otherwise = 4
  where
    c = ord ch

-- | UTF-8 length of a text, computed without encoding it.
utf8Len :: Text -> Int
utf8Len = T.foldl' (\n c -> n + utf8CharLen c) 0

-- | At most @limit@ characters, with an ellipsis when something was cut.
clipText :: Int -> Text -> Text
clipText limit t
  | T.length (T.take (limit + 1) t) > limit = T.take limit t <> "..."
  | otherwise = t
