package com.codingadventures.hyperloglog;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Objects;

public final class HyperLogLog {
    private static final int DEFAULT_PRECISION = 14;
    private static final int MIN_PRECISION = 4;
    private static final int MAX_PRECISION = 16;
    private static final long FNV_OFFSET_BASIS = 0xcbf29ce484222325L;
    private static final long FNV_PRIME = 0x100000001b3L;

    private final byte[] registers;
    private final int precisionBits;

    public HyperLogLog() {
        this(DEFAULT_PRECISION);
    }

    public HyperLogLog(int precision) {
        if (precision < MIN_PRECISION || precision > MAX_PRECISION) {
            throw new HyperLogLogError("precision must be between " + MIN_PRECISION + " and " + MAX_PRECISION + ", got " + precision);
        }
        this.precisionBits = precision;
        this.registers = new byte[1 << precision];
    }

    public static HyperLogLog withPrecision(int precision) {
        return new HyperLogLog(precision);
    }

    public static HyperLogLog tryWithPrecision(int precision) {
        try {
            return new HyperLogLog(precision);
        } catch (HyperLogLogError error) {
            return null;
        }
    }

    public HyperLogLog add(Object element) {
        addBytes(valueToBytes(element));
        return this;
    }

    public void addBytes(byte[] bytes) {
        long hash = fmix64(fnv1a64(bytes));
        int bucket = (int) Long.divideUnsigned(hash, 1L << (64 - precisionBits));
        int remainingBits = 64 - precisionBits;
        long mask = remainingBits == 64 ? -1L : (1L << remainingBits) - 1;
        long remaining = hash & mask;
        int rho = countLeadingZeros(remaining, remainingBits) + 1;
        if (rho > Byte.toUnsignedInt(registers[bucket])) {
            registers[bucket] = (byte) rho;
        }
    }

    public HyperLogLog copy() {
        HyperLogLog clone = new HyperLogLog(precisionBits);
        System.arraycopy(registers, 0, clone.registers, 0, registers.length);
        return clone;
    }

    public int count() {
        int registerCount = numRegisters();
        double zSum = 0.0;
        int zeroRegisters = 0;
        for (byte register : registers) {
            int unsigned = Byte.toUnsignedInt(register);
            zSum += Math.pow(2.0, -unsigned);
            if (unsigned == 0) {
                zeroRegisters++;
            }
        }
        double estimate = alphaForRegisters(registerCount) * registerCount * registerCount / zSum;
        if (estimate <= 2.5 * registerCount && zeroRegisters > 0) {
            estimate = registerCount * Math.log(registerCount / (double) zeroRegisters);
        }
        double two32 = 4294967296d;
        if (estimate > two32 / 30.0) {
            double ratio = 1.0 - (estimate / two32);
            if (ratio > 0.0) {
                estimate = -two32 * Math.log(ratio);
            }
        }
        return Math.max(0, (int) Math.round(estimate));
    }

    public HyperLogLog merge(HyperLogLog other) {
        HyperLogLog merged = tryMerge(other);
        if (merged == null) {
            throw new HyperLogLogError("precision mismatch: " + precisionBits + " vs " + other.precision());
        }
        return merged;
    }

    public HyperLogLog tryMerge(HyperLogLog other) {
        Objects.requireNonNull(other, "other");
        if (precisionBits != other.precisionBits) {
            return null;
        }
        HyperLogLog merged = new HyperLogLog(precisionBits);
        for (int index = 0; index < registers.length; index++) {
            merged.registers[index] = (byte) Math.max(
                Byte.toUnsignedInt(registers[index]),
                Byte.toUnsignedInt(other.registers[index])
            );
        }
        return merged;
    }

    public int len() {
        return count();
    }

    public int precision() {
        return precisionBits;
    }

    public int numRegisters() {
        return registers.length;
    }

    public double errorRate() {
        return errorRateForPrecision(precisionBits);
    }

    public static double errorRateForPrecision(int precision) {
        return 1.04 / Math.sqrt(1 << precision);
    }

    public static int memoryBytes(int precision) {
        return ((1 << precision) * 6) / 8;
    }

    public static int optimalPrecision(double desiredError) {
        double minRegisters = Math.pow(1.04 / desiredError, 2);
        int precision = (int) Math.ceil(Math.log(minRegisters) / Math.log(2));
        return Math.min(MAX_PRECISION, Math.max(MIN_PRECISION, precision));
    }

    @Override
    public boolean equals(Object other) {
        return other instanceof HyperLogLog that
            && precisionBits == that.precisionBits
            && Arrays.equals(registers, that.registers);
    }

    @Override
    public int hashCode() {
        return 31 * precisionBits + Arrays.hashCode(registers);
    }

    @Override
    public String toString() {
        return "HyperLogLog(precision=" + precisionBits
            + ", registers=" + numRegisters()
            + ", error_rate=" + BigDecimal.valueOf(errorRate() * 100.0).stripTrailingZeros().toPlainString()
            + "%)";
    }

    private static byte[] valueToBytes(Object value) {
        if (value instanceof byte[] bytes) {
            return Arrays.copyOf(bytes, bytes.length);
        }
        return String.valueOf(value).getBytes(StandardCharsets.UTF_8);
    }

    private static long fnv1a64(byte[] bytes) {
        long hash = FNV_OFFSET_BASIS;
        for (byte value : bytes) {
            hash ^= Byte.toUnsignedLong(value);
            hash *= FNV_PRIME;
        }
        return hash;
    }

    private static long fmix64(long value) {
        value ^= value >>> 33;
        value *= 0xff51afd7ed558ccdl;
        value ^= value >>> 33;
        value *= 0xc4ceb9fe1a85ec53l;
        value ^= value >>> 33;
        return value;
    }

    private static int countLeadingZeros(long value, int bitWidth) {
        if (bitWidth <= 0) {
            return 0;
        }
        if (value == 0L) {
            return bitWidth;
        }
        return Long.numberOfLeadingZeros(value) - (64 - bitWidth);
    }

    private static double alphaForRegisters(int registers) {
        return switch (registers) {
            case 16 -> 0.673;
            case 32 -> 0.697;
            case 64 -> 0.709;
            default -> 0.7213 / (1.0 + (1.079 / registers));
        };
    }
}
