package com.codingadventures.itf;

import com.codingadventures.barcodelayout1d.BarcodeLayout1DV1;
import com.codingadventures.paintinstructions.PaintScene;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests, 5-Class Security Showcase, bijective oracle, and 10,000-iteration PRNG mutation fuzz loop.
 */
class ItfTest {

    @Test
    void testVersionExists() {
        assertEquals("0.1.0", Itf.getVersion());
        assertEquals("0.1.0", ItfConstants.VERSION);
    }

    @Nested
    @DisplayName("5-Class Barcode/ITF Security Showcase")
    class SecurityShowcase {

        @Test
        @DisplayName("Class 1: Strict Precedence Enforcement")
        void testClass1PrecedenceEnforcement() {
            // Precedence 1: Null input throws IllegalArgumentException with payload-blind message
            IllegalArgumentException nullEx = assertThrows(IllegalArgumentException.class, () -> Itf.normalizeItf(null));
            assertEquals(ItfMessages.MESSAGE_NULL_INPUT, nullEx.getMessage());

            // Precedence 1: Lone high surrogate throws IllegalArgumentException
            IllegalArgumentException highSurrogateEx = assertThrows(
                    IllegalArgumentException.class,
                    () -> Itf.normalizeItf("\uD800")
            );
            assertEquals(ItfMessages.MESSAGE_INVALID_SCALARS, highSurrogateEx.getMessage());

            // Precedence 1: Lone low surrogate throws IllegalArgumentException
            IllegalArgumentException lowSurrogateEx = assertThrows(
                    IllegalArgumentException.class,
                    () -> Itf.normalizeItf("\uDC00")
            );
            assertEquals(ItfMessages.MESSAGE_INVALID_SCALARS, lowSurrogateEx.getMessage());

            // Precedence 1: Reversed surrogate pair throws IllegalArgumentException
            assertThrows(IllegalArgumentException.class, () -> Itf.normalizeItf("\uDC00\uD800"));

            // Precedence 1: Valid surrogate pair counts as exactly 1 Unicode scalar
            // 4095 '0's + 1 astral scalar (4097 UTF-16 code units = 4096 scalars, even length).
            // Must pass surrogate validation and scalar limit, then fail with invalid-character (NOT input-too-long).
            String astralEvenValidCapacity = "0".repeat(4095) + "\uD83D\uDE00";
            InvalidItfInputException astralCharEx = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf(astralEvenValidCapacity)
            );
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, astralCharEx.getErrorId());

            // 4096 '0's + 1 astral scalar (4098 UTF-16 code units = 4097 scalars).
            // Must fail with input-too-long.
            String astralOverLimit = "0".repeat(4096) + "\uD83D\uDE00";
            InvalidItfInputException astralOverEx = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf(astralOverLimit)
            );
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, astralOverEx.getErrorId());

            // Precedence 2: Over 4096 scalar characters must fail with input-too-long
            // even if length is odd and contains non-digits (e.g. 4097 'A's)
            String overLimitOddNonDigits = "A".repeat(4097);
            InvalidItfInputException overLimitEx = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf(overLimitOddNonDigits)
            );
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, overLimitEx.getErrorId());
            assertEquals(ItfErrorCode.INPUT_TOO_LONG, overLimitEx.getErrorCode());

            // Precedence 3: Length check (0 or odd) occurs before character scan.
            // Odd length with invalid character ('A') must fail with invalid-length, NOT invalid-character!
            InvalidItfInputException oddEx1 = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf("A")
            );
            assertEquals(ItfMessages.ERROR_INVALID_LENGTH, oddEx1.getErrorId());

            InvalidItfInputException oddEx2 = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf("123")
            );
            assertEquals(ItfMessages.ERROR_INVALID_LENGTH, oddEx2.getErrorId());

            // Empty string fails with invalid-length
            InvalidItfInputException emptyEx = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf("")
            );
            assertEquals(ItfMessages.ERROR_INVALID_LENGTH, emptyEx.getErrorId());

            // Precedence 4: Valid length with non-digit character fails with invalid-character
            InvalidItfInputException charEx = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf("12A4")
            );
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, charEx.getErrorId());
            assertEquals(ItfErrorCode.INVALID_CHARACTER, charEx.getErrorCode());
        }

        @Test
        @DisplayName("Class 2: Strict ASCII Digits Only (Reject Non-ASCII Digits)")
        void testClass2NonAsciiDigitRejection() {
            // Fullwidth digits (e.g. U+FF11 '１', U+FF12 '２') must be rejected as invalid-character
            InvalidItfInputException fullwidthEx = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf("\uFF11\uFF12")
            );
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, fullwidthEx.getErrorId());

            // Arabic-Indic digits (U+0661 '١', U+0662 '٢') must be rejected as invalid-character
            InvalidItfInputException arabicEx = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf("\u0661\u0662")
            );
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, arabicEx.getErrorId());

            // Punctuation and spaces
            assertThrows(InvalidItfInputException.class, () -> Itf.normalizeItf("1 23"));
            assertThrows(InvalidItfInputException.class, () -> Itf.normalizeItf("1\n23"));
            assertThrows(InvalidItfInputException.class, () -> Itf.normalizeItf("12-4"));
        }

        @Test
        @DisplayName("Class 3: High Capacity & Boundary Limit Verification")
        void testClass3CapacityLimits() {
            // Exactly 4096 ASCII digits must be accepted
            String limitInput = "0".repeat(4096);
            assertEquals(limitInput, Itf.normalizeItf(limitInput));

            // 4097 digits must be rejected with input-too-long
            String over4097 = "0".repeat(4097);
            InvalidItfInputException ex4097 = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf(over4097)
            );
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, ex4097.getErrorId());

            // 4098 digits (even length) must also be rejected with input-too-long
            String over4098 = "0".repeat(4098);
            InvalidItfInputException ex4098 = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf(over4098)
            );
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, ex4098.getErrorId());
        }

        @Test
        @DisplayName("Class 4: Structural & Layout Invariants")
        void testClass4StructuralInvariants() {
            String data = "1234";
            List<EncodedPair> pairs = Itf.encodeItf(data);
            assertEquals(2, pairs.size());
            assertEquals("12", pairs.get(0).pair());
            assertEquals("34", pairs.get(1).pair());

            List<BarcodeLayout1DV1.Run> runs = Itf.expandItfRuns(data);
            assertFalse(runs.isEmpty());

            // Start pattern runs
            assertEquals(ItfConstants.ROLE_START, runs.get(0).role());
            assertEquals(ItfConstants.LABEL_START, runs.get(0).sourceLabel());

            // Stop pattern runs
            assertEquals(ItfConstants.ROLE_STOP, runs.get(runs.size() - 1).role());
            assertEquals(ItfConstants.LABEL_STOP, runs.get(runs.size() - 1).sourceLabel());

            // Layout scene
            PaintScene scene = Itf.drawItf(data);
            assertNotNull(scene);
            assertEquals(ItfConstants.METADATA_VALUE_SYMBOLOGY, scene.metadata.get(ItfConstants.METADATA_KEY_SYMBOLOGY));
            assertEquals("2", scene.metadata.get(ItfConstants.METADATA_KEY_PAIR_COUNT));
            assertEquals("10", scene.metadata.get("leftQuietZoneModules"));
            assertEquals("10", scene.metadata.get("rightQuietZoneModules"));
            assertTrue(scene.width > 0);
            assertEquals(120, scene.height);
        }

        @Test
        @DisplayName("Class 5: Payload-Blind Security & Zero Leak")
        void testClass5PayloadBlindSecurity() {
            String secretSensitiveData = "9876543210";
            InvalidItfInputException ex = assertThrows(
                    InvalidItfInputException.class,
                    () -> Itf.normalizeItf(secretSensitiveData + "X")
            );
            assertFalse(ex.getMessage().contains(secretSensitiveData));
            assertFalse(ex.getMessage().contains("X"));

            // Default scene label must NOT interpolate caller payload
            PaintScene scene = Itf.drawItf("1234");
            assertEquals(ItfConstants.DEFAULT_SCENE_LABEL, scene.metadata.get("label"));
            assertFalse(scene.metadata.get("label").contains("1234"));
        }
    }

    @Test
    @DisplayName("Bijective Decode Oracle for all 100 Digit Pairs")
    void testBijectiveDecodeOracle() {
        // Table of digit patterns
        List<String> patterns = ItfConstants.DIGIT_PATTERNS;

        for (int d1 = 0; d1 <= 9; d1++) {
            for (int d2 = 0; d2 <= 9; d2++) {
                String pairStr = "" + d1 + d2;
                List<EncodedPair> pairs = Itf.encodeItf(pairStr);
                assertEquals(1, pairs.size());
                EncodedPair encoded = pairs.get(0);
                assertEquals(pairStr, encoded.pair());

                // Decode binaryPattern back to d1 and d2
                String bin = encoded.binaryPattern();
                // 5 pairs of (bar run, space run)
                StringBuilder recoveredBarPattern = new StringBuilder();
                StringBuilder recoveredSpacePattern = new StringBuilder();

                int cursor = 0;
                for (int elem = 0; elem < 5; elem++) {
                    // bar run: 1s
                    int barLen = 0;
                    while (cursor < bin.length() && bin.charAt(cursor) == '1') {
                        barLen++;
                        cursor++;
                    }
                    assertTrue(barLen == 1 || barLen == 3, "Invalid bar length: " + barLen);
                    recoveredBarPattern.append(barLen == 3 ? '1' : '0');

                    // space run: 0s
                    int spaceLen = 0;
                    while (cursor < bin.length() && bin.charAt(cursor) == '0') {
                        spaceLen++;
                        cursor++;
                    }
                    assertTrue(spaceLen == 1 || spaceLen == 3, "Invalid space length: " + spaceLen);
                    recoveredSpacePattern.append(spaceLen == 3 ? '1' : '0');
                }
                assertEquals(bin.length(), cursor);

                assertEquals(patterns.get(d1), recoveredBarPattern.toString());
                assertEquals(patterns.get(d2), recoveredSpacePattern.toString());

                int decodedD1 = patterns.indexOf(recoveredBarPattern.toString());
                int decodedD2 = patterns.indexOf(recoveredSpacePattern.toString());
                assertEquals(d1, decodedD1);
                assertEquals(d2, decodedD2);
            }
        }
    }

    @Test
    @DisplayName("10,000-Iteration PRNG Mutation Fuzz Loop")
    void test10000IterationFuzzLoop() {
        Random rng = new Random(0x17F_D00DL);

        for (int i = 0; i < 10_000; i++) {
            int testType = rng.nextInt(5);
            switch (testType) {
                case 0 -> {
                    // Valid random even-length numeric string (length 2 to 40)
                    int pairCount = 1 + rng.nextInt(20);
                    StringBuilder sb = new StringBuilder();
                    for (int p = 0; p < pairCount * 2; p++) {
                        sb.append((char) ('0' + rng.nextInt(10)));
                    }
                    String valid = sb.toString();
                    String normalized = Itf.normalizeItf(valid);
                    assertEquals(valid, normalized);
                    List<EncodedPair> encoded = Itf.encodeItf(valid);
                    assertEquals(pairCount, encoded.size());
                    List<BarcodeLayout1DV1.Run> runs = Itf.expandItfRuns(valid);
                    assertFalse(runs.isEmpty());
                    PaintScene scene = Itf.drawItf(valid);
                    assertNotNull(scene);
                }
                case 1 -> {
                    // Random odd-length numeric string (must fail with invalid-length)
                    int length = 1 + 2 * rng.nextInt(20);
                    StringBuilder sb = new StringBuilder();
                    for (int p = 0; p < length; p++) {
                        sb.append((char) ('0' + rng.nextInt(10)));
                    }
                    InvalidItfInputException ex = assertThrows(
                            InvalidItfInputException.class,
                            () -> Itf.normalizeItf(sb.toString())
                    );
                    assertEquals(ItfMessages.ERROR_INVALID_LENGTH, ex.getErrorId());
                }
                case 2 -> {
                    // Even-length string with non-digit character (must fail with invalid-character)
                    int pairCount = 1 + rng.nextInt(10);
                    char[] chars = new char[pairCount * 2];
                    for (int p = 0; p < chars.length; p++) {
                        chars[p] = (char) ('0' + rng.nextInt(10));
                    }
                    int corruptIdx = rng.nextInt(chars.length);
                    // corrupt with a non-ASCII digit or letter
                    chars[corruptIdx] = rng.nextBoolean() ? (char) ('A' + rng.nextInt(26)) : (char) (rng.nextInt(48));
                    if (chars[corruptIdx] >= '0' && chars[corruptIdx] <= '9') {
                        chars[corruptIdx] = 'X';
                    }
                    String corrupted = new String(chars);
                    InvalidItfInputException ex = assertThrows(
                            InvalidItfInputException.class,
                            () -> Itf.normalizeItf(corrupted)
                    );
                    assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, ex.getErrorId());
                }
                case 3 -> {
                    // Over-limit string (> 4096)
                    int extra = 1 + rng.nextInt(20);
                    String over = "0".repeat(4096 + extra);
                    InvalidItfInputException ex = assertThrows(
                            InvalidItfInputException.class,
                            () -> Itf.normalizeItf(over)
                    );
                    assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, ex.getErrorId());
                }
                case 4 -> {
                    // Lone surrogate corruption
                    String base = "12";
                    String surrogate = rng.nextBoolean() ? "\uD800" : "\uDC00";
                    String corrupted = base + surrogate;
                    assertThrows(IllegalArgumentException.class, () -> Itf.normalizeItf(corrupted));
                }
            }
        }
    }

    @Test
    @DisplayName("Custom SceneOptions and Metadata Handling")
    void testCustomSceneOptions() {
        Map<String, String> callerMeta = new HashMap<>();
        callerMeta.put("customKey", "customVal");

        BarcodeLayout1DV1.SceneOptions options = new BarcodeLayout1DV1.SceneOptions(
                6,
                160,
                "#112233",
                "#ffffff",
                "Custom Label",
                callerMeta,
                null,
                false,
                null
        );

        PaintScene scene = Itf.layoutItf("1234", options);
        assertEquals("Custom Label", scene.metadata.get("label"));
        assertEquals("customVal", scene.metadata.get("customKey"));
        assertEquals(ItfConstants.METADATA_VALUE_SYMBOLOGY, scene.metadata.get(ItfConstants.METADATA_KEY_SYMBOLOGY));
        assertEquals("2", scene.metadata.get(ItfConstants.METADATA_KEY_PAIR_COUNT));
        assertEquals(160, scene.height);
    }

    @Test
    @DisplayName("ItfMessages messageFor coverage")
    void testItfMessages() {
        assertEquals(ItfMessages.MESSAGE_INPUT_TOO_LONG, ItfMessages.messageFor(ItfErrorCode.INPUT_TOO_LONG));
        assertEquals(ItfMessages.MESSAGE_INVALID_LENGTH, ItfMessages.messageFor(ItfErrorCode.INVALID_LENGTH));
        assertEquals(ItfMessages.MESSAGE_INVALID_CHARACTER, ItfMessages.messageFor(ItfErrorCode.INVALID_CHARACTER));
        assertEquals(ItfMessages.MESSAGE_INVALID_CHARACTER, ItfMessages.messageFor(null));
        assertEquals("<redacted>", ItfMessages.REDACTED_PLACEHOLDER);
        assertEquals("<invalid>", ItfMessages.INVALID_PLACEHOLDER);
    }
}
