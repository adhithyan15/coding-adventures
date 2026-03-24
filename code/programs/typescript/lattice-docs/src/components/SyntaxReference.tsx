import { useState } from "react";

const TABS = [
  {
    id: "variables",
    label: "Variables",
    sections: [
      {
        title: "Declaration",
        description:
          "Variables start with $ and are declared like CSS properties. They can hold any CSS value.",
        code: `$color-primary: #4a90d9;
$font-size-base: 16px;
$spacing-unit: 8px;
$font-stack: system-ui, sans-serif;`,
      },
      {
        title: "Usage",
        description:
          "Use a variable anywhere a CSS value is expected. Variables are resolved at compile time.",
        code: `.button {
  background: $color-primary;
  font-size: $font-size-base;
  padding: $spacing-unit calc($spacing-unit * 2);
  font-family: $font-stack;
}`,
      },
      {
        title: "Scope Shadowing",
        description:
          "Variables can be re-declared inside a nested scope. The inner declaration shadows the outer one.",
        code: `$color: red;

.parent {
  $color: blue;   // shadows $color for this scope
  color: $color;  // → blue
}

.sibling {
  color: $color;  // → red (outer scope)
}`,
      },
    ],
  },
  {
    id: "mixins",
    label: "Mixins",
    sections: [
      {
        title: "Definition",
        description:
          "Define a mixin with @mixin. Parameters are optional; defaults are specified with : value.",
        code: `@mixin reset-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

@mixin button($bg: #4a90d9, $fg: white) {
  background: $bg;
  color: $fg;
  border: none;
  border-radius: 4px;
  padding: 0.5rem 1rem;
  cursor: pointer;
}`,
      },
      {
        title: "Inclusion",
        description:
          "Include a mixin with @include. Arguments are matched positionally or by name.",
        code: `ul.nav {
  @include reset-list;
}

.btn-primary {
  @include button;         // uses defaults
}

.btn-danger {
  @include button(#e74c3c);  // overrides $bg only
}

.btn-custom {
  @include button(#2ecc71, #111);
}`,
      },
      {
        title: "Accessing Scope",
        description:
          "Mixins inherit the caller's variable scope. Variables defined before @include are visible inside the mixin body.",
        code: `$brand: #4a90d9;

@mixin themed-card {
  // $brand is visible here from the caller's scope
  border-left: 4px solid $brand;
  padding: 1rem;
}

.card {
  @include themed-card;  // $brand = #4a90d9
}`,
      },
    ],
  },
  {
    id: "control",
    label: "Control Flow",
    sections: [
      {
        title: "@if / @else",
        description:
          "Conditionally include CSS blocks. Conditions support ==, !=, <, >, <=, >=, and, or, not.",
        code: `$theme: dark;

@if $theme == dark {
  body { background: #111; color: #eee; }
} @else if $theme == light {
  body { background: #fff; color: #111; }
} @else {
  body { background: #f5f5f5; color: #333; }
}`,
      },
      {
        title: "@for",
        description:
          "Loop over a numeric range. 'through' includes the end value; 'to' excludes it.",
        code: `// 'through' — inclusive: i = 1, 2, 3, 4, 5
@for $i from 1 through 5 {
  .mt-$i { margin-top: calc($i * 4px); }
}

// 'to' — exclusive: i = 1, 2, 3, 4
@for $i from 1 to 5 {
  .col-$i { flex: 0 0 calc($i * 25%); }
}`,
      },
      {
        title: "@each",
        description:
          "Iterate over a comma-separated list. Multiple variables unpack list items in pairs.",
        code: `// Single variable — iterates each item
$breakpoints: sm, md, lg, xl;
@each $bp in $breakpoints {
  .hidden-$bp { display: none; }
}

// Multiple variables — unpacks pairs
$colors: primary #4a90d9, danger #e74c3c, success #2ecc71;
@each $name, $hex in $colors {
  .text-$name { color: $hex; }
  .bg-$name   { background: $hex; }
}`,
      },
    ],
  },
  {
    id: "functions",
    label: "Functions",
    sections: [
      {
        title: "Definition & Return",
        description:
          "Define compile-time functions with @function. Use @return to produce a value.",
        code: `@function spacing($n) {
  @return $n * 8px;
}

@function clamp-val($v, $min, $max) {
  @if $v < $min { @return $min; }
  @if $v > $max { @return $max; }
  @return $v;
}`,
      },
      {
        title: "Calling Functions",
        description:
          "Call a function anywhere a CSS value is expected. Functions run at compile time.",
        code: `.card {
  padding: spacing(2);     // → 16px
  margin: spacing(4);      // → 32px
  font-size: clamp-val(18px, 14px, 24px);  // → 18px
}

@for $i from 1 through 6 {
  .space-$i { margin: spacing($i); }
}`,
      },
      {
        title: "Isolated Scope",
        description:
          "Unlike mixins, functions have isolated scope. They cannot see the caller's variables — only their parameters.",
        code: `$outer: red;

@function get-color() {
  // $outer is NOT visible here — functions have isolated scope
  @return blue;
}

.el {
  color: get-color();  // → blue
}`,
      },
    ],
  },
];

export function SyntaxReference() {
  const [activeTab, setActiveTab] = useState("variables");
  const activeData = TABS.find((t) => t.id === activeTab)!;

  return (
    <section className="syntax-section" id="syntax">
      <div className="section-container">
        <div className="section-header">
          <h2 className="section-title">Syntax reference</h2>
          <p className="section-subtitle">
            Every Lattice feature explained with examples.
          </p>
        </div>

        <div className="syntax-tabs">
          {TABS.map((t) => (
            <button
              key={t.id}
              className={`syntax-tab ${activeTab === t.id ? "active" : ""}`}
              onClick={() => setActiveTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div className="syntax-content">
          {activeData.sections.map((s) => (
            <div key={s.title} className="syntax-row">
              <div className="syntax-desc">
                <h3 className="syntax-heading">{s.title}</h3>
                <p className="syntax-text">{s.description}</p>
              </div>
              <pre className="syntax-code"><code>{s.code}</code></pre>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
