//! Convert [`BrowserRenderTree`] values from `html-parser` into the shared
//! [`LayoutNode`] intermediate representation.
//!
//! This crate is deliberately a front-end adapter. Parsing stays in
//! `html-parser`; geometry stays in layout engines such as `layout-block`; and
//! painting stays in `layout-to-paint`.

use std::collections::HashMap;

use coding_adventures_html_parser::{BrowserRenderNode, BrowserRenderTree};
use layout_ir::{
    color_black, edges_all, edges_xy, font_bold, font_italic, font_spec, rgb, Color, ExtValue,
    FontSpec, ImageContent, ImageFit, LayoutNode, SizeValue, TextAlign, TextContent,
};

pub const VERSION: &str = "0.2.0";

/// Fully resolved visual defaults applied before a future CSS cascade exists.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlTheme {
    pub body_font: FontSpec,
    pub heading_fonts: [FontSpec; 6],
    pub code_font: FontSpec,
    pub text_color: Color,
    pub heading_color: Color,
    pub link_color: Color,
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
        page_background: rgb(192, 192, 192),
        page_padding: 16.0,
        block_spacing: 12.0,
        heading_spacing: 12.0,
    }
}

/// Convert a browser render tree into a shared layout tree.
///
/// The returned root fills the available width, wraps its content height, and
/// carries the theme's page background in the `paint` extension namespace.
pub fn html_render_tree_to_layout(
    render_tree: &BrowserRenderTree,
    theme: &HtmlTheme,
) -> LayoutNode {
    let style = InheritedStyle {
        font: theme.body_font.clone(),
        color: theme.text_color,
    };
    let children = render_tree
        .children
        .iter()
        .filter_map(|node| convert_node(node, theme, &style))
        .collect();

    LayoutNode::container(children)
        .with_padding(edges_all(theme.page_padding))
        .with_width(SizeValue::Fill)
        .with_height(SizeValue::Wrap)
        .with_ext("block", display_ext("block"))
        .with_ext("html", root_html_ext())
        .with_ext("paint", background_ext(theme.page_background))
}

#[derive(Clone)]
struct InheritedStyle {
    font: FontSpec,
    color: Color,
}

fn convert_node(
    node: &BrowserRenderNode,
    theme: &HtmlTheme,
    inherited: &InheritedStyle,
) -> Option<LayoutNode> {
    if node.display == "none" || node.hidden {
        return None;
    }

    let style = style_for_node(node, theme, inherited);
    let mut layout = match node.display.as_str() {
        "inline-text" => text_leaf(node.text.as_deref().unwrap_or_default(), &style),
        "line-break" => text_leaf("\n", &style),
        "inline-replaced" if node.role == "image" => image_leaf(node),
        _ => container_or_fallback(node, theme, &style),
    };

    if let Some(id) = &node.id {
        layout = layout.with_id(id);
    }
    apply_size_hints(&mut layout, node);
    apply_spacing(&mut layout, node, theme);
    layout.ext.insert("html".into(), html_ext(node));
    layout
        .ext
        .insert("block".into(), display_ext(node.display.as_str()));
    Some(layout)
}

fn container_or_fallback(
    node: &BrowserRenderNode,
    theme: &HtmlTheme,
    style: &InheritedStyle,
) -> LayoutNode {
    let children: Vec<_> = node
        .children
        .iter()
        .filter_map(|child| convert_node(child, theme, style))
        .collect();

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

fn text_leaf(value: &str, style: &InheritedStyle) -> LayoutNode {
    LayoutNode::leaf_text(TextContent {
        value: value.to_string(),
        font: style.font.clone(),
        color: style.color,
        max_lines: None,
        text_align: TextAlign::Start,
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

fn style_for_node(
    node: &BrowserRenderNode,
    theme: &HtmlTheme,
    inherited: &InheritedStyle,
) -> InheritedStyle {
    let mut style = inherited.clone();
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
        style.color = theme.link_color;
    }
    style
}

fn apply_size_hints(layout: &mut LayoutNode, node: &BrowserRenderNode) {
    if let Some(width) = parse_dimension(node.width.as_deref()) {
        layout.width = Some(SizeValue::Fixed(width));
    } else if node.display == "block" || node.display.starts_with("table") {
        layout.width = Some(SizeValue::Fill);
    } else if layout.width.is_none() {
        layout.width = Some(SizeValue::Wrap);
    }

    if let Some(height) = parse_dimension(node.height.as_deref()) {
        layout.height = Some(SizeValue::Fixed(height));
    } else if layout.height.is_none() {
        layout.height = Some(SizeValue::Wrap);
    }
}

fn apply_spacing(layout: &mut LayoutNode, node: &BrowserRenderNode, theme: &HtmlTheme) {
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

fn background_ext(color: Color) -> ExtValue {
    ExtValue::Map(HashMap::from([(
        "backgroundColor".into(),
        color_ext(color),
    )]))
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
                line_count: 1,
            }
        }
    }

    #[test]
    fn mosaic_theme_uses_era_defaults() {
        let theme = mosaic_html_theme();
        assert_eq!(theme.body_font.family, "Times New Roman");
        assert_eq!(theme.link_color, rgb(0, 0, 238));
        assert_eq!(theme.page_background, rgb(192, 192, 192));
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
