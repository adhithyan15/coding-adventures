//! Compare the actual emitted title glyphs with the admitted layout bands.
use super::*;

#[test]
fn imported_subgraph_margin_emits_exact_enclosed_title_glyphs() {
    let mut cells=0;
    for direction in [DiagramDirection::Lr,DiagramDirection::Rl,DiagramDirection::Tb,DiagramDirection::Bt] {
        for padding in [None,Some(100.0),Some(65_536.0)] {
            for (top,bottom) in [(0.0,0.0),(100.0,7.0),(65_536.0,65_536.0)] {
                let mut input=mermaid_parser::parse_swimlane("swimlane-beta LR\nsubgraph L\nsubgraph G\nsubgraph H\na[N]\nend\nend\nend").unwrap();
                input.direction=direction.clone();input.flowchart_config.padding=padding;
                input.flowchart_config.subgraph_title_margin_top=Some(top);
                input.flowchart_config.subgraph_title_margin_bottom=Some(bottom);
                let layout=diagram_layout_hierarchy::try_layout_swimlane(&input).unwrap();
                assert_eq!((layout.lanes.len(),layout.groups.len()),(1,2));
                let original=layout.clone();
                let opts=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
                let scene=diagram_to_paint_swimlane(&layout,&opts);
                assert_eq!(layout,original);
                let lane=&layout.lanes[0];
                let mut boxes=vec![(lane.label.as_str(),lane.x+8.0,lane.y+7.0+top,lane.width-16.0,26.0,opts.title_font.clone(),Color{r:38,g:50,b:56,a:255})];
                for group in &layout.groups {
                    boxes.push((&group.label,group.x+8.0,group.y+4.0+top,group.width-16.0,22.0,opts.label_font.clone(),Color{r:69,g:90,b:100,a:255}));
                }
                let children=boxes.iter().map(|(label,x,y,w,h,font,color)|text_node(label,*x,*y,*w,*h,font.clone(),*color)).collect();
                let expected=layout_to_paint(&PositionedNode{x:0.0,y:0.0,width:layout.width,height:layout.height,id:None,content:None,children,ext:HashMap::new()},
                    &LayoutToPaintOptions{width:layout.width,height:layout.height,background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
                let titles=|instructions:Vec<PaintInstruction>|instructions.into_iter().filter_map(|p|match p {
                    PaintInstruction::GlyphRun(run) if run.glyphs.len()==1 && [u32::from('L'),u32::from('G'),u32::from('H')].contains(&run.glyphs[0].glyph_id)=>Some(run),_=>None,
                }).collect::<Vec<_>>();
                let actual=titles(scene.instructions);let expected=titles(expected.instructions);
                assert_eq!(actual,expected,"complete glyph runs: {direction:?}, node padding={padding:?}, margins={top}/{bottom}");
                assert_eq!(actual.len(),3);
                for (run,(_,x,y,w,h,_,_)) in actual.iter().zip(&boxes) {
                    let glyph=&run.glyphs[0];
                    assert!(glyph.x>=*x && glyph.x+run.font_size/2.0<=x+w);
                    assert!(glyph.y-run.font_size*0.8+0.01>=*y && glyph.y+run.font_size*0.2<=y+h+0.01);
                    assert!(*y>=0.0 && y+h<=layout.height);
                }
                cells+=1;
            }
        }
    }
    assert_eq!(cells,36);
}
