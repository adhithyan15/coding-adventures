-- | Canonical rendering and kind-sensitive equality: total, bounded, never
-- @show@ on a user value (VER05n 7, hazard 5).
module CanonSpec (tests) where

import Control.Exception
import qualified Data.ByteString as B
import Data.Bits (shiftL)
import Data.Char (chr)
import Data.Text (Text)
import qualified Data.Text as T

import CodingAdventures.TestBootstrap.CorpusJson.Canon
import CodingAdventures.TestBootstrap.CorpusJson.Escape
import CodingAdventures.TestBootstrap.CorpusJson.Val
import Harness

r :: Val -> IO Text
r = render

-- | A user type whose Canonical instance misbehaves, to show render is total.
newtype Hostile = Hostile Int

instance Canonical Hostile where
  toVal (Hostile 0) = error "toVal boom"
  toVal (Hostile 1) = VList (VInt 1 : error "tail boom")
  toVal (Hostile 2) = VList (map VInt [1 ..]) -- infinite
  toVal (Hostile _) = VNil

tests :: [Test]
tests =
  [ test "scalars" $ do
      t1 <- r VNil
      t2 <- r (VBool True)
      t3 <- r (VBool False)
      t4 <- r (VInt (-12))
      assertEq "scalars" ["nil", "true", "false", "-12"] [t1, t2, t3, t4]
  , test "bytes and text" $ do
      b <- r (VBytes (B.pack [0, 255, 16]))
      e <- r (VBytes B.empty)
      s <- r (VText "a\"b\\c\nd\re\tf")
      assertEq "bytes" "0x00ff10" b
      assertEq "empty bytes" "0x" e
      assertEq "text" "\"a\\\"b\\\\c\\nd\\re\\tf\"" s
  , test "collections" $ do
      l <- r (VList [VInt 1, VBool True, VList []])
      t <- r (VTuple [VInt 1, VText "x"])
      assertEq "list" "[1,true,[]]" l
      assertEq "tuple" "(1,\"x\")" t
  , test "control, C1, separator, bidi and invisible characters are escaped" $ do
      let chars =
            [0x00, 0x01, 0x1f, 0x7f, 0x80, 0x85, 0x9f, 0x2028, 0x2029, 0x061c, 0x200e, 0x200f]
              ++ [0x202a .. 0x202e] ++ [0x2066 .. 0x2069]
              ++ [0xad, 0x34f, 0x180e, 0xfeff, 0x200b, 0x200c, 0x200d]
              ++ [0x2060 .. 0x2064] ++ [0xfff9 .. 0xfffb]
              ++ [0xe0000, 0xe0001, 0xe007f]
      mapM_
        ( \c -> do
            out <- r (VText (T.singleton (chr c)))
            assertTrue ("escaped U+" ++ show c) (T.all (\x -> x < '\x7f' && x >= ' ') out)
            assertTrue ("backslash-u form " ++ show c) ("\\u" `T.isInfixOf` out)
        )
        chars
      hi <- r (VText (T.singleton (chr 0xe0001)))
      assertEq "above the BMP" "\"\\u{e0001}\"" hi
      lo <- r (VText (T.singleton '\x1f'))
      assertEq "four digits" "\"\\u001f\"" lo
      bidi <- r (VText "a\x202e\&b")
      assertEq "bidi override" "\"a\\u202eb\"" bidi
  , test "ordinary non-ASCII text is kept" $ do
      out <- r (VText "caf\233 \26085")
      assertEq "kept" "\"caf\233 \26085\"" out
  , test "needsEscape boundaries" $ do
      assertFalse "U+0020" (needsEscape ' ')
      assertFalse "U+007E" (needsEscape '~')
      assertFalse "U+00A0" (needsEscape '\xa0')
      assertFalse "U+200A" (needsEscape '\x200a')
      assertFalse "U+2065" (needsEscape '\x2065')
      assertFalse "U+E0080" (needsEscape '\xe0080')
      assertFalse "U+FFFC" (needsEscape '\xfffc')
      assertTrue "U+E0000" (needsEscape '\xe0000')
  , test "depth cap renders an ellipsis" $ do
      let nest :: Int -> Val
          nest 0 = VInt 1
          nest n = VList [nest (n - 1)]
      shallow <- r (nest 5)
      assertEq "five deep" "[[[[[1]]]]]" shallow
      deep <- r (nest 20)
      assertTrue "ellipsis" ("..." `T.isInfixOf` deep)
      assertTrue "bounded" (T.length deep < 100)
  , test "a very deep value is rendered without walking it" $ do
      let v = iterate (\x -> VList [x]) VNil !! 200000
      out <- within 20 (r v)
      assertTrue "ellipsis" ("..." `T.isInfixOf` out)
  , test "a huge integer is not expanded" $ do
      big <- r (VInt (1 `shiftL` 100000))
      assertEq "100001 bits" "<bigint 100001 bits>" big
      neg <- r (VInt (negate (1 `shiftL` 200000)))
      assertEq "negative" "<bigint 200001 bits>" neg
      ok <- r (VInt ((1 `shiftL` 100000) - 1))
      assertTrue "99999+ bits still shown" (T.length ok > 30000)
  , test "node cap truncates" $ do
      out <- r (VList (replicate 20000 (VInt 1)))
      assertTrue "ellipsis" ("..." `T.isSuffixOf` out)
      assertTrue "at most the node cap" (T.count "1" out <= maxNodes)
  , test "output cap truncates" $ do
      out <- r (VText (T.replicate 3000000 "a"))
      assertTrue "capped" (T.length out <= maxOutput + 8)
      assertTrue "ellipsis" ("..." `T.isSuffixOf` out)
      bs <- r (VBytes (B.replicate (6 * 1048576) 7))
      assertTrue "bytes capped" (T.length bs <= maxOutput + 8)
      big <- r (VList (replicate 5000 (VText (T.replicate 1000 "z"))))
      assertTrue "list of texts capped" (T.length big <= maxOutput + 8)
  , test "an escaping text cannot expand past the cap" $ do
      out <- r (VText (T.replicate 3000000 "\x202e"))
      assertTrue "capped" (T.length out <= maxOutput + 8)
  , test "a throwing toVal makes the whole witness unrenderable" $ do
      t0 <- renderText (Hostile 0)
      t1 <- renderText (Hostile 1)
      assertEq "boom" "<unrenderable>" t0
      assertEq "tail boom" "<unrenderable>" t1
  , test "an infinite list terminates" $ do
      out <- within 20 (renderText (Hostile 2))
      assertTrue "truncated" ("..." `T.isSuffixOf` out)
  , test "render does not swallow asynchronous exceptions" $ do
      res <- try (render (VList [VInt 1, throw ThreadKilled]))
      case res of
        Left ThreadKilled -> pure ()
        Left e -> failTest ("wrong exception " ++ show e)
        Right t -> failTest ("swallowed: " ++ T.unpack t)
  , test "escapeBounded stops at its budget" $ do
      let (t, cut) = escapeBounded True 10 (T.replicate 1000000 "\n")
      assertTrue "cut" cut
      assertTrue "small" (T.length t <= 12)
      let (u, cut2) = escapeBounded True 10 "abc"
      assertEq "short text" ("abc", False) (u, cut2)
  , test "clipText and utf8Len" $ do
      assertEq "clip" "abc..." (clipText 3 "abcdef")
      assertEq "no clip" "abc" (clipText 3 "abc")
      assertEq "utf8 length" 10 (utf8Len "a\233\26085\128512")
  , test "sameVal is kind sensitive" $ do
      assertFalse "true vs 1" (sameVal (VBool True) (VInt 1))
      assertFalse "bytes vs text" (sameVal (VBytes "a") (VText "a"))
      assertFalse "list vs tuple" (sameVal (VList [VInt 1]) (VTuple [VInt 1]))
      assertFalse "length" (sameVal (VList [VInt 1]) (VList [VInt 1, VInt 2]))
      assertTrue "equal" (sameVal (VList [VInt 1, VText "x", VBytes "y"]) (VList [VInt 1, VText "x", VBytes "y"]))
      assertTrue "nil" (sameVal VNil VNil)
  , test "sameVal is bounded on infinite and deep values" $ do
      let inf = VList (map VInt [1 ..])
      assertFalse "two infinite lists" (sameVal inf inf)
      let nest :: Int -> Val
          nest 0 = VNil
          nest n = VList [nest (n - 1)]
      assertTrue "64 deep" (sameVal (nest 60) (nest 60))
      assertFalse "past the depth cap" (sameVal (nest 100) (nest 100))
      assertFalse "past the node cap" (sameVal (VList (replicate 20000 VNil)) (VList (replicate 20000 VNil)))
  ]
