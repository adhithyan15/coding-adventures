// AUTO-GENERATED — DO NOT EDIT.
// Browser bundle of @coding-adventures/spreadsheet-engine + its dependency
// closure (CAS + excel-parser), exposed on window.SpreadsheetEngine.
// Regenerate with: bash demo/visicalc-html/scripts/bundle-engine.sh
"use strict";
var SpreadsheetEngine = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // ../../code/packages/typescript/spreadsheet-engine/src/index.ts
  var index_exports = {};
  __export(index_exports, {
    DependencyGraph: () => DependencyGraph,
    EMPTY: () => EMPTY,
    Workbook: () => Workbook,
    addressKey: () => addressKey,
    bool: () => bool,
    columnToLetters: () => columnToLetters,
    createSpreadsheet: () => createSpreadsheet,
    err: () => err,
    excelCasAdapter: () => excelCasAdapter,
    expandRange: () => expandRange,
    formatValue: () => formatValue,
    isError: () => isError,
    lettersToColumn: () => lettersToColumn,
    normalizeRange: () => normalizeRange,
    num: () => num,
    parseA1: () => parseA1,
    parseRange: () => parseRange,
    printA1: () => printA1,
    text: () => text,
    toBoolean: () => toBoolean,
    toNumber: () => toNumber,
    toText: () => toText
  });

  // ../../code/packages/typescript/spreadsheet-engine/src/address.ts
  var A_CHARCODE = "A".charCodeAt(0);
  function columnToLetters(col) {
    if (col < 0 || !Number.isInteger(col)) {
      throw new RangeError(`column index must be a non-negative integer, got ${col}`);
    }
    let n = col + 1;
    let out = "";
    while (n > 0) {
      const rem = (n - 1) % 26;
      out = String.fromCharCode(A_CHARCODE + rem) + out;
      n = Math.floor((n - 1) / 26);
    }
    return out;
  }
  function lettersToColumn(letters) {
    if (letters.length === 0) {
      throw new SyntaxError("empty column letters");
    }
    let n = 0;
    for (const ch of letters.toUpperCase()) {
      const code = ch.charCodeAt(0) - A_CHARCODE;
      if (code < 0 || code > 25) {
        throw new SyntaxError(`invalid column letter: ${ch}`);
      }
      n = n * 26 + (code + 1);
    }
    return n - 1;
  }
  var A1_RE = /^(\$?)([A-Za-z]+)(\$?)([0-9]+)$/;
  function parseA1(a1) {
    const m = A1_RE.exec(a1.trim());
    if (!m) {
      throw new SyntaxError(`not a valid A1 cell address: ${JSON.stringify(a1)}`);
    }
    const [, dollarCol, letters, dollarRow, digits] = m;
    const row = Number.parseInt(digits, 10) - 1;
    if (row < 0) {
      throw new SyntaxError(`row number must be >= 1 in ${JSON.stringify(a1)}`);
    }
    return {
      col: lettersToColumn(letters),
      row,
      absoluteCol: dollarCol === "$",
      absoluteRow: dollarRow === "$"
    };
  }
  function printA1(addr) {
    const c = (addr.absoluteCol ? "$" : "") + columnToLetters(addr.col);
    const r = (addr.absoluteRow ? "$" : "") + String(addr.row + 1);
    return c + r;
  }
  function addressKey(addr) {
    return `${addr.col},${addr.row}`;
  }
  function parseRange(s) {
    const colon = s.indexOf(":");
    if (colon === -1) {
      const a = parseA1(s);
      return { start: a, end: a };
    }
    const start = parseA1(s.slice(0, colon));
    const end = parseA1(s.slice(colon + 1));
    return normalizeRange({ start, end });
  }
  function normalizeRange(range) {
    const minCol = Math.min(range.start.col, range.end.col);
    const maxCol = Math.max(range.start.col, range.end.col);
    const minRow = Math.min(range.start.row, range.end.row);
    const maxRow = Math.max(range.start.row, range.end.row);
    return {
      start: { col: minCol, row: minRow },
      end: { col: maxCol, row: maxRow }
    };
  }
  var MAX_RANGE_CELLS = 1048576;
  var RangeTooLargeError = class extends Error {
    constructor(cellCount) {
      super(
        `range covers ${cellCount} cells, which exceeds the ${MAX_RANGE_CELLS}-cell safety cap (MAX_RANGE_CELLS)`
      );
      this.cellCount = cellCount;
      this.name = "RangeTooLargeError";
    }
    cellCount;
  };
  function expandRange(range) {
    const { start, end } = normalizeRange(range);
    const count = (end.col - start.col + 1) * (end.row - start.row + 1);
    if (count > MAX_RANGE_CELLS) {
      throw new RangeTooLargeError(count);
    }
    const out = [];
    for (let row = start.row; row <= end.row; row++) {
      for (let col = start.col; col <= end.col; col++) {
        out.push({ col, row });
      }
    }
    return out;
  }

  // ../../code/packages/typescript/spreadsheet-engine/src/cell-value.ts
  var EMPTY = { kind: "empty" };
  function num(value) {
    return { kind: "number", value };
  }
  function text(value) {
    return { kind: "text", value };
  }
  function bool(value) {
    return { kind: "boolean", value };
  }
  function err(code) {
    return { kind: "error", code };
  }
  function isError(v) {
    return v.kind === "error";
  }
  function toNumber(v) {
    switch (v.kind) {
      case "empty":
        return 0;
      case "number":
        return v.value;
      case "boolean":
        return v.value ? 1 : 0;
      case "text": {
        const trimmed = v.value.trim();
        if (trimmed === "") return { kind: "error", code: "#VALUE!" };
        const n = Number(trimmed);
        return Number.isNaN(n) ? { kind: "error", code: "#VALUE!" } : n;
      }
      case "error":
        return v;
    }
  }
  function toText(v) {
    switch (v.kind) {
      case "empty":
        return "";
      case "number":
        return String(v.value);
      case "text":
        return v.value;
      case "boolean":
        return v.value ? "TRUE" : "FALSE";
      case "error":
        return v.code;
    }
  }
  function toBoolean(v) {
    switch (v.kind) {
      case "empty":
        return false;
      case "boolean":
        return v.value;
      case "number":
        return v.value !== 0;
      case "text": {
        const u = v.value.trim().toUpperCase();
        if (u === "TRUE") return true;
        if (u === "FALSE") return false;
        return { kind: "error", code: "#VALUE!" };
      }
      case "error":
        return v;
    }
  }
  function formatValue(v) {
    switch (v.kind) {
      case "empty":
        return "<empty>";
      case "number":
        return String(v.value);
      case "text":
        return JSON.stringify(v.value);
      case "boolean":
        return v.value ? "TRUE" : "FALSE";
      case "error":
        return v.code;
    }
  }

  // ../../code/packages/typescript/spreadsheet-engine/src/dependency-graph.ts
  var DependencyGraph = class {
    /** cell → the set of cells it depends on (reads). Keyed by addressKey. */
    edgesOut = /* @__PURE__ */ new Map();
    /** cell → the set of cells that depend on it (are read by). */
    edgesIn = /* @__PURE__ */ new Map();
    /** Replace the full out-edge set of `cell` with `deps`.
     *
     * Called every time a formula is (re)entered. We first tear down the old
     * edges (so a formula that used to read A1 but no longer does stops being
     * woken up by A1), then add the new ones. Both directions stay consistent. */
    setDependencies(cell, deps) {
      const key = addressKey(cell);
      const old = this.edgesOut.get(key);
      if (old) {
        for (const depKey of old) {
          this.edgesIn.get(depKey)?.delete(key);
        }
      }
      const newOut = /* @__PURE__ */ new Set();
      for (const dep of deps) {
        const depKey = addressKey(dep);
        newOut.add(depKey);
        let inSet = this.edgesIn.get(depKey);
        if (!inSet) {
          inSet = /* @__PURE__ */ new Set();
          this.edgesIn.set(depKey, inSet);
        }
        inSet.add(key);
      }
      this.edgesOut.set(key, newOut);
    }
    /** Remove a cell entirely from the graph (used when a cell is cleared). */
    removeCell(cell) {
      const key = addressKey(cell);
      const out = this.edgesOut.get(key);
      if (out) {
        for (const depKey of out) this.edgesIn.get(depKey)?.delete(key);
      }
      this.edgesOut.delete(key);
    }
    /** The set of cells `cell` directly reads. */
    dependenciesOf(key) {
      return this.edgesOut.get(key) ?? EMPTY_SET;
    }
    /** The set of cells that directly read `cell`. */
    dependentsOf(key) {
      return this.edgesIn.get(key) ?? EMPTY_SET;
    }
    /**
     * Compute the **dirty set**: `seeds` plus every cell transitively downstream
     * of them (i.e. reachable by following edgesIn). This is everything that
     * *might* need recomputing after the seed cells changed.
     *
     * Implemented as a breadth-first walk over the reverse edges.
     */
    dirtySet(seeds) {
      const dirty = /* @__PURE__ */ new Set();
      const queue = [];
      for (const s of seeds) {
        const k = addressKey(s);
        if (!dirty.has(k)) {
          dirty.add(k);
          queue.push(k);
        }
      }
      while (queue.length > 0) {
        const cur = queue.shift();
        for (const dependent of this.dependentsOf(cur)) {
          if (!dirty.has(dependent)) {
            dirty.add(dependent);
            queue.push(dependent);
          }
        }
      }
      return dirty;
    }
    /**
     * Topologically order the subgraph induced by `subset`, considering only
     * edges *within* the subset. Returns:
     *
     *   - `order`: the cells in a valid evaluation order (dependencies first),
     *     and
     *   - `cyclic`: the cells that could not be ordered because they take part in
     *     a cycle (or depend on one).
     *
     * This is **Kahn's algorithm** restricted to the subset. We count, for each
     * cell, how many of its dependencies are *also in the subset* (its in-degree
     * within the subgraph). Cells with in-degree 0 are ready to evaluate; as we
     * "remove" each one we decrement its dependents' counts. Whatever never
     * reaches in-degree 0 is exactly the set of cells tangled in a cycle — those
     * become `#CIRC!`.
     *
     * ### Worked example
     *
     * ```text
     *   subset = {A2, A3}      A2 = A3 + 1,  A3 = 5   (A3 has no in-subset deps)
     *   in-degree:  A3 → 0,  A2 → 1
     *   queue starts [A3] → emit A3, decrement A2 → 0 → queue [A2] → emit A2
     *   order = [A3, A2], cyclic = {}
     * ```
     */
    topoOrderSubset(subset) {
      const inDegree = /* @__PURE__ */ new Map();
      for (const key of subset) {
        let deg = 0;
        for (const dep of this.dependenciesOf(key)) {
          if (subset.has(dep)) deg++;
        }
        inDegree.set(key, deg);
      }
      const queue = [...inDegree.entries()].filter(([, d]) => d === 0).map(([k]) => k).sort();
      const order = [];
      while (queue.length > 0) {
        const cur = queue.shift();
        order.push(cur);
        const readyNow = [];
        for (const dependent of this.dependentsOf(cur)) {
          if (!subset.has(dependent)) continue;
          const d = (inDegree.get(dependent) ?? 0) - 1;
          inDegree.set(dependent, d);
          if (d === 0) readyNow.push(dependent);
        }
        if (readyNow.length > 0) {
          queue.push(...readyNow);
          queue.sort();
        }
      }
      const cyclic = /* @__PURE__ */ new Set();
      for (const key of subset) {
        if (!order.includes(key)) cyclic.add(key);
      }
      return { order, cyclic };
    }
  };
  var EMPTY_SET = /* @__PURE__ */ new Set();

  // ../../code/packages/typescript/spreadsheet-engine/src/workbook.ts
  var Workbook = class {
    adapter;
    mode;
    /** address-key → Cell. The grid itself. */
    cells = /* @__PURE__ */ new Map();
    graph = new DependencyGraph();
    /** Bumped on every recalc pass; stamped onto each cell we (re)evaluate. */
    epoch = 0;
    constructor(options) {
      this.adapter = options.adapter;
      this.mode = options.mode ?? "auto";
    }
    /** Switch recalc mode at runtime. Switching *to* auto does not trigger a
     *  recalc by itself — call `recalcAll()` if you want one. */
    setMode(mode) {
      this.mode = mode;
    }
    // -------------------------------------------------------------------------
    // Editing
    // -------------------------------------------------------------------------
    /**
     * Set the contents of a cell from raw text (`"42"`, `"hello"`, `"=A1+B1"`).
     *
     * Empty string clears the cell. In `auto` mode this triggers an incremental
     * recalc of the cell and everything downstream; in `manual` mode it only
     * records the edit and updates the graph (call `recalcAll()` to compute).
     */
    setCell(a1, raw) {
      const addr = parseA1(a1);
      const key = addressKey(addr);
      if (raw === "") {
        this.cells.delete(key);
        this.graph.removeCell(addr);
        if (this.mode === "auto") this.recalcFrom([addr]);
        return;
      }
      if (this.adapter.isFormula(raw)) {
        const deps = this.adapter.dependencies(raw);
        this.graph.setDependencies(addr, deps);
        this.cells.set(key, {
          kind: "formula",
          raw,
          value: void 0,
          lastEvalEpoch: -1
        });
      } else {
        this.graph.setDependencies(addr, []);
        this.cells.set(key, {
          kind: "literal",
          raw,
          value: parseLiteral(raw)
        });
      }
      if (this.mode === "auto") this.recalcFrom([addr]);
    }
    /** Convenience: set many cells, deferring recalc until all are in. Useful for
     *  bulk-loading a grid without N intermediate recalcs. */
    setCells(entries) {
      const wasAuto = this.mode === "auto";
      this.mode = "manual";
      const touched = [];
      for (const [a1, raw] of Object.entries(entries)) {
        this.setCell(a1, raw);
        touched.push(parseA1(a1));
      }
      if (wasAuto) {
        this.mode = "auto";
        this.recalcFrom(touched);
      }
    }
    // -------------------------------------------------------------------------
    // Reading
    // -------------------------------------------------------------------------
    /** The current value of a cell. Unknown / blank cells read as `{kind:"empty"}`. */
    getValue(a1) {
      return this.valueAt(parseA1(a1));
    }
    /** The raw source text of a cell (`"=A1+B1"` or `"42"`), or `""` if blank. */
    getRaw(a1) {
      return this.cells.get(addressKey(parseA1(a1)))?.raw ?? "";
    }
    /** Snapshot every non-empty cell's value, keyed by canonical A1 string.
     *  Handy for assertions and for rendering the whole grid. */
    getValues() {
      const out = {};
      for (const [key, cell] of this.cells) {
        const [col, row] = key.split(",").map(Number);
        out[printA1({ col, row })] = cell.value ?? EMPTY;
      }
      return out;
    }
    // -------------------------------------------------------------------------
    // Recalc
    // -------------------------------------------------------------------------
    /** Recompute *every* formula in the workbook. Bumps the epoch. Use this after
     *  bulk edits in manual mode, or to force a clean pass. */
    recalcAll() {
      this.recalcFrom([...this.cells.keys()].map(keyToAddress));
    }
    /**
     * The recalc core. Given the seed cells that just changed, compute the dirty
     * set, order it, and evaluate. Cells caught in a cycle become `#CIRC!`.
     */
    recalcFrom(seeds) {
      this.epoch++;
      const dirty = this.graph.dirtySet(seeds);
      const { order, cyclic } = this.graph.topoOrderSubset(dirty);
      for (const key of cyclic) {
        const cell = this.cells.get(key);
        if (cell && cell.kind === "formula") {
          cell.value = err("#CIRC!");
          cell.lastEvalEpoch = this.epoch;
        }
      }
      const resolve = (addr) => this.valueAt(addr);
      for (const key of order) {
        const cell = this.cells.get(key);
        if (!cell || cell.kind !== "formula") continue;
        try {
          cell.value = this.adapter.evaluate(cell.raw, resolve);
        } catch {
          cell.value = err("#VALUE!");
        }
        cell.lastEvalEpoch = this.epoch;
      }
    }
    /** Resolve a cell address to its current value, for the adapter's resolver. */
    valueAt(addr) {
      const cell = this.cells.get(addressKey(addr));
      if (!cell) return EMPTY;
      return cell.value ?? EMPTY;
    }
  };
  function parseLiteral(raw) {
    const trimmed = raw.trim();
    if (trimmed !== "") {
      const n = Number(trimmed);
      if (!Number.isNaN(n)) return num(n);
    }
    return text(raw);
  }
  function keyToAddress(key) {
    const [col, row] = key.split(",").map(Number);
    return { col, row };
  }

  // ../../code/packages/typescript/parser/src/grammar-parser.ts
  function isASTNode(child) {
    return "ruleName" in child;
  }
  var GrammarParseError = class extends Error {
    token;
    constructor(message, token) {
      if (token) {
        super(`Parse error at ${token.line}:${token.column}: ${message}`);
      } else {
        super(`Parse error: ${message}`);
      }
      this.name = "GrammarParseError";
      this.token = token ?? null;
    }
  };
  var GrammarParser = class {
    tokens;
    grammar;
    pos;
    rules;
    /** Index of each rule name for memo key generation. */
    ruleIndex;
    /** Whether newlines are significant in this grammar. */
    newlinesSignificant;
    /** Packrat memoization cache: [ruleIndex, position] -> MemoEntry. */
    memo;
    /** Furthest position reached during parsing. */
    furthestPos;
    /** What was expected at the furthest position. */
    furthestExpected;
    /** Pre-parse hooks: transform token list before parsing. */
    _preParseHooks = [];
    /** Post-parse hooks: transform AST after parsing. */
    _postParseHooks = [];
    /** Whether trace mode is enabled. */
    trace;
    /** Whether AST nodes should retain token-derived source info. */
    preserveSourceInfo;
    constructor(tokens, grammar, options) {
      this.tokens = tokens;
      this.grammar = grammar;
      this.pos = 0;
      this.memo = /* @__PURE__ */ new Map();
      this.furthestPos = 0;
      this.furthestExpected = [];
      this.trace = options?.trace ?? false;
      this.preserveSourceInfo = options?.preserveSourceInfo ?? false;
      const ruleMap = /* @__PURE__ */ new Map();
      const ruleIndex = /* @__PURE__ */ new Map();
      for (let i = 0; i < grammar.rules.length; i++) {
        const rule2 = grammar.rules[i];
        ruleMap.set(rule2.name, rule2);
        ruleIndex.set(rule2.name, i);
      }
      this.rules = ruleMap;
      this.ruleIndex = ruleIndex;
      this.newlinesSignificant = this.grammarReferencesNewline();
    }
    /**
     * Whether newlines are treated as significant tokens in this grammar.
     */
    isNewlinesSignificant() {
      return this.newlinesSignificant;
    }
    /**
     * Register a token transform to run before parsing.
     *
     * The hook receives the token list and returns a (possibly modified)
     * token list. Multiple hooks compose left-to-right.
     */
    addPreParse(hook) {
      this._preParseHooks.push(hook);
    }
    /**
     * Register an AST transform to run after parsing.
     *
     * The hook receives the final AST and returns a (possibly modified)
     * AST. Multiple hooks compose left-to-right.
     */
    addPostParse(hook) {
      this._postParseHooks.push(hook);
    }
    /**
     * Parse the token stream using the first grammar rule as entry point.
     */
    parse() {
      if (this._preParseHooks.length > 0) {
        let mutableTokens = [...this.tokens];
        for (const hook of this._preParseHooks) {
          mutableTokens = hook(mutableTokens);
        }
        this.tokens = mutableTokens;
      }
      if (this.grammar.rules.length === 0) {
        throw new GrammarParseError("Grammar has no rules");
      }
      const entryRule = this.grammar.rules[0];
      const result = this.parseRule(entryRule.name);
      if (result === null) {
        const tok = this.current();
        if (this.furthestExpected.length > 0) {
          const expected = this.furthestExpected.join(" or ");
          const furthestTok = this.furthestPos < this.tokens.length ? this.tokens[this.furthestPos] : tok;
          throw new GrammarParseError(
            `Expected ${expected}, got ${JSON.stringify(furthestTok.value)}`,
            furthestTok
          );
        }
        throw new GrammarParseError("Failed to parse", tok);
      }
      while (this.pos < this.tokens.length && this.current().type === "NEWLINE") {
        this.pos++;
      }
      if (this.pos < this.tokens.length && this.current().type !== "EOF") {
        const tok = this.current();
        if (this.furthestExpected.length > 0 && this.furthestPos > this.pos) {
          const expected = this.furthestExpected.join(" or ");
          const furthestTok = this.furthestPos < this.tokens.length ? this.tokens[this.furthestPos] : tok;
          throw new GrammarParseError(
            `Expected ${expected}, got ${JSON.stringify(furthestTok.value)}`,
            furthestTok
          );
        }
        throw new GrammarParseError(
          `Unexpected token: ${JSON.stringify(tok.value)}`,
          tok
        );
      }
      let ast = result;
      for (const hook of this._postParseHooks) {
        ast = hook(ast);
      }
      return ast;
    }
    // =========================================================================
    // HELPERS
    // =========================================================================
    current() {
      if (this.pos < this.tokens.length) {
        return this.tokens[this.pos];
      }
      return this.tokens[this.tokens.length - 1];
    }
    recordFailure(expected) {
      if (this.pos > this.furthestPos) {
        this.furthestPos = this.pos;
        this.furthestExpected = [expected];
      } else if (this.pos === this.furthestPos) {
        if (!this.furthestExpected.includes(expected)) {
          this.furthestExpected.push(expected);
        }
      }
    }
    // =========================================================================
    // NEWLINE DETECTION
    // =========================================================================
    grammarReferencesNewline() {
      for (const rule2 of this.grammar.rules) {
        if (this.elementReferencesNewline(rule2.body)) {
          return true;
        }
      }
      return false;
    }
    elementReferencesNewline(element) {
      switch (element.type) {
        case "token_reference":
          return element.name === "NEWLINE";
        case "sequence":
          return element.elements.some((e) => this.elementReferencesNewline(e));
        case "alternation":
          return element.choices.some((c) => this.elementReferencesNewline(c));
        case "repetition":
        case "optional":
        case "group":
        case "positive_lookahead":
        case "negative_lookahead":
        case "one_or_more":
          return this.elementReferencesNewline(element.element);
        case "separated_repetition":
          return this.elementReferencesNewline(element.element) || this.elementReferencesNewline(element.separator);
        default:
          return false;
      }
    }
    // =========================================================================
    // RULE PARSING (with packrat memoization)
    // =========================================================================
    parseRule(ruleName) {
      const rule2 = this.rules.get(ruleName);
      if (!rule2) {
        return null;
      }
      const idx = this.ruleIndex.get(ruleName);
      if (idx !== void 0) {
        const key = `${idx},${this.pos}`;
        const cached = this.memo.get(key);
        if (cached !== void 0) {
          this.pos = cached.endPos;
          if (!cached.ok) {
            return null;
          }
          return this.buildNode(
            ruleName,
            cached.children
          );
        }
      }
      const startPos = this.pos;
      if (idx !== void 0) {
        const key = `${idx},${startPos}`;
        this.memo.set(key, { children: null, endPos: startPos, ok: false });
      }
      if (this.trace) {
        const tok = this.current();
        process.stderr.write(
          `[TRACE] rule '${ruleName}' at token ${startPos} (${tok.type} "${tok.value}") \u2192 `
        );
      }
      let children = this.matchElement(rule2.body);
      if (this.trace) {
        process.stderr.write(children !== null ? "match\n" : "fail\n");
      }
      if (idx !== void 0) {
        const key = `${idx},${startPos}`;
        if (children !== null) {
          this.memo.set(key, { children, endPos: this.pos, ok: true });
        } else {
          this.memo.set(key, { children: null, endPos: this.pos, ok: false });
        }
        if (children !== null) {
          for (; ; ) {
            const prevEnd = this.pos;
            this.pos = startPos;
            this.memo.set(key, { children, endPos: prevEnd, ok: true });
            const newChildren = this.matchElement(rule2.body);
            if (newChildren === null || this.pos <= prevEnd) {
              this.pos = prevEnd;
              this.memo.set(key, { children, endPos: prevEnd, ok: true });
              break;
            }
            children = newChildren;
          }
        }
      }
      if (children === null) {
        this.pos = startPos;
        this.recordFailure(ruleName);
        return null;
      }
      return this.buildNode(ruleName, children);
    }
    // =========================================================================
    // ELEMENT MATCHING
    // =========================================================================
    matchElement(element) {
      const savePos = this.pos;
      switch (element.type) {
        case "sequence": {
          const children = [];
          for (const sub of element.elements) {
            const result = this.matchElement(sub);
            if (result === null) {
              this.pos = savePos;
              return null;
            }
            children.push(...result);
          }
          return children;
        }
        case "alternation": {
          for (const choice of element.choices) {
            this.pos = savePos;
            const result = this.matchElement(choice);
            if (result !== null) {
              return result;
            }
          }
          this.pos = savePos;
          return null;
        }
        case "repetition": {
          const children = [];
          while (true) {
            const saveRep = this.pos;
            const result = this.matchElement(element.element);
            if (result === null) {
              this.pos = saveRep;
              break;
            }
            children.push(...result);
          }
          return children;
        }
        case "optional": {
          const result = this.matchElement(element.element);
          if (result === null) {
            return [];
          }
          return result;
        }
        case "group":
          return this.matchElement(element.element);
        case "token_reference":
          return this.matchTokenReference(element.name);
        case "rule_reference": {
          const node = this.parseRule(element.name);
          if (node !== null) {
            return [node];
          }
          this.pos = savePos;
          return null;
        }
        case "literal": {
          let token = this.current();
          if (!this.newlinesSignificant) {
            while (token.type === "NEWLINE") {
              this.pos++;
              token = this.current();
            }
          }
          if (token.value === element.value) {
            this.pos++;
            return [token];
          }
          this.recordFailure(`"${element.value}"`);
          return null;
        }
        // ---------------------------------------------------------------
        // Extension: Syntactic predicates (lookahead without consuming)
        // ---------------------------------------------------------------
        case "positive_lookahead": {
          const result = this.matchElement(element.element);
          this.pos = savePos;
          return result !== null ? [] : null;
        }
        case "negative_lookahead": {
          const result = this.matchElement(element.element);
          this.pos = savePos;
          return result === null ? [] : null;
        }
        // ---------------------------------------------------------------
        // Extension: One-or-more repetition
        // ---------------------------------------------------------------
        case "one_or_more": {
          const first = this.matchElement(element.element);
          if (first === null) {
            this.pos = savePos;
            return null;
          }
          const children = [...first];
          while (true) {
            const saveRep = this.pos;
            const result = this.matchElement(element.element);
            if (result === null) {
              this.pos = saveRep;
              break;
            }
            children.push(...result);
          }
          return children;
        }
        // ---------------------------------------------------------------
        // Extension: Separated repetition
        // ---------------------------------------------------------------
        case "separated_repetition": {
          const first = this.matchElement(element.element);
          if (first === null) {
            this.pos = savePos;
            if (element.atLeastOne) return null;
            return [];
          }
          const children = [...first];
          while (true) {
            const saveSep = this.pos;
            const sep = this.matchElement(element.separator);
            if (sep === null) {
              this.pos = saveSep;
              break;
            }
            const next = this.matchElement(element.element);
            if (next === null) {
              this.pos = saveSep;
              break;
            }
            children.push(...sep, ...next);
          }
          return children;
        }
        default:
          return null;
      }
    }
    // =========================================================================
    // TOKEN REFERENCE MATCHING
    // =========================================================================
    matchTokenReference(expectedType) {
      let token = this.current();
      if (!this.newlinesSignificant && expectedType !== "NEWLINE") {
        while (token.type === "NEWLINE") {
          this.pos++;
          token = this.current();
        }
      }
      if (token.type === expectedType) {
        this.pos++;
        return [token];
      }
      this.recordFailure(expectedType);
      return null;
    }
    buildNode(ruleName, children) {
      const pos = computeNodePosition(children);
      const sourceInfo = this.preserveSourceInfo ? computeNodeSourceInfo(children) : null;
      return {
        ruleName,
        children,
        ...pos ?? {},
        ...sourceInfo ?? {}
      };
    }
  };
  function computeNodePosition(children) {
    const first = findFirstToken(children);
    const last = findLastToken(children);
    if (!first || !last) return null;
    return {
      startLine: first.line,
      startColumn: first.column,
      endLine: last.line,
      endColumn: last.column
    };
  }
  function computeNodeSourceInfo(children) {
    const first = findFirstToken(children);
    const last = findLastToken(children);
    if (!first || !last) {
      return null;
    }
    const info = {};
    if (first.startOffset !== void 0) {
      info.startOffset = first.startOffset;
    }
    if (last.endOffset !== void 0) {
      info.endOffset = last.endOffset;
    }
    if (first.tokenIndex !== void 0) {
      info.firstTokenIndex = first.tokenIndex;
    }
    if (last.tokenIndex !== void 0) {
      info.lastTokenIndex = last.tokenIndex;
    }
    if (first.leadingTrivia !== void 0) {
      info.leadingTrivia = first.leadingTrivia;
    }
    return info;
  }
  function findFirstToken(children) {
    for (const child of children) {
      if (isASTNode(child)) {
        const tok = findFirstToken(child.children);
        if (tok) return tok;
      } else {
        return child;
      }
    }
    return null;
  }
  function findLastToken(children) {
    for (let i = children.length - 1; i >= 0; i--) {
      const child = children[i];
      if (isASTNode(child)) {
        const tok = findLastToken(child.children);
        if (tok) return tok;
      } else {
        return child;
      }
    }
    return null;
  }

  // ../../code/packages/typescript/lexer/src/token.ts
  var TOKEN_CONTEXT_KEYWORD = 2;

  // ../../code/packages/typescript/lexer/src/tokenizer.ts
  var LexerError = class extends Error {
    line;
    column;
    constructor(message, line, column) {
      super(`Lexer error at ${line}:${column}: ${message}`);
      this.name = "LexerError";
      this.line = line;
      this.column = column;
    }
  };

  // ../../code/packages/typescript/lexer/src/grammar-lexer.ts
  function escapeRegExp(s) {
    return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }
  function lowerScopedCaseInsensitiveGroups(pattern) {
    return pattern.replace(
      /\(\?i:([^()]+)\)/g,
      (_match, body) => body.replace(/[A-Za-z]/g, (ch) => `[${ch.toLowerCase()}${ch.toUpperCase()}]`)
    );
  }
  function compileGrammarRegExp(patternSource, flags) {
    return new RegExp(lowerScopedCaseInsensitiveGroups(patternSource), flags);
  }
  function resolveTokenType(tokenName, value, keywordSet, reservedSet, alias, line, column) {
    if (tokenName === "NAME" && reservedSet.has(value)) {
      throw new LexerError(
        `Reserved keyword '${value}' cannot be used as an identifier`,
        line,
        column
      );
    }
    if (tokenName === "NAME" && keywordSet.has(value)) {
      return "KEYWORD";
    }
    if (alias) {
      return alias;
    }
    return tokenName;
  }
  function processEscapes(s) {
    const result = [];
    let i = 0;
    while (i < s.length) {
      if (s[i] === "\\" && i + 1 < s.length) {
        const escapeMap = {
          n: "\n",
          t: "	",
          "\\": "\\",
          '"': '"'
        };
        const nextChar = s[i + 1];
        result.push(escapeMap[nextChar] ?? nextChar);
        i += 2;
      } else {
        result.push(s[i]);
        i += 1;
      }
    }
    return result.join("");
  }
  var LexerContext = class {
    /** @internal Reference to the lexer (for reading group stack state). */
    _lexer;
    /** @internal The full source string being tokenized. */
    _source;
    /** @internal Position in the source immediately after the current token. */
    _posAfter;
    /** @internal Whether the current token should be suppressed from output. */
    _suppressed = false;
    /** @internal Synthetic tokens to inject after the current one. */
    _emitted = [];
    /** @internal Group stack actions recorded by the callback: ("push", name) or ("pop", ""). */
    _groupActions = [];
    /** @internal New skip-enabled state, or null if unchanged. */
    _skipEnabled = null;
    /** @internal The most recently emitted token (for lookbehind). */
    _previousToken;
    /** @internal The current token's line number (for newline detection). */
    _currentTokenLine;
    constructor(lexer, source, posAfterToken, previousToken, currentTokenLine) {
      this._lexer = lexer;
      this._source = source;
      this._posAfter = posAfterToken;
      this._previousToken = previousToken;
      this._currentTokenLine = currentTokenLine;
    }
    /**
     * Push a pattern group onto the stack.
     *
     * The pushed group becomes active for the **next** token match.
     * Throws an Error if the group name is not defined in the grammar.
     *
     * Multiple pushes in a single callback are applied in order, so you
     * can stack multiple groups if needed (though this is rare).
     */
    pushGroup(groupName) {
      if (!this._lexer.hasGroup(groupName)) {
        throw new Error(
          `Unknown pattern group: '${groupName}'. Available groups: ${this._lexer.availableGroups().sort().join(", ")}`
        );
      }
      this._groupActions.push(["push", groupName]);
    }
    /**
     * Pop the current group from the stack.
     *
     * If only the "default" group remains (stack depth = 1), this is a
     * no-op. The default group is the floor and cannot be popped — this
     * prevents accidental stack underflow in recursive structures.
     */
    popGroup() {
      this._groupActions.push(["pop", ""]);
    }
    /**
     * Return the name of the currently active group.
     *
     * The active group is the top of the group stack. When no groups
     * have been pushed, this is always "default".
     */
    activeGroup() {
      return this._lexer.activeGroup();
    }
    /**
     * Return the depth of the group stack (always >= 1).
     *
     * A depth of 1 means only the "default" group is on the stack.
     * A depth of 2 means one group has been pushed on top of default.
     */
    groupStackDepth() {
      return this._lexer.groupStackDepth();
    }
    /**
     * Inject a synthetic token after the current one.
     *
     * Emitted tokens do NOT trigger the callback (this prevents infinite
     * loops — a callback that emits tokens which trigger the callback
     * which emits more tokens...). Multiple `emit()` calls produce
     * tokens in call order.
     */
    emit(token) {
      this._emitted.push(token);
    }
    /**
     * Suppress the current token — do not include it in output.
     *
     * Combined with `emit()`, this enables **token replacement**: suppress
     * the original token and emit a modified version in its place.
     */
    suppress() {
      this._suppressed = true;
    }
    /**
     * Peek at a source character past the current token.
     *
     * This provides lookahead capability without advancing the lexer's
     * position. Useful for making group-switching decisions based on
     * what comes next in the source.
     *
     * @param offset - Number of characters ahead (1 = immediately after token).
     * @returns The character, or empty string if past EOF.
     */
    peek(offset = 1) {
      const idx = this._posAfter + offset - 1;
      if (idx >= 0 && idx < this._source.length) {
        return this._source[idx];
      }
      return "";
    }
    /**
     * Peek at the next `length` characters past the current token.
     *
     * Returns a substring starting immediately after the current token.
     * If fewer than `length` characters remain, returns whatever is left.
     */
    peekStr(length) {
      return this._source.slice(this._posAfter, this._posAfter + length);
    }
    /**
     * Toggle skip pattern processing.
     *
     * When disabled, skip patterns (whitespace, comments) are not tried.
     * This is useful for groups where whitespace is significant — for
     * example, CDATA sections in XML where spaces must be preserved
     * as part of the content rather than being silently consumed.
     */
    setSkipEnabled(enabled) {
      this._skipEnabled = enabled;
    }
    // -----------------------------------------------------------------------
    // Extension: Token Lookbehind
    // -----------------------------------------------------------------------
    /**
     * Return the most recently emitted token, or null at the start of input.
     *
     * "Emitted" means the token actually made it into the output list —
     * suppressed tokens are not counted. This provides **lookbehind**
     * capability for context-sensitive decisions.
     *
     * For example, in JavaScript `/` is a regex literal after `=`, `(`
     * or `,` but a division operator after `)`, `]`, identifiers, or
     * numbers. The callback can check `ctx.previousToken()?.type` to
     * decide which interpretation to use.
     *
     * @returns The last token in the output list, or null if no tokens
     *          have been emitted yet.
     */
    previousToken() {
      return this._previousToken;
    }
    // -----------------------------------------------------------------------
    // Extension: Bracket Depth Tracking
    // -----------------------------------------------------------------------
    /**
     * Return the current nesting depth for a specific bracket type,
     * or the total depth across all types if no argument is given.
     *
     * Depth starts at 0 and increments on each opener (`(`, `[`, `{`),
     * decrements on each closer (`)`, `]`, `}`). The count never goes
     * below 0 — unmatched closers are clamped.
     *
     * This is essential for template literal interpolation in languages
     * like JavaScript, Kotlin, and Ruby, where `}` at brace-depth 0
     * closes the interpolation rather than being part of a nested
     * expression.
     *
     * @param kind - Optional bracket type to query. If omitted, returns
     *               the sum of all three depths.
     */
    bracketDepth(kind) {
      return this._lexer.bracketDepth(kind);
    }
    // -----------------------------------------------------------------------
    // Extension: Newline Detection
    // -----------------------------------------------------------------------
    /**
     * Return true if a newline appeared between the previous token
     * and the current token (i.e., they are on different lines).
     *
     * This is used by languages with automatic semicolon insertion
     * (JavaScript, Go) to detect line breaks that trigger implicit
     * statement termination. The lexer exposes this as a convenience
     * so callbacks and post-tokenize hooks can set the
     * `TOKEN_PRECEDED_BY_NEWLINE` flag on tokens that need it.
     *
     * Returns false if there is no previous token (start of input).
     */
    precededByNewline() {
      if (this._previousToken === null) return false;
      return this._previousToken.line < this._currentTokenLine;
    }
  };
  var GrammarLexer = class {
    // -- Source and position tracking --
    /** The complete source code string being tokenized. */
    _source;
    /** Current position (index) in the source string. */
    _pos = 0;
    /** Current line number (1-based), for error reporting. */
    _line = 1;
    /** Current column number (1-based), for error reporting. */
    _column = 1;
    // -- Grammar metadata --
    /** The TokenGrammar that defines which tokens to recognize. */
    _grammar;
    /** Pre-computed set of keywords for O(1) lookup. */
    _keywordSet;
    /** Reserved keywords that cause lex errors. */
    _reservedSet;
    /** Whether the grammar has skip patterns defined. */
    _hasSkipPatterns;
    /** Whether indentation mode is active. */
    _indentationMode;
    /** Whether Haskell-style layout mode is active. */
    _layoutMode;
    /**
     * Whether token matching is case-sensitive.
     *
     * When true (the default), the source is matched as-is. When false,
     * the source is lowercased before matching so that patterns written
     * in lowercase will match input regardless of case.
     */
    _caseSensitive;
    /**
     * Whether keyword matching is case-insensitive (from `grammar.caseInsensitive`).
     *
     * When true, NAME tokens are checked against the keyword set using their
     * uppercased form, and keyword tokens are emitted with their value normalized
     * to uppercase. Non-keyword identifiers retain their original casing.
     */
    _caseInsensitive;
    // -- Compiled patterns --
    /** Default group compiled patterns, in priority order. */
    _patterns;
    /** Compiled skip patterns (comments, whitespace). */
    _skipPatterns;
    /** Compiled patterns per group. "default" + named groups. */
    _groupPatterns;
    /** Maps definition names to their aliases (e.g., STRING_DQ -> STRING). */
    _aliasMap;
    // -- Group stack and callback --
    /**
     * The group stack. Bottom is always "default". Top is the active
     * group whose patterns are tried during token matching.
     */
    _groupStack = ["default"];
    /**
     * On-token callback — null means no callback (zero overhead).
     * When set, fires after each token match with a LexerContext.
     */
    _onToken = null;
    /**
     * Skip enabled flag — can be toggled by callbacks for groups
     * where whitespace is significant (e.g., CDATA, raw text).
     */
    _skipEnabled = true;
    // -- Extension: Token lookbehind --
    /**
     * The most recently emitted token, for lookbehind in callbacks.
     * Updated after each token push (including callback-emitted tokens).
     * Reset to null on each tokenize() call.
     */
    _lastEmittedToken = null;
    // -- Extension: Bracket depth tracking --
    /**
     * Per-type bracket nesting depth counters.
     *
     * Tracks `()`, `[]`, and `{}` independently. Updated after each
     * token match in both standard and indentation modes. Exposed to
     * callbacks via `LexerContext.bracketDepth()`.
     *
     * This enables context-sensitive lexing for template literals,
     * string interpolation, and other constructs where bracket nesting
     * determines how to tokenize subsequent input.
     */
    _bracketDepths = { paren: 0, bracket: 0, brace: 0 };
    // -- Extension: Context keywords --
    /**
     * Pre-computed set of context-sensitive keywords for O(1) lookup.
     * Words in this set are emitted as NAME with TOKEN_CONTEXT_KEYWORD flag.
     */
    _contextKeywordSet;
    /** Layout introducer keywords used when layout mode is active. */
    _layoutKeywordSet;
    /** Pre-tokenize hooks: transform source text before lexing. */
    _preTokenizeHooks = [];
    /** Post-tokenize hooks: transform token list after lexing. */
    _postTokenizeHooks = [];
    /** Whether token/trivia source metadata should be preserved. */
    _preserveSourceInfo;
    /** Trivia collected since the previous emitted token. */
    _pendingTrivia = [];
    /** Sequential token index assigned in emission order. */
    _nextTokenIndex = 0;
    constructor(source, grammar, options) {
      this._grammar = grammar;
      this._preserveSourceInfo = options?.preserveSourceInfo === true;
      this._caseInsensitive = grammar.caseInsensitive === true;
      this._caseSensitive = grammar.caseSensitive !== false && !this._caseInsensitive;
      this._source = !this._caseSensitive && !this._caseInsensitive ? source.toLowerCase() : source;
      this._keywordSet = new Set(
        this._caseInsensitive ? grammar.keywords.map((k) => k.toUpperCase()) : grammar.keywords
      );
      this._reservedSet = new Set(grammar.reservedKeywords ?? []);
      this._contextKeywordSet = new Set(grammar.contextKeywords ?? []);
      this._indentationMode = grammar.mode === "indentation";
      this._layoutMode = grammar.mode === "layout";
      this._layoutKeywordSet = new Set(grammar.layoutKeywords ?? []);
      this._hasSkipPatterns = (grammar.skipDefinitions ?? []).length > 0;
      this._aliasMap = {};
      for (const defn of grammar.definitions) {
        if (defn.alias) {
          this._aliasMap[defn.name] = defn.alias;
        }
      }
      const reFlags = this._caseInsensitive ? "i" : "";
      this._patterns = grammar.definitions.map((defn) => {
        const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
        return {
          name: defn.name,
          pattern: compileGrammarRegExp(patternSource, reFlags),
          alias: defn.alias
        };
      });
      this._skipPatterns = (grammar.skipDefinitions ?? []).map((defn) => {
        const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
        return {
          name: defn.name,
          pattern: compileGrammarRegExp(patternSource, reFlags)
        };
      });
      this._groupPatterns = {
        default: [...this._patterns]
      };
      if (grammar.groups) {
        for (const [groupName, group] of Object.entries(grammar.groups)) {
          const compiled = group.definitions.map((defn) => {
            const patternSource = defn.isRegex ? defn.pattern : escapeRegExp(defn.pattern);
            if (defn.alias) {
              this._aliasMap[defn.name] = defn.alias;
            }
            return {
              name: defn.name,
              pattern: compileGrammarRegExp(patternSource, reFlags),
              alias: defn.alias
            };
          });
          this._groupPatterns[groupName] = compiled;
        }
      }
    }
    // -- Public API: callback registration --
    /**
     * Register a callback that fires on every token match.
     *
     * The callback receives the matched token and a `LexerContext`. It can
     * use the context to push/pop groups, emit extra tokens, suppress the
     * current token, or toggle skip processing.
     *
     * Only one callback can be registered at a time. Pass `null` to clear.
     *
     * The callback is NOT invoked for:
     * - Skip pattern matches (they produce no tokens)
     * - Tokens emitted via `ctx.emit()` (prevents infinite loops)
     * - The EOF token
     */
    setOnToken(callback) {
      this._onToken = callback;
    }
    // -- Public API: group introspection (used by LexerContext) --
    /** Check whether a group name is defined in the grammar. */
    hasGroup(groupName) {
      return groupName in this._groupPatterns;
    }
    /** Return all available group names. */
    availableGroups() {
      return Object.keys(this._groupPatterns);
    }
    /** Return the name of the currently active group (top of stack). */
    activeGroup() {
      return this._groupStack[this._groupStack.length - 1];
    }
    /** Return the depth of the group stack (always >= 1). */
    groupStackDepth() {
      return this._groupStack.length;
    }
    // -- Extension: Bracket depth --
    /**
     * Return the current nesting depth for a specific bracket type,
     * or the total depth across all types if no argument is given.
     *
     * This is the public API used by LexerContext to expose bracket
     * depth to callbacks. Language packages use this for template
     * literal interpolation and similar nested constructs.
     */
    bracketDepth(kind) {
      if (kind === void 0) {
        return this._bracketDepths.paren + this._bracketDepths.bracket + this._bracketDepths.brace;
      }
      return this._bracketDepths[kind];
    }
    // -- Hook registration --
    /**
     * Register a text transform to run before tokenization.
     *
     * The hook receives the raw source string and returns a (possibly
     * modified) source string. Multiple hooks compose left-to-right.
     */
    addPreTokenize(hook) {
      this._preTokenizeHooks.push(hook);
    }
    /**
     * Register a token transform to run after tokenization.
     *
     * The hook receives the full token list (including EOF) and returns
     * a (possibly modified) token list. Multiple hooks compose left-to-right.
     */
    addPostTokenize(hook) {
      this._postTokenizeHooks.push(hook);
    }
    // -- Main tokenization entry point --
    /**
     * Tokenize the source code using the grammar's token definitions.
     *
     * Dispatches to the appropriate tokenization method based on whether
     * indentation mode is active. Resets the group stack and skip flag
     * at the end so the lexer can be reused for multiple `tokenize()` calls.
     *
     * Pre-tokenize hooks transform the source text before lexing begins.
     * Post-tokenize hooks transform the token list after lexing completes.
     *
     * @returns A list of Token objects, always ending with an EOF token.
     * @throws LexerError if an unexpected character is encountered, a
     *         reserved keyword is used, or indentation is inconsistent.
     */
    tokenize() {
      if (this._preTokenizeHooks.length > 0) {
        let source = this._source;
        for (const hook of this._preTokenizeHooks) {
          source = hook(source);
        }
        this._source = source;
      }
      this._lastEmittedToken = null;
      this._bracketDepths = { paren: 0, bracket: 0, brace: 0 };
      this._pendingTrivia = [];
      this._nextTokenIndex = 0;
      let tokens;
      if (this._indentationMode) {
        tokens = this._tokenizeIndentation();
      } else if (this._layoutMode) {
        tokens = this._tokenizeLayout();
      } else {
        tokens = this._tokenizeStandard();
      }
      for (const hook of this._postTokenizeHooks) {
        tokens = hook(tokens);
      }
      return tokens;
    }
    // -- Standard (non-indentation) tokenization --
    /**
     * Tokenize without indentation tracking.
     *
     * The algorithm:
     *
     * 1. While there are characters left:
     *    a. If skip patterns exist and skip is enabled, try them.
     *    b. If no skip patterns, use default whitespace skip.
     *    c. If the current character is a newline, emit NEWLINE.
     *    d. Try active group's token patterns (first match wins).
     *    e. If callback registered, invoke it and process actions.
     *    f. If nothing matches, raise LexerError.
     * 2. Append EOF.
     *
     * When pattern groups are active, the lexer uses `_groupStack[-1]`
     * to determine which set of patterns to try. When a callback is
     * registered via `setOnToken()`, it fires after each token match
     * and can push/pop groups, emit extra tokens, or suppress the
     * current token.
     */
    _tokenizeStandard() {
      const tokens = [];
      while (this._pos < this._source.length) {
        const char = this._source[this._pos];
        if (this._hasSkipPatterns) {
          if (this._skipEnabled && this._trySkip()) {
            continue;
          }
        } else {
          if (char === " " || char === "	" || char === "\r") {
            this._consumeDefaultWhitespace();
            continue;
          }
        }
        if (char === "\n") {
          const newlineTok = {
            type: "NEWLINE",
            value: "\\n",
            line: this._line,
            column: this._column
          };
          const startOffset = this._pos;
          this._advance();
          this._emitToken(tokens, this._withOptionalSourceInfo(newlineTok, startOffset));
          continue;
        }
        const activeGroupName = this._groupStack[this._groupStack.length - 1];
        const token = this._tryMatchTokenInGroup(activeGroupName);
        if (token !== null) {
          this._updateBracketDepth(token.value);
          if (this._onToken !== null) {
            const ctx = new LexerContext(
              this,
              this._source,
              this._pos,
              this._lastEmittedToken,
              token.line
            );
            this._onToken(token, ctx);
            if (!ctx._suppressed) {
              this._emitToken(tokens, token);
            }
            for (const emitted of ctx._emitted) {
              this._emitToken(tokens, emitted);
            }
            for (const [action, groupName] of ctx._groupActions) {
              if (action === "push") {
                this._groupStack.push(groupName);
              } else if (action === "pop" && this._groupStack.length > 1) {
                this._groupStack.pop();
              }
            }
            if (ctx._skipEnabled !== null) {
              this._skipEnabled = ctx._skipEnabled;
            }
          } else {
            this._emitToken(tokens, token);
          }
          continue;
        }
        throw new LexerError(
          `Unexpected character: ${JSON.stringify(char)}`,
          this._line,
          this._column
        );
      }
      const eof = {
        type: "EOF",
        value: "",
        line: this._line,
        column: this._column
      };
      this._emitToken(tokens, this._withOptionalSourceInfo(eof, this._pos));
      this._groupStack = ["default"];
      this._skipEnabled = true;
      return tokens;
    }
    // -- Extension: Bracket depth tracking helper --
    /**
     * Update bracket depth counters based on a token's value.
     *
     * Called after each token match in both standard and indentation modes.
     * Only single-character values are checked — multi-character tokens
     * cannot be brackets.
     */
    _updateBracketDepth(value) {
      if (value.length !== 1) return;
      switch (value) {
        case "(":
          this._bracketDepths.paren++;
          break;
        case ")":
          if (this._bracketDepths.paren > 0) this._bracketDepths.paren--;
          break;
        case "[":
          this._bracketDepths.bracket++;
          break;
        case "]":
          if (this._bracketDepths.bracket > 0) this._bracketDepths.bracket--;
          break;
        case "{":
          this._bracketDepths.brace++;
          break;
        case "}":
          if (this._bracketDepths.brace > 0) this._bracketDepths.brace--;
          break;
      }
    }
    // -- Indentation mode tokenization --
    /**
     * Tokenize with Python-style indentation tracking.
     *
     * This method implements the full indentation algorithm: it maintains
     * an indent stack, tracks bracket depth for implicit line joining,
     * and emits synthetic INDENT/DEDENT/NEWLINE tokens.
     */
    _tokenizeIndentation() {
      const tokens = [];
      const indentStack = [0];
      let bracketDepth = 0;
      let atLineStart = true;
      while (this._pos < this._source.length) {
        if (atLineStart && bracketDepth === 0) {
          const result = this._processLineStart(indentStack);
          if (result === "skip") {
            continue;
          }
          for (const token of result) {
            this._emitToken(tokens, token);
          }
          atLineStart = false;
          if (this._pos >= this._source.length) {
            break;
          }
        }
        const char = this._source[this._pos];
        if (char === "\n") {
          if (bracketDepth === 0) {
            const newlineTok = {
              type: "NEWLINE",
              value: "\\n",
              line: this._line,
              column: this._column
            };
            const startOffset = this._pos;
            this._advance();
            this._emitToken(tokens, this._withOptionalSourceInfo(newlineTok, startOffset));
          } else {
            this._advance();
          }
          atLineStart = true;
          continue;
        }
        if (bracketDepth > 0 && (char === " " || char === "	" || char === "\r")) {
          this._consumeDefaultWhitespace();
          continue;
        }
        if (this._trySkip()) {
          continue;
        }
        const tok = this._tryMatchTokenInGroup("default");
        if (tok !== null) {
          if (tok.value === "(" || tok.value === "[" || tok.value === "{") {
            bracketDepth++;
          } else if (tok.value === ")" || tok.value === "]" || tok.value === "}") {
            bracketDepth--;
          }
          this._updateBracketDepth(tok.value);
          this._emitToken(tokens, tok);
          continue;
        }
        throw new LexerError(
          `Unexpected character: ${JSON.stringify(char)}`,
          this._line,
          this._column
        );
      }
      while (indentStack.length > 1) {
        indentStack.pop();
        this._emitToken(tokens, this._withOptionalSourceInfo({
          type: "DEDENT",
          value: "",
          line: this._line,
          column: this._column
        }, this._pos));
      }
      if (tokens.length === 0 || tokens[tokens.length - 1].type !== "NEWLINE") {
        this._emitToken(tokens, this._withOptionalSourceInfo({
          type: "NEWLINE",
          value: "\\n",
          line: this._line,
          column: this._column
        }, this._pos));
      }
      this._emitToken(tokens, this._withOptionalSourceInfo({
        type: "EOF",
        value: "",
        line: this._line,
        column: this._column
      }, this._pos));
      this._groupStack = ["default"];
      this._skipEnabled = true;
      return tokens;
    }
    _tokenizeLayout() {
      return this._applyLayout(this._tokenizeStandard());
    }
    _applyLayout(tokens) {
      const result = [];
      const layoutStack = [];
      let pendingLayouts = 0;
      let suppressDepth = 0;
      for (let index = 0; index < tokens.length; index++) {
        const token = tokens[index];
        const typeName = token.typeName ?? token.type;
        if (typeName === "NEWLINE") {
          result.push(token);
          const nextToken = this._nextLayoutToken(tokens, index + 1);
          if (suppressDepth === 0 && nextToken !== null) {
            while (layoutStack.length > 0 && nextToken.column < layoutStack[layoutStack.length - 1]) {
              result.push(this._virtualLayoutToken("VIRTUAL_RBRACE", "}", nextToken));
              layoutStack.pop();
            }
            if (layoutStack.length > 0 && (nextToken.typeName ?? nextToken.type) !== "EOF" && nextToken.value !== "}" && nextToken.column === layoutStack[layoutStack.length - 1]) {
              result.push(this._virtualLayoutToken("VIRTUAL_SEMICOLON", ";", nextToken));
            }
          }
          continue;
        }
        if (typeName === "EOF") {
          while (layoutStack.length > 0) {
            result.push(this._virtualLayoutToken("VIRTUAL_RBRACE", "}", token));
            layoutStack.pop();
          }
          result.push(token);
          continue;
        }
        if (pendingLayouts > 0) {
          if (token.value === "{") {
            pendingLayouts -= 1;
          } else {
            for (let count = 0; count < pendingLayouts; count++) {
              layoutStack.push(token.column);
              result.push(this._virtualLayoutToken("VIRTUAL_LBRACE", "{", token));
            }
            pendingLayouts = 0;
          }
        }
        result.push(token);
        if (!this._isVirtualLayoutToken(token)) {
          if (token.value === "(" || token.value === "[" || token.value === "{") {
            suppressDepth += 1;
          } else if ((token.value === ")" || token.value === "]" || token.value === "}") && suppressDepth > 0) {
            suppressDepth -= 1;
          }
        }
        if (this._isLayoutKeyword(token)) {
          pendingLayouts += 1;
        }
      }
      return result;
    }
    _nextLayoutToken(tokens, startIndex) {
      for (let index = startIndex; index < tokens.length; index++) {
        const token = tokens[index];
        if ((token.typeName ?? token.type) !== "NEWLINE") {
          return token;
        }
      }
      return null;
    }
    _virtualLayoutToken(typeName, value, anchor) {
      return this._withOptionalSourceInfo({
        type: typeName,
        typeName,
        value,
        line: anchor.line,
        column: anchor.column
      }, anchor.startOffset ?? this._pos);
    }
    _isVirtualLayoutToken(token) {
      return (token.typeName ?? token.type).startsWith("VIRTUAL_");
    }
    _isLayoutKeyword(token) {
      if (this._layoutKeywordSet.size === 0) {
        return false;
      }
      const value = token.value ?? "";
      return this._layoutKeywordSet.has(value) || this._layoutKeywordSet.has(value.toLowerCase());
    }
    /**
     * Process indentation at the start of a logical line.
     *
     * Returns "skip" if the line should be skipped (blank/comment),
     * or an array of INDENT/DEDENT tokens.
     */
    _processLineStart(indentStack) {
      let indent = 0;
      const indentStartLine = this._line;
      const indentStartColumn = this._column;
      const indentStartOffset = this._pos;
      while (this._pos < this._source.length) {
        const char = this._source[this._pos];
        if (char === " ") {
          indent++;
          this._advance();
        } else if (char === "	") {
          throw new LexerError(
            "Tab character in indentation (use spaces only)",
            this._line,
            this._column
          );
        } else {
          break;
        }
      }
      if (indent > 0 && this._preserveSourceInfo) {
        this._pushTrivia(
          "WHITESPACE",
          this._source.slice(indentStartOffset, this._pos),
          indentStartLine,
          indentStartColumn,
          indentStartOffset
        );
      }
      if (this._pos >= this._source.length) {
        return "skip";
      }
      if (this._source[this._pos] === "\n") {
        const newlineStartLine = this._line;
        const newlineStartColumn = this._column;
        const newlineStartOffset = this._pos;
        this._advance();
        this._pushTrivia(
          "NEWLINE",
          "\n",
          newlineStartLine,
          newlineStartColumn,
          newlineStartOffset
        );
        return "skip";
      }
      const remaining = this._source.slice(this._pos);
      for (const pat of this._skipPatterns) {
        const match = pat.pattern.exec(remaining);
        if (match !== null && match.index === 0) {
          const peekPos = this._pos + match[0].length;
          if (peekPos >= this._source.length || this._source[peekPos] === "\n") {
            const triviaStartLine = this._line;
            const triviaStartColumn = this._column;
            const triviaStartOffset = this._pos;
            for (let i = 0; i < match[0].length; i++) {
              this._advance();
            }
            this._pushTrivia(
              pat.name,
              match[0],
              triviaStartLine,
              triviaStartColumn,
              triviaStartOffset
            );
            if (this._pos < this._source.length && this._source[this._pos] === "\n") {
              const newlineStartLine = this._line;
              const newlineStartColumn = this._column;
              const newlineStartOffset = this._pos;
              this._advance();
              this._pushTrivia(
                "NEWLINE",
                "\n",
                newlineStartLine,
                newlineStartColumn,
                newlineStartOffset
              );
            }
            return "skip";
          }
        }
      }
      const currentIndent = indentStack[indentStack.length - 1];
      const indentTokens = [];
      if (indent > currentIndent) {
        indentStack.push(indent);
        indentTokens.push(this._withOptionalSourceInfo({
          type: "INDENT",
          value: "",
          line: this._line,
          column: 1
        }, this._pos));
      } else if (indent < currentIndent) {
        while (indentStack.length > 1 && indentStack[indentStack.length - 1] > indent) {
          indentStack.pop();
          indentTokens.push(this._withOptionalSourceInfo({
            type: "DEDENT",
            value: "",
            line: this._line,
            column: 1
          }, this._pos));
        }
        if (indentStack[indentStack.length - 1] !== indent) {
          throw new LexerError(
            "Inconsistent dedent",
            this._line,
            this._column
          );
        }
      }
      return indentTokens;
    }
    // -- Shared helpers --
    /**
     * Try to match and consume a skip pattern at the current position.
     *
     * Skip patterns are defined in the `skip:` section of a .tokens file.
     * They match text that should be consumed without emitting a token —
     * typically comments and inline whitespace.
     *
     * @returns true if a skip pattern matched (text was consumed), false otherwise.
     */
    _trySkip() {
      const remaining = this._source.slice(this._pos);
      for (const pat of this._skipPatterns) {
        const match = pat.pattern.exec(remaining);
        if (match !== null && match.index === 0) {
          const startLine = this._line;
          const startColumn = this._column;
          const startOffset = this._pos;
          for (let i = 0; i < match[0].length; i++) {
            this._advance();
          }
          this._pushTrivia(pat.name, match[0], startLine, startColumn, startOffset);
          return true;
        }
      }
      return false;
    }
    /**
     * Try to match a token pattern from a specific group.
     *
     * Tries each compiled pattern in the named group in priority order
     * (first match wins). Handles keyword detection, reserved word
     * checking, aliases, and string escape processing.
     *
     * @param groupName - The pattern group to use (e.g., "default", "tag").
     * @returns A Token if a pattern matched, null otherwise.
     */
    _tryMatchTokenInGroup(groupName) {
      const remaining = this._source.slice(this._pos);
      const patterns = this._groupPatterns[groupName] ?? this._patterns;
      for (const { name, pattern, alias } of patterns) {
        const match = pattern.exec(remaining);
        if (match !== null && match.index === 0) {
          let value = match[0];
          const startLine = this._line;
          const startColumn = this._column;
          const startOffset = this._pos;
          const lookupValue = this._caseInsensitive ? value.toUpperCase() : value;
          const tokenType = resolveTokenType(
            name,
            lookupValue,
            this._keywordSet,
            this._reservedSet,
            alias,
            startLine,
            startColumn
          );
          if (this._caseInsensitive && tokenType === "KEYWORD") {
            value = lookupValue;
          }
          const effectiveName = this._aliasMap[name] ?? name;
          if (effectiveName === "STRING" || name === "STRING" || name.includes("STRING") || alias && alias.includes("STRING")) {
            if (value.length >= 6 && (value.startsWith('"""') || value.startsWith("'''"))) {
              const inner = value.slice(3, -3);
              value = this._grammar.escapeMode === "none" ? inner : processEscapes(inner);
            } else if (value.length >= 2 && (value[0] === '"' || value[0] === "'")) {
              const inner = value.slice(1, -1);
              value = this._grammar.escapeMode === "none" ? inner : processEscapes(inner);
            }
          }
          let flags;
          if (tokenType === "NAME" && this._contextKeywordSet.size > 0 && this._contextKeywordSet.has(value)) {
            flags = TOKEN_CONTEXT_KEYWORD;
          }
          const tok = flags !== void 0 ? { type: tokenType, value, line: startLine, column: startColumn, flags } : { type: tokenType, value, line: startLine, column: startColumn };
          for (let i = 0; i < match[0].length; i++) {
            this._advance();
          }
          return this._withOptionalSourceInfo(tok, startOffset);
        }
      }
      return null;
    }
    _consumeDefaultWhitespace() {
      const startLine = this._line;
      const startColumn = this._column;
      const startOffset = this._pos;
      while (this._pos < this._source.length) {
        const char = this._source[this._pos];
        if (char !== " " && char !== "	" && char !== "\r") {
          break;
        }
        this._advance();
      }
      if (this._pos > startOffset) {
        this._pushTrivia(
          "WHITESPACE",
          this._source.slice(startOffset, this._pos),
          startLine,
          startColumn,
          startOffset
        );
      }
    }
    _pushTrivia(type, value, line, column, startOffset) {
      if (!this._preserveSourceInfo) {
        return;
      }
      this._pendingTrivia.push({
        type,
        value,
        line,
        column,
        endLine: this._line,
        endColumn: this._column,
        startOffset,
        endOffset: this._pos
      });
    }
    _withOptionalSourceInfo(token, startOffset) {
      if (!this._preserveSourceInfo) {
        return token;
      }
      return {
        ...token,
        startOffset,
        endOffset: this._pos,
        endLine: this._line,
        endColumn: this._column
      };
    }
    _emitToken(tokens, token) {
      let finalized = token;
      if (this._preserveSourceInfo) {
        finalized = {
          ...token,
          tokenIndex: this._nextTokenIndex++,
          ...this._pendingTrivia.length > 0 ? { leadingTrivia: [...this._pendingTrivia] } : {}
        };
        this._pendingTrivia = [];
      }
      tokens.push(finalized);
      this._lastEmittedToken = finalized;
    }
    /**
     * Move position forward by one character, tracking line and column.
     *
     * When we encounter a newline character, we increment the line counter
     * and reset the column to 1. For all other characters, we just increment
     * the column.
     */
    _advance() {
      if (this._pos < this._source.length) {
        if (this._source[this._pos] === "\n") {
          this._line += 1;
          this._column = 1;
        } else {
          this._column += 1;
        }
        this._pos += 1;
      }
    }
  };

  // ../../code/packages/typescript/excel-lexer/src/_grammar.ts
  var TOKEN_GRAMMAR = {
    version: 1,
    caseInsensitive: true,
    caseSensitive: true,
    definitions: [
      {
        name: "SPACE",
        pattern: " +",
        isRegex: true,
        lineNumber: 13
      },
      {
        name: "REF_PREFIX_QUOTED",
        pattern: "(\\[[^\\]]+\\])?'([^']|'')*'!",
        isRegex: true,
        lineNumber: 15,
        alias: "REF_PREFIX"
      },
      {
        name: "REF_PREFIX_WB_BARE",
        pattern: "\\[[^\\]]+\\][A-Za-z_][A-Za-z0-9_.:]*!",
        isRegex: true,
        lineNumber: 16,
        alias: "REF_PREFIX"
      },
      {
        name: "REF_PREFIX_BARE",
        pattern: "[A-Za-z_][A-Za-z0-9_.:]*!",
        isRegex: true,
        lineNumber: 17,
        alias: "REF_PREFIX"
      },
      {
        name: "STRING",
        pattern: '"([^"]|"")*"',
        isRegex: true,
        lineNumber: 19
      },
      {
        name: "ERROR_CONSTANT",
        pattern: "#[a-z0-9\\/\\?!]*[a-z0-9\\?!]",
        isRegex: true,
        lineNumber: 20
      },
      {
        name: "NUMBER_DOT_EXP",
        pattern: "\\.[0-9]+[eE][-+]?[0-9]+",
        isRegex: true,
        lineNumber: 26,
        alias: "NUMBER"
      },
      {
        name: "NUMBER_DOT",
        pattern: "\\.[0-9]+",
        isRegex: true,
        lineNumber: 27,
        alias: "NUMBER"
      },
      {
        name: "NUMBER_EXP",
        pattern: "[0-9]+\\.?[0-9]*[eE][-+]?[0-9]+",
        isRegex: true,
        lineNumber: 28,
        alias: "NUMBER"
      },
      {
        name: "NUMBER",
        pattern: "[0-9]+\\.?[0-9]*",
        isRegex: true,
        lineNumber: 29
      },
      {
        name: "STRUCTURED_KEYWORD",
        pattern: "\\[#[a-z ]+\\]",
        isRegex: true,
        lineNumber: 31
      },
      {
        name: "STRUCTURED_COLUMN",
        pattern: "\\[[^\\[\\]]+\\]",
        isRegex: true,
        lineNumber: 32
      },
      {
        name: "CELL",
        pattern: "\\$?[A-Za-z]{1,3}\\$?[0-9]{1,7}",
        isRegex: true,
        lineNumber: 35
      },
      {
        name: "NOT_EQUALS",
        pattern: "<>",
        isRegex: false,
        lineNumber: 37
      },
      {
        name: "LESS_EQUALS",
        pattern: "<=",
        isRegex: false,
        lineNumber: 38
      },
      {
        name: "GREATER_EQUALS",
        pattern: ">=",
        isRegex: false,
        lineNumber: 39
      },
      {
        name: "PLUS",
        pattern: "+",
        isRegex: false,
        lineNumber: 41
      },
      {
        name: "MINUS",
        pattern: "-",
        isRegex: false,
        lineNumber: 42
      },
      {
        name: "STAR",
        pattern: "*",
        isRegex: false,
        lineNumber: 43
      },
      {
        name: "SLASH",
        pattern: "/",
        isRegex: false,
        lineNumber: 44
      },
      {
        name: "CARET",
        pattern: "^",
        isRegex: false,
        lineNumber: 45
      },
      {
        name: "AMP",
        pattern: "&",
        isRegex: false,
        lineNumber: 46
      },
      {
        name: "PERCENT",
        pattern: "%",
        isRegex: false,
        lineNumber: 47
      },
      {
        name: "EQUALS",
        pattern: "=",
        isRegex: false,
        lineNumber: 48
      },
      {
        name: "LESS_THAN",
        pattern: "<",
        isRegex: false,
        lineNumber: 49
      },
      {
        name: "GREATER_THAN",
        pattern: ">",
        isRegex: false,
        lineNumber: 50
      },
      {
        name: "BANG",
        pattern: "!",
        isRegex: false,
        lineNumber: 51
      },
      {
        name: "DOLLAR",
        pattern: "$",
        isRegex: false,
        lineNumber: 52
      },
      {
        name: "LPAREN",
        pattern: "(",
        isRegex: false,
        lineNumber: 53
      },
      {
        name: "RPAREN",
        pattern: ")",
        isRegex: false,
        lineNumber: 54
      },
      {
        name: "LBRACE",
        pattern: "{",
        isRegex: false,
        lineNumber: 55
      },
      {
        name: "RBRACE",
        pattern: "}",
        isRegex: false,
        lineNumber: 56
      },
      {
        name: "LBRACKET",
        pattern: "[",
        isRegex: false,
        lineNumber: 57
      },
      {
        name: "RBRACKET",
        pattern: "]",
        isRegex: false,
        lineNumber: 58
      },
      {
        name: "COMMA",
        pattern: ",",
        isRegex: false,
        lineNumber: 59
      },
      {
        name: "SEMICOLON",
        pattern: ";",
        isRegex: false,
        lineNumber: 60
      },
      {
        name: "COLON",
        pattern: ":",
        isRegex: false,
        lineNumber: 61
      },
      {
        name: "AT",
        pattern: "@",
        isRegex: false,
        lineNumber: 62
      },
      {
        name: "NAME",
        pattern: "[A-Za-z_\\\\][A-Za-z0-9_\\.]*",
        isRegex: true,
        lineNumber: 64
      },
      {
        name: "FUNCTION_NAME",
        pattern: "[^\\s\\S]",
        isRegex: true,
        lineNumber: 70
      },
      {
        name: "TABLE_NAME",
        pattern: "[^\\s\\S]",
        isRegex: true,
        lineNumber: 71
      },
      {
        name: "COLUMN_REF",
        pattern: "[^\\s\\S]",
        isRegex: true,
        lineNumber: 72
      },
      {
        name: "ROW_REF",
        pattern: "[^\\s\\S]",
        isRegex: true,
        lineNumber: 73
      }
    ],
    keywords: ["true", "false"],
    mode: void 0,
    escapeMode: void 0,
    skipDefinitions: [
      {
        name: "NONSPACE_WHITESPACE",
        pattern: "[\\t\\r\\n]+",
        isRegex: true,
        lineNumber: 11
      }
    ],
    reservedKeywords: [],
    layoutKeywords: [],
    contextKeywords: [],
    errorDefinitions: [],
    groups: {}
  };

  // ../../code/packages/typescript/excel-lexer/src/tokenizer.ts
  function nextNonSpaceChar(ctx) {
    let offset = 1;
    for (; ; ) {
      const ch = ctx.peek(offset);
      if (ch === "" || ch !== " ") {
        return ch;
      }
      offset += 1;
    }
  }
  function excelOnToken(token, ctx) {
    if (token.type !== "NAME") {
      return;
    }
    const nextChar = nextNonSpaceChar(ctx);
    if (nextChar === "(") {
      ctx.suppress();
      ctx.emit({ ...token, type: "FUNCTION_NAME" });
      return;
    }
    if (nextChar === "[") {
      ctx.suppress();
      ctx.emit({ ...token, type: "TABLE_NAME" });
    }
  }
  function createExcelLexer(source) {
    const lexer = new GrammarLexer(source, TOKEN_GRAMMAR);
    lexer.setOnToken(excelOnToken);
    return lexer;
  }
  function tokenizeExcelFormula(source) {
    return createExcelLexer(source).tokenize();
  }

  // ../../code/packages/typescript/excel-parser/src/_grammar.ts
  var PARSER_GRAMMAR = {
    version: 1,
    rules: [
      {
        name: "formula",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "ws" },
          { type: "optional", element: { type: "sequence", elements: [
            { type: "token_reference", name: "EQUALS" },
            { type: "rule_reference", name: "ws" }
          ] } },
          { type: "rule_reference", name: "expression" },
          { type: "rule_reference", name: "ws" }
        ] },
        lineNumber: 15
      },
      {
        name: "ws",
        body: { type: "repetition", element: { type: "token_reference", name: "SPACE" } },
        lineNumber: 17
      },
      {
        name: "req_space",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "SPACE" },
          { type: "repetition", element: { type: "token_reference", name: "SPACE" } }
        ] },
        lineNumber: 18
      },
      {
        name: "expression",
        body: { type: "rule_reference", name: "comparison_expr" },
        lineNumber: 20
      },
      {
        name: "comparison_expr",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "concat_expr" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "comparison_op" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "concat_expr" }
          ] } }
        ] },
        lineNumber: 22
      },
      {
        name: "comparison_op",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "EQUALS" },
          { type: "token_reference", name: "NOT_EQUALS" },
          { type: "token_reference", name: "LESS_THAN" },
          { type: "token_reference", name: "LESS_EQUALS" },
          { type: "token_reference", name: "GREATER_THAN" },
          { type: "token_reference", name: "GREATER_EQUALS" }
        ] },
        lineNumber: 23
      },
      {
        name: "concat_expr",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "additive_expr" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "AMP" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "additive_expr" }
          ] } }
        ] },
        lineNumber: 26
      },
      {
        name: "additive_expr",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "multiplicative_expr" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "PLUS" },
              { type: "token_reference", name: "MINUS" }
            ] } },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "multiplicative_expr" }
          ] } }
        ] },
        lineNumber: 27
      },
      {
        name: "multiplicative_expr",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "power_expr" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "group", element: { type: "alternation", choices: [
              { type: "token_reference", name: "STAR" },
              { type: "token_reference", name: "SLASH" }
            ] } },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "power_expr" }
          ] } }
        ] },
        lineNumber: 28
      },
      {
        name: "power_expr",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "unary_expr" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "CARET" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "unary_expr" }
          ] } }
        ] },
        lineNumber: 29
      },
      {
        name: "unary_expr",
        body: { type: "sequence", elements: [
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "prefix_op" },
            { type: "rule_reference", name: "ws" }
          ] } },
          { type: "rule_reference", name: "postfix_expr" }
        ] },
        lineNumber: 30
      },
      {
        name: "prefix_op",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "PLUS" },
          { type: "token_reference", name: "MINUS" }
        ] },
        lineNumber: 31
      },
      {
        name: "postfix_expr",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "primary" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "PERCENT" }
          ] } }
        ] },
        lineNumber: 32
      },
      {
        name: "primary",
        body: { type: "alternation", choices: [
          { type: "rule_reference", name: "parenthesized_expression" },
          { type: "rule_reference", name: "constant" },
          { type: "rule_reference", name: "function_call" },
          { type: "rule_reference", name: "structure_reference" },
          { type: "rule_reference", name: "reference_expression" },
          { type: "rule_reference", name: "bang_reference" },
          { type: "rule_reference", name: "bang_name" },
          { type: "rule_reference", name: "name_reference" }
        ] },
        lineNumber: 34
      },
      {
        name: "parenthesized_expression",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "expression" },
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "RPAREN" }
        ] },
        lineNumber: 43
      },
      {
        name: "constant",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "NUMBER" },
          { type: "token_reference", name: "STRING" },
          { type: "token_reference", name: "KEYWORD" },
          { type: "token_reference", name: "ERROR_CONSTANT" },
          { type: "rule_reference", name: "array_constant" }
        ] },
        lineNumber: 45
      },
      {
        name: "array_constant",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "LBRACE" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "array_row" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "SEMICOLON" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "array_row" }
          ] } },
          { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "SEMICOLON" }
          ] } },
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "RBRACE" }
        ] },
        lineNumber: 47
      },
      {
        name: "array_row",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "array_item" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "array_item" }
          ] } },
          { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COMMA" }
          ] } }
        ] },
        lineNumber: 48
      },
      {
        name: "array_item",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "NUMBER" },
          { type: "token_reference", name: "STRING" },
          { type: "token_reference", name: "KEYWORD" },
          { type: "token_reference", name: "ERROR_CONSTANT" }
        ] },
        lineNumber: 49
      },
      {
        name: "function_call",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "function_name" },
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "ws" },
          { type: "optional", element: { type: "rule_reference", name: "function_argument_list" } },
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "RPAREN" }
        ] },
        lineNumber: 51
      },
      {
        name: "function_name",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "FUNCTION_NAME" },
          { type: "token_reference", name: "NAME" }
        ] },
        lineNumber: 52
      },
      {
        name: "function_argument_list",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "function_argument" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "function_argument" }
          ] } },
          { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COMMA" }
          ] } }
        ] },
        lineNumber: 53
      },
      {
        name: "function_argument",
        body: { type: "optional", element: { type: "rule_reference", name: "expression" } },
        lineNumber: 54
      },
      {
        name: "reference_expression",
        body: { type: "rule_reference", name: "union_reference" },
        lineNumber: 56
      },
      {
        name: "union_reference",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "intersection_reference" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "intersection_reference" }
          ] } }
        ] },
        lineNumber: 57
      },
      {
        name: "intersection_reference",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "range_reference" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "req_space" },
            { type: "rule_reference", name: "range_reference" }
          ] } }
        ] },
        lineNumber: 58
      },
      {
        name: "range_reference",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "reference_primary" },
          { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "reference_primary" }
          ] } }
        ] },
        lineNumber: 59
      },
      {
        name: "reference_primary",
        body: { type: "alternation", choices: [
          { type: "rule_reference", name: "parenthesized_reference" },
          { type: "rule_reference", name: "prefixed_reference" },
          { type: "rule_reference", name: "external_reference" },
          { type: "rule_reference", name: "structure_reference" },
          { type: "rule_reference", name: "a1_reference" },
          { type: "rule_reference", name: "bang_reference" },
          { type: "rule_reference", name: "bang_name" },
          { type: "rule_reference", name: "name_reference" }
        ] },
        lineNumber: 61
      },
      {
        name: "parenthesized_reference",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "LPAREN" },
          { type: "rule_reference", name: "ws" },
          { type: "rule_reference", name: "reference_expression" },
          { type: "rule_reference", name: "ws" },
          { type: "token_reference", name: "RPAREN" }
        ] },
        lineNumber: 70
      },
      {
        name: "prefixed_reference",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "REF_PREFIX" },
          { type: "group", element: { type: "alternation", choices: [
            { type: "rule_reference", name: "a1_reference" },
            { type: "rule_reference", name: "name_reference" },
            { type: "rule_reference", name: "structure_reference" }
          ] } }
        ] },
        lineNumber: 71
      },
      {
        name: "external_reference",
        body: { type: "token_reference", name: "REF_PREFIX" },
        lineNumber: 72
      },
      {
        name: "bang_reference",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "BANG" },
          { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "CELL" },
            { type: "token_reference", name: "COLUMN_REF" },
            { type: "token_reference", name: "ROW_REF" },
            { type: "token_reference", name: "NUMBER" }
          ] } }
        ] },
        lineNumber: 73
      },
      {
        name: "bang_name",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "BANG" },
          { type: "rule_reference", name: "name_reference" }
        ] },
        lineNumber: 74
      },
      {
        name: "name_reference",
        body: { type: "token_reference", name: "NAME" },
        lineNumber: 75
      },
      {
        name: "column_reference",
        body: { type: "sequence", elements: [
          { type: "optional", element: { type: "token_reference", name: "DOLLAR" } },
          { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "COLUMN_REF" },
            { type: "token_reference", name: "NAME" }
          ] } }
        ] },
        lineNumber: 77
      },
      {
        name: "row_reference",
        body: { type: "sequence", elements: [
          { type: "optional", element: { type: "token_reference", name: "DOLLAR" } },
          { type: "group", element: { type: "alternation", choices: [
            { type: "token_reference", name: "ROW_REF" },
            { type: "token_reference", name: "NUMBER" }
          ] } }
        ] },
        lineNumber: 78
      },
      {
        name: "a1_reference",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "CELL" },
          { type: "rule_reference", name: "column_reference" },
          { type: "rule_reference", name: "row_reference" },
          { type: "token_reference", name: "COLUMN_REF" },
          { type: "token_reference", name: "ROW_REF" },
          { type: "token_reference", name: "NAME" },
          { type: "token_reference", name: "NUMBER" }
        ] },
        lineNumber: 80
      },
      {
        name: "structure_reference",
        body: { type: "sequence", elements: [
          { type: "optional", element: { type: "rule_reference", name: "table_name" } },
          { type: "rule_reference", name: "intra_table_reference" }
        ] },
        lineNumber: 82
      },
      {
        name: "table_name",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "TABLE_NAME" },
          { type: "token_reference", name: "NAME" }
        ] },
        lineNumber: 83
      },
      {
        name: "intra_table_reference",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "STRUCTURED_KEYWORD" },
          { type: "rule_reference", name: "structured_column_range" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "LBRACKET" },
            { type: "rule_reference", name: "ws" },
            { type: "optional", element: { type: "rule_reference", name: "inner_structure_reference" } },
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "RBRACKET" }
          ] }
        ] },
        lineNumber: 84
      },
      {
        name: "inner_structure_reference",
        body: { type: "alternation", choices: [
          { type: "sequence", elements: [
            { type: "rule_reference", name: "structured_keyword_list" },
            { type: "optional", element: { type: "sequence", elements: [
              { type: "rule_reference", name: "ws" },
              { type: "token_reference", name: "COMMA" },
              { type: "rule_reference", name: "ws" },
              { type: "rule_reference", name: "structured_column_range" }
            ] } }
          ] },
          { type: "rule_reference", name: "structured_column_range" }
        ] },
        lineNumber: 87
      },
      {
        name: "structured_keyword_list",
        body: { type: "sequence", elements: [
          { type: "token_reference", name: "STRUCTURED_KEYWORD" },
          { type: "repetition", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COMMA" },
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "STRUCTURED_KEYWORD" }
          ] } }
        ] },
        lineNumber: 89
      },
      {
        name: "structured_column_range",
        body: { type: "sequence", elements: [
          { type: "rule_reference", name: "structured_column" },
          { type: "optional", element: { type: "sequence", elements: [
            { type: "rule_reference", name: "ws" },
            { type: "token_reference", name: "COLON" },
            { type: "rule_reference", name: "ws" },
            { type: "rule_reference", name: "structured_column" }
          ] } }
        ] },
        lineNumber: 90
      },
      {
        name: "structured_column",
        body: { type: "alternation", choices: [
          { type: "token_reference", name: "STRUCTURED_COLUMN" },
          { type: "sequence", elements: [
            { type: "token_reference", name: "AT" },
            { type: "token_reference", name: "STRUCTURED_COLUMN" }
          ] }
        ] },
        lineNumber: 91
      }
    ]
  };

  // ../../code/packages/typescript/excel-parser/src/parser.ts
  function previousSignificantToken(tokens, index) {
    for (let i = index - 1; i >= 0; i -= 1) {
      if (tokens[i].type !== "SPACE") {
        return tokens[i];
      }
    }
    return null;
  }
  function nextSignificantToken(tokens, index) {
    for (let i = index + 1; i < tokens.length; i += 1) {
      if (tokens[i].type !== "SPACE") {
        return tokens[i];
      }
    }
    return null;
  }
  function normalizeExcelReferenceTokens(tokens) {
    return tokens.map((token, index) => {
      if (token.type !== "NAME" && token.type !== "NUMBER") {
        return token;
      }
      const previous = previousSignificantToken(tokens, index);
      const next = nextSignificantToken(tokens, index);
      const adjacentToColon = previous?.type === "COLON" || next?.type === "COLON";
      if (token.type === "NAME" && adjacentToColon) {
        return { ...token, type: "COLUMN_REF" };
      }
      if (token.type === "NUMBER" && adjacentToColon) {
        return { ...token, type: "ROW_REF" };
      }
      return token;
    });
  }
  function parseExcelFormula(source) {
    const tokens = tokenizeExcelFormula(source);
    const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
    parser.addPreParse(normalizeExcelReferenceTokens);
    return parser.parse();
  }

  // ../../code/packages/typescript/symbolic-ir/src/index.ts
  function sym(name) {
    return Object.freeze({ kind: "symbol", name });
  }
  function int(value) {
    return Object.freeze({ kind: "integer", value: toBigInt(value) });
  }
  function rational(numer, denom) {
    let n = toBigInt(numer);
    let d = toBigInt(denom);
    if (d === 0n) {
      throw new RangeError("IRRational denominator cannot be zero");
    }
    if (d < 0n) {
      n = -n;
      d = -d;
    }
    const g = gcd(abs(n), d);
    return Object.freeze({ kind: "rational", numer: n / g, denom: d / g });
  }
  function numberNode(value) {
    if (!Number.isFinite(value)) {
      throw new RangeError("IRFloat value must be finite");
    }
    return Object.freeze({ kind: "float", value });
  }
  function app(head, args) {
    return Object.freeze({
      kind: "apply",
      head,
      args: Object.freeze([...args])
    });
  }
  function headName(node) {
    return node.kind === "symbol" ? node.name : "";
  }
  function toBigInt(value) {
    if (typeof value === "bigint") return value;
    if (typeof value === "string") return BigInt(value);
    if (!Number.isSafeInteger(value)) {
      throw new RangeError("integer number inputs must be safe integers; pass a string or bigint for larger values");
    }
    return BigInt(value);
  }
  function gcd(a, b) {
    while (b !== 0n) {
      const t = b;
      b = a % b;
      a = t;
    }
    return a === 0n ? 1n : a;
  }
  function abs(n) {
    return n < 0n ? -n : n;
  }
  var ADD = sym("Add");
  var SUB = sym("Sub");
  var MUL = sym("Mul");
  var DIV = sym("Div");
  var POW = sym("Pow");
  var NEG = sym("Neg");
  var INV = sym("Inv");
  var EXP = sym("Exp");
  var LOG = sym("Log");
  var SIN = sym("Sin");
  var COS = sym("Cos");
  var TAN = sym("Tan");
  var SQRT = sym("Sqrt");
  var ATAN = sym("Atan");
  var ASIN = sym("Asin");
  var ACOS = sym("Acos");
  var SINH = sym("Sinh");
  var COSH = sym("Cosh");
  var TANH = sym("Tanh");
  var ASINH = sym("Asinh");
  var ACOSH = sym("Acosh");
  var ATANH = sym("Atanh");
  var COTH = sym("Coth");
  var SECH = sym("Sech");
  var CSCH = sym("Csch");
  var D = sym("D");
  var INTEGRATE = sym("Integrate");
  var SUM = sym("Sum");
  var PRODUCT = sym("Product");
  var FACTOR = sym("Factor");
  var SOLVE = sym("Solve");
  var SIMPLIFY = sym("Simplify");
  var SUBST = sym("Subst");
  var EQUAL = sym("Equal");
  var NOT_EQUAL = sym("NotEqual");
  var LESS = sym("Less");
  var GREATER = sym("Greater");
  var LESS_EQUAL = sym("LessEqual");
  var GREATER_EQUAL = sym("GreaterEqual");
  var TRUE = sym("True");
  var FALSE = sym("False");
  var AND = sym("And");
  var OR = sym("Or");
  var NOT = sym("Not");
  var IF = sym("If");
  var LIST = sym("List");
  var ASSIGN = sym("Assign");
  var DEFINE = sym("Define");
  var RULE = sym("Rule");
  var BLOCK = sym("Block");
  var RETURN = sym("Return");
  var WHILE = sym("While");
  var FOR_RANGE = sym("ForRange");
  var FOR_EACH = sym("ForEach");
  var ASSUME = sym("Assume");
  var FORGET = sym("Forget");
  var IS = sym("Is");
  var SIGN = sym("Sign");
  var LEGENDRE_P = sym("LegendreP");
  var LEGENDRE_Q = sym("LegendreQ");
  var BESSEL_J = sym("BesselJ");
  var BESSEL_Y = sym("BesselY");
  var HERMITE_H = sym("HermiteH");
  var HERMITE_H2 = sym("HermiteH2");
  var CHEBYSHEV_T = sym("ChebyshevT");
  var CHEBYSHEV_U = sym("ChebyshevU");

  // ../../code/packages/typescript/cas-pattern-matching/src/index.ts
  var BLANK = "Blank";
  var PATTERN = "Pattern";
  var RULE2 = "Rule";
  function blank() {
    return app(sym(BLANK), []);
  }
  function named(name, inner) {
    return app(sym(PATTERN), [sym(name), inner]);
  }
  function rule(lhs, rhs) {
    return app(sym(RULE2), [lhs, rhs]);
  }

  // ../../code/packages/typescript/cas-simplify/src/rules.ts
  function buildIdentityRules() {
    const x = () => named("x", blank());
    const zero = int(0);
    const one = int(1);
    return [
      rule(app(ADD, [x(), zero]), x()),
      rule(app(ADD, [zero, x()]), x()),
      rule(app(MUL, [x(), one]), x()),
      rule(app(MUL, [one, x()]), x()),
      rule(app(MUL, [x(), zero]), zero),
      rule(app(MUL, [zero, x()]), zero),
      rule(app(POW, [x(), zero]), one),
      rule(app(POW, [x(), one]), x()),
      rule(app(POW, [one, x()]), one),
      rule(app(SUB, [x(), x()]), zero),
      rule(app(DIV, [x(), x()]), one),
      rule(app(LOG, [app(EXP, [x()])]), x()),
      rule(app(EXP, [app(LOG, [x()])]), x()),
      rule(app(SIN, [zero]), zero),
      rule(app(COS, [zero]), one)
    ];
  }
  var IDENTITY_RULES = Object.freeze(buildIdentityRules());

  // ../../code/packages/typescript/cas-simplify/src/assumptions.ts
  var ZERO_IR = int(0);
  var RELATION_HEAD_TO_OP = /* @__PURE__ */ new Map([
    [GREATER.name, ">"],
    [LESS.name, "<"],
    [GREATER_EQUAL.name, ">="],
    [LESS_EQUAL.name, "<="],
    [EQUAL.name, "="],
    [NOT_EQUAL.name, "!="]
  ]);

  // ../../code/packages/typescript/cas-simplify/src/exponentialize.ts
  var IMAGINARY_UNIT = sym("ImaginaryUnit");
  var TWO = int(2);

  // ../../code/packages/typescript/cas-simplify/src/heads.ts
  var CANONICAL = sym("Canonical");
  var ASSUME2 = sym("Assume");
  var FORGET2 = sym("Forget");
  var IS2 = sym("Is");
  var SIGN2 = sym("Sign");
  var RADCAN = sym("Radcan");
  var LOGCONTRACT = sym("LogContract");
  var LOGEXPAND = sym("LogExpand");
  var EXPONENTIALIZE = sym("Exponentialize");
  var DEMOIVRE = sym("DeMoivre");

  // ../../code/packages/typescript/cas-simplify/src/index.ts
  function numericFold(node) {
    if (node.kind !== "apply") return node;
    const head = numericFold(node.head);
    const args = node.args.map(numericFold);
    const name = headName(head);
    if (name === NEG.name && args.length === 1 && args[0].kind === "integer") return int(-args[0].value);
    if (name === INV.name && args.length === 1 && args[0].kind === "integer") return rational(1, args[0].value);
    if (args.length === 2 && args[0].kind === "integer" && args[1].kind === "integer") {
      const [a, b] = [args[0].value, args[1].value];
      if (name === ADD.name) return int(a + b);
      if (name === SUB.name) return int(a - b);
      if (name === MUL.name) return int(a * b);
      if (name === DIV.name) return rational(a, b);
      if (name === POW.name && b >= 0n) return int(a ** b);
    }
    return app(head, args);
  }

  // ../../code/packages/typescript/spreadsheet-engine/src/adapters/excel-cas.ts
  var FormulaError = class {
    constructor(value) {
      this.value = value;
    }
    value;
  };
  function fail(code) {
    throw new FormulaError(err(code));
  }
  function isToken(c) {
    return !isASTNode(c);
  }
  function kids(node) {
    return node.children.filter((c) => !(isASTNode(c) && c.ruleName === "ws"));
  }
  function unwrap(node) {
    let cur = node;
    while (isASTNode(cur)) {
      const k = kids(cur);
      if (k.length === 1) {
        cur = k[0];
      } else {
        break;
      }
    }
    return cur;
  }
  function collectRefs(node, out) {
    if (isToken(node)) return;
    if (node.ruleName === "range_reference") {
      pushRangeRefCells(node, out);
      return;
    }
    for (const child of kids(node)) {
      if (isASTNode(child)) {
        collectRefs(child, out);
      } else if (child.type === "CELL") {
        out.push(parseCellToken(child.value));
      }
    }
  }
  function pushRangeRefCells(rangeNode, out) {
    const range = rangeReferenceToRange(rangeNode);
    if (range) for (const a of expandRange(range)) out.push(a);
  }
  function rangeReferenceToRange(rangeNode) {
    const parts = kids(rangeNode);
    const colonIdx = parts.findIndex((c) => isToken(c) && c.type === "COLON");
    if (colonIdx === -1) {
      const cell = firstCellToken(rangeNode);
      if (!cell) return void 0;
      const a = parseCellToken(cell);
      return { start: a, end: a };
    }
    const left = firstCellTokenIn(parts.slice(0, colonIdx));
    const right = firstCellTokenIn(parts.slice(colonIdx + 1));
    if (!left || !right) return void 0;
    return { start: parseCellToken(left), end: parseCellToken(right) };
  }
  function firstCellToken(node) {
    if (isToken(node)) return node.type === "CELL" ? node.value : null;
    for (const c of kids(node)) {
      const found = firstCellToken(c);
      if (found) return found;
    }
    return null;
  }
  function firstCellTokenIn(nodes) {
    for (const n of nodes) {
      const found = firstCellToken(n);
      if (found) return found;
    }
    return null;
  }
  function parseCellToken(value) {
    return parseA1(value);
  }
  function evalExpr(node, resolve) {
    const n = unwrap(node);
    if (isToken(n)) {
      return evalToken(n, resolve);
    }
    switch (n.ruleName) {
      case "additive_expr":
      case "multiplicative_expr":
        return evalBinaryChain(
          n,
          resolve,
          /*rightAssoc*/
          false
        );
      case "power_expr":
        return evalBinaryChain(
          n,
          resolve,
          /*rightAssoc*/
          true
        );
      case "concat_expr":
        return evalConcat(n, resolve);
      case "comparison_expr":
        return evalComparison(n, resolve);
      case "unary_expr":
        return evalUnary(n, resolve);
      case "postfix_expr":
        return evalPostfix(n, resolve);
      case "parenthesized_expression": {
        const inner = kids(n).find((c) => isASTNode(c));
        return inner ? evalExpr(inner, resolve) : err("#VALUE!");
      }
      case "function_call":
        return evalFunctionCall(n, resolve);
      case "range_reference": {
        const range = rangeReferenceToRange(n);
        if (range && range.start.col === range.end.col && range.start.row === range.end.row) {
          return resolve(range.start);
        }
        return err("#VALUE!");
      }
      default:
        const only = kids(n);
        if (only.length === 1) return evalExpr(only[0], resolve);
        return err("#VALUE!");
    }
  }
  function evalToken(tok, resolve) {
    switch (tok.type) {
      case "NUMBER":
        return num(Number(tok.value));
      case "STRING":
        return text(tok.value);
      case "CELL": {
        const v = resolve(parseCellToken(tok.value));
        return v;
      }
      case "KEYWORD": {
        const u = tok.value.toUpperCase();
        if (u === "TRUE") return bool(true);
        if (u === "FALSE") return bool(false);
        return err("#NAME?");
      }
      default:
        return err("#VALUE!");
    }
  }
  function evalBinaryChain(node, resolve, rightAssoc) {
    const parts = kids(node);
    const operands = [];
    const ops = [];
    for (let i = 0; i < parts.length; i++) {
      const p = parts[i];
      if (isToken(p) && isBinaryOpToken(p.type)) {
        ops.push(p.type);
      } else {
        operands.push(toIR(evalExpr(p, resolve)));
      }
    }
    if (operands.length === 1) return irToValue(operands[0]);
    let ir;
    if (rightAssoc) {
      ir = operands[operands.length - 1];
      for (let i = operands.length - 2; i >= 0; i--) {
        ir = applyOp(ops[i], operands[i], ir);
      }
    } else {
      ir = operands[0];
      for (let i = 0; i < ops.length; i++) {
        ir = applyOp(ops[i], ir, operands[i + 1]);
      }
    }
    return irToValue(ir);
  }
  function isBinaryOpToken(type) {
    return type === "PLUS" || type === "MINUS" || type === "STAR" || type === "SLASH" || type === "CARET";
  }
  function applyOp(op, lhs, rhs) {
    switch (op) {
      case "PLUS":
        return app(ADD, [lhs, rhs]);
      case "MINUS":
        return app(SUB, [lhs, rhs]);
      case "STAR":
        return app(MUL, [lhs, rhs]);
      case "SLASH":
        if (isZeroIR(rhs)) fail("#DIV/0!");
        return app(DIV, [lhs, rhs]);
      case "CARET":
        return app(POW, [lhs, rhs]);
      default:
        return fail("#VALUE!");
    }
  }
  function isZeroIR(node) {
    if (node.kind === "integer") return node.value === 0n;
    if (node.kind === "float") return node.value === 0;
    if (node.kind === "rational") return node.numer === 0n;
    return false;
  }
  function toIR(v) {
    const n = toNumber(v);
    if (typeof n !== "number") throw new FormulaError(n);
    return numberToIR(n);
  }
  function numberToIR(n) {
    if (Number.isInteger(n)) return int(BigInt(n));
    return numberNode(n);
  }
  function irToValue(ir) {
    const folded = numericFold(ir);
    const direct = irToNumber(folded);
    if (direct !== void 0) return num(direct);
    const f = evalIRFloat(folded);
    return num(f);
  }
  function irToNumber(node) {
    switch (node.kind) {
      case "integer":
        return Number(node.value);
      case "rational":
        return Number(node.numer) / Number(node.denom);
      case "float":
        return node.value;
      default:
        return void 0;
    }
  }
  function evalIRFloat(node) {
    const concrete = irToNumber(node);
    if (concrete !== void 0) return concrete;
    if (node.kind !== "apply") fail("#VALUE!");
    const head = node.head.kind === "symbol" ? node.head.name : "";
    const a = node.args.map(evalIRFloat);
    switch (head) {
      case "Add":
        return a.reduce((x, y) => x + y, 0);
      case "Sub":
        return a.length === 1 ? -a[0] : a[0] - a[1];
      case "Mul":
        return a.reduce((x, y) => x * y, 1);
      case "Div":
        return a[0] / a[1];
      case "Pow":
        return Math.pow(a[0], a[1]);
      case "Neg":
        return -a[0];
      default:
        return fail("#VALUE!");
    }
  }
  function evalConcat(node, resolve) {
    const parts = kids(node).filter((c) => !(isToken(c) && c.type === "AMP"));
    let out = "";
    for (const p of parts) {
      const v = evalExpr(p, resolve);
      if (isError(v)) return v;
      out += toText(v);
    }
    return text(out);
  }
  function evalComparison(node, resolve) {
    const parts = kids(node);
    if (parts.length !== 3) {
      return parts.length ? evalExpr(parts[0], resolve) : err("#VALUE!");
    }
    const lhs = evalExpr(parts[0], resolve);
    const rhs = evalExpr(parts[2], resolve);
    if (isError(lhs)) return lhs;
    if (isError(rhs)) return rhs;
    const opNode = parts[1];
    const op = isToken(opNode) ? opNode.type : firstAnyToken(opNode)?.type ?? "";
    const ln = toNumber(lhs);
    const rn = toNumber(rhs);
    let cmp;
    if (typeof ln === "number" && typeof rn === "number") {
      cmp = ln < rn ? -1 : ln > rn ? 1 : 0;
    } else {
      const lt = toText(lhs);
      const rt = toText(rhs);
      cmp = lt < rt ? -1 : lt > rt ? 1 : 0;
    }
    switch (op) {
      case "EQUALS":
        return bool(cmp === 0);
      case "NOT_EQUALS":
        return bool(cmp !== 0);
      case "LESS_THAN":
        return bool(cmp < 0);
      case "GREATER_THAN":
        return bool(cmp > 0);
      case "LESS_EQUALS":
        return bool(cmp <= 0);
      case "GREATER_EQUALS":
        return bool(cmp >= 0);
      default:
        return err("#VALUE!");
    }
  }
  function evalUnary(node, resolve) {
    const parts = kids(node);
    const operand = parts[parts.length - 1];
    let v = evalExpr(operand, resolve);
    for (let i = parts.length - 2; i >= 0; i--) {
      const opTok = firstTokenOfType(parts[i], "MINUS");
      if (opTok) {
        const ir = numericFold(app(NEG, [toIR(v)]));
        v = irToValue(ir);
      }
    }
    return v;
  }
  function firstTokenOfType(node, type) {
    if (isToken(node)) return node.type === type ? node : null;
    for (const c of kids(node)) {
      const found = firstTokenOfType(c, type);
      if (found) return found;
    }
    return null;
  }
  function firstAnyToken(node) {
    if (isToken(node)) return node;
    for (const c of kids(node)) {
      const found = firstAnyToken(c);
      if (found) return found;
    }
    return null;
  }
  function evalPostfix(node, resolve) {
    const parts = kids(node);
    let v = evalExpr(parts[0], resolve);
    for (let i = 1; i < parts.length; i++) {
      const t = parts[i];
      if (isToken(t) && t.type === "PERCENT") {
        const n = toNumber(v);
        if (typeof n !== "number") return n;
        v = num(n / 100);
      }
    }
    return v;
  }
  function evalFunctionCall(node, resolve) {
    const children = kids(node);
    const nameNode = children.find(
      (c) => isASTNode(c) && c.ruleName === "function_name"
    );
    const nameTok = nameNode ? kids(nameNode).find((c) => isToken(c) && c.type === "FUNCTION_NAME") : children.find((c) => isToken(c) && c.type === "FUNCTION_NAME");
    if (!nameTok) return err("#NAME?");
    const name = nameTok.value.toUpperCase();
    const fn = FUNCTIONS[name];
    if (!fn) return err("#NAME?");
    const argList = children.find(
      (c) => isASTNode(c) && c.ruleName === "function_argument_list"
    );
    try {
      const numbers = collectArgNumbers(argList, resolve);
      return fn(numbers);
    } catch (e) {
      if (e instanceof FormulaError) return e.value;
      if (e instanceof RangeTooLargeError) return err("#REF!");
      throw e;
    }
  }
  function collectArgNumbers(container, resolve) {
    if (!container) return [];
    const out = [];
    collectArgNumbersInto(container, resolve, out);
    return out;
  }
  function collectArgNumbersInto(node, resolve, out) {
    const n = unwrap(node);
    if (isToken(n)) {
      pushScalar(
        evalToken(n, resolve),
        out,
        /*skipEmpty*/
        false
      );
      return;
    }
    switch (n.ruleName) {
      // The list of `function_argument` nodes (commas, if present, sit between).
      case "function_argument_list":
      case "function_argument":
        for (const child of kids(n)) {
          if (isToken(child) && child.type === "COMMA") continue;
          collectArgNumbersInto(child, resolve, out);
        }
        return;
      // `SUM(A1,B1,5)` parses its comma-separated args as a single
      // `union_reference` holding COMMA-separated members. Each member is its own
      // argument.
      case "union_reference":
        for (const child of kids(n)) {
          if (isToken(child) && child.type === "COMMA") continue;
          collectArgNumbersInto(child, resolve, out);
        }
        return;
      // A `range_reference`: either a single cell or an expanded range. Empty
      // cells inside a range are skipped (blank ≠ zero for SUM/AVERAGE/COUNT).
      case "range_reference": {
        const range = rangeReferenceToRange(n);
        if (range) {
          const isSingle = range.start.col === range.end.col && range.start.row === range.end.row;
          for (const addr of expandRange(range)) {
            pushScalar(
              resolve(addr),
              out,
              /*skipEmpty*/
              !isSingle
            );
          }
          return;
        }
        break;
      }
    }
    pushScalar(
      evalExpr(n, resolve),
      out,
      /*skipEmpty*/
      false
    );
  }
  function pushScalar(v, out, skipEmpty) {
    if (v.kind === "empty") {
      if (!skipEmpty) out.push(0);
      return;
    }
    if (isError(v)) throw new FormulaError(v);
    const n = toNumber(v);
    if (typeof n !== "number") throw new FormulaError(n);
    out.push(n);
  }
  var FUNCTIONS = {
    SUM: (nums) => num(nums.reduce((a, b) => a + b, 0)),
    AVERAGE: (nums) => nums.length === 0 ? err("#DIV/0!") : num(nums.reduce((a, b) => a + b, 0) / nums.length),
    MIN: (nums) => nums.length === 0 ? num(0) : num(Math.min(...nums)),
    MAX: (nums) => nums.length === 0 ? num(0) : num(Math.max(...nums)),
    COUNT: (nums) => num(nums.length),
    PRODUCT: (nums) => num(nums.reduce((a, b) => a * b, 1))
  };
  var excelCasAdapter = {
    isFormula(raw) {
      return raw.startsWith("=");
    },
    dependencies(raw) {
      try {
        const cst = parseExcelFormula(raw);
        const refs = [];
        collectRefs(cst, refs);
        return refs;
      } catch {
        return [];
      }
    },
    evaluate(raw, resolve) {
      let cst;
      try {
        cst = parseExcelFormula(raw);
      } catch {
        return err("#NAME?");
      }
      const top = kids(cst);
      const exprNode = top.find((c) => !(isToken(c) && c.type === "EQUALS"));
      if (!exprNode) return err("#VALUE!");
      try {
        return evalExpr(exprNode, resolve);
      } catch (e) {
        if (e instanceof FormulaError) return e.value;
        if (e instanceof RangeTooLargeError) return err("#REF!");
        return err("#VALUE!");
      }
    }
  };

  // ../../code/packages/typescript/spreadsheet-engine/src/index.ts
  function createSpreadsheet(options = {}) {
    return new Workbook({ adapter: excelCasAdapter, mode: options.mode });
  }
  return __toCommonJS(index_exports);
})();
