// At width576/padding80 the declared fishbone arithmetic has exact integer
// coordinates. Keep the entire source IR, resolved layout, scene and resolver
// trace: no instruction-presence assertion can hide a lost theme component.
#[test]
fn ishikawa_sources_retain_complete_ir_layout_scene_and_ordered_queries() {
    for family in ["Georgia","CourierNew"] {for use_max_width in [false,true] {for custom_colors in [false,true] {for directive in [false,true] {
        let (line,fill,text,rgb)=if custom_colors {("#123456","#abcdef","#112233","17, 34, 51")} else {("#654321","#fedcba","#445566","68, 85, 102")};
        let prefix=if directive {
            let config=format!("\"diagramPadding\":80,\"useMaxWidth\":{use_max_width}");
            let theme=format!("\"fontFamily\":\"{family}\",\"lineColor\":\"{line}\",\"mainBkg\":\"{fill}\",\"textColor\":\"{text}\"");
            ["%%{init: {\"ishikawa\":{",&config,"},\"themeVariables\":{",&theme,"}}}%%\n"].concat()
        } else {format!("---\nconfig:\n  ishikawa:\n    diagramPadding: 80\n    useMaxWidth: {use_max_width}\nthemeVariables:\n  fontFamily: {family}\n  lineColor: '{line}'\n  mainBkg: '{fill}'\n  textColor: '{text}'\n---\n")};
        let source=[prefix.as_str(),"ishikawa-beta\nE\n  A\n"].concat();
        let config=diagram_ir::IshikawaConfig {diagram_padding:80.0,use_max_width,font_family:family.into(),line_color:line.into(),fill_color:fill.into(),text_color:text.into()};
        let expected_ir=diagram_ir::IshikawaDiagram {effect:"E".into(),causes:vec![diagram_ir::IshikawaCause {id:"cause-1".into(),label:"A".into(),parent_id:None,depth:1}],config:config.clone()};
        let parsed=mermaid_parser::parse_ishikawa(&source).unwrap();assert_eq!(parsed,expected_ir,"whole IR source={source}");
        let expected_layout=LayoutedIshikawaDiagram {width:576.0,height:360.0,effect:"E".into(),config,
            effect_x:346.0,effect_y:151.0,effect_width:150.0,effect_height:58.0,spine_from:Point{x:96.0,y:180.0},spine_to:Point{x:346.0,y:180.0},
            bones:vec![diagram_ir::LayoutedIshikawaBone {from:Point{x:161.0,y:108.0},to:Point{x:221.0,y:180.0},label:"A".into(),label_position:Point{x:191.0,y:102.0},label_width:72.0,label_height:28.0,depth:1}]};
        let layout=diagram_layout_geometric::layout_ishikawa(&parsed,576.0);assert_eq!(layout,expected_layout,"whole layout source={source}");
        let path=|from:Point,to:Point,width|PaintInstruction::Path(PaintPath {base:PaintBase::default(),commands:vec![PathCommand::MoveTo{x:from.x,y:from.y},PathCommand::LineTo{x:to.x,y:to.y}],fill:Some("none".into()),fill_rule:None,stroke:Some(line.into()),stroke_width:Some(width),stroke_cap:Some(StrokeCap::Round),stroke_join:Some(StrokeJoin::Round),stroke_dash:None,stroke_dash_offset:None});
        let glyph=|c:char,x,y,size|PaintInstruction::GlyphRun(PaintGlyphRun {base:PaintBase::default(),glyphs:vec![GlyphPosition{glyph_id:c as u32,x,y}],font_ref:"fake:test".into(),font_size:size,fill:Some(format!("rgb({rgb})"))});
        let expected=PaintScene {width:576.0,height:360.0,background:"rgb(255,255,255)".into(),id:None,
            metadata:Some(HashMap::from([("ishikawa.diagramPadding".into(),"80".into()),("ishikawa.useMaxWidth".into(),use_max_width.to_string()),("ishikawa.fontFamily".into(),family.into()),("ishikawa.lineColor".into(),line.into()),("ishikawa.fillColor".into(),fill.into()),("ishikawa.textColor".into(),text.into())])),
            instructions:vec![path(Point{x:96.0,y:180.0},Point{x:346.0,y:180.0},3.0),path(Point{x:161.0,y:108.0},Point{x:221.0,y:180.0},2.5),
                PaintInstruction::Rect(PaintRect {base:PaintBase::default(),x:346.0,y:151.0,width:150.0,height:58.0,fill:Some(fill.into()),stroke:Some(line.into()),stroke_width:Some(2.0),corner_radius:Some(8.0),stroke_dash:None,stroke_dash_offset:None}),
                glyph('A',187.0,104.8,16.0),glyph('E',416.0,175.0,20.0)]};
        let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
        let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,
            label_font:FontSpec {family:"Ignored Caller".into(),size:16.0,weight:600,italic:true,stretch:FontStretch::Condensed,..defaults.label_font},
            title_font:FontSpec {family:"Ignored Title".into(),size:20.0,weight:800,italic:true,stretch:FontStretch::Expanded,..defaults.title_font},shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        assert_eq!(diagram_to_paint_ishikawa(&layout,&options),expected,"whole scene source={source}");
        assert_eq!(*resolver.0.borrow(),vec![FontQuery::named(family).with_weight(text_interfaces::FontWeight(600)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Condensed),FontQuery::named(family).with_weight(text_interfaces::FontWeight(800)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Expanded)],"complete ordered query trace");
    }}}}
}
