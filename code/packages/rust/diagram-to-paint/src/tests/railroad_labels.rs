//! Display text is an authored golden; semantic labels remain parser-owned.
use super::*;

#[derive(Default)]
struct Queries(std::cell::RefCell<Vec<FontQuery>>);
impl FontResolver for Queries {
    type Handle=FakeHandle;
    fn resolve(&self,q:&FontQuery)->Result<FakeHandle,FontResolutionError> {
        self.0.borrow_mut().push(q.clone());Ok(FakeHandle)
    }
}

#[test]
fn railroad_labels_four_dialects_keep_exact_fonts_glyphs_ports_and_metadata() {
    use diagram_ir::RailroadExpression as E;
    let mut cells=0;
    for (body,semantic,display) in [
        ("railroad-beta\nlong_rule = special(\"C\");",E::Special("C".into()),"? C ?"),
        ("railroad-ebnf-beta\nlong_rule = ?C?;",E::Special("C".into()),"? C ?"),
        ("railroad-abnf-beta\nlong_rule = B;",E::NonTerminal("B".into()),"B"),
        ("railroad-peg-beta\nlong_rule <- .;",E::Special(".".into()),"? . ?"),
    ] {for theme in [false,true] {for font_size in [0.5,64.0] {
        for compact in [false,true] {for markers in [false,true] {for radius in [0.0,100.0,65536.0] {
            let typography=if theme {format!("themeVariables:\n  fontSize: '{font_size}px'\n  fontFamily: Courier New\n")}
                else {String::new()};
            let font=if theme {String::new()}else{format!("    fontSize: {font_size}\n    fontFamily: Courier New\n")};
            let source=format!("---\n{typography}config:\n  railroad:\n{font}    padding: 10\n    compactMode: {compact}\n    showMarkers: {markers}\n    markerRadius: {radius}\n---\n{body}");
            let parsed=mermaid_parser::parse_railroad(&source).unwrap();let parsed_before=parsed.clone();
            assert_eq!(parsed.rules[0].definition,semantic);
            let layout=diagram_layout_hierarchy::try_layout_railroad(&parsed).unwrap();let original=layout.clone();
            assert_eq!(parsed,parsed_before);let rule=&layout.rules[0];
            let name_width=11.0*font_size*0.61+20.0;
            let start=10.0+name_width;let origin=start+20.0;
            let width=font_size*3.15+20.0;let height=font_size*1.15+20.0;
            let center=36.0+height/2.0;let end=origin+width+10.0;
            assert_eq!((rule.start_marker.x,rule.start_marker.y,rule.end_marker.x,rule.end_marker.y),(start,center,end,center));
            assert_eq!(rule.elements.len(),1);let element=&rule.elements[0];
            assert_eq!((element.label.as_str(),element.x,element.y,element.width,element.height),(display,origin,36.0,width,height));
            assert_eq!(rule.paths.iter().map(|p|p.points.iter().map(|p|(p.x,p.y)).collect::<Vec<_>>()).collect::<Vec<_>>(),
                vec![vec![(start+radius,center),(origin,center)],vec![(origin+width,center),(end-radius,center)]]);
            let resolver=Queries::default();let base=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
            let options=DiagramToPaintOptions{background:base.background,device_pixel_ratio:base.device_pixel_ratio,
                label_font:base.label_font.clone(),title_font:base.title_font.clone(),shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
            let scene=diagram_to_paint_railroad(&layout,&options);assert_eq!(layout,original);
            let mut bold_query=FontQuery::named("Courier New");bold_query.weight=text_interfaces::FontWeight::BOLD;
            assert_eq!(*resolver.0.borrow(),vec![bold_query,FontQuery::named("Courier New")]);
            let ellipses=scene.instructions.iter().filter_map(|p|if let PaintInstruction::Ellipse(e)=p{Some(e)}else{None}).collect::<Vec<_>>();
            assert_eq!(ellipses.len(),if markers{2}else{0});
            for (ellipse,x) in ellipses.iter().zip([start,end]) {
                assert_eq!((ellipse.cx,ellipse.cy,ellipse.rx,ellipse.ry),(x,center,radius,radius));
                assert_eq!((ellipse.fill.as_deref(),ellipse.stroke.as_deref()),(Some("#000000"),None));
            }
            let mut normal=base.label_font.clone();normal.family="Courier New".into();normal.size=font_size;
            let mut bold=normal.clone();bold.weight=700;
            let name_height=28.0_f64.max(font_size*1.15);let leaf_height=24.0_f64.max(font_size*1.15).min(height);
            let children=vec![text_node_no_wrap("long_rule =",10.0,center-name_height/2.0,name_width,name_height,bold,Color{r:0,g:0,b:102,a:255}),
                text_node_no_wrap(display,origin+10.0,36.0+(height-leaf_height)/2.0,width-20.0,leaf_height,normal,Color{r:0,g:0,b:0,a:255})];
            let expected=layout_to_paint(&PositionedNode{x:0.0,y:0.0,width:layout.width,height:layout.height,id:None,content:None,children,ext:HashMap::new()},
                &LayoutToPaintOptions{width:layout.width,height:layout.height,background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
            let glyphs=|instructions:&[PaintInstruction]|instructions.iter().filter_map(|p|if let PaintInstruction::GlyphRun(g)=p{Some(g.clone())}else{None}).collect::<Vec<_>>();
            let actual=glyphs(&scene.instructions);assert_eq!(actual,glyphs(&expected.instructions),"{source}");
            assert_eq!(actual.len(),2);
            for (run,text) in actual.iter().zip(["long_rule =",display]) {
                assert_eq!(run.glyphs.iter().map(|g|g.glyph_id).collect::<Vec<_>>(),text.chars().map(|c|c as u32).collect::<Vec<_>>());
            }
            assert_eq!(scene.metadata,Some(HashMap::from([
                ("railroad.compactMode".into(),compact.to_string()),("railroad.padding".into(),"10".into()),
                ("railroad.verticalSeparation".into(),"8".into()),("railroad.horizontalSeparation".into(),"10".into()),
                ("railroad.arcRadius".into(),"10".into()),("railroad.fontSize".into(),font_size.to_string()),
                ("railroad.fontFamily".into(),"Courier New".into()),("railroad.lineColor".into(),"#000000".into()),
                ("railroad.strokeWidth".into(),"2".into()),("railroad.markerFill".into(),"#000000".into()),
                ("railroad.showMarkers".into(),markers.to_string()),("railroad.markerRadius".into(),radius.to_string()),
            ])));
            // Large marker radii may reverse these two connectors. These exact
            // ports establish bounded behavior, not general visual clearance.
            cells+=1;
        }}}}}
    }
    assert_eq!(cells,192);
}
