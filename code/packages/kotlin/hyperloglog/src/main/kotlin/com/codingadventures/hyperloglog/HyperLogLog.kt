package com.codingadventures.hyperloglog

import kotlin.math.ceil
import kotlin.math.ln
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sqrt

class HyperLogLog(precision: Int = DEFAULT_PRECISION) {
    private val registers: ByteArray
    private val precisionBits: Int

    init {
        if (precision !in MIN_PRECISION..MAX_PRECISION) {
            throw HyperLogLogError("precision must be between $MIN_PRECISION and $MAX_PRECISION, got $precision")
        }
        precisionBits = precision
        registers = ByteArray(1 shl precision)
    }

    fun add(element: Any?): HyperLogLog {
        addBytes(valueToBytes(element))
        return this
    }

    fun addBytes(bytes: ByteArray) {
        val hash = fmix64(fnv1a64(bytes))
        val bucket = (hash shr (64 - precisionBits)).toInt()
        val remainingBits = 64 - precisionBits
        val mask = if (remainingBits == 64) ULong.MAX_VALUE else (1UL shl remainingBits) - 1UL
        val remaining = hash and mask
        val rho = countLeadingZeros(remaining, remainingBits) + 1
        if (rho > registers[bucket].toUByte().toInt()) {
            registers[bucket] = rho.toByte()
        }
    }

    fun copy(): HyperLogLog = HyperLogLog(precisionBits).also { clone ->
        registers.copyInto(clone.registers)
    }

    fun count(): Int {
        val registerCount = numRegisters()
        var zSum = 0.0
        var zeroRegisters = 0
        for (register in registers) {
            val unsigned = register.toUByte().toInt()
            zSum += 2.0.pow(-unsigned)
            if (unsigned == 0) zeroRegisters += 1
        }
        var estimate = alphaForRegisters(registerCount) * registerCount * registerCount / zSum
        if (estimate <= 2.5 * registerCount && zeroRegisters > 0) {
            estimate = registerCount * ln(registerCount / zeroRegisters.toDouble())
        }
        val two32 = 4294967296.0
        if (estimate > two32 / 30.0) {
            val ratio = 1.0 - (estimate / two32)
            if (ratio > 0.0) estimate = -two32 * ln(ratio)
        }
        return estimate.roundToInt().coerceAtLeast(0)
    }

    fun merge(other: HyperLogLog): HyperLogLog =
        tryMerge(other) ?: throw HyperLogLogError("precision mismatch: $precisionBits vs ${other.precision()}")

    fun tryMerge(other: HyperLogLog): HyperLogLog? {
        if (precisionBits != other.precisionBits) return null
        return HyperLogLog(precisionBits).also { merged ->
            for (index in registers.indices) {
                merged.registers[index] = maxOf(registers[index].toUByte().toInt(), other.registers[index].toUByte().toInt()).toByte()
            }
        }
    }

    fun len(): Int = count()
    fun precision(): Int = precisionBits
    fun numRegisters(): Int = registers.size
    fun errorRate(): Double = errorRateForPrecision(precisionBits)

    override fun equals(other: Any?): Boolean =
        other is HyperLogLog && precisionBits == other.precisionBits && registers.contentEquals(other.registers)

    override fun hashCode(): Int = 31 * precisionBits + registers.contentHashCode()

    override fun toString(): String =
        "HyperLogLog(precision=$precisionBits, registers=${numRegisters()}, error_rate=${(errorRate() * 100).toBigDecimal().stripTrailingZeros().toPlainString()}%)"

    companion object {
        private const val DEFAULT_PRECISION = 14
        private const val MIN_PRECISION = 4
        private const val MAX_PRECISION = 16
        private const val FNV_OFFSET_BASIS = 0xcbf29ce484222325UL
        private const val FNV_PRIME = 0x100000001b3UL

        fun withPrecision(precision: Int): HyperLogLog = HyperLogLog(precision)

        fun tryWithPrecision(precision: Int): HyperLogLog? =
            runCatching { HyperLogLog(precision) }.getOrNull()

        fun errorRateForPrecision(precision: Int): Double = 1.04 / sqrt((1 shl precision).toDouble())

        fun memoryBytes(precision: Int): Int = ((1 shl precision) * 6) / 8

        fun optimalPrecision(desiredError: Double): Int {
            val minRegisters = (1.04 / desiredError).pow(2)
            val precision = ceil(ln(minRegisters) / ln(2.0)).toInt()
            return precision.coerceIn(MIN_PRECISION, MAX_PRECISION)
        }

        private fun valueToBytes(value: Any?): ByteArray = when (value) {
            is ByteArray -> value.copyOf()
            else -> value.toString().encodeToByteArray()
        }

        private fun fnv1a64(bytes: ByteArray): ULong {
            var hash = FNV_OFFSET_BASIS
            for (value in bytes) {
                hash = (hash xor value.toUByte().toULong()) * FNV_PRIME
            }
            return hash
        }

        private fun fmix64(value: ULong): ULong {
            var mixed = value
            mixed = mixed xor (mixed shr 33)
            mixed *= 0xff51afd7ed558ccdUL
            mixed = mixed xor (mixed shr 33)
            mixed *= 0xc4ceb9fe1a85ec53UL
            mixed = mixed xor (mixed shr 33)
            return mixed
        }

        private fun countLeadingZeros(value: ULong, bitWidth: Int): Int {
            if (bitWidth <= 0) return 0
            if (value == 0UL) return bitWidth
            return value.countLeadingZeroBits() - (64 - bitWidth)
        }

        private fun alphaForRegisters(registers: Int): Double = when (registers) {
            16 -> 0.673
            32 -> 0.697
            64 -> 0.709
            else -> 0.7213 / (1.0 + (1.079 / registers))
        }
    }
}

class HyperLogLogError(message: String) : RuntimeException(message)
