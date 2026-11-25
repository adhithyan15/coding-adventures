package com.codingadventures.testbootstrap.fuzz;

import com.codingadventures.testbootstrap.prng.SplitMix64;

/**
 * What a {@link Target} sees: coverage reporting and two deterministic budgets
 * (VER06f sections 4.2 and 5). One handle is reused across executions; a target
 * must not retain it.
 */
public final class Handle {
  /** Which budget a sentinel stands for. */
  enum Budget {
    STEPS,
    MEMORY
  }

  /**
   * Thrown by {@link #tick()} and {@link #reserve(long)}. It is an {@link Error},
   * not an {@link Exception}, so an ordinary {@code catch (Exception)} in the
   * target does not swallow it; it carries the handle that raised it so an inner
   * campaign never claims an outer campaign's budget. It captures no stack: it is
   * thrown from tight loops.
   */
  static final class BudgetExceeded extends Error {
    private static final long serialVersionUID = 1L;
    final transient Handle owner;
    final Budget budget;

    BudgetExceeded(Handle owner, Budget budget) {
      super(null, null, false, false);
      this.owner = owner;
      this.budget = budget;
    }
  }

  Bitmap cov;
  int steps;
  final int stepBudget;
  long reserved;
  final int memBudget;

  /**
   * The sentinel of the FIRST breach of either budget. Exhaustion is sticky and shared:
   * once set, {@link #tick()} and {@link #reserve(long)} both rethrow it at once, so a
   * target that swallows one breach and calls the other budget's hook is still stopped
   * and reported as the budget that ran out first.
   */
  BudgetExceeded exhausted;

  Handle(int stepBudget, int memBudget) {
    this.stepBudget = stepBudget;
    this.memBudget = memBudget;
  }

  /**
   * A stable edge id derived from a name (FNV-1a), so a campaign is deterministic
   * across runs and machines (no identity hash codes, no hash randomisation).
   */
  public static long edgeId(String label) {
    return SplitMix64.fnv1a64(label);
  }

  /** Reports that the target executed the program point with this id (reduced mod 65,536, unsigned). */
  public void edge(long id) {
    cov.record(id);
  }

  /**
   * Counts one unit of work. After {@code stepBudget} ticks the execution is a
   * timeout: {@code tick} throws a private sentinel, and keeps throwing on every
   * later call, so a target that swallows one {@code catch (Throwable)} cannot keep
   * ticking past the budget. A target that loops without ever ticking cannot be
   * stopped (Java cannot pre-empt a thread and the engine starts none), so CI
   * needs an external wall-clock limit.
   */
  public void tick() {
    if (exhausted != null) {
      throw exhausted;
    }
    if (steps >= stepBudget) {
      exhausted = new BudgetExceeded(this, Budget.STEPS);
      throw exhausted;
    }
    steps++;
  }

  /**
   * Declares an allocation of {@code n} bytes about to be made on behalf of the
   * input. When the running total of one execution exceeds {@code memBudget} the
   * execution is an out-of-memory outcome. A non-positive {@code n} reserves
   * nothing. Call it before {@code new byte[n]} sized by input data.
   */
  public void reserve(long n) {
    if (exhausted != null) {
      throw exhausted;
    }
    if (n <= 0) {
      return;
    }
    if (n > memBudget - reserved) {
      exhausted = new BudgetExceeded(this, Budget.MEMORY);
      throw exhausted;
    }
    reserved += n;
  }
}
