using System;
using System.Collections.Generic;
using FieldMath = CodingAdventures.Gf256.Gf256;

namespace CodingAdventures.ReedSolomon;

// ReedSolomon.cs -- Reed-Solomon block codes built from GF(256) arithmetic
// ==========================================================================
//
// Reed-Solomon treats a byte array as coefficients of a polynomial over GF(256).
// The encoder appends check bytes so the codeword vanishes at a run of field roots.
// The decoder walks the classic pipeline:
//
//   syndromes -> Berlekamp-Massey -> Chien search -> Forney -> corrections
//
// We keep the repo's big-endian codeword convention:
//
//   codeword[0] * x^(n-1) + ... + codeword[n-1]

public sealed class TooManyErrorsError : Exception
{
    public TooManyErrorsError() : base("reed-solomon: too many errors — codeword is unrecoverable")
    {
    }
}

public sealed class InvalidInputError : Exception
{
    public InvalidInputError(string message) : base($"reed-solomon: invalid input — {message}")
    {
    }
}

public static class ReedSolomon
{
    public const string VERSION = "0.1.0";

    public static byte[] BuildGenerator(int nCheck)
    {
        ValidatePositiveEven(nCheck);

        var generator = new byte[] { 1 };
        for (var powerIndex = 1; powerIndex <= nCheck; powerIndex++)
        {
            var alphaPower = FieldMath.Power(2, powerIndex);
            var next = new byte[generator.Length + 1];

            for (var index = 0; index < generator.Length; index++)
            {
                next[index] ^= FieldMath.Multiply(generator[index], alphaPower);
                next[index + 1] ^= generator[index];
            }

            generator = next;
        }

        return generator;
    }

    public static byte[] Encode(byte[] message, int nCheck)
    {
        if (message is null)
        {
            throw new ArgumentNullException(nameof(message));
        }

        ValidatePositiveEven(nCheck);

        var totalLength = checked(message.Length + nCheck);
        if (totalLength > 255)
        {
            throw new InvalidInputError($"total codeword length {totalLength} exceeds GF(256) block size limit of 255");
        }

        var generatorLittleEndian = BuildGenerator(nCheck);
        Array.Reverse(generatorLittleEndian);

        var shifted = new byte[totalLength];
        Array.Copy(message, shifted, message.Length);

        var remainder = PolyModBigEndian(shifted, generatorLittleEndian);
        var codeword = new byte[totalLength];
        Array.Copy(message, codeword, message.Length);

        var pad = nCheck - remainder.Length;
        Array.Copy(remainder, 0, codeword, message.Length + pad, remainder.Length);
        return codeword;
    }

    public static byte[] Syndromes(byte[] received, int nCheck)
    {
        if (received is null)
        {
            throw new ArgumentNullException(nameof(received));
        }

        var syndromes = new byte[nCheck];
        for (var index = 1; index <= nCheck; index++)
        {
            syndromes[index - 1] = PolyEvalBigEndian(received, FieldMath.Power(2, index));
        }

        return syndromes;
    }

    public static byte[] Decode(byte[] received, int nCheck)
    {
        if (received is null)
        {
            throw new ArgumentNullException(nameof(received));
        }

        ValidatePositiveEven(nCheck);

        if (received.Length < nCheck)
        {
            throw new InvalidInputError($"received length {received.Length} < nCheck {nCheck}");
        }

        var correctionCapacity = nCheck / 2;
        var messageLength = received.Length - nCheck;
        var syndromes = Syndromes(received, nCheck);

        if (AllZero(syndromes))
        {
            return received[..messageLength];
        }

        var (lambda, errorCount) = BerlekampMassey(syndromes);
        if (errorCount > correctionCapacity)
        {
            throw new TooManyErrorsError();
        }

        var positions = ChienSearch(lambda, received.Length);
        if (positions.Count != errorCount)
        {
            throw new TooManyErrorsError();
        }

        var magnitudes = Forney(lambda, syndromes, positions, received.Length);
        var corrected = (byte[])received.Clone();

        for (var index = 0; index < positions.Count; index++)
        {
            corrected[positions[index]] ^= magnitudes[index];
        }

        return corrected[..messageLength];
    }

    public static byte[] ErrorLocator(byte[] syndromes)
    {
        if (syndromes is null)
        {
            throw new ArgumentNullException(nameof(syndromes));
        }

        return BerlekampMassey(syndromes).Lambda;
    }

    private static void ValidatePositiveEven(int nCheck)
    {
        if (nCheck <= 0 || (nCheck & 1) != 0)
        {
            throw new InvalidInputError($"nCheck must be a positive even number, got {nCheck}");
        }
    }

    private static bool AllZero(byte[] values)
    {
        foreach (var value in values)
        {
            if (value != 0)
            {
                return false;
            }
        }

        return true;
    }

    private static byte PolyEvalBigEndian(byte[] coefficients, byte x)
    {
        byte accumulator = 0;
        foreach (var coefficient in coefficients)
        {
            accumulator = FieldMath.Add(FieldMath.Multiply(accumulator, x), coefficient);
        }

        return accumulator;
    }

    private static byte PolyEvalLittleEndian(byte[] coefficients, byte x)
    {
        byte accumulator = 0;
        for (var index = coefficients.Length - 1; index >= 0; index--)
        {
            accumulator = FieldMath.Add(FieldMath.Multiply(accumulator, x), coefficients[index]);
        }

        return accumulator;
    }

    private static byte[] PolyMulLittleEndian(byte[] left, byte[] right)
    {
        if (left.Length == 0 || right.Length == 0)
        {
            return [];
        }

        var result = new byte[left.Length + right.Length - 1];
        for (var leftIndex = 0; leftIndex < left.Length; leftIndex++)
        {
            for (var rightIndex = 0; rightIndex < right.Length; rightIndex++)
                {
                result[leftIndex + rightIndex] ^= FieldMath.Multiply(left[leftIndex], right[rightIndex]);
            }
        }

        return result;
    }

    private static byte[] PolyModBigEndian(byte[] dividend, byte[] divisor)
    {
        if (divisor.Length == 0)
        {
            throw new InvalidOperationException("poly_mod_be requires a non-empty divisor");
        }

        if (divisor[0] != 1)
        {
            throw new InvalidOperationException("poly_mod_be requires a monic divisor");
        }

        var remainder = (byte[])dividend.Clone();
        if (remainder.Length < divisor.Length)
        {
            return remainder;
        }

        var steps = remainder.Length - divisor.Length + 1;
        for (var step = 0; step < steps; step++)
        {
            var coefficient = remainder[step];
            if (coefficient == 0)
            {
                continue;
            }

            for (var index = 0; index < divisor.Length; index++)
            {
                remainder[step + index] ^= FieldMath.Multiply(coefficient, divisor[index]);
            }
        }

        return remainder[(remainder.Length - (divisor.Length - 1))..];
    }

    private static byte InvLocator(int position, int length)
    {
        var exponent = (position + 256 - length) % 255;
        return FieldMath.Power(2, exponent);
    }

    private static (byte[] Lambda, int ErrorCount) BerlekampMassey(byte[] syndromes)
    {
        var current = new byte[] { 1 };
        var previous = new byte[] { 1 };
        var errorCount = 0;
        var shift = 1;
        byte previousScale = 1;

        for (var sequenceIndex = 0; sequenceIndex < syndromes.Length; sequenceIndex++)
        {
            byte discrepancy = syndromes[sequenceIndex];

            for (var index = 1; index <= errorCount; index++)
            {
                if (index < current.Length && sequenceIndex >= index)
                {
                    discrepancy ^= FieldMath.Multiply(current[index], syndromes[sequenceIndex - index]);
                }
            }

            if (discrepancy == 0)
            {
                shift++;
                continue;
            }

            var scale = FieldMath.Divide(discrepancy, previousScale);
            var neededLength = shift + previous.Length;
            if (current.Length < neededLength)
            {
                Array.Resize(ref current, neededLength);
            }

            var saved = (byte[])current.Clone();
            for (var index = 0; index < previous.Length; index++)
            {
                current[shift + index] ^= FieldMath.Multiply(scale, previous[index]);
            }

            if (2 * errorCount <= sequenceIndex)
            {
                errorCount = sequenceIndex + 1 - errorCount;
                previous = saved;
                previousScale = discrepancy;
                shift = 1;
            }
            else
            {
                shift++;
            }
        }

        return (TrimTrailingZerosLittleEndian(current), errorCount);
    }

    private static List<int> ChienSearch(byte[] lambda, int length)
    {
        var positions = new List<int>();
        for (var position = 0; position < length; position++)
        {
            if (PolyEvalLittleEndian(lambda, InvLocator(position, length)) == 0)
            {
                positions.Add(position);
            }
        }

        return positions;
    }

    private static List<byte> Forney(byte[] lambda, byte[] syndromes, IReadOnlyList<int> positions, int length)
    {
        var omegaFull = PolyMulLittleEndian(syndromes, lambda);
        var omegaLength = Math.Min(syndromes.Length, omegaFull.Length);
        var omega = new byte[omegaLength];
        Array.Copy(omegaFull, omega, omegaLength);

        var lambdaPrime = new byte[Math.Max(0, lambda.Length - 1)];
        for (var index = 1; index < lambda.Length; index += 2)
        {
            lambdaPrime[index - 1] ^= lambda[index];
        }

        var magnitudes = new List<byte>(positions.Count);
        foreach (var position in positions)
        {
            var xiInverse = InvLocator(position, length);
            var numerator = PolyEvalLittleEndian(omega, xiInverse);
            var denominator = PolyEvalLittleEndian(lambdaPrime, xiInverse);

            if (denominator == 0)
            {
                throw new TooManyErrorsError();
            }

            magnitudes.Add(FieldMath.Divide(numerator, denominator));
        }

        return magnitudes;
    }

    private static byte[] TrimTrailingZerosLittleEndian(byte[] polynomial)
    {
        var last = polynomial.Length - 1;
        while (last > 0 && polynomial[last] == 0)
        {
            last--;
        }

        return polynomial[..(last + 1)];
    }
}
