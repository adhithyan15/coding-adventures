//! The checked semantic-IR API must report exact error ownership.
use diagram_ir::*;
use diagram_layout_geometric::{try_layout_wardley,WardleyLayoutError as E};
fn config()->WardleyConfig { WardleyConfig {width:900.0,height:600.0,padding:48.0,
node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,
show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {annotation_stroke:"#000000".into(),annotation_text_color:"#222222".into(),annotation_fill:"#ffffff".into(),background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}} }
fn stages()->Vec<String> { ["Genesis","Custom Built","Product","Commodity"].map(String::from).to_vec() }
fn source(config:WardleyConfig)->WardleyDiagram { WardleyDiagram {accessibility_title:None,accessibility_description:None,title:Some("T".into()),config,
stages:stages(),nodes:vec![WardleyNode{label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false,id:"a".into(),label:"A".into(),visibility:1.0,evolution:0.0,anchor:true,pipeline_parent:false},
WardleyNode{label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false,id:"b".into(),label:"B".into(),visibility:0.0,evolution:1.0,anchor:false,pipeline_parent:false}],
links:vec![WardleyLink{dashed:false,label:None,flow:None,source:"A".into(),target:"B".into()}],
evolves:vec![WardleyEvolution{component:"A".into(),target:0.5}],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]} }

#[test]
fn wardley_config_errors_name_the_exact_field_before_empty_paths() {
    macro_rules! rejects {($field:ident,$error:expr,$bad:expr)=>{{
        let value=$bad;
        for empty in [false,true] {
            let mut d=source(config());d.config.$field=value;
            if empty {d.nodes.clear();d.links.clear();d.evolves.clear();}
            assert_eq!(try_layout_wardley(&d),Err($error),"{}={value} empty={empty}",stringify!($field));
            eprintln!("WARDLEY-ERROR {}={value} empty={empty}",stringify!($field));
        }
    }}}
    for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0,65537.0,1.5e308] {
        rejects!(width,E::Width,bad);rejects!(height,E::Height,bad);rejects!(padding,E::Padding,bad);
        rejects!(node_radius,E::NodeRadius,bad);rejects!(node_label_offset,E::NodeLabelOffset,bad);
        rejects!(axis_font_size,E::AxisFontSize,bad);rejects!(label_font_size,E::LabelFontSize,bad);
    }
    for bad in [0.0,-0.0] {
        rejects!(width,E::Width,bad);rejects!(height,E::Height,bad);rejects!(node_radius,E::NodeRadius,bad);
        rejects!(axis_font_size,E::AxisFontSize,bad);rejects!(label_font_size,E::LabelFontSize,bad);
    }
    for bad in [0.5,f64::MIN_POSITIVE] {rejects!(axis_font_size,E::AxisFontSize,bad);rejects!(label_font_size,E::LabelFontSize,bad);}
}

#[test]
fn wardley_node_evolution_and_reference_errors_are_distinct() {
    for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-0.01,1.01] {
        for which in [0,1,2] {
            let mut d=source(config());
            match which {0=>d.nodes[0].visibility=bad,1=>d.nodes[1].evolution=bad,_=>d.evolves[0].target=bad}
            assert_eq!(try_layout_wardley(&d),Err(if which==2 {E::EvolutionTarget} else {E::NodeCoordinates}),"kind{which}={bad}");
            eprintln!("WARDLEY-ERROR-NORMALIZED kind{which}={bad}");
        }
    }
    for which in [0,1,2] {
        let mut d=source(config());
        match which {0=>d.links[0].source="missing".into(),1=>d.links[0].target="missing".into(),_=>d.evolves[0].component="missing".into()}
        assert_eq!(try_layout_wardley(&d),Err(if which==2 {E::UnknownEvolutionComponent} else {E::UnknownLinkEndpoint}),"kind{which}");
        eprintln!("WARDLEY-ERROR-REFERENCE kind{which}");
    }
}
