//! Independent exact Wardley frames and rejection at the legacy typed entry.
use diagram_ir::*;
use diagram_layout_geometric::layout_wardley;

fn config()->WardleyConfig { WardleyConfig {width:900.0,height:600.0,padding:48.0,
node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,
show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}} }
fn stages()->Vec<String> { ["Genesis","Custom Built","Product","Commodity"].map(String::from).to_vec() }
fn source(config:WardleyConfig)->WardleyDiagram { WardleyDiagram {title:Some("T".into()),config,
stages:stages(),nodes:vec![WardleyNode{id:"a".into(),label:"A".into(),visibility:1.0,evolution:0.0,anchor:true,pipeline_parent:false},
WardleyNode{id:"b".into(),label:"B".into(),visibility:0.0,evolution:1.0,anchor:false,pipeline_parent:false}],
links:vec![WardleyLink{source:"A".into(),target:"B".into()}],
evolves:vec![WardleyEvolution{component:"A".into(),target:0.5}],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]} }
fn frame(config:WardleyConfig,width:f64,height:f64,lo:f64,hi_x:f64,hi_y:f64,mid_x:f64)->LayoutedWardleyDiagram {
LayoutedWardleyDiagram {width,height,title:Some("T".into()),config,stages:stages(),
nodes:vec![LayoutedWardleyNode{id:"a".into(),label:"A".into(),position:Point{x:lo,y:lo},anchor:true,pipeline_parent:false},
LayoutedWardleyNode{id:"b".into(),label:"B".into(),position:Point{x:hi_x,y:hi_y},anchor:false,pipeline_parent:false}],
links:vec![LayoutedWardleyLink{from:Point{x:lo,y:lo},to:Point{x:hi_x,y:hi_y}}],
evolves:vec![LayoutedWardleyEvolution{from:Point{x:lo,y:lo},to:Point{x:mid_x,y:lo}}],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![]} }

#[test]
fn wardley_complete_default_minimum_and_cap_frames_are_literal() {
    for (name,c,w,h,lo,x,y,mid) in [
        ("default",config(),900.0,600.0,48.0,852.0,552.0,450.0),
        ("minimum",WardleyConfig{width:1.0,height:1.0,axis_font_size:1.0,label_font_size:1.0,..config()},420.0,300.0,48.0,372.0,252.0,210.0),
        ("cap",WardleyConfig{width:65536.0,height:65536.0,padding:0.0,node_radius:65536.0,node_label_offset:65536.0,axis_font_size:65536.0,label_font_size:65536.0,..config()},65536.0,65536.0,0.0,65536.0,65536.0,32768.0),
    ] {
        assert_eq!(layout_wardley(&source(c.clone())),frame(c,w,h,lo,x,y,mid),"{name}");
        eprintln!("WARDLEY-FRAME {name}");
    }
}

#[test]
fn wardley_typed_config_is_checked_even_without_nodes() {
    macro_rules! rejects {($field:ident,$bad:expr)=>{{
        let value=$bad;
        for empty in [false,true] {
            let mut d=source(config());d.config.$field=value;
            if empty {d.nodes.clear();d.links.clear();d.evolves.clear();}
            assert!(std::panic::catch_unwind(||layout_wardley(&d)).is_err(),"{}={value} empty={empty}",stringify!($field));
            eprintln!("WARDLEY-TYPED {}={value} empty={empty}",stringify!($field));
        }
    }}}
    for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0,65537.0,1.5e308] {
        rejects!(width,bad);rejects!(height,bad);rejects!(padding,bad);
        rejects!(node_radius,bad);rejects!(node_label_offset,bad);
        rejects!(axis_font_size,bad);rejects!(label_font_size,bad);
    }
    for bad in [0.0,-0.0] {
        rejects!(width,bad);rejects!(height,bad);rejects!(node_radius,bad);
        rejects!(axis_font_size,bad);rejects!(label_font_size,bad);
    }
    for bad in [0.5,f64::MIN_POSITIVE] {rejects!(axis_font_size,bad);rejects!(label_font_size,bad);}
}

#[test]
fn wardley_typed_normalized_nodes_and_evolution_are_checked() {
    for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-0.01,1.01] {
        for which in [0,1,2] {
            let mut d=source(config());
            match which {0=>d.nodes[0].visibility=bad,1=>d.nodes[1].evolution=bad,_=>d.evolves[0].target=bad}
            assert!(std::panic::catch_unwind(||layout_wardley(&d)).is_err(),"kind{which}={bad}");
            eprintln!("WARDLEY-NORMALIZED kind{which}={bad}");
        }
    }
}

#[test]
fn wardley_typed_unknown_references_are_not_silently_dropped() {
    for which in [0,1,2] {
        let mut d=source(config());
        match which {0=>d.links[0].source="missing".into(),1=>d.links[0].target="missing".into(),_=>d.evolves[0].component="missing".into()}
        assert!(std::panic::catch_unwind(||layout_wardley(&d)).is_err(),"kind{which}");
        eprintln!("WARDLEY-REFERENCE kind{which}");
    }
}
