//! # draw-instructions-svg
//!
//! SVG renderer for backend-neutral draw instructions.

use draw_instructions::{
    DrawGroupInstruction, DrawInstruction, DrawRectInstruction, DrawScene, DrawTextInstruction, Metadata, Renderer,
};

pub const VERSION: &str = "0.1.0";

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn metadata_to_attributes(metadata: &Metadata) -> String {
    metadata
        .iter()
        .map(|(key, value)| format!(" data-{}=\"{}\"", key, xml_escape(value)))
        .collect::<Vec<_>>()
        .join("")
}

fn render_rect(rect: &DrawRectInstruction) -> String {
    format!(
        "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"{} />",
        rect.x, rect.y, rect.width, rect.height, xml_escape(&rect.fill), metadata_to_attributes(&rect.metadata)
    )
}

fn render_text(text: &DrawTextInstruction) -> String {
    format!(
        "  <text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"{}>{}</text>",
        text.x, text.y, text.align, xml_escape(&text.font_family), text.font_size, xml_escape(&text.fill),
        metadata_to_attributes(&text.metadata), xml_escape(&text.value)
    )
}

fn render_group(group: &DrawGroupInstruction) -> String {
    let children = group.children.iter().map(render_instruction).collect::<Vec<_>>().join("\n");
    format!("  <g{}>\n{}\n  </g>", metadata_to_attributes(&group.metadata), children)
}

fn render_instruction(instruction: &DrawInstruction) -> String {
    match instruction {
        DrawInstruction::Rect(rect) => render_rect(rect),
        DrawInstruction::Text(text) => render_text(text),
        DrawInstruction::Group(group) => render_group(group),
    }
}

pub struct SvgRenderer;

impl Renderer<String> for SvgRenderer {
    fn render(&self, scene: &DrawScene) -> String {
        let label = scene.metadata.get("label").cloned().unwrap_or_else(|| "draw instructions scene".into());
        let mut lines = vec![
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\" aria-label=\"{}\">",
                scene.width, scene.height, scene.width, scene.height, xml_escape(&label)
            ),
            format!(
                "  <rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{}\" />",
                scene.width, scene.height, xml_escape(&scene.background)
            ),
        ];
        lines.extend(scene.instructions.iter().map(render_instruction));
        lines.push("</svg>".into());
        lines.join("\n")
    }
}

pub fn render_svg(scene: &DrawScene) -> String {
    SvgRenderer.render(scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use draw_instructions::{create_scene, draw_group, draw_rect, draw_text, Metadata};

    #[test]
    fn renders_svg() {
        let mut scene_meta = Metadata::new();
        scene_meta.insert("label".into(), "demo".into());
        let scene = create_scene(100, 50, vec![draw_rect(10, 10, 20, 30, "#000000", Metadata::new())], "", scene_meta);
        let svg = render_svg(&scene);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("aria-label=\"demo\""));
    }

    #[test]
    fn escapes_text_and_renders_groups() {
        let scene = create_scene(
            100,
            50,
            vec![draw_group(vec![draw_text(10, 20, "A&B", Metadata::new())], Metadata::new())],
            "",
            Metadata::new(),
        );
        let svg = render_svg(&scene);
        assert!(svg.contains("A&amp;B"));
        assert!(svg.contains("<g"));
    }
}
