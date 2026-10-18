# HL72 — Spanish changelog direct owners

## 1. Problem

After #17701 migrated nine non-Indian track changelogs, Spanish remained the
only registered human-language track with a tracked `CHANGELOG.md`. At the
exact post-merge main revision, that aggregate contains 213 independently
authored level-2 entries in 282,875 bytes. It appears in 0, 3, and 5 commits in
the first-parent 50/100/200 human-language windows. The quiet short window
reflects which tracks happened to receive recent work; it does not give future
Spanish authors independent ownership.

Spanish is still work in progress, and every track is intended to continue
through genuine C2. Keeping its release notes in one prepend-only file would
therefore restore the same avoidable same-track merge seam as soon as parallel
Spanish work resumes.

Spanish also predates the strict document-shard convention: 52 detailed change
reports live in lowercase `changelog.d/`. They are independent narrative
artifacts, not sections of the aggregate. On case-insensitive filesystems that
directory aliases the canonical uppercase `CHANGELOG.d/` path, so both cannot
coexist safely.

## 2. Ownership boundary

`code/learning/human-languages/spanish/CHANGELOG.d/` is the source of truth.

- `_meta.md` owns only the document preamble.
- Each level-2 changelog entry owns one strict rank/slug/heading-digest file.
- Rank order reconstructs the document newest first. Equal ranks remain safe
  because the heading slug and digest provide distinct deterministic names.
- `CHANGELOG.md` is an ignored local projection, never tracked content.

Adding one Spanish release note must add one owner file. It must not edit
`_meta.md`, an earlier owner, or a shared aggregate.

The 52 pre-existing detailed reports move byte-for-byte to
`spanish/change-notes.d/`. Their filenames and contents remain unchanged. This
separate name describes their role and cannot case-fold onto the canonical
release-note shard directory.

## 3. Migration contract

Split the existing document only at level-2 headings and reproduce all 213
entries, every historical byte, and their order exactly. This migration may not
normalize or rewrite prose, spacing, punctuation, or headings.

Before splitting, move the legacy narrative-report directory as one preserved
file set. Pin its exact 52-file name/content digest so the path repair cannot
silently drop, merge, or rewrite a report.

Spanish inherits the existing strict document-shard protections: exact plan
matching, real-file and symlink checks, repository-relative path containment,
canonical filenames, filename/body agreement, case-fold collision rejection,
append-only history, deterministic sorting, and exact split/join reconstruction.

## 4. Regression contract

Focused tests must prove that Spanish:

1. is registered as a newest-first level-2 document-shard plan;
2. reconstructs the pre-migration document byte-for-byte from direct owners;
3. has no tracked aggregate in a clean checkout; and
4. appears in the append-only deletion guard, aggregate resurrection guard,
   ignore policy, and shard whitespace policy.
5. preserves all 52 legacy detailed reports at `change-notes.d/`, with no
   lowercase `changelog.d/` path left to collide on case-insensitive systems.

The general adversarial document-shard tests continue to reject malformed,
linked, non-canonical, and case-fold-colliding owners for every registered plan.

## 5. Verification

Run the focused document-shard tests, the TypeScript build, the repository
document-shard check, the full human-language-data suite and coverage, and the
exact-main hot-file audit after merge.
