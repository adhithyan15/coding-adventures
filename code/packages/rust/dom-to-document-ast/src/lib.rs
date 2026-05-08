//! DOM to Document AST projection for Venture pipelines.
//!
//! `html-parser` deliberately stops at `dom-core::Document`. This package is
//! the separate adapter for consumers that want to feed parsed HTML into the
//! existing `document-ast` layout/rendering stack.

use document_ast::{
    BlockNode, BlockquoteNode, CodeBlockNode, CodeSpanNode, DocumentNode as DocumentAstNode,
    EmphasisNode, HardBreakNode, HeadingNode, ImageNode, InlineNode, LinkNode, ListChildNode,
    ListItemNode, ListNode, ParagraphNode, StrongNode, TextNode, ThematicBreakNode,
};
use dom_core::{Document, Element, Node};

/// Project a recovered Venture DOM document into the format-agnostic Document AST.
pub fn dom_to_document_ast(document: &Document) -> DocumentAstNode {
    DocumentAstNode {
        children: blocks_from_nodes(document_ast_body_nodes(document)),
    }
}

fn document_ast_body_nodes(document: &Document) -> &[Node] {
    let Some(html) = document.children.iter().find_map(|node| match node {
        Node::Element(element) if element.name == "html" => Some(element),
        _ => None,
    }) else {
        return document.children.as_slice();
    };

    html.children
        .iter()
        .find_map(|node| match node {
            Node::Element(element) if element.name == "body" => Some(element.children.as_slice()),
            _ => None,
        })
        .unwrap_or(html.children.as_slice())
}

fn blocks_from_nodes(nodes: &[Node]) -> Vec<BlockNode> {
    let mut blocks = Vec::new();
    let mut pending_inlines = Vec::new();

    for node in nodes {
        match node {
            Node::Element(element) if is_document_ast_block_element(&element.name) => {
                flush_paragraph(&mut blocks, &mut pending_inlines);
                blocks.extend(blocks_from_block_element(element));
            }
            Node::Element(element) if element_has_block_content(element) => {
                flush_paragraph(&mut blocks, &mut pending_inlines);
                blocks.extend(blocks_from_nodes(&element.children));
            }
            _ => {
                let inlines = inlines_from_node(node);
                if pending_inlines.is_empty() && inlines_are_whitespace(&inlines) {
                    continue;
                }
                for inline in inlines {
                    push_inline(&mut pending_inlines, inline);
                }
            }
        }
    }

    flush_paragraph(&mut blocks, &mut pending_inlines);
    blocks
}

fn blocks_from_block_element(element: &Element) -> Vec<BlockNode> {
    match element.name.as_str() {
        "html" | "body" | "div" | "center" | "section" | "article" | "main" => {
            blocks_from_nodes(&element.children)
        }
        "head" | "title" | "script" | "style" | "meta" | "link" | "base" => Vec::new(),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level = element.name[1..].parse::<u8>().unwrap_or(6).clamp(1, 6);
            vec![BlockNode::Heading(HeadingNode {
                level,
                children: inlines_from_nodes(&element.children),
            })]
        }
        "p" => paragraph_from_nodes(&element.children)
            .map(BlockNode::Paragraph)
            .into_iter()
            .collect(),
        "ul" | "ol" => vec![BlockNode::List(list_from_element(element))],
        "li" => vec![BlockNode::ListItem(list_item_from_nodes(&element.children))],
        "pre" | "listing" | "xmp" | "plaintext" => vec![BlockNode::CodeBlock(CodeBlockNode {
            language: None,
            value: code_block_value(&element.children),
        })],
        "blockquote" => vec![BlockNode::Blockquote(BlockquoteNode {
            children: blocks_from_nodes(&element.children),
        })],
        "hr" => vec![BlockNode::ThematicBreak(ThematicBreakNode)],
        _ => blocks_from_nodes(&element.children),
    }
}

fn paragraph_from_nodes(nodes: &[Node]) -> Option<ParagraphNode> {
    let children = inlines_from_nodes(nodes);
    (!inlines_are_whitespace(&children)).then_some(ParagraphNode { children })
}

fn list_from_element(element: &Element) -> ListNode {
    let mut children = Vec::new();
    let mut loose_children = Vec::new();

    for child in &element.children {
        match child {
            Node::Element(item) if item.name == "li" => {
                flush_list_item(&mut children, &mut loose_children);
                children.push(ListChildNode::ListItem(list_item_from_nodes(
                    &item.children,
                )));
            }
            _ => loose_children.push(child.clone()),
        }
    }
    flush_list_item(&mut children, &mut loose_children);

    ListNode {
        ordered: element.name == "ol",
        start: element
            .attribute("start")
            .and_then(|value| value.parse::<i64>().ok()),
        tight: false,
        children,
    }
}

fn flush_list_item(children: &mut Vec<ListChildNode>, loose_children: &mut Vec<Node>) {
    if loose_children.is_empty() {
        return;
    }

    let blocks = blocks_from_nodes(loose_children);
    if !blocks.is_empty() {
        children.push(ListChildNode::ListItem(ListItemNode { children: blocks }));
    }
    loose_children.clear();
}

fn list_item_from_nodes(nodes: &[Node]) -> ListItemNode {
    let children = blocks_from_nodes(nodes);
    if children.is_empty() {
        let inlines = inlines_from_nodes(nodes);
        if inlines_are_whitespace(&inlines) {
            ListItemNode {
                children: Vec::new(),
            }
        } else {
            ListItemNode {
                children: vec![BlockNode::Paragraph(ParagraphNode { children: inlines })],
            }
        }
    } else {
        ListItemNode { children }
    }
}

fn flush_paragraph(blocks: &mut Vec<BlockNode>, pending_inlines: &mut Vec<InlineNode>) {
    if inlines_are_whitespace(pending_inlines) {
        pending_inlines.clear();
        return;
    }

    blocks.push(BlockNode::Paragraph(ParagraphNode {
        children: std::mem::take(pending_inlines),
    }));
}

fn inlines_from_nodes(nodes: &[Node]) -> Vec<InlineNode> {
    let mut inlines = Vec::new();
    for node in nodes {
        for inline in inlines_from_node(node) {
            push_inline(&mut inlines, inline);
        }
    }
    inlines
}

fn inlines_from_node(node: &Node) -> Vec<InlineNode> {
    match node {
        Node::Text(text) => vec![InlineNode::Text(TextNode {
            value: text.data.clone(),
        })],
        Node::Element(element) => inlines_from_element(element),
        Node::Comment(_) | Node::DocumentType(_) => Vec::new(),
    }
}

fn inlines_from_element(element: &Element) -> Vec<InlineNode> {
    match element.name.as_str() {
        "em" | "i" => vec![InlineNode::Emphasis(EmphasisNode {
            children: inlines_from_nodes(&element.children),
        })],
        "strong" | "b" => vec![InlineNode::Strong(StrongNode {
            children: inlines_from_nodes(&element.children),
        })],
        "code" => vec![InlineNode::CodeSpan(CodeSpanNode {
            value: collect_text(&element.children),
        })],
        "a" => vec![InlineNode::Link(LinkNode {
            destination: element.attribute("href").unwrap_or_default().to_string(),
            title: optional_attribute(element, "title"),
            children: inlines_from_nodes(&element.children),
        })],
        "img" => vec![InlineNode::Image(ImageNode {
            destination: element.attribute("src").unwrap_or_default().to_string(),
            title: optional_attribute(element, "title"),
            alt: element.attribute("alt").unwrap_or_default().to_string(),
        })],
        "br" => vec![InlineNode::HardBreak(HardBreakNode)],
        "hr" => Vec::new(),
        _ => inlines_from_nodes(&element.children),
    }
}

fn optional_attribute(element: &Element, name: &str) -> Option<String> {
    element
        .attribute(name)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn push_inline(inlines: &mut Vec<InlineNode>, inline: InlineNode) {
    match (inlines.last_mut(), inline) {
        (Some(InlineNode::Text(existing)), InlineNode::Text(next)) => {
            existing.value.push_str(&next.value);
        }
        (_, inline) => inlines.push(inline),
    }
}

fn inlines_are_whitespace(inlines: &[InlineNode]) -> bool {
    inlines.iter().all(|inline| {
        matches!(inline, InlineNode::Text(text) if text.value.chars().all(char::is_whitespace))
    })
}

fn code_block_value(nodes: &[Node]) -> String {
    let mut value = collect_text(nodes);
    if !value.ends_with('\n') {
        value.push('\n');
    }
    value
}

fn collect_text(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            Node::Text(data) => text.push_str(&data.data),
            Node::Element(element) => text.push_str(&collect_text(&element.children)),
            Node::Comment(_) | Node::DocumentType(_) => {}
        }
    }
    text
}

fn element_has_block_content(element: &Element) -> bool {
    element.children.iter().any(|node| match node {
        Node::Element(child) => {
            is_document_ast_block_element(&child.name) || element_has_block_content(child)
        }
        _ => false,
    })
}

fn is_document_ast_block_element(name: &str) -> bool {
    matches!(
        name,
        "html"
            | "head"
            | "body"
            | "title"
            | "div"
            | "center"
            | "section"
            | "article"
            | "main"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "p"
            | "ul"
            | "ol"
            | "li"
            | "pre"
            | "listing"
            | "xmp"
            | "plaintext"
            | "blockquote"
            | "hr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_html_parser::parse_html;

    #[test]
    fn projects_mosaic_inline_and_block_tags_to_document_ast() {
        let dom = parse_html(
            "<h1>Hello <em>Venture</em></h1><p>See <a href=page.html title=Next><b>Mosaic</b></a> <img src=cat.gif alt=Cat></p>",
        )
        .unwrap();
        let document = dom_to_document_ast(&dom);

        assert_eq!(document.children.len(), 2);
        match &document.children[0] {
            BlockNode::Heading(heading) => {
                assert_eq!(heading.level, 1);
                assert_eq!(
                    heading.children,
                    vec![
                        InlineNode::Text(TextNode {
                            value: "Hello ".to_string()
                        }),
                        InlineNode::Emphasis(EmphasisNode {
                            children: vec![InlineNode::Text(TextNode {
                                value: "Venture".to_string()
                            })]
                        })
                    ]
                );
            }
            other => panic!("expected heading, got {other:?}"),
        }

        match &document.children[1] {
            BlockNode::Paragraph(paragraph) => {
                assert_eq!(paragraph.children.len(), 4);
                assert_eq!(
                    paragraph.children[0],
                    InlineNode::Text(TextNode {
                        value: "See ".to_string()
                    })
                );
                match &paragraph.children[1] {
                    InlineNode::Link(link) => {
                        assert_eq!(link.destination, "page.html");
                        assert_eq!(link.title.as_deref(), Some("Next"));
                        assert_eq!(
                            link.children,
                            vec![InlineNode::Strong(StrongNode {
                                children: vec![InlineNode::Text(TextNode {
                                    value: "Mosaic".to_string()
                                })]
                            })]
                        );
                    }
                    other => panic!("expected link, got {other:?}"),
                }
                assert_eq!(
                    paragraph.children[2],
                    InlineNode::Text(TextNode {
                        value: " ".to_string()
                    })
                );
                assert_eq!(
                    paragraph.children[3],
                    InlineNode::Image(ImageNode {
                        destination: "cat.gif".to_string(),
                        title: None,
                        alt: "Cat".to_string(),
                    })
                );
            }
            other => panic!("expected paragraph, got {other:?}"),
        }
    }

    #[test]
    fn projects_lists_pre_blockquotes_breaks_and_unknown_tags_to_document_ast() {
        let dom = parse_html(
            "<ul><li>One<li><unknown>Two</unknown></ul><blockquote><p>Quote<br>line</p></blockquote><pre>\n  code</pre><hr>",
        )
        .unwrap();
        let document = dom_to_document_ast(&dom);

        assert_eq!(document.children.len(), 4);
        match &document.children[0] {
            BlockNode::List(list) => {
                assert!(!list.ordered);
                assert_eq!(list.children.len(), 2);
                match &list.children[1] {
                    ListChildNode::ListItem(item) => match &item.children[0] {
                        BlockNode::Paragraph(paragraph) => assert_eq!(
                            paragraph.children,
                            vec![InlineNode::Text(TextNode {
                                value: "Two".to_string()
                            })]
                        ),
                        other => panic!("expected list paragraph, got {other:?}"),
                    },
                    other => panic!("expected list item, got {other:?}"),
                }
            }
            other => panic!("expected list, got {other:?}"),
        }

        match &document.children[1] {
            BlockNode::Blockquote(blockquote) => match &blockquote.children[0] {
                BlockNode::Paragraph(paragraph) => assert_eq!(
                    paragraph.children,
                    vec![
                        InlineNode::Text(TextNode {
                            value: "Quote".to_string()
                        }),
                        InlineNode::HardBreak(HardBreakNode),
                        InlineNode::Text(TextNode {
                            value: "line".to_string()
                        })
                    ]
                ),
                other => panic!("expected paragraph in blockquote, got {other:?}"),
            },
            other => panic!("expected blockquote, got {other:?}"),
        }

        assert_eq!(
            document.children[2],
            BlockNode::CodeBlock(CodeBlockNode {
                language: None,
                value: "  code\n".to_string(),
            })
        );
        assert_eq!(
            document.children[3],
            BlockNode::ThematicBreak(ThematicBreakNode)
        );
    }
}
