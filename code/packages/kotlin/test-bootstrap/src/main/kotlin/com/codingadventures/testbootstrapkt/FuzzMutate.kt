package com.codingadventures.testbootstrapkt

import com.codingadventures.testbootstrap.fuzz.Mutate
import java.math.BigInteger

/*
 * The mutator algebra of VER06 section 3, as Kotlin extension functions on ByteArray
 * (VER06g section 2.3).  Every function delegates to the Java engine's `Mutate`:
 * the Kotlin layer only types the arguments (ULong, Endian) and never decides what
 * a mutation does.  No mutator modifies its receiver; each returns a new array.
 *
 * Each mutator has a Long overload and a BigInteger overload: the shared fuzz-v1
 * corpus carries integers of up to 1,000 digits, and the BigInteger forms reduce
 * them exactly (floor modulo for offsets, saturation for counts) before calling
 * the same core as the Long forms.
 */

/** The byte order of the multi-byte mutators. */
enum class Endian {
    /** Least significant byte first. */
    LITTLE,

    /** Most significant byte first. */
    BIG,
}

private fun Endian.toEngine(): Mutate.Endian {
    val e = if (Seams.swapEndian) (if (this == Endian.BIG) Endian.LITTLE else Endian.BIG) else this
    return Mutate.Endian.valueOf(e.name)
}

private fun BigInteger.seamed(): BigInteger = if (Seams.fuzzOffsetDelta == 0L) this else add(BigInteger.valueOf(Seams.fuzzOffsetDelta))

private fun Long.seamed(): Long = this + Seams.fuzzOffsetDelta

/** Ceilings the engine enforces (CWE-400). */
object MutatorLimits {
    /** Most mutation steps one `mutateSequence` call (or campaign) may run. */
    const val MAX_ITERATIONS: Int = Mutate.MAX_ITERATIONS

    /** Longest input `mutateSequence` accepts. */
    const val MAX_INPUT_BYTES: Int = Mutate.MAX_INPUT_BYTES
}

/**
 * Flips [count] consecutive bits starting at bit [bitOffset] (wrapping, floor modulo).
 *
 * ```
 *   00 00 . bitFlip(0, 1) -> 01 00
 * ```
 */
fun ByteArray.bitFlip(bitOffset: Long, count: Long): ByteArray = Mutate.bitFlip(this, bitOffset.seamed(), count)

/** [bitFlip] for arbitrary-size integers. */
fun ByteArray.bitFlip(bitOffset: BigInteger, count: BigInteger): ByteArray = Mutate.bitFlipBig(this, bitOffset.seamed(), count)

/** Inverts [count] consecutive bytes starting at [byteOffset]. */
fun ByteArray.byteFlip(byteOffset: Long, count: Long): ByteArray = Mutate.byteFlip(this, byteOffset.seamed(), count)

/** [byteFlip] for arbitrary-size integers. */
fun ByteArray.byteFlip(byteOffset: BigInteger, count: BigInteger): ByteArray = Mutate.byteFlipBig(this, byteOffset.seamed(), count)

/** Adds [delta] modulo 2^(8 * [width]) to the integer at [byteOffset]; [width] in 1, 2, 4, 8 (else unchanged). */
fun ByteArray.byteArithmetic(byteOffset: Long, delta: Long, width: Int, endian: Endian): ByteArray =
    Mutate.byteArithmetic(this, byteOffset.seamed(), delta, width, endian.toEngine())

/** [byteArithmetic] for arbitrary-size integers. */
fun ByteArray.byteArithmetic(byteOffset: BigInteger, delta: BigInteger, width: Int, endian: Endian): ByteArray =
    Mutate.byteArithmeticBig(this, byteOffset.seamed(), delta, width, endian.toEngine())

/** Overwrites [width] bytes at [byteOffset] with [value] modulo 2^(8 * [width]). */
fun ByteArray.boundaryInsert(byteOffset: Long, width: Int, value: ULong, endian: Endian): ByteArray =
    Mutate.boundaryInsert(this, byteOffset.seamed(), width, value.toLong(), endian.toEngine())

/** [boundaryInsert] for arbitrary-size integers (a negative [value] is its two's complement). */
fun ByteArray.boundaryInsert(byteOffset: BigInteger, width: Int, value: BigInteger, endian: Endian): ByteArray =
    Mutate.boundaryInsertBig(this, byteOffset.seamed(), width, value, endian.toEngine())

/** Deletes [length] bytes at [offset]. */
fun ByteArray.blockDelete(offset: Long, length: Long): ByteArray = Mutate.blockDelete(this, offset.seamed(), length)

/** [blockDelete] for arbitrary-size integers. */
fun ByteArray.blockDelete(offset: BigInteger, length: BigInteger): ByteArray = Mutate.blockDeleteBig(this, offset.seamed(), length)

/** Inserts [insert] at [offset]. */
fun ByteArray.blockInsert(offset: Long, insert: ByteArray): ByteArray = Mutate.blockInsert(this, offset.seamed(), insert)

/** [blockInsert] for arbitrary-size integers. */
fun ByteArray.blockInsert(offset: BigInteger, insert: ByteArray): ByteArray = Mutate.blockInsertBig(this, offset.seamed(), insert)

/** Copies [length] bytes from [srcOffset] and inserts the copy at [dstOffset]. */
fun ByteArray.blockDuplicate(srcOffset: Long, length: Long, dstOffset: Long): ByteArray =
    Mutate.blockDuplicate(this, srcOffset.seamed(), length, dstOffset)

/** [blockDuplicate] for arbitrary-size integers. */
fun ByteArray.blockDuplicate(srcOffset: BigInteger, length: BigInteger, dstOffset: BigInteger): ByteArray =
    Mutate.blockDuplicateBig(this, srcOffset.seamed(), length, dstOffset)

/** `this[:targetSplit] + donor[donorSplit:]`, both splits clamped into their buffers. */
fun ByteArray.splice(donor: ByteArray, targetSplit: Long, donorSplit: Long): ByteArray =
    Mutate.splice(this, donor, targetSplit.seamed(), donorSplit)

/** [splice] for arbitrary-size integers. */
fun ByteArray.splice(donor: ByteArray, targetSplit: BigInteger, donorSplit: BigInteger): ByteArray =
    Mutate.spliceBig(this, donor, targetSplit.seamed(), donorSplit)

/**
 * Applies [steps] seeded mutations (the cross-lane draw order of VER06 section 3.5).
 * [maxLen] bounds growth.  Over-cap arguments throw `IllegalArgumentException`.
 */
fun ByteArray.mutateSequence(seed: ULong, steps: Int, maxLen: Int = MutatorLimits.MAX_INPUT_BYTES): ByteArray =
    Mutate.mutateSequence(this, seed.toEngineSeed(), steps, maxLen)
