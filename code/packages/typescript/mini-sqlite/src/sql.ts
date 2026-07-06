// stripStatementTerminator trims leading whitespace and one trailing semicolon
// plus surrounding whitespace.  We do this imperatively rather than with a
// `\s*;?\s*$` regex tail because that pattern triggers polynomial backtracking
// (codeql js/polynomial-redos) on adversarial whitespace-heavy inputs.
function stripStatementTerminator(sql: string): string {
  let s = sql.trim();
  if (s.endsWith(";")) s = s.slice(0, -1).trimEnd();
  return s;
}

export function firstKeyword(sql: string): string {
  const trimmed = stripStatementTerminator(sql).trimStart();
  const match = /^[A-Za-z]+/.exec(trimmed);
  return match ? match[0].toUpperCase() : "";
}
