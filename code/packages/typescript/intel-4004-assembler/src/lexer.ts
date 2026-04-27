export interface ParsedLine {
  readonly label: string | null;
  readonly mnemonic: string | null;
  readonly operands: readonly string[];
  readonly source: string;
}

const LABEL_RE = /^([A-Za-z_][A-Za-z0-9_]*):/;

export function lexLine(source: string): ParsedLine {
  const commentFree = source.split(";", 1)[0].trimEnd();
  let stripped = commentFree.trimStart();
  let label: string | null = null;

  const match = LABEL_RE.exec(stripped);
  if (match) {
    label = match[1];
    stripped = stripped.slice(match[0].length).trimStart();
  }

  if (!stripped) {
    return { label, mnemonic: null, operands: [], source };
  }

  const firstWhitespace = stripped.search(/\s/);
  if (firstWhitespace === -1) {
    return { label, mnemonic: stripped.toUpperCase(), operands: [], source };
  }

  const mnemonic = stripped.slice(0, firstWhitespace).toUpperCase();
  const operandText = stripped.slice(firstWhitespace).trim();
  if (!operandText) {
    return { label, mnemonic, operands: [], source };
  }

  const operands = operandText
    .split(",")
    .map((operand) => operand.trim())
    .filter((operand) => operand.length > 0);

  return { label, mnemonic, operands, source };
}

export function lexProgram(text: string): ParsedLine[] {
  return text.split(/\r?\n/).map((line) => lexLine(line));
}
