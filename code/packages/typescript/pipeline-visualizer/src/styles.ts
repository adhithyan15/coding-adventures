/**
 * Styles — Embedded CSS for the HTML visualization.
 * ==================================================
 *
 * The generated HTML file must be *self-contained* — it should open
 * in any browser without needing an internet connection or external
 * stylesheets. To achieve this, we embed all CSS directly in a
 * `<style>` tag in the HTML `<head>`.
 *
 * The visual design follows a "code editor" aesthetic:
 *
 * - **Dark background** (#1e1e2e) inspired by popular editor themes
 * - **Monospace font** for code and data, sans-serif for headings
 * - **Syntax highlighting colors** for tokens, AST nodes, etc.
 * - **Table styling** with alternating row colors for readability
 * - **Responsive layout** that works on both desktop and mobile
 *
 * Why embed styles as a string constant rather than a CSS file?
 * Because the renderer produces a *single HTML file*. There's no
 * build step, no bundler, no file server — just one .html file you
 * can email to someone or open from your desktop. Embedding the CSS
 * as a string constant means the renderer can stamp it directly into
 * the HTML output.
 */

// ===========================================================================
// Color Palette
// ===========================================================================

/**
 * The color palette is inspired by the Catppuccin Mocha theme, which
 * provides good contrast ratios for accessibility while looking
 * attractive. Each color has a semantic meaning:
 *
 * - Blue (#89b4fa)     — Names, identifiers, variables
 * - Green (#a6e3a1)    — Numbers, literals, constants
 * - Red (#f38ba8)      — Operators, punctuation
 * - Yellow (#f9e2af)   — Keywords, reserved words
 * - Mauve (#cba6f7)    — Strings, types
 * - Peach (#fab387)    — Special tokens, flags
 * - Teal (#94e2d5)     — Addresses, memory locations
 * - Text (#cdd6f4)     — Default text color
 * - Surface (#313244)  — Table row backgrounds (alternating)
 * - Base (#1e1e2e)     — Page background
 */

// ===========================================================================
// CSS String
// ===========================================================================

/**
 * Returns the complete CSS stylesheet as a string.
 *
 * We use a function rather than a constant so that in the future
 * we could accept a theme parameter to switch between dark/light
 * modes. For now, only the dark theme is implemented.
 */
export function getStyles(): string {
  return `
    /* === Reset & Base === */
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background-color: #1e1e2e;
      color: #cdd6f4;
      line-height: 1.6;
      padding: 2rem;
      max-width: 1200px;
      margin: 0 auto;
    }

    /* === Typography === */
    h1 { color: #cba6f7; font-size: 2rem; margin-bottom: 0.5rem; }
    h2 { color: #89b4fa; font-size: 1.5rem; margin: 2rem 0 1rem; border-bottom: 1px solid #313244; padding-bottom: 0.5rem; }
    h3 { color: #a6e3a1; font-size: 1.1rem; margin: 1rem 0 0.5rem; }

    code, pre {
      font-family: "JetBrains Mono", "Fira Code", "Consolas", monospace;
    }

    /* === Header === */
    header {
      margin-bottom: 2rem;
      padding-bottom: 1rem;
      border-bottom: 2px solid #313244;
    }

    .source-code {
      background: #313244;
      padding: 1rem;
      border-radius: 8px;
      font-size: 1.1rem;
      margin: 1rem 0;
      overflow-x: auto;
    }

    .meta-info {
      color: #a6adc8;
      font-size: 0.9rem;
    }

    /* === Sections === */
    section {
      background: #181825;
      border-radius: 8px;
      padding: 1.5rem;
      margin-bottom: 1.5rem;
    }

    .stage-meta {
      display: flex;
      gap: 2rem;
      color: #a6adc8;
      font-size: 0.85rem;
      margin-bottom: 1rem;
    }

    /* === Token Badges === */
    .token-list {
      display: flex;
      flex-wrap: wrap;
      gap: 0.5rem;
      margin: 1rem 0;
    }

    .token {
      display: inline-flex;
      flex-direction: column;
      align-items: center;
      background: #313244;
      border-radius: 6px;
      padding: 0.4rem 0.6rem;
      font-size: 0.85rem;
    }

    .token-type {
      font-size: 0.7rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      margin-bottom: 0.2rem;
    }

    .token-value {
      font-family: "JetBrains Mono", "Fira Code", monospace;
      font-weight: bold;
    }

    /* Token type colors */
    .token-name     { border-left: 3px solid #89b4fa; }
    .token-name .token-type { color: #89b4fa; }
    .token-number   { border-left: 3px solid #a6e3a1; }
    .token-number .token-type { color: #a6e3a1; }
    .token-operator { border-left: 3px solid #f38ba8; }
    .token-operator .token-type { color: #f38ba8; }
    .token-keyword  { border-left: 3px solid #f9e2af; }
    .token-keyword .token-type { color: #f9e2af; }
    .token-string   { border-left: 3px solid #cba6f7; }
    .token-string .token-type { color: #cba6f7; }
    .token-default  { border-left: 3px solid #a6adc8; }
    .token-default .token-type { color: #a6adc8; }

    /* === AST Tree (SVG) === */
    .ast-container {
      overflow-x: auto;
      margin: 1rem 0;
    }

    .ast-container svg {
      display: block;
      margin: 0 auto;
    }

    /* === Tables (Bytecode, VM, Assembly, etc.) === */
    table {
      width: 100%;
      border-collapse: collapse;
      font-family: "JetBrains Mono", "Fira Code", monospace;
      font-size: 0.85rem;
      margin: 1rem 0;
    }

    th {
      background: #313244;
      color: #cba6f7;
      text-align: left;
      padding: 0.6rem 0.8rem;
      font-weight: 600;
    }

    td {
      padding: 0.5rem 0.8rem;
      border-bottom: 1px solid #313244;
    }

    tr:nth-child(even) { background: #1e1e2e; }
    tr:nth-child(odd)  { background: #181825; }

    /* === Stack Visualization === */
    .stack {
      display: inline-flex;
      flex-direction: column-reverse;
      border: 1px solid #45475a;
      border-radius: 4px;
      min-width: 3rem;
      min-height: 1.5rem;
    }

    .stack-item {
      padding: 0.2rem 0.4rem;
      text-align: center;
      border-top: 1px solid #45475a;
      color: #a6e3a1;
      font-size: 0.8rem;
    }

    .stack-item:first-child { border-top: none; }

    /* === Binary Encoding === */
    .encoding {
      display: inline-flex;
      gap: 2px;
      margin: 0.25rem 0;
    }

    .bit-field {
      display: inline-flex;
      flex-direction: column;
      align-items: center;
      font-size: 0.75rem;
    }

    .bit-field-label {
      font-size: 0.65rem;
      color: #a6adc8;
      margin-bottom: 2px;
    }

    .bit-field-value {
      padding: 0.15rem 0.3rem;
      border-radius: 3px;
      font-family: "JetBrains Mono", monospace;
      letter-spacing: 0.05em;
    }

    /* Bit field colors — each field gets a different color */
    .bit-field:nth-child(7n+1) .bit-field-value { background: #89b4fa33; color: #89b4fa; }
    .bit-field:nth-child(7n+2) .bit-field-value { background: #a6e3a133; color: #a6e3a1; }
    .bit-field:nth-child(7n+3) .bit-field-value { background: #f38ba833; color: #f38ba8; }
    .bit-field:nth-child(7n+4) .bit-field-value { background: #f9e2af33; color: #f9e2af; }
    .bit-field:nth-child(7n+5) .bit-field-value { background: #cba6f733; color: #cba6f7; }
    .bit-field:nth-child(7n+6) .bit-field-value { background: #fab38733; color: #fab387; }
    .bit-field:nth-child(7n+7) .bit-field-value { background: #94e2d533; color: #94e2d5; }

    /* === Register Changes === */
    .reg-changed {
      color: #f9e2af;
      font-weight: bold;
    }

    /* === ALU Operations === */
    .alu-bits {
      font-family: "JetBrains Mono", monospace;
      letter-spacing: 0.1em;
      color: #a6e3a1;
    }

    .flag-set   { color: #f9e2af; font-weight: bold; }
    .flag-clear { color: #585b70; }

    /* === Gate Trace === */
    .gate-group {
      margin: 1rem 0;
      padding: 0.75rem;
      background: #1e1e2e;
      border-radius: 6px;
    }

    .gate-group-title {
      color: #94e2d5;
      font-weight: 600;
      margin-bottom: 0.5rem;
    }

    .gate-row {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      padding: 0.25rem 0;
      font-family: "JetBrains Mono", monospace;
      font-size: 0.85rem;
    }

    .gate-name {
      color: #cba6f7;
      min-width: 3rem;
      font-weight: bold;
    }

    .gate-inputs  { color: #89b4fa; }
    .gate-arrow   { color: #585b70; }
    .gate-output  { color: #a6e3a1; font-weight: bold; }
    .gate-label   { color: #a6adc8; font-size: 0.8rem; }

    /* === Variables Table === */
    .variables {
      display: inline-flex;
      gap: 0.5rem;
    }

    .var-pair {
      background: #313244;
      padding: 0.15rem 0.4rem;
      border-radius: 3px;
      font-size: 0.8rem;
    }

    .var-name  { color: #89b4fa; }
    .var-value { color: #a6e3a1; }

    /* === Footer === */
    footer {
      margin-top: 2rem;
      padding-top: 1rem;
      border-top: 1px solid #313244;
      color: #585b70;
      font-size: 0.85rem;
      text-align: center;
    }

    /* === Responsive === */
    @media (max-width: 768px) {
      body { padding: 1rem; }
      h1 { font-size: 1.5rem; }
      table { font-size: 0.75rem; }
    }
  `;
}
