package com.codingadventures.itf

import com.codingadventures.barcodelayout1d.BarcodeLayout1DV1
import com.codingadventures.paintinstructions.PaintScene
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Test
import java.util.Random
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * Unit tests, 5-Class Security Showcase, bijective oracle, and 10,000-iteration PRNG mutation fuzz loop in Kotlin.
 */
class ItfTest {

    @Test
    fun testVersionExists() {
        assertEquals("0.1.0", Itf.version)
        assertEquals("0.1.0", ItfConstants.VERSION)
    }

    @Nested
    @DisplayName("5-Class Barcode/ITF Security Showcase")
    inner class SecurityShowcase {

        @Test
        @DisplayName("Class 1: Strict Precedence Enforcement")
        fun testClass1PrecedenceEnforcement() {
            // Precedence 1: Null input throws IllegalArgumentException with payload-blind message
            val nullEx = assertFailsWith<IllegalArgumentException> { Itf.normalizeItf(null) }
            assertEquals(ItfMessages.MESSAGE_NULL_INPUT, nullEx.message)

            // Precedence 1: Lone high surrogate throws IllegalArgumentException
            val highSurrogateEx = assertFailsWith<IllegalArgumentException> {
                Itf.normalizeItf("\uD800")
            }
            assertEquals(ItfMessages.MESSAGE_INVALID_SCALARS, highSurrogateEx.message)

            // Precedence 1: Lone low surrogate throws IllegalArgumentException
            val lowSurrogateEx = assertFailsWith<IllegalArgumentException> {
                Itf.normalizeItf("\uDC00")
            }
            assertEquals(ItfMessages.MESSAGE_INVALID_SCALARS, lowSurrogateEx.message)

            // Precedence 1: Reversed surrogate pair throws IllegalArgumentException
            assertFailsWith<IllegalArgumentException> { Itf.normalizeItf("\uDC00\uD800") }

            // Precedence 1: Valid surrogate pair counts as exactly 1 Unicode scalar
            // 4095 '0's + 1 astral scalar (4097 UTF-16 code units = 4096 scalars, even length).
            // Must pass surrogate validation and scalar limit, then fail with invalid-character (NOT input-too-long).
            val astralEvenValidCapacity = "0".repeat(4095) + "\uD83D\uDE00"
            val astralCharEx = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf(astralEvenValidCapacity)
            }
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, astralCharEx.errorId)

            // 4096 '0's + 1 astral scalar (4098 UTF-16 code units = 4097 scalars).
            // Must fail with input-too-long.
            val astralOverLimit = "0".repeat(4096) + "\uD83D\uDE00"
            val astralOverEx = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf(astralOverLimit)
            }
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, astralOverEx.errorId)

            // Precedence 2: Over 4096 scalar characters must fail with input-too-long
            // even if length is odd and contains non-digits (e.g. 4097 'A's)
            val overLimitOddNonDigits = "A".repeat(4097)
            val overLimitEx = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf(overLimitOddNonDigits)
            }
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, overLimitEx.errorId)
            assertEquals(ItfErrorCode.INPUT_TOO_LONG, overLimitEx.errorCode)

            // Precedence 3: Length check (0 or odd) occurs before character scan.
            // Odd length with invalid character ('A') must fail with invalid-length, NOT invalid-character!
            val oddEx1 = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf("A")
            }
            assertEquals(ItfMessages.ERROR_INVALID_LENGTH, oddEx1.errorId)

            val oddEx2 = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf("123")
            }
            assertEquals(ItfMessages.ERROR_INVALID_LENGTH, oddEx2.errorId)

            // Empty string fails with invalid-length
            val emptyEx = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf("")
            }
            assertEquals(ItfMessages.ERROR_INVALID_LENGTH, emptyEx.errorId)

            // Precedence 4: Valid length with non-digit character fails with invalid-character
            val charEx = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf("12A4")
            }
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, charEx.errorId)
            assertEquals(ItfErrorCode.INVALID_CHARACTER, charEx.errorCode)
        }

        @Test
        @DisplayName("Class 2: Strict ASCII Digits Only (Reject Non-ASCII Digits)")
        fun testClass2NonAsciiDigitRejection() {
            // Fullwidth digits (e.g. U+FF11 '１', U+FF12 '２') must be rejected as invalid-character
            val fullwidthEx = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf("\uFF11\uFF12")
            }
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, fullwidthEx.errorId)

            // Arabic-Indic digits (U+0661 '١', U+0662 '٢') must be rejected as invalid-character
            val arabicEx = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf("\u0661\u0662")
            }
            assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, arabicEx.errorId)

            // Punctuation and spaces
            assertFailsWith<InvalidItfInputException> { Itf.normalizeItf("1 23") }
            assertFailsWith<InvalidItfInputException> { Itf.normalizeItf("1\n23") }
            assertFailsWith<InvalidItfInputException> { Itf.normalizeItf("12-4") }
        }

        @Test
        @DisplayName("Class 3: High Capacity & Boundary Limit Verification")
        fun testClass3CapacityLimits() {
            // Exactly 4096 ASCII digits must be accepted
            val limitInput = "0".repeat(4096)
            assertEquals(limitInput, Itf.normalizeItf(limitInput))

            // 4097 digits must be rejected with input-too-long
            val over4097 = "0".repeat(4097)
            val ex4097 = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf(over4097)
            }
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, ex4097.errorId)

            // 4098 digits (even length) must also be rejected with input-too-long
            val over4098 = "0".repeat(4098)
            val ex4098 = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf(over4098)
            }
            assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, ex4098.errorId)
        }

        @Test
        @DisplayName("Class 4: Structural & Layout Invariants")
        fun testClass4StructuralInvariants() {
            val data = "1234"
            val pairs = Itf.encodeItf(data)
            assertEquals(2, pairs.size)
            assertEquals("12", pairs[0].pair)
            assertEquals("34", pairs[1].pair)

            val runs = Itf.expandItfRuns(data)
            assertFalse(runs.isEmpty())

            // Start pattern runs
            assertEquals(ItfConstants.ROLE_START, runs[0].role)
            assertEquals(ItfConstants.LABEL_START, runs[0].sourceLabel)

            // Stop pattern runs
            assertEquals(ItfConstants.ROLE_STOP, runs[runs.size - 1].role)
            assertEquals(ItfConstants.LABEL_STOP, runs[runs.size - 1].sourceLabel)

            // Layout scene
            val scene = Itf.drawItf(data)
            assertNotNull(scene)
            assertEquals(ItfConstants.METADATA_VALUE_SYMBOLOGY, scene.metadata[ItfConstants.METADATA_KEY_SYMBOLOGY])
            assertEquals("2", scene.metadata[ItfConstants.METADATA_KEY_PAIR_COUNT])
            assertEquals("10", scene.metadata["leftQuietZoneModules"])
            assertEquals("10", scene.metadata["rightQuietZoneModules"])
            assertTrue(scene.width > 0)
            assertEquals(120, scene.height)
        }

        @Test
        @DisplayName("Class 5: Payload-Blind Security & Zero Leak")
        fun testClass5PayloadBlindSecurity() {
            val secretSensitiveData = "9876543210"
            val ex = assertFailsWith<InvalidItfInputException> {
                Itf.normalizeItf(secretSensitiveData + "X")
            }
            val msg = ex.message ?: ""
            assertFalse(msg.contains(secretSensitiveData))
            assertFalse(msg.contains("X"))

            // Default scene label must NOT interpolate caller payload
            val scene = Itf.drawItf("1234")
            assertEquals(ItfConstants.DEFAULT_SCENE_LABEL, scene.metadata["label"])
            val label = scene.metadata["label"] ?: ""
            assertFalse(label.contains("1234"))
        }
    }

    @Test
    @DisplayName("Bijective Decode Oracle for all 100 Digit Pairs")
    fun testBijectiveDecodeOracle() {
        val patterns = ItfConstants.DIGIT_PATTERNS

        for (d1 in 0..9) {
            for (d2 in 0..9) {
                val pairStr = "$d1$d2"
                val pairs = Itf.encodeItf(pairStr)
                assertEquals(1, pairs.size)
                val encoded = pairs[0]
                assertEquals(pairStr, encoded.pair)

                val bin = encoded.binaryPattern
                val recoveredBarPattern = java.lang.StringBuilder()
                val recoveredSpacePattern = java.lang.StringBuilder()

                var cursor = 0
                for (elem in 0 until 5) {
                    var barLen = 0
                    while (cursor < bin.length && bin[cursor] == '1') {
                        barLen++
                        cursor++
                    }
                    assertTrue(barLen == 1 || barLen == 3, "Invalid bar length: $barLen")
                    recoveredBarPattern.append(if (barLen == 3) '1' else '0')

                    var spaceLen = 0
                    while (cursor < bin.length && bin[cursor] == '0') {
                        spaceLen++
                        cursor++
                    }
                    assertTrue(spaceLen == 1 || spaceLen == 3, "Invalid space length: $spaceLen")
                    recoveredSpacePattern.append(if (spaceLen == 3) '1' else '0')
                }
                assertEquals(bin.length, cursor)

                assertEquals(patterns[d1], recoveredBarPattern.toString())
                assertEquals(patterns[d2], recoveredSpacePattern.toString())

                val decodedD1 = patterns.indexOf(recoveredBarPattern.toString())
                val decodedD2 = patterns.indexOf(recoveredSpacePattern.toString())
                assertEquals(d1, decodedD1)
                assertEquals(d2, decodedD2)
            }
        }
    }

    @Test
    @DisplayName("10,000-Iteration PRNG Mutation Fuzz Loop")
    fun test10000IterationFuzzLoop() {
        val rng = Random(0x17F_D00DL)

        for (i in 0 until 10_000) {
            when (rng.nextInt(5)) {
                0 -> {
                    val pairCount = 1 + rng.nextInt(20)
                    val sb = StringBuilder()
                    for (p in 0 until pairCount * 2) {
                        sb.append(('0'.code + rng.nextInt(10)).toChar())
                    }
                    val valid = sb.toString()
                    val normalized = Itf.normalizeItf(valid)
                    assertEquals(valid, normalized)
                    val encoded = Itf.encodeItf(valid)
                    assertEquals(pairCount, encoded.size)
                    val runs = Itf.expandItfRuns(valid)
                    assertFalse(runs.isEmpty())
                    val scene = Itf.drawItf(valid)
                    assertNotNull(scene)
                }
                1 -> {
                    val length = 1 + 2 * rng.nextInt(20)
                    val sb = StringBuilder()
                    for (p in 0 until length) {
                        sb.append(('0'.code + rng.nextInt(10)).toChar())
                    }
                    val ex = assertFailsWith<InvalidItfInputException> {
                        Itf.normalizeItf(sb.toString())
                    }
                    assertEquals(ItfMessages.ERROR_INVALID_LENGTH, ex.errorId)
                }
                2 -> {
                    val pairCount = 1 + rng.nextInt(10)
                    val chars = CharArray(pairCount * 2)
                    for (p in chars.indices) {
                        chars[p] = ('0'.code + rng.nextInt(10)).toChar()
                    }
                    val corruptIdx = rng.nextInt(chars.size)
                    chars[corruptIdx] = if (rng.nextBoolean()) {
                        ('A'.code + rng.nextInt(26)).toChar()
                    } else {
                        rng.nextInt(48).toChar()
                    }
                    if (chars[corruptIdx] in '0'..'9') {
                        chars[corruptIdx] = 'X'
                    }
                    val corrupted = String(chars)
                    val ex = assertFailsWith<InvalidItfInputException> {
                        Itf.normalizeItf(corrupted)
                    }
                    assertEquals(ItfMessages.ERROR_INVALID_CHARACTER, ex.errorId)
                }
                3 -> {
                    val extra = 1 + rng.nextInt(20)
                    val over = "0".repeat(4096 + extra)
                    val ex = assertFailsWith<InvalidItfInputException> {
                        Itf.normalizeItf(over)
                    }
                    assertEquals(ItfMessages.ERROR_INPUT_TOO_LONG, ex.errorId)
                }
                4 -> {
                    val base = "12"
                    val surrogate = if (rng.nextBoolean()) "\uD800" else "\uDC00"
                    val corrupted = base + surrogate
                    assertFailsWith<IllegalArgumentException> { Itf.normalizeItf(corrupted) }
                }
            }
        }
    }

    @Test
    @DisplayName("Custom SceneOptions and Metadata Handling")
    fun testCustomSceneOptions() {
        val callerMeta = HashMap<String, String>()
        callerMeta["customKey"] = "customVal"

        val options = BarcodeLayout1DV1.SceneOptions(
            6,
            160,
            "#112233",
            "#ffffff",
            "Custom Label",
            callerMeta,
            null,
            false,
            null
        )

        val scene = Itf.layoutItf("1234", options)
        assertEquals("Custom Label", scene.metadata["label"])
        assertEquals("customVal", scene.metadata["customKey"])
        assertEquals(ItfConstants.METADATA_VALUE_SYMBOLOGY, scene.metadata[ItfConstants.METADATA_KEY_SYMBOLOGY])
        assertEquals("2", scene.metadata[ItfConstants.METADATA_KEY_PAIR_COUNT])
        assertEquals(160, scene.height)
    }

    @Test
    @DisplayName("ItfMessages messageFor coverage")
    fun testItfMessages() {
        assertEquals(ItfMessages.MESSAGE_INPUT_TOO_LONG, ItfMessages.messageFor(ItfErrorCode.INPUT_TOO_LONG))
        assertEquals(ItfMessages.MESSAGE_INVALID_LENGTH, ItfMessages.messageFor(ItfErrorCode.INVALID_LENGTH))
        assertEquals(ItfMessages.MESSAGE_INVALID_CHARACTER, ItfMessages.messageFor(ItfErrorCode.INVALID_CHARACTER))
        assertEquals(ItfMessages.MESSAGE_INVALID_CHARACTER, ItfMessages.messageFor(null))
        assertEquals("<redacted>", ItfMessages.REDACTED_PLACEHOLDER)
        assertEquals("<invalid>", ItfMessages.INVALID_PLACEHOLDER)
    }
}
