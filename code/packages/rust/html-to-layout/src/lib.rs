//! Convert [`BrowserRenderTree`] values from `html-parser` into the shared
//! [`LayoutNode`] intermediate representation.
//!
//! This crate is deliberately a front-end adapter. Parsing stays in
//! `html-parser`; geometry stays in layout engines such as `layout-block`; and
//! painting stays in `layout-to-paint`.

use std::cmp::Ordering;
use std::collections::HashMap;

use coding_adventures_css_parser::create_css_parser;
use coding_adventures_html_parser::{BrowserRenderNode, BrowserRenderTree};
use layout_ir::{
    color_black, edges_all, edges_xy, font_bold, font_italic, font_spec, rgb, Color, Edges,
    ExtValue, FontSpec, ImageContent, ImageFit, LayoutNode, SizeValue, TextAlign, TextContent,
    TextDecoration,
};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

pub const VERSION: &str = "0.4.0";

/// A parsed author stylesheet ready for deterministic cascade evaluation.
///
/// CSS syntax is validated by the shared grammar-backed parser. This adapter
/// only performs the semantic projection needed by HTML layout; hosts never
/// parse selectors or declarations themselves.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlAuthorStylesheet {
    rules: Vec<StyleRule>,
    imports: Vec<HtmlStylesheetImport>,
}

/// One grammar-validated `@import`, still independent from fetch policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlStylesheetImport {
    pub href: String,
    pub media: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlStyleError {
    pub message: String,
}

impl std::fmt::Display for HtmlStyleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for HtmlStyleError {}

impl HtmlAuthorStylesheet {
    pub fn parse(source: &str) -> Result<Self, HtmlStyleError> {
        let ast = create_css_parser(source)
            .parse()
            .map_err(|error| HtmlStyleError {
                message: error.to_string(),
            })?;
        let mut rules = Vec::new();
        collect_style_rules(&ast, &[], &mut rules);
        let mut imports = Vec::new();
        collect_stylesheet_imports(&ast, &mut imports);
        Ok(Self { rules, imports })
    }

    pub fn imports(&self) -> &[HtmlStylesheetImport] {
        &self.imports
    }
}

/// Reusable UA/author cascade input consumed by HTML layout and paint.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlStyleContext {
    pub theme: HtmlTheme,
    pub author_stylesheets: Vec<HtmlAuthorStylesheet>,
    pub viewport_width: f64,
    pub viewport_height: f64,
}

impl HtmlStyleContext {
    pub fn new(theme: HtmlTheme) -> Self {
        Self {
            theme,
            author_stylesheets: Vec::new(),
            viewport_width: 800.0,
            viewport_height: 600.0,
        }
    }

    pub fn with_viewport(mut self, width: f64, height: f64) -> Self {
        self.viewport_width = finite_non_negative(width);
        self.viewport_height = finite_non_negative(height);
        self
    }

    pub fn with_author_stylesheets(
        theme: HtmlTheme,
        sources: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, HtmlStyleError> {
        let author_stylesheets = sources
            .into_iter()
            .map(|source| HtmlAuthorStylesheet::parse(source.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            theme,
            author_stylesheets,
            viewport_width: 800.0,
            viewport_height: 600.0,
        })
    }
}

/// The resolved visual values at the layout boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlComputedStyle {
    pub font: FontSpec,
    pub color: Color,
    pub decoration: Option<TextDecoration>,
    pub display: Option<String>,
    pub background: Option<Color>,
    pub width: Option<f64>,
    pub width_percent: Option<f64>,
    pub width_auto: bool,
    pub height: Option<f64>,
    pub min_width: Option<f64>,
    pub max_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_height: Option<f64>,
    pub margin: Option<Edges>,
    pub padding: Option<Edges>,
    pub margin_auto: [bool; 4],
    pub border_width: Edges,
    pub border_color: [Option<Color>; 4],
    pub box_sizing: String,
    pub text_align: TextAlign,
    pub white_space: String,
    pub custom_properties: HashMap<String, Vec<String>>,
}

/// Fully resolved visual defaults applied before a future CSS cascade exists.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlTheme {
    pub body_font: FontSpec,
    pub heading_fonts: [FontSpec; 6],
    pub code_font: FontSpec,
    pub text_color: Color,
    pub heading_color: Color,
    pub link_color: Color,
    pub visited_link_color: Color,
    pub link_decoration: Option<TextDecoration>,
    pub page_background: Color,
    pub page_padding: f64,
    pub block_spacing: f64,
    pub heading_spacing: f64,
}

/// The recognizable defaults used by Mosaic-era HTML documents.
pub fn mosaic_html_theme() -> HtmlTheme {
    HtmlTheme {
        body_font: FontSpec {
            line_height: 1.4,
            ..font_spec("Times New Roman", 14.0)
        },
        heading_fonts: [
            font_bold(font_spec("Times New Roman", 24.0)),
            font_bold(font_spec("Times New Roman", 20.0)),
            font_bold(font_spec("Times New Roman", 18.0)),
            font_bold(font_spec("Times New Roman", 16.0)),
            font_bold(font_spec("Times New Roman", 14.0)),
            font_bold(font_spec("Times New Roman", 12.0)),
        ],
        code_font: font_spec("Courier New", 13.0),
        text_color: color_black(),
        heading_color: color_black(),
        link_color: rgb(0, 0, 238),
        visited_link_color: rgb(85, 26, 139),
        link_decoration: Some(TextDecoration::underline()),
        page_background: rgb(192, 192, 192),
        page_padding: 16.0,
        block_spacing: 12.0,
        heading_spacing: 12.0,
    }
}

/// Evaluate the shared screen-media profile for a concrete logical viewport.
/// Loading and cascade callers use this same function so inactive resources
/// and inactive rules cannot disagree.
pub fn html_media_query_applies(media: Option<&str>, width: f64, height: f64) -> bool {
    let Some(media) = media.map(str::trim).filter(|media| !media.is_empty()) else {
        return true;
    };
    media_query_list_applies(&[media.to_string()], width, height)
}

/// Convert a browser render tree into a shared layout tree.
///
/// The returned root fills the available width, wraps its content height, and
/// carries the theme's page background in the `paint` extension namespace.
pub fn html_render_tree_to_layout(
    render_tree: &BrowserRenderTree,
    theme: &HtmlTheme,
) -> LayoutNode {
    html_render_tree_to_layout_with_link_state(render_tree, theme, &never_visited)
}

/// Convert a browser render tree while resolving session-owned link state.
///
/// The callback is intentionally narrower than browser history: HTML layout
/// receives only a resolved URL and a visited answer, so navigation policy and
/// storage remain reusable and independent from layout.
pub fn html_render_tree_to_layout_with_link_state<F>(
    render_tree: &BrowserRenderTree,
    theme: &HtmlTheme,
    is_visited: &F,
) -> LayoutNode
where
    F: Fn(&str) -> bool + ?Sized,
{
    html_render_tree_to_layout_with_style_context(
        render_tree,
        &HtmlStyleContext::new(theme.clone()),
        is_visited,
    )
}

/// Convert HTML through the reusable UA/author computed-style boundary.
pub fn html_render_tree_to_layout_with_style_context<F>(
    render_tree: &BrowserRenderTree,
    context: &HtmlStyleContext,
    is_visited: &F,
) -> LayoutNode
where
    F: Fn(&str) -> bool + ?Sized,
{
    let theme = &context.theme;
    let style = root_computed_style(context);
    let ancestors = Vec::new();
    let children = convert_children(
        &render_tree.children,
        context,
        &style,
        &ancestors,
        is_visited,
    );

    let mut root = LayoutNode::container(children)
        .with_padding(
            style
                .padding
                .unwrap_or_else(|| edges_all(theme.page_padding)),
        )
        .with_width(SizeValue::Fill)
        .with_height(SizeValue::Wrap)
        .with_ext("block", display_ext("block"))
        .with_ext("html", root_html_ext());
    root.ext.insert(
        "paint".into(),
        background_ext(style.background.unwrap_or(theme.page_background)),
    );
    root
}

fn convert_node<F>(
    node: &BrowserRenderNode,
    context: &HtmlStyleContext,
    inherited: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    position: Option<NodePosition>,
    is_visited: &F,
) -> Option<LayoutNode>
where
    F: Fn(&str) -> bool + ?Sized,
{
    let style = style_for_node(node, context, inherited, ancestors, position, is_visited);
    let display = style.display.as_deref().unwrap_or(&node.display);
    if display == "none" || node.hidden {
        return None;
    }

    let mut next_ancestors = ancestors.to_vec();
    next_ancestors.push(node);
    let mut layout = match display {
        "inline-text" => text_leaf(node.text.as_deref().unwrap_or_default(), &style),
        "line-break" => text_leaf("\n", &style),
        "inline-replaced" if node.role == "image" => image_leaf(node),
        _ => container_or_fallback(node, context, &style, &next_ancestors, is_visited),
    };

    if let Some(id) = &node.id {
        layout = layout.with_id(id);
    }
    apply_size_hints(&mut layout, node, &style, display);
    apply_spacing(&mut layout, node, &context.theme, &style);
    if let Some(padding) = style.padding {
        layout.padding = Some(padding);
    }
    if style.background.is_some() || style.border_width != Edges::default() {
        layout.ext.insert("paint".into(), box_paint_ext(&style));
    }
    layout.ext.insert("html".into(), html_ext(node));
    layout
        .ext
        .insert("block".into(), block_ext(node, display, &style));
    Some(layout)
}

fn convert_children<F>(
    nodes: &[BrowserRenderNode],
    context: &HtmlStyleContext,
    inherited: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    is_visited: &F,
) -> Vec<LayoutNode>
where
    F: Fn(&str) -> bool + ?Sized,
{
    let element_count = nodes.iter().filter(|node| node.name.is_some()).count();
    let mut element_index = 0;
    nodes
        .iter()
        .filter_map(|node| {
            let position = node.name.as_ref().map(|_| {
                element_index += 1;
                NodePosition {
                    index: element_index,
                    count: element_count,
                }
            });
            convert_node(node, context, inherited, ancestors, position, is_visited)
        })
        .collect()
}

fn container_or_fallback<F>(
    node: &BrowserRenderNode,
    context: &HtmlStyleContext,
    style: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    is_visited: &F,
) -> LayoutNode
where
    F: Fn(&str) -> bool + ?Sized,
{
    let children = convert_children(&node.children, context, style, ancestors, is_visited);

    if !children.is_empty() {
        return LayoutNode::container(children);
    }

    let fallback = node
        .text
        .as_deref()
        .or(node.accessible_name.as_deref())
        .or(node.value.as_deref())
        .or(node.alt.as_deref());
    match fallback {
        Some(text) if !text.is_empty() => text_leaf(text, style),
        _ => LayoutNode::empty(),
    }
}

fn text_leaf(value: &str, style: &HtmlComputedStyle) -> LayoutNode {
    LayoutNode::leaf_text(TextContent {
        value: value.to_string(),
        font: style.font.clone(),
        color: style.color,
        decoration: style.decoration,
        max_lines: None,
        wrap: !matches!(style.white_space.as_str(), "nowrap" | "pre"),
        text_align: style.text_align,
    })
    .with_width(SizeValue::Wrap)
    .with_height(SizeValue::Wrap)
}

fn image_leaf(node: &BrowserRenderNode) -> LayoutNode {
    let source = node
        .resolved_src
        .as_deref()
        .or(node.src.as_deref())
        .unwrap_or_default();
    LayoutNode::leaf_image(ImageContent {
        src: source.to_string(),
        fit: ImageFit::Contain,
    })
}

fn style_for_node<F>(
    node: &BrowserRenderNode,
    context: &HtmlStyleContext,
    inherited: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    position: Option<NodePosition>,
    is_visited: &F,
) -> HtmlComputedStyle
where
    F: Fn(&str) -> bool + ?Sized,
{
    let theme = &context.theme;
    let mut style = inherited.clone();
    style.display = None;
    style.background = None;
    style.width = None;
    style.width_percent = None;
    style.width_auto = false;
    style.height = None;
    style.min_width = None;
    style.max_width = None;
    style.min_height = None;
    style.max_height = None;
    style.margin = None;
    style.padding = None;
    style.margin_auto = [false; 4];
    style.border_width = Edges::default();
    style.border_color = [None; 4];
    style.box_sizing = "content-box".into();
    if node.role == "heading" {
        let level = node.heading_level.unwrap_or(1).clamp(1, 6);
        style.font = theme.heading_fonts[usize::from(level - 1)].clone();
        style.color = theme.heading_color;
    } else if node.role == "preformatted" {
        style.font = theme.code_font.clone();
    }

    match node.name.as_deref() {
        Some("b" | "strong") => style.font = font_bold(style.font),
        Some("em" | "i") => style.font = font_italic(style.font),
        _ => {}
    }
    if node.role == "link" {
        let href = node.resolved_href.as_deref().or(node.href.as_deref());
        style.color = if href.is_some_and(is_visited) {
            theme.visited_link_color
        } else {
            theme.link_color
        };
        style.decoration = theme.link_decoration;
    }
    apply_author_cascade(&mut style, node, ancestors, position, context, is_visited);
    style
}

#[derive(Clone, Debug, PartialEq)]
struct StyleRule {
    selectors: Vec<Selector>,
    declarations: Vec<Declaration>,
    media: Vec<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
struct Declaration {
    property: String,
    value: Vec<String>,
    important: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Selector {
    compounds: Vec<CompoundSelector>,
    relations: Vec<SelectorRelation>,
    specificity: (u16, u16, u16),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SelectorRelation {
    Descendant,
    Child,
    Unsupported,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct CompoundSelector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<AttributeSelector>,
    link_state: Option<LinkState>,
    structural: Vec<StructuralPseudo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AttributeSelector {
    name: String,
    operator: Option<String>,
    value: Option<String>,
    case_insensitive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StructuralPseudo {
    First,
    Last,
    Nth(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NodePosition {
    index: usize,
    count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinkState {
    Link,
    Visited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CascadePriority {
    important: bool,
    specificity: (u16, u16, u16),
    order: usize,
}

impl Ord for CascadePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.important, self.specificity, self.order).cmp(&(
            other.important,
            other.specificity,
            other.order,
        ))
    }
}

impl PartialOrd for CascadePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn root_computed_style(context: &HtmlStyleContext) -> HtmlComputedStyle {
    let mut style = HtmlComputedStyle {
        font: context.theme.body_font.clone(),
        color: context.theme.text_color,
        decoration: None,
        display: Some("block".into()),
        background: Some(context.theme.page_background),
        width: None,
        width_percent: None,
        width_auto: false,
        height: None,
        min_width: None,
        max_width: None,
        min_height: None,
        max_height: None,
        margin: None,
        padding: Some(edges_all(context.theme.page_padding)),
        margin_auto: [false; 4],
        border_width: Edges::default(),
        border_color: [None; 4],
        box_sizing: "content-box".into(),
        text_align: TextAlign::Start,
        white_space: "normal".into(),
        custom_properties: HashMap::new(),
    };
    let mut winners = HashMap::new();
    let mut order = 0;
    for stylesheet in &context.author_stylesheets {
        for rule in &stylesheet.rules {
            if !rule_media_applies(rule, context) {
                continue;
            }
            for selector in &rule.selectors {
                if selector.matches_virtual_body() {
                    record_declarations(
                        &mut winners,
                        &rule.declarations,
                        selector.specificity,
                        &mut order,
                    );
                }
            }
        }
    }
    apply_declaration_winners(&mut style, winners, context);
    style
}

fn apply_author_cascade<F>(
    style: &mut HtmlComputedStyle,
    node: &BrowserRenderNode,
    ancestors: &[&BrowserRenderNode],
    position: Option<NodePosition>,
    context: &HtmlStyleContext,
    is_visited: &F,
) where
    F: Fn(&str) -> bool + ?Sized,
{
    let mut winners = HashMap::new();
    let mut order = 0;
    for stylesheet in &context.author_stylesheets {
        for rule in &stylesheet.rules {
            if !rule_media_applies(rule, context) {
                continue;
            }
            for selector in &rule.selectors {
                if selector.matches(node, ancestors, position, is_visited) {
                    record_declarations(
                        &mut winners,
                        &rule.declarations,
                        selector.specificity,
                        &mut order,
                    );
                }
            }
        }
    }
    if let Some(inline_style) = node.style.as_deref() {
        if let Ok(declarations) = parse_inline_declarations(inline_style) {
            record_declarations(&mut winners, &declarations, (u16::MAX, 0, 0), &mut order);
        }
    }
    apply_declaration_winners(style, winners, context);
}

fn record_declarations(
    winners: &mut HashMap<String, (CascadePriority, Vec<String>)>,
    declarations: &[Declaration],
    specificity: (u16, u16, u16),
    order: &mut usize,
) {
    for declaration in declarations {
        for (property, value) in expand_declaration(declaration) {
            let priority = CascadePriority {
                important: declaration.important,
                specificity,
                order: *order,
            };
            *order += 1;
            let should_replace = winners
                .get(&property)
                .is_none_or(|(current, _)| priority >= *current);
            if should_replace {
                winners.insert(property, (priority, value));
            }
        }
    }
}

fn apply_declaration_winners(
    style: &mut HtmlComputedStyle,
    winners: HashMap<String, (CascadePriority, Vec<String>)>,
    context: &HtmlStyleContext,
) {
    let inherited_font_size = style.font.size;
    let mut custom_winners = winners
        .iter()
        .filter(|(property, _)| property.starts_with("--"))
        .map(|(property, (_, value))| (property.clone(), value.clone()))
        .collect::<Vec<_>>();
    custom_winners.sort_by(|left, right| left.0.cmp(&right.0));
    for (property, value) in custom_winners {
        style.custom_properties.insert(property, value);
    }
    if let Some((_, raw_value)) = winners.get("font-size") {
        if let Some(value) = resolve_css_value(raw_value, &style.custom_properties, 0) {
            if let Some(size) = parse_css_length_in_context(
                &value,
                inherited_font_size,
                context.theme.body_font.size,
                inherited_font_size,
            ) {
                style.font.size = size;
            }
        }
    }
    let mut resolved_winners = winners.into_iter().collect::<Vec<_>>();
    resolved_winners.sort_by(|left, right| left.0.cmp(&right.0));
    for (property, (_, raw_value)) in resolved_winners {
        if property.starts_with("--") {
            continue;
        }
        let Some(value) = resolve_css_value(&raw_value, &style.custom_properties, 0) else {
            continue;
        };
        match property.as_str() {
            "color" => {
                if let Some(color) = parse_color(&value) {
                    style.color = color;
                }
            }
            "background" | "background-color" => style.background = parse_color(&value),
            "font-family" => {
                if let Some(family) = value.first() {
                    style.font.family = family.trim_matches(['\'', '"']).to_string();
                }
            }
            "font-size" => {}
            "font-weight" => {
                if value.first().is_some_and(|value| value == "bold") {
                    style.font = font_bold(style.font.clone());
                } else if let Some(weight) = value.first().and_then(|value| value.parse().ok()) {
                    style.font.weight = weight;
                }
            }
            "font-style"
                if value
                    .first()
                    .is_some_and(|value| matches!(value.as_str(), "italic" | "oblique")) =>
            {
                style.font = font_italic(style.font.clone());
            }
            "line-height" => {
                if let Some(line_height) = value.first().and_then(|value| value.parse().ok()) {
                    if line_height > 0.0 {
                        style.font.line_height = line_height;
                    }
                }
            }
            "text-decoration" | "text-decoration-line" => {
                style.decoration = value
                    .iter()
                    .any(|value| value == "underline")
                    .then(TextDecoration::underline);
            }
            "display" => style.display = value.first().cloned(),
            "width" => apply_width_value(style, &value, context),
            "height" => {
                style.height = parse_css_length_in_context(
                    &value,
                    style.font.size,
                    context.theme.body_font.size,
                    context.viewport_height,
                )
            }
            "min-width" => {
                style.min_width = parse_box_length(&value, style, context, context.viewport_width)
            }
            "max-width" => {
                style.max_width = parse_box_length(&value, style, context, context.viewport_width)
            }
            "min-height" => {
                style.min_height = parse_box_length(&value, style, context, context.viewport_height)
            }
            "max-height" => {
                style.max_height = parse_box_length(&value, style, context, context.viewport_height)
            }
            "margin-top" => apply_edge_value(style, EdgeSide::Top, &value, context, true),
            "margin-right" => apply_edge_value(style, EdgeSide::Right, &value, context, true),
            "margin-bottom" => apply_edge_value(style, EdgeSide::Bottom, &value, context, true),
            "margin-left" => apply_edge_value(style, EdgeSide::Left, &value, context, true),
            "padding-top" => apply_edge_value(style, EdgeSide::Top, &value, context, false),
            "padding-right" => apply_edge_value(style, EdgeSide::Right, &value, context, false),
            "padding-bottom" => apply_edge_value(style, EdgeSide::Bottom, &value, context, false),
            "padding-left" => apply_edge_value(style, EdgeSide::Left, &value, context, false),
            "box-sizing" if value.first().is_some_and(|value| value == "border-box") => {
                style.box_sizing = "border-box".into();
            }
            "border-width" => {
                if let Some(width) =
                    parse_box_length(&value, style, context, context.viewport_width)
                {
                    style.border_width = edges_all(width);
                }
            }
            "border-top-width" => apply_border_width(style, EdgeSide::Top, &value, context),
            "border-right-width" => apply_border_width(style, EdgeSide::Right, &value, context),
            "border-bottom-width" => apply_border_width(style, EdgeSide::Bottom, &value, context),
            "border-left-width" => apply_border_width(style, EdgeSide::Left, &value, context),
            "border-color" => {
                if let Some(color) = parse_color(&value) {
                    style.border_color = [Some(color); 4];
                }
            }
            "border-top-color" => apply_border_color(style, EdgeSide::Top, &value),
            "border-right-color" => apply_border_color(style, EdgeSide::Right, &value),
            "border-bottom-color" => apply_border_color(style, EdgeSide::Bottom, &value),
            "border-left-color" => apply_border_color(style, EdgeSide::Left, &value),
            "text-align" => {
                style.text_align = match value.first().map(String::as_str) {
                    Some("center") => TextAlign::Center,
                    Some("right" | "end") => TextAlign::End,
                    _ => TextAlign::Start,
                }
            }
            "white-space" => {
                if let Some(value) = value
                    .first()
                    .filter(|value| matches!(value.as_str(), "normal" | "nowrap" | "pre"))
                {
                    style.white_space = value.clone();
                }
            }
            _ => {}
        }
    }
}

impl Selector {
    fn matches<F>(
        &self,
        node: &BrowserRenderNode,
        ancestors: &[&BrowserRenderNode],
        position: Option<NodePosition>,
        is_visited: &F,
    ) -> bool
    where
        F: Fn(&str) -> bool + ?Sized,
    {
        let Some(last) = self.compounds.last() else {
            return false;
        };
        if !last.matches(node, position, is_visited) {
            return false;
        }
        let mut ancestor_limit = ancestors.len();
        for index in (0..self.compounds.len().saturating_sub(1)).rev() {
            let relation = &self.relations[index];
            let compound = &self.compounds[index];
            match relation {
                SelectorRelation::Child => {
                    if ancestor_limit == 0 {
                        return false;
                    }
                    ancestor_limit -= 1;
                    if !compound.matches(ancestors[ancestor_limit], None, is_visited) {
                        return false;
                    }
                }
                SelectorRelation::Descendant => {
                    let Some(found) = (0..ancestor_limit)
                        .rev()
                        .find(|position| compound.matches(ancestors[*position], None, is_visited))
                    else {
                        return false;
                    };
                    ancestor_limit = found;
                }
                SelectorRelation::Unsupported => return false,
            }
        }
        true
    }

    fn matches_virtual_body(&self) -> bool {
        self.compounds.len() == 1
            && self.compounds[0].tag.as_deref() == Some("body")
            && self.compounds[0].id.is_none()
            && self.compounds[0].classes.is_empty()
            && self.compounds[0].attributes.is_empty()
            && self.compounds[0].link_state.is_none()
            && self.compounds[0].structural.is_empty()
    }
}

impl CompoundSelector {
    fn matches<F>(
        &self,
        node: &BrowserRenderNode,
        position: Option<NodePosition>,
        is_visited: &F,
    ) -> bool
    where
        F: Fn(&str) -> bool + ?Sized,
    {
        if self
            .tag
            .as_deref()
            .is_some_and(|tag| tag != "*" && node.name.as_deref() != Some(tag))
        {
            return false;
        }
        if self
            .id
            .as_deref()
            .is_some_and(|id| node.id.as_deref() != Some(id))
        {
            return false;
        }
        if self
            .classes
            .iter()
            .any(|class| !node.classes.iter().any(|candidate| candidate == class))
        {
            return false;
        }
        if self
            .attributes
            .iter()
            .any(|selector| !selector.matches(node))
        {
            return false;
        }
        if self
            .structural
            .iter()
            .any(|selector| !selector.matches(position))
        {
            return false;
        }
        match self.link_state {
            Some(LinkState::Link) => node.role == "link",
            Some(LinkState::Visited) => {
                node.role == "link"
                    && node
                        .resolved_href
                        .as_deref()
                        .or(node.href.as_deref())
                        .is_some_and(is_visited)
            }
            None => true,
        }
    }
}

impl AttributeSelector {
    fn matches(&self, node: &BrowserRenderNode) -> bool {
        let Some(actual) = node_attribute(node, &self.name) else {
            return false;
        };
        let Some(expected) = self.value.as_deref() else {
            return true;
        };
        let (actual, expected) = if self.case_insensitive {
            (actual.to_ascii_lowercase(), expected.to_ascii_lowercase())
        } else {
            (actual, expected.to_string())
        };
        match self.operator.as_deref() {
            Some("=") => actual == expected,
            Some("~=") => actual.split_whitespace().any(|part| part == expected),
            Some("|=") => actual == expected || actual.starts_with(&format!("{expected}-")),
            Some("^=") => actual.starts_with(&expected),
            Some("$=") => actual.ends_with(&expected),
            Some("*=") => actual.contains(&expected),
            None => true,
            _ => false,
        }
    }
}

impl StructuralPseudo {
    fn matches(self, position: Option<NodePosition>) -> bool {
        let Some(position) = position else {
            return false;
        };
        match self {
            Self::First => position.index == 1,
            Self::Last => position.index == position.count,
            Self::Nth(index) => position.index == index,
        }
    }
}

fn collect_style_rules(node: &GrammarASTNode, media: &[Vec<String>], rules: &mut Vec<StyleRule>) {
    match node.rule_name.as_str() {
        "qualified_rule" => {
            if let Some(mut rule) = parse_style_rule(node) {
                rule.media = media.to_vec();
                rules.push(rule);
            }
            return;
        }
        "at_rule" => {
            let keyword = node.children.iter().find_map(|child| match child {
                ASTNodeOrToken::Token(token) => Some(token.value.as_str()),
                ASTNodeOrToken::Node(_) => None,
            });
            let prelude = child_nodes(node)
                .into_iter()
                .find(|child| child.rule_name == "at_prelude")
                .map(descendant_tokens)
                .unwrap_or_default();
            if keyword == Some("@media") {
                let mut nested_media = media.to_vec();
                nested_media.push(prelude);
                for child in child_nodes(node) {
                    if child.rule_name == "block" {
                        collect_style_rules(child, &nested_media, rules);
                    }
                }
            }
            return;
        }
        _ => {}
    }
    for child in child_nodes(node) {
        collect_style_rules(child, media, rules);
    }
}

fn collect_stylesheet_imports(node: &GrammarASTNode, imports: &mut Vec<HtmlStylesheetImport>) {
    if node.rule_name == "qualified_rule" {
        return;
    }
    if node.rule_name == "at_rule" {
        let keyword = node.children.iter().find_map(|child| match child {
            ASTNodeOrToken::Token(token) => Some(token.value.as_str()),
            ASTNodeOrToken::Node(_) => None,
        });
        if keyword == Some("@import") {
            if let Some(prelude) = child_nodes(node)
                .into_iter()
                .find(|child| child.rule_name == "at_prelude")
                .map(descendant_tokens)
                .and_then(parse_stylesheet_import)
            {
                imports.push(prelude);
            }
        }
        return;
    }
    for child in child_nodes(node) {
        collect_stylesheet_imports(child, imports);
    }
}

fn parse_stylesheet_import(tokens: Vec<String>) -> Option<HtmlStylesheetImport> {
    let href_index = tokens
        .iter()
        .position(|token| {
            token.starts_with(['\'', '"']) || token.starts_with("url(") || token == "url"
        })
        .or((!tokens.is_empty()).then_some(0))?;
    let token = &tokens[href_index];
    let (href, consumed) = if token == "url" || token == "url(" {
        let value = tokens.get(href_index + 1)?;
        (trim_css_string(value), 2)
    } else if let Some(value) = token.strip_prefix("url(") {
        (trim_css_string(value.trim_end_matches(')')), 1)
    } else {
        (trim_css_string(token), 1)
    };
    if href.is_empty() {
        return None;
    }
    let media = tokens[href_index + consumed..]
        .iter()
        .filter(|token| token.as_str() != ")")
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    Some(HtmlStylesheetImport {
        href,
        media: (!media.trim().is_empty()).then(|| media.trim().to_string()),
    })
}

fn parse_style_rule(node: &GrammarASTNode) -> Option<StyleRule> {
    let selector_list = child_nodes(node)
        .into_iter()
        .find(|child| child.rule_name == "selector_list")?;
    let block = child_nodes(node)
        .into_iter()
        .find(|child| child.rule_name == "block")?;
    let selectors = child_nodes(selector_list)
        .into_iter()
        .filter(|child| child.rule_name == "complex_selector")
        .filter_map(parse_selector)
        .collect::<Vec<_>>();
    let mut declarations = Vec::new();
    collect_direct_declarations(block, &mut declarations);
    (!selectors.is_empty() && !declarations.is_empty()).then_some(StyleRule {
        selectors,
        declarations,
        media: Vec::new(),
    })
}

fn parse_inline_declarations(source: &str) -> Result<Vec<Declaration>, HtmlStyleError> {
    let stylesheet = HtmlAuthorStylesheet::parse(&format!("* {{{source};}}"))?;
    Ok(stylesheet
        .rules
        .into_iter()
        .next()
        .map(|rule| rule.declarations)
        .unwrap_or_default())
}

fn collect_direct_declarations(node: &GrammarASTNode, declarations: &mut Vec<Declaration>) {
    if node.rule_name == "qualified_rule" || node.rule_name == "at_rule" {
        return;
    }
    if node.rule_name == "declaration" {
        let property = descendant_named_node(node, "property")
            .and_then(|property| descendant_tokens(property).first().cloned());
        let value = descendant_named_node(node, "value_list").map(descendant_tokens);
        if let (Some(property), Some(value)) = (property, value) {
            declarations.push(Declaration {
                property: property.to_ascii_lowercase(),
                value,
                important: descendant_named_node(node, "priority").is_some(),
            });
        }
        return;
    }
    for child in child_nodes(node) {
        collect_direct_declarations(child, declarations);
    }
}

fn parse_selector(node: &GrammarASTNode) -> Option<Selector> {
    let mut compounds = Vec::new();
    let mut relations = Vec::new();
    for child in child_nodes(node) {
        match child.rule_name.as_str() {
            "compound_selector" => {
                if !compounds.is_empty() && relations.len() < compounds.len() {
                    relations.push(SelectorRelation::Descendant);
                }
                compounds.push(parse_compound_selector(child));
            }
            "combinator" => {
                let relation = match descendant_tokens(child).first().map(String::as_str) {
                    Some(">") => SelectorRelation::Child,
                    Some("+" | "~") => SelectorRelation::Unsupported,
                    _ => SelectorRelation::Descendant,
                };
                if !compounds.is_empty() {
                    if relations.len() == compounds.len() {
                        relations.pop();
                    }
                    relations.push(relation);
                }
            }
            _ => {}
        }
    }
    if compounds.is_empty() {
        return None;
    }
    while relations.len() + 1 < compounds.len() {
        relations.push(SelectorRelation::Descendant);
    }
    let specificity = compounds.iter().fold((0, 0, 0), |mut total, compound| {
        total.0 += u16::from(compound.id.is_some());
        total.1 += compound.classes.len() as u16
            + compound.attributes.len() as u16
            + compound.structural.len() as u16
            + u16::from(compound.link_state.is_some());
        total.2 += u16::from(compound.tag.as_deref().is_some_and(|tag| tag != "*"));
        total
    });
    Some(Selector {
        compounds,
        relations,
        specificity,
    })
}

fn parse_compound_selector(node: &GrammarASTNode) -> CompoundSelector {
    let mut compound = CompoundSelector::default();
    collect_compound_parts(node, &mut compound);
    compound
}

fn collect_compound_parts(node: &GrammarASTNode, compound: &mut CompoundSelector) {
    for child in child_nodes(node) {
        match child.rule_name.as_str() {
            "simple_selector" => {
                compound.tag = descendant_tokens(child)
                    .first()
                    .map(|tag| tag.to_ascii_lowercase())
            }
            "class_selector" => {
                if let Some(class) = descendant_tokens(child).get(1) {
                    compound.classes.push(class.clone());
                }
            }
            "id_selector" => {
                compound.id = descendant_tokens(child)
                    .first()
                    .map(|id| id.trim_start_matches('#').to_string())
            }
            "attribute_selector" => {
                if let Some(selector) = parse_attribute_selector(&descendant_tokens(child)) {
                    compound.attributes.push(selector);
                }
            }
            "pseudo_class" => {
                let tokens = descendant_tokens(child);
                if tokens.iter().any(|token| token == "visited") {
                    compound.link_state = Some(LinkState::Visited);
                } else if tokens.iter().any(|token| token == "link") {
                    compound.link_state = Some(LinkState::Link);
                } else if tokens.iter().any(|token| token == "first-child") {
                    compound.structural.push(StructuralPseudo::First);
                } else if tokens.iter().any(|token| token == "last-child") {
                    compound.structural.push(StructuralPseudo::Last);
                } else if tokens.iter().any(|token| token.starts_with("nth-child")) {
                    if let Some(index) = tokens.iter().find_map(|token| token.parse().ok()) {
                        compound.structural.push(StructuralPseudo::Nth(index));
                    }
                }
            }
            _ => collect_compound_parts(child, compound),
        }
    }
}

fn parse_attribute_selector(tokens: &[String]) -> Option<AttributeSelector> {
    let tokens = tokens
        .iter()
        .filter(|token| !matches!(token.as_str(), "[" | "]"))
        .collect::<Vec<_>>();
    let name = tokens.first()?.to_ascii_lowercase();
    let operator_index = tokens
        .iter()
        .position(|token| matches!(token.as_str(), "=" | "~=" | "|=" | "^=" | "$=" | "*="));
    let operator = operator_index.map(|index| tokens[index].to_string());
    let value = operator_index
        .and_then(|index| tokens.get(index + 1))
        .map(|value| trim_css_string(value));
    let case_insensitive = operator_index
        .and_then(|index| tokens.get(index + 2))
        .is_some_and(|flag| flag.eq_ignore_ascii_case("i"));
    Some(AttributeSelector {
        name,
        operator,
        value,
        case_insensitive,
    })
}

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|child| match child {
            ASTNodeOrToken::Node(node) => Some(node),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn descendant_named_node<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a GrammarASTNode> {
    if node.rule_name == name {
        return Some(node);
    }
    child_nodes(node)
        .into_iter()
        .find_map(|child| descendant_named_node(child, name))
}

fn descendant_tokens(node: &GrammarASTNode) -> Vec<String> {
    let mut tokens = Vec::new();
    collect_tokens(node, &mut tokens);
    tokens
}

fn collect_tokens(node: &GrammarASTNode, tokens: &mut Vec<String>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(node) => collect_tokens(node, tokens),
            ASTNodeOrToken::Token(token) => tokens.push(token.value.clone()),
        }
    }
}

fn node_attribute(node: &BrowserRenderNode, name: &str) -> Option<String> {
    match name {
        "id" => node.id.clone(),
        "class" => (!node.classes.is_empty()).then(|| node.classes.join(" ")),
        "style" => node.style.clone(),
        "title" => node.title.clone(),
        "lang" => node.lang.clone(),
        "dir" => node.dir.clone(),
        "href" => node.href.clone(),
        "target" => node.target.clone(),
        "rel" => node.rel.clone(),
        "src" => node.src.clone(),
        "alt" => node.alt.clone(),
        "width" => node.width.clone(),
        "height" => node.height.clone(),
        "type" => node.type_hint.clone(),
        "media" => node.media.clone(),
        "value" => node.value.clone(),
        "placeholder" => node.placeholder.clone(),
        "disabled" if node.disabled => Some(String::new()),
        "required" if node.required => Some(String::new()),
        "readonly" if node.readonly => Some(String::new()),
        "checked" if node.checked => Some(String::new()),
        "selected" if node.selected => Some(String::new()),
        "multiple" if node.multiple => Some(String::new()),
        "hidden" if node.hidden => Some(String::new()),
        "open" if node.open => Some(String::new()),
        _ => None,
    }
}

fn rule_media_applies(rule: &StyleRule, context: &HtmlStyleContext) -> bool {
    rule.media.iter().all(|query| {
        media_query_list_applies(query, context.viewport_width, context.viewport_height)
    })
}

fn media_query_list_applies(tokens: &[String], width: f64, height: f64) -> bool {
    let query = tokens.join(" ").to_ascii_lowercase();
    query.split(',').any(|candidate| {
        let candidate = candidate.trim();
        if candidate.starts_with("not ") || candidate.contains("print") {
            return false;
        }
        let type_applies =
            candidate.contains("screen") || candidate.contains("all") || candidate.starts_with('(');
        type_applies
            && media_feature_applies(candidate, "min-width", width, |actual, limit| {
                actual >= limit
            })
            && media_feature_applies(candidate, "max-width", width, |actual, limit| {
                actual <= limit
            })
            && media_feature_applies(candidate, "min-height", height, |actual, limit| {
                actual >= limit
            })
            && media_feature_applies(candidate, "max-height", height, |actual, limit| {
                actual <= limit
            })
    })
}

fn media_feature_applies(
    query: &str,
    feature: &str,
    actual: f64,
    compare: impl Fn(f64, f64) -> bool,
) -> bool {
    let Some(index) = query.find(feature) else {
        return true;
    };
    let remainder = &query[index + feature.len()..];
    let value = remainder
        .trim_start_matches([' ', ':'])
        .split(|character: char| character == ')' || character.is_whitespace())
        .find(|part| !part.is_empty())
        .and_then(|part| parse_css_length(&[part.to_string()]));
    value.is_some_and(|limit| compare(actual, limit))
}

fn expand_declaration(declaration: &Declaration) -> Vec<(String, Vec<String>)> {
    if declaration.property == "border" {
        let mut expanded = Vec::new();
        if let Some(width) = declaration
            .value
            .iter()
            .find(|token| parse_css_length(&[(*token).clone()]).is_some())
        {
            expanded.push(("border-width".into(), vec![width.clone()]));
        }
        if let Some(color) = declaration
            .value
            .iter()
            .find(|token| parse_color(&[(*token).clone()]).is_some())
        {
            expanded.push(("border-color".into(), vec![color.clone()]));
        }
        return expanded;
    }
    if !matches!(declaration.property.as_str(), "margin" | "padding") {
        return vec![(declaration.property.clone(), declaration.value.clone())];
    }
    let Some(edges) = expand_edge_values(&declaration.value) else {
        return Vec::new();
    };
    ["top", "right", "bottom", "left"]
        .into_iter()
        .zip(edges)
        .map(|(side, value)| (format!("{}-{side}", declaration.property), vec![value]))
        .collect()
}

fn expand_edge_values(value: &[String]) -> Option<[String; 4]> {
    let values = value
        .iter()
        .filter(|token| token.as_str() != ",")
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    match values.as_slice() {
        [all] => Some([all.clone(), all.clone(), all.clone(), all.clone()]),
        [vertical, horizontal] => Some([
            vertical.clone(),
            horizontal.clone(),
            vertical.clone(),
            horizontal.clone(),
        ]),
        [top, horizontal, bottom] => Some([
            top.clone(),
            horizontal.clone(),
            bottom.clone(),
            horizontal.clone(),
        ]),
        [top, right, bottom, left] => {
            Some([top.clone(), right.clone(), bottom.clone(), left.clone()])
        }
        _ => None,
    }
}

fn resolve_css_value(
    value: &[String],
    custom_properties: &HashMap<String, Vec<String>>,
    depth: usize,
) -> Option<Vec<String>> {
    if depth > 16 {
        return None;
    }
    let Some(var_index) = value
        .iter()
        .position(|token| token == "var" || token.starts_with("var("))
    else {
        return Some(value.to_vec());
    };
    let name_index = value[var_index..]
        .iter()
        .position(|token| token.starts_with("--"))?
        + var_index;
    let name = &value[name_index];
    let end_index = value[name_index..]
        .iter()
        .position(|token| token == ")")
        .map(|index| name_index + index)
        .unwrap_or(name_index);
    let replacement = custom_properties.get(name).cloned().or_else(|| {
        value[name_index + 1..=end_index]
            .iter()
            .position(|token| token == ",")
            .map(|comma| value[name_index + 2 + comma..end_index].to_vec())
    })?;
    let mut resolved = value[..var_index].to_vec();
    resolved.extend(replacement);
    resolved.extend_from_slice(&value[end_index.saturating_add(1)..]);
    resolve_css_value(&resolved, custom_properties, depth + 1)
}

#[derive(Clone, Copy)]
enum EdgeSide {
    Top,
    Right,
    Bottom,
    Left,
}

impl EdgeSide {
    fn index(self) -> usize {
        match self {
            Self::Top => 0,
            Self::Right => 1,
            Self::Bottom => 2,
            Self::Left => 3,
        }
    }
}

fn set_edge(edges: &mut Option<Edges>, side: EdgeSide, value: Option<f64>) {
    let Some(value) = value else {
        return;
    };
    let edges = edges.get_or_insert_with(Edges::default);
    match side {
        EdgeSide::Top => edges.top = value,
        EdgeSide::Right => edges.right = value,
        EdgeSide::Bottom => edges.bottom = value,
        EdgeSide::Left => edges.left = value,
    }
}

fn trim_css_string(value: &str) -> String {
    value
        .trim()
        .trim_matches(['\'', '"'])
        .trim_end_matches(')')
        .to_string()
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

fn parse_css_length(value: &[String]) -> Option<f64> {
    let value = value.first()?.trim();
    if value == "0" {
        return Some(0.0);
    }
    value
        .strip_suffix("px")?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn parse_css_length_in_context(
    value: &[String],
    em_size: f64,
    rem_size: f64,
    percentage_basis: f64,
) -> Option<f64> {
    let value = value.first()?.trim().to_ascii_lowercase();
    if value == "0" {
        return Some(0.0);
    }
    let (number, scale) = if let Some(number) = value.strip_suffix("rem") {
        (number, rem_size)
    } else if let Some(number) = value.strip_suffix("em") {
        (number, em_size)
    } else if let Some(number) = value.strip_suffix("px") {
        (number, 1.0)
    } else if let Some(number) = value.strip_suffix('%') {
        (number, percentage_basis / 100.0)
    } else {
        return None;
    };
    number
        .parse::<f64>()
        .ok()
        .map(|number| number * scale)
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn parse_percentage(value: &[String]) -> Option<f64> {
    value
        .first()?
        .trim()
        .strip_suffix('%')?
        .parse::<f64>()
        .ok()
        .map(|value| value / 100.0)
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn parse_box_length(
    value: &[String],
    style: &HtmlComputedStyle,
    context: &HtmlStyleContext,
    percentage_basis: f64,
) -> Option<f64> {
    parse_css_length_in_context(
        value,
        style.font.size,
        context.theme.body_font.size,
        percentage_basis,
    )
}

fn apply_width_value(style: &mut HtmlComputedStyle, value: &[String], context: &HtmlStyleContext) {
    style.width_auto = value.first().is_some_and(|value| value == "auto");
    style.width_percent = parse_percentage(value);
    style.width = if style.width_percent.is_some() || style.width_auto {
        None
    } else {
        parse_box_length(value, style, context, context.viewport_width)
    };
}

fn apply_edge_value(
    style: &mut HtmlComputedStyle,
    side: EdgeSide,
    value: &[String],
    context: &HtmlStyleContext,
    margin: bool,
) {
    if margin && value.first().is_some_and(|value| value == "auto") {
        style.margin_auto[side.index()] = true;
        set_edge(&mut style.margin, side, Some(0.0));
        return;
    }
    let length = parse_box_length(value, style, context, context.viewport_width);
    if margin {
        set_edge(&mut style.margin, side, length);
    } else {
        set_edge(&mut style.padding, side, length);
    }
}

fn apply_border_width(
    style: &mut HtmlComputedStyle,
    side: EdgeSide,
    value: &[String],
    context: &HtmlStyleContext,
) {
    if let Some(width) = parse_box_length(value, style, context, context.viewport_width) {
        match side {
            EdgeSide::Top => style.border_width.top = width,
            EdgeSide::Right => style.border_width.right = width,
            EdgeSide::Bottom => style.border_width.bottom = width,
            EdgeSide::Left => style.border_width.left = width,
        }
    }
}

fn apply_border_color(style: &mut HtmlComputedStyle, side: EdgeSide, value: &[String]) {
    style.border_color[side.index()] = parse_color(value);
}

fn parse_color(value: &[String]) -> Option<Color> {
    let value = value.first()?.to_ascii_lowercase();
    match value.as_str() {
        "black" => Some(rgb(0, 0, 0)),
        "white" => Some(rgb(255, 255, 255)),
        "red" => Some(rgb(255, 0, 0)),
        "green" => Some(rgb(0, 128, 0)),
        "blue" => Some(rgb(0, 0, 255)),
        "yellow" => Some(rgb(255, 255, 0)),
        "gray" | "grey" => Some(rgb(128, 128, 128)),
        "silver" => Some(rgb(192, 192, 192)),
        "maroon" => Some(rgb(128, 0, 0)),
        "navy" => Some(rgb(0, 0, 128)),
        "purple" => Some(rgb(128, 0, 128)),
        "teal" => Some(rgb(0, 128, 128)),
        _ if value.starts_with('#') => parse_hex_color(&value),
        _ => None,
    }
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    match hex.len() {
        3 => Some(rgb(
            u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?,
        )),
        6 => Some(rgb(
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        )),
        _ => None,
    }
}

fn never_visited(_url: &str) -> bool {
    false
}

fn apply_size_hints(
    layout: &mut LayoutNode,
    node: &BrowserRenderNode,
    style: &HtmlComputedStyle,
    display: &str,
) {
    let horizontal_insets = style.padding.unwrap_or_default().left
        + style.padding.unwrap_or_default().right
        + style.border_width.left
        + style.border_width.right;
    if let Some(fraction) = style.width_percent {
        layout.width = Some(SizeValue::Percent(fraction));
    } else if let Some(width) = style
        .width
        .or_else(|| parse_dimension(node.width.as_deref()))
    {
        let width = if style.box_sizing == "content-box" {
            width + horizontal_insets
        } else {
            width
        };
        layout.width = Some(SizeValue::Fixed(width));
    } else if display == "block" || display.starts_with("table") {
        layout.width = Some(SizeValue::Fill);
    } else if layout.width.is_none() {
        layout.width = Some(SizeValue::Wrap);
    }

    if let Some(height) = style
        .height
        .or_else(|| parse_dimension(node.height.as_deref()))
    {
        layout.height = Some(SizeValue::Fixed(height));
    } else if layout.height.is_none() {
        layout.height = Some(SizeValue::Wrap);
    }
    layout.min_width = style.min_width;
    layout.max_width = style.max_width;
    layout.min_height = style.min_height;
    layout.max_height = style.max_height;
}

fn apply_spacing(
    layout: &mut LayoutNode,
    node: &BrowserRenderNode,
    theme: &HtmlTheme,
    style: &HtmlComputedStyle,
) {
    if let Some(margin) = style.margin {
        layout.margin = Some(margin);
        return;
    }
    let spacing = if node.role == "heading" {
        theme.heading_spacing
    } else if matches!(
        node.role.as_str(),
        "paragraph" | "preformatted" | "list" | "quote_block" | "description_list" | "figure"
    ) {
        theme.block_spacing
    } else {
        0.0
    };
    if spacing > 0.0 {
        layout.margin = Some(edges_xy(0.0, spacing / 2.0));
    }
}

fn parse_dimension(value: Option<&str>) -> Option<f64> {
    value?
        .trim()
        .trim_end_matches("px")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn html_ext(node: &BrowserRenderNode) -> ExtValue {
    let mut values = HashMap::new();
    values.insert("role".into(), ExtValue::Str(node.role.clone()));
    values.insert("display".into(), ExtValue::Str(node.display.clone()));
    insert_optional(&mut values, "tag", node.name.as_deref());
    insert_optional(
        &mut values,
        "href",
        node.resolved_href.as_deref().or(node.href.as_deref()),
    );
    insert_optional(&mut values, "target", node.target.as_deref());
    insert_optional(&mut values, "lang", node.lang.as_deref());
    insert_optional(&mut values, "dir", node.dir.as_deref());
    insert_optional(&mut values, "alt", node.alt.as_deref());
    ExtValue::Map(values)
}

fn root_html_ext() -> ExtValue {
    ExtValue::Map(HashMap::from([(
        "role".into(),
        ExtValue::Str("document".into()),
    )]))
}

fn display_ext(display: &str) -> ExtValue {
    ExtValue::Map(HashMap::from([(
        "display".into(),
        ExtValue::Str(display.to_string()),
    )]))
}

fn block_ext(node: &BrowserRenderNode, display: &str, style: &HtmlComputedStyle) -> ExtValue {
    let mut values = HashMap::from([("display".into(), ExtValue::Str(display.to_string()))]);
    if node.role == "preformatted" || style.white_space != "normal" {
        let white_space = if node.role == "preformatted" {
            "pre"
        } else {
            &style.white_space
        };
        values.insert("whiteSpace".into(), ExtValue::Str(white_space.to_string()));
    }
    values.insert(
        "marginLeftAuto".into(),
        ExtValue::Bool(style.margin_auto[EdgeSide::Left.index()]),
    );
    values.insert(
        "marginRightAuto".into(),
        ExtValue::Bool(style.margin_auto[EdgeSide::Right.index()]),
    );
    ExtValue::Map(values)
}

fn background_ext(color: Color) -> ExtValue {
    ExtValue::Map(HashMap::from([(
        "backgroundColor".into(),
        color_ext(color),
    )]))
}

fn box_paint_ext(style: &HtmlComputedStyle) -> ExtValue {
    let mut values = HashMap::new();
    if let Some(background) = style.background {
        values.insert("backgroundColor".into(), color_ext(background));
    }
    for (name, side, width) in [
        ("Top", EdgeSide::Top, style.border_width.top),
        ("Right", EdgeSide::Right, style.border_width.right),
        ("Bottom", EdgeSide::Bottom, style.border_width.bottom),
        ("Left", EdgeSide::Left, style.border_width.left),
    ] {
        if width > 0.0 {
            values.insert(format!("border{name}Width"), ExtValue::Float(width));
            values.insert(
                format!("border{name}Color"),
                color_ext(style.border_color[side.index()].unwrap_or(style.color)),
            );
        }
    }
    ExtValue::Map(values)
}

fn color_ext(color: Color) -> ExtValue {
    ExtValue::Map(HashMap::from([
        ("r".into(), ExtValue::Int(i64::from(color.r))),
        ("g".into(), ExtValue::Int(i64::from(color.g))),
        ("b".into(), ExtValue::Int(i64::from(color.b))),
        ("a".into(), ExtValue::Int(i64::from(color.a))),
    ]))
}

fn insert_optional(values: &mut HashMap<String, ExtValue>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        values.insert(key.into(), ExtValue::Str(value.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_browser_render_tree;
    use layout_block::layout_block;
    use layout_ir::{constraints_width, Content, MeasureResult, PositionedNode, TextMeasurer};

    struct TestMeasurer;

    impl TextMeasurer for TestMeasurer {
        fn measure(&self, text: &str, font: &FontSpec, max_width: Option<f64>) -> MeasureResult {
            let width = text.chars().count() as f64 * font.size * 0.5;
            MeasureResult {
                width: max_width.map_or(width, |limit| width.min(limit)),
                height: font.size * font.line_height,
                baseline: font.size * 0.8,
                line_count: 1,
            }
        }
    }

    #[test]
    fn mosaic_theme_uses_era_defaults() {
        let theme = mosaic_html_theme();
        assert_eq!(theme.body_font.family, "Times New Roman");
        assert_eq!(theme.link_color, rgb(0, 0, 238));
        assert_eq!(theme.visited_link_color, rgb(85, 26, 139));
        assert_eq!(theme.link_decoration, Some(TextDecoration::underline()));
        assert_eq!(theme.page_background, rgb(192, 192, 192));
    }

    #[test]
    fn link_state_colors_and_underlines_nested_text_without_leaking_policy() {
        let render = parse_browser_render_tree(
            "<base href='https://example.test/docs/'>\
             <p><a href='seen'><strong>Seen</strong> page</a> and \
             <a href='new'>new</a>.</p>",
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_link_state(&render, &mosaic_html_theme(), &|url| {
                url == "https://example.test/docs/seen"
            });

        let seen = find_by_href(&layout, "https://example.test/docs/seen").unwrap();
        let unseen = find_by_href(&layout, "https://example.test/docs/new").unwrap();
        let seen_text = first_text(seen).unwrap();
        assert_eq!(seen_text.color, rgb(85, 26, 139));
        assert_eq!(seen_text.decoration, Some(TextDecoration::underline()));
        assert_eq!(seen_text.font.weight, 700);
        assert_eq!(first_text(unseen).unwrap().color, rgb(0, 0, 238));
        assert_eq!(
            first_text(unseen).unwrap().decoration,
            Some(TextDecoration::underline())
        );

        let default_layout = html_render_tree_to_layout(&render, &mosaic_html_theme());
        assert_eq!(
            first_text(find_by_href(&default_layout, "https://example.test/docs/seen").unwrap())
                .unwrap()
                .color,
            rgb(0, 0, 238)
        );
    }

    #[test]
    fn author_cascade_resolves_specificity_inheritance_and_display() {
        let render = parse_browser_render_tree(
            "<main><p id='hero' class='lead'>Styled</p><p class='hidden'>Gone</p></main>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["p { color: red; } .lead { color: blue; } \
                 #hero { color: #00ff00 !important; font-size: 20px; } \
                 .hidden { display: none; } body { background-color: white; padding: 8px; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);

        let hero = find_by_id(&layout, "hero").unwrap();
        let text = first_text(hero).unwrap();
        assert_eq!(text.color, rgb(0, 255, 0));
        assert_eq!(text.font.size, 20.0);
        assert!(!all_text(&layout).contains(&"Gone"));
        assert_eq!(layout.padding, Some(edges_all(8.0)));
        assert_eq!(
            layout.ext.get("paint"),
            Some(&background_ext(rgb(255, 255, 255)))
        );
    }

    #[test]
    fn visited_pseudo_class_stays_session_owned() {
        let render =
            parse_browser_render_tree("<a href='https://example.test/seen'>Seen</a>").unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["a:link { color: blue; } a:visited { color: maroon; }"],
        )
        .unwrap();
        let layout = html_render_tree_to_layout_with_style_context(&render, &context, &|url| {
            url == "https://example.test/seen"
        });
        assert_eq!(first_text(&layout).unwrap().color, rgb(128, 0, 0));
    }

    #[test]
    fn stylesheet_media_rules_apply_only_to_the_screen_profile() {
        let render = parse_browser_render_tree("<p>Screen</p>").unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["@media print { p { color: red; } } @media screen { p { color: green; } }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        assert_eq!(first_text(&layout).unwrap().color, rgb(0, 128, 0));
        assert!(HtmlAuthorStylesheet::parse("p { color: ; }").is_err());
    }

    #[test]
    fn element_styles_custom_properties_and_edge_shorthands_share_the_cascade() {
        let render = parse_browser_render_tree(
            "<main style='--accent: #123456'><p id='hero' \
             style='color: var(--accent); margin: 1px 2px 3px 4px; padding: 5px 6px'>Inline</p></main>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["#hero { color: red; margin-left: 99px; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let hero = find_by_id(&layout, "hero").unwrap();
        assert_eq!(first_text(hero).unwrap().color, rgb(18, 52, 86));
        assert_eq!(
            hero.margin,
            Some(Edges {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            })
        );
        assert_eq!(
            hero.padding,
            Some(Edges {
                top: 5.0,
                right: 6.0,
                bottom: 5.0,
                left: 6.0,
            })
        );
    }

    #[test]
    fn computed_box_values_project_relative_sizes_flow_and_border_paint() {
        let render = parse_browser_render_tree(
            "<main><p id='box'>Centered text that must stay together</p></main>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["#box { width: 50%; min-width: 120px; max-width: 240px; \
              margin: 1em auto; padding: 1em 2rem; border: 2px solid blue; \
              box-sizing: border-box; font-size: 20px; text-align: center; \
              white-space: nowrap; }"],
        )
        .unwrap()
        .with_viewport(400.0, 300.0);
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let box_node = find_by_id(&layout, "box").unwrap();
        assert_eq!(box_node.width, Some(SizeValue::Percent(0.5)));
        assert_eq!(box_node.min_width, Some(120.0));
        assert_eq!(box_node.max_width, Some(240.0));
        assert_eq!(
            box_node.padding,
            Some(Edges {
                top: 20.0,
                right: 28.0,
                bottom: 20.0,
                left: 28.0,
            })
        );
        let text = first_text(box_node).unwrap();
        assert_eq!(text.text_align, TextAlign::Center);
        assert!(!text.wrap);
        assert!(matches!(
            box_node.ext.get("paint"),
            Some(ExtValue::Map(values))
                if values.get("borderTopWidth") == Some(&ExtValue::Float(2.0))
        ));

        let positioned = layout_block(&layout, constraints_width(400.0), &TestMeasurer);
        let positioned_box = find_positioned_by_id(&positioned, "box").unwrap();
        assert_eq!(positioned_box.width, 184.0);
        assert_eq!(positioned_box.x, 92.0);
    }

    #[test]
    fn attribute_and_structural_selectors_match_rendered_element_positions() {
        let render = parse_browser_render_tree(
            "<section><p id='first' title='lead story'>One</p><p id='second'>Two</p><p id='last'>Three</p></section>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["p[title~='story']:first-child { color: red; } \
              p:nth-child(2) { color: green; } p:last-child { color: blue; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        assert_eq!(
            first_text(find_by_id(&layout, "first").unwrap())
                .unwrap()
                .color,
            rgb(255, 0, 0)
        );
        assert_eq!(
            first_text(find_by_id(&layout, "second").unwrap())
                .unwrap()
                .color,
            rgb(0, 128, 0)
        );
        assert_eq!(
            first_text(find_by_id(&layout, "last").unwrap())
                .unwrap()
                .color,
            rgb(0, 0, 255)
        );
    }

    #[test]
    fn viewport_media_and_import_metadata_remain_parser_owned() {
        let stylesheet = HtmlAuthorStylesheet::parse(
            "@import url('base.css') screen and (min-width: 400px); \
             @media screen and (max-width: 500px) { p { color: red; } } \
             @media screen and (min-width: 501px) { p { color: blue; } }",
        )
        .unwrap();
        assert_eq!(stylesheet.imports().len(), 1);
        assert_eq!(stylesheet.imports()[0].href, "base.css");
        assert!(stylesheet.imports()[0]
            .media
            .as_deref()
            .is_some_and(|media| media.contains("min-width")));

        let render = parse_browser_render_tree("<p>Viewport</p>").unwrap();
        let mut narrow = HtmlStyleContext::new(mosaic_html_theme()).with_viewport(480.0, 320.0);
        narrow.author_stylesheets.push(stylesheet.clone());
        let mut wide = HtmlStyleContext::new(mosaic_html_theme()).with_viewport(800.0, 600.0);
        wide.author_stylesheets.push(stylesheet);
        assert_eq!(
            first_text(&html_render_tree_to_layout_with_style_context(
                &render,
                &narrow,
                &never_visited,
            ))
            .unwrap()
            .color,
            rgb(255, 0, 0)
        );
        assert_eq!(
            first_text(&html_render_tree_to_layout_with_style_context(
                &render,
                &wide,
                &never_visited,
            ))
            .unwrap()
            .color,
            rgb(0, 0, 255)
        );
    }

    #[test]
    fn maps_heading_link_and_resolved_image_metadata() {
        let render = parse_browser_render_tree(
            r#"<base href="https://example.test/docs/">
               <h2 id="intro">Hello</h2>
               <p>Read <a href="next.html">next</a>.</p>
               <img src="logo.gif" alt="Logo" width="64" height="32">"#,
        )
        .unwrap();
        let layout = html_render_tree_to_layout(&render, &mosaic_html_theme());

        let heading = find_by_id(&layout, "intro").unwrap();
        let heading_text = first_text(heading).unwrap();
        assert_eq!(heading_text.value, "Hello");
        assert_eq!(heading_text.font.size, 20.0);

        let link = find_by_html_role(&layout, "link").unwrap();
        assert_eq!(
            html_string(link, "href"),
            Some("https://example.test/docs/next.html")
        );
        assert_eq!(first_text(link).unwrap().color, rgb(0, 0, 238));

        let image = find_by_html_role(&layout, "image").unwrap();
        assert_eq!(image.width, Some(SizeValue::Fixed(64.0)));
        assert_eq!(image.height, Some(SizeValue::Fixed(32.0)));
        assert!(matches!(
            image.content.as_ref(),
            Some(Content::Image(content))
                if content.src == "https://example.test/docs/logo.gif"
        ));
    }

    #[test]
    fn omits_hidden_browser_content() {
        let render = parse_browser_render_tree(
            "<p>shown</p><p aria-hidden=\"true\">visual</p><p hidden>hidden</p>",
        )
        .unwrap();
        let layout = html_render_tree_to_layout(&render, &mosaic_html_theme());
        let text = all_text(&layout);
        assert_eq!(text, vec!["shown", "visual"]);
    }

    #[test]
    fn preformatted_content_projects_shared_white_space_policy() {
        let render = parse_browser_render_tree("<pre>one  two\nthree</pre>").unwrap();
        let layout = html_render_tree_to_layout(&render, &mosaic_html_theme());
        let pre = find_by_html_role(&layout, "preformatted").unwrap();
        assert!(matches!(
            pre.ext.get("block"),
            Some(ExtValue::Map(values))
                if values.get("whiteSpace") == Some(&ExtValue::Str("pre".into()))
        ));
    }

    #[test]
    fn parser_to_positioned_layout_acceptance_path_is_executable() {
        let render = parse_browser_render_tree(
            r#"<base href="https://example.test/">
               <main><h1>Mosaic lives</h1>
               <p>The browser pipeline is <a href="status">connected</a>.</p></main>"#,
        )
        .unwrap();
        let layout = html_render_tree_to_layout(&render, &mosaic_html_theme());
        let positioned = layout_block(&layout, constraints_width(640.0), &TestMeasurer);

        assert_eq!(positioned.width, 640.0);
        assert!(positioned.height > 0.0);
        assert_eq!(
            positioned_text(&positioned),
            vec!["Mosaic lives", "The browser pipeline is", "connected", "."]
        );
        let link = find_positioned_by_html_role(&positioned, "link").unwrap();
        let leading_text = find_positioned_text(&positioned, "The browser pipeline is").unwrap();
        let trailing_text = find_positioned_text(&positioned, ".").unwrap();
        assert_eq!(link.y, leading_text.y);
        assert_eq!(trailing_text.y, link.y);
        assert!(link.x > leading_text.x);
        assert!(trailing_text.x > link.x);
        assert_eq!(
            positioned_html_string(link, "href"),
            Some("https://example.test/status")
        );
    }

    fn find_by_id<'a>(node: &'a LayoutNode, id: &str) -> Option<&'a LayoutNode> {
        if node.id.as_deref() == Some(id) {
            return Some(node);
        }
        node.children.iter().find_map(|child| find_by_id(child, id))
    }

    fn find_by_html_role<'a>(node: &'a LayoutNode, role: &str) -> Option<&'a LayoutNode> {
        if html_string(node, "role") == Some(role) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_by_html_role(child, role))
    }

    fn find_by_href<'a>(node: &'a LayoutNode, href: &str) -> Option<&'a LayoutNode> {
        if html_string(node, "href") == Some(href) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_by_href(child, href))
    }

    fn html_string<'a>(node: &'a LayoutNode, key: &str) -> Option<&'a str> {
        let ExtValue::Map(values) = node.ext.get("html")? else {
            return None;
        };
        let ExtValue::Str(value) = values.get(key)? else {
            return None;
        };
        Some(value)
    }

    fn first_text(node: &LayoutNode) -> Option<&TextContent> {
        if let Some(Content::Text(text)) = &node.content {
            return Some(text);
        }
        node.children.iter().find_map(first_text)
    }

    fn all_text(node: &LayoutNode) -> Vec<&str> {
        let mut values = Vec::new();
        if let Some(Content::Text(text)) = &node.content {
            values.push(text.value.as_str());
        }
        for child in &node.children {
            values.extend(all_text(child));
        }
        values
    }

    fn positioned_text(node: &PositionedNode) -> Vec<&str> {
        let mut values = Vec::new();
        if let Some(Content::Text(text)) = &node.content {
            values.push(text.value.as_str());
        }
        for child in &node.children {
            values.extend(positioned_text(child));
        }
        values
    }

    fn find_positioned_text<'a>(
        node: &'a PositionedNode,
        value: &str,
    ) -> Option<&'a PositionedNode> {
        if matches!(&node.content, Some(Content::Text(text)) if text.value == value) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_positioned_text(child, value))
    }

    fn find_positioned_by_id<'a>(node: &'a PositionedNode, id: &str) -> Option<&'a PositionedNode> {
        if node.id.as_deref() == Some(id) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_positioned_by_id(child, id))
    }

    fn find_positioned_by_html_role<'a>(
        node: &'a PositionedNode,
        role: &str,
    ) -> Option<&'a PositionedNode> {
        if positioned_html_string(node, "role") == Some(role) {
            return Some(node);
        }
        node.children
            .iter()
            .find_map(|child| find_positioned_by_html_role(child, role))
    }

    fn positioned_html_string<'a>(node: &'a PositionedNode, key: &str) -> Option<&'a str> {
        let ExtValue::Map(values) = node.ext.get("html")? else {
            return None;
        };
        let ExtValue::Str(value) = values.get(key)? else {
            return None;
        };
        Some(value)
    }
}
