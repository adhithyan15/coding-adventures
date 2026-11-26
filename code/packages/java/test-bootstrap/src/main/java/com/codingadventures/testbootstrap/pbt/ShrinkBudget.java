package com.codingadventures.testbootstrap.pbt;

import java.util.function.Predicate;

/**
 * The budget of a shrink, as a tree of scopes (VER05e section 4.1, ported from
 * the VER05g security review and redesigned over four review rounds).
 *
 * <p>Every public entry point that runs user code (a property, a shrinker)
 * opens a <em>child</em> node of the ambient scope with a fresh local cap;
 * internal combinators join their enclosing node. A charge walks from the node
 * up to the outermost root and is <b>all or nothing</b>: every node on the path
 * must have room, then every counter is incremented. Whatever a property or a
 * custom shrinker does, however it nests, the total work of one top-level
 * {@code Runner.check} is therefore capped by the outermost root. The bound
 * does not depend on classifying a call as "inside a property" or "inside a
 * shrinker".
 *
 * <p>Two things are charged: <b>attempts</b> (one per property evaluation) and
 * <b>work</b> (elements copied, at least 1 per charge). One lock, owned by the
 * outermost root, guards every counter in its tree; user code never runs while
 * it is held.
 *
 * <p>Scopes are tracked in an {@link InheritableThreadLocal}: a thread created
 * inside a scope starts with that node and charges the same tree. A parent is
 * the nearest <em>open</em> ancestor; a thread whose whole chain has closed gets
 * a fresh root, so a dead root is never charged. Threads that already existed
 * (pool workers) have no ambient scope; see the spec.
 */
final class ShrinkBudget implements AutoCloseable {
  /** The most nested owning scopes; deeper scopes join, keeping the charge walk short. */
  static final int MAX_DEPTH = 64;

  private static final InheritableThreadLocal<ShrinkBudget> AMBIENT = new InheritableThreadLocal<>();

  /** Test hook, read only when a root is created ({@code Hooks.reset} restores it). */
  static long defaultMaxWork = Limits.MAX_SHRINK_WORK;

  /** Work spent by the most recent owning scope that ended (for tests). */
  static volatile long lastWork;

  /** Attempts spent by the most recent owning scope that ended (for tests). */
  static volatile long lastAttempts;

  private final Object lock; // the outermost root's lock, shared by the whole tree
  private final ShrinkBudget root; // outermost node of the tree (this for a root)
  private final ShrinkBudget parent; // nearest open ancestor at creation, or null
  private final ShrinkBudget target; // non-null for a joined handle: the owner it aliases
  private final ShrinkBudget previous; // ambient value to restore when an owner ends
  private final int depth;
  private final long maxAttempts;
  private final long maxWork;
  private long attempts; // guarded by lock
  private long work; // guarded by lock
  private volatile boolean closed;
  private volatile boolean refused; // set on a ROOT when any charge in its tree was refused

  private ShrinkBudget(ShrinkBudget parent, long maxAttempts, long maxWork, ShrinkBudget previous) {
    this.parent = parent;
    this.root = parent == null ? this : parent.root;
    this.lock = parent == null ? new Object() : parent.lock;
    this.target = null;
    this.previous = previous;
    this.depth = parent == null ? 0 : parent.depth + 1;
    this.maxAttempts = maxAttempts;
    this.maxWork = maxWork;
  }

  /** A joined handle: charges and queries go to the owner it aliases. */
  private ShrinkBudget(ShrinkBudget owner) {
    this.parent = null;
    this.root = owner.root;
    this.lock = owner.lock;
    this.target = owner;
    this.previous = null;
    this.depth = owner.depth;
    this.maxAttempts = 0;
    this.maxWork = 0;
  }

  private ShrinkBudget owner() {
    return target != null ? target : this;
  }

  /** True when this scope has an owning ancestor (it is not an outermost root). */
  boolean isNested() {
    return owner().parent != null;
  }

  /** True once any charge anywhere in this scope's tree has been refused. */
  boolean wasRefused() {
    return root.refused;
  }

  /** True for a handle that joined an existing scope instead of owning one. */
  boolean isJoined() {
    return target != null;
  }

  /** The nearest open node from {@code a} upward whose root is open, or null. */
  private static ShrinkBudget usable(ShrinkBudget a) {
    while (a != null && a.closed) {
      a = a.parent;
    }
    return a != null && !a.root.closed ? a : null;
  }

  /** The ambient open budget of this thread, or null outside any shrink. */
  static ShrinkBudget current() {
    return usable(AMBIENT.get());
  }

  /** Joins the ambient scope when there is one, else opens a fresh root. Close the result. */
  static ShrinkBudget enter(int maxSteps) {
    ShrinkBudget ambient = AMBIENT.get();
    ShrinkBudget open = usable(ambient);
    if (open != null) {
      return new ShrinkBudget(open);
    }
    return newRoot(Limits.attemptBudget(maxSteps), defaultMaxWork, null);
  }

  /**
   * The scope of a public entry point: a CHILD of the ambient scope with a fresh
   * local cap, or a fresh root when there is no open ambient scope. Deeper than
   * {@link #MAX_DEPTH} it joins instead.
   */
  static ShrinkBudget enterChild(int maxSteps) {
    return enterChild(maxSteps, Limits.MAX_SHRINK_WORK);
  }

  /** {@link #enterChild(int)} with an explicit local work cap (also the root cap when none is open). */
  static ShrinkBudget enterChild(int maxSteps, long maxWork) {
    ShrinkBudget ambient = AMBIENT.get();
    ShrinkBudget open = usable(ambient);
    if (open == null) {
      return newRoot(
          Limits.attemptBudget(maxSteps),
          maxWork == Limits.MAX_SHRINK_WORK ? defaultMaxWork : maxWork,
          null);
    }
    if (open.depth + 1 >= MAX_DEPTH) {
      return new ShrinkBudget(open);
    }
    ShrinkBudget child = new ShrinkBudget(open, Limits.attemptBudget(maxSteps), maxWork, ambient);
    AMBIENT.set(child);
    return child;
  }

  /**
   * The scope of a whole {@code Runner.check} (case loop included): a root when no
   * open ambient scope exists, a child otherwise. Its attempt cap is
   * {@code numTests} plus the shrink attempt cap, so a nested check's cases and
   * shrink are all charged up the chain.
   */
  static ShrinkBudget enterCheck(long numTests, int maxSteps) {
    long attempts = numTests + Limits.attemptBudget(maxSteps);
    ShrinkBudget ambient = AMBIENT.get();
    ShrinkBudget open = usable(ambient);
    if (open == null) {
      return newRoot(attempts, defaultMaxWork, null);
    }
    if (open.depth + 1 >= MAX_DEPTH) {
      return new ShrinkBudget(open);
    }
    ShrinkBudget child = new ShrinkBudget(open, attempts, Limits.MAX_SHRINK_WORK, ambient);
    AMBIENT.set(child);
    return child;
  }

  private static ShrinkBudget newRoot(long maxAttempts, long maxWork, ShrinkBudget previous) {
    ShrinkBudget fresh = new ShrinkBudget(null, maxAttempts, maxWork, previous);
    AMBIENT.set(fresh);
    return fresh;
  }

  /** Attempts charged to this node (and its joined handles) so far. */
  long attempts() {
    ShrinkBudget o = owner();
    synchronized (o.lock) {
      return o.attempts;
    }
  }

  /** Elements charged to this node so far. */
  long work() {
    ShrinkBudget o = owner();
    synchronized (o.lock) {
      return o.work;
    }
  }

  /**
   * Charges one attempt over a candidate of {@code elements} elements (at least
   * 1) to this node and every ancestor. Returns false, charging nothing, when
   * any of them cannot afford it: callers must check this BEFORE building the
   * candidate.
   */
  boolean charge(long elements) {
    return take(elements, 1);
  }

  /**
   * Charges a copy that is not itself a property evaluation (an initial copy, a
   * list-to-array conversion): work only, so attempt counts are unchanged.
   */
  boolean chargeCopy(long elements) {
    return take(elements, 0);
  }

  private boolean take(long elements, int attemptCost) {
    final ShrinkBudget start = owner();
    final long cost = Math.max(1, elements);
    synchronized (start.lock) {
      for (ShrinkBudget n = start; n != null; n = n.parent) {
        // Overflow-safe: work <= maxWork always, so the subtraction cannot wrap.
        if (n.attempts >= n.maxAttempts || cost > n.maxWork - n.work) {
          start.root.refused = true;
          return false;
        }
      }
      for (ShrinkBudget n = start; n != null; n = n.parent) {
        n.attempts += attemptCost;
        n.work += cost;
      }
      return true;
    }
  }

  /** True while at least one more attempt of one element could be afforded everywhere on the path. */
  boolean hasRoom() {
    final ShrinkBudget start = owner();
    synchronized (start.lock) {
      for (ShrinkBudget n = start; n != null; n = n.parent) {
        if (n.attempts >= n.maxAttempts || n.work >= n.maxWork) {
          return false;
        }
      }
      return true;
    }
  }

  /** Wraps a property so each evaluation is charged one attempt first (false once refused). */
  <V> Predicate<V> charged(Predicate<V> property) {
    return v -> charge(1) && property.test(v);
  }

  @Override
  public void close() {
    if (target != null) {
      return;
    }
    try {
      // Restore the ambient budget FIRST, so nothing below can leave a stale one.
      if (previous == null) {
        AMBIENT.remove();
      } else {
        AMBIENT.set(previous);
      }
    } finally {
      closed = true;
      synchronized (lock) {
        lastWork = work;
        lastAttempts = attempts;
      }
    }
  }
}
