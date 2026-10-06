/**
 * glob-match.ts -- Bounded Portable Glob Matching
 * =================================================
 *
 * BUILD files are shared data: a declared source pattern must mean the same
 * thing in TypeScript, Go, Python, Ruby, and every other build-tool engine.
 * Delegating to a host regular-expression or filesystem glob implementation
 * would make that promise depend on the host. This module instead compiles a
 * deliberately small language:
 *
 * - a whole double-star segment crosses zero or more path segments;
 * - `*` consumes zero or more Unicode scalars inside one segment;
 * - `?` consumes exactly one Unicode scalar inside one segment; and
 * - `[...]` consumes one scalar from a strict literal/range class.
 *
 * Compilation validates the whole pattern before matching. Both matching
 * layers use rolling-row dynamic programs, so an adversarial near miss visits
 * each pattern/candidate state once rather than recursively revisiting suffixes.
 *
 * @module
 */

const INVALID_PATTERN_MESSAGE =
  "ambiguous or descending character class in glob pattern";

/** Stable host-visible failure for rejected portable glob syntax. */
export class GlobPatternError extends Error {
  constructor() {
    super(INVALID_PATTERN_MESSAGE);
    this.name = "GlobPatternError";
  }
}

type LiteralMember = Readonly<{
  kind: "literal";
  value: string;
}>;

type RangeMember = Readonly<{
  kind: "range";
  start: string;
  end: string;
}>;

type CharacterClassMember = LiteralMember | RangeMember;

type SegmentToken =
  | Readonly<{ kind: "star" }>
  | Readonly<{ kind: "question" }>
  | LiteralMember
  | Readonly<{
      kind: "character-class";
      negated: boolean;
      members: readonly CharacterClassMember[];
    }>;

type CompiledSegment =
  | Readonly<{ kind: "globstar" }>
  | Readonly<{ kind: "segment"; tokens: readonly SegmentToken[] }>;

/** Immutable, validated representation of one portable path pattern. */
export type CompiledPattern = readonly CompiledSegment[];

/** Parser evidence used by tests to pin linear bracket lookup work. */
export type ParsedSegmentWithStateCount = Readonly<{
  tokens: readonly SegmentToken[];
  visitedStates: number;
}>;

/** Matcher evidence used by tests to pin the dynamic-program state bound. */
export type MatchWithStateCount = Readonly<{
  matched: boolean;
  visitedStates: number;
}>;

const GLOBSTAR: CompiledSegment = Object.freeze({ kind: "globstar" });

function unicodeScalarValue(value: string): number {
  // Every caller supplies one value from Array.from(), so this cannot be empty.
  return value.codePointAt(0) as number;
}

function splitPath(value: string): string[] {
  if (value.length === 0) return [];
  return value.split("/").filter((segment) => segment.length > 0);
}

function parseCharacterClass(
  scalars: readonly string[],
  opening: number,
  nextClosingBracket: readonly (number | undefined)[],
): Readonly<{ token: SegmentToken; nextIndex: number }> | null {
  let cursor = opening + 1;
  const negated = cursor < scalars.length && scalars[cursor] === "!";
  if (negated) cursor += 1;

  // A leading closing bracket is a literal member, so the class closes at the
  // following bracket. Without that following bracket, the opening bracket is
  // just a literal, matching Python fnmatchcase.
  let closing = nextClosingBracket[cursor];
  if (closing === cursor) closing = nextClosingBracket[cursor + 1];
  if (closing === undefined) return null;

  const body = scalars.slice(cursor, closing);
  for (let index = 0; index + 1 < body.length; index += 1) {
    const value = body[index];
    if (
      value === body[index + 1] &&
      (value === "-" || value === "&" || value === "~" || value === "|")
    ) {
      throw new GlobPatternError();
    }
  }

  const members: CharacterClassMember[] = [];
  let memberIndex = 0;
  while (memberIndex < body.length) {
    if (memberIndex + 2 < body.length && body[memberIndex + 1] === "-") {
      const start = body[memberIndex];
      const end = body[memberIndex + 2];
      if (unicodeScalarValue(start) > unicodeScalarValue(end)) {
        throw new GlobPatternError();
      }
      members.push(Object.freeze({ kind: "range", start, end }));
      memberIndex += 3;
    } else {
      members.push(Object.freeze({ kind: "literal", value: body[memberIndex] }));
      memberIndex += 1;
    }
  }

  return Object.freeze({
    token: Object.freeze({
      kind: "character-class",
      negated,
      members: Object.freeze(members),
    }),
    nextIndex: closing + 1,
  });
}

/** Compile one path segment while reporting deterministic parser work. */
export function parseSegmentWithStateCount(
  segment: string,
): ParsedSegmentWithStateCount {
  const scalars = Array.from(segment);
  const nextClosingBracket = new Array<number | undefined>(scalars.length + 1);
  let nextClosing: number | undefined;

  // This reverse pass makes every later opening-bracket lookup O(1). Without
  // it, a run of unmatched brackets would repeatedly scan the same suffix.
  for (let index = scalars.length - 1; index >= 0; index -= 1) {
    if (scalars[index] === "]") nextClosing = index;
    nextClosingBracket[index] = nextClosing;
  }

  const tokens: SegmentToken[] = [];
  let index = 0;
  let visitedStates = scalars.length;
  while (index < scalars.length) {
    visitedStates += 1;
    const scalar = scalars[index];
    if (scalar === "*") {
      if (tokens.at(-1)?.kind !== "star") {
        tokens.push(Object.freeze({ kind: "star" }));
      }
      index += 1;
    } else if (scalar === "?") {
      tokens.push(Object.freeze({ kind: "question" }));
      index += 1;
    } else if (scalar === "[") {
      const parsed = parseCharacterClass(scalars, index, nextClosingBracket);
      if (parsed === null) {
        tokens.push(Object.freeze({ kind: "literal", value: "[" }));
        index += 1;
      } else {
        tokens.push(parsed.token);
        index = parsed.nextIndex;
      }
    } else {
      tokens.push(Object.freeze({ kind: "literal", value: scalar }));
      index += 1;
    }
  }

  return Object.freeze({
    tokens: Object.freeze(tokens),
    visitedStates,
  });
}

function compileSegment(segment: string): CompiledSegment {
  return Object.freeze({
    kind: "segment",
    tokens: parseSegmentWithStateCount(segment).tokens,
  });
}

/** Compile and validate one portable path pattern. */
export function compilePattern(pattern: string): CompiledPattern {
  const segments: CompiledSegment[] = [];
  for (const segment of splitPath(pattern)) {
    if (segment === "**") {
      if (segments.at(-1)?.kind !== "globstar") segments.push(GLOBSTAR);
    } else {
      segments.push(compileSegment(segment));
    }
  }
  return Object.freeze(segments);
}

/** Compile the complete declared list before a caller examines candidates. */
export function compilePatterns(
  patterns: readonly string[],
): readonly CompiledPattern[] {
  return Object.freeze(patterns.map((pattern) => compilePattern(pattern)));
}

function tokenMatches(token: SegmentToken, value: string): boolean {
  if (token.kind === "literal") return token.value === value;
  if (token.kind !== "character-class") return false;

  const included = token.members.some((member) => {
    if (member.kind === "literal") return member.value === value;
    const scalar = unicodeScalarValue(value);
    return (
      unicodeScalarValue(member.start) <= scalar &&
      scalar <= unicodeScalarValue(member.end)
    );
  });
  return token.negated ? !included : included;
}

function matchCompiledSegmentWithStateCount(
  tokens: readonly SegmentToken[],
  text: string,
): MatchWithStateCount {
  const values = Array.from(text);
  const valueCount = values.length;
  let nextRow = new Array<boolean>(valueCount + 1).fill(false);
  nextRow[valueCount] = true;

  for (let tokenIndex = tokens.length - 1; tokenIndex >= 0; tokenIndex -= 1) {
    const token = tokens[tokenIndex];
    const row = new Array<boolean>(valueCount + 1).fill(false);
    if (token.kind === "star") {
      row[valueCount] = nextRow[valueCount];
      for (let valueIndex = valueCount - 1; valueIndex >= 0; valueIndex -= 1) {
        row[valueIndex] = nextRow[valueIndex] || row[valueIndex + 1];
      }
    } else if (token.kind === "question") {
      for (let valueIndex = 0; valueIndex < valueCount; valueIndex += 1) {
        row[valueIndex] = nextRow[valueIndex + 1];
      }
    } else {
      for (let valueIndex = 0; valueIndex < valueCount; valueIndex += 1) {
        row[valueIndex] =
          nextRow[valueIndex + 1] && tokenMatches(token, values[valueIndex]);
      }
    }
    nextRow = row;
  }

  return Object.freeze({
    matched: nextRow[0],
    visitedStates: (tokens.length + 1) * (valueCount + 1),
  });
}

/** Match one segment and report its exact dynamic-program state count. */
export function matchSegmentWithStateCount(
  pattern: string,
  text: string,
): MatchWithStateCount {
  return matchCompiledSegmentWithStateCount(
    parseSegmentWithStateCount(pattern).tokens,
    text,
  );
}

/** Match one segment against `*`, `?`, literals, and strict classes. */
export function matchSegment(pattern: string, text: string): boolean {
  return matchSegmentWithStateCount(pattern, text).matched;
}

/** Match a compiled pattern without repeating parser work. */
export function matchCompiledPathWithStateCount(
  compiledPattern: CompiledPattern,
  filePath: string,
): MatchWithStateCount {
  const pathSegments = splitPath(filePath);
  const pathCount = pathSegments.length;
  let nextRow = new Array<boolean>(pathCount + 1).fill(false);
  nextRow[pathCount] = true;
  let visitedStates = pathCount + 1;

  for (
    let patternIndex = compiledPattern.length - 1;
    patternIndex >= 0;
    patternIndex -= 1
  ) {
    const segment = compiledPattern[patternIndex];
    const row = new Array<boolean>(pathCount + 1).fill(false);
    visitedStates += pathCount + 1;
    if (segment.kind === "globstar") {
      row[pathCount] = nextRow[pathCount];
      for (let pathIndex = pathCount - 1; pathIndex >= 0; pathIndex -= 1) {
        row[pathIndex] = nextRow[pathIndex] || row[pathIndex + 1];
      }
    } else {
      for (let pathIndex = pathCount - 1; pathIndex >= 0; pathIndex -= 1) {
        row[pathIndex] =
          nextRow[pathIndex + 1] &&
          matchCompiledSegmentWithStateCount(
            segment.tokens,
            pathSegments[pathIndex],
          ).matched;
      }
    }
    nextRow = row;
  }

  return Object.freeze({ matched: nextRow[0], visitedStates });
}

/** Match a path against a previously compiled portable pattern. */
export function matchCompiledPath(
  compiledPattern: CompiledPattern,
  filePath: string,
): boolean {
  return matchCompiledPathWithStateCount(compiledPattern, filePath).matched;
}

/** Compile and match one path while reporting path-level state work. */
export function matchPathWithStateCount(
  pattern: string,
  filePath: string,
): MatchWithStateCount {
  return matchCompiledPathWithStateCount(compilePattern(pattern), filePath);
}

/** Compile and match one path against one portable glob pattern. */
export function matchPath(pattern: string, filePath: string): boolean {
  return matchPathWithStateCount(pattern, filePath).matched;
}
