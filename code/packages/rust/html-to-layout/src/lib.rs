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
use layout_flexbox::{
    flex_ext, AlignContent, AlignItems, AlignSelf, FlexBasis, FlexContainerStyle, FlexDirection,
    FlexItemStyle, FlexWrap, JustifyContent,
};
use layout_float::{float_ext, Clear, FloatSide, FloatStyle};
use layout_generated::{
    evaluate_content, format_marker, generated_ext, ContentPart, CounterChange, CounterContext,
    CounterStyle, GeneratedKind, MarkerPosition,
};
use layout_grid::{
    grid_ext, GridAlignment, GridAutoFlow, GridContainerStyle, GridContentAlignment, GridItemStyle,
    GridSelfAlignment, GridTrack,
};
use layout_inline_box::{inline_box_ext, BoxDecorationBreak};
use layout_ir::{
    color_black, edges_all, edges_xy, font_bold, font_italic, font_spec, rgb, Color, Edges,
    ExtValue, FontSpec, ImageContent, ImageFit, LayoutNode, SizeValue, TextAlign, TextContent,
    TextDecoration,
};
use layout_positioned::{positioned_ext, Overflow, Position, PositionedStyle};
use layout_replaced::replaced_ext;
use layout_table::{
    table_ext, BorderCollapse, CaptionSide, TableContainerStyle, TableItemStyle, TableLayout,
    VerticalAlign,
};
use lexer::token::TokenType;
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
    pub image_intrinsics: HashMap<String, (f64, f64)>,
}

impl HtmlStyleContext {
    pub fn new(theme: HtmlTheme) -> Self {
        Self {
            theme,
            author_stylesheets: Vec::new(),
            viewport_width: 800.0,
            viewport_height: 600.0,
            image_intrinsics: HashMap::new(),
        }
    }

    pub fn with_viewport(mut self, width: f64, height: f64) -> Self {
        self.viewport_width = finite_non_negative(width);
        self.viewport_height = finite_non_negative(height);
        self
    }

    pub fn with_image_intrinsics(
        mut self,
        values: impl IntoIterator<Item = (String, f64, f64)>,
    ) -> Self {
        self.image_intrinsics = values
            .into_iter()
            .filter_map(|(url, width, height)| {
                (width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0)
                    .then_some((url, (width, height)))
            })
            .collect();
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
            image_intrinsics: HashMap::new(),
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
    pub box_decoration_break: BoxDecorationBreak,
    pub aspect_ratio: Option<f64>,
    pub object_fit: ImageFit,
    pub float: FloatStyle,
    pub flex_container: FlexContainerStyle,
    pub flex_item: FlexItemStyle,
    pub grid_container: GridContainerStyle,
    pub grid_item: GridItemStyle,
    pub table_container: TableContainerStyle,
    pub table_item: TableItemStyle,
    pub positioned: PositionedStyle,
    pub generated_content: Option<Vec<ContentPart>>,
    pub list_style_type: Option<CounterStyle>,
    pub list_style_position: MarkerPosition,
    pub counter_reset: Vec<CounterChange>,
    pub counter_set: Vec<CounterChange>,
    pub counter_increment: Vec<CounterChange>,
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
    let mut counters = CounterContext::default();
    let _root_counter_scope = counters.enter(&style.counter_reset, &style.counter_set);
    counters.increment(&style.counter_increment);
    let children = convert_children(
        &render_tree.children,
        context,
        &style,
        &ancestors,
        &mut counters,
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
        .with_ext("positioned", positioned_ext(style.positioned))
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
    counters: &mut CounterContext,
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

    let counter_scope = counters.enter(&style.counter_reset, &style.counter_set);
    counters.increment(&style.counter_increment);

    let supports_generated =
        node.name.is_some() && !matches!(display, "inline-text" | "line-break" | "inline-replaced");
    let mut generated = Vec::new();
    if supports_generated {
        if display == "list-item" {
            counters.set("list-item", list_item_ordinal(node, ancestors));
            if let Some(marker) = marker_box(
                node, context, &style, ancestors, position, counters, is_visited,
            ) {
                generated.push(marker);
            }
        }
        if let Some(before) = pseudo_box(
            node,
            PseudoElement::Before,
            GeneratedKind::Before,
            context,
            &style,
            ancestors,
            position,
            counters,
            is_visited,
        ) {
            generated.push(before);
        }
    }

    let mut next_ancestors = ancestors.to_vec();
    next_ancestors.push(node);
    let mut layout = match display {
        "inline-text" => text_leaf(node.text.as_deref().unwrap_or_default(), &style),
        "line-break" => text_leaf("\n", &style),
        "inline-replaced" if node.role == "image" => image_leaf(node, &style, context),
        _ => container_or_fallback(node, context, &style, &next_ancestors, counters, is_visited),
    };

    if supports_generated {
        if display == "list-item" {
            counters.set("list-item", list_item_ordinal(node, ancestors));
        }
        let after = pseudo_box(
            node,
            PseudoElement::After,
            GeneratedKind::After,
            context,
            &style,
            ancestors,
            position,
            counters,
            is_visited,
        );
        if !generated.is_empty() || after.is_some() {
            if layout.content.is_some() {
                layout = LayoutNode::container(vec![layout]);
            }
            generated.append(&mut layout.children);
            if let Some(after) = after {
                generated.push(after);
            }
            layout.children = generated;
        }
    }

    if let Some(id) = &node.id {
        layout = layout.with_id(id);
    }
    apply_size_hints(&mut layout, node, &style, display);
    apply_spacing(&mut layout, node, &context.theme, &style);
    if let Some(padding) = style.padding {
        layout.padding = Some(padding);
    } else if display == "list-item" && style.list_style_position == MarkerPosition::Outside {
        layout.padding = Some(Edges {
            left: style.font.size * 1.75,
            ..Edges::default()
        });
    }
    if style.background.is_some() || style.border_width != Edges::default() {
        layout.ext.insert("paint".into(), box_paint_ext(&style));
    }
    layout.ext.insert("html".into(), html_ext(node));
    layout
        .ext
        .insert("block".into(), block_ext(node, display, &style));
    layout.ext.insert("float".into(), float_ext(style.float));
    layout.ext.insert(
        "inlineBox".into(),
        inline_box_ext(style.box_decoration_break),
    );
    layout.ext.insert(
        "flex".into(),
        flex_ext(style.flex_container, style.flex_item),
    );
    layout.ext.insert(
        "grid".into(),
        grid_ext(&style.grid_container, &style.grid_item),
    );
    layout.ext.insert(
        "table".into(),
        table_ext(style.table_container, &table_item_for_node(node, &style)),
    );
    layout
        .ext
        .insert("positioned".into(), positioned_ext(style.positioned));
    counters.exit(counter_scope);
    Some(layout)
}

fn convert_children<F>(
    nodes: &[BrowserRenderNode],
    context: &HtmlStyleContext,
    inherited: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    counters: &mut CounterContext,
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
            convert_node(
                node, context, inherited, ancestors, position, counters, is_visited,
            )
        })
        .collect()
}

fn container_or_fallback<F>(
    node: &BrowserRenderNode,
    context: &HtmlStyleContext,
    style: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    counters: &mut CounterContext,
    is_visited: &F,
) -> LayoutNode
where
    F: Fn(&str) -> bool + ?Sized,
{
    let children = convert_children(
        &node.children,
        context,
        style,
        ancestors,
        counters,
        is_visited,
    );

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

#[allow(clippy::too_many_arguments)]
fn pseudo_box<F>(
    node: &BrowserRenderNode,
    pseudo: PseudoElement,
    kind: GeneratedKind,
    context: &HtmlStyleContext,
    inherited: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    position: Option<NodePosition>,
    counters: &CounterContext,
    is_visited: &F,
) -> Option<LayoutNode>
where
    F: Fn(&str) -> bool + ?Sized,
{
    let style = style_for_pseudo(
        node, pseudo, inherited, ancestors, position, context, is_visited,
    );
    let content = style.generated_content.as_ref()?;
    if style.display.as_deref() == Some("none") {
        return None;
    }
    let value = evaluate_content(content, counters, |name| node_attribute(node, name));
    if value.is_empty() {
        return None;
    }
    Some(generated_text_box(
        &value,
        &style,
        kind,
        MarkerPosition::Inside,
    ))
}

#[allow(clippy::too_many_arguments)]
fn marker_box<F>(
    node: &BrowserRenderNode,
    context: &HtmlStyleContext,
    inherited: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    position: Option<NodePosition>,
    counters: &CounterContext,
    is_visited: &F,
) -> Option<LayoutNode>
where
    F: Fn(&str) -> bool + ?Sized,
{
    let mut style = style_for_pseudo(
        node,
        PseudoElement::Marker,
        inherited,
        ancestors,
        position,
        context,
        is_visited,
    );
    if style.display.as_deref() == Some("none") {
        return None;
    }
    style.white_space = "nowrap".into();
    let marker_style = inherited
        .list_style_type
        .or_else(|| html_marker_style(node, ancestors))
        .unwrap_or(CounterStyle::Disc);
    let value = if let Some(content) = style.generated_content.as_ref() {
        evaluate_content(content, counters, |name| node_attribute(node, name))
    } else {
        format_marker(list_item_ordinal(node, ancestors), marker_style)
    };
    if value.is_empty() {
        return None;
    }
    let value = if inherited.list_style_position == MarkerPosition::Inside {
        format!("{value} ")
    } else {
        value
    };
    Some(generated_text_box(
        &value,
        &style,
        GeneratedKind::Marker,
        inherited.list_style_position,
    ))
}

fn generated_text_box(
    value: &str,
    style: &HtmlComputedStyle,
    kind: GeneratedKind,
    position: MarkerPosition,
) -> LayoutNode {
    let mut node = text_leaf(value, style);
    node.ext
        .insert("generated".into(), generated_ext(kind, position));
    node.ext.insert("block".into(), display_ext("inline-text"));
    node
}

fn html_marker_style(
    node: &BrowserRenderNode,
    ancestors: &[&BrowserRenderNode],
) -> Option<CounterStyle> {
    node.list_marker_type
        .as_deref()
        .and_then(CounterStyle::parse)
        .or_else(|| {
            ancestors
                .iter()
                .rev()
                .find(|ancestor| ancestor.role == "list")
                .and_then(|list| {
                    list.list_marker_type
                        .as_deref()
                        .and_then(CounterStyle::parse)
                        .or(match list.list_kind.as_deref() {
                            Some("ordered") => Some(CounterStyle::Decimal),
                            Some("unordered" | "menu" | "directory") => Some(CounterStyle::Disc),
                            _ => None,
                        })
                })
        })
}

fn list_item_ordinal(node: &BrowserRenderNode, ancestors: &[&BrowserRenderNode]) -> i64 {
    let Some(list) = ancestors
        .iter()
        .rev()
        .find(|ancestor| ancestor.role == "list")
    else {
        return 1;
    };
    let item_count = list
        .children
        .iter()
        .filter(|child| child.role == "list_item")
        .count() as i64;
    let reversed = list.list_reversed;
    let mut value = list
        .list_start
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(if reversed { item_count } else { 1 });
    let step = if reversed { -1 } else { 1 };
    for child in &list.children {
        if child.role != "list_item" {
            continue;
        }
        if let Some(authored) = child
            .list_item_value
            .as_deref()
            .and_then(|value| value.parse::<i64>().ok())
        {
            value = authored;
        }
        if std::ptr::eq(child, node) {
            return value;
        }
        value = value.saturating_add(step);
    }
    value
}

fn image_leaf(
    node: &BrowserRenderNode,
    style: &HtmlComputedStyle,
    context: &HtmlStyleContext,
) -> LayoutNode {
    let source = node
        .resolved_src
        .as_deref()
        .or(node.src.as_deref())
        .unwrap_or_default();
    let mut layout = LayoutNode::leaf_image(ImageContent {
        src: source.to_string(),
        fit: style.object_fit,
    });
    let intrinsic = context.image_intrinsics.get(source).copied();
    layout.ext.insert(
        "replaced".into(),
        replaced_ext(
            intrinsic.map(|size| size.0),
            intrinsic.map(|size| size.1),
            style.aspect_ratio,
        ),
    );
    layout
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
    style.box_decoration_break = BoxDecorationBreak::Slice;
    style.aspect_ratio = None;
    style.object_fit = ImageFit::Fill;
    style.float = FloatStyle::default();
    style.flex_container = FlexContainerStyle::default();
    style.flex_item = FlexItemStyle::default();
    style.grid_container = GridContainerStyle::default();
    style.grid_item = GridItemStyle::default();
    style.table_container = TableContainerStyle::default();
    style.table_item = TableItemStyle::default();
    style.positioned = PositionedStyle::default();
    style.generated_content = None;
    style.counter_reset.clear();
    style.counter_set.clear();
    style.counter_increment.clear();
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
    pseudo_element: Option<PseudoElement>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PseudoElement {
    Before,
    After,
    Marker,
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
        box_decoration_break: BoxDecorationBreak::Slice,
        aspect_ratio: None,
        object_fit: ImageFit::Fill,
        float: FloatStyle::default(),
        flex_container: FlexContainerStyle::default(),
        flex_item: FlexItemStyle::default(),
        grid_container: GridContainerStyle::default(),
        grid_item: GridItemStyle::default(),
        table_container: TableContainerStyle::default(),
        table_item: TableItemStyle::default(),
        positioned: PositionedStyle::default(),
        generated_content: None,
        list_style_type: None,
        list_style_position: MarkerPosition::Outside,
        counter_reset: Vec::new(),
        counter_set: Vec::new(),
        counter_increment: Vec::new(),
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
                if selector.pseudo_element().is_none()
                    && selector.matches(node, ancestors, position, is_visited)
                {
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

fn style_for_pseudo<F>(
    node: &BrowserRenderNode,
    pseudo: PseudoElement,
    inherited: &HtmlComputedStyle,
    ancestors: &[&BrowserRenderNode],
    position: Option<NodePosition>,
    context: &HtmlStyleContext,
    is_visited: &F,
) -> HtmlComputedStyle
where
    F: Fn(&str) -> bool + ?Sized,
{
    let mut style = inherited.clone();
    style.display = Some("inline-text".into());
    style.background = None;
    style.generated_content = None;
    style.counter_reset.clear();
    style.counter_set.clear();
    style.counter_increment.clear();
    let mut winners = HashMap::new();
    let mut order = 0;
    for stylesheet in &context.author_stylesheets {
        for rule in &stylesheet.rules {
            if !rule_media_applies(rule, context) {
                continue;
            }
            for selector in &rule.selectors {
                if selector.pseudo_element() == Some(pseudo)
                    && selector.matches(node, ancestors, position, is_visited)
                {
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
            "content" => style.generated_content = parse_generated_content(&value),
            "list-style-type" => {
                style.list_style_type = value.first().and_then(|value| CounterStyle::parse(value))
            }
            "list-style-position" => {
                style.list_style_position = if value.first().is_some_and(|value| value == "inside")
                {
                    MarkerPosition::Inside
                } else {
                    MarkerPosition::Outside
                }
            }
            "list-style" => apply_list_style(style, &value),
            "counter-reset" => style.counter_reset = parse_counter_changes(&value, 0),
            "counter-set" => style.counter_set = parse_counter_changes(&value, 0),
            "counter-increment" => style.counter_increment = parse_counter_changes(&value, 1),
            "box-decoration-break" => {
                style.box_decoration_break = if value.first().is_some_and(|value| value == "clone")
                {
                    BoxDecorationBreak::Clone
                } else {
                    BoxDecorationBreak::Slice
                }
            }
            "aspect-ratio" => {
                style.aspect_ratio = parse_aspect_ratio(&value);
            }
            "object-fit" => {
                style.object_fit = match value.first().map(String::as_str) {
                    Some("contain") | Some("scale-down") => ImageFit::Contain,
                    Some("cover") => ImageFit::Cover,
                    Some("none") => ImageFit::None,
                    _ => ImageFit::Fill,
                }
            }
            "float" => {
                style.float.side = match value.first().map(String::as_str) {
                    Some("left") => FloatSide::Left,
                    Some("right") => FloatSide::Right,
                    _ => FloatSide::None,
                }
            }
            "clear" => {
                style.float.clear = match value.first().map(String::as_str) {
                    Some("left") => Clear::Left,
                    Some("right") => Clear::Right,
                    Some("both") => Clear::Both,
                    _ => Clear::None,
                }
            }
            "position" => {
                style.positioned.position = match value.first().map(String::as_str) {
                    Some("relative") => Position::Relative,
                    Some("absolute") => Position::Absolute,
                    Some("fixed") => Position::Fixed,
                    Some("sticky") => Position::Sticky,
                    _ => Position::Static,
                }
            }
            "top" => style.positioned.insets.top = parse_inset(&value, style, context, false),
            "right" => style.positioned.insets.right = parse_inset(&value, style, context, true),
            "bottom" => style.positioned.insets.bottom = parse_inset(&value, style, context, false),
            "left" => style.positioned.insets.left = parse_inset(&value, style, context, true),
            "inset" => apply_inset_shorthand(style, &value, context),
            "z-index" => {
                style.positioned.z_index = value
                    .first()
                    .filter(|value| value.as_str() != "auto")
                    .and_then(|value| value.parse().ok())
            }
            "overflow" => {
                let first = parse_overflow(&value);
                style.positioned.overflow_x = first;
                style.positioned.overflow_y = value
                    .get(1)
                    .map(|value| parse_overflow(std::slice::from_ref(value)))
                    .unwrap_or(first);
            }
            "overflow-x" => style.positioned.overflow_x = parse_overflow(&value),
            "overflow-y" => style.positioned.overflow_y = parse_overflow(&value),
            "table-layout" => {
                style.table_container.layout =
                    if value.first().is_some_and(|value| value == "fixed") {
                        TableLayout::Fixed
                    } else {
                        TableLayout::Auto
                    }
            }
            "border-collapse" => {
                style.table_container.border_collapse =
                    if value.first().is_some_and(|value| value == "collapse") {
                        BorderCollapse::Collapse
                    } else {
                        BorderCollapse::Separate
                    }
            }
            "border-spacing" => apply_border_spacing(style, &value, context),
            "caption-side" => {
                style.table_container.caption_side =
                    if value.first().is_some_and(|value| value == "bottom") {
                        CaptionSide::Bottom
                    } else {
                        CaptionSide::Top
                    }
            }
            "vertical-align" => {
                style.table_item.vertical_align = match value.first().map(String::as_str) {
                    Some("top" | "text-top") => VerticalAlign::Top,
                    Some("bottom" | "text-bottom") => VerticalAlign::Bottom,
                    _ => VerticalAlign::Middle,
                }
            }
            "flex-direction" => {
                style.flex_container.direction = match value.first().map(String::as_str) {
                    Some("row-reverse") => FlexDirection::RowReverse,
                    Some("column") => FlexDirection::Column,
                    Some("column-reverse") => FlexDirection::ColumnReverse,
                    _ => FlexDirection::Row,
                }
            }
            "flex-wrap" => {
                style.flex_container.wrap = match value.first().map(String::as_str) {
                    Some("wrap") => FlexWrap::Wrap,
                    Some("wrap-reverse") => FlexWrap::WrapReverse,
                    _ => FlexWrap::NoWrap,
                }
            }
            "flex-flow" => apply_flex_flow(style, &value),
            "gap" | "grid-gap" => apply_layout_gap(style, &value, context),
            "row-gap" | "grid-row-gap" => {
                if let Some(gap) = parse_box_length(&value, style, context, context.viewport_width)
                {
                    style.flex_container.row_gap = gap;
                    style.grid_container.row_gap = gap;
                }
            }
            "column-gap" | "grid-column-gap" => {
                if let Some(gap) = parse_box_length(&value, style, context, context.viewport_width)
                {
                    style.flex_container.column_gap = gap;
                    style.grid_container.column_gap = gap;
                }
            }
            "justify-content" => {
                style.flex_container.justify_content = parse_justify_content(&value);
                style.grid_container.justify_content = parse_grid_content_alignment(&value);
            }
            "align-items" => {
                style.flex_container.align_items = parse_align_items(&value);
                style.grid_container.align_items = parse_grid_alignment(&value);
            }
            "align-content" => {
                style.flex_container.align_content = parse_align_content(&value);
                style.grid_container.align_content = parse_grid_content_alignment(&value);
            }
            "align-self" => {
                style.flex_item.align_self = parse_align_self(&value);
                style.grid_item.align_self = parse_grid_self_alignment(&value);
            }
            "justify-items" => style.grid_container.justify_items = parse_grid_alignment(&value),
            "justify-self" => style.grid_item.justify_self = parse_grid_self_alignment(&value),
            "place-items" => apply_place_items(style, &value),
            "place-self" => apply_place_self(style, &value),
            "place-content" => apply_place_content(style, &value),
            "order" => {
                if let Some(order) = value.first().and_then(|value| value.parse().ok()) {
                    style.flex_item.order = order;
                    style.grid_item.order = order;
                }
            }
            "flex-grow" => {
                if let Some(grow) = parse_non_negative_number(&value) {
                    style.flex_item.grow = grow;
                }
            }
            "flex-shrink" => {
                if let Some(shrink) = parse_non_negative_number(&value) {
                    style.flex_item.shrink = shrink;
                }
            }
            "flex-basis" => style.flex_item.basis = parse_flex_basis(&value, style, context),
            "flex" => apply_flex_shorthand(style, &value, context),
            "grid-template-columns" => {
                if let Some(tracks) = parse_grid_track_list(&value, style, context) {
                    style.grid_container.template_columns = tracks;
                }
            }
            "grid-template-rows" => {
                if let Some(tracks) = parse_grid_track_list(&value, style, context) {
                    style.grid_container.template_rows = tracks;
                }
            }
            "grid-template-areas" => {
                style.grid_container.template_areas = parse_grid_template_areas(&value)
            }
            "grid-template" => apply_grid_template(style, &value, context),
            "grid" => apply_grid_template(style, &value, context),
            "grid-auto-columns" => {
                if let Some(track) = parse_grid_track(&value, style, context) {
                    style.grid_container.auto_columns = track;
                }
            }
            "grid-auto-rows" => {
                if let Some(track) = parse_grid_track(&value, style, context) {
                    style.grid_container.auto_rows = track;
                }
            }
            "grid-auto-flow" => style.grid_container.auto_flow = parse_grid_auto_flow(&value),
            "grid-column-start" => apply_grid_line(&mut style.grid_item, true, true, &value),
            "grid-column-end" => apply_grid_line(&mut style.grid_item, true, false, &value),
            "grid-row-start" => apply_grid_line(&mut style.grid_item, false, true, &value),
            "grid-row-end" => apply_grid_line(&mut style.grid_item, false, false, &value),
            "grid-column" => apply_grid_axis_shorthand(&mut style.grid_item, true, &value),
            "grid-row" => apply_grid_axis_shorthand(&mut style.grid_item, false, &value),
            "grid-area" => apply_grid_area(&mut style.grid_item, &value),
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

fn apply_flex_flow(style: &mut HtmlComputedStyle, value: &[String]) {
    for token in value {
        match token.as_str() {
            "row" => style.flex_container.direction = FlexDirection::Row,
            "row-reverse" => style.flex_container.direction = FlexDirection::RowReverse,
            "column" => style.flex_container.direction = FlexDirection::Column,
            "column-reverse" => style.flex_container.direction = FlexDirection::ColumnReverse,
            "nowrap" => style.flex_container.wrap = FlexWrap::NoWrap,
            "wrap" => style.flex_container.wrap = FlexWrap::Wrap,
            "wrap-reverse" => style.flex_container.wrap = FlexWrap::WrapReverse,
            _ => {}
        }
    }
}

fn apply_list_style(style: &mut HtmlComputedStyle, value: &[String]) {
    for token in value {
        if token == "inside" {
            style.list_style_position = MarkerPosition::Inside;
        } else if token == "outside" {
            style.list_style_position = MarkerPosition::Outside;
        } else if let Some(counter_style) = CounterStyle::parse(token) {
            style.list_style_type = Some(counter_style);
        }
    }
}

fn parse_counter_changes(value: &[String], default_value: i64) -> Vec<CounterChange> {
    if value.first().is_some_and(|token| token == "none") {
        return Vec::new();
    }
    let tokens = value
        .iter()
        .filter(|token| !matches!(token.as_str(), "," | "(" | ")"))
        .collect::<Vec<_>>();
    let mut changes = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let name = tokens[index];
        if name.parse::<i64>().is_ok() {
            index += 1;
            continue;
        }
        let authored = tokens
            .get(index + 1)
            .and_then(|token| token.parse::<i64>().ok());
        changes.push(CounterChange {
            name: name.to_string(),
            value: authored.unwrap_or(default_value),
        });
        index += usize::from(authored.is_some()) + 1;
    }
    changes
}

fn parse_generated_content(value: &[String]) -> Option<Vec<ContentPart>> {
    if value
        .first()
        .is_some_and(|token| matches!(token.as_str(), "none" | "normal"))
    {
        return None;
    }
    let source = value.join(" ");
    let mut parts = Vec::new();
    let mut index = 0;
    let bytes = source.as_bytes();
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() || bytes[index] == b',' {
            index += 1;
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"') {
            let quote = bytes[index];
            index += 1;
            let start = index;
            while index < bytes.len() && bytes[index] != quote {
                if bytes[index] == b'\\' && index + 1 < bytes.len() {
                    index += 1;
                }
                index += 1;
            }
            let text = String::from_utf8_lossy(&bytes[start..index])
                .replace("\\\"", "\"")
                .replace("\\'", "'")
                .replace("\\\\", "\\");
            index += usize::from(index < bytes.len());
            parts.push(ContentPart::Text(text));
            continue;
        }
        let start = index;
        while index < bytes.len()
            && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'-' | b'_'))
        {
            index += 1;
        }
        if start == index {
            index += 1;
            continue;
        }
        let name = source[start..index].to_ascii_lowercase();
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'(' {
            continue;
        }
        index += 1;
        let arguments_start = index;
        let mut depth = 1;
        let mut quote = None;
        while index < bytes.len() && depth > 0 {
            let byte = bytes[index];
            if let Some(active) = quote {
                if byte == active && bytes.get(index.wrapping_sub(1)) != Some(&b'\\') {
                    quote = None;
                }
            } else if matches!(byte, b'\'' | b'"') {
                quote = Some(byte);
            } else if byte == b'(' {
                depth += 1;
            } else if byte == b')' {
                depth -= 1;
            }
            index += 1;
        }
        let arguments_end = index.saturating_sub(1);
        let arguments = split_css_arguments(&source[arguments_start..arguments_end]);
        match name.as_str() {
            "attr" => {
                if let Some(attribute) = arguments.first() {
                    parts.push(ContentPart::Attribute(
                        attribute.trim().to_ascii_lowercase(),
                    ));
                }
            }
            "counter" => {
                if let Some(counter) = arguments.first() {
                    parts.push(ContentPart::Counter {
                        name: counter.trim().to_string(),
                        style: arguments
                            .get(1)
                            .and_then(|style| CounterStyle::parse(style.trim()))
                            .unwrap_or_default(),
                    });
                }
            }
            "counters" => {
                if let Some(counter) = arguments.first() {
                    parts.push(ContentPart::Counters {
                        name: counter.trim().to_string(),
                        separator: arguments
                            .get(1)
                            .map(|separator| trim_css_string(separator.trim()))
                            .unwrap_or_default(),
                        style: arguments
                            .get(2)
                            .and_then(|style| CounterStyle::parse(style.trim()))
                            .unwrap_or_default(),
                    });
                }
            }
            _ => {}
        }
    }
    (!parts.is_empty()).then_some(parts)
}

fn split_css_arguments(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut start = 0;
    let mut quote = None;
    for (index, character) in value.char_indices() {
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character == ',' {
            values.push(value[start..index].trim().to_string());
            start = index + 1;
        }
    }
    values.push(value[start..].trim().to_string());
    values
}

fn apply_layout_gap(style: &mut HtmlComputedStyle, value: &[String], context: &HtmlStyleContext) {
    let values = value
        .iter()
        .filter(|token| token.as_str() != ",")
        .collect::<Vec<_>>();
    let parse = |token: &String| {
        parse_box_length(
            std::slice::from_ref(token),
            style,
            context,
            context.viewport_width,
        )
    };
    if let Some(row_gap) = values.first().and_then(|token| parse(token)) {
        let column_gap = values
            .get(1)
            .and_then(|token| parse(token))
            .unwrap_or(row_gap);
        style.flex_container.row_gap = row_gap;
        style.flex_container.column_gap = column_gap;
        style.grid_container.row_gap = row_gap;
        style.grid_container.column_gap = column_gap;
    }
}

fn parse_inset(
    value: &[String],
    style: &HtmlComputedStyle,
    context: &HtmlStyleContext,
    horizontal: bool,
) -> Option<f64> {
    if value.first().is_some_and(|value| value == "auto") {
        return None;
    }
    parse_box_length(
        value,
        style,
        context,
        if horizontal {
            context.viewport_width
        } else {
            context.viewport_height
        },
    )
}

fn apply_inset_shorthand(
    style: &mut HtmlComputedStyle,
    value: &[String],
    context: &HtmlStyleContext,
) {
    let expanded = match value {
        [all] => [all, all, all, all],
        [vertical, horizontal] => [vertical, horizontal, vertical, horizontal],
        [top, horizontal, bottom] => [top, horizontal, bottom, horizontal],
        [top, right, bottom, left, ..] => [top, right, bottom, left],
        _ => return,
    };
    style.positioned.insets.top =
        parse_inset(std::slice::from_ref(expanded[0]), style, context, false);
    style.positioned.insets.right =
        parse_inset(std::slice::from_ref(expanded[1]), style, context, true);
    style.positioned.insets.bottom =
        parse_inset(std::slice::from_ref(expanded[2]), style, context, false);
    style.positioned.insets.left =
        parse_inset(std::slice::from_ref(expanded[3]), style, context, true);
}

fn parse_overflow(value: &[String]) -> Overflow {
    match value.first().map(String::as_str) {
        Some("hidden") => Overflow::Hidden,
        Some("auto") => Overflow::Auto,
        Some("scroll") => Overflow::Scroll,
        _ => Overflow::Visible,
    }
}

fn apply_border_spacing(
    style: &mut HtmlComputedStyle,
    value: &[String],
    context: &HtmlStyleContext,
) {
    let tokens: Vec<_> = value.iter().filter(|token| token.as_str() != ",").collect();
    let Some(horizontal) = tokens.first().and_then(|token| {
        parse_box_length(
            std::slice::from_ref(*token),
            style,
            context,
            context.viewport_width,
        )
    }) else {
        return;
    };
    let vertical = tokens
        .get(1)
        .and_then(|token| {
            parse_box_length(
                std::slice::from_ref(*token),
                style,
                context,
                context.viewport_height,
            )
        })
        .unwrap_or(horizontal);
    style.table_container.border_spacing_x = horizontal;
    style.table_container.border_spacing_y = vertical;
}

fn parse_grid_alignment(value: &[String]) -> GridAlignment {
    match value.first().map(String::as_str) {
        Some("start" | "flex-start" | "self-start" | "left") => GridAlignment::Start,
        Some("end" | "flex-end" | "self-end" | "right") => GridAlignment::End,
        Some("center") => GridAlignment::Center,
        _ => GridAlignment::Stretch,
    }
}

fn parse_grid_self_alignment(value: &[String]) -> GridSelfAlignment {
    match value.first().map(String::as_str) {
        Some("start" | "flex-start" | "self-start" | "left") => GridSelfAlignment::Start,
        Some("end" | "flex-end" | "self-end" | "right") => GridSelfAlignment::End,
        Some("center") => GridSelfAlignment::Center,
        Some("stretch") => GridSelfAlignment::Stretch,
        _ => GridSelfAlignment::Auto,
    }
}

fn parse_grid_content_alignment(value: &[String]) -> GridContentAlignment {
    match value.first().map(String::as_str) {
        Some("start" | "flex-start" | "left") => GridContentAlignment::Start,
        Some("end" | "flex-end" | "right") => GridContentAlignment::End,
        Some("center") => GridContentAlignment::Center,
        Some("space-between") => GridContentAlignment::SpaceBetween,
        Some("space-around") => GridContentAlignment::SpaceAround,
        Some("space-evenly") => GridContentAlignment::SpaceEvenly,
        _ => GridContentAlignment::Stretch,
    }
}

fn apply_place_items(style: &mut HtmlComputedStyle, value: &[String]) {
    if value.is_empty() {
        return;
    }
    style.grid_container.align_items = parse_grid_alignment(&value[..1]);
    style.grid_container.justify_items = value
        .get(1)
        .map(|_| parse_grid_alignment(&value[1..2]))
        .unwrap_or(style.grid_container.align_items);
}

fn apply_place_self(style: &mut HtmlComputedStyle, value: &[String]) {
    if value.is_empty() {
        return;
    }
    style.grid_item.align_self = parse_grid_self_alignment(&value[..1]);
    style.grid_item.justify_self = value
        .get(1)
        .map(|_| parse_grid_self_alignment(&value[1..2]))
        .unwrap_or(style.grid_item.align_self);
}

fn apply_place_content(style: &mut HtmlComputedStyle, value: &[String]) {
    if value.is_empty() {
        return;
    }
    style.grid_container.align_content = parse_grid_content_alignment(&value[..1]);
    style.grid_container.justify_content = value
        .get(1)
        .map(|_| parse_grid_content_alignment(&value[1..2]))
        .unwrap_or(style.grid_container.align_content);
}

fn parse_grid_auto_flow(value: &[String]) -> GridAutoFlow {
    let column = value.iter().any(|token| token == "column");
    let dense = value.iter().any(|token| token == "dense");
    match (column, dense) {
        (true, true) => GridAutoFlow::ColumnDense,
        (true, false) => GridAutoFlow::Column,
        (false, true) => GridAutoFlow::RowDense,
        (false, false) => GridAutoFlow::Row,
    }
}

fn parse_grid_track_list(
    value: &[String],
    style: &HtmlComputedStyle,
    context: &HtmlStyleContext,
) -> Option<Vec<GridTrack>> {
    let source = normalized_grid_track_source(value, style, context);
    GridTrack::parse_list(&source).ok()
}

fn parse_grid_track(
    value: &[String],
    style: &HtmlComputedStyle,
    context: &HtmlStyleContext,
) -> Option<GridTrack> {
    let source = normalized_grid_track_source(value, style, context);
    GridTrack::parse(&source).ok()
}

fn normalized_grid_track_source(
    value: &[String],
    style: &HtmlComputedStyle,
    context: &HtmlStyleContext,
) -> String {
    let converted = value
        .iter()
        .map(|token| {
            if token == "0" {
                return "0px".into();
            }
            if token.ends_with("em") || token.ends_with("rem") {
                return parse_css_length_in_context(
                    std::slice::from_ref(token),
                    style.font.size,
                    context.theme.body_font.size,
                    context.viewport_width,
                )
                .map(|value| format!("{value}px"))
                .unwrap_or_else(|| token.clone());
            }
            token.clone()
        })
        .collect::<Vec<_>>();
    css_component_text(&converted)
}

fn css_component_text(tokens: &[String]) -> String {
    let mut result = String::new();
    for token in tokens {
        let punctuation = matches!(token.as_str(), "(" | ")" | ",");
        let adjacent =
            result.ends_with('(') || result.ends_with(',') || token == ")" || token == ",";
        if !result.is_empty() && !punctuation && !adjacent {
            result.push(' ');
        }
        if token == "(" && result.ends_with(' ') {
            result.pop();
        }
        result.push_str(token);
    }
    result
}

fn parse_grid_template_areas(value: &[String]) -> Vec<Vec<Option<String>>> {
    value
        .iter()
        .filter(|token| !matches!(token.as_str(), "none" | "," | "/"))
        .map(|row| {
            row.trim_matches(['\'', '"'])
                .split_whitespace()
                .map(|name| (name != ".").then(|| name.to_string()))
                .collect()
        })
        .collect()
}

fn apply_grid_template(
    style: &mut HtmlComputedStyle,
    value: &[String],
    context: &HtmlStyleContext,
) {
    let Some(slash) = value.iter().position(|token| token == "/") else {
        return;
    };
    if let Some(rows) = parse_grid_track_list(&value[..slash], style, context) {
        style.grid_container.template_rows = rows;
    }
    if let Some(columns) = parse_grid_track_list(&value[slash + 1..], style, context) {
        style.grid_container.template_columns = columns;
    }
}

fn apply_grid_line(item: &mut GridItemStyle, column: bool, start: bool, value: &[String]) {
    if value.first().is_some_and(|token| token == "auto") {
        set_grid_line(item, column, start, None);
        return;
    }
    if let Some(span) = value
        .iter()
        .position(|token| token == "span")
        .and_then(|index| value.get(index + 1))
        .and_then(|token| token.parse::<usize>().ok())
        .filter(|span| *span > 0)
    {
        if column {
            item.column_span = span;
        } else {
            item.row_span = span;
        }
        return;
    }
    let line = value
        .iter()
        .find_map(|token| token.parse::<usize>().ok())
        .filter(|line| *line > 0);
    set_grid_line(item, column, start, line);
}

fn set_grid_line(item: &mut GridItemStyle, column: bool, start: bool, value: Option<usize>) {
    match (column, start) {
        (true, true) => item.column_start = value,
        (true, false) => item.column_end = value,
        (false, true) => item.row_start = value,
        (false, false) => item.row_end = value,
    }
}

fn apply_grid_axis_shorthand(item: &mut GridItemStyle, column: bool, value: &[String]) {
    let slash = value.iter().position(|token| token == "/");
    let (start, end) = slash
        .map(|index| (&value[..index], &value[index + 1..]))
        .unwrap_or((value, &[]));
    apply_grid_line(item, column, true, start);
    if !end.is_empty() {
        apply_grid_line(item, column, false, end);
    }
}

fn apply_grid_area(item: &mut GridItemStyle, value: &[String]) {
    if !value.iter().any(|token| token == "/") {
        item.area = value
            .first()
            .filter(|token| token.as_str() != "auto")
            .cloned();
        return;
    }
    let parts = value.split(|token| token == "/").collect::<Vec<_>>();
    for (index, part) in parts.into_iter().take(4).enumerate() {
        match index {
            0 => apply_grid_line(item, false, true, part),
            1 => apply_grid_line(item, true, true, part),
            2 => apply_grid_line(item, false, false, part),
            3 => apply_grid_line(item, true, false, part),
            _ => unreachable!(),
        }
    }
}

fn parse_justify_content(value: &[String]) -> JustifyContent {
    match value.first().map(String::as_str) {
        Some("end" | "flex-end" | "right") => JustifyContent::End,
        Some("center") => JustifyContent::Center,
        Some("space-between") => JustifyContent::SpaceBetween,
        Some("space-around") => JustifyContent::SpaceAround,
        Some("space-evenly") => JustifyContent::SpaceEvenly,
        _ => JustifyContent::Start,
    }
}

fn parse_align_items(value: &[String]) -> AlignItems {
    match value.first().map(String::as_str) {
        Some("start" | "flex-start" | "self-start") => AlignItems::Start,
        Some("end" | "flex-end" | "self-end") => AlignItems::End,
        Some("center") => AlignItems::Center,
        _ => AlignItems::Stretch,
    }
}

fn parse_align_self(value: &[String]) -> AlignSelf {
    match value.first().map(String::as_str) {
        Some("start" | "flex-start" | "self-start") => AlignSelf::Start,
        Some("end" | "flex-end" | "self-end") => AlignSelf::End,
        Some("center") => AlignSelf::Center,
        Some("stretch") => AlignSelf::Stretch,
        _ => AlignSelf::Auto,
    }
}

fn parse_align_content(value: &[String]) -> AlignContent {
    match value.first().map(String::as_str) {
        Some("start" | "flex-start") => AlignContent::Start,
        Some("end" | "flex-end") => AlignContent::End,
        Some("center") => AlignContent::Center,
        Some("space-between") => AlignContent::SpaceBetween,
        Some("space-around") => AlignContent::SpaceAround,
        Some("space-evenly") => AlignContent::SpaceEvenly,
        _ => AlignContent::Stretch,
    }
}

fn parse_non_negative_number(value: &[String]) -> Option<f64> {
    value
        .first()?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn parse_flex_basis(
    value: &[String],
    style: &HtmlComputedStyle,
    context: &HtmlStyleContext,
) -> FlexBasis {
    let Some(value) = value.first() else {
        return FlexBasis::Auto;
    };
    match value.as_str() {
        "auto" => FlexBasis::Auto,
        "content" => FlexBasis::Content,
        "min-content" => FlexBasis::MinContent,
        _ if value.ends_with('%') => value
            .strip_suffix('%')
            .and_then(|value| value.parse::<f64>().ok())
            .map(|value| FlexBasis::Percent((value / 100.0).max(0.0)))
            .unwrap_or(FlexBasis::Auto),
        _ => parse_css_length_in_context(
            std::slice::from_ref(value),
            style.font.size,
            context.theme.body_font.size,
            context.viewport_width,
        )
        .map(FlexBasis::Points)
        .unwrap_or(FlexBasis::Auto),
    }
}

fn apply_flex_shorthand(
    style: &mut HtmlComputedStyle,
    value: &[String],
    context: &HtmlStyleContext,
) {
    match value.first().map(String::as_str) {
        Some("none") => {
            style.flex_item.grow = 0.0;
            style.flex_item.shrink = 0.0;
            style.flex_item.basis = FlexBasis::Auto;
            return;
        }
        Some("auto") => {
            style.flex_item.grow = 1.0;
            style.flex_item.shrink = 1.0;
            style.flex_item.basis = FlexBasis::Auto;
            return;
        }
        Some("initial") => {
            style.flex_item = FlexItemStyle::default();
            return;
        }
        _ => {}
    }
    let mut numbers = value
        .iter()
        .filter_map(|token| token.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0);
    if let Some(grow) = numbers.next() {
        style.flex_item.grow = grow;
        style.flex_item.shrink = numbers.next().unwrap_or(1.0);
        style.flex_item.basis = value
            .iter()
            .find(|token| token.parse::<f64>().is_err())
            .map(|token| parse_flex_basis(std::slice::from_ref(token), style, context))
            .unwrap_or(FlexBasis::Percent(0.0));
    }
}

impl Selector {
    fn pseudo_element(&self) -> Option<PseudoElement> {
        self.compounds
            .last()
            .and_then(|compound| compound.pseudo_element)
    }

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
            && self.compounds[0].pseudo_element.is_none()
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
        let value = descendant_named_node(node, "value_list").map(descendant_css_value_tokens);
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
        total.2 += u16::from(compound.pseudo_element.is_some());
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
            "pseudo_element" => {
                compound.pseudo_element = descendant_tokens(child).iter().find_map(|token| {
                    match token.trim_start_matches(':') {
                        "before" => Some(PseudoElement::Before),
                        "after" => Some(PseudoElement::After),
                        "marker" => Some(PseudoElement::Marker),
                        _ => None,
                    }
                });
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

fn descendant_css_value_tokens(node: &GrammarASTNode) -> Vec<String> {
    let mut tokens = Vec::new();
    collect_css_value_tokens(node, &mut tokens);
    tokens
}

fn collect_css_value_tokens(node: &GrammarASTNode, tokens: &mut Vec<String>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(node) => collect_css_value_tokens(node, tokens),
            ASTNodeOrToken::Token(token) if token.type_ == TokenType::String => {
                tokens.push(format!("{:?}", token.value));
            }
            ASTNodeOrToken::Token(token) => tokens.push(token.value.clone()),
        }
    }
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
    } else {
        let number = value.strip_suffix('%')?;
        (number, percentage_basis / 100.0)
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

fn parse_aspect_ratio(value: &[String]) -> Option<f64> {
    let authored = value
        .iter()
        .filter(|token| token.as_str() != "auto")
        .cloned()
        .collect::<String>();
    let (numerator, denominator) = authored.split_once('/').unwrap_or((authored.as_str(), "1"));
    let numerator = numerator.parse::<f64>().ok()?;
    let denominator = denominator.parse::<f64>().ok()?;
    let ratio = numerator / denominator;
    (ratio.is_finite() && ratio > 0.0).then_some(ratio)
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
    } else if display == "block"
        || display == "list-item"
        || display == "flex"
        || display == "grid"
        || display.starts_with("table")
    {
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

fn table_item_for_node(node: &BrowserRenderNode, style: &HtmlComputedStyle) -> TableItemStyle {
    let mut item = style.table_item.clone();
    item.column_span = positive_attribute(node.colspan.as_deref())
        .or_else(|| positive_attribute(node.span.as_deref()))
        .unwrap_or(item.column_span)
        .min(1024);
    item.row_span = positive_attribute(node.rowspan.as_deref())
        .unwrap_or(item.row_span)
        .min(65_534);
    item.section_kind = node.table_section_kind.clone();
    item
}

fn positive_attribute(value: Option<&str>) -> Option<usize> {
    value?
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
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
    insert_optional(&mut values, "colspan", node.colspan.as_deref());
    insert_optional(&mut values, "rowspan", node.rowspan.as_deref());
    insert_optional(
        &mut values,
        "sectionKind",
        node.table_section_kind.as_deref(),
    );
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
    fn computed_inline_box_edges_fragment_through_shared_layout() {
        let render = parse_browser_render_tree(
            "<p id='line'><a id='edge' href='next.html'>one two three four five</a></p>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            [
                "#line { width: 70px; margin: 0; } #edge { margin: 0 2px; padding: 2px 4px; \
              border: 1px solid blue; background: red; box-decoration-break: clone; }",
            ],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let edge = find_by_id(&layout, "edge").unwrap();
        assert_eq!(
            layout_inline_box::InlineBoxStyle::from_layout(edge).decoration_break,
            BoxDecorationBreak::Clone
        );

        let positioned = layout_block(&layout, constraints_width(240.0), &TestMeasurer);
        let mut fragments = Vec::new();
        collect_positioned_by_href(&positioned, "next.html", &mut fragments);
        assert!(fragments.len() >= 2);
        assert!(fragments.iter().all(|fragment| {
            matches!(fragment.ext.get("paint"), Some(ExtValue::Map(values))
                if values.get("borderLeftWidth") == Some(&ExtValue::Float(1.0))
                    && values.get("borderRightWidth") == Some(&ExtValue::Float(1.0)))
        }));
        assert!(fragments.windows(2).all(|pair| pair[0].y < pair[1].y));
    }

    #[test]
    fn decoded_intrinsics_and_css_ratio_project_replaced_geometry() {
        let render =
            parse_browser_render_tree("<img id='hero' src='hero.gif' alt='hero'>").unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["#hero { width: 80px; height: auto; aspect-ratio: 4 / 3; object-fit: cover; }"],
        )
        .unwrap()
        .with_image_intrinsics([("hero.gif".into(), 40.0, 80.0)]);
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let hero = find_by_id(&layout, "hero").unwrap();
        assert!(matches!(
            &hero.content,
            Some(Content::Image(image)) if image.fit == ImageFit::Cover
        ));
        assert_eq!(
            layout_replaced::IntrinsicSize::from_layout(hero),
            layout_replaced::IntrinsicSize {
                width: Some(40.0),
                height: Some(80.0),
                aspect_ratio: Some(4.0 / 3.0),
            }
        );
        let positioned = layout_block(&layout, constraints_width(200.0), &TestMeasurer);
        let hero = find_positioned_by_id(&positioned, "hero").unwrap();
        assert_eq!((hero.width, hero.height), (80.0, 60.0));
    }

    #[test]
    fn computed_positioning_removes_items_from_flow_and_orders_stacking() {
        let render = parse_browser_render_tree(
            "<main id='stage'><div id='normal'>N</div><div id='front'>F</div>\
             <div id='back'>B</div><div id='shifted'>R</div></main>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["#stage { position: relative; width: 200px; height: 100px; overflow: hidden; } \
              #normal { height: 20px; } \
              #front { position: absolute; inset: 10px auto auto 30px; width: 40px; height: 20px; z-index: 4; } \
              #back { position: absolute; left: 5px; top: 2px; width: 20px; height: 10px; z-index: -1; } \
              #shifted { position: relative; left: 5px; top: 3px; height: 10px; }"],
        )
        .unwrap()
        .with_viewport(240.0, 160.0);
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let stage = find_by_id(&layout, "stage").unwrap();
        let stage_style = PositionedStyle::from_layout(stage);
        assert_eq!(stage_style.position, Position::Relative);
        assert_eq!(stage_style.overflow_x, Overflow::Hidden);

        let positioned = layout_block(&layout, constraints_width(240.0), &TestMeasurer);
        let stage = find_positioned_by_id(&positioned, "stage").unwrap();
        assert_eq!(stage.height, 100.0);
        assert_eq!(find_positioned_by_id(stage, "normal").unwrap().y, 0.0);
        assert_eq!(find_positioned_by_id(stage, "shifted").unwrap().y, 23.0);
        assert_eq!(find_positioned_by_id(stage, "front").unwrap().x, 30.0);
        assert_eq!(find_positioned_by_id(stage, "front").unwrap().y, 10.0);
        assert_eq!(stage.children.first().unwrap().id.as_deref(), Some("back"));
        assert_eq!(stage.children.last().unwrap().id.as_deref(), Some("front"));
    }

    #[test]
    fn computed_float_and_clear_values_drive_exclusion_geometry() {
        let render = parse_browser_render_tree(
            "<main id='stage'><div id='left'>L</div><div id='right'>R</div>\
             <p id='flow'>one two three four five six</p><div id='clear'>C</div></main>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["#stage { width: 240px; } \
              #left { float: left; width: 70px; height: 48px; } \
              #right { float: right; width: 50px; height: 36px; } \
              #flow { margin: 0; } #clear { clear: both; height: 20px; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        assert_eq!(
            FloatStyle::from_layout(find_by_id(&layout, "left").unwrap()).side,
            FloatSide::Left
        );
        assert_eq!(
            FloatStyle::from_layout(find_by_id(&layout, "clear").unwrap()).clear,
            Clear::Both
        );

        let positioned = layout_block(&layout, constraints_width(240.0), &TestMeasurer);
        let stage = find_positioned_by_id(&positioned, "stage").unwrap();
        assert_eq!(find_positioned_by_id(stage, "left").unwrap().x, 0.0);
        assert_eq!(
            find_positioned_by_id(stage, "right").unwrap().x,
            stage.width - 50.0
        );
        let flow = find_positioned_by_id(stage, "flow").unwrap();
        assert!(flow.x >= 70.0);
        assert!(flow.x + flow.width <= stage.width - 50.0);
        assert!(find_positioned_by_id(stage, "clear").unwrap().y >= 48.0);
    }

    #[test]
    fn computed_flex_values_drive_shared_wrapped_geometry() {
        let render = parse_browser_render_tree(
            "<div id='deck'><div id='a'>A</div><div id='b'>B</div><div id='c'>C</div></div>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["#deck { display: flex; width: 240px; height: 100px; \
                flex-flow: row wrap; gap: 10px; align-content: space-between; \
                align-items: center; } \
              #deck > div { flex: 0 0 100px; height: 20px; } \
              #a { order: 2; } #b { order: 0; } #c { order: 1; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let deck = find_by_id(&layout, "deck").unwrap();
        let flex = FlexContainerStyle::from_node(deck);
        assert_eq!(flex.wrap, FlexWrap::Wrap);
        assert_eq!(flex.row_gap, 10.0);
        assert_eq!(flex.column_gap, 10.0);
        assert_eq!(flex.align_content, AlignContent::SpaceBetween);
        assert_eq!(
            FlexItemStyle::from_node(find_by_id(&layout, "a").unwrap()).order,
            2
        );

        let positioned = layout_block(&layout, constraints_width(640.0), &TestMeasurer);
        let deck = find_positioned_by_id(&positioned, "deck").unwrap();
        assert_eq!(deck.children.len(), 3);
        assert_eq!(deck.children[0].id.as_deref(), Some("b"));
        assert_eq!(deck.children[1].id.as_deref(), Some("c"));
        assert_eq!(deck.children[2].id.as_deref(), Some("a"));
        assert_eq!(deck.children[0].x, 0.0);
        assert_eq!(deck.children[1].x, 110.0);
        assert_eq!(deck.children[2].y, 80.0);
        assert_eq!(deck.children[2].width, 100.0);
    }

    #[test]
    fn computed_grid_values_drive_named_track_geometry() {
        let render = parse_browser_render_tree(
            "<div id='grid'><div id='head'>Head</div><div id='main'>Main</div><div id='side'>Side</div></div>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["#grid { display: grid; width: 300px; height: 120px; \
                grid-template-columns: 80px minmax(50px, 1fr) 1fr; \
                grid-template-rows: 40px 1fr; gap: 10px; \
                grid-template-areas: 'head head side' 'main main side'; } \
              #head { grid-area: head; } #main { grid-area: main; } \
              #side { grid-area: side; order: -1; justify-self: center; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let grid = find_by_id(&layout, "grid").unwrap();
        let style = GridContainerStyle::from_node(grid);
        assert_eq!(style.template_columns.len(), 3);
        assert_eq!(style.template_areas.len(), 2);
        assert_eq!(style.row_gap, 10.0);
        assert_eq!(
            GridItemStyle::from_node(find_by_id(&layout, "side").unwrap()).order,
            -1
        );

        let positioned = layout_block(&layout, constraints_width(640.0), &TestMeasurer);
        let grid = find_positioned_by_id(&positioned, "grid").unwrap();
        assert_eq!(grid.children[0].id.as_deref(), Some("side"));
        assert_eq!((grid.children[0].x, grid.children[0].y), (236.0, 0.0));
        assert_eq!((grid.children[1].x, grid.children[1].y), (0.0, 0.0));
        assert_eq!((grid.children[2].x, grid.children[2].y), (0.0, 50.0));
        assert_eq!(grid.children[2].width, 190.0);
    }

    #[test]
    fn computed_table_values_drive_sections_spans_and_caption_geometry() {
        let render = parse_browser_render_tree(
            "<table id='prices'><tfoot><tr id='foot'><td id='total' colspan='2'>Total</td></tr></tfoot>\
             <tbody><tr id='body'><td id='item'>Tea</td><td>$4</td></tr></tbody>\
             <thead><tr id='head'><th id='label'>Item</th><th>Price</th></tr></thead>\
             <caption id='caption'>Menu</caption></table>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            [
                "#prices { width: 240px; table-layout: fixed; border-collapse: separate; \
                border-spacing: 4px 6px; caption-side: bottom; } \
              #label { width: 80px; } td { vertical-align: bottom; }",
            ],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let table = find_by_id(&layout, "prices").unwrap();
        let table_style = TableContainerStyle::from_node(table);
        assert_eq!(table_style.layout, TableLayout::Fixed);
        assert_eq!(table_style.border_spacing_x, 4.0);
        assert_eq!(table_style.border_spacing_y, 6.0);
        assert_eq!(
            TableItemStyle::from_node(find_by_id(&layout, "total").unwrap()).column_span,
            2
        );

        let positioned = layout_block(&layout, constraints_width(640.0), &TestMeasurer);
        let table = find_positioned_by_id(&positioned, "prices").unwrap();
        assert_eq!(table.width, 240.0);
        assert_eq!(table.children[0].id.as_deref(), Some("head"));
        assert_eq!(table.children[1].id.as_deref(), Some("body"));
        assert_eq!(table.children[2].id.as_deref(), Some("foot"));
        assert_eq!(table.children[3].id.as_deref(), Some("caption"));
        let head = &table.children[0];
        assert_eq!(head.children[0].width, 80.0);
        assert_eq!(head.children[1].x, 88.0);
        assert_eq!(table.children[2].children[0].width, 232.0);
    }

    #[test]
    fn list_markers_resolve_html_ordinals_authored_style_and_outside_gutters() {
        let render = parse_browser_render_tree(
            "<ol start='3' reversed><li id='first'>Tea</li><li id='second' value='7'>Cake</li></ol>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["ol { list-style: upper-alpha outside; } li::marker { color: blue; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let first = find_by_id(&layout, "first").unwrap();
        let second = find_by_id(&layout, "second").unwrap();
        assert!(matches!(
            &first.children[0].content,
            Some(Content::Text(text)) if text.value == "C." && text.color == rgb(0, 0, 255)
        ));
        assert!(matches!(
            &second.children[0].content,
            Some(Content::Text(text)) if text.value == "G."
        ));
        assert_eq!(
            layout_generated::generated_kind(&first.children[0]),
            Some(GeneratedKind::Marker)
        );

        let positioned = layout_block(&layout, constraints_width(240.0), &TestMeasurer);
        let first = find_positioned_by_id(&positioned, "first").unwrap();
        assert!(first.children[0].x < first.children[1].x);
        assert_eq!(first.children[0].y, first.children[1].y);
    }

    #[test]
    fn pseudo_content_evaluates_scoped_counters_attributes_and_source_order() {
        let render = parse_browser_render_tree(
            "<section><h2 id='one' title='Intro'>Alpha</h2><h2 id='two' title='Next'>Beta</h2></section>",
        )
        .unwrap();
        let context = HtmlStyleContext::with_author_stylesheets(
            mosaic_html_theme(),
            ["section { counter-reset: chapter 0; } h2 { counter-increment: chapter; } \
              h2::before { content: 'Chapter ' counter(chapter, upper-roman) ': ' attr(title) ' — '; } \
              h2::after { content: '!'; color: red; }"],
        )
        .unwrap();
        let layout =
            html_render_tree_to_layout_with_style_context(&render, &context, &never_visited);
        let one = find_by_id(&layout, "one").unwrap();
        let two = find_by_id(&layout, "two").unwrap();
        let Some(Content::Text(one_before)) = &one.children[0].content else {
            panic!("expected generated text");
        };
        let Some(Content::Text(two_before)) = &two.children[0].content else {
            panic!("expected generated text");
        };
        assert_eq!(one_before.value, "Chapter I: Intro — ");
        assert_eq!(two_before.value, "Chapter II: Next — ");
        assert!(matches!(
            &one.children[2].content,
            Some(Content::Text(text)) if text.value == "!" && text.color == rgb(255, 0, 0)
        ));
        assert_eq!(
            layout_generated::generated_kind(&one.children[0]),
            Some(GeneratedKind::Before)
        );
        assert_eq!(
            layout_generated::generated_kind(&one.children[2]),
            Some(GeneratedKind::After)
        );
        assert!(one.children[0].id.is_none() && one.children[2].id.is_none());
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

    fn collect_positioned_by_href<'a>(
        node: &'a PositionedNode,
        href: &str,
        matches: &mut Vec<&'a PositionedNode>,
    ) {
        if positioned_html_string(node, "href") == Some(href) {
            matches.push(node);
        }
        for child in &node.children {
            collect_positioned_by_href(child, href, matches);
        }
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
