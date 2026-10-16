// Five companions exercise the literal default scene, not a second custom
// palette. The padding20 geometry table uses exact IEEE754 literals; the
// 64.80000000000001 ordinate is the declared two-stage fishbone arithmetic.
#[test]
fn ishikawa_default_and_rejected_settings_keep_complete_scene_queries() {
    for prefix in [
        "",
        "%%{init: {\"ishikawa\":{\"diagramPadding\":\"80\"junk,\"useMaxWidth\":\"true\"junk},\"themeVariables\":{\"fontFamily\":\"Bad;Font\",\"lineColor\":\"red'url(evil)\",\"mainBkg\":\"blue'evil\",\"textColor\":\"green'evil\"}}}%%\n",
        "---\nconfig:\n  ishikawa:\n    diagramPadding: '80'junk\n    useMaxWidth: 'true'junk\nthemeVariables:\n  fontFamily: 'Bad;Font'\n  lineColor: \"red'url(evil)\"\n  mainBkg: \"blue'evil\"\n  textColor: \"green'evil\"\n---\n",
        "---\nnotes: |\n  %%{init: {\"ishikawa\":{\"diagramPadding\":80,\"useMaxWidth\":true},\"themeVariables\":{\"fontFamily\":\"Georgia\",\"lineColor\":\"red\",\"mainBkg\":\"blue\",\"textColor\":\"green\"}}}%%\n---\n",
        "---\nconfig:\n  ishikawa:\n    diagramPadding: 80\n    useMaxWidth: true\nthemeVariables:\n  fontFamily: Georgia\n  lineColor: red\n  mainBkg: blue\n  textColor: green\n---\n%%{init: {\"ishikawa\":{},\"themeVariables\":{}}}%%\n",
    ] {
        let source=[prefix,"ishikawa-beta\nE\n  A\n"].concat();
        let config=diagram_ir::IshikawaConfig {font_size:16.0,look:diagram_ir::IshikawaLook::Classic,hand_drawn_seed:0,diagram_padding:20.0,use_max_width:false,font_family:"sans-serif".into(),line_color:"#333333".into(),fill_color:"#ffffff".into(),text_color:"#333333".into()};
        let expected_ir=diagram_ir::IshikawaDiagram {effect:"E".into(),causes:vec![diagram_ir::IshikawaCause {id:"cause-1".into(),label:"A".into(),parent_id:None,depth:1}],config:config.clone()};
        let parsed=mermaid_parser::parse_ishikawa(&source).unwrap();assert_eq!(parsed,expected_ir,"whole default IR source={source}");
        let expected_layout=LayoutedIshikawaDiagram {width:576.0,height:360.0,effect:"E".into(),config,
            effect_x:406.0,effect_y:151.0,effect_width:150.0,effect_height:58.0,spine_from:Point{x:36.0,y:180.0},spine_to:Point{x:406.0,y:180.0},
            bones:vec![diagram_ir::LayoutedIshikawaBone {from:Point{x:132.2,y:64.80000000000001},to:Point{x:221.0,y:180.0},label:"A".into(),label_position:Point{x:176.6,y:80.4},label_width:72.0,label_height:28.0,depth:1}]};
        let layout=diagram_layout_geometric::layout_ishikawa(&parsed,576.0);assert_eq!(layout,expected_layout,"whole default layout source={source}");
        let path=|from:Point,to:Point,width|PaintInstruction::Path(PaintPath {base:PaintBase::default(),commands:vec![PathCommand::MoveTo{x:from.x,y:from.y},PathCommand::LineTo{x:to.x,y:to.y}],fill:Some("none".into()),fill_rule:None,stroke:Some("#333333".into()),stroke_width:Some(width),stroke_cap:Some(StrokeCap::Round),stroke_join:Some(StrokeJoin::Round),stroke_dash:None,stroke_dash_offset:None});
        let glyph=|c:char,x,y,size|PaintInstruction::GlyphRun(PaintGlyphRun {base:PaintBase::default(),glyphs:vec![GlyphPosition{glyph_id:c as u32,x,y}],font_ref:"fake:test".into(),font_size:size,fill:Some("rgb(51, 51, 51)".into())});
        let expected=PaintScene {width:576.0,height:360.0,background:"rgb(255,255,255)".into(),id:None,
            metadata:Some(HashMap::from([("ishikawa.diagramPadding".into(),"20".into()),("ishikawa.useMaxWidth".into(),"false".into()),("ishikawa.fontFamily".into(),"sans-serif".into()),("ishikawa.lineColor".into(),"#333333".into()),("ishikawa.fillColor".into(),"#ffffff".into()),("ishikawa.textColor".into(),"#333333".into())])),
            instructions:vec![path(Point{x:36.0,y:180.0},Point{x:406.0,y:180.0},3.0),path(Point{x:132.2,y:64.80000000000001},Point{x:221.0,y:180.0},2.5),
                PaintInstruction::Rect(PaintRect {base:PaintBase::default(),x:140.6,y:66.4,width:72.0,height:28.0,fill:Some("#ffffff".into()),stroke:Some("#333333".into()),stroke_width:Some(1.0),corner_radius:Some(4.0),stroke_dash:None,stroke_dash_offset:None}),
                PaintInstruction::Rect(PaintRect {base:PaintBase::default(),x:406.0,y:151.0,width:150.0,height:58.0,fill:Some("#ffffff".into()),stroke:Some("#333333".into()),stroke_width:Some(2.0),corner_radius:Some(8.0),stroke_dash:None,stroke_dash_offset:None}),
                glyph('A',172.6,81.2,16.0),glyph('E',476.0,175.0,20.0)]};
        let resolver=VennDebugRecordingResolver(std::cell::RefCell::new(vec![]));let defaults=make_opts(&FakeShaper,&FakeMetrics,&FakeResolver);
        let options=DiagramToPaintOptions {background:defaults.background,device_pixel_ratio:defaults.device_pixel_ratio,
            label_font:FontSpec {family:"Ignored Caller".into(),size:16.0,weight:600,italic:true,stretch:FontStretch::Condensed,..defaults.label_font},
            title_font:FontSpec {family:"Ignored Title".into(),size:20.0,weight:800,italic:true,stretch:FontStretch::Expanded,..defaults.title_font},shaper:&FakeShaper,metrics:&FakeMetrics,resolver:&resolver};
        assert_eq!(diagram_to_paint_ishikawa(&layout,&options),expected,"whole default scene source={source}");
        assert_eq!(*resolver.0.borrow(),vec![FontQuery::named("sans-serif").with_weight(text_interfaces::FontWeight(600)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Condensed),FontQuery::named("sans-serif").with_weight(text_interfaces::FontWeight(800)).with_style(text_interfaces::FontStyle::Italic).with_stretch(text_interfaces::FontStretch::Expanded)],"complete default resolver trace");
    }
}
