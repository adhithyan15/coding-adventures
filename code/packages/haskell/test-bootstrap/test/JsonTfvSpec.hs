-- | The strict JSON reader and the TFV validator, including every rejection
-- class and every cap (VER05n 8).
module JsonTfvSpec (tests) where

import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import Data.Either (isRight)

import CodingAdventures.TestBootstrap.CorpusJson.StrictJson
import qualified CodingAdventures.TestBootstrap.Tfv as Tfv
import Fixture (readCorpus, readTfv)
import Harness

ok :: B.ByteString -> Either JsonError Json
ok = parse

rejected :: String -> B.ByteString -> IO ()
rejected label t = case parse t of
  Left (JsonError m) -> assertTrue (label ++ ": a reason is given") (length m > 0)
  Right _ -> failTest (label ++ " must be rejected")

accepted :: String -> B.ByteString -> IO ()
accepted label t = assertTrue (label ++ " must be accepted") (isRight (parse t))

tfvBad :: String -> B.ByteString -> IO ()
tfvBad label t = case Tfv.validate t of
  Left m -> assertTrue (label ++ ": a reason is given") (length m > 0)
  Right () -> failTest (label ++ " must be rejected")

tfvGood :: String -> B.ByteString -> IO ()
tfvGood label t = assertTrue (label ++ " must be accepted") (Tfv.valid t)

tests :: [Test]
tests =
  [ test "json values" $ do
      assertEq "object" (Right (JObject [("a", JArray [JNumber "1", JBool True, JNull])])) (ok "{\"a\": [1, true, null]}")
      assertEq "empty" (Right (JArray [])) (ok " [ ] ")
      assertEq "empty object" (Right (JObject [])) (ok "{}")
  , test "numbers stay text" $ do
      assertEq "30 digits" (Right (JNumber "123456789012345678901234567890")) (ok "123456789012345678901234567890")
      assertEq "exponent" (Right (JNumber "-1.5E+10")) (ok "-1.5E+10")
      assertEq "zero" (Right (JNumber "0")) (ok "0")
  , test "string escapes decode to UTF-8 bytes" $ do
      assertEq "simple" (Right (JString "a\"\\/\b\f\n\r\t")) (ok "\"a\\\"\\\\\\/\\b\\f\\n\\r\\t\"")
      assertEq "BMP" (Right (JString "\195\169")) (ok "\"\\u00e9\"")
      assertEq "pair" (Right (JString "\240\159\152\128")) (ok "\"\\ud83d\\ude00\"")
      assertEq "upper hex" (Right (JString "\195\169")) (ok "\"\\u00E9\"")
  , test "duplicate keys are refused, compared by decoded bytes" $ do
      rejected "plain" "{\"a\":1,\"a\":2}"
      rejected "escaped" "{\"a\":1,\"\\u0061\":2}"
      accepted "different keys" "{\"a\":1,\"b\":2}"
      rejected "nested dup" "[{\"k\":1,\"k\":1}]"
  , test "trailing data and empty input are refused" $ do
      rejected "trailing" "[] x"
      rejected "second doc" "{} {}"
      rejected "empty" ""
      rejected "whitespace only" "  "
  , test "malformed numbers" $
      mapM_ (\t -> rejected (BC.unpack t) t) ["01", "-01", "+1", ".5", "1.", "1e", "1e+", "-", "NaN", "Infinity", "0x10", "00"]
  , test "malformed literals and structure" $
      mapM_
        (\t -> rejected (BC.unpack t) t)
        ["tru", "nul", "True", "[1,]", "{\"a\":1,}", "[1 2]", "{\"a\" 1}", "{a:1}", "[", "{", "{\"a\":}", "// c\n1", "/* c */ 1", "[1,,2]"]
  , test "malformed strings" $ do
      rejected "unterminated" "\"abc"
      rejected "raw newline" "\"a\nb\""
      rejected "raw tab" "\"a\tb\""
      rejected "bad escape" "\"\\x\""
      rejected "lone high" "\"\\ud800\""
      rejected "lone low" "\"\\udc00\""
      rejected "reversed pair" "\"\\udc00\\ud800\""
      rejected "high then non-escape" "\"\\ud800a\""
      rejected "short hex" "\"\\u12\""
      rejected "non-hex" "\"\\u12g4\""
      rejected "trailing escape" "\"\\"
  , test "invalid UTF-8 and a BOM are refused" $ do
      rejected "BOM" "\239\187\191[]"
      rejected "lone continuation" "\"\128\""
      rejected "overlong C0 80" "\"\192\128\""
      rejected "overlong E0 80 80" "\"\224\128\128\""
      rejected "surrogate ED A0 80" "\"\237\160\128\""
      rejected "above 10FFFF" "\"\244\144\128\128\""
      rejected "truncated" "\"\226\130\""
      rejected "FF" "\"\255\""
      accepted "valid multibyte" "\"\226\130\172 \240\159\152\128\""
  , test "depth is capped at 64 containers" $ do
      let nest n = BC.pack (replicate n '[' ++ replicate n ']')
      accepted "64" (nest 64)
      rejected "65" (nest 65)
      let objs n = BC.pack (concat (replicate n "{\"a\":") ++ "1" ++ replicate n '}')
      accepted "64 objects" (objs 64)
      rejected "65 objects" (objs 65)
      rejected "a million deep" (BC.pack (replicate 1000000 '['))
  , test "size and node caps" $ do
      rejected "over 8 MiB" (B.replicate (8 * 1024 * 1024 + 1) 0x20)
      accepted "exactly 8 MiB of whitespace plus a value" (B.append (B.replicate (8 * 1024 * 1024 - 2) 0x20) "[]")
      let arr n = BC.pack ("[" ++ replicate (n - 1) '1' `joinWith` n ++ "]")
          joinWith _ n = concat (replicate (n - 1) "1,") ++ "1"
      accepted "199,999 elements" (arr 199999)
      rejected "200,000 elements" (arr 200000)
  , test "the committed corpus parses strictly and round trips" $ do
      raw <- readCorpus
      assertTrue "parses" (isRight (parse raw))
      rejected "with a trailing byte" (B.append raw "x")
  , test "tfv accepts minimal and valid documents" $ do
      tfvGood "header only" "tfv 1\n"
      tfvGood "header no newline" "tfv 1"
      tfvGood "comment and blank" "tfv 1\n# note\n\nr R1 Kop S706274\n"
      tfvGood "all value kinds" "tfv 1\nr R6 Ka N Kb T Kc F Kd I-12 Ke X00ff Kf L2 I1 I2\n"
      tfvGood "float ok" "tfv 1\nr R1 Kf D3ff0000000000000"
      tfvGood "map ok" "tfv 1\nr R1 Km M2 S61 I1 S62 I2"
  , test "the committed tfv is valid" $ do
      raw <- readTfv
      tfvGood "cases.tfv" raw
  , test "tfv rejections" $ do
      tfvBad "bad header" "bad header\n"
      tfvBad "empty" ""
      tfvBad "header with trailing space" "tfv 1 \n"
      tfvBad "uppercase hex" "tfv 1\nr R1 Kdata XFF\n"
      tfvBad "odd hex" "tfv 1\nr R1 Kdata Xf\n"
      tfvBad "I-0" "tfv 1\nr R1 Kn I-0\n"
      tfvBad "I007" "tfv 1\nr R1 Kn I007\n"
      tfvBad "empty int" "tfv 1\nr R1 Kn I\n"
      tfvBad "count disagrees (too few)" "tfv 1\nr R1 Kl L2 I1\n"
      tfvBad "trailing tokens" "tfv 1\nr R1 Kn I1 I2\n"
      tfvBad "doubled space" "tfv 1\nr  R1 Kn I1\n"
      tfvBad "trailing space" "tfv 1\nr R1 Kn I1 \n"
      tfvBad "duplicate field" "tfv 1\nr R2 Ka I1 Ka I2\n"
      tfvBad "duplicate record" "tfv 1\nr R1 Ka I1\nr R1 Ka I2\n"
      tfvBad "invalid UTF-8 in S" "tfv 1\nr R1 Ks Sff\n"
      tfvBad "invalid UTF-8 bytes" "tfv 1\nr R1 Ks S61\255\n"
      tfvBad "carriage return" "tfv 1\nr R1 Kn I1\r\n"
      tfvBad "bad field key" "tfv 1\nr R1 KBad I1\n"
      tfvBad "field without K" "tfv 1\nr R1 a I1\n"
      tfvBad "bad record id" "tfv 1\nr!x R1 Ka I1\n"
      tfvBad "missing R" "tfv 1\nr X1 Ka I1\n"
      tfvBad "leading zero count" "tfv 1\nr R01 Ka I1\n"
      tfvBad "huge count" "tfv 1\nr R9999999999 Ka I1\n"
      tfvBad "record ends early" "tfv 1\nr R2 Ka I1\n"
      tfvBad "unknown tag" "tfv 1\nr R1 Ka Z1\n"
      tfvBad "N with a body" "tfv 1\nr R1 Ka N1\n"
      tfvBad "short float" "tfv 1\nr R1 Kf D3ff\n"
      tfvBad "duplicate map key" "tfv 1\nr R1 Km M2 S61 I1 S61 I2\n"
  , test "tfv depth, integer, input and token caps" $ do
      let nest n = BC.pack ("tfv 1\nr R1 Kl " ++ concat (replicate n "L1 ") ++ "N\n")
      tfvGood "64 deep" (nest 64)
      tfvBad "65 deep" (nest 65)
      tfvGood "1000 digits" (BC.pack ("tfv 1\nr R1 Kn I1" ++ replicate 999 '0' ++ "\n"))
      tfvBad "1001 digits" (BC.pack ("tfv 1\nr R1 Kn I1" ++ replicate 1000 '0' ++ "\n"))
      tfvBad "over 1 MiB" (B.append "tfv 1\n# " (B.replicate (1048576 + 1) 0x61))
      tfvGood "199,990 tokens" (BC.pack ("tfv 1\nr R1 Kl L199985" ++ concat (replicate 199985 " I0") ++ "\n"))
      tfvBad "200,001 tokens" (BC.pack ("tfv 1\nr R1 Kl L200000" ++ concat (replicate 200000 " I0") ++ "\n"))
      -- a count is never trusted for allocation: this must be refused quickly
      tfvBad "huge list count" "tfv 1\nr R1 Kl L999999999 I0\n"
  ]
