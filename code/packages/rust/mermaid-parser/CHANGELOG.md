# Changelog

## 0.126.0

- Preserve nested Mermaid XY x/y-axis label, title, and spine configuration independently.

## 0.125.0

- Preserve Mermaid XY-chart core init configuration for dimensions, title rendering, bar data labels, and data-label color.

## 0.124.0

- Infer Mermaid's one-based numeric x-axis for category-free XY series and truncate categorical plots to visible bands.

## 0.123.0

- Parse Mermaid 11.16.1 XY line and bar data-point labels into typed semantic chart points.

## 0.122.0

- Parse XY-chart orientation, axis forms, named series, and accessibility metadata through a dedicated Mermaid 11.16.1 grammar.

## 0.115.0

- Resolve GitGraph commit history and validate merge and cherry-pick operations against Mermaid 11.16.1 semantics.

## 0.114.0

- Match Mermaid 11.16.1 branch creation semantics by checking out new GitGraph branches and rejecting duplicates.

## 0.113.0

- Preserve repeated GitGraph commit, merge, and cherry-pick tags in source order.

## 0.112.0

- Parse GitGraph titles and accessibility metadata into temporal semantic IR.

## 0.111.0

- Graduate Requirement diagrams to full Mermaid 11.16.1 compatibility with a pinned upstream acceptance corpus.

## 0.110.0

- Resolve quoted Requirement node and class identifiers in style and class statements without splitting on spaces or quoted commas.

## 0.109.0

- Parse quoted and unquoted multiword Requirement identifiers across definitions, relationships, and shorthand classes.

## 0.108.0

- Parse Requirement headers, statements, fields, enum values, and relationships case-insensitively.

## 0.107.0

- Resolve standalone Requirement `:::` class shorthand into structural styles.

## 0.106.0

- Parse Requirement font size, weight, style, and family into structural semantic styles.

## 0.105.0

- Resolve Requirement class definitions, assignments, default styles, and inline shorthand.

## 0.104.0

- Parse Requirement direct node styles into structural semantic IR.

## 0.103.0

- Parse Requirement accessibility statements into structural semantic IR.

## 0.102.0

- Lower all six Requirement definition kinds into typed semantic metadata.

## 0.101.0

- Parse Requirement fields with dedicated grammar tokens and typed semantic metadata.

## 0.100.0

- Preserve Requirement layout direction in structural IR.

## 0.99.0

- Preserve all seven Requirement relationship semantics and reverse-arrow orientation in structural IR.

## 0.98.0

- Parse core Requirement definitions, elements, and typed relationships into structural IR.

## 0.97.0

- Graduate Journey to full Mermaid 11.16.1 compatibility after pinned corpus and native render coverage.

## 0.96.0

- Parse Journey `leftMargin` and `maxLabelWidth` init options.

## 0.95.0

- Parse Journey `actorColours`, `sectionFills`, and `sectionColours` init arrays.

## 0.94.0

- Parse Journey `titleFontSize`, `titleFontFamily`, and `titleColor` init options.

## 0.93.0

- Parse Journey `taskFontSize` and `taskFontFamily` init options.

## 0.92.0

- Parse Journey numeric geometry options from Mermaid init directives.

## 0.91.0

- Gate the pinned Journey parser corpus and reject task scores outside Mermaid's documented one-to-five domain.

## 0.90.0

- Parse Journey accessibility metadata and normalize upstream HTML break-tag label forms.

## 0.89.0

- Parse the core Journey grammar into typed semantic IR and native dispatch.

## 0.88.0

- Gate quadrant compatibility on the pinned upstream parser and style-validation corpus.
- Match upstream point-style validation and reject malformed input without lexer panics.

## 0.87.0

- Parse all 15 pinned quadrant theme variables for native rendering.

## 0.86.0

- Parse all pinned quadrant font-size and label-padding controls from Mermaid init directives.

## 0.85.0

- Parse quadrant padding and internal/external border widths from Mermaid init directives.

## 0.84.0

- Parse quadrant chart dimensions, axis positions, and default point radius from Mermaid init directives.

## 0.83.0

- Parse inline quadrant comments, empty charts, one-sided axes, dangling arrows, and quoted point labels containing brackets.

## 0.82.0

- Parse case-insensitive quadrant keywords, extended axis arrows, Unicode labels, and Mermaid markdown strings.

## 0.81.0

- Parse quadrant accessibility titles and single-line or multiline descriptions into chart IR.

## 0.80.0

- Resolve quadrant point classes and inline radius/fill/stroke styles into chart IR.

## 0.79.0

- Parse an initial grammar-backed Mermaid 11.16.1 quadrant-chart slice into chart IR.

## 0.78.0

- Lower whitespace-adjacent bare states and single-percent identifiers without treating them as comments.

## 0.77.0

- Parse bare state declarations and normalize all pinned HTML line-break variants in labels and notes before layout.

## 0.76.0

- Attach state notes to composite-group endpoints without creating duplicate ordinary nodes.

## 0.75.0

- Lower Mermaid state `background` and CSS-like solid `border` styles into backend-neutral graph fill, stroke, and stroke-width fields.

## 0.74.0

- Preserve quoted or unquoted state `font-family` declarations in graph IR.

## 0.73.0

- Preserve normal and italic state `font-style` declarations in graph IR.

## 0.72.0

- Preserve normal, bold, and numeric state `font-weight` styles in graph IR.

## 0.71.0

- Preserve state `font-size` styles in graph semantic IR.

## 0.70.0

- Preserve authored colons inside state descriptions and transition labels.

## 0.69.0

- Compose multiple named classes from one state `class` statement in source order.

## 0.68.0

- Preserve trailing descriptions on quoted state aliases as ordered multiline labels.

## 0.67.0

- Ignore pinned state-diagram `#` comments while preserving entities and hexadecimal style colors.

## 0.66.0

- Apply one inline state style statement to comma-delimited nodes and groups.

## 0.65.0

- Decode Mermaid entities and HTML line breaks in state text before layout.

## 0.64.0

- Preserve grammar-backed `hide empty description` rendering semantics.

## 0.63.0

- Preserve grammar-backed state `scale N width` requests in graph IR.

## 0.62.0

- Preserve local `direction` statements on composite state groups.

## 0.61.0

- Preserve repeated state descriptions as ordered multiline labels.

## 0.60.0

- Preserve modern and legacy state diagram titles in graph semantic IR.

## 0.59.0

- Preserve composite state IDs as transition endpoints without synthetic nodes.

## 0.58.0

- Preserve grammar-backed concurrent state regions as ordered group membership.

## 0.57.0

- Parse quoted composite-state aliases and apply inline or named styles to graph groups.

## 0.56.0

- Parse nested composite states into graph-group semantic IR.

## 0.55.0

- Preserve state click URLs and optional tooltips as graph node links.

## 0.54.0

- Preserve state accessibility titles and descriptions in graph semantic IR.

## 0.53.0

- Parse multiline attached notes and quoted floating notes into graph note IR.

## 0.52.0

- Parse single-line state notes into note nodes and note-association edges in graph IR.

## 0.51.0

- Resolve state `:::` class shorthand on standalone states and transition endpoints.

## 0.50.0

- Parse state `classDef` and `class` statements and resolve reusable styles into graph IR.

## 0.49.0

- Parse state inline fill, stroke, text-color, and stroke-width styles into shared graph IR.

## 0.48.0

- Parse Mermaid state fork/join markers and lower them to compact, styled graph-IR bars.

## 0.47.0

- Parse both Mermaid state choice marker spellings and lower choices to graph-IR diamonds.

## 0.46.0

- Parse Mermaid state description statements without a leading `state` keyword and carry their labels through graph IR.

## 0.45.0

- Parse an initial grammar-backed Mermaid 11.16.1 state-diagram slice into graph IR for native layout and Paint lowering.

## 0.44.0

- Blank leading YAML front matter before sequence grammar parsing while preserving source line positions.

## 0.43.0

- Preprocess Mermaid directives before sequence grammar parsing and apply the global `wrap` directive to default-wrapped semantic labels.

## 0.42.0

- Reject `+` and `-` activation suffixes on central sequence connections, matching the pinned Mermaid grammar alternatives.

## 0.37.0

- Reject explicit and message-suffix deactivation of inactive sequence participants.

## 0.36.0

- Reject duplicate participant IDs in `create` declarations and participants assigned to multiple boxes.

## 0.35.0

- Bind `create` and `destroy` declarations to their required following messages and reject invalid lifecycle sequences.

## 0.34.0

- Give start-only sequence `autonumber` statements their Mermaid 11.16.1 default increment of one.

## 0.33.0

- Parse Mermaid 11.16.1 sequence headers and keywords case-insensitively.

## 0.32.0

- Decode Mermaid 11.16.1 YAML/JSON-schema escapes in quoted sequence participant configuration values.

## 0.31.0

- Preserve commas inside quoted sequence participant configuration aliases.

## 0.30.0

- Accept Mermaid 11.16.1 sequence `#` comments without losing entity-coded text.

## 0.29.0

- Preserve ordered sequence `autonumber`, `autonumber off`, and counter-reset statements.

## 0.28.0

- Preserve `wrap:` and `nowrap:` directives on sequence participant-box labels.

## 0.27.0

- Preserve `wrap:` and `nowrap:` directives on sequence participant aliases.

## 0.26.0

- Preserve `wrap:` and `nowrap:` directives on sequence control-block and branch labels.

## 0.25.0

- Preserve hyphens inside sequence actor identifiers without confusing them with message deactivation markers.

## 0.24.0

- Parse multiword sequence actor identifiers across declarations, messages, notes, lifecycle statements, and metadata commands.

## 0.23.0

- Preserve explicit message and note `wrap:` and `nowrap:` semantics.

## 0.22.0

- Convert sequence `<br>`, `<br/>`, and `<br />` tags to semantic newlines.

## 0.21.0

- Decode Mermaid numeric and HTML named entity codes in sequence text.

## 0.20.0

- Parse legacy colon-prefixed sequence titles.

## 0.19.0

- Parse HSL sequence box and rect colors and normalize them to backend-safe RGB.
- Preserve complete functional colors containing internal whitespace.

## 0.18.0

- Accept semicolon terminators between sequence statements and block contents.

## 0.17.0

- Parse multiline sequence accessibility descriptions.

## 0.16.0

- Parse single-line sequence accessibility titles and descriptions.

## 0.15.0

- Parse sequence actor `details` element IDs into semantic IR.

## 0.14.0

- Parse and merge arbitrary JSON-valued sequence actor properties.

## 0.13.0

- Parse singular and JSON-map sequence actor links into semantic IR.

## 0.12.0

- Parse nested `rect` blocks with required `rgb` or `rgba` fills.

## 0.11.0

- Parse Mermaid 11.15+ autonumber start and increment values.

## 0.10.0

- Parse source, destination, and dual sequence central connections.

## 0.9.0

- Parse normal and reverse filled/stick half arrows into sequence IR.

## 0.8.0

- Parse participant `type` and `alias` configuration with external-alias precedence.

## 0.7.0

- Parse Mermaid sequence `box` declarations into participant-group IR.

## 0.6.0

- Added grammar-backed `create participant`, `create actor`, and `destroy` lowering.

## 0.5.0

- Added recursive grammar and semantic lowering for nested sequence control blocks.
- Rejects unterminated blocks instead of silently degrading their contents.

## 0.4.0

- Added grammar-backed sequence parsing for participants, actors, aliases,
  messages, notes, activations, titles, and automatic numbering.
- Added sequence dispatch into the shared semantic IR and marked the family partial.

## 0.3.0

- Pinned the compatibility target to Mermaid 11.16.1.
- Added detection for every documented core diagram family and the external
  ZenUML family.
- Added grammar-backed Pie parsing into the shared chart IR.
- Added a machine-readable compatibility manifest and conformance tests.

## 0.1.0

- Added a grammar-driven Mermaid flowchart parser that lowers into `diagram-ir::GraphDiagram`.
