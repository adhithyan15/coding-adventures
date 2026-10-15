//! Complete portable shape/style/glyph traces with source-retained configuration.
use super::*;

#[derive(Default)]
struct RecordingResolver(std::cell::RefCell<Vec<FontQuery>>);
impl FontResolver for RecordingResolver {
    type Handle=FakeHandle;
    fn resolve(&self,query:&FontQuery)->Result<FakeHandle,FontResolutionError> {
        self.0.borrow_mut().push(query.clone());Ok(FakeHandle)
    }
}

#[test]
fn railroad_style_retains_ordered_shapes_glyphs_and_complete_metadata() {
    let mut cells=0;
    for font_size in [0.5,14.0,18.25,64.0,65536.0] {for padding in [0.0,0.5,10.0,65536.0] {for markers in [false,true] {
        let source=format!("---\nconfig:\n  railroad:\n    fontSize: {font_size}\n    padding: {padding}\n    fontFamily: Courier New\n    terminalFill: '#dcfce7'\n    terminalStroke: '#166534'\n    terminalTextColor: '#14532d'\n    nonTerminalFill: '#dbeafe'\n    nonTerminalStroke: '#1d4ed8'\n    nonTerminalTextColor: '#172554'\n    lineColor: '#dc2626'\n    strokeWidth: 3\n    markerFill: '#7c2d12'\n    specialFill: '#fef3c7'\n    specialStroke: '#b45309'\n    ruleNameColor: '#581c87'\n    showMarkers: {markers}\n    markerRadius: 8\n---\nrailroad-beta\nr = sequence(terminal(\"A\"), nonterminal(\"B\"), special(\"C\"));");
        let input=mermaid_parser::parse_railroad(&source).unwrap();let config=&input.config;
        let layout=diagram_layout_hierarchy::try_layout_railroad(&input).unwrap();let original=layout.clone();
        let base=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);let resolver=RecordingResolver::default();
        let options=DiagramToPaintOptions{background:base.background,device_pixel_ratio:base.device_pixel_ratio,
            label_font:base.label_font,title_font:base.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        let scene=diagram_to_paint_railroad(&layout,&options);
        // Bold rule labels and normal leaves have two distinct font keys.
        // Repeated normal leaves share the cache; compare ordered real queries.
        let mut bold_query=FontQuery::named("Courier New");bold_query.weight=text_interfaces::FontWeight::BOLD;
        assert_eq!(*resolver.0.borrow(),vec![bold_query,FontQuery::named("Courier New")]);
        let kinds:Vec<_>=scene.instructions.iter().map(|i|match i{PaintInstruction::Ellipse(_)=>0,
            PaintInstruction::Path(_)=>1,PaintInstruction::Rect(_)=>2,PaintInstruction::GlyphRun(_)=>3,_=>4}).collect();
        let mut expected_kinds=if markers{vec![0,0]}else{vec![]};
        expected_kinds.extend([vec![1;4],vec![2;3],vec![3;4]].concat());assert_eq!(kinds,expected_kinds);
        assert_eq!(layout,original);let rule=&layout.rules[0];
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.compactMode".into(),"false".into()),
            ("railroad.padding".into(),padding.to_string()),("railroad.verticalSeparation".into(),"8".into()),
            ("railroad.horizontalSeparation".into(),"10".into()),("railroad.fontSize".into(),font_size.to_string()),
            ("railroad.arcRadius".into(),"10".into()),
            ("railroad.fontFamily".into(),"Courier New".into()),("railroad.lineColor".into(),"#dc2626".into()),
            ("railroad.strokeWidth".into(),"3".into()),("railroad.markerFill".into(),"#7c2d12".into()),
            ("railroad.showMarkers".into(),markers.to_string()),("railroad.markerRadius".into(),"8".into()),
        ])));
        let ellipses:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Ellipse(e)=i{Some(e)}else{None}).collect();
        assert_eq!(ellipses.len(),if markers{2}else{0});
        for (e,marker) in ellipses.iter().zip([&rule.start_marker,&rule.end_marker]) {
            assert_eq!((e.cx,e.cy,e.rx,e.ry),(marker.x,marker.y,8.0,8.0));assert_eq!(e.fill.as_deref(),Some("#7c2d12"));assert_eq!(e.stroke,None);
        }
        let paths:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Path(p)=i{Some(p.clone())}else{None}).collect();
        let expected:Vec<_>=rule.paths.iter().map(|p|line_path(&p.points,"#dc2626",3.0)).collect();assert_eq!(paths,expected);
        let rects:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Rect(r)=i{Some(r)}else{None}).collect();assert_eq!(rects.len(),3);
        for ((rect,e),(fill,stroke,radius,dash)) in rects.iter().zip(&rule.elements).zip([
            ("#dcfce7","#166534",14.0,None),("#dbeafe","#1d4ed8",2.0,None),("#fef3c7","#b45309",7.0,Some(vec![5.0,3.0]))]) {
            assert_eq!((rect.x,rect.y,rect.width,rect.height),(e.x,e.y,e.width,e.height));
            assert_eq!((rect.fill.as_deref(),rect.stroke.as_deref(),rect.stroke_width,rect.corner_radius,rect.stroke_dash.clone()),(Some(fill),Some(stroke),Some(3.0),Some(radius),dash));
        }
        let mut font=options.label_font.clone();font.family="Courier New".into();font.size=font_size;
        let name_height=28.0_f64.max(font_size*1.15);
        let mut name_font=font.clone();name_font.weight=700;
        let mut children=vec![text_node_no_wrap("r =",padding,rule.start_marker.y-name_height/2.0,3.0*font_size*0.61+20.0,name_height,name_font,Color{r:88,g:28,b:135,a:255})];
        for (e,color) in rule.elements.iter().zip([Color{r:20,g:83,b:45,a:255},Color{r:23,g:37,b:84,a:255},Color{r:23,g:37,b:84,a:255}]) {
            let height=24.0_f64.max(font_size*1.15).min(e.height);
            children.push(text_node_no_wrap(&e.label,e.x+padding,e.y+(e.height-height)/2.0,e.width-2.0*padding,height,font.clone(),color));
        }
        let expected=layout_to_paint(&PositionedNode{x:0.0,y:0.0,width:layout.width,height:layout.height,id:None,content:None,children,ext:HashMap::new()},&LayoutToPaintOptions{width:layout.width,height:layout.height,background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
        let glyphs=|v:&[PaintInstruction]|v.iter().filter_map(|i|if let PaintInstruction::GlyphRun(g)=i{Some(g.clone())}else{None}).collect::<Vec<_>>();
        let actual=glyphs(&scene.instructions);assert_eq!(actual,glyphs(&expected.instructions),"font={font_size},p={padding},markers={markers}");assert_eq!(actual.len(),4);
        for (g,e) in actual[1..].iter().zip(&rule.elements) {assert_eq!(g.glyphs.len(),e.label.chars().count());for p in &g.glyphs {assert!(p.x>=e.x&&p.x+g.font_size/2.0<=e.x+e.width+0.01);assert!(p.y-g.font_size*0.8+0.01>=e.y&&p.y+g.font_size*0.2<=e.y+e.height+0.01,"font={font_size}: {g:?} in {e:?}");}}
        assert_eq!(config.font_size,font_size);cells+=1;
    }}}
    assert_eq!(cells,40);
}

// The original forty explicit-style oracle cells above retain all prior entries plus the new default compact metadata key.
#[test]
fn railroad_theme_retains_ordered_shapes_glyphs_and_complete_metadata() {
    let mut cells=0;
    for font_size in [0.5,14.0,18.25,64.0,65536.0] {for padding in [0.0,0.5,10.0,65536.0] {for markers in [false,true] {
        let source=format!("---\nthemeVariables:\n  fontSize: '{font_size}px'\n  fontFamily: Courier New\n  secondBkg: '#dcfce7'\n  secondaryBorderColor: '#166534'\n  secondaryTextColor: '#14532d'\n  mainBkg: '#dbeafe'\n  primaryBorderColor: '#1d4ed8'\n  primaryTextColor: '#172554'\n  lineColor: '#dc2626'\n  tertiaryColor: '#fef3c7'\n  tertiaryBorderColor: '#b45309'\n  titleColor: '#581c87'\nconfig:\n  railroad:\n    padding: {padding}\n    strokeWidth: 3\n    markerFill: '#7c2d12'\n    showMarkers: {markers}\n    markerRadius: 8\n---\nrailroad-beta\nr = sequence(terminal(\"A\"), nonterminal(\"B\"), special(\"C\"));");
        let input=mermaid_parser::parse_railroad(&source).unwrap();let config=&input.config;
        let layout=diagram_layout_hierarchy::try_layout_railroad(&input).unwrap();let original=layout.clone();
        let base=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);let resolver=RecordingResolver::default();
        let options=DiagramToPaintOptions{background:base.background,device_pixel_ratio:base.device_pixel_ratio,
            label_font:base.label_font,title_font:base.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        let scene=diagram_to_paint_railroad(&layout,&options);
        // Bold rule labels and normal leaves have two distinct font keys.
        // Repeated normal leaves share the cache; compare ordered real queries.
        let mut bold_query=FontQuery::named("Courier New");bold_query.weight=text_interfaces::FontWeight::BOLD;
        assert_eq!(*resolver.0.borrow(),vec![bold_query,FontQuery::named("Courier New")]);
        let kinds:Vec<_>=scene.instructions.iter().map(|i|match i{PaintInstruction::Ellipse(_)=>0,
            PaintInstruction::Path(_)=>1,PaintInstruction::Rect(_)=>2,PaintInstruction::GlyphRun(_)=>3,_=>4}).collect();
        let mut expected_kinds=if markers{vec![0,0]}else{vec![]};
        expected_kinds.extend([vec![1;4],vec![2;3],vec![3;4]].concat());assert_eq!(kinds,expected_kinds);
        assert_eq!(layout,original);let rule=&layout.rules[0];
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.compactMode".into(),"false".into()),
            ("railroad.padding".into(),padding.to_string()),("railroad.verticalSeparation".into(),"8".into()),
            ("railroad.horizontalSeparation".into(),"10".into()),("railroad.fontSize".into(),font_size.to_string()),
            ("railroad.arcRadius".into(),"10".into()),
            ("railroad.fontFamily".into(),"Courier New".into()),("railroad.lineColor".into(),"#dc2626".into()),
            ("railroad.strokeWidth".into(),"3".into()),("railroad.markerFill".into(),"#7c2d12".into()),
            ("railroad.showMarkers".into(),markers.to_string()),("railroad.markerRadius".into(),"8".into()),
        ])));
        let ellipses:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Ellipse(e)=i{Some(e)}else{None}).collect();
        assert_eq!(ellipses.len(),if markers{2}else{0});
        for (e,marker) in ellipses.iter().zip([&rule.start_marker,&rule.end_marker]) {
            assert_eq!((e.cx,e.cy,e.rx,e.ry),(marker.x,marker.y,8.0,8.0));assert_eq!(e.fill.as_deref(),Some("#7c2d12"));assert_eq!(e.stroke,None);
        }
        let paths:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Path(p)=i{Some(p.clone())}else{None}).collect();
        let expected:Vec<_>=rule.paths.iter().map(|p|line_path(&p.points,"#dc2626",3.0)).collect();assert_eq!(paths,expected);
        let rects:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Rect(r)=i{Some(r)}else{None}).collect();assert_eq!(rects.len(),3);
        for ((rect,e),(fill,stroke,radius,dash)) in rects.iter().zip(&rule.elements).zip([
            ("#dcfce7","#166534",14.0,None),("#dbeafe","#1d4ed8",2.0,None),("#fef3c7","#b45309",7.0,Some(vec![5.0,3.0]))]) {
            assert_eq!((rect.x,rect.y,rect.width,rect.height),(e.x,e.y,e.width,e.height));
            assert_eq!((rect.fill.as_deref(),rect.stroke.as_deref(),rect.stroke_width,rect.corner_radius,rect.stroke_dash.clone()),(Some(fill),Some(stroke),Some(3.0),Some(radius),dash));
        }
        let mut font=options.label_font.clone();font.family="Courier New".into();font.size=font_size;
        let name_height=28.0_f64.max(font_size*1.15);
        let mut name_font=font.clone();name_font.weight=700;
        let mut children=vec![text_node_no_wrap("r =",padding,rule.start_marker.y-name_height/2.0,3.0*font_size*0.61+20.0,name_height,name_font,Color{r:88,g:28,b:135,a:255})];
        for (e,color) in rule.elements.iter().zip([Color{r:20,g:83,b:45,a:255},Color{r:23,g:37,b:84,a:255},Color{r:23,g:37,b:84,a:255}]) {
            let height=24.0_f64.max(font_size*1.15).min(e.height);
            children.push(text_node_no_wrap(&e.label,e.x+padding,e.y+(e.height-height)/2.0,e.width-2.0*padding,height,font.clone(),color));
        }
        let expected=layout_to_paint(&PositionedNode{x:0.0,y:0.0,width:layout.width,height:layout.height,id:None,content:None,children,ext:HashMap::new()},&LayoutToPaintOptions{width:layout.width,height:layout.height,background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
        let glyphs=|v:&[PaintInstruction]|v.iter().filter_map(|i|if let PaintInstruction::GlyphRun(g)=i{Some(g.clone())}else{None}).collect::<Vec<_>>();
        let actual=glyphs(&scene.instructions);assert_eq!(actual,glyphs(&expected.instructions),"font={font_size},p={padding},markers={markers}");assert_eq!(actual.len(),4);
        for (g,e) in actual[1..].iter().zip(&rule.elements) {assert_eq!(g.glyphs.len(),e.label.chars().count());for p in &g.glyphs {assert!(p.x>=e.x&&p.x+g.font_size/2.0<=e.x+e.width+0.01);assert!(p.y-g.font_size*0.8+0.01>=e.y&&p.y+g.font_size*0.2<=e.y+e.height+0.01,"font={font_size}: {g:?} in {e:?}");}}
        assert_eq!(config.font_size,font_size);cells+=1;
    }}}
    assert_eq!(cells,40);
}

// Both earlier forty-scene oracle blocks retain all prior entries plus the new default compact metadata key.
#[test]
fn railroad_sanitize_retains_ordered_shapes_glyphs_and_complete_metadata() {
    let mut cells=0;
    for font_size in [0.5,14.0,18.25,64.0,65536.0] {for padding in [0.0,0.5,10.0,65536.0] {for markers in [false,true] {
        let source=format!("---\nthemeVariables:\n  fontSize: '{font_size}px'\n  fontFamily: Courier New\n  secondBkg: '#dcfce7'\n  secondaryBorderColor: '#166534'\n  secondaryTextColor: '#14532d'\n  mainBkg: '#dbeafe'\n  primaryBorderColor: '#1d4ed8'\n  primaryTextColor: '#172554'\n  lineColor: '#dc2626'\n  tertiaryColor: '#fef3c7'\n  tertiaryBorderColor: '#b45309'\n  titleColor: '#581c87'\nconfig:\n  railroad:\n    fontFamily: Unsafe; color:red\n    terminalFill: url(x)\n    terminalStroke: url(x)\n    terminalTextColor: url(x)\n    nonTerminalFill: url(x)\n    nonTerminalStroke: url(x)\n    nonTerminalTextColor: url(x)\n    lineColor: url(x)\n    specialFill: url(x)\n    specialStroke: url(x)\n    ruleNameColor: url(x)\n    padding: {padding}\n    strokeWidth: 3\n    markerFill: 'url(x)'\n    showMarkers: {markers}\n    markerRadius: 8\n---\nrailroad-beta\nr = sequence(terminal(\"A\"), nonterminal(\"B\"), special(\"C\"));");
        let input=mermaid_parser::parse_railroad(&source).unwrap();let config=&input.config;
        let layout=diagram_layout_hierarchy::try_layout_railroad(&input).unwrap();let original=layout.clone();
        let base=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);let resolver=RecordingResolver::default();
        let options=DiagramToPaintOptions{background:base.background,device_pixel_ratio:base.device_pixel_ratio,
            label_font:base.label_font,title_font:base.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        let scene=diagram_to_paint_railroad(&layout,&options);
        // Bold rule labels and normal leaves have two distinct font keys.
        // Repeated normal leaves share the cache; compare ordered real queries.
        let mut bold_query=FontQuery::named("Courier New");bold_query.weight=text_interfaces::FontWeight::BOLD;
        assert_eq!(*resolver.0.borrow(),vec![bold_query,FontQuery::named("Courier New")]);
        let kinds:Vec<_>=scene.instructions.iter().map(|i|match i{PaintInstruction::Ellipse(_)=>0,
            PaintInstruction::Path(_)=>1,PaintInstruction::Rect(_)=>2,PaintInstruction::GlyphRun(_)=>3,_=>4}).collect();
        let mut expected_kinds=if markers{vec![0,0]}else{vec![]};
        expected_kinds.extend([vec![1;4],vec![2;3],vec![3;4]].concat());assert_eq!(kinds,expected_kinds);
        assert_eq!(layout,original);let rule=&layout.rules[0];
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.compactMode".into(),"false".into()),
            ("railroad.padding".into(),padding.to_string()),("railroad.verticalSeparation".into(),"8".into()),
            ("railroad.horizontalSeparation".into(),"10".into()),("railroad.fontSize".into(),font_size.to_string()),
            ("railroad.arcRadius".into(),"10".into()),
            ("railroad.fontFamily".into(),"Courier New".into()),("railroad.lineColor".into(),"#dc2626".into()),
            ("railroad.strokeWidth".into(),"3".into()),("railroad.markerFill".into(),"#dc2626".into()),
            ("railroad.showMarkers".into(),markers.to_string()),("railroad.markerRadius".into(),"8".into()),
        ])));
        let ellipses:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Ellipse(e)=i{Some(e)}else{None}).collect();
        assert_eq!(ellipses.len(),if markers{2}else{0});
        for (e,marker) in ellipses.iter().zip([&rule.start_marker,&rule.end_marker]) {
            assert_eq!((e.cx,e.cy,e.rx,e.ry),(marker.x,marker.y,8.0,8.0));assert_eq!(e.fill.as_deref(),Some("#dc2626"));assert_eq!(e.stroke,None);
        }
        let paths:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Path(p)=i{Some(p.clone())}else{None}).collect();
        let expected:Vec<_>=rule.paths.iter().map(|p|line_path(&p.points,"#dc2626",3.0)).collect();assert_eq!(paths,expected);
        let rects:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Rect(r)=i{Some(r)}else{None}).collect();assert_eq!(rects.len(),3);
        for ((rect,e),(fill,stroke,radius,dash)) in rects.iter().zip(&rule.elements).zip([
            ("#dcfce7","#166534",14.0,None),("#dbeafe","#1d4ed8",2.0,None),("#fef3c7","#b45309",7.0,Some(vec![5.0,3.0]))]) {
            assert_eq!((rect.x,rect.y,rect.width,rect.height),(e.x,e.y,e.width,e.height));
            assert_eq!((rect.fill.as_deref(),rect.stroke.as_deref(),rect.stroke_width,rect.corner_radius,rect.stroke_dash.clone()),(Some(fill),Some(stroke),Some(3.0),Some(radius),dash));
        }
        let mut font=options.label_font.clone();font.family="Courier New".into();font.size=font_size;
        let name_height=28.0_f64.max(font_size*1.15);
        let mut name_font=font.clone();name_font.weight=700;
        let mut children=vec![text_node_no_wrap("r =",padding,rule.start_marker.y-name_height/2.0,3.0*font_size*0.61+20.0,name_height,name_font,Color{r:88,g:28,b:135,a:255})];
        for (e,color) in rule.elements.iter().zip([Color{r:20,g:83,b:45,a:255},Color{r:23,g:37,b:84,a:255},Color{r:23,g:37,b:84,a:255}]) {
            let height=24.0_f64.max(font_size*1.15).min(e.height);
            children.push(text_node_no_wrap(&e.label,e.x+padding,e.y+(e.height-height)/2.0,e.width-2.0*padding,height,font.clone(),color));
        }
        let expected=layout_to_paint(&PositionedNode{x:0.0,y:0.0,width:layout.width,height:layout.height,id:None,content:None,children,ext:HashMap::new()},&LayoutToPaintOptions{width:layout.width,height:layout.height,background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
        let glyphs=|v:&[PaintInstruction]|v.iter().filter_map(|i|if let PaintInstruction::GlyphRun(g)=i{Some(g.clone())}else{None}).collect::<Vec<_>>();
        let actual=glyphs(&scene.instructions);assert_eq!(actual,glyphs(&expected.instructions),"font={font_size},p={padding},markers={markers}");assert_eq!(actual.len(),4);
        for (g,e) in actual[1..].iter().zip(&rule.elements) {assert_eq!(g.glyphs.len(),e.label.chars().count());for p in &g.glyphs {assert!(p.x>=e.x&&p.x+g.font_size/2.0<=e.x+e.width+0.01);assert!(p.y-g.font_size*0.8+0.01>=e.y&&p.y+g.font_size*0.2<=e.y+e.height+0.01,"font={font_size}: {g:?} in {e:?}");}}
        assert_eq!(config.font_size,font_size);cells+=1;
    }}}
    assert_eq!(cells,40);
}

// Forty compacttrue controls use the same independent complete geometry/style goldens.
#[test]
fn railroad_compact_true_retains_ordered_shapes_glyphs_and_complete_metadata() {
    let mut cells=0;
    for font_size in [0.5,14.0,18.25,64.0,65536.0] {for padding in [0.0,0.5,10.0,65536.0] {for markers in [false,true] {
        let source=format!("---\nconfig:\n  railroad:\n    compactMode: true\n    fontSize: {font_size}\n    padding: {padding}\n    fontFamily: Courier New\n    terminalFill: '#dcfce7'\n    terminalStroke: '#166534'\n    terminalTextColor: '#14532d'\n    nonTerminalFill: '#dbeafe'\n    nonTerminalStroke: '#1d4ed8'\n    nonTerminalTextColor: '#172554'\n    lineColor: '#dc2626'\n    strokeWidth: 3\n    markerFill: '#7c2d12'\n    specialFill: '#fef3c7'\n    specialStroke: '#b45309'\n    ruleNameColor: '#581c87'\n    showMarkers: {markers}\n    markerRadius: 8\n---\nrailroad-beta\nr = sequence(terminal(\"A\"), nonterminal(\"B\"), special(\"C\"));");
        let input=mermaid_parser::parse_railroad(&source).unwrap();let config=&input.config;assert!(config.compact_mode);
        let layout=diagram_layout_hierarchy::try_layout_railroad(&input).unwrap();let original=layout.clone();
        let base=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);let resolver=RecordingResolver::default();
        let options=DiagramToPaintOptions{background:base.background,device_pixel_ratio:base.device_pixel_ratio,
            label_font:base.label_font,title_font:base.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        let scene=diagram_to_paint_railroad(&layout,&options);
        // Bold rule labels and normal leaves have two distinct font keys.
        // Repeated normal leaves share the cache; compare ordered real queries.
        let mut bold_query=FontQuery::named("Courier New");bold_query.weight=text_interfaces::FontWeight::BOLD;
        assert_eq!(*resolver.0.borrow(),vec![bold_query,FontQuery::named("Courier New")]);
        let kinds:Vec<_>=scene.instructions.iter().map(|i|match i{PaintInstruction::Ellipse(_)=>0,
            PaintInstruction::Path(_)=>1,PaintInstruction::Rect(_)=>2,PaintInstruction::GlyphRun(_)=>3,_=>4}).collect();
        let mut expected_kinds=if markers{vec![0,0]}else{vec![]};
        expected_kinds.extend([vec![1;4],vec![2;3],vec![3;4]].concat());assert_eq!(kinds,expected_kinds);
        assert_eq!(layout,original);let rule=&layout.rules[0];
        assert_eq!(scene.metadata,Some(HashMap::from([
            ("railroad.compactMode".into(),"true".into()),
            ("railroad.padding".into(),padding.to_string()),("railroad.verticalSeparation".into(),"8".into()),
            ("railroad.horizontalSeparation".into(),"10".into()),("railroad.fontSize".into(),font_size.to_string()),
            ("railroad.arcRadius".into(),"10".into()),
            ("railroad.fontFamily".into(),"Courier New".into()),("railroad.lineColor".into(),"#dc2626".into()),
            ("railroad.strokeWidth".into(),"3".into()),("railroad.markerFill".into(),"#7c2d12".into()),
            ("railroad.showMarkers".into(),markers.to_string()),("railroad.markerRadius".into(),"8".into()),
        ])));
        let ellipses:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Ellipse(e)=i{Some(e)}else{None}).collect();
        assert_eq!(ellipses.len(),if markers{2}else{0});
        for (e,marker) in ellipses.iter().zip([&rule.start_marker,&rule.end_marker]) {
            assert_eq!((e.cx,e.cy,e.rx,e.ry),(marker.x,marker.y,8.0,8.0));assert_eq!(e.fill.as_deref(),Some("#7c2d12"));assert_eq!(e.stroke,None);
        }
        let paths:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Path(p)=i{Some(p.clone())}else{None}).collect();
        let expected:Vec<_>=rule.paths.iter().map(|p|line_path(&p.points,"#dc2626",3.0)).collect();assert_eq!(paths,expected);
        let rects:Vec<_>=scene.instructions.iter().filter_map(|i|if let PaintInstruction::Rect(r)=i{Some(r)}else{None}).collect();assert_eq!(rects.len(),3);
        for ((rect,e),(fill,stroke,radius,dash)) in rects.iter().zip(&rule.elements).zip([
            ("#dcfce7","#166534",14.0,None),("#dbeafe","#1d4ed8",2.0,None),("#fef3c7","#b45309",7.0,Some(vec![5.0,3.0]))]) {
            assert_eq!((rect.x,rect.y,rect.width,rect.height),(e.x,e.y,e.width,e.height));
            assert_eq!((rect.fill.as_deref(),rect.stroke.as_deref(),rect.stroke_width,rect.corner_radius,rect.stroke_dash.clone()),(Some(fill),Some(stroke),Some(3.0),Some(radius),dash));
        }
        let mut font=options.label_font.clone();font.family="Courier New".into();font.size=font_size;
        let name_height=28.0_f64.max(font_size*1.15);
        let mut name_font=font.clone();name_font.weight=700;
        let mut children=vec![text_node_no_wrap("r =",padding,rule.start_marker.y-name_height/2.0,3.0*font_size*0.61+20.0,name_height,name_font,Color{r:88,g:28,b:135,a:255})];
        for (e,color) in rule.elements.iter().zip([Color{r:20,g:83,b:45,a:255},Color{r:23,g:37,b:84,a:255},Color{r:23,g:37,b:84,a:255}]) {
            let height=24.0_f64.max(font_size*1.15).min(e.height);
            children.push(text_node_no_wrap(&e.label,e.x+padding,e.y+(e.height-height)/2.0,e.width-2.0*padding,height,font.clone(),color));
        }
        let expected=layout_to_paint(&PositionedNode{x:0.0,y:0.0,width:layout.width,height:layout.height,id:None,content:None,children,ext:HashMap::new()},&LayoutToPaintOptions{width:layout.width,height:layout.height,background:Color{r:0,g:0,b:0,a:0},device_pixel_ratio:1.0,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&FakeResolver});
        let glyphs=|v:&[PaintInstruction]|v.iter().filter_map(|i|if let PaintInstruction::GlyphRun(g)=i{Some(g.clone())}else{None}).collect::<Vec<_>>();
        let actual=glyphs(&scene.instructions);assert_eq!(actual,glyphs(&expected.instructions),"font={font_size},p={padding},markers={markers}");assert_eq!(actual.len(),4);
        for (g,e) in actual[1..].iter().zip(&rule.elements) {assert_eq!(g.glyphs.len(),e.label.chars().count());for p in &g.glyphs {assert!(p.x>=e.x&&p.x+g.font_size/2.0<=e.x+e.width+0.01);assert!(p.y-g.font_size*0.8+0.01>=e.y&&p.y+g.font_size*0.2<=e.y+e.height+0.01,"font={font_size}: {g:?} in {e:?}");}}
        assert_eq!(config.font_size,font_size);cells+=1;
    }}}
    assert_eq!(cells,40);
}
