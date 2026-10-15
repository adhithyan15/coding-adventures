//! Compare complete shaped glyph traces and real declared metric bounds.
use super::*;
use diagram_ir::{RailroadConfig,RailroadDiagram,RailroadExpression as E,RailroadRule};

#[test]
fn railroad_small_padding_keeps_exact_leaf_glyphs_inside_emitted_boxes() {
    let mut cells=0;let mut failures=Vec::new();
    for padding in [0.0,0.5,3.5,4.0,10.0,65536.0] {
        for expression in [E::Terminal("A".into()),E::NonTerminal("B".into()),E::Special("C".into())] {
            let diagram=RailroadDiagram{title:None,accessibility_title:None,accessibility_description:None,
                config:RailroadConfig{padding,..Default::default()},rules:vec![RailroadRule{name:"r".into(),definition:expression}]};
            let layout=diagram_layout_hierarchy::layout_railroad(&diagram);let original=layout.clone();
            let element=&layout.rules[0].elements[0];let options=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
            let scene=diagram_to_paint_railroad(&layout,&options);assert_eq!(layout,original);
            let rectangles:Vec<_>=scene.instructions.iter().filter_map(|p|match p{PaintInstruction::Rect(r)=>Some(r),_=>None}).collect();
            assert_eq!(rectangles.len(),1);let rect=rectangles[0];
            assert_eq!((rect.x,rect.y,rect.width,rect.height),(element.x,element.y,element.width,element.height));
            let height=24.0_f64.min(element.height);
            let text=text_node_no_wrap(&element.label,element.x+padding,element.y+(element.height-height)/2.0,
                element.width-2.0*padding,height,options.label_font.clone(),Color{r:0,g:0,b:0,a:255});
            let expected=layout_to_paint(&text,&LayoutToPaintOptions{width:layout.width,height:layout.height,
                background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
            let leaf=|instructions:&[PaintInstruction]|instructions.iter().filter_map(|p|match p {
                PaintInstruction::GlyphRun(run) if run.glyphs.len()==1&&[65,66,67].contains(&run.glyphs[0].glyph_id)=>Some(run.clone()),_=>None,
            }).collect::<Vec<_>>();
            let actual=leaf(&scene.instructions);let expected=leaf(&expected.instructions);
            if actual!=expected {failures.push(format!("padding={padding} completeglyph mismatch: {actual:?} expected{expected:?}"));}
            assert_eq!(actual.len(),1);
            let run=&actual[0];let glyph=&run.glyphs[0];
            if !(glyph.x>=rect.x&&glyph.x+run.font_size/2.0<=rect.x+rect.width&&glyph.y-run.font_size*0.8+0.01>=rect.y&&glyph.y+run.font_size*0.2<=rect.y+rect.height+0.01) {
                failures.push(format!("padding={padding} glyphoutsidebox: {run:?} in{rect:?}"));
            }
            cells+=1;
        }
    }
    assert_eq!(cells,18);assert!(failures.is_empty(),"{} glyph/box failures: {failures:?}",failures.len());
}

#[test]
fn railroad_source_dialects_keep_complete_ordered_glyphs_and_config_metadata() {
    let cases=[
        ("railroad-beta\nr = terminal(\"A\");",E::Terminal("A".into())),
        ("railroad-beta\nr = nonterminal(\"B\");",E::NonTerminal("B".into())),
        ("railroad-beta\nr = special(\"C\");",E::Special("C".into())),
        ("railroad-ebnf-beta\nr = 'A';",E::Terminal("A".into())),
        ("railroad-ebnf-beta\nr = B;",E::NonTerminal("B".into())),
        ("railroad-ebnf-beta\nr = ?C?;",E::Special("C".into())),
        ("railroad-abnf-beta\nr = \"A\";",E::Terminal("A".into())),
        ("railroad-abnf-beta\nr = B;",E::NonTerminal("B".into())),
        ("railroad-peg-beta\nr <- \"A\";",E::Terminal("A".into())),
        ("railroad-peg-beta\nr <- B;",E::NonTerminal("B".into())),
        ("railroad-peg-beta\nr <- .;",E::Special(".".into())),
    ];
    let mut cells=0;
    for (body,definition) in cases {for padding in [0.0,0.5,3.5,4.0,10.0,65536.0] {
        let source=format!("---\nconfig:\n  railroad:\n    padding: {padding}\n---\n{body}");
        let input=mermaid_parser::parse_railroad(&source).unwrap();
        assert_eq!(input.rules.len(),1);assert_eq!(input.rules[0].definition,definition);
        assert_eq!(input.config.padding,padding);
        let layout=diagram_layout_hierarchy::layout_railroad(&input);let original=layout.clone();
        let options=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
        let scene=diagram_to_paint_railroad(&layout,&options);assert_eq!(layout,original);
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.padding".into(),padding.to_string()),("railroad.verticalSeparation".into(),"8".into()),
            ("railroad.horizontalSeparation".into(),"10".into()),
            ("railroad.arcRadius".into(),"10".into()),
            ("railroad.fontSize".into(),"14".into()),("railroad.fontFamily".into(),"monospace".into()),
            ("railroad.lineColor".into(),"#000000".into()),("railroad.strokeWidth".into(),"2".into()),
            ("railroad.markerFill".into(),"#000000".into()),("railroad.showMarkers".into(),"true".into()),
            ("railroad.markerRadius".into(),"5".into()),
        ])));
        let rule=&layout.rules[0];assert_eq!(rule.elements.len(),1);let element=&rule.elements[0];
        let height=24.0_f64.min(element.height);
        let children=vec![
            text_node_no_wrap("r",padding,rule.y+rule.height/2.0-14.0,92.0,28.0,
                options.label_font.clone(),Color{r:0,g:0,b:102,a:255}),
            text_node_no_wrap(&element.label,element.x+padding,element.y+(element.height-height)/2.0,
                element.width-2.0*padding,height,options.label_font.clone(),Color{r:0,g:0,b:0,a:255}),
        ];
        let expected=layout_to_paint(&PositionedNode{x:0.0,y:0.0,width:layout.width,height:layout.height,
            id:None,content:None,children,ext:HashMap::new()},&LayoutToPaintOptions{width:layout.width,height:layout.height,
            background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
        let runs=|instructions:&[PaintInstruction]|instructions.iter().filter_map(|p|match p {
            PaintInstruction::GlyphRun(run)=>Some(run.clone()),_=>None,
        }).collect::<Vec<_>>();
        let actual=runs(&scene.instructions);
        assert_eq!(actual,runs(&expected.instructions),"complete ordered source trace: {source}");
        assert_eq!(actual.len(),2);assert_eq!(actual[1].glyphs.len(),1);
        let run=&actual[1];let glyph=&run.glyphs[0];
        assert!(glyph.x>=element.x&&glyph.x+run.font_size/2.0<=element.x+element.width);
        assert!(glyph.y-run.font_size*0.8+0.01>=element.y&&glyph.y+run.font_size*0.2<=element.y+element.height+0.01);
        cells+=1;
    }}
    assert_eq!(cells,66);
}
