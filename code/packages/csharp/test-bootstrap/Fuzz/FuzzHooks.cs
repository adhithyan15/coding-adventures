using System.Numerics;
using CodingAdventures.TestBootstrap.Tfv;

namespace CodingAdventures.TestBootstrap.Fuzz;

/// <summary>
/// Sabotage hooks: the engine entry points the corpus runner calls, as swappable fields. These are the
/// only production indirections added for the negative controls of VER06h section 10: a test replaces
/// one with a broken version, runs the corpus, and the runner must report problems. Production code
/// never reassigns them, and the class is internal (visible to the test assembly only).
/// </summary>
internal static class FuzzHooks
{
    internal delegate byte[] Flip(byte[] data, BigInteger offset, BigInteger count);

    internal delegate byte[] Arithmetic(byte[] data, BigInteger offset, BigInteger delta, int width, Endian e);

    internal delegate byte[] Boundary(byte[] data, BigInteger offset, int width, BigInteger val, Endian e);

    internal delegate byte[] Insert(byte[] data, BigInteger offset, byte[] insert);

    internal delegate byte[] Duplicate(byte[] data, BigInteger src, BigInteger length, BigInteger dst);

    internal delegate byte[] Splice(byte[] target, byte[] donor, BigInteger targetSplit, BigInteger donorSplit);

    internal delegate Mutate.CountedResult Sequence(byte[] data, ulong seed, int steps, int maxLen, long workCap);

    internal delegate Bitmap.Novelty Admit(ulong[] existing, ulong[] trace);

    internal delegate bool Dedup(string kindA, IReadOnlyList<string> framesA, string kindB, IReadOnlyList<string> framesB);

    internal delegate string Tap(int n, string target, string failureId, string sig);

    internal static volatile Flip BitFlip = Mutate.BitFlipBig;
    internal static volatile Flip ByteFlip = Mutate.ByteFlipBig;
    internal static volatile Arithmetic ByteArithmetic = Mutate.ByteArithmeticBig;
    internal static volatile Boundary BoundaryInsert = Mutate.BoundaryInsertBig;
    internal static volatile Flip BlockDelete = Mutate.BlockDeleteBig;
    internal static volatile Insert BlockInsert = Mutate.BlockInsertBig;
    internal static volatile Duplicate BlockDuplicate = Mutate.BlockDuplicateBig;
    internal static volatile Splice SpliceHook = Mutate.SpliceBig;
    internal static volatile Sequence MutateSequence = Mutate.MutateSequenceCounted;
    internal static volatile Admit CorpusAdmit = Bitmap.CorpusAdmit;
    internal static volatile Dedup IsDuplicate = (a, fa, b, fb) => FuzzSignature.IsDuplicateCrash(a, fa, b, fb);
    internal static volatile Func<string?, bool> IsFailureId = FuzzFailureIds.IsFuzzFailureId;
    internal static volatile Tap FormatTap = FuzzSignature.FormatTapFailure;
    internal static volatile Func<string, bool> TfvValid = TfvValidator.IsValid;

    /// <summary>Restores every hook to its production value.</summary>
    internal static void Reset()
    {
        BitFlip = Mutate.BitFlipBig;
        ByteFlip = Mutate.ByteFlipBig;
        ByteArithmetic = Mutate.ByteArithmeticBig;
        BoundaryInsert = Mutate.BoundaryInsertBig;
        BlockDelete = Mutate.BlockDeleteBig;
        BlockInsert = Mutate.BlockInsertBig;
        BlockDuplicate = Mutate.BlockDuplicateBig;
        SpliceHook = Mutate.SpliceBig;
        MutateSequence = Mutate.MutateSequenceCounted;
        CorpusAdmit = Bitmap.CorpusAdmit;
        IsDuplicate = (a, fa, b, fb) => FuzzSignature.IsDuplicateCrash(a, fa, b, fb);
        IsFailureId = FuzzFailureIds.IsFuzzFailureId;
        FormatTap = FuzzSignature.FormatTapFailure;
        TfvValid = TfvValidator.IsValid;
    }
}
