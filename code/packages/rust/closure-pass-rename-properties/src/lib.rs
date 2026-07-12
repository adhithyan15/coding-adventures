//! Aggressive property renaming pass for the Closure Compiler clone
//! (**ADVANCED**-only) — Closure Compiler's `RENAME_PROPERTIES` in
//! miniature. Consistently shortens program-private object **property
//! names** across the whole program:
//!
//! ```js
//! // before
//! obj.computeTotal = function () {};
//! widget.computeTotal();
//! var cfg = { renderMode: 1 };
//! use(cfg.renderMode);
//!
//! // after rename-properties (ADVANCED)
//! obj.a = function () {};
//! widget.a();
//! var cfg = { b: 1 };
//! use(cfg.b);
//! ```
//!
//! Property access is *by name*: renaming a property name at **every**
//! place it appears keeps the program's meaning, no matter which objects
//! carry it. So `computeTotal` → `a` is applied to every `.computeTotal`
//! dotted access and every unquoted `{ computeTotal: … }` key.
//!
//! # Soundness — the externs / quoting contract
//!
//! Property renaming is sound only under Closure's **externs contract**
//! (the property analogue of the contract the global renamer uses): every
//! property name reachable from *outside* this compilation — a DOM
//! property the browser calls (`onload`), a field another file reads, a
//! key accessed by a dynamically-built string — must be in the
//! do-not-rename boundary. Everything else is private and may be
//! shortened. A name is renamed only when ALL hold:
//!
//!   * **It appears in a renameable (dotted / unquoted-key) position.**
//!     `obj.x` and the key in `{ x: 1 }` are *identifier* positions.
//!   * **It is NOT quoted via a computed string member.** `obj["x"]` is a
//!     *string* position the bridge preserves; the author quoted it to
//!     signal "external / dynamic — leave it alone." A name quoted this
//!     way anywhere is declined everywhere (renaming the dotted
//!     occurrences would desync them from the quoted access we never
//!     touch). **Bridge limitation:** a *quoted object key*
//!     `{ "x": 1 }` is currently collapsed to an identifier key by the
//!     parser bridge, so it is NOT a usable quoting signal — protect such
//!     names via `--externs` instead.
//!   * **It is not a bundled built-in** — neither [`BUILTIN_PROPERTIES`]
//!     (the ECMAScript surface: `length`, `prototype`, `toString`, `push`,
//!     …) nor [`DOM_PROPERTIES`] (the browser/host surface: `innerHTML`,
//!     `addEventListener`, `onclick`, `classList`, …). closurec ships no
//!     externs file, so these two lists are the default-externs substitute
//!     that keeps `arr.length` from becoming `arr.a` and `el.innerHTML`
//!     from becoming `el.a`. They cover the common ECMAScript + DOM surface
//!     out of the box; vendor-/library-specific external properties still
//!     need a `--externs` file, which the [`RenamePropertiesPass::new`]
//!     do-not-rename set unions on top of the bundled lists.
//!   * **It is longer than one character** (already minimal otherwise).
//!
//! **Dynamic computed access (`obj[expr]`) is the author's
//! responsibility**, exactly as in Closure: reaching a renameable
//! property through a runtime-built string requires quoting its
//! definitions (or listing it in externs). We cannot see the runtime
//! string, so this is a documented contract, not something we can check.
//!
//! Each distinct renameable property gets a **distinct** fresh name (no
//! reuse), so renamed properties never collide. Property names live in
//! their own namespace, so a property may be renamed to `a` even when a
//! *variable* `a` exists — the fresh name only avoids other property
//! names, the built-ins, and the do-not-rename set.
//!
//! # Why ADVANCED-only
//!
//! SIMPLE never renames property names (it cannot assume the quoting
//! contract holds). ADVANCED does, which is part of what makes ADVANCED
//! output smaller. This pass runs after the structural passes and the
//! variable renamers.

use std::collections::{HashMap, HashSet};

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_closure_scope_analyzer::program_contains_with_statement;
use coding_adventures_correlation_vector::Contribution;
use coding_adventures_javascript_ast::statement::TaggedStatement;
use serde_json::json;
use coding_adventures_javascript_ast::{
    ArrowBody, AssignmentTarget, ClassMember, Declaration, Expression, ForInit, ObjectMember,
    Program, ProgramItem, PropertyKey, Statement,
};

/// `Pass::depends_on` value — empty. Property renaming is correct
/// standalone.
const DEPS: &[&str] = &[];

/// Reserved words we must never emit as a fresh short name.
const RESERVED: &[&str] = &["do", "if", "in", "of", "as", "is", "or"];

/// Built-in **ECMAScript** property names that must never be renamed — the
/// default-externs substitute (closurec ships no externs file). Renaming
/// any of these would break code that uses the standard library
/// (`arr.length`, `x.toString()`, `p.then(...)`, …). The browser/host
/// surface lives in the companion [`DOM_PROPERTIES`] list; both are always
/// protected, and the user's `--externs` files extend the boundary further.
/// Together they cover the common ECMAScript + DOM surface so the pass is
/// safe by default.
///
/// It is intentionally conservative (over-protecting a user-defined
/// property that happens to share a built-in's name merely forgoes a
/// rename — never a miscompile).
const BUILTIN_PROPERTIES: &[&str] = &[
    // Object / function plumbing
    "prototype",
    "constructor",
    "__proto__",
    "name",
    "length",
    "arguments",
    "caller",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toString",
    "toLocaleString",
    "valueOf",
    "call",
    "apply",
    "bind",
    // Error
    "message",
    "stack",
    "cause",
    // Promise / iterator / generator
    "then",
    "catch",
    "finally",
    "next",
    "return",
    "throw",
    "value",
    "done",
    "resolve",
    "reject",
    "all",
    "race",
    "any",
    "allSettled",
    // Collections
    "size",
    "add",
    "has",
    "get",
    "set",
    "delete",
    "clear",
    "keys",
    "values",
    "entries",
    "forEach",
    // Array
    "push",
    "pop",
    "shift",
    "unshift",
    "slice",
    "splice",
    "concat",
    "join",
    "indexOf",
    "lastIndexOf",
    "includes",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "filter",
    "map",
    "reduce",
    "reduceRight",
    "some",
    "every",
    "sort",
    "reverse",
    "fill",
    "flat",
    "flatMap",
    "copyWithin",
    "at",
    "from",
    "of",
    "isArray",
    // String
    "charAt",
    "charCodeAt",
    "codePointAt",
    "substring",
    "substr",
    "replace",
    "replaceAll",
    "split",
    "trim",
    "trimStart",
    "trimEnd",
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "padStart",
    "padEnd",
    "startsWith",
    "endsWith",
    "repeat",
    "normalize",
    "localeCompare",
    "match",
    "matchAll",
    "search",
    "fromCharCode",
    "fromCodePoint",
    "raw",
    // Number / Math / RegExp / JSON / Date
    "toFixed",
    "toPrecision",
    "toExponential",
    "test",
    "exec",
    "source",
    "flags",
    "global",
    "ignoreCase",
    "multiline",
    "lastIndex",
    "parse",
    "stringify",
    "now",
    "getTime",
    "getFullYear",
    "getMonth",
    "getDate",
    "getDay",
    "getHours",
    "getMinutes",
    "getSeconds",
    "getMilliseconds",
    "setFullYear",
    "toISOString",
    "toJSON",
    // Console / common host
    "log",
    "warn",
    "error",
    "info",
    "debug",
    "assert",
];

/// Curated **DOM / host** property names that must never be renamed — the
/// part of the external boundary the bundled [`BUILTIN_PROPERTIES`]
/// (ECMAScript only) used to omit. closurec ships no browser externs file,
/// so renaming `el.innerHTML`, `node.addEventListener`, or `btn.onclick`
/// would silently break browser code that the host (the DOM, the event
/// system, CSSOM, the fetch/XHR stack, storage, …) reads or writes by name.
///
/// Like [`BUILTIN_PROPERTIES`] this list is **always protected** in addition
/// to the externs do-not-rename set, and is deliberately conservative:
/// over-protecting a user-defined property that happens to share a DOM name
/// merely forgoes a rename — never a miscompile. It cannot be exhaustive
/// (host surfaces evolve and vendor-/library-specific properties exist), so
/// a `--externs` file remains the authoritative boundary; this bundle is the
/// safety net that covers the common browser surface out of the box.
///
/// Grouped by host area for auditability. Names already covered by
/// [`BUILTIN_PROPERTIES`] (e.g. `length`, `value`, `name`) are not repeated;
/// the two lists are unioned where the protected set is built.
const DOM_PROPERTIES: &[&str] = &[
    // ---- EventTarget / events ----
    "addEventListener",
    "removeEventListener",
    "dispatchEvent",
    "preventDefault",
    "stopPropagation",
    "stopImmediatePropagation",
    "target",
    "currentTarget",
    "relatedTarget",
    "srcElement",
    "type",
    "bubbles",
    "cancelable",
    "composed",
    "defaultPrevented",
    "eventPhase",
    "isTrusted",
    "timeStamp",
    "detail",
    "key",
    "code",
    "keyCode",
    "charCode",
    "which",
    "altKey",
    "ctrlKey",
    "shiftKey",
    "metaKey",
    "button",
    "buttons",
    "clientX",
    "clientY",
    "screenX",
    "screenY",
    "pageX",
    "pageY",
    "offsetX",
    "offsetY",
    "movementX",
    "movementY",
    "deltaX",
    "deltaY",
    "deltaZ",
    "deltaMode",
    "touches",
    "targetTouches",
    "changedTouches",
    "pointerId",
    "pointerType",
    "pressure",
    // ---- common inline event handlers (on*) ----
    "onclick",
    "ondblclick",
    "onmousedown",
    "onmouseup",
    "onmousemove",
    "onmouseover",
    "onmouseout",
    "onmouseenter",
    "onmouseleave",
    "oncontextmenu",
    "onwheel",
    "onkeydown",
    "onkeyup",
    "onkeypress",
    "onfocus",
    "onblur",
    "onfocusin",
    "onfocusout",
    "onchange",
    "oninput",
    "oninvalid",
    "onsubmit",
    "onreset",
    "onselect",
    "onload",
    "onunload",
    "onbeforeunload",
    "onerror",
    "onresize",
    "onscroll",
    "onhashchange",
    "onpopstate",
    "onpageshow",
    "onpagehide",
    "onreadystatechange",
    "ondomcontentloaded",
    "onanimationstart",
    "onanimationend",
    "onanimationiteration",
    "ontransitionend",
    "ondragstart",
    "ondrag",
    "ondragend",
    "ondragenter",
    "ondragover",
    "ondragleave",
    "ondrop",
    "ontouchstart",
    "ontouchmove",
    "ontouchend",
    "ontouchcancel",
    "onpointerdown",
    "onpointerup",
    "onpointermove",
    "onpointerenter",
    "onpointerleave",
    "onmessage",
    "onopen",
    "onclose",
    // ---- Node / Element ----
    "nodeType",
    "nodeName",
    "nodeValue",
    "textContent",
    "innerHTML",
    "outerHTML",
    "innerText",
    "outerText",
    "tagName",
    "localName",
    "namespaceURI",
    "id",
    "className",
    "classList",
    "attributes",
    "dataset",
    "style",
    "title",
    "lang",
    "dir",
    "hidden",
    "tabIndex",
    "contentEditable",
    "isContentEditable",
    "draggable",
    "spellcheck",
    "accessKey",
    "parentNode",
    "parentElement",
    "childNodes",
    "children",
    "firstChild",
    "lastChild",
    "firstElementChild",
    "lastElementChild",
    "nextSibling",
    "previousSibling",
    "nextElementSibling",
    "previousElementSibling",
    "childElementCount",
    "ownerDocument",
    "shadowRoot",
    "assignedSlot",
    "appendChild",
    "removeChild",
    "replaceChild",
    "insertBefore",
    "cloneNode",
    "contains",
    "compareDocumentPosition",
    "normalize",
    "append",
    "prepend",
    "before",
    "after",
    "replaceWith",
    "remove",
    "insertAdjacentHTML",
    "insertAdjacentText",
    "insertAdjacentElement",
    "getAttribute",
    "setAttribute",
    "removeAttribute",
    "hasAttribute",
    "hasAttributes",
    "getAttributeNS",
    "setAttributeNS",
    "removeAttributeNS",
    "toggleAttribute",
    "getAttributeNames",
    "querySelector",
    "querySelectorAll",
    "getElementById",
    "getElementsByClassName",
    "getElementsByTagName",
    "getElementsByName",
    "closest",
    "matches",
    "getBoundingClientRect",
    "getClientRects",
    "scrollIntoView",
    "scrollTo",
    "scrollBy",
    "focus",
    "blur",
    "click",
    "clientWidth",
    "clientHeight",
    "clientLeft",
    "clientTop",
    "offsetWidth",
    "offsetHeight",
    "offsetLeft",
    "offsetTop",
    "offsetParent",
    "scrollWidth",
    "scrollHeight",
    "scrollLeft",
    "scrollTop",
    // ---- classList / DOMTokenList ----
    "toggle",
    "replace",
    "item",
    // ---- form / input ----
    "checked",
    "selected",
    "disabled",
    "readOnly",
    "required",
    "multiple",
    "placeholder",
    "defaultValue",
    "defaultChecked",
    "selectedIndex",
    "selectedOptions",
    "options",
    "elements",
    "form",
    "files",
    "validity",
    "validationMessage",
    "willValidate",
    "checkValidity",
    "reportValidity",
    "setCustomValidity",
    "setSelectionRange",
    "select",
    "submit",
    "reset",
    "labels",
    "htmlFor",
    "action",
    "method",
    "enctype",
    "autocomplete",
    "autofocus",
    "maxLength",
    "minLength",
    "min",
    "max",
    "step",
    "pattern",
    // ---- attributes commonly read by name ----
    "href",
    "src",
    "srcset",
    "alt",
    "rel",
    "media",
    "content",
    "width",
    "height",
    "rows",
    "cols",
    "colSpan",
    "rowSpan",
    "cellPadding",
    "cellSpacing",
    "crossOrigin",
    "referrerPolicy",
    "loading",
    "decoding",
    "currentSrc",
    "naturalWidth",
    "naturalHeight",
    "complete",
    // ---- CSSStyleDeclaration ----
    "cssText",
    "getPropertyValue",
    "setProperty",
    "removeProperty",
    "getPropertyPriority",
    // ---- Document ----
    "documentElement",
    "head",
    "body",
    "title",
    "URL",
    "documentURI",
    "domain",
    "referrer",
    "cookie",
    "readyState",
    "characterSet",
    "contentType",
    "createElement",
    "createElementNS",
    "createTextNode",
    "createComment",
    "createDocumentFragment",
    "createEvent",
    "createRange",
    "createTreeWalker",
    "importNode",
    "adoptNode",
    "write",
    "writeln",
    "execCommand",
    "elementFromPoint",
    "getSelection",
    "hasFocus",
    "activeElement",
    "defaultView",
    "scrollingElement",
    "visibilityState",
    "hidden",
    // ---- Window ----
    "document",
    "location",
    "navigator",
    "history",
    "screen",
    "frames",
    "parent",
    "top",
    "self",
    "window",
    "opener",
    "frameElement",
    "innerWidth",
    "innerHeight",
    "outerWidth",
    "outerHeight",
    "scrollX",
    "scrollY",
    "pageXOffset",
    "pageYOffset",
    "devicePixelRatio",
    "localStorage",
    "sessionStorage",
    "performance",
    "crypto",
    "console",
    "alert",
    "confirm",
    "prompt",
    "open",
    "close",
    "print",
    "focus",
    "blur",
    "scroll",
    "moveTo",
    "moveBy",
    "resizeTo",
    "resizeBy",
    "requestAnimationFrame",
    "cancelAnimationFrame",
    "requestIdleCallback",
    "cancelIdleCallback",
    "setTimeout",
    "clearTimeout",
    "setInterval",
    "clearInterval",
    "getComputedStyle",
    "matchMedia",
    "postMessage",
    "fetch",
    "btoa",
    "atob",
    "structuredClone",
    "queueMicrotask",
    "getSelection",
    // ---- Location ----
    "protocol",
    "host",
    "hostname",
    "port",
    "pathname",
    "search",
    "hash",
    "origin",
    "username",
    "password",
    "assign",
    "reload",
    "toString",
    // ---- History ----
    "pushState",
    "replaceState",
    "go",
    "back",
    "forward",
    "scrollRestoration",
    // ---- Storage ----
    "getItem",
    "setItem",
    "removeItem",
    "clear",
    "key",
    // ---- Navigator ----
    "userAgent",
    "platform",
    "language",
    "languages",
    "onLine",
    "cookieEnabled",
    "hardwareConcurrency",
    "deviceMemory",
    "maxTouchPoints",
    "sendBeacon",
    "vibrate",
    "clipboard",
    "geolocation",
    "mediaDevices",
    "serviceWorker",
    "permissions",
    // ---- XHR / fetch / Response ----
    "responseType",
    "responseText",
    "responseXML",
    "response",
    "status",
    "statusText",
    "withCredentials",
    "timeout",
    "readyState",
    "send",
    "abort",
    "getResponseHeader",
    "getAllResponseHeaders",
    "setRequestHeader",
    "overrideMimeType",
    "ok",
    "redirected",
    "headers",
    "url",
    "body",
    "bodyUsed",
    "arrayBuffer",
    "blob",
    "formData",
    "clone",
    "text",
    "json",
    // ---- CustomEvent / dataTransfer ----
    "dataTransfer",
    "effectAllowed",
    "dropEffect",
    "setData",
    "getData",
    "clearData",
    "setDragImage",
];

/// Aggressive property renaming pass. Holds the **do-not-rename set** of
/// property names supplied at construction (typically the property names
/// collected from `--externs` files); the built-in property list is
/// always protected on top of it.
#[derive(Debug, Default, Clone)]
pub struct RenamePropertiesPass {
    do_not_rename: HashSet<String>,
}

impl RenamePropertiesPass {
    /// Construct with an externs property do-not-rename set (extends the
    /// always-protected [`BUILTIN_PROPERTIES`]).
    pub fn new(do_not_rename: HashSet<String>) -> Self {
        Self { do_not_rename }
    }

    /// Construct protecting only the built-in properties (no extra
    /// externs property names).
    pub fn with_builtins_only() -> Self {
        Self {
            do_not_rename: HashSet::new(),
        }
    }
}

impl Pass for RenamePropertiesPass {
    fn name(&self) -> &'static str {
        "rename-properties"
    }

    fn depends_on(&self) -> &[&'static str] {
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // After one whole-program walk every renameable property has been
        // shortened; re-running does nothing.
        IterationPolicy::OneShot
    }

    fn cost(&self) -> u32 {
        // Two whole-program walks: classify (dotted vs quoted) + rewrite.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // `with` soundness gate (CLOC12.187 PR2a). Inside `with (obj) …` a bare
        // name like `foo` may be the property access `obj.foo` in disguise.
        // Property renaming rewrites `.foo` member accesses and object-literal
        // keys consistently, but it cannot see that a *bare* `foo` in a `with`
        // body is really a property read — so renaming `foo` elsewhere would
        // desynchronize from that disguised access. When the program contains a
        // `with` we therefore decline to rename properties and return the input
        // unchanged. `with` is rare (a strict-mode syntax error), so this costs
        // little. See [`program_contains_with_statement`].
        if program_contains_with_statement(ctx.program) {
            return Ok(PassOutput {
                program: ctx.program.clone(),
                contributions: Vec::new(),
                changed: false,
                diagnostics: Vec::new(),
                stats: PassStats { nodes_touched: 1 },
            });
        }

        let mut program = ctx.program.clone();
        let mut nodes_touched: u32 = 1;
        let (changed, renames) =
            rename_properties(&mut program, &self.do_not_rename, &mut nodes_touched);

        // CV provenance (#89): record every property rename as a `renamed`
        // contribution carrying `{from, to}`. The pipeline attaches these
        // to the program-root CV entry, so a `--correlation_vector`
        // consumer can map a minified property (`o.a`) back to its
        // original name (`o.longProp`) — provenance that renaming would
        // otherwise erase. Mirrors the rename-globals pass. (Per-node
        // span attachment — contributing to each renamed property
        // occurrence's own CV id — is a documented follow-up that needs
        // the log threaded through the `rewrite_*` recursion.)
        let contributions: Vec<Contribution> = renames
            .into_iter()
            .map(|(from, to)| Contribution {
                source: "rename-properties".to_string(),
                tag: "renamed".to_string(),
                meta: [
                    ("from".to_string(), json!(from)),
                    ("to".to_string(), json!(to)),
                ]
                .into_iter()
                .collect(),
            })
            .collect();

        Ok(PassOutput {
            program,
            contributions,
            changed,
            diagnostics: Vec::new(),
            stats: PassStats { nodes_touched },
        })
    }
}

// =========================================================================
// Implementation
// =========================================================================

/// The per-name evidence gathered in the classification walk.
#[derive(Default)]
struct Classify {
    /// Names seen in a renameable (dotted / unquoted-key) position, in
    /// first-seen source order (deterministic fresh-name assignment).
    dotted_order: Vec<String>,
    dotted_seen: HashSet<String>,
    /// Names seen in a quoted position (`obj["x"]` / `{ "x": 1 }`) — these
    /// are off-limits and disable the name everywhere.
    quoted: HashSet<String>,
}

impl Classify {
    fn see_dotted(&mut self, name: &str) {
        if self.dotted_seen.insert(name.to_string()) {
            self.dotted_order.push(name.to_string());
        }
    }
    fn see_quoted(&mut self, name: &str) {
        self.quoted.insert(name.to_string());
    }
}

/// Renames qualifying dotted property names to fresh short names.
///
/// Returns `(changed, renames)` where `renames` is the applied rename
/// table as `(from, to)` pairs sorted by original name (deterministic
/// order for stable CV provenance). `renames` is empty exactly when
/// `changed` is `false`.
fn rename_properties(
    program: &mut Program,
    do_not_rename: &HashSet<String>,
    nodes_touched: &mut u32,
) -> (bool, Vec<(String, String)>) {
    // 1. Classify every property occurrence as dotted (renameable shape)
    //    or quoted (off-limits).
    let mut cls = Classify::default();
    for item in &program.body {
        classify_item(item, &mut cls, nodes_touched);
    }

    // 2. Decide the renames. A property is renameable when it appears
    //    dotted, never quoted, is not a built-in (ECMAScript OR DOM/host),
    //    is not in the externs do-not-rename set, and is longer than one
    //    character. The protected baseline is the union of the bundled
    //    ECMAScript and DOM/host property lists.
    let builtins: HashSet<&str> = BUILTIN_PROPERTIES
        .iter()
        .chain(DOM_PROPERTIES.iter())
        .copied()
        .collect();
    // Fresh names avoid every property name in the program plus the
    // built-ins and externs set (property namespace only — variable names
    // are irrelevant).
    let mut avoid: HashSet<String> = HashSet::new();
    avoid.extend(cls.dotted_seen.iter().cloned());
    avoid.extend(cls.quoted.iter().cloned());
    avoid.extend(builtins.iter().map(|s| s.to_string()));
    avoid.extend(do_not_rename.iter().cloned());

    let mut map: HashMap<String, String> = HashMap::new();
    let mut gen = FreshNames::new();
    for name in &cls.dotted_order {
        if cls.quoted.contains(name)
            || builtins.contains(name.as_str())
            || do_not_rename.contains(name)
            || name.len() <= 1
        {
            continue;
        }
        let fresh = gen.next(&avoid);
        avoid.insert(fresh.clone());
        map.insert(name.clone(), fresh);
    }

    if map.is_empty() {
        return (false, Vec::new());
    }

    // 3. Rewrite every dotted / unquoted-key occurrence of a renamed name.
    for item in &mut program.body {
        rewrite_item(item, &map);
    }

    // The rename table drives CV provenance (#89). Sort by original name
    // so the emitted contributions are deterministic run to run.
    let mut renames: Vec<(String, String)> = map.into_iter().collect();
    renames.sort();
    (true, renames)
}

/// Collect **every property name that appears anywhere** in `program` —
/// the property-namespace boundary an externs file declares.
///
/// This is the property-renaming analogue of collecting an externs file's
/// top-level variable/function names (the *value*-namespace boundary). A
/// driver that wants to feed an externs file's properties into a
/// [`RenamePropertiesPass`] `do_not_rename` set walks each externs program
/// through this function and unions the results.
///
/// We return the **union of dotted and quoted occurrences**, deliberately
/// over-collecting:
///
/// * `el.innerHTML` (dotted)        → `innerHTML`
/// * `obj["data-id"]` (quoted)      → `data-id`
/// * `{ onload: f }` (unquoted key) → `onload`
/// * `{ "aria-label": s }` (quoted) → `aria-label`
///
/// Why every occurrence, not just the renameable (dotted) ones? Because an
/// externs file is a *declaration of the external boundary*: any property
/// it names is part of the host/library contract and must be preserved in
/// the program being compiled. Including quoted names too only ever
/// *protects more* — forgoing a rename is never a miscompile, whereas
/// renaming a genuinely external property is. (Computed dynamic keys like
/// `obj[runtimeExpr]` contribute nothing — there is no static name to
/// protect; that access is the author's own contract, exactly as in the
/// pass itself.)
///
/// ```
/// use coding_adventures_closure_pass_rename_properties::collect_property_names;
/// use coding_adventures_javascript_ast::{Program, SourceType};
/// use coding_adventures_javascript_tokens::EsVersion;
/// // An empty externs program declares no property boundary.
/// let empty = Program::new("ext.1".to_string(), EsVersion::Es2025, SourceType::Module);
/// assert!(collect_property_names(&empty).is_empty());
/// ```
pub fn collect_property_names(program: &Program) -> HashSet<String> {
    let mut cls = Classify::default();
    let mut nodes_touched: u32 = 0;
    for item in &program.body {
        classify_item(item, &mut cls, &mut nodes_touched);
    }
    // The union of both buckets: dotted (renameable-shape) and quoted
    // (off-limits-shape) occurrences. As an externs boundary, both kinds
    // are equally external and must be protected.
    let mut names = cls.dotted_seen;
    names.extend(cls.quoted);
    names
}

/// Generates `a`, `b`, …, `z`, `aa`, … skipping reserved words and the
/// caller's `avoid` set.
struct FreshNames {
    counter: usize,
}

impl FreshNames {
    fn new() -> Self {
        FreshNames { counter: 0 }
    }
    fn next(&mut self, avoid: &HashSet<String>) -> String {
        loop {
            let name = encode(self.counter);
            self.counter += 1;
            if !RESERVED.contains(&name.as_str()) && !avoid.contains(&name) {
                return name;
            }
        }
    }
}

/// Bijective base-26 encoding: 0→a, 25→z, 26→aa, …
fn encode(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'a' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).expect("ascii")
}

// ---- classification ------------------------------------------------------

fn classify_item(item: &ProgramItem, cls: &mut Classify, nodes_touched: &mut u32) {
    match item {
        ProgramItem::Declaration(d) => classify_decl(d, cls, nodes_touched),
        ProgramItem::Statement(s) => classify_stmt(s, cls, nodes_touched),
    }
}

fn classify_decl(decl: &Declaration, cls: &mut Classify, nodes_touched: &mut u32) {
    *nodes_touched += 1;
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                if let Some(init) = &d.init {
                    classify_expr(init, cls);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &fd.body.body {
                classify_stmt(s, cls, nodes_touched);
            }
        }
        // A class *declaration* classifies its members exactly like a class
        // *expression* — the class name is a variable binding, not a property,
        // so only the member keys + bodies matter.
        Declaration::ClassDeclaration(cd) => classify_class_members(&cd.super_class, &cd.body, cls),
        // An import declaration has no property accesses/keys to classify.
        Declaration::ImportDeclaration(_) => {}
    }
}

/// Classify the property names inside the shared `[extends S] { members }` tail
/// of a class — reused by the class *expression* arm of [`classify_expr`] and
/// the class *declaration* arm of [`classify_decl`], which share their body
/// shape. Each non-computed method key is recorded as a property name (with the
/// `constructor` keyword pinned as un-renameable), computed-key expressions are
/// recursed, and each method body is classified. The class's own name is a
/// variable, not a property, so it never enters here.
fn classify_class_members(
    super_class: &Option<Box<Expression>>,
    body: &[ClassMember],
    cls: &mut Classify,
) {
    if let Some(sup) = super_class {
        classify_expr(sup, cls);
    }
    for member in body {
        match member {
            ClassMember::Method(m) => {
                if !m.computed {
                    match &m.key {
                        // `constructor` is a class-semantic keyword-as-key, NOT
                        // an ordinary property: renaming it would turn the
                        // constructor into a plain prototype method and the
                        // class would silently get an implicit ctor — `new C(x)`
                        // would stop initialising (miscompile). Pin it via the
                        // same channel as a quoted key so it is never eligible
                        // for renaming anywhere.
                        PropertyKey::Identifier(id) if id.name == "constructor" => {
                            cls.see_quoted("constructor")
                        }
                        PropertyKey::Identifier(id) => cls.see_dotted(&id.name),
                        PropertyKey::StringLiteral(s) => cls.see_quoted(&s.value),
                        _ => {}
                    }
                } else if let PropertyKey::Expression(e) = &m.key {
                    // A computed key `[expr]` is a dynamic access — no static
                    // name recorded, but recurse for nested property accesses
                    // inside the key expression.
                    classify_expr(e, cls);
                }
                let mut nested = 0u32;
                for s in &m.value.body.body {
                    classify_stmt(s, cls, &mut nested);
                }
            }
            // A class field's KEY is a renameable property name, exactly like a
            // method key — but a field is NEVER the constructor, so there is no
            // `constructor` pin here. A computed key + the initializer value
            // may contain nested property accesses, so recurse both.
            ClassMember::Field(f) => {
                if !f.computed {
                    match &f.key {
                        PropertyKey::Identifier(id) => cls.see_dotted(&id.name),
                        PropertyKey::StringLiteral(s) => cls.see_quoted(&s.value),
                        _ => {}
                    }
                } else if let PropertyKey::Expression(e) = &f.key {
                    classify_expr(e, cls);
                }
                if let Some(v) = &f.value {
                    classify_expr(v, cls);
                }
            }
            // A static-init block has NO key (nothing renameable as a property
            // name), but its statements may contain property accesses — classify
            // each, mirroring the method-body recursion.
            ClassMember::StaticBlock(b) => {
                let mut nested = 0u32;
                for s in &b.body {
                    classify_stmt(s, cls, &mut nested);
                }
            }
        }
    }
}

fn classify_stmt(stmt: &Statement, cls: &mut Classify, nodes_touched: &mut u32) {
    *nodes_touched += 1;
    match stmt {
        Statement::Declaration(d) => classify_decl(d, cls, nodes_touched),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => classify_expr(&es.expression, cls),
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    classify_stmt(s, cls, nodes_touched);
                }
            }
            TaggedStatement::IfStatement(is) => {
                classify_expr(&is.test, cls);
                classify_stmt(&is.consequent, cls, nodes_touched);
                if let Some(alt) = &is.alternate {
                    classify_stmt(alt, cls, nodes_touched);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                classify_expr(&ws.test, cls);
                classify_stmt(&ws.body, cls, nodes_touched);
            }
            // `with (o) body` (CLOC12.187) — classify the object and body.
            TaggedStatement::WithStatement(ws) => {
                classify_expr(&ws.object, cls);
                classify_stmt(&ws.body, cls, nodes_touched);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                classify_expr(&ds.test, cls);
                classify_stmt(&ds.body, cls, nodes_touched);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(i) = &d.init {
                                    classify_expr(i, cls);
                                }
                            }
                        }
                        ForInit::Expression(e) => classify_expr(e, cls),
                    }
                }
                if let Some(test) = &fs.test {
                    classify_expr(test, cls);
                }
                if let Some(update) = &fs.update {
                    classify_expr(update, cls);
                }
                classify_stmt(&fs.body, cls, nodes_touched);
            }
            TaggedStatement::ForInStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                classify_expr(i, cls);
                            }
                        }
                    }
                    ForInit::Expression(e) => classify_expr(e, cls),
                }
                classify_expr(&fs.right, cls);
                classify_stmt(&fs.body, cls, nodes_touched);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                classify_expr(i, cls);
                            }
                        }
                    }
                    ForInit::Expression(e) => classify_expr(e, cls),
                }
                classify_expr(&fs.right, cls);
                classify_stmt(&fs.body, cls, nodes_touched);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &rs.argument {
                    classify_expr(a, cls);
                }
            }
            TaggedStatement::ThrowStatement(ts) => classify_expr(&ts.argument, cls),
            TaggedStatement::LabeledStatement(ls) => classify_stmt(&ls.body, cls, nodes_touched),
            TaggedStatement::SwitchStatement(ss) => {
                classify_expr(&ss.discriminant, cls);
                for c in &ss.cases {
                    if let Some(test) = &c.test {
                        classify_expr(test, cls);
                    }
                    for s in &c.consequent {
                        classify_stmt(s, cls, nodes_touched);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // Property renaming is variable-agnostic — recurse into the
                // three blocks; the catch `param` is a variable binding, not a
                // property, so nothing special is needed.
                for s in &ts.block.body {
                    classify_stmt(s, cls, nodes_touched);
                }
                if let Some(h) = &ts.handler {
                    for s in &h.body.body {
                        classify_stmt(s, cls, nodes_touched);
                    }
                }
                if let Some(f) = &ts.finalizer {
                    for s in &f.body {
                        classify_stmt(s, cls, nodes_touched);
                    }
                }
            }
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

fn classify_expr(expr: &Expression, cls: &mut Classify) {
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // A regex literal (like `/ab+c/g`) is an inert leaf: no sub-expressions
        // and no property names, so it is grouped with the other literals.
        | Expression::RegExpLiteral(_)
        // `this` holds no property name to classify or rewrite.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            classify_expr(&be.left, cls);
            classify_expr(&be.right, cls);
        }
        Expression::LogicalExpression(le) => {
            classify_expr(&le.left, cls);
            classify_expr(&le.right, cls);
        }
        Expression::UnaryExpression(ue) => classify_expr(&ue.argument, cls),
        Expression::UpdateExpression(ue) => classify_expr(&ue.argument, cls),
        Expression::AssignmentExpression(ae) => {
            if let AssignmentTarget::MemberExpression(m) = &ae.left {
                classify_member(&m.object, &m.property, m.computed, cls);
            }
            classify_expr(&ae.right, cls);
        }
        Expression::ConditionalExpression(ce) => {
            classify_expr(&ce.test, cls);
            classify_expr(&ce.consequent, cls);
            classify_expr(&ce.alternate, cls);
        }
        Expression::CallExpression(ce) => {
            classify_expr(&ce.callee, cls);
            for a in &ce.arguments {
                classify_expr(a, cls);
            }
        }
        Expression::NewExpression(ne) => {
            classify_expr(&ne.callee, cls);
            for a in &ne.arguments {
                classify_expr(a, cls);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                classify_expr(e, cls);
            }
        }
        Expression::MemberExpression(m) => classify_member(&m.object, &m.property, m.computed, cls),
        // `a?.b` / `a?.[k]` — the optional short-circuit does not change which
        // property name is accessed, so it classifies exactly like a plain
        // member access.
        Expression::OptionalMemberExpression(m) => {
            classify_member(&m.object, &m.property, m.computed, cls)
        }
        // `a?.()` — classify callee and each argument, as for an ordinary call.
        Expression::OptionalCallExpression(ce) => {
            classify_expr(&ce.callee, cls);
            for a in &ce.arguments {
                classify_expr(a, cls);
            }
        }
        // A chain expression transparently wraps its optional-chain spine —
        // descend into the inner expression.
        Expression::ChainExpression(c) => classify_expr(&c.expression, cls),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                classify_expr(el, cls);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        if prop.computed {
                            // `{ [expr]: v }` — Phase-1 rejects this at the bridge,
                            // but be defensive: recurse the key, record nothing.
                            if let PropertyKey::Expression(e) = &prop.key {
                                classify_expr(e, cls);
                            }
                        } else {
                            match &prop.key {
                                // `{ x: v }` — a renameable dotted key.
                                PropertyKey::Identifier(id) => cls.see_dotted(&id.name),
                                // `{ "x": v }` — a quoted key disables `x`.
                                PropertyKey::StringLiteral(s) => cls.see_quoted(&s.value),
                                // Numeric keys / others — not renameable identifiers.
                                _ => {}
                            }
                        }
                        classify_expr(&prop.value, cls);
                    }
                    // `{ ...expr }` — a spread has no property NAME, so it
                    // touches no property namespace. Only walk its argument
                    // sub-expression so a quoted access inside it still
                    // disables the affected name.
                    ObjectMember::Spread(s) => {
                        classify_expr(&s.argument, cls);
                    }
                }
            }
        }
        // Classify property accesses inside a function *value*'s body — a
        // quoted `o["foo"]` written there must still DISABLE renaming of
        // `foo`, so we cannot skip the body (that would risk an unsound
        // rename). The fn's name/params are variable bindings, never
        // property names, so they don't touch the property namespace.
        // `nodes_touched` counts statements/declarations for stats only;
        // `classify_expr` isn't threaded it, so a throwaway counter is
        // used for the nested walk.
        Expression::FunctionExpression(fe) => {
            let mut nested = 0u32;
            for s in &fe.body.body {
                classify_stmt(s, cls, &mut nested);
            }
        }
        // A class expression. Two namespaces meet here:
        //
        //   * Each method's KEY *defines* a property on the class/prototype,
        //     so it classifies exactly like an object-literal key (the
        //     `ObjectExpression` arm above): an unquoted `foo() {}` is a
        //     renameable dotted property (`see_dotted`) that must move in
        //     lock-step with any `c.foo` access, while a quoted `"foo"() {}`
        //     DISABLES renaming of `foo` (`see_quoted`).
        //   * Each method's VALUE body is a function scope, walked for nested
        //     property accesses exactly like the `FunctionExpression` arm
        //     (a quoted `o["bar"]` inside a method body must still disable
        //     `bar`). The method params/self-name are variable bindings, not
        //     property names.
        //
        // The `extends` operand is an ordinary expression — recurse into it.
        Expression::ClassExpression(ce) => classify_class_members(&ce.super_class, &ce.body, cls),
        // Classify property accesses inside an arrow-value's body too —
        // a quoted `o["foo"]` written there must still disable renaming
        // of `foo`. Params are variable names, never property names, so
        // they don't touch the property namespace.
        Expression::ArrowFunctionExpression(ae) => match &ae.body {
            ArrowBody::Block(b) => {
                let mut nested = 0u32;
                for s in &b.body {
                    classify_stmt(s, cls, &mut nested);
                }
            }
            ArrowBody::Expression(e) => classify_expr(e, cls),
        },
        // Classify property accesses inside each `${…}` insert too. Quasis
        // are leaf strings — only the insert expressions can hold a member
        // access that touches the property namespace.
        Expression::TemplateLiteral(t) => {
            for e in &t.expressions {
                classify_expr(e, cls);
            }
        }
        // A member access touching the property namespace can hide in the tag
        // callee or a `${…}` insert. Quasis are leaf strings — nothing to walk.
        Expression::TaggedTemplateExpression(t) => {
            classify_expr(&t.tag, cls);
            for e in &t.quasi.expressions {
                classify_expr(e, cls);
            }
        }
        // `...arg` — recurse into the spread argument to classify accesses in it.
        Expression::SpreadElement(s) => classify_expr(&s.argument, cls),
        Expression::YieldExpression(y) => { if let Some(a) = &y.argument { classify_expr(a, cls); } }
        Expression::AwaitExpression(a) => classify_expr(&a.argument, cls),
        Expression::ImportExpression(e) => classify_expr(&e.source, cls),
    }
}

fn classify_member(object: &Expression, property: &Expression, computed: bool, cls: &mut Classify) {
    classify_expr(object, cls);
    if computed {
        // `obj["x"]` — a quoted access disables `x`. Any other computed
        // key is a dynamic access (the author's contract responsibility);
        // we still recurse it for nested property accesses.
        if let Expression::StringLiteral(s) = property {
            cls.see_quoted(&s.value);
        } else {
            classify_expr(property, cls);
        }
    } else if let Expression::Identifier(id) = property {
        // `obj.x` — a renameable dotted access.
        cls.see_dotted(&id.name);
    }
}

// ---- rewrite -------------------------------------------------------------

fn rewrite_item(item: &mut ProgramItem, map: &HashMap<String, String>) {
    match item {
        ProgramItem::Declaration(d) => rewrite_decl(d, map),
        ProgramItem::Statement(s) => rewrite_stmt(s, map),
    }
}

fn rewrite_decl(decl: &mut Declaration, map: &HashMap<String, String>) {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &mut vd.declarations {
                if let Some(init) = &mut d.init {
                    rewrite_expr(init, map);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &mut fd.body.body {
                rewrite_stmt(s, map);
            }
        }
        // A class *declaration* rewrites its members exactly like a class
        // *expression* (the class name is a variable, untouched by property
        // renaming) — same shared helper, kept in lockstep with `classify_decl`.
        Declaration::ClassDeclaration(cd) => {
            rewrite_class_members(&mut cd.super_class, &mut cd.body, map)
        }
        // An import declaration has no property accesses/keys to rewrite.
        Declaration::ImportDeclaration(_) => {}
    }
}

/// Rewrite the property keys inside the shared `[extends S] { members }` tail of
/// a class — the mirror of [`classify_class_members`], reused by both the class
/// *expression* and *declaration* arms so the classification and the rewrite
/// cover exactly the same positions. Renames each non-computed method key found
/// in `map` (never `constructor`), recurses computed-key expressions, and
/// rewrites each method body.
fn rewrite_class_members(
    super_class: &mut Option<Box<Expression>>,
    body: &mut [ClassMember],
    map: &HashMap<String, String>,
) {
    if let Some(sup) = super_class {
        rewrite_expr(sup, map);
    }
    for member in body {
        match member {
            ClassMember::Method(m) => {
                if m.computed {
                    if let PropertyKey::Expression(e) = &mut m.key {
                        rewrite_expr(e, map);
                    }
                } else if let PropertyKey::Identifier(id) = &mut m.key {
                    // Never rewrite a `constructor` key (see classify: it is
                    // `see_quoted`-pinned, so it is not in `map` anyway — this
                    // guard is belt-and-braces against a future map that might
                    // contain it, since renaming it is a construction-semantics
                    // miscompile).
                    if id.name != "constructor" {
                        if let Some(new) = map.get(&id.name) {
                            id.name = new.clone();
                        }
                    }
                }
                for s in &mut m.value.body.body {
                    rewrite_stmt(s, map);
                }
            }
            // Rewrite a field key (a renameable property name, no `constructor`
            // pin) and recurse the computed key + initializer, kept in lockstep
            // with `classify_class_members`.
            ClassMember::Field(f) => {
                if f.computed {
                    if let PropertyKey::Expression(e) = &mut f.key {
                        rewrite_expr(e, map);
                    }
                } else if let PropertyKey::Identifier(id) = &mut f.key {
                    if let Some(new) = map.get(&id.name) {
                        id.name = new.clone();
                    }
                }
                if let Some(v) = &mut f.value {
                    rewrite_expr(v, map);
                }
            }
            // A static-init block has no key to rewrite; its statements may
            // contain property accesses — rewrite each, in lockstep with
            // `classify_class_members`.
            ClassMember::StaticBlock(b) => {
                for s in &mut b.body {
                    rewrite_stmt(s, map);
                }
            }
        }
    }
}

fn rewrite_stmt(stmt: &mut Statement, map: &HashMap<String, String>) {
    match stmt {
        Statement::Declaration(d) => rewrite_decl(d, map),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => rewrite_expr(&mut es.expression, map),
            TaggedStatement::BlockStatement(b) => {
                for s in &mut b.body {
                    rewrite_stmt(s, map);
                }
            }
            TaggedStatement::IfStatement(is) => {
                rewrite_expr(&mut is.test, map);
                rewrite_stmt(&mut is.consequent, map);
                if let Some(alt) = &mut is.alternate {
                    rewrite_stmt(alt, map);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                rewrite_expr(&mut ws.test, map);
                rewrite_stmt(&mut ws.body, map);
            }
            // `with (o) body` (CLOC12.187) — rewrite property refs in object + body.
            TaggedStatement::WithStatement(ws) => {
                rewrite_expr(&mut ws.object, map);
                rewrite_stmt(&mut ws.body, map);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                rewrite_expr(&mut ds.test, map);
                rewrite_stmt(&mut ds.body, map);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &mut fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &mut vd.declarations {
                                if let Some(i) = &mut d.init {
                                    rewrite_expr(i, map);
                                }
                            }
                        }
                        ForInit::Expression(e) => rewrite_expr(e, map),
                    }
                }
                if let Some(test) = &mut fs.test {
                    rewrite_expr(test, map);
                }
                if let Some(update) = &mut fs.update {
                    rewrite_expr(update, map);
                }
                rewrite_stmt(&mut fs.body, map);
            }
            TaggedStatement::ForInStatement(fs) => {
                match &mut fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            if let Some(i) = &mut d.init {
                                rewrite_expr(i, map);
                            }
                        }
                    }
                    ForInit::Expression(e) => rewrite_expr(e, map),
                }
                rewrite_expr(&mut fs.right, map);
                rewrite_stmt(&mut fs.body, map);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &mut fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            if let Some(i) = &mut d.init {
                                rewrite_expr(i, map);
                            }
                        }
                    }
                    ForInit::Expression(e) => rewrite_expr(e, map),
                }
                rewrite_expr(&mut fs.right, map);
                rewrite_stmt(&mut fs.body, map);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &mut rs.argument {
                    rewrite_expr(a, map);
                }
            }
            TaggedStatement::ThrowStatement(ts) => rewrite_expr(&mut ts.argument, map),
            TaggedStatement::LabeledStatement(ls) => rewrite_stmt(&mut ls.body, map),
            TaggedStatement::SwitchStatement(ss) => {
                rewrite_expr(&mut ss.discriminant, map);
                for c in &mut ss.cases {
                    if let Some(test) = &mut c.test {
                        rewrite_expr(test, map);
                    }
                    for s in &mut c.consequent {
                        rewrite_stmt(s, map);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // Rewrite property accesses inside the three blocks.
                for s in &mut ts.block.body {
                    rewrite_stmt(s, map);
                }
                if let Some(h) = &mut ts.handler {
                    for s in &mut h.body.body {
                        rewrite_stmt(s, map);
                    }
                }
                if let Some(f) = &mut ts.finalizer {
                    for s in &mut f.body {
                        rewrite_stmt(s, map);
                    }
                }
            }
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

fn rewrite_expr(expr: &mut Expression, map: &HashMap<String, String>) {
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // A regex literal (like `/ab+c/g`) is an inert leaf: no sub-expressions
        // and no property names, so it is grouped with the other literals.
        | Expression::RegExpLiteral(_)
        // `this` holds no property name to classify or rewrite.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            rewrite_expr(&mut be.left, map);
            rewrite_expr(&mut be.right, map);
        }
        Expression::LogicalExpression(le) => {
            rewrite_expr(&mut le.left, map);
            rewrite_expr(&mut le.right, map);
        }
        Expression::UnaryExpression(ue) => rewrite_expr(&mut ue.argument, map),
        Expression::UpdateExpression(ue) => rewrite_expr(&mut ue.argument, map),
        Expression::AssignmentExpression(ae) => {
            if let AssignmentTarget::MemberExpression(m) = &mut ae.left {
                rewrite_member(&mut m.object, &mut m.property, m.computed, map);
            }
            rewrite_expr(&mut ae.right, map);
        }
        Expression::ConditionalExpression(ce) => {
            rewrite_expr(&mut ce.test, map);
            rewrite_expr(&mut ce.consequent, map);
            rewrite_expr(&mut ce.alternate, map);
        }
        Expression::CallExpression(ce) => {
            rewrite_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rewrite_expr(a, map);
            }
        }
        Expression::NewExpression(ne) => {
            rewrite_expr(&mut ne.callee, map);
            for a in &mut ne.arguments {
                rewrite_expr(a, map);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &mut se.expressions {
                rewrite_expr(e, map);
            }
        }
        Expression::MemberExpression(m) => {
            rewrite_member(&mut m.object, &mut m.property, m.computed, map)
        }
        // `a?.b` / `a?.[k]` — rewrite the accessed property name exactly as a
        // plain member access.
        Expression::OptionalMemberExpression(m) => {
            rewrite_member(&mut m.object, &mut m.property, m.computed, map)
        }
        // `a?.()` — rewrite callee and each argument, as for an ordinary call.
        Expression::OptionalCallExpression(ce) => {
            rewrite_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rewrite_expr(a, map);
            }
        }
        // A chain expression transparently wraps its optional-chain spine —
        // descend into the inner expression.
        Expression::ChainExpression(c) => rewrite_expr(&mut c.expression, map),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                rewrite_expr(el, map);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &mut oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &mut prop.key {
                                rewrite_expr(e, map);
                            }
                        } else if let PropertyKey::Identifier(id) = &mut prop.key {
                            // Rewrite a renameable unquoted key; a `StringLiteral`
                            // (quoted) key is never in `map`, so it is left alone.
                            if let Some(new) = map.get(&id.name) {
                                id.name = new.clone();
                            }
                        }
                        rewrite_expr(&mut prop.value, map);
                    }
                    // `{ ...expr }` — a spread has no property NAME to rewrite;
                    // only walk its argument sub-expression.
                    ObjectMember::Spread(s) => {
                        rewrite_expr(&mut s.argument, map);
                    }
                }
            }
        }
        // Rewrite property accesses inside a function *value*'s body, the
        // mirror of classifying them above.
        Expression::FunctionExpression(fe) => {
            for s in &mut fe.body.body {
                rewrite_stmt(s, map);
            }
        }
        // Rewrite a class expression, the mirror of classifying it above: a
        // renameable unquoted method KEY is rewritten through `map` exactly
        // like an object-literal identifier key (a quoted key is never in
        // `map`, so it is left alone); each method VALUE body is walked for
        // nested property accesses like a `FunctionExpression` body; and the
        // `extends` operand recurses as an ordinary expression.
        Expression::ClassExpression(ce) => rewrite_class_members(&mut ce.super_class, &mut ce.body, map),
        // Rewrite property accesses inside an arrow-value's body, the
        // mirror of classifying them above.
        Expression::ArrowFunctionExpression(ae) => match &mut ae.body {
            ArrowBody::Block(b) => {
                for s in &mut b.body {
                    rewrite_stmt(s, map);
                }
            }
            ArrowBody::Expression(e) => rewrite_expr(e, map),
        },
        // Rewrite property accesses inside each `${…}` insert, the mirror of
        // classifying them above. Quasis are leaf strings — nothing to walk.
        Expression::TemplateLiteral(t) => {
            for e in &mut t.expressions {
                rewrite_expr(e, map);
            }
        }
        // Rewrite property accesses in the tag callee and each `${…}` insert,
        // the mirror of classifying them above.
        Expression::TaggedTemplateExpression(t) => {
            rewrite_expr(&mut t.tag, map);
            for e in &mut t.quasi.expressions {
                rewrite_expr(e, map);
            }
        }
        // `...arg` — recurse into the spread argument to rewrite accesses in it.
        Expression::SpreadElement(s) => rewrite_expr(&mut s.argument, map),
        Expression::YieldExpression(y) => { if let Some(a) = &mut y.argument { rewrite_expr(a, map); } }
        Expression::AwaitExpression(a) => rewrite_expr(&mut a.argument, map),
        Expression::ImportExpression(e) => rewrite_expr(&mut e.source, map),
    }
}

fn rewrite_member(
    object: &mut Expression,
    property: &mut Expression,
    computed: bool,
    map: &HashMap<String, String>,
) {
    rewrite_expr(object, map);
    if computed {
        // A quoted `obj["x"]` is never renamed (the name was disabled at
        // classification); a dynamic key is recursed for nested accesses.
        rewrite_expr(property, map);
    } else if let Expression::Identifier(id) = property {
        if let Some(new) = map.get(&id.name) {
            id.name = new.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    //! Source → bridge → rename-properties → emit roundtrips, plus the
    //! metadata contract.
    use super::*;
    use coding_adventures_closure_emitter::{emit, EmitOptions};
    use coding_adventures_closure_pass_pipeline::PassContext;
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    fn rename_with(src: &str, externs: &[&str]) -> String {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        let set: HashSet<String> = externs.iter().map(|s| s.to_string()).collect();
        let pass = RenamePropertiesPass::new(set);
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(false);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("rename-properties");
        let mut cv2 = CVLog::new(false);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        emit(&out.program, &sidecar, &mut cv2, &opts)
            .expect("emit")
            .code
    }

    fn rename(src: &str) -> String {
        rename_with(src, &[])
    }

    /// Run the pass and return its CV contributions (the rename table).
    fn rename_contributions(src: &str) -> Vec<Contribution> {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        let pass = RenamePropertiesPass::new(HashSet::new());
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        pass.run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("rename-properties")
        .contributions
    }

    // ----- CV provenance (#89) -----

    #[test]
    fn emits_renamed_contribution_per_property() {
        // `o.longProp;` — `longProp` is dotted, unquoted, not a built-in,
        // len>1 → renamed; the pass records a `renamed` contribution
        // mapping the original property name to its short form.
        let contribs = rename_contributions("o.longProp;");
        let renamed: Vec<_> = contribs
            .iter()
            .filter(|c| c.source == "rename-properties" && c.tag == "renamed")
            .collect();
        assert_eq!(
            renamed.len(),
            1,
            "expected exactly one renamed contribution; got {:?}",
            contribs
        );
        let c = renamed[0];
        assert_eq!(
            c.meta.get("from").and_then(|v| v.as_str()),
            Some("longProp")
        );
        let to = c
            .meta
            .get("to")
            .and_then(|v| v.as_str())
            .expect("`to` present");
        assert!(
            to.len() < "longProp".len(),
            "renamed to a shorter name; got {:?}",
            to
        );
    }

    #[test]
    fn no_contributions_when_nothing_renamed() {
        // `o.length;` — `length` is a built-in property, never renamed,
        // so there is nothing to rename and no contribution is emitted.
        let contribs = rename_contributions("o.length;");
        assert!(
            contribs.is_empty(),
            "expected no contributions; got {:?}",
            contribs
        );
    }

    // ----- metadata -----

    #[test]
    fn name_is_rename_properties() {
        assert_eq!(
            RenamePropertiesPass::with_builtins_only().name(),
            "rename-properties"
        );
    }

    #[test]
    fn iteration_policy_is_one_shot() {
        assert_eq!(
            RenamePropertiesPass::with_builtins_only().iteration_policy(),
            IterationPolicy::OneShot
        );
    }

    #[test]
    fn cost_is_three() {
        assert_eq!(RenamePropertiesPass::with_builtins_only().cost(), 3);
    }

    #[test]
    fn depends_on_is_empty() {
        assert!(RenamePropertiesPass::with_builtins_only()
            .depends_on()
            .is_empty());
    }

    #[test]
    fn run_on_empty_program_is_identity() {
        let pass = RenamePropertiesPass::with_builtins_only();
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("ok");
        assert!(!out.changed);
        assert_eq!(out.stats.nodes_touched, 1);
    }

    #[test]
    fn pass_is_default_and_clone() {
        let _a: RenamePropertiesPass = Default::default();
        let _b = RenamePropertiesPass::with_builtins_only();
        let _c = _b.clone();
    }

    // ----- behaviour -----

    // NOTE on inputs: a member-ASSIGNMENT statement (`obj.x = 1;`) is not
    // in the Phase-1 grammar, so these tests drive property accesses via
    // member READS (call args / initializers) and object literals.

    #[test]
    fn renames_a_dotted_property_consistently() {
        // `renderMode` appears dotted on two different objects and as an
        // unquoted key; all three become the same fresh name.
        assert_eq!(
            rename("read(a.renderMode); read(b.renderMode); var c = { renderMode: 3 };"),
            "read(a.a);read(b.a);var c={a:3};"
        );
    }

    #[test]
    fn does_not_rename_a_property_quoted_via_computed_member() {
        // `mode` appears quoted (`other["mode"]`, a computed string member
        // — which the bridge preserves as a StringLiteral). That disables
        // `mode` everywhere, even at the dotted `obj.mode` (renaming would
        // desync it from the quoted access we never touch).
        assert_eq!(
            rename("read(obj.mode); read(other[\"mode\"]);"),
            "read(obj.mode);read(other[\"mode\"]);"
        );
    }

    #[test]
    fn does_not_rename_builtins() {
        // `length` / `push` / `toString` are built-ins → never renamed.
        // The user property `tally` (an object key) IS renamed.
        assert_eq!(
            rename("var n = arr.length; list.push(x); s.toString(); var o = { tally: 1 };"),
            "var n=arr.length;list.push(x);s.toString();var o={a:1};"
        );
    }

    #[test]
    fn does_not_rename_externs_property() {
        // `apiField` is supplied as an externs property → kept; the
        // private `helperField` is renamed.
        assert_eq!(
            rename_with("read(obj.apiField); read(obj.helperField);", &["apiField"]),
            "read(obj.apiField);read(obj.a);"
        );
    }

    #[test]
    fn does_not_rename_dom_properties() {
        // The bundled DOM/host list protects `innerHTML`, `addEventListener`,
        // and `onclick` out of the box — no `--externs` needed — while the
        // program-private `secretField` is still renamed. Without the DOM
        // bundle these would be renamed and break browser code.
        assert_eq!(
            rename(
                "el.addEventListener(t, h); read(el.innerHTML); read(el.onclick); read(el.secretField); read(el.secretField);"
            ),
            "el.addEventListener(t,h);read(el.innerHTML);read(el.onclick);read(el.a);read(el.a);"
        );
    }

    #[test]
    fn dom_property_protected_without_externs() {
        // A lone DOM property the author never lists in externs is still
        // kept — the safety net the DOM bundle provides.
        assert_eq!(
            rename("read(node.textContent); read(node.textContent);"),
            "read(node.textContent);read(node.textContent);"
        );
    }

    #[test]
    fn renames_a_computed_member_object_but_not_the_dynamic_key() {
        // `obj[idx]` — `idx` is a variable (dynamic), not a property name,
        // so it is left alone; the dotted `obj.field` IS renamed. (`idx`
        // never appears dotted, so it is not a property at all.)
        assert_eq!(
            rename("var v = obj[idx]; read(obj.field);"),
            "var v=obj[idx];read(obj.a);"
        );
    }

    #[test]
    fn skips_single_char_property() {
        // Already minimal.
        assert_eq!(
            rename("read(obj.x); read(obj.x);"),
            "read(obj.x);read(obj.x);"
        );
    }

    #[test]
    fn renames_nested_property_chain() {
        // Both links of `a.outerField.innerField` are renameable; the
        // outer object's property (`outerField`) is seen first → `a`,
        // then `innerField` → `b`.
        assert_eq!(rename("read(a.outerField.innerField);"), "read(a.a.b);");
    }

    // ----- collect_property_names (externs property boundary) -----

    /// Parse `src` and collect its property names — the helper a driver
    /// uses to turn an externs file into a `do_not_rename` set.
    fn collect(src: &str) -> HashSet<String> {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        collect_property_names(&prog)
    }

    #[test]
    fn collect_empty_program_is_empty() {
        assert!(collect_property_names(&program()).is_empty());
    }

    #[test]
    fn collect_dotted_member_read() {
        // `el.innerHTML` — a dotted access names `innerHTML` as external.
        let names = collect("read(el.innerHTML);");
        assert!(names.contains("innerHTML"));
    }

    #[test]
    fn collect_quoted_member_read() {
        // `obj["data-id"]` — a quoted access still names `data-id`. As an
        // externs boundary we protect it (over-collecting is always safe).
        let names = collect("read(obj[\"data-id\"]);");
        assert!(names.contains("data-id"));
    }

    #[test]
    fn collect_unquoted_object_key() {
        // `{ onload: f }` — an unquoted key names `onload`.
        let names = collect("var handlers = { onload: cb };");
        assert!(names.contains("onload"));
    }

    #[test]
    fn collect_quoted_object_key() {
        // `{ "aria-label": s }` — a quoted key names `aria-label`.
        let names = collect("var attrs = { \"aria-label\": label };");
        assert!(names.contains("aria-label"));
    }

    #[test]
    fn collect_unions_multiple_occurrences() {
        // Dotted + quoted + object-key occurrences all land in one set.
        let names =
            collect("read(el.innerHTML); read(node[\"textContent\"]); var o = { onclick: h };");
        assert!(names.contains("innerHTML"));
        assert!(names.contains("textContent"));
        assert!(names.contains("onclick"));
    }

    #[test]
    fn collect_ignores_dynamic_computed_key() {
        // `obj[runtimeKey]` has no static name — there is nothing to
        // protect, so it contributes nothing to the boundary. (`prefix`
        // is still collected from the dotted access.)
        let names = collect("read(obj[runtimeKey]); read(obj.prefix);");
        assert!(names.contains("prefix"));
        assert!(!names.contains("runtimeKey"));
    }

    #[test]
    fn collect_walks_into_function_bodies() {
        // Property accesses nested inside a function declaration are still
        // part of the externs boundary.
        let names = collect("function api(x){ return x.payload; }");
        assert!(names.contains("payload"));
    }

    #[test]
    fn collected_externs_protect_a_property_from_rename() {
        // End-to-end intent: feeding collected externs names into the pass
        // keeps those properties while still renaming program-private ones.
        let externs = collect("read(boundary.innerHTML);");
        let externs_vec: Vec<&str> = externs.iter().map(|s| s.as_str()).collect();
        // `innerHTML` is in the boundary → kept; `secretField` is private
        // → renamed to a short name.
        let out = rename_with(
            "read(node.innerHTML); read(node.secretField); read(node.secretField);",
            &externs_vec,
        );
        assert!(out.contains(".innerHTML"));
        assert!(!out.contains("secretField"));
    }

    // -------------------------------------------------------------------
    // CLOC12.187 PR2a — `with` soundness gate. Inside `with (obj) …` a bare
    // `foo` may be `obj.foo` in disguise, so renaming the property `foo`
    // elsewhere would desynchronize from that hidden access.
    // RenamePropertiesPass must bail. The bridge does not yet produce `with`
    // (PR2b), so the AST is hand-built.
    // -------------------------------------------------------------------
    #[test]
    fn with_statement_disables_property_renaming() {
        use coding_adventures_javascript_ast::{
            BindingTarget, BlockStatement, Declaration, Expression, Identifier, NumericLiteral,
            ObjectExpression, ObjectMember, ProgramItem, Property, PropertyKey, PropertyKind,
            Statement, VarKind, VariableDeclaration, VariableDeclarator, WithStatement,
        };

        let id = |n: &str| Identifier {
            cv: None,
            name: n.to_string(),
        };

        // `var rec = { longProp: 1 };` — `longProp` is an object-literal key
        // RenamePropertiesPass would normally shorten to `a`.
        let obj = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![ObjectMember::Property(Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::Identifier(id("longProp")),
                value: Box::new(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                })),
                computed: false,
                shorthand: false,
                method: false,
            })],
        });
        let vd = VariableDeclaration {
            cv: None,
            kind: VarKind::Var,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: BindingTarget::Identifier(id("rec")),
                init: Some(obj),
            }],
        };
        // `with (o) {}`
        let with = Statement::with_statement(WithStatement {
            cv: None,
            object: Expression::Identifier(id("o")),
            body: Box::new(Statement::block_statement(BlockStatement { cv: None, body: vec![] })),
        });

        let mut prog = program();
        prog.body = vec![
            ProgramItem::Statement(with),
            ProgramItem::Declaration(Declaration::VariableDeclaration(vd)),
        ];

        let pass = RenamePropertiesPass::new(HashSet::new());
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(false);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("rename-properties");

        assert!(!out.changed, "must not rename properties when a `with` is present");
        assert_eq!(out.program, prog, "program must be returned unchanged");
        assert!(out.contributions.is_empty());
    }
}
