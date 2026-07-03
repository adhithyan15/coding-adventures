/**
 * Single source of truth for the package version.
 *
 * Kept in sync with package.json's `version` field by hand — there's no
 * build step that injects it.  When bumping, update both files.
 */
export const VERSION = "1.7.0" as const;
