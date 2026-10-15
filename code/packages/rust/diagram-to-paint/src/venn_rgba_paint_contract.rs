// Sixteen independent complete scenes preserve rgba alpha multiplication,
// raw resolved-fill default stroke, explicit stroke precedence, glyph colors,
// debug vectors, full metadata and ordered font queries. Prior192 stay exact.
#[test]
fn venn_rgba_source_to_complete_layout_scene_and_ordered_font_queries() {
    // All geometry is binary-exact: available250, orbit40, radius62.5;
    // overlap radius22.5, box42.75, labeled <=2-row offset13.125.
    let palette=["#112233","#445566","#34d399","#f472b6","#a78bfa","#22d3ee","#fb7185","#a3e635"].map(str::to_string).to_vec();
    for family in ["Georgia","CourierNew"] {for authored in [false,true] {for debug in [false,true] {for double in [false,true] {
        let source=format!("---\nconfig:\n  venn:\n    width: 400\n    height: 300\n    padding: 25\n    useDebugLayout: {debug}\nthemeVariables:\n  fontFamily: {family}\n  venn1: '#112233'\n  venn2: '#445566'\n---\nvenn-beta\nset a[\"A\"]: 1\nset b[\"B\"]: 1\nunion a,b[\"U\"]: 0.5\ntext n[\"N\"]\n{}style a fill:rgba(255,107,107,0.8)\n{}style a stroke-width:3\nstyle a fill-opacity:0.5{}\nstyle n fill:#102030\nstyle n color:#00ff00\nstyle n stroke:#ffffff,stroke-width:0,fill-opacity:0",if double {"text m[\"M\"]\n"} else {""},if authored {"style a stroke:#123456\n"} else {""},if authored {",color:#ff0000"} else {""});
        let parsed=mermaid_parser::parse_venn(&source).unwrap();let config=diagram_ir::VennConfig {width:400.0,height:300.0,padding:25.0,use_debug_layout:debug,font_family:family.into(),theme_colors:palette.clone(),..Default::default()};assert_eq!(parsed.config,config);
        let special=diagram_ir::VennStyle {fill:Some("rgba(255,107,107,0.8)".into()),stroke:authored.then(||"#123456".into()),stroke_width:Some(3.0),fill_opacity:Some(0.5),text_color:authored.then(||"#ff0000".into())};
        let note_style=diagram_ir::VennStyle {fill:Some("#102030".into()),stroke:Some("#ffffff".into()),stroke_width:Some(0.0),text_color:Some("#00ff00".into()),fill_opacity:Some(0.0)};
        let mut labels=vec![diagram_ir::LayoutedVennLabel {text:"A".into(),x:276.25,y:150.0,style:special.clone()},
            diagram_ir::LayoutedVennLabel {text:"B".into(),x:123.75,y:150.0,style:Default::default()},diagram_ir::LayoutedVennLabel {text:"U".into(),x:200.0,y:150.0,style:Default::default()},diagram_ir::LayoutedVennLabel {text:"N".into(),x:200.0,y:170.0,style:note_style}];
        if double {labels.push(diagram_ir::LayoutedVennLabel {text:"M".into(),x:200.0,y:170.0,style:Default::default()});}
        let debug_circles=if debug {vec![diagram_ir::LayoutedVennDebugCircle {cx:200.0,cy:150.0,radius:22.5}]} else {vec![]};
        let debug_cells=if debug {if double {vec![diagram_ir::LayoutedVennDebugCell {x:178.625,y:141.75,width:21.375,height:42.75},diagram_ir::LayoutedVennDebugCell {x:200.0,y:141.75,width:21.375,height:42.75}]} else {vec![diagram_ir::LayoutedVennDebugCell {x:178.625,y:141.75,width:42.75,height:42.75}]}} else {vec![]};
        let layout=LayoutedVennDiagram {width:400.0,height:300.0,title:None,config,circles:vec![diagram_ir::LayoutedVennCircle {id:"a".into(),label:"A".into(),cx:240.0,cy:150.0,radius:62.5,style:special},diagram_ir::LayoutedVennCircle {id:"b".into(),label:"B".into(),cx:160.0,cy:150.0,radius:62.5,style:Default::default()}],labels,debug_circles,debug_cells};
        assert_eq!(diagram_layout_geometric::try_layout_venn(&parsed).unwrap(),layout,"complete source layout {source}");
        let mut instructions=vec![PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx:240.0,cy:150.0,rx:62.5,ry:62.5,fill:Some("rgba(255,107,107,0.4)".into()),stroke:Some(if authored {"#123456"} else {"rgba(255,107,107,0.8)"}.into()),stroke_width:Some(3.0),stroke_dash:None,stroke_dash_offset:None}),PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx:160.0,cy:150.0,rx:62.5,ry:62.5,fill:Some("rgba(68,85,102,0.1)".into()),stroke:Some("#445566".into()),stroke_width:Some(1.25),stroke_dash:None,stroke_dash_offset:None})];
        if debug {instructions.push(PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx:200.0,cy:150.0,rx:22.5,ry:22.5,fill:None,stroke:Some("#800080".into()),stroke_width:Some(0.375),stroke_dash:Some(vec![1.5,1.0]),stroke_dash_offset:None}));
            for (x,w) in if double {vec![(178.625,21.375),(200.0,21.375)]} else {vec![(178.625,42.75)]} {instructions.push(PaintInstruction::Rect(PaintRect {base:PaintBase::default(),x,y:141.75,width:w,height:42.75,fill:None,stroke:Some("#008080".into()),stroke_width:Some(0.25),corner_radius:None,stroke_dash:Some(vec![1.0,0.75]),stroke_dash_offset:None}));}}
        let glyph=|c:char,x,y,fill:&str|PaintInstruction::GlyphRun(PaintGlyphRun {base:PaintBase::default(),glyphs:vec![GlyphPosition {glyph_id:c as u32,x,y}],font_ref:"fake:test".into(),font_size:14.0,fill:Some(fill.into())});
        for (c,x,y,color) in [('A',272.75,150.0,if authored {"rgb(255, 0, 0)"} else {"rgb(15, 23, 42)"}),('B',120.25,150.0,"rgb(15, 23, 42)"),('U',196.5,150.0,"rgb(15, 23, 42)"),('N',196.5,170.0,"rgb(0, 255, 0)")] {instructions.push(glyph(c,x,y-12.0+11.2,color));}
        if double {instructions.push(glyph('M',196.5,170.0-12.0+11.2,"rgb(15, 23, 42)"));}
        let expected=PaintScene {width:400.0,height:300.0,background:"rgb(255,255,255)".into(),instructions,id:None,metadata:Some(venn_debug_metadata(400.0,300.0,25.0,debug,family,&palette))};
        let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,label_font:defaults.label_font,title_font:defaults.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        assert_eq!(diagram_to_paint_venn(&layout,&options),expected,"complete source scene {source}");assert_eq!(*resolver.0.borrow(),vec![FontQuery::named(family)],"ordered source font queries");
    }}}}
}
