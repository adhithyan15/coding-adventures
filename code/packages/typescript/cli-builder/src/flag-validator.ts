/**
 * flag-validator.ts — Validate flag constraints after scanning.
 *
 * === What Does Flag Validation Check? ===
 *
 * After Phase 2 (scanning) has built `parsedFlags`, we run a series of
 * constraint checks from §6.4.2:
 *
 * 1. **conflicts_with** — Two flags that cannot co-exist were both provided.
 *    The conflict is pairwise and bilateral (if A conflicts with B, and B
 *    is present when A is given, that is an error).
 *
 * 2. **requires (transitive)** — A flag was used but one of its (transitively
 *    required) dependencies is absent. We use the flag dependency graph G_flag
 *    to compute the transitive closure of dependencies.
 *
 * 3. **required flags** — A flag marked `required: true` is absent. Unless
 *    `required_unless` is satisfied (any one of those IDs is present).
 *
 * 4. **mutually_exclusive_groups** — Within each exclusive group, at most one
 *    flag may be present (or exactly one, if the group is `required: true`).
 *
 * Like PositionalResolver, FlagValidator collects ALL errors rather than
 * stopping at the first failure.
 *
 * === The Flag Dependency Graph (G_flag) ===
 *
 * G_flag is a directed graph where an edge A → B means "flag A requires B".
 * The spec validation in SpecLoader already guarantees G_flag has no cycles.
 *
 * At validation time, for each flag A that is present, we compute the
 * transitive closure of A in G_flag (all nodes reachable from A following
 * `requires` edges). Every flag in that closure must also be present.
 *
 * Example: -h (human-readable) requires -l (long). If -h is present but
 * -l is absent, we record: `missing_dependency_flag` for -h → -l.
 *
 * @module flag-validator
 */

import { Graph } from "@coding-adventures/directed-graph";
import type { ExclusiveGroup, FlagDef } from "./types.js";
import type { ParseError } from "./errors.js";

// ---------------------------------------------------------------------------
// FlagValidator
// ---------------------------------------------------------------------------

/**
 * Validates flag constraint rules against a set of parsed flags.
 *
 * Constructed with the active flags for the resolved command scope
 * and the exclusive groups at that scope.
 *
 * @example
 * ```typescript
 * const validator = new FlagValidator(activeFlags, exclusiveGroups);
 * const errors = validator.validate(parsedFlags, context);
 * ```
 */
export class FlagValidator {
  private readonly _activeFlags: FlagDef[];
  private readonly _exclusiveGroups: ExclusiveGroup[];
  /** Pre-built flag dependency graph for this scope. */
  private readonly _flagGraph: Graph;
  /** Map from flag ID to FlagDef for O(1) lookups. */
  private readonly _flagById: Map<string, FlagDef>;

  constructor(activeFlags: FlagDef[], exclusiveGroups: ExclusiveGroup[]) {
    this._activeFlags = activeFlags;
    this._exclusiveGroups = exclusiveGroups;
    this._flagById = new Map(activeFlags.map((f) => [f.id, f]));

    // Build G_flag: edge A → B means "A requires B"
    this._flagGraph = new Graph();
    for (const flag of activeFlags) {
      this._flagGraph.addNode(flag.id);
    }
    for (const flag of activeFlags) {
      for (const reqId of flag.requires) {
        if (this._flagGraph.hasNode(reqId)) {
          this._flagGraph.addEdge(flag.id, reqId);
        }
      }
    }
  }

  /**
   * Validate all flag constraints and return a list of errors.
   *
   * @param parsedFlags - The flags collected during Phase 2. Keys are
   *   flag IDs; values are the coerced values. A flag is "present" if its
   *   value is not `false` (booleans) or not `null` (non-booleans).
   * @param context - The current command_path for error messages.
   */
  validate(
    parsedFlags: Record<string, unknown>,
    context: string[],
  ): ParseError[] {
    const errors: ParseError[] = [];

    // Determine which flags are "present" in this invocation.
    // A flag is present if:
    // - It's a boolean and its value is true
    // - It's non-boolean and its value is not null
    // - It's repeatable and its value is an array with length > 0
    const presentFlagIds = new Set<string>(
      this._activeFlags
        .filter((f) => this._isFlagPresent(f.id, parsedFlags))
        .map((f) => f.id),
    );

    // --- Check 1: conflicts_with ---
    //
    // For each present flag, check all of its conflicting flag IDs.
    // We only report each conflict once (A vs B), not twice (A vs B AND B vs A).
    const reportedConflicts = new Set<string>();

    for (const flagId of presentFlagIds) {
      const flag = this._flagById.get(flagId);
      if (!flag) continue;

      for (const otherId of flag.conflictsWith) {
        if (presentFlagIds.has(otherId)) {
          const conflictKey = [flagId, otherId].sort().join("\0");
          if (!reportedConflicts.has(conflictKey)) {
            reportedConflicts.add(conflictKey);
            const other = this._flagById.get(otherId);
            errors.push({
              errorType: "conflicting_flags",
              message: `${this._flagDisplay(flag)} and ${this._flagDisplay(other)} cannot be used together`,
              context,
            });
          }
        }
      }
    }

    // --- Check 2: requires (transitive, via G_flag) ---
    //
    // For each present flag A, compute transitiveClosure(A) in G_flag.
    // Every flag in the closure must also be present.
    for (const flagId of presentFlagIds) {
      const required = this._flagGraph.transitiveClosure(flagId);
      for (const reqId of required) {
        if (!presentFlagIds.has(reqId)) {
          const flag = this._flagById.get(flagId);
          const reqFlag = this._flagById.get(reqId);
          errors.push({
            errorType: "missing_dependency_flag",
            message: `${this._flagDisplay(flag)} requires ${this._flagDisplay(reqFlag)}`,
            context,
          });
        }
      }
    }

    // --- Check 3: required flags ---
    //
    // For each flag marked required: true, it must be present.
    // Unless any flag listed in required_unless is also present.
    for (const flag of this._activeFlags) {
      if (!flag.required) continue;
      if (presentFlagIds.has(flag.id)) continue;

      const isExempt = flag.requiredUnless.some((id) =>
        presentFlagIds.has(id),
      );
      if (!isExempt) {
        errors.push({
          errorType: "missing_required_flag",
          message: `${this._flagDisplay(flag)} is required`,
          context,
        });
      }
    }

    // --- Check 4: mutually_exclusive_groups ---
    //
    // For each group, count how many of its flags are present.
    // If required: at least 1 must be present.
    // If not required: at most 1 may be present.
    for (const group of this._exclusiveGroups) {
      const presentInGroup = group.flagIds.filter((id) =>
        presentFlagIds.has(id),
      );

      if (presentInGroup.length > 1) {
        const flagNames = presentInGroup
          .map((id) => this._flagDisplay(this._flagById.get(id)))
          .join(", ");
        errors.push({
          errorType: "exclusive_group_violation",
          message: `Only one of ${flagNames} may be used`,
          context,
        });
      }

      if (group.required && presentInGroup.length === 0) {
        const flagNames = group.flagIds
          .map((id) => this._flagDisplay(this._flagById.get(id)))
          .join(", ");
        errors.push({
          errorType: "missing_exclusive_group",
          message: `One of ${flagNames} is required`,
          context,
        });
      }
    }

    return errors;
  }

  /**
   * Determine if a flag is "present" in parsedFlags.
   *
   * - Boolean flags: present if value === true
   * - Repeatable flags: present if value is an array with length > 0
   * - Other flags: present if value is not null and not undefined
   */
  private _isFlagPresent(
    flagId: string,
    parsedFlags: Record<string, unknown>,
  ): boolean {
    const flag = this._flagById.get(flagId);
    if (!flag) return false;

    const val = parsedFlags[flagId];

    if (flag.repeatable) {
      return Array.isArray(val) && val.length > 0;
    }
    if (flag.type === "boolean") {
      return val === true;
    }
    return val !== null && val !== undefined;
  }

  /**
   * Format a flag for display in an error message.
   * Returns something like "-l/--long-listing" or "--verbose".
   */
  private _flagDisplay(flag: FlagDef | undefined): string {
    if (!flag) return "(unknown)";
    const parts: string[] = [];
    if (flag.short) parts.push(`-${flag.short}`);
    if (flag.long) parts.push(`--${flag.long}`);
    if (flag.singleDashLong) parts.push(`-${flag.singleDashLong}`);
    return parts.join("/");
  }
}
