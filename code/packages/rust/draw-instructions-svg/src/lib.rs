//! # draw-instructions-svg
//!
//! SVG renderer for backend-neutral draw instructions.

use draw_instructions::{
    DrawClipInstruction, DrawGroupInstruction, DrawInstruction, DrawLineInstruction,
    DrawRectInstruction, DrawScene, DrawTextInstruction, Metadata, Renderer,
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
    let stroke_attrs = match (&rect.stroke, rect.stroke_width) {
        (Some(color), Some(width)) => format!(" stroke=\"{}\" stroke-width=\"{}\"", xml_escape(color), width),
        (Some(color), None) => format!(" stroke=\"{}\"", xml_escape(color)),
        _ => String::new(),
    };
    format!(
        "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"{}{} />",
        rect.x, rect.y, rect.width, rect.height, xml_escape(&rect.fill), stroke_attrs, metadata_to_attributes(&rect.metadata)
    )
}

fn render_text(text: &DrawTextInstruction) -> String {
    let weight_attr = match &text.font_weight {
        Some(w) if w != "normal" => format!(" font-weight=\"{}\"", xml_escape(w)),
        _ => String::new(),
    };
    format!(
        "  <text x=\"{}\" y=\"{}\" text-anchor=\"{}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\"{}{}>{}</text>",
        text.x, text.y, text.align, xml_escape(&text.font_family), text.font_size, xml_escape(&text.fill),
        weight_attr, metadata_to_attributes(&text.metadata), xml_escape(&text.value)
    )
}

fn render_line(line: &DrawLineInstruction) -> String {
    format!(
        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
        line.x1, line.y1, line.x2, line.y2, xml_escape(&line.stroke), line.stroke_width,
        metadata_to_attributes(&line.metadata)
    )
}

fn render_clip(clip: &DrawClipInstruction, clip_counter: &mut usize) -> String {
    let id = *clip_counter;
    *clip_counter += 1;
    let clip_id = format!("clip-{}", id);
    let children = clip
        .children
        .iter()
        .map(|c| render_instruction_with_counter(c, clip_counter))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "  <defs>\n    <clipPath id=\"{}\">\n      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n    </clipPath>\n  </defs>\n  <g clip-path=\"url(#{})\"{}>\n{}\n  </g>",
        clip_id, clip.x, clip.y, clip.width, clip.height, clip_id,
        metadata_to_attributes(&clip.metadata), children
    )
}

fn render_group_with_counter(group: &DrawGroupInstruction, clip_counter: &mut usize) -> String {
    let children = group
        .children
        .iter()
        .map(|c| render_instruction_with_counter(c, clip_counter))
        .collect::<Vec<_>>()
        .join("\n");
    format!("  <g{}>\n{}\n  </g>", metadata_to_attributes(&group.metadata), children)
}

fn render_instruction_with_counter(instruction: &DrawInstruction, clip_counter: &mut usize) -> String {
    match instruction {
        DrawInstruction::Rect(rect) => render_rect(rect),
        DrawInstruction::Text(text) => render_text(text),
        DrawInstruction::Group(group) => render_group_with_counter(group, clip_counter),
        DrawInstruction::Line(line) => render_line(line),
        DrawInstruction::Clip(clip) => render_clip(clip, clip_counter),
    }
}

pub struct SvgRenderer;

impl Renderer<String> for SvgRenderer {
    fn render(&self, scene: &DrawScene) -> String {
        let label = scene.metadata.get("label").cloned().unwrap_or_else(|| "draw instructions scene".into());
        let mut clip_counter: usize = 0;
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
        lines.extend(
            scene.instructions.iter().map(|i| render_instruction_with_counter(i, &mut clip_counter)),
        );
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
    use draw_instructions::{
        create_scene, draw_clip, draw_group, draw_line, draw_rect, draw_text,
        DrawInstruction, DrawRectInstruction, DrawTextInstruction, Metadata,
    };

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

    #[test]
    fn renders_line_instruction() {
        let scene = create_scene(
            200, 100,
            vec![draw_line(10.0, 20.0, 190.0, 80.0, "#ff0000", 2.5)],
            "", Metadata::new(),
        );
        let svg = render_svg(&scene);
        assert!(svg.contains("<line"), "expected <line> element in SVG");
        assert!(svg.contains("x1=\"10\""));
        assert!(svg.contains("y2=\"80\""));
        assert!(svg.contains("stroke=\"#ff0000\""));
        assert!(svg.contains("stroke-width=\"2.5\""));
    }

    #[test]
    fn renders_clip_instruction() {
        let child = draw_rect(5, 5, 10, 10, "#000", Metadata::new());
        let scene = create_scene(
            100, 100,
            vec![draw_clip(0.0, 0.0, 50.0, 50.0, vec![child])],
            "", Metadata::new(),
        );
        let svg = render_svg(&scene);
        assert!(svg.contains("<clipPath id=\"clip-0\""), "expected clipPath definition");
        assert!(svg.contains("clip-path=\"url(#clip-0)\""), "expected clip-path reference");
        assert!(svg.contains("<rect x=\"5\""), "expected child rect inside clip group");
    }

    #[test]
    fn renders_stroked_rect() {
        let rect = DrawInstruction::Rect(DrawRectInstruction {
            x: 10, y: 10, width: 80, height: 60,
            fill: "#ffffff".into(),
            stroke: Some("#333333".into()),
            stroke_width: Some(3.0),
            metadata: Metadata::new(),
        });
        let scene = create_scene(100, 100, vec![rect], "", Metadata::new());
        let svg = render_svg(&scene);
        assert!(svg.contains("stroke=\"#333333\""), "expected stroke attribute");
        assert!(svg.contains("stroke-width=\"3\""), "expected stroke-width attribute");
    }

    #[test]
    fn renders_bold_text() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 50, y: 50,
            value: "Bold".into(),
            fill: "#000".into(),
            font_family: "sans-serif".into(),
            font_size: 20,
            align: "middle".into(),
            font_weight: Some("bold".into()),
            metadata: Metadata::new(),
        });
        let scene = create_scene(100, 100, vec![text], "", Metadata::new());
        let svg = render_svg(&scene);
        assert!(svg.contains("font-weight=\"bold\""), "expected font-weight attribute");
    }

    #[test]
    fn normal_font_weight_omitted() {
        let text = DrawInstruction::Text(DrawTextInstruction {
            x: 50, y: 50,
            value: "Normal".into(),
            fill: "#000".into(),
            font_family: "sans-serif".into(),
            font_size: 20,
            align: "middle".into(),
            font_weight: Some("normal".into()),
            metadata: Metadata::new(),
        });
        let scene = create_scene(100, 100, vec![text], "", Metadata::new());
        let svg = render_svg(&scene);
        assert!(!svg.contains("font-weight"), "font-weight=\"normal\" should be omitted");
    }
}
