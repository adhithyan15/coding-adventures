//! §13.2.6 Tree construction.
//!
//! [`TreeBuilder`] receives tokens from the tokenizer (`html-lexer`) one at a
//! time and grows the arena tree. Its state is exactly the state §13.2.4 lists
//! — the insertion mode, the stack of open elements, the list of active
//! formatting elements, the head and form element pointers, and three flags —
//! and each insertion mode is one method named after it. To check a rule,
//! open the specification at the mode's section and read the method beside
//! it; the arms are in the specification's order.
//!
//! Two liberties, both invisible in the resulting tree:
//!
//! - **Character tokens arrive in runs.** The specification processes one
//!   character at a time. The builder splits each text token into runs of
//!   whitespace, U+0000, and other characters, and processes a run as one
//!   token. Every rule treats all the characters of such a run the same way,
//!   so the result is identical, and far fewer reprocessing steps happen.
//! - **Reprocessing is a return value.** "Reprocess the token" is written as
//!   `return Flow::Reprocess(token)`, and the dispatcher loops. A bound on that
//!   loop turns any future mistake that would cycle between modes into a
//!   diagnostic instead of a hang.

use crate::active_formatting::{ActiveFormatting, Entry, FormattingToken};
use crate::arena::{Arena, Namespace, NodeId, NodeKind};
use crate::elements::{
    adjust_foreign_attribute, adjust_mathml_attribute, adjust_svg_attribute,
    document_mode_for_doctype, has_implied_end_tag, has_implied_end_tag_thoroughly, is_formatting,
    is_heading, is_html_whitespace, is_special, DocumentMode,
};
use crate::insertion_mode::InsertionMode;
use crate::open_elements::{OpenElements, Scope};
use coding_adventures_html_lexer::{HtmlLexContext, HtmlScriptingMode, SourcePosition, Token};
use dom_core::Attribute;

/// A tree-construction parse error: the specification's code where it names
/// one, and where in the source the token that caused it was emitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeDiagnostic {
    pub code: &'static str,
    pub position: SourcePosition,
}

/// A start tag as the tree builder sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub self_closing: bool,
}

impl Tag {
    fn named(name: &str) -> Self {
        Tag {
            name: name.to_string(),
            attributes: Vec::new(),
            self_closing: false,
        }
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }
}

/// What a run of character tokens contains; see the module comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// Only U+0009, U+000A, U+000C, U+000D and U+0020.
    Whitespace,
    /// Only U+0000.
    Null,
    /// Anything else.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    Doctype {
        name: Option<String>,
        public_identifier: Option<String>,
        system_identifier: Option<String>,
        force_quirks: bool,
    },
    StartTag(Tag),
    EndTag(String),
    Text(TextKind, String),
    Comment(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
    Eof,
}

/// The result of one rule: the token is finished with, or must be handled
/// again by the (new) current insertion mode.
#[must_use]
pub enum Flow {
    Done,
    Reprocess(Tok),
}

/// Where a node goes: "inside `parent`, before `before`" (or at the end).
#[derive(Debug, Clone, Copy)]
struct InsertionPoint {
    parent: NodeId,
    before: Option<NodeId>,
}

/// How many times one token may be handed from mode to mode. The longest
/// legitimate chain (text in `Initial` walking through `BeforeHtml`,
/// `BeforeHead`, `InHead`, `AfterHead` into `InBody`) is six.
const MAX_REPROCESS: usize = 32;

/// A resource limit, not a specification rule: the most elements the stack of
/// open elements holds when a start tag arrives. Scope checks, "any other end
/// tag" and the adoption agency all walk the stack, so an unbounded stack
/// (`<div>` × 100,000) makes every token cost as much as the whole document.
/// A start tag that finds the stack full first closes the current node, as if
/// its end tag had been omitted, and reports `tree-builder-open-elements-limit`.
/// (Reconstruction may still add up to 64 formatting elements past it.)
/// Output depth is capped separately, in `arena::MAX_TREE_DEPTH`.
pub const MAX_OPEN_ELEMENTS: usize = 512;

pub struct TreeBuilder {
    pub(crate) arena: Arena,
    mode: InsertionMode,
    original_mode: InsertionMode,
    /// §13.2.6.4.9 "pending table character tokens", kept as the runs they
    /// arrived in.
    pending_table_text: Vec<(TextKind, String)>,
    template_modes: Vec<InsertionMode>,
    open: OpenElements,
    formatting: ActiveFormatting,
    head: Option<NodeId>,
    form: Option<NodeId>,
    /// §13.4's context element. Always `None` until fragment parsing is
    /// written (BR03 §5 step 4); the rules that consult it already do.
    fragment_context: Option<NodeId>,
    scripting: HtmlScriptingMode,
    frameset_ok: bool,
    foster_parenting: bool,
    document_mode: DocumentMode,
    ignore_next_line_feed: bool,
    self_closing_acknowledged: bool,
    tokenizer_request: Option<HtmlLexContext>,
    position: SourcePosition,
    stopped: bool,
    pub(crate) diagnostics: Vec<TreeDiagnostic>,
}

impl TreeBuilder {
    pub fn new(scripting: HtmlScriptingMode) -> Self {
        Self {
            arena: Arena::new(),
            mode: InsertionMode::Initial,
            original_mode: InsertionMode::Initial,
            pending_table_text: Vec::new(),
            template_modes: Vec::new(),
            open: OpenElements::default(),
            formatting: ActiveFormatting::default(),
            head: None,
            form: None,
            fragment_context: None,
            scripting,
            frameset_ok: true,
            foster_parenting: false,
            document_mode: DocumentMode::NoQuirks,
            ignore_next_line_feed: false,
            self_closing_acknowledged: false,
            tokenizer_request: None,
            position: SourcePosition::default(),
            stopped: false,
            diagnostics: Vec::new(),
        }
    }

    pub fn document_mode(&self) -> DocumentMode {
        self.document_mode
    }

    /// The tokenizer state the last token asked for, if any (§13.2.6: the tree
    /// builder switches the tokenizer into RCDATA, RAWTEXT, script data or
    /// PLAINTEXT after certain start tags, and back to data at their end).
    pub fn take_tokenizer_request(&mut self) -> Option<HtmlLexContext> {
        self.tokenizer_request.take()
    }

    /// Feed one tokenizer token.
    pub fn process(&mut self, token: Token, position: SourcePosition) {
        if self.stopped {
            return;
        }
        self.position = position;
        let token = match token {
            Token::Text(text) => {
                let text = if std::mem::take(&mut self.ignore_next_line_feed) {
                    text.strip_prefix('\n').map(str::to_string).unwrap_or(text)
                } else {
                    text
                };
                for (kind, run) in split_runs(&text) {
                    self.run(Tok::Text(kind, run));
                }
                return;
            }
            Token::StartTag {
                name,
                attributes,
                self_closing,
            } => Tok::StartTag(Tag {
                name,
                attributes: attributes
                    .into_iter()
                    .map(|attribute| Attribute {
                        name: attribute.name,
                        value: attribute.value,
                    })
                    .collect(),
                self_closing,
            }),
            Token::EndTag { name } => Tok::EndTag(name),
            Token::Comment(data) => Tok::Comment(data),
            Token::ProcessingInstruction { target, data } => {
                Tok::ProcessingInstruction { target, data }
            }
            Token::Doctype {
                name,
                public_identifier,
                system_identifier,
                force_quirks,
            } => Tok::Doctype {
                name,
                public_identifier,
                system_identifier,
                force_quirks,
            },
            Token::Eof => Tok::Eof,
        };
        self.ignore_next_line_feed = false;
        if matches!(token, Tok::StartTag(_)) && self.open.len() >= MAX_OPEN_ELEMENTS {
            self.make_room();
        }
        let self_closing = matches!(&token, Tok::StartTag(tag) if tag.self_closing);
        self.self_closing_acknowledged = false;
        self.run(token);
        if self_closing && !self.self_closing_acknowledged {
            self.error("non-void-html-element-start-tag-with-trailing-solidus");
        }
    }

    /// Close current nodes until the stack is below [`MAX_OPEN_ELEMENTS`],
    /// keeping the other state consistent with each close: the node leaves the
    /// list of active formatting elements, an element that pushed a marker
    /// clears to it, a template pops its template insertion mode, and the
    /// insertion mode is reset afterwards.
    fn make_room(&mut self) {
        self.error("tree-builder-open-elements-limit");
        while self.open.len() >= MAX_OPEN_ELEMENTS {
            let Some(node) = self.open.pop(&self.arena) else {
                break;
            };
            self.formatting.remove(node);
            let pushed_marker = [
                "applet", "object", "marquee", "template", "td", "th", "caption",
            ]
            .iter()
            .any(|name| self.arena.is_html(node, name));
            if pushed_marker {
                self.formatting.clear_to_last_marker();
            }
            if self.arena.is_html(node, "template") {
                self.template_modes.pop();
            }
        }
        if self.mode != InsertionMode::Text {
            self.reset_insertion_mode();
        }
    }

    fn run(&mut self, token: Tok) {
        let mut token = token;
        // Closing N open templates at end of file legitimately reprocesses the
        // EOF token N times, so the bound grows with the stack (which is finite).
        let limit = MAX_REPROCESS + self.open.len();
        for _ in 0..limit {
            if self.stopped {
                return;
            }
            match self.dispatch(token) {
                Flow::Done => return,
                Flow::Reprocess(again) => token = again,
            }
        }
        self.error("tree-builder-reprocess-limit");
    }

    /// §13.2.6 "tree construction dispatcher". Foreign content is not written
    /// yet (BR03 §5 step 3), so every token takes the HTML-content branch.
    fn dispatch(&mut self, token: Tok) -> Flow {
        self.process_in(self.mode, token)
    }

    /// "Process the token using the rules for the `mode` insertion mode".
    fn process_in(&mut self, mode: InsertionMode, token: Tok) -> Flow {
        match mode {
            InsertionMode::Initial => self.initial(token),
            InsertionMode::BeforeHtml => self.before_html(token),
            InsertionMode::BeforeHead => self.before_head(token),
            InsertionMode::InHead => self.in_head(token),
            InsertionMode::InHeadNoscript => self.in_head_noscript(token),
            InsertionMode::AfterHead => self.after_head(token),
            InsertionMode::InBody => self.in_body(token),
            InsertionMode::Text => self.text(token),
            InsertionMode::InTable => self.in_table(token),
            InsertionMode::InTableText => self.in_table_text(token),
            InsertionMode::InCaption => self.in_caption(token),
            InsertionMode::InColumnGroup => self.in_column_group(token),
            InsertionMode::InTableBody => self.in_table_body(token),
            InsertionMode::InRow => self.in_row(token),
            InsertionMode::InCell => self.in_cell(token),
            InsertionMode::InTemplate => self.in_template(token),
            InsertionMode::AfterBody => self.after_body(token),
            InsertionMode::InFrameset => self.in_frameset(token),
            InsertionMode::AfterFrameset => self.after_frameset(token),
            InsertionMode::AfterAfterBody => self.after_after_body(token),
            InsertionMode::AfterAfterFrameset => self.after_after_frameset(token),
        }
    }

    pub fn finish(self) -> (Arena, Vec<TreeDiagnostic>) {
        (self.arena, self.diagnostics)
    }

    // ----------------------------------------------------------------------
    // §13.2.6.1 Creating and inserting nodes
    // ----------------------------------------------------------------------

    fn error(&mut self, code: &'static str) {
        self.diagnostics.push(TreeDiagnostic {
            code,
            position: self.position,
        });
    }

    fn current(&self) -> Option<NodeId> {
        self.open.current()
    }

    fn current_is(&self, name: &str) -> bool {
        self.current()
            .is_some_and(|node| self.arena.is_html(node, name))
    }

    fn current_is_any(&self, names: &[&str]) -> bool {
        names.iter().any(|name| self.current_is(name))
    }

    fn fragment_context_is(&self, name: &str) -> bool {
        self.fragment_context
            .is_some_and(|context| self.arena.is_html(context, name))
    }

    fn is_special_node(&self, node: NodeId) -> bool {
        self.arena
            .element(node)
            .is_some_and(|element| is_special(element.namespace, &element.name))
    }

    /// "The appropriate place for inserting a node", optionally with an
    /// override target.
    fn appropriate_place(&self, override_target: Option<NodeId>) -> InsertionPoint {
        let target = override_target
            .or_else(|| self.current())
            .unwrap_or(Arena::DOCUMENT);
        let fosters = self.foster_parenting
            && ["table", "tbody", "tfoot", "thead", "tr"]
                .iter()
                .any(|name| self.arena.is_html(target, name));

        let point = if fosters {
            self.foster_parent_place()
        } else {
            InsertionPoint {
                parent: target,
                before: None,
            }
        };

        // "If the adjusted insertion location is inside a template element,
        // let it instead be inside the template element's template contents."
        match self
            .arena
            .element(point.parent)
            .and_then(|element| element.template_contents)
        {
            Some(contents) => InsertionPoint {
                parent: contents,
                before: None,
            },
            None => point,
        }
    }

    /// The foster-parenting branch of "the appropriate place" (step 2).
    fn foster_parent_place(&self) -> InsertionPoint {
        let stack = self.open.as_slice();
        let last_of = |name: &str| {
            stack
                .iter()
                .rposition(|&node| self.arena.is_html(node, name))
        };
        let last_template = last_of("template");
        let last_table = last_of("table");

        if let Some(template) = last_template {
            if last_table.is_none_or(|table| template > table) {
                return InsertionPoint {
                    parent: stack[template],
                    before: None,
                };
            }
        }
        let Some(table_index) = last_table else {
            // The fragment case: inside the first element (the html element).
            return InsertionPoint {
                parent: stack.first().copied().unwrap_or(Arena::DOCUMENT),
                before: None,
            };
        };
        let table = stack[table_index];
        if let Some(parent) = self.arena.parent(table) {
            return InsertionPoint {
                parent,
                before: Some(table),
            };
        }
        InsertionPoint {
            parent: stack[table_index.saturating_sub(1)],
            before: None,
        }
    }

    fn insert_at(&mut self, point: InsertionPoint, node: NodeId) {
        match point.before {
            Some(before) => self.arena.insert_before(point.parent, node, before),
            None => self.arena.append(point.parent, node),
        }
    }

    /// "Insert a foreign element" for `tag` in `namespace`, and push it.
    fn insert_element(&mut self, namespace: Namespace, tag: &Tag) -> NodeId {
        let point = self.appropriate_place(None);
        let element =
            self.arena
                .create_element(namespace, tag.name.clone(), tag.attributes.clone());
        self.insert_at(point, element);
        self.open.push(&self.arena, element);
        element
    }

    /// "Insert an HTML element".
    fn insert_html_element(&mut self, tag: &Tag) -> NodeId {
        self.insert_element(Namespace::Html, tag)
    }

    /// Insert an element and pop it straight away: the void elements, and
    /// self-closing foreign elements.
    fn insert_void_element(&mut self, tag: &Tag) {
        self.insert_html_element(tag);
        self.open.pop(&self.arena);
        self.self_closing_acknowledged = true;
    }

    /// "Insert a character": extend the Text node just before the insertion
    /// point, or create one. Characters never go directly into the Document.
    fn insert_text(&mut self, text: &str) {
        let point = self.appropriate_place(None);
        if point.parent == Arena::DOCUMENT {
            return;
        }
        if let Some(previous) = self.arena.child_before(point.parent, point.before) {
            if let NodeKind::Text(existing) = self.arena.kind_mut(previous) {
                existing.push_str(text);
                return;
            }
        }
        let node = self.arena.create(NodeKind::Text(text.to_string()));
        self.insert_at(point, node);
    }

    /// "Insert a comment" (also used for processing instructions, which the
    /// tokenizer reports separately and the tree places the same way).
    fn insert_comment_like(&mut self, token: Tok, parent: Option<NodeId>) {
        let kind = match token {
            Tok::Comment(data) => NodeKind::Comment(data),
            Tok::ProcessingInstruction { target, data } => {
                NodeKind::ProcessingInstruction { target, data }
            }
            _ => return,
        };
        let point = match parent {
            Some(parent) => InsertionPoint {
                parent,
                before: None,
            },
            None => self.appropriate_place(None),
        };
        let node = self.arena.create(kind);
        self.insert_at(point, node);
    }

    /// §13.2.6.2 "generic raw text / RCDATA element parsing algorithm":
    /// insert the element, switch the tokenizer, and collect its text in the
    /// `Text` insertion mode.
    fn parse_text_element(&mut self, tag: &Tag) {
        self.insert_html_element(tag);
        self.tokenizer_request =
            HtmlLexContext::for_element_text_with_scripting(&tag.name, self.scripting);
        self.original_mode = self.mode;
        self.mode = InsertionMode::Text;
    }

    // ----------------------------------------------------------------------
    // §13.2.6.3 Closing elements that have implied end tags
    // ----------------------------------------------------------------------

    fn generate_implied_end_tags(&mut self, except: Option<&str>) {
        while let Some(current) = self.current() {
            let Some(element) = self.arena.element(current) else {
                break;
            };
            if element.namespace != Namespace::Html
                || !has_implied_end_tag(&element.name)
                || Some(element.name.as_str()) == except
            {
                break;
            }
            self.open.pop(&self.arena);
        }
    }

    fn generate_all_implied_end_tags_thoroughly(&mut self) {
        while let Some(current) = self.current() {
            match self.arena.element(current) {
                Some(element)
                    if element.namespace == Namespace::Html
                        && has_implied_end_tag_thoroughly(&element.name) =>
                {
                    self.open.pop(&self.arena);
                }
                _ => break,
            }
        }
    }

    /// "Close a p element".
    fn close_p_element(&mut self) {
        self.generate_implied_end_tags(Some("p"));
        if !self.current_is("p") {
            self.error("end-tag-too-early");
        }
        self.open.pop_until_html(&self.arena, "p");
    }

    fn close_p_if_in_button_scope(&mut self) {
        if self.open.has_in_scope(&self.arena, "p", Scope::Button) {
            self.close_p_element();
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.4.1 Resetting the insertion mode appropriately
    // ----------------------------------------------------------------------

    fn reset_insertion_mode(&mut self) {
        for index in (0..self.open.len()).rev() {
            let last = index == 0;
            let node = match (last, self.fragment_context) {
                (true, Some(context)) => context,
                _ => self.open.get(index).expect("index is in range"),
            };
            let Some(element) = self.arena.element(node) else {
                continue;
            };
            let name = if element.namespace == Namespace::Html {
                element.name.as_str()
            } else {
                ""
            };
            let mode = match name {
                "td" | "th" if !last => InsertionMode::InCell,
                "tr" => InsertionMode::InRow,
                "tbody" | "thead" | "tfoot" => InsertionMode::InTableBody,
                "caption" => InsertionMode::InCaption,
                "colgroup" => InsertionMode::InColumnGroup,
                "table" => InsertionMode::InTable,
                "template" => *self.template_modes.last().unwrap_or(&InsertionMode::InBody),
                "head" if !last => InsertionMode::InHead,
                "body" => InsertionMode::InBody,
                "frameset" => InsertionMode::InFrameset,
                "html" => {
                    if self.head.is_none() {
                        InsertionMode::BeforeHead
                    } else {
                        InsertionMode::AfterHead
                    }
                }
                _ if last => InsertionMode::InBody,
                _ => continue,
            };
            self.mode = mode;
            return;
        }
        self.mode = InsertionMode::InBody;
    }

    // ----------------------------------------------------------------------
    // §13.2.4.3 Reconstructing the active formatting elements
    // ----------------------------------------------------------------------

    fn reconstruct_active_formatting(&mut self) {
        let entries = self.formatting.entries();
        let settled = |entry: &Entry| match entry {
            Entry::Marker => true,
            Entry::Element { node, .. } => self.open.contains(*node),
        };
        let Some(last) = entries.last() else {
            return;
        };
        if settled(last) {
            return;
        }
        // Rewind to the earliest entry after the last settled one…
        let mut index = entries.len() - 1;
        while index > 0 && !settled(&entries[index - 1]) {
            index -= 1;
        }
        // …then advance, creating a fresh element for each.
        for position in index..self.formatting.len() {
            let Some(Entry::Element { token, .. }) = self.formatting.get(position).cloned() else {
                continue;
            };
            let fresh = self.insert_html_element(&Tag {
                name: token.name,
                attributes: token.attributes,
                self_closing: false,
            });
            self.formatting.set_node(position, fresh);
        }
    }

    fn push_formatting_element(&mut self, tag: &Tag) {
        self.reconstruct_active_formatting();
        let node = self.insert_html_element(tag);
        let capped = self.formatting.push(
            node,
            FormattingToken {
                name: tag.name.clone(),
                attributes: tag.attributes.clone(),
            },
        );
        if capped {
            self.error("tree-builder-formatting-limit");
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.7 The adoption agency algorithm
    // ----------------------------------------------------------------------

    /// Returns `false` when the algorithm says to "act as described in the
    /// 'any other end tag' entry" instead.
    fn adoption_agency(&mut self, subject: &str) -> bool {
        // Step 2.
        if let Some(current) = self.current() {
            if self.arena.is_html(current, subject) && !self.formatting.contains(current) {
                self.open.pop(&self.arena);
                return true;
            }
        }

        // Steps 3–4: the outer loop, at most eight times.
        for _ in 0..8 {
            // 4.3
            let Some(formatting_element) = self
                .formatting
                .last_named_after_marker(&self.arena, subject)
            else {
                return false;
            };
            // 4.4
            let Some(formatting_index) = self.open.position(formatting_element) else {
                self.error("adoption-agency-1.2");
                self.formatting.remove(formatting_element);
                return true;
            };
            // 4.5
            if !self
                .open
                .has_node_in_scope(&self.arena, formatting_element, Scope::Default)
            {
                self.error("adoption-agency-4.4");
                return true;
            }
            // 4.6
            if self.current() != Some(formatting_element) {
                self.error("adoption-agency-1.3");
            }
            // 4.7: the furthest block is the topmost special element below the
            // formatting element.
            let furthest_block = self.open.as_slice()[formatting_index + 1..]
                .iter()
                .copied()
                .find(|&node| self.is_special_node(node));
            // 4.8
            let Some(furthest_block) = furthest_block else {
                self.open.pop_until_node(&self.arena, formatting_element);
                self.formatting.remove(formatting_element);
                return true;
            };
            // 4.9
            let common_ancestor = self
                .open
                .get(formatting_index.saturating_sub(1))
                .expect("the html element is above every formatting element");
            // 4.10: the bookmark starts on the formatting element itself.
            let mut bookmark_after: Option<NodeId> = None;
            // 4.11
            let mut node_index = self
                .open
                .position(furthest_block)
                .expect("furthest block is on the stack");
            let mut last_node = furthest_block;
            // 4.13: the inner loop.
            let mut inner = 0;
            loop {
                inner += 1;
                node_index -= 1;
                let node = self
                    .open
                    .get(node_index)
                    .expect("walks up to the formatting element");
                if node == formatting_element {
                    break;
                }
                if inner > 3 && self.formatting.contains(node) {
                    self.formatting.remove(node);
                }
                let Some(token) = self.formatting.token_for(node).cloned() else {
                    self.open.remove(&self.arena, node);
                    continue;
                };
                let fresh =
                    self.arena
                        .create_element(Namespace::Html, token.name, token.attributes);
                self.formatting.replace_node(node, fresh);
                self.open.replace(&self.arena, node, fresh);
                if last_node == furthest_block {
                    bookmark_after = Some(fresh);
                }
                self.arena.append(fresh, last_node);
                last_node = fresh;
            }
            // 4.14: insert lastNode at the appropriate place for commonAncestor,
            // unless that would make it its own ancestor (the specification's
            // pre-insert validity condition; foster parenting can aim there).
            let place = self.appropriate_place(Some(common_ancestor));
            self.arena.detach(last_node);
            if !self.arena.is_inclusive_ancestor(last_node, place.parent) {
                self.insert_at(place, last_node);
            }
            // 4.15–4.17
            let token = self
                .formatting
                .token_for(formatting_element)
                .cloned()
                .expect("the formatting element is in the list");
            let fresh = self.arena.create_element(
                Namespace::Html,
                token.name.clone(),
                token.attributes.clone(),
            );
            self.arena.reparent_children(furthest_block, fresh);
            self.arena.append(furthest_block, fresh);
            // 4.18
            match bookmark_after {
                None => self.formatting.replace_node(formatting_element, fresh),
                Some(after) => {
                    self.formatting.remove(formatting_element);
                    let index = self
                        .formatting
                        .position(after)
                        .map_or(self.formatting.len(), |index| index + 1);
                    self.formatting
                        .insert(index, Entry::Element { node: fresh, token });
                }
            }
            // 4.19
            self.open.remove(&self.arena, formatting_element);
            let below = self
                .open
                .position(furthest_block)
                .expect("furthest block is on the stack");
            self.open.insert(&self.arena, below + 1, fresh);
        }
        true
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.1 The "initial" insertion mode
    // ----------------------------------------------------------------------

    fn initial(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, _) => Flow::Done,
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, Some(Arena::DOCUMENT));
                Flow::Done
            }
            Tok::Doctype {
                name,
                public_identifier,
                system_identifier,
                force_quirks,
            } => {
                if name.as_deref() != Some("html")
                    || public_identifier.is_some()
                    || system_identifier
                        .as_deref()
                        .is_some_and(|system| system != "about:legacy-compat")
                {
                    self.error("unknown-doctype");
                }
                self.document_mode = document_mode_for_doctype(
                    name.as_deref(),
                    public_identifier.as_deref(),
                    system_identifier.as_deref(),
                    force_quirks,
                );
                let doctype = self.arena.create(NodeKind::Doctype {
                    name,
                    public_identifier,
                    system_identifier,
                });
                self.arena.append(Arena::DOCUMENT, doctype);
                self.mode = InsertionMode::BeforeHtml;
                Flow::Done
            }
            other => {
                self.error("expected-doctype-but-got-other");
                self.document_mode = DocumentMode::Quirks;
                self.mode = InsertionMode::BeforeHtml;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.2 The "before html" insertion mode
    // ----------------------------------------------------------------------

    fn before_html(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, Some(Arena::DOCUMENT));
                Flow::Done
            }
            Tok::Text(TextKind::Whitespace, _) => Flow::Done,
            Tok::StartTag(tag) if tag.name == "html" => {
                self.create_root(tag.attributes);
                self.mode = InsertionMode::BeforeHead;
                Flow::Done
            }
            Tok::EndTag(name) if !matches!(name.as_str(), "head" | "body" | "html" | "br") => {
                self.error("unexpected-end-tag-before-html");
                Flow::Done
            }
            other => {
                self.create_root(Vec::new());
                self.mode = InsertionMode::BeforeHead;
                Flow::Reprocess(other)
            }
        }
    }

    fn create_root(&mut self, attributes: Vec<Attribute>) {
        let html = self
            .arena
            .create_element(Namespace::Html, "html", attributes);
        self.arena.append(Arena::DOCUMENT, html);
        self.open.push(&self.arena, html);
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.3 The "before head" insertion mode
    // ----------------------------------------------------------------------

    fn before_head(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, _) => Flow::Done,
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None);
                Flow::Done
            }
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::StartTag(tag) if tag.name == "head" => {
                self.head = Some(self.insert_html_element(&tag));
                self.mode = InsertionMode::InHead;
                Flow::Done
            }
            Tok::EndTag(name) if !matches!(name.as_str(), "head" | "body" | "html" | "br") => {
                self.error("end-tag-after-implied-root");
                Flow::Done
            }
            other => {
                self.head = Some(self.insert_html_element(&Tag::named("head")));
                self.mode = InsertionMode::InHead;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.4 The "in head" insertion mode
    // ----------------------------------------------------------------------

    fn in_head(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, text) => {
                self.insert_text(&text);
                Flow::Done
            }
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None);
                Flow::Done
            }
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "base" | "basefont" | "bgsound" | "link" | "meta"
                ) =>
            {
                self.insert_void_element(&tag);
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "title" => {
                self.parse_text_element(&tag);
                Flow::Done
            }
            Tok::StartTag(tag)
                if tag.name == "noscript" && self.scripting == HtmlScriptingMode::Disabled =>
            {
                self.insert_html_element(&tag);
                self.mode = InsertionMode::InHeadNoscript;
                Flow::Done
            }
            Tok::StartTag(tag)
                if matches!(tag.name.as_str(), "noscript" | "noframes" | "style") =>
            {
                self.parse_text_element(&tag);
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "script" => {
                // Script elements are "parser-inserted" and never run here; the
                // JavaScript engine (BR02 P9) is what will run them.
                self.parse_text_element(&tag);
                Flow::Done
            }
            Tok::EndTag(name) if name == "head" => {
                self.open.pop(&self.arena);
                self.mode = InsertionMode::AfterHead;
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "template" => {
                self.insert_html_element(&tag);
                self.formatting.push_marker();
                self.frameset_ok = false;
                self.mode = InsertionMode::InTemplate;
                self.template_modes.push(InsertionMode::InTemplate);
                Flow::Done
            }
            Tok::EndTag(name) if name == "template" => {
                if !self.open.contains_html("template") {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.generate_all_implied_end_tags_thoroughly();
                if !self.current_is("template") {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_html(&self.arena, "template");
                self.formatting.clear_to_last_marker();
                self.template_modes.pop();
                self.reset_insertion_mode();
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "head" => {
                self.error("two-heads-are-not-better-than-one");
                Flow::Done
            }
            Tok::EndTag(name) if !matches!(name.as_str(), "body" | "html" | "br") => {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            other => {
                self.open.pop(&self.arena);
                self.mode = InsertionMode::AfterHead;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.5 The "in head noscript" insertion mode
    // ----------------------------------------------------------------------

    fn in_head_noscript(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::EndTag(ref name) if name == "noscript" => {
                self.open.pop(&self.arena);
                self.mode = InsertionMode::InHead;
                Flow::Done
            }
            Tok::Text(TextKind::Whitespace, _)
            | Tok::Comment(_)
            | Tok::ProcessingInstruction { .. } => self.in_head(token),
            Tok::StartTag(ref tag)
                if matches!(
                    tag.name.as_str(),
                    "basefont" | "bgsound" | "link" | "meta" | "noframes" | "style"
                ) =>
            {
                self.in_head(token)
            }
            Tok::StartTag(ref tag) if matches!(tag.name.as_str(), "head" | "noscript") => {
                self.error("unexpected-start-tag");
                Flow::Done
            }
            Tok::EndTag(ref name) if name != "br" => {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            other => {
                self.error("unexpected-token-in-head-noscript");
                self.open.pop(&self.arena);
                self.mode = InsertionMode::InHead;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.6 The "after head" insertion mode
    // ----------------------------------------------------------------------

    fn after_head(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, text) => {
                self.insert_text(&text);
                Flow::Done
            }
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None);
                Flow::Done
            }
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::StartTag(tag) if tag.name == "body" => {
                self.insert_html_element(&tag);
                self.frameset_ok = false;
                self.mode = InsertionMode::InBody;
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "frameset" => {
                self.insert_html_element(&tag);
                self.mode = InsertionMode::InFrameset;
                Flow::Done
            }
            Tok::StartTag(ref tag)
                if matches!(
                    tag.name.as_str(),
                    "base"
                        | "basefont"
                        | "bgsound"
                        | "link"
                        | "meta"
                        | "noframes"
                        | "script"
                        | "style"
                        | "template"
                        | "title"
                ) =>
            {
                self.error("unexpected-start-tag-out-of-my-head");
                let Some(head) = self.head else {
                    return self.in_head(token);
                };
                self.open.push(&self.arena, head);
                let flow = self.in_head(token);
                self.open.remove(&self.arena, head);
                flow
            }
            Tok::EndTag(ref name) if name == "template" => self.in_head(token),
            Tok::StartTag(ref tag) if tag.name == "head" => {
                self.error("unexpected-start-tag");
                Flow::Done
            }
            Tok::EndTag(ref name) if !matches!(name.as_str(), "body" | "html" | "br") => {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            other => {
                self.insert_html_element(&Tag::named("body"));
                self.frameset_ok = true;
                self.mode = InsertionMode::InBody;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.7 The "in body" insertion mode
    // ----------------------------------------------------------------------

    fn in_body(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Null, _) => {
                self.error("unexpected-null-character");
            }
            Tok::Text(TextKind::Whitespace, text) => {
                self.reconstruct_active_formatting();
                self.insert_text(&text);
            }
            Tok::Text(TextKind::Other, text) => {
                self.reconstruct_active_formatting();
                self.insert_text(&text);
                self.frameset_ok = false;
            }
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None);
            }
            Tok::Doctype { .. } => self.error("unexpected-doctype"),
            Tok::StartTag(tag) => return self.in_body_start_tag(tag),
            Tok::EndTag(name) => return self.in_body_end_tag(name),
            Tok::Eof => {
                if !self.template_modes.is_empty() {
                    return self.in_template(Tok::Eof);
                }
                self.check_open_elements_at_end();
                self.stopped = true;
            }
        }
        Flow::Done
    }

    /// The parse error the EOF and `</body>` rules share: something other than
    /// an element whose end tag may be omitted is still open.
    fn check_open_elements_at_end(&mut self) {
        let unclosed = self.open.as_slice().iter().any(|&node| {
            !self.arena.element(node).is_some_and(|element| {
                element.namespace == Namespace::Html
                    && matches!(
                        element.name.as_str(),
                        "dd" | "dt"
                            | "li"
                            | "optgroup"
                            | "option"
                            | "p"
                            | "rb"
                            | "rp"
                            | "rt"
                            | "rtc"
                            | "tbody"
                            | "td"
                            | "tfoot"
                            | "th"
                            | "thead"
                            | "tr"
                            | "body"
                            | "html"
                    )
            })
        });
        if unclosed {
            self.error("expected-closing-tag-but-got-eof");
        }
    }

    /// Copy each attribute of `tag` that `target` lacks onto it (`<html>` and
    /// `<body>` start tags met in the body).
    fn merge_attributes(&mut self, target: NodeId, tag: Tag) {
        if let Some(element) = self.arena.element_mut(target) {
            for attribute in tag.attributes {
                if !element
                    .attributes
                    .iter()
                    .any(|existing| existing.name == attribute.name)
                {
                    element.attributes.push(attribute);
                }
            }
        }
    }

    fn in_body_start_tag(&mut self, mut tag: Tag) -> Flow {
        match tag.name.as_str() {
            "html" => {
                self.error("non-html-root");
                if !self.open.contains_html("template") {
                    if let Some(html) = self.open.top() {
                        self.merge_attributes(html, tag);
                    }
                }
            }
            "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style"
            | "template" | "title" => return self.in_head(Tok::StartTag(tag)),
            "body" => {
                self.error("unexpected-start-tag");
                let second = self.open.get(1);
                if self.open.len() > 1
                    && second.is_some_and(|node| self.arena.is_html(node, "body"))
                    && !self.open.contains_html("template")
                {
                    self.frameset_ok = false;
                    self.merge_attributes(second.expect("checked above"), tag);
                }
            }
            "frameset" => {
                self.error("unexpected-start-tag");
                let second = self.open.get(1);
                if self.open.len() > 1
                    && second.is_some_and(|node| self.arena.is_html(node, "body"))
                    && self.frameset_ok
                {
                    self.arena.detach(second.expect("checked above"));
                    while self.open.len() > 1 {
                        self.open.pop(&self.arena);
                    }
                    self.insert_html_element(&tag);
                    self.mode = InsertionMode::InFrameset;
                }
            }
            "address" | "article" | "aside" | "blockquote" | "center" | "details" | "dialog"
            | "dir" | "div" | "dl" | "fieldset" | "figcaption" | "figure" | "footer" | "header"
            | "hgroup" | "main" | "menu" | "nav" | "ol" | "p" | "search" | "section"
            | "summary" | "ul" => {
                self.close_p_if_in_button_scope();
                self.insert_html_element(&tag);
            }
            name if is_heading(name) => {
                self.close_p_if_in_button_scope();
                if self
                    .current()
                    .and_then(|node| self.arena.element(node))
                    .is_some_and(|element| {
                        element.namespace == Namespace::Html && is_heading(&element.name)
                    })
                {
                    self.error("unexpected-start-tag");
                    self.open.pop(&self.arena);
                }
                self.insert_html_element(&tag);
            }
            "pre" | "listing" => {
                self.close_p_if_in_button_scope();
                self.insert_html_element(&tag);
                self.ignore_next_line_feed = true;
                self.frameset_ok = false;
            }
            "form" => {
                let in_template = self.open.contains_html("template");
                if self.form.is_some() && !in_template {
                    self.error("unexpected-start-tag");
                } else {
                    self.close_p_if_in_button_scope();
                    let form = self.insert_html_element(&tag);
                    if !in_template {
                        self.form = Some(form);
                    }
                }
            }
            "li" => {
                self.frameset_ok = false;
                self.close_list_item(&["li"]);
                self.close_p_if_in_button_scope();
                self.insert_html_element(&tag);
            }
            "dd" | "dt" => {
                self.frameset_ok = false;
                self.close_list_item(&["dd", "dt"]);
                self.close_p_if_in_button_scope();
                self.insert_html_element(&tag);
            }
            "plaintext" => {
                self.close_p_if_in_button_scope();
                self.insert_html_element(&tag);
                self.tokenizer_request =
                    HtmlLexContext::for_element_text_with_scripting("plaintext", self.scripting);
            }
            "button" => {
                if self
                    .open
                    .has_in_scope(&self.arena, "button", Scope::Default)
                {
                    self.error("unexpected-start-tag-implies-end-tag");
                    self.generate_implied_end_tags(None);
                    self.open.pop_until_html(&self.arena, "button");
                }
                self.reconstruct_active_formatting();
                self.insert_html_element(&tag);
                self.frameset_ok = false;
            }
            "a" => {
                if let Some(existing) = self.formatting.last_named_after_marker(&self.arena, "a") {
                    self.error("unexpected-start-tag-implies-end-tag");
                    if !self.adoption_agency("a") {
                        self.any_other_end_tag("a");
                    }
                    self.formatting.remove(existing);
                    self.open.remove(&self.arena, existing);
                }
                self.push_formatting_element(&tag);
            }
            "nobr" => {
                self.reconstruct_active_formatting();
                if self.open.has_in_scope(&self.arena, "nobr", Scope::Default) {
                    self.error("unexpected-start-tag-implies-end-tag");
                    if !self.adoption_agency("nobr") {
                        self.any_other_end_tag("nobr");
                    }
                }
                self.push_formatting_element(&tag);
            }
            name if is_formatting(name) => self.push_formatting_element(&tag),
            "applet" | "marquee" | "object" => {
                self.reconstruct_active_formatting();
                self.insert_html_element(&tag);
                self.formatting.push_marker();
                self.frameset_ok = false;
            }
            "table" => {
                if self.document_mode != DocumentMode::Quirks {
                    self.close_p_if_in_button_scope();
                }
                self.insert_html_element(&tag);
                self.frameset_ok = false;
                self.mode = InsertionMode::InTable;
            }
            "area" | "br" | "embed" | "img" | "keygen" | "wbr" => {
                self.reconstruct_active_formatting();
                self.insert_void_element(&tag);
                self.frameset_ok = false;
            }
            "input" => {
                if self.fragment_context_is("select") {
                    self.error("unexpected-start-tag-in-select");
                    return Flow::Done;
                }
                if self
                    .open
                    .has_in_scope(&self.arena, "select", Scope::Default)
                {
                    self.error("unexpected-start-tag-in-select");
                    self.open.pop_until_html(&self.arena, "select");
                }
                self.reconstruct_active_formatting();
                let hidden = tag
                    .attribute("type")
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("hidden"));
                self.insert_void_element(&tag);
                if !hidden {
                    self.frameset_ok = false;
                }
            }
            "param" | "source" | "track" => self.insert_void_element(&tag),
            "hr" => {
                self.close_p_if_in_button_scope();
                if self
                    .open
                    .has_in_scope(&self.arena, "select", Scope::Default)
                {
                    self.generate_implied_end_tags(None);
                    if self
                        .open
                        .has_in_scope(&self.arena, "option", Scope::Default)
                        || self
                            .open
                            .has_in_scope(&self.arena, "optgroup", Scope::Default)
                    {
                        self.error("unexpected-start-tag-in-select");
                    }
                }
                self.insert_void_element(&tag);
                self.frameset_ok = false;
            }
            "image" => {
                self.error("unexpected-start-tag-treated-as");
                tag.name = "img".to_string();
                return Flow::Reprocess(Tok::StartTag(tag));
            }
            "textarea" => {
                self.parse_text_element(&tag);
                self.ignore_next_line_feed = true;
                self.frameset_ok = false;
            }
            "xmp" => {
                self.close_p_if_in_button_scope();
                self.reconstruct_active_formatting();
                self.frameset_ok = false;
                self.parse_text_element(&tag);
            }
            "iframe" => {
                self.frameset_ok = false;
                self.parse_text_element(&tag);
            }
            "noembed" => self.parse_text_element(&tag),
            "noscript" if self.scripting == HtmlScriptingMode::Enabled => {
                self.parse_text_element(&tag)
            }
            "select" => {
                if self.fragment_context_is("select") {
                    self.error("unexpected-start-tag-in-select");
                } else if self
                    .open
                    .has_in_scope(&self.arena, "select", Scope::Default)
                {
                    // A nested <select> closes the open one instead.
                    self.error("unexpected-start-tag-in-select");
                    self.open.pop_until_html(&self.arena, "select");
                } else {
                    self.reconstruct_active_formatting();
                    self.insert_html_element(&tag);
                    self.frameset_ok = false;
                }
            }
            "option" => {
                if self
                    .open
                    .has_in_scope(&self.arena, "select", Scope::Default)
                {
                    self.generate_implied_end_tags(Some("optgroup"));
                    if self
                        .open
                        .has_in_scope(&self.arena, "option", Scope::Default)
                    {
                        self.error("unexpected-start-tag-in-select");
                    }
                } else if self.current_is("option") {
                    self.open.pop(&self.arena);
                }
                self.reconstruct_active_formatting();
                self.insert_html_element(&tag);
            }
            "optgroup" => {
                if self
                    .open
                    .has_in_scope(&self.arena, "select", Scope::Default)
                {
                    self.generate_implied_end_tags(None);
                    if self
                        .open
                        .has_in_scope(&self.arena, "option", Scope::Default)
                        || self
                            .open
                            .has_in_scope(&self.arena, "optgroup", Scope::Default)
                    {
                        self.error("unexpected-start-tag-in-select");
                    }
                } else if self.current_is("option") {
                    self.open.pop(&self.arena);
                }
                self.reconstruct_active_formatting();
                self.insert_html_element(&tag);
            }
            "rb" | "rtc" => {
                if self.open.has_in_scope(&self.arena, "ruby", Scope::Default) {
                    self.generate_implied_end_tags(None);
                    if !self.current_is("ruby") {
                        self.error("unexpected-start-tag");
                    }
                }
                self.insert_html_element(&tag);
            }
            "rp" | "rt" => {
                if self.open.has_in_scope(&self.arena, "ruby", Scope::Default) {
                    self.generate_implied_end_tags(Some("rtc"));
                    if !self.current_is_any(&["rtc", "ruby"]) {
                        self.error("unexpected-start-tag");
                    }
                }
                self.insert_html_element(&tag);
            }
            "math" => {
                self.reconstruct_active_formatting();
                adjust_attributes(&mut tag, adjust_mathml_attribute);
                adjust_attributes(&mut tag, adjust_foreign_attribute);
                self.insert_element(Namespace::MathMl, &tag);
                if tag.self_closing {
                    self.open.pop(&self.arena);
                    self.self_closing_acknowledged = true;
                }
            }
            "svg" => {
                self.reconstruct_active_formatting();
                adjust_attributes(&mut tag, adjust_svg_attribute);
                adjust_attributes(&mut tag, adjust_foreign_attribute);
                self.insert_element(Namespace::Svg, &tag);
                if tag.self_closing {
                    self.open.pop(&self.arena);
                    self.self_closing_acknowledged = true;
                }
            }
            "caption" | "col" | "colgroup" | "frame" | "head" | "tbody" | "td" | "tfoot" | "th"
            | "thead" | "tr" => self.error("unexpected-start-tag-ignored"),
            _ => {
                self.reconstruct_active_formatting();
                self.insert_html_element(&tag);
            }
        }
        Flow::Done
    }

    /// The shared walk of the `li` and `dd`/`dt` start-tag rules: close the
    /// nearest open list item of the same kind, unless a special element
    /// (other than `address`, `div`, `p`) comes first.
    fn close_list_item(&mut self, names: &[&str]) {
        for index in (0..self.open.len()).rev() {
            let node = self.open.get(index).expect("index is in range");
            if let Some(name) = names.iter().find(|name| self.arena.is_html(node, name)) {
                self.generate_implied_end_tags(Some(name));
                if !self.current_is(name) {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_html(&self.arena, name);
                return;
            }
            if self.is_special_node(node)
                && !["address", "div", "p"]
                    .iter()
                    .any(|name| self.arena.is_html(node, name))
            {
                return;
            }
        }
    }

    fn in_body_end_tag(&mut self, name: String) -> Flow {
        match name.as_str() {
            "template" => return self.in_head(Tok::EndTag(name)),
            "body" | "html" => {
                if !self.open.has_in_scope(&self.arena, "body", Scope::Default) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.check_open_elements_at_end();
                self.mode = InsertionMode::AfterBody;
                if name == "html" {
                    return Flow::Reprocess(Tok::EndTag(name));
                }
            }
            "address" | "article" | "aside" | "blockquote" | "button" | "center" | "details"
            | "dialog" | "dir" | "div" | "dl" | "fieldset" | "figcaption" | "figure" | "footer"
            | "header" | "hgroup" | "listing" | "main" | "menu" | "nav" | "ol" | "pre"
            | "search" | "section" | "select" | "summary" | "ul" => {
                if !self.open.has_in_scope(&self.arena, &name, Scope::Default) {
                    self.error("end-tag-too-early");
                    return Flow::Done;
                }
                self.generate_implied_end_tags(None);
                if !self.current_is(&name) {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_html(&self.arena, &name);
            }
            "form" => {
                if self.open.contains_html("template") {
                    if !self.open.has_in_scope(&self.arena, "form", Scope::Default) {
                        self.error("unexpected-end-tag");
                        return Flow::Done;
                    }
                    self.generate_implied_end_tags(None);
                    if !self.current_is("form") {
                        self.error("end-tag-too-early");
                    }
                    self.open.pop_until_html(&self.arena, "form");
                } else {
                    let node = self.form.take();
                    let Some(node) = node.filter(|&node| {
                        self.open
                            .has_node_in_scope(&self.arena, node, Scope::Default)
                    }) else {
                        self.error("unexpected-end-tag");
                        return Flow::Done;
                    };
                    self.generate_implied_end_tags(None);
                    if self.current() != Some(node) {
                        self.error("end-tag-too-early-ignored");
                    }
                    self.open.remove(&self.arena, node);
                }
            }
            "p" => {
                if !self.open.has_in_scope(&self.arena, "p", Scope::Button) {
                    self.error("unexpected-end-tag");
                    self.insert_html_element(&Tag::named("p"));
                }
                self.close_p_element();
            }
            "li" => {
                if !self.open.has_in_scope(&self.arena, "li", Scope::ListItem) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.generate_implied_end_tags(Some("li"));
                if !self.current_is("li") {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_html(&self.arena, "li");
            }
            "dd" | "dt" => {
                if !self.open.has_in_scope(&self.arena, &name, Scope::Default) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.generate_implied_end_tags(Some(&name));
                if !self.current_is(&name) {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_html(&self.arena, &name);
            }
            heading if is_heading(heading) => {
                const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
                if !self
                    .open
                    .has_any_in_scope(&self.arena, &HEADINGS, Scope::Default)
                {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.generate_implied_end_tags(None);
                if !self.current_is(heading) {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_any_html(&self.arena, &HEADINGS);
            }
            formatting if is_formatting(formatting) => {
                if !self.adoption_agency(formatting) {
                    self.any_other_end_tag(formatting);
                }
            }
            "applet" | "marquee" | "object" => {
                if !self.open.has_in_scope(&self.arena, &name, Scope::Default) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.generate_implied_end_tags(None);
                if !self.current_is(&name) {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_html(&self.arena, &name);
                self.formatting.clear_to_last_marker();
            }
            "br" => {
                self.error("unexpected-end-tag-treated-as");
                return self.in_body_start_tag(Tag::named("br"));
            }
            _ => self.any_other_end_tag(&name),
        }
        Flow::Done
    }

    /// The "any other end tag" entry of the in-body rules.
    fn any_other_end_tag(&mut self, name: &str) {
        for index in (0..self.open.len()).rev() {
            let node = self.open.get(index).expect("index is in range");
            if self.arena.is_html(node, name) {
                self.generate_implied_end_tags(Some(name));
                if self.current() != Some(node) {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_node(&self.arena, node);
                return;
            }
            if self.is_special_node(node) {
                self.error("unexpected-end-tag");
                return;
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.8 The "text" insertion mode
    // ----------------------------------------------------------------------

    fn text(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(_, text) => {
                self.insert_text(&text);
                Flow::Done
            }
            Tok::Eof => {
                self.error("expected-named-closing-tag-but-got-eof");
                self.open.pop(&self.arena);
                self.mode = self.original_mode;
                Flow::Reprocess(Tok::Eof)
            }
            Tok::EndTag(_) => {
                self.open.pop(&self.arena);
                self.mode = self.original_mode;
                self.tokenizer_request = Some(HtmlLexContext::data());
                Flow::Done
            }
            // The tokenizer emits nothing else while it is in a text state.
            _ => Flow::Done,
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.9 The "in table" insertion mode
    // ----------------------------------------------------------------------

    fn current_is_html_any(&self, names: &[&str]) -> bool {
        self.current_is_any(names)
    }

    /// "Clear the stack back to" a context: pop while the current node is not
    /// one of `names` (or `html`, which ends every such loop).
    fn clear_stack_back_to(&mut self, names: &[&str]) {
        while let Some(current) = self.current() {
            if self.arena.is_html(current, "html")
                || names.iter().any(|name| self.arena.is_html(current, name))
            {
                break;
            }
            self.open.pop(&self.arena);
        }
    }

    fn clear_to_table_context(&mut self) {
        self.clear_stack_back_to(&["table", "template"]);
    }

    fn clear_to_table_body_context(&mut self) {
        self.clear_stack_back_to(&["tbody", "tfoot", "thead", "template"]);
    }

    fn clear_to_table_row_context(&mut self) {
        self.clear_stack_back_to(&["tr", "template"]);
    }

    fn in_table(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(..)
                if self.current_is_html_any(&[
                    "table", "tbody", "template", "tfoot", "thead", "tr",
                ]) =>
            {
                self.pending_table_text.clear();
                self.original_mode = self.mode;
                self.mode = InsertionMode::InTableText;
                Flow::Reprocess(token)
            }
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None);
                Flow::Done
            }
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "caption" => {
                self.clear_to_table_context();
                self.formatting.push_marker();
                self.insert_html_element(&tag);
                self.mode = InsertionMode::InCaption;
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "colgroup" => {
                self.clear_to_table_context();
                self.insert_html_element(&tag);
                self.mode = InsertionMode::InColumnGroup;
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "col" => {
                self.clear_to_table_context();
                self.insert_html_element(&Tag::named("colgroup"));
                self.mode = InsertionMode::InColumnGroup;
                Flow::Reprocess(token)
            }
            Tok::StartTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.clear_to_table_context();
                self.insert_html_element(&tag);
                self.mode = InsertionMode::InTableBody;
                Flow::Done
            }
            Tok::StartTag(ref tag) if matches!(tag.name.as_str(), "td" | "th" | "tr") => {
                self.clear_to_table_context();
                self.insert_html_element(&Tag::named("tbody"));
                self.mode = InsertionMode::InTableBody;
                Flow::Reprocess(token)
            }
            Tok::StartTag(ref tag) if tag.name == "table" => {
                self.error("unexpected-start-tag-implies-end-tag");
                if !self.open.has_in_scope(&self.arena, "table", Scope::Table) {
                    return Flow::Done;
                }
                self.open.pop_until_html(&self.arena, "table");
                self.reset_insertion_mode();
                Flow::Reprocess(token)
            }
            Tok::EndTag(ref name) if name == "table" => {
                if !self.open.has_in_scope(&self.arena, "table", Scope::Table) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.open.pop_until_html(&self.arena, "table");
                self.reset_insertion_mode();
                Flow::Done
            }
            Tok::EndTag(ref name)
                if matches!(
                    name.as_str(),
                    "body"
                        | "caption"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            Tok::StartTag(ref tag)
                if matches!(tag.name.as_str(), "style" | "script" | "template") =>
            {
                self.in_head(token)
            }
            Tok::EndTag(ref name) if name == "template" => self.in_head(token),
            Tok::StartTag(ref tag)
                if tag.name == "input"
                    && tag
                        .attribute("type")
                        .is_some_and(|kind| kind.eq_ignore_ascii_case("hidden")) =>
            {
                self.error("unexpected-hidden-input-in-table");
                let Tok::StartTag(tag) = token else {
                    unreachable!("matched a start tag")
                };
                self.insert_void_element(&tag);
                Flow::Done
            }
            Tok::StartTag(tag) if tag.name == "form" => {
                self.error("unexpected-form-in-table");
                let in_template = self.open.contains_html("template");
                if self.form.is_some() && !in_template {
                    return Flow::Done;
                }
                let form = self.insert_html_element(&tag);
                if !in_template {
                    self.form = Some(form);
                }
                self.open.pop(&self.arena);
                Flow::Done
            }
            Tok::Eof => self.in_body(token),
            other => self.in_table_anything_else(other),
        }
    }

    /// The "anything else" entry of "in table": the in-body rules, with
    /// foster parenting on, so content lands before the table.
    fn in_table_anything_else(&mut self, token: Tok) -> Flow {
        self.error("unexpected-token-in-table");
        self.foster_parenting = true;
        let flow = self.in_body(token);
        self.foster_parenting = false;
        flow
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.10 The "in table text" insertion mode
    // ----------------------------------------------------------------------

    fn in_table_text(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Null, _) => {
                self.error("unexpected-null-character");
                Flow::Done
            }
            Tok::Text(kind, text) => {
                self.pending_table_text.push((kind, text));
                Flow::Done
            }
            other => {
                let pending = std::mem::take(&mut self.pending_table_text);
                if pending
                    .iter()
                    .any(|(kind, _)| *kind != TextKind::Whitespace)
                {
                    // Non-whitespace in a table is foster-parented, runs and all.
                    for (kind, text) in pending {
                        let flow = self.in_table_anything_else(Tok::Text(kind, text));
                        debug_assert!(matches!(flow, Flow::Done));
                    }
                } else {
                    for (_, text) in pending {
                        self.insert_text(&text);
                    }
                }
                self.mode = self.original_mode;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.11 The "in caption" insertion mode
    // ----------------------------------------------------------------------

    /// The shared steps of `</caption>` and the tokens that imply it. Returns
    /// whether a caption was closed.
    fn close_caption(&mut self) -> bool {
        if !self.open.has_in_scope(&self.arena, "caption", Scope::Table) {
            self.error("unexpected-end-tag");
            return false;
        }
        self.generate_implied_end_tags(None);
        if !self.current_is("caption") {
            self.error("end-tag-too-early");
        }
        self.open.pop_until_html(&self.arena, "caption");
        self.formatting.clear_to_last_marker();
        self.mode = InsertionMode::InTable;
        true
    }

    fn in_caption(&mut self, token: Tok) -> Flow {
        match token {
            Tok::EndTag(ref name) if name == "caption" => {
                self.close_caption();
                Flow::Done
            }
            Tok::StartTag(ref tag)
                if matches!(
                    tag.name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                if self.close_caption() {
                    Flow::Reprocess(token)
                } else {
                    Flow::Done
                }
            }
            Tok::EndTag(ref name) if name == "table" => {
                if self.close_caption() {
                    Flow::Reprocess(token)
                } else {
                    Flow::Done
                }
            }
            Tok::EndTag(ref name)
                if matches!(
                    name.as_str(),
                    "body"
                        | "col"
                        | "colgroup"
                        | "html"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            other => self.in_body(other),
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.12 The "in column group" insertion mode
    // ----------------------------------------------------------------------

    fn in_column_group(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, text) => {
                self.insert_text(&text);
                Flow::Done
            }
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None);
                Flow::Done
            }
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::StartTag(tag) if tag.name == "col" => {
                self.insert_void_element(&tag);
                Flow::Done
            }
            Tok::EndTag(ref name) if name == "colgroup" => {
                if !self.current_is("colgroup") {
                    self.error("unexpected-end-tag");
                } else {
                    self.open.pop(&self.arena);
                    self.mode = InsertionMode::InTable;
                }
                Flow::Done
            }
            Tok::EndTag(ref name) if name == "col" => {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "template" => self.in_head(token),
            Tok::EndTag(ref name) if name == "template" => self.in_head(token),
            Tok::Eof => self.in_body(token),
            other => {
                if !self.current_is("colgroup") {
                    self.error("unexpected-token-in-column-group");
                    return Flow::Done;
                }
                self.open.pop(&self.arena);
                self.mode = InsertionMode::InTable;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.13 The "in table body" insertion mode
    // ----------------------------------------------------------------------

    fn in_table_body(&mut self, token: Tok) -> Flow {
        match token {
            Tok::StartTag(tag) if tag.name == "tr" => {
                self.clear_to_table_body_context();
                self.insert_html_element(&tag);
                self.mode = InsertionMode::InRow;
                Flow::Done
            }
            Tok::StartTag(ref tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.error("unexpected-cell-in-table-body");
                self.clear_to_table_body_context();
                self.insert_html_element(&Tag::named("tr"));
                self.mode = InsertionMode::InRow;
                Flow::Reprocess(token)
            }
            Tok::EndTag(ref name) if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self.open.has_in_scope(&self.arena, name, Scope::Table) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.clear_to_table_body_context();
                self.open.pop(&self.arena);
                self.mode = InsertionMode::InTable;
                Flow::Done
            }
            Tok::StartTag(ref tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead"
                ) =>
            {
                self.leave_table_body(token)
            }
            Tok::EndTag(ref name) if name == "table" => self.leave_table_body(token),
            Tok::EndTag(ref name)
                if matches!(
                    name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr"
                ) =>
            {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            other => self.in_table(other),
        }
    }

    /// Close the open table section, then reprocess in "in table".
    fn leave_table_body(&mut self, token: Tok) -> Flow {
        if !self
            .open
            .has_any_in_scope(&self.arena, &["tbody", "thead", "tfoot"], Scope::Table)
        {
            self.error("unexpected-token-in-table-body");
            return Flow::Done;
        }
        self.clear_to_table_body_context();
        self.open.pop(&self.arena);
        self.mode = InsertionMode::InTable;
        Flow::Reprocess(token)
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.14 The "in row" insertion mode
    // ----------------------------------------------------------------------

    /// Close the open `tr` (the shared steps of `</tr>` and the tokens that
    /// imply it). Returns whether one was closed.
    fn close_row(&mut self) -> bool {
        if !self.open.has_in_scope(&self.arena, "tr", Scope::Table) {
            self.error("unexpected-end-tag");
            return false;
        }
        self.clear_to_table_row_context();
        self.open.pop(&self.arena);
        self.mode = InsertionMode::InTableBody;
        true
    }

    fn in_row(&mut self, token: Tok) -> Flow {
        match token {
            Tok::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.clear_to_table_row_context();
                self.insert_html_element(&tag);
                self.mode = InsertionMode::InCell;
                self.formatting.push_marker();
                Flow::Done
            }
            Tok::EndTag(ref name) if name == "tr" => {
                self.close_row();
                Flow::Done
            }
            Tok::StartTag(ref tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr"
                ) =>
            {
                if self.close_row() {
                    Flow::Reprocess(token)
                } else {
                    Flow::Done
                }
            }
            Tok::EndTag(ref name) if name == "table" => {
                if self.close_row() {
                    Flow::Reprocess(token)
                } else {
                    Flow::Done
                }
            }
            Tok::EndTag(ref name) if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self.open.has_in_scope(&self.arena, name, Scope::Table) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                if !self.open.has_in_scope(&self.arena, "tr", Scope::Table) {
                    return Flow::Done;
                }
                self.clear_to_table_row_context();
                self.open.pop(&self.arena);
                self.mode = InsertionMode::InTableBody;
                Flow::Reprocess(token)
            }
            Tok::EndTag(ref name)
                if matches!(
                    name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th"
                ) =>
            {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            other => self.in_table(other),
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.15 The "in cell" insertion mode
    // ----------------------------------------------------------------------

    /// "Close the cell".
    fn close_cell(&mut self) {
        self.generate_implied_end_tags(None);
        if !self.current_is_any(&["td", "th"]) {
            self.error("end-tag-too-early");
        }
        self.open.pop_until_any_html(&self.arena, &["td", "th"]);
        self.formatting.clear_to_last_marker();
        self.mode = InsertionMode::InRow;
    }

    fn in_cell(&mut self, token: Tok) -> Flow {
        match token {
            Tok::EndTag(ref name) if matches!(name.as_str(), "td" | "th") => {
                if !self.open.has_in_scope(&self.arena, name, Scope::Table) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.generate_implied_end_tags(None);
                if !self.current_is(name) {
                    self.error("end-tag-too-early");
                }
                self.open.pop_until_html(&self.arena, name);
                self.formatting.clear_to_last_marker();
                self.mode = InsertionMode::InRow;
                Flow::Done
            }
            Tok::StartTag(ref tag)
                if matches!(
                    tag.name.as_str(),
                    "caption"
                        | "col"
                        | "colgroup"
                        | "tbody"
                        | "td"
                        | "tfoot"
                        | "th"
                        | "thead"
                        | "tr"
                ) =>
            {
                // The specification asserts a cell is in table scope here; if a
                // hostile stack ever breaks that, ignoring the token is what
                // keeps "close the cell and reprocess" from looping.
                if !self
                    .open
                    .has_any_in_scope(&self.arena, &["td", "th"], Scope::Table)
                {
                    self.error("unexpected-start-tag");
                    return Flow::Done;
                }
                self.close_cell();
                Flow::Reprocess(token)
            }
            Tok::EndTag(ref name)
                if matches!(
                    name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html"
                ) =>
            {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            Tok::EndTag(ref name)
                if matches!(name.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr") =>
            {
                if !self.open.has_in_scope(&self.arena, name, Scope::Table) {
                    self.error("unexpected-end-tag");
                    return Flow::Done;
                }
                self.close_cell();
                Flow::Reprocess(token)
            }
            other => self.in_body(other),
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.16 The "in template" insertion mode
    // ----------------------------------------------------------------------

    fn in_template(&mut self, token: Tok) -> Flow {
        let switch_to = |builder: &mut Self, mode: InsertionMode, token: Tok| {
            builder.template_modes.pop();
            builder.template_modes.push(mode);
            builder.mode = mode;
            Flow::Reprocess(token)
        };
        match token {
            Tok::Text(..)
            | Tok::Comment(_)
            | Tok::ProcessingInstruction { .. }
            | Tok::Doctype { .. } => self.in_body(token),
            Tok::StartTag(ref tag)
                if matches!(
                    tag.name.as_str(),
                    "base"
                        | "basefont"
                        | "bgsound"
                        | "link"
                        | "meta"
                        | "noframes"
                        | "script"
                        | "style"
                        | "template"
                        | "title"
                ) =>
            {
                self.in_head(token)
            }
            Tok::EndTag(ref name) if name == "template" => self.in_head(token),
            Tok::StartTag(ref tag) => match tag.name.as_str() {
                "caption" | "colgroup" | "tbody" | "tfoot" | "thead" => {
                    switch_to(self, InsertionMode::InTable, token)
                }
                "col" => switch_to(self, InsertionMode::InColumnGroup, token),
                "tr" => switch_to(self, InsertionMode::InTableBody, token),
                "td" | "th" => switch_to(self, InsertionMode::InRow, token),
                _ => switch_to(self, InsertionMode::InBody, token),
            },
            Tok::EndTag(_) => {
                self.error("unexpected-end-tag");
                Flow::Done
            }
            Tok::Eof => {
                if !self.open.contains_html("template") {
                    self.stopped = true;
                    return Flow::Done;
                }
                self.error("eof-in-template");
                self.open.pop_until_html(&self.arena, "template");
                self.formatting.clear_to_last_marker();
                self.template_modes.pop();
                self.reset_insertion_mode();
                Flow::Reprocess(Tok::Eof)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.17 The "after body" insertion mode
    // ----------------------------------------------------------------------

    fn after_body(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, _) => self.in_body(token),
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                let html = self.open.top();
                self.insert_comment_like(token, html);
                Flow::Done
            }
            Tok::Doctype { .. } => {
                self.error("unexpected-doctype");
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::EndTag(ref name) if name == "html" => {
                self.mode = InsertionMode::AfterAfterBody;
                Flow::Done
            }
            Tok::Eof => {
                self.stopped = true;
                Flow::Done
            }
            other => {
                self.error("unexpected-token-after-body");
                self.mode = InsertionMode::InBody;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.18 The "in frameset" insertion mode
    // ----------------------------------------------------------------------

    fn in_frameset(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, text) => self.insert_text(&text),
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None)
            }
            Tok::Doctype { .. } => self.error("unexpected-doctype"),
            Tok::StartTag(ref tag) if tag.name == "html" => return self.in_body(token),
            Tok::StartTag(tag) if tag.name == "frameset" => {
                self.insert_html_element(&tag);
            }
            Tok::EndTag(name) if name == "frameset" => {
                if self.open.len() <= 1 {
                    self.error("unexpected-end-tag");
                } else {
                    self.open.pop(&self.arena);
                    if !self.current_is("frameset") {
                        self.mode = InsertionMode::AfterFrameset;
                    }
                }
            }
            Tok::StartTag(tag) if tag.name == "frame" => self.insert_void_element(&tag),
            Tok::StartTag(ref tag) if tag.name == "noframes" => return self.in_head(token),
            Tok::Eof => {
                if self.open.len() > 1 {
                    self.error("eof-in-frameset");
                }
                self.stopped = true;
            }
            _ => self.error("unexpected-token-in-frameset"),
        }
        Flow::Done
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.19 The "after frameset" insertion mode
    // ----------------------------------------------------------------------

    fn after_frameset(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Text(TextKind::Whitespace, text) => self.insert_text(&text),
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, None)
            }
            Tok::Doctype { .. } => self.error("unexpected-doctype"),
            Tok::StartTag(ref tag) if tag.name == "html" => return self.in_body(token),
            Tok::EndTag(name) if name == "html" => self.mode = InsertionMode::AfterAfterFrameset,
            Tok::StartTag(ref tag) if tag.name == "noframes" => return self.in_head(token),
            Tok::Eof => self.stopped = true,
            _ => self.error("unexpected-token-after-frameset"),
        }
        Flow::Done
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.20 The "after after body" insertion mode
    // ----------------------------------------------------------------------

    fn after_after_body(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, Some(Arena::DOCUMENT));
                Flow::Done
            }
            Tok::Doctype { .. } | Tok::Text(TextKind::Whitespace, _) => self.in_body(token),
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::Eof => {
                self.stopped = true;
                Flow::Done
            }
            other => {
                self.error("expected-eof-but-got-other");
                self.mode = InsertionMode::InBody;
                Flow::Reprocess(other)
            }
        }
    }

    // ----------------------------------------------------------------------
    // §13.2.6.4.21 The "after after frameset" insertion mode
    // ----------------------------------------------------------------------

    fn after_after_frameset(&mut self, token: Tok) -> Flow {
        match token {
            Tok::Comment(_) | Tok::ProcessingInstruction { .. } => {
                self.insert_comment_like(token, Some(Arena::DOCUMENT));
                Flow::Done
            }
            Tok::Doctype { .. } | Tok::Text(TextKind::Whitespace, _) => self.in_body(token),
            Tok::StartTag(ref tag) if tag.name == "html" => self.in_body(token),
            Tok::Eof => {
                self.stopped = true;
                Flow::Done
            }
            Tok::StartTag(ref tag) if tag.name == "noframes" => self.in_head(token),
            _ => {
                self.error("expected-eof-but-got-other");
                Flow::Done
            }
        }
    }
}

/// Rename attributes by one of the adjustment tables in `elements`.
fn adjust_attributes(tag: &mut Tag, adjust: fn(&str) -> Option<&'static str>) {
    for attribute in &mut tag.attributes {
        if let Some(adjusted) = adjust(&attribute.name) {
            attribute.name = adjusted.to_string();
        }
    }
}

/// Split a text token into maximal runs of one [`TextKind`].
fn split_runs(text: &str) -> Vec<(TextKind, String)> {
    let kind_of = |character: char| {
        if character == '\0' {
            TextKind::Null
        } else if is_html_whitespace(character) {
            TextKind::Whitespace
        } else {
            TextKind::Other
        }
    };
    let mut runs: Vec<(TextKind, String)> = Vec::new();
    for character in text.chars() {
        let kind = kind_of(character);
        match runs.last_mut() {
            Some((last, run)) if *last == kind => run.push(character),
            _ => runs.push((kind, character.to_string())),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_runs_groups_by_kind() {
        assert_eq!(
            split_runs(" \nab\0\0c "),
            vec![
                (TextKind::Whitespace, " \n".to_string()),
                (TextKind::Other, "ab".to_string()),
                (TextKind::Null, "\0\0".to_string()),
                (TextKind::Other, "c".to_string()),
                (TextKind::Whitespace, " ".to_string()),
            ]
        );
        assert!(split_runs("").is_empty());
    }
}
