// The 104 previous complete scenes keep their original oracles. Validate every
// changed default BEFORE reversing it, then compare every original field. This
// adapter never drops an instruction or touches metadata, glyphs or font queries.
fn restore_reviewed_venn_defaults(mut scene: PaintScene, layout: &LayoutedVennDiagram) -> PaintScene {
    for (index, circle) in layout.circles.iter().enumerate() {
        let PaintInstruction::Ellipse(ellipse)=&mut scene.instructions[index] else {panic!("base circle order changed");};
        let parts:Vec<_>=ellipse.fill.as_deref().unwrap().strip_prefix("rgba(").unwrap().strip_suffix(')').unwrap().split(',').collect();
        assert_eq!(parts.len(),4);
        let rgb=[parts[0].parse::<u8>().unwrap(),parts[1].parse::<u8>().unwrap(),parts[2].parse::<u8>().unwrap()];
        if circle.style.fill_opacity.is_none() {assert_eq!(parts[3],"0.1","new default opacity before reversal");}
        if circle.style.stroke.is_none() {
            let expected=format!("#{:02x}{:02x}{:02x}",rgb[0],rgb[1],rgb[2]);
            assert_eq!(ellipse.stroke.as_deref(),Some(expected.as_str()),"new raw resolved-fill stroke before reversal");
            ellipse.stroke=Some("#334155".into());
        }
        if circle.style.stroke_width.is_none() {
            let expected=match layout.width {400.0=>1.25,1600.0=>5.0,_=>panic!("legacy adapter admits only its declared fixture widths")};
            assert_eq!(ellipse.stroke_width,Some(expected),"new scaled default width before reversal");
            ellipse.stroke_width=Some(2.0);
        }
        if circle.style.fill_opacity.is_none() {ellipse.fill=Some(format!("rgba({},{},{},0.38)",rgb[0],rgb[1],rgb[2]));}
    }
    scene
}
