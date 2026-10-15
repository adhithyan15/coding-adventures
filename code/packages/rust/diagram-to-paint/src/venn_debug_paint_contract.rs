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
            let expected=match layout.width {400.0=>1.25,800.0=>2.5,_=>panic!("legacy adapter admits only its declared fixture widths")};
            assert_eq!(ellipse.stroke_width,Some(expected),"new scaled default width before reversal");
            ellipse.stroke_width=Some(2.0);
        }
        if circle.style.fill_opacity.is_none() {ellipse.fill=Some(format!("rgba({},{},{},0.38)",rgb[0],rgb[1],rgb[2]));}
    }
    scene
}

struct VennDebugRecordingResolver(std::cell::RefCell<Vec<FontQuery>>);
impl FontResolver for VennDebugRecordingResolver {
    type Handle=FakeHandle;
    fn resolve(&self,q:&FontQuery)->Result<FakeHandle,FontResolutionError> {self.0.borrow_mut().push(q.clone());Ok(FakeHandle)}
}
fn venn_debug_metadata(width:f64,height:f64,padding:f64,debug:bool,family:&str,palette:&[String])->HashMap<String,String> {
    let mut m=HashMap::from([("venn.width".into(),width.to_string()),("venn.height".into(),height.to_string()),
        ("venn.padding".into(),padding.to_string()),("venn.useMaxWidth".into(),"true".into()),("venn.useDebugLayout".into(),debug.to_string()),
        ("venn.fontFamily".into(),family.into()),("venn.titleTextColor".into(),"#0f172a".into()),("venn.setTextColor".into(),"#0f172a".into())]);
    for (i,color) in palette.iter().enumerate() {m.insert(format!("venn.themeColor{}",i+1),color.clone());} m
}
fn venn_debug_base_ellipse(cx:f64,fill:&str,stroke:&str,width:f64)->PaintInstruction {
    PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx,cy:100.0,rx:20.0,ry:20.0,fill:Some(fill.into()),stroke:Some(stroke.into()),stroke_width:Some(width),stroke_dash:None,stroke_dash_offset:None})
}
#[test]
fn venn_debug_typed_complete_scenes_preserve_independent_defaults_overrides_and_flag_transport() {
    // 56 complete scenes: empty/two-color palettes, two widths, both flag
    // values, seven independent authored/default choices. Explicit vectors
    // paint even when the config flag is false; layout owns that flag gate.
    for empty in [false,true] {for (width,default_width,debug_width,debug_dash,cell_width,cell_dash) in [
        (800.0,2.5,0.75,vec![3.0,2.0],0.5,vec![2.0,1.5]),
        (1600.0,5.0,1.5,vec![6.0,4.0],1.0,vec![4.0,3.0])] {
        for debug in [false,true] {for mode in 0..7 {
            let palette:Vec<String>=if empty {vec![]} else {vec!["#112233".into(),"#445566".into()]};
            let (base_rgb,base_stroke,second_rgb,second_stroke)=if empty {("96,165,250","#60a5fa","96,165,250","#60a5fa")} else {("17,34,51","#112233","68,85,102","#445566")};
            let mut style=diagram_ir::VennStyle::default();let(mut rgb,mut stroke,mut alpha,mut stroke_width)=(base_rgb,base_stroke,"0.1",default_width);
            match mode {
                1=>{style.fill=Some("#abcdef".into());rgb="171,205,239";stroke="#abcdef";},
                2=>{style.fill_opacity=Some(0.0);alpha="0";},
                3=>{style.stroke=Some("#001122".into());stroke="#001122";},
                4=>{style.stroke_width=Some(0.0);stroke_width=0.0;},
                5|6=>{style.fill=Some("#abcdef".into());style.stroke=Some("#001122".into());rgb="171,205,239";stroke="#001122";
                    if mode==5 {style.fill_opacity=Some(0.7);style.stroke_width=Some(3.0);alpha="0.7";stroke_width=3.0;}
                    else {style.fill_opacity=Some(0.0);style.stroke_width=Some(0.0);alpha="0";stroke_width=0.0;}},_=>{}
            }
            let config=diagram_ir::VennConfig {width,height:300.0,use_debug_layout:debug,theme_colors:palette.clone(),..Default::default()};
            let layout=LayoutedVennDiagram {width,height:300.0,title:None,config,circles:vec![
                diagram_ir::LayoutedVennCircle {id:"A".into(),label:"A".into(),cx:100.0,cy:100.0,radius:20.0,style},
                diagram_ir::LayoutedVennCircle {id:"B".into(),label:"B".into(),cx:200.0,cy:100.0,radius:20.0,style:Default::default()}],labels:vec![],
                debug_circles:vec![diagram_ir::LayoutedVennDebugCircle {cx:10.0,cy:20.0,radius:8.0}],
                debug_cells:vec![diagram_ir::LayoutedVennDebugCell {x:30.0,y:40.0,width:50.0,height:60.0}]};
            let expected=PaintScene {width,height:300.0,background:"rgb(255,255,255)".into(),id:None,
                metadata:Some(venn_debug_metadata(width,300.0,8.0,debug,"sans-serif",&palette)),instructions:vec![
                    venn_debug_base_ellipse(100.0,&format!("rgba({rgb},{alpha})"),stroke,stroke_width),
                    venn_debug_base_ellipse(200.0,&format!("rgba({second_rgb},0.1)"),second_stroke,default_width),
                    PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx:10.0,cy:20.0,rx:8.0,ry:8.0,fill:None,stroke:Some("#800080".into()),stroke_width:Some(debug_width),stroke_dash:Some(debug_dash.clone()),stroke_dash_offset:None}),
                    PaintInstruction::Rect(PaintRect {base:PaintBase::default(),x:30.0,y:40.0,width:50.0,height:60.0,fill:None,stroke:Some("#008080".into()),stroke_width:Some(cell_width),corner_radius:None,stroke_dash:Some(cell_dash.clone()),stroke_dash_offset:None})]};
            let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
            let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,label_font:defaults.label_font,title_font:defaults.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
            assert_eq!(diagram_to_paint_venn(&layout,&options),expected,"palette-empty={empty} width={width} flag={debug} mode={mode}");
            assert!(resolver.0.borrow().is_empty(),"complete no-label query trace");
        }}
    }}
}

#[test]
fn venn_debug_source_to_complete_layout_scene_and_ordered_font_queries() {
    // All geometry is binary-exact: available250, orbit40, radius62.5;
    // overlap radius22.5, box42.75, labeled <=2-row offset13.125.
    let palette=["#112233","#445566","#34d399","#f472b6","#a78bfa","#22d3ee","#fb7185","#a3e635"].map(str::to_string).to_vec();
    for family in ["Georgia","CourierNew"] {for authored in [false,true] {for debug in [false,true] {for double in [false,true] {
        let source=format!("---\nconfig:\n  venn:\n    width: 400\n    height: 300\n    padding: 25\n    useDebugLayout: {debug}\nthemeVariables:\n  fontFamily: {family}\n  venn1: '#112233'\n  venn2: '#445566'\n---\nvenn-beta\nset a[\"A\"]: 1\nset b[\"B\"]: 1\nunion a,b[\"U\"]: 0.5\ntext n[\"N\"]\n{}style a fill:#abcdef,stroke:#123456,stroke-width:3,fill-opacity:0.5{}",if double {"text m[\"M\"]\n"} else {""},if authored {",color:#ff0000"} else {""});
        let parsed=mermaid_parser::parse_venn(&source).unwrap();let config=diagram_ir::VennConfig {width:400.0,height:300.0,padding:25.0,use_debug_layout:debug,font_family:family.into(),theme_colors:palette.clone(),..Default::default()};assert_eq!(parsed.config,config);
        let special=diagram_ir::VennStyle {fill:Some("#abcdef".into()),stroke:Some("#123456".into()),stroke_width:Some(3.0),fill_opacity:Some(0.5),text_color:authored.then(||"#ff0000".into())};
        let mut labels=vec![diagram_ir::LayoutedVennLabel {text:"A".into(),x:276.25,y:150.0,style:special.clone()},
            diagram_ir::LayoutedVennLabel {text:"B".into(),x:123.75,y:150.0,style:Default::default()},diagram_ir::LayoutedVennLabel {text:"U".into(),x:200.0,y:150.0,style:Default::default()},diagram_ir::LayoutedVennLabel {text:"N".into(),x:200.0,y:170.0,style:Default::default()}];
        if double {labels.push(diagram_ir::LayoutedVennLabel {text:"M".into(),x:200.0,y:170.0,style:Default::default()});}
        let debug_circles=if debug {vec![diagram_ir::LayoutedVennDebugCircle {cx:200.0,cy:150.0,radius:22.5}]} else {vec![]};
        let debug_cells=if debug {if double {vec![diagram_ir::LayoutedVennDebugCell {x:178.625,y:141.75,width:21.375,height:42.75},diagram_ir::LayoutedVennDebugCell {x:200.0,y:141.75,width:21.375,height:42.75}]} else {vec![diagram_ir::LayoutedVennDebugCell {x:178.625,y:141.75,width:42.75,height:42.75}]}} else {vec![]};
        let layout=LayoutedVennDiagram {width:400.0,height:300.0,title:None,config,circles:vec![diagram_ir::LayoutedVennCircle {id:"a".into(),label:"A".into(),cx:240.0,cy:150.0,radius:62.5,style:special},diagram_ir::LayoutedVennCircle {id:"b".into(),label:"B".into(),cx:160.0,cy:150.0,radius:62.5,style:Default::default()}],labels,debug_circles,debug_cells};
        assert_eq!(diagram_layout_geometric::try_layout_venn(&parsed).unwrap(),layout,"complete source layout {source}");
        let mut instructions=vec![PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx:240.0,cy:150.0,rx:62.5,ry:62.5,fill:Some("rgba(171,205,239,0.5)".into()),stroke:Some("#123456".into()),stroke_width:Some(3.0),stroke_dash:None,stroke_dash_offset:None}),PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx:160.0,cy:150.0,rx:62.5,ry:62.5,fill:Some("rgba(68,85,102,0.1)".into()),stroke:Some("#445566".into()),stroke_width:Some(1.25),stroke_dash:None,stroke_dash_offset:None})];
        if debug {instructions.push(PaintInstruction::Ellipse(PaintEllipse {base:PaintBase::default(),cx:200.0,cy:150.0,rx:22.5,ry:22.5,fill:None,stroke:Some("#800080".into()),stroke_width:Some(0.375),stroke_dash:Some(vec![1.5,1.0]),stroke_dash_offset:None}));
            for (x,w) in if double {vec![(178.625,21.375),(200.0,21.375)]} else {vec![(178.625,42.75)]} {instructions.push(PaintInstruction::Rect(PaintRect {base:PaintBase::default(),x,y:141.75,width:w,height:42.75,fill:None,stroke:Some("#008080".into()),stroke_width:Some(0.25),corner_radius:None,stroke_dash:Some(vec![1.0,0.75]),stroke_dash_offset:None}));}}
        let glyph=|c:char,x,y,fill:&str|PaintInstruction::GlyphRun(PaintGlyphRun {base:PaintBase::default(),glyphs:vec![GlyphPosition {glyph_id:c as u32,x,y}],font_ref:"fake:test".into(),font_size:14.0,fill:Some(fill.into())});
        for (c,x,y,color) in [('A',272.75,150.0,if authored {"rgb(255, 0, 0)"} else {"rgb(15, 23, 42)"}),('B',120.25,150.0,"rgb(15, 23, 42)"),('U',196.5,150.0,"rgb(15, 23, 42)"),('N',196.5,170.0,"rgb(15, 23, 42)")] {instructions.push(glyph(c,x,y-12.0+11.2,color));}
        if double {instructions.push(glyph('M',196.5,170.0-12.0+11.2,"rgb(15, 23, 42)"));}
        let expected=PaintScene {width:400.0,height:300.0,background:"rgb(255,255,255)".into(),instructions,id:None,metadata:Some(venn_debug_metadata(400.0,300.0,25.0,debug,family,&palette))};
        let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,label_font:defaults.label_font,title_font:defaults.title_font,shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        assert_eq!(diagram_to_paint_venn(&layout,&options),expected,"complete source scene {source}");assert_eq!(*resolver.0.borrow(),vec![FontQuery::named(family)],"ordered source font queries");
    }}}}
}
