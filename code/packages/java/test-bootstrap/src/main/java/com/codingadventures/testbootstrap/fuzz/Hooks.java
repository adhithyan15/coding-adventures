package com.codingadventures.testbootstrap.fuzz;

import com.codingadventures.testbootstrap.fuzz.Mutate.Endian;
import com.codingadventures.testbootstrap.tfv.Tfv;
import java.math.BigInteger;
import java.util.List;
import java.util.function.Predicate;

/**
 * Sabotage hooks: the engine entry points the corpus runner calls, as swappable
 * fields. These are the only production indirections added for the negative
 * controls of VER06f section 10: a test replaces one with a broken version, runs
 * the corpus, and the runner must report problems. Production code never
 * reassigns them, and the class is package-private so no consumer can.
 */
final class Hooks {
  private Hooks() {}

  interface Flip {
    byte[] apply(byte[] data, BigInteger offset, BigInteger count);
  }

  interface Arithmetic {
    byte[] apply(byte[] data, BigInteger offset, BigInteger delta, int width, Endian e);
  }

  interface Boundary {
    byte[] apply(byte[] data, BigInteger offset, int width, BigInteger val, Endian e);
  }

  interface Insert {
    byte[] apply(byte[] data, BigInteger offset, byte[] insert);
  }

  interface Duplicate {
    byte[] apply(byte[] data, BigInteger src, BigInteger length, BigInteger dst);
  }

  interface Splice {
    byte[] apply(byte[] target, byte[] donor, BigInteger targetSplit, BigInteger donorSplit);
  }

  interface Sequence {
    Mutate.Counted apply(byte[] data, long seed, int steps, int maxLen, long workCap);
  }

  interface Admit {
    Bitmap.Novelty apply(long[] existing, long[] trace);
  }

  interface Dedup {
    boolean apply(String kindA, List<String> framesA, String kindB, List<String> framesB);
  }

  interface Tap {
    String apply(int n, String target, String failureId, String sig);
  }

  static volatile Flip bitFlip = Mutate::bitFlipBig;
  static volatile Flip byteFlip = Mutate::byteFlipBig;
  static volatile Arithmetic byteArithmetic = Mutate::byteArithmeticBig;
  static volatile Boundary boundaryInsert = Mutate::boundaryInsertBig;
  static volatile Flip blockDelete = Mutate::blockDeleteBig;
  static volatile Insert blockInsert = Mutate::blockInsertBig;
  static volatile Duplicate blockDuplicate = Mutate::blockDuplicateBig;
  static volatile Splice splice = Mutate::spliceBig;
  static volatile Sequence mutateSequence = Mutate::mutateSequenceCounted;
  static volatile Admit corpusAdmit = Bitmap::corpusAdmit;
  static volatile Dedup isDuplicate = Signature::isDuplicateCrash;
  static volatile Predicate<String> isFailureId = FailureIds::isFuzzFailureId;
  static volatile Tap formatTap = Signature::formatTapFailure;
  static volatile Predicate<String> tfvValid = Tfv::isValid;

  /** Restores every hook to its production value. */
  static void reset() {
    bitFlip = Mutate::bitFlipBig;
    byteFlip = Mutate::byteFlipBig;
    byteArithmetic = Mutate::byteArithmeticBig;
    boundaryInsert = Mutate::boundaryInsertBig;
    blockDelete = Mutate::blockDeleteBig;
    blockInsert = Mutate::blockInsertBig;
    blockDuplicate = Mutate::blockDuplicateBig;
    splice = Mutate::spliceBig;
    mutateSequence = Mutate::mutateSequenceCounted;
    corpusAdmit = Bitmap::corpusAdmit;
    isDuplicate = Signature::isDuplicateCrash;
    isFailureId = FailureIds::isFuzzFailureId;
    formatTap = Signature::formatTapFailure;
    tfvValid = Tfv::isValid;
  }
}
