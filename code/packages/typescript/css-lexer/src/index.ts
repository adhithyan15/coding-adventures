export interface CssToken {
  type: string;
  value: string;
  line: number;
  column: number;
}

interface Pattern {
  type: string;
  regex?: RegExp;
  literal?: string;
  transform?: (raw: string) => string;
}

const skipPatterns: RegExp[] = [/^\/\*[\s\S]*?\*\//, /^[ \t\r\n]+/];

const tokenPatterns: Pattern[] = [
  { type: "STRING", regex: /^"([^"\\\n]|\\.)*"/, transform: stripQuotes },
  { type: "STRING", regex: /^'([^'\\\n]|\\.)*'/, transform: stripQuotes },
  { type: "DIMENSION", regex: /^-?[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?[a-zA-Z]+/ },
  { type: "PERCENTAGE", regex: /^-?[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?%/ },
  { type: "NUMBER", regex: /^-?[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?/ },
  { type: "HASH", regex: /^#[a-zA-Z0-9_-]+/ },
  { type: "AT_KEYWORD", regex: /^@-?[a-zA-Z][a-zA-Z0-9-]*/ },
  { type: "URL_TOKEN", regex: /^url\([^)'"]*\)/ },
  { type: "FUNCTION", regex: /^-?[a-zA-Z_][a-zA-Z0-9_-]*\(/ },
  { type: "CDO", literal: "<!--" },
  { type: "CDC", literal: "-->" },
  { type: "UNICODE_RANGE", regex: /^[Uu]\+[0-9a-fA-F?]{1,6}(-[0-9a-fA-F]{1,6})?/ },
  { type: "CUSTOM_PROPERTY", regex: /^--[a-zA-Z_][a-zA-Z0-9_-]*/ },
  { type: "IDENT", regex: /^-?[a-zA-Z_][a-zA-Z0-9_-]*/ },
  { type: "COLON_COLON", literal: "::" },
  { type: "TILDE_EQUALS", literal: "~=" },
  { type: "PIPE_EQUALS", literal: "|=" },
  { type: "CARET_EQUALS", literal: "^=" },
  { type: "DOLLAR_EQUALS", literal: "$=" },
  { type: "STAR_EQUALS", literal: "*=" },
  { type: "LBRACE", literal: "{" },
  { type: "RBRACE", literal: "}" },
  { type: "LPAREN", literal: "(" },
  { type: "RPAREN", literal: ")" },
  { type: "LBRACKET", literal: "[" },
  { type: "RBRACKET", literal: "]" },
  { type: "SEMICOLON", literal: ";" },
  { type: "COLON", literal: ":" },
  { type: "COMMA", literal: "," },
  { type: "DOT", literal: "." },
  { type: "PLUS", literal: "+" },
  { type: "GREATER", literal: ">" },
  { type: "TILDE", literal: "~" },
  { type: "STAR", literal: "*" },
  { type: "PIPE", literal: "|" },
  { type: "BANG", literal: "!" },
  { type: "SLASH", literal: "/" },
  { type: "EQUALS", literal: "=" },
  { type: "AMPERSAND", literal: "&" },
  { type: "MINUS", literal: "-" },
];

const errorPatterns: Pattern[] = [
  { type: "BAD_STRING", regex: /^"[^"]*$/ },
  { type: "BAD_URL", regex: /^url\([^)]*$/ },
];

export class CssLexerError extends Error {
  constructor(message: string, readonly line: number, readonly column: number) {
    super(message);
    this.name = "CssLexerError";
  }
}

export function tokenizeCss(source: string): CssToken[] {
  const lexer = new CssLexer(source);
  return lexer.tokenize();
}

export class CssLexer {
  private offset = 0;
  private line = 1;
  private column = 1;

  constructor(private readonly source: string) {}

  tokenize(): CssToken[] {
    const tokens: CssToken[] = [];

    while (!this.atEnd()) {
      if (this.skipIgnored()) {
        continue;
      }

      const token = this.matchPatterns(tokenPatterns) ?? this.matchPatterns(errorPatterns);
      if (token === undefined) {
        throw new CssLexerError(
          `Unexpected character ${JSON.stringify(this.source[this.offset])}`,
          this.line,
          this.column,
        );
      }
      tokens.push(token);
    }

    tokens.push({ type: "EOF", value: "", line: this.line, column: this.column });
    return tokens;
  }

  private atEnd(): boolean {
    return this.offset >= this.source.length;
  }

  private skipIgnored(): boolean {
    for (const regex of skipPatterns) {
      const match = regex.exec(this.remaining());
      if (match?.[0]) {
        this.advance(match[0]);
        return true;
      }
    }
    return false;
  }

  private matchPatterns(patterns: Pattern[]): CssToken | undefined {
    const remaining = this.remaining();
    for (const pattern of patterns) {
      const raw = pattern.literal !== undefined
        ? remaining.startsWith(pattern.literal) ? pattern.literal : undefined
        : pattern.regex?.exec(remaining)?.[0];
      if (raw === undefined || raw.length === 0) {
        continue;
      }

      const token: CssToken = {
        type: pattern.type,
        value: pattern.transform?.(raw) ?? raw,
        line: this.line,
        column: this.column,
      };
      this.advance(raw);
      return token;
    }
    return undefined;
  }

  private remaining(): string {
    return this.source.slice(this.offset);
  }

  private advance(text: string): void {
    for (const char of text) {
      this.offset += char.length;
      if (char === "\n") {
        this.line += 1;
        this.column = 1;
      } else {
        this.column += 1;
      }
    }
  }
}

function stripQuotes(raw: string): string {
  return raw.slice(1, -1);
}

export const createCssLexer = (source: string): CssLexer => new CssLexer(source);
