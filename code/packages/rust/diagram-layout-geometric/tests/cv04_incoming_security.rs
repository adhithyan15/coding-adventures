//! Typed callers must obey the same incoming-field domains as authored source.
use diagram_ir::*;
use diagram_layout_geometric::{layout_ishikawa, try_layout_wardley, WardleyLayoutError};

fn wardley() -> WardleyDiagram {
    WardleyDiagram { title:None,accessibility_title:None,accessibility_description:None,config:WardleyConfig::default(),stages:vec![],stage_boundaries:vec![],
        nodes:vec![WardleyNode{id:"a".into(),label:"A".into(),visibility:0.5,evolution:0.5,anchor:false,pipeline_parent:false,label_offset_x:None,label_offset_y:None,source_strategy:None,inertia:false}],
        links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![] }
}

#[test]
fn invalid_typed_label_offsets_refuse_after_existing_diagnostics() {
    for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,65537.0,-65537.0,1e308,0.5] {
        for axis in [0,1] {
            let mut diagram=wardley();
            if axis==0 {diagram.nodes[0].label_offset_x=Some(bad);} else {diagram.nodes[0].label_offset_y=Some(bad);}
            let error=try_layout_wardley(&diagram).expect_err("invalid typed offset accepted");
            assert_eq!(format!("{error:?}"),"NodeLabelOffsets");
            diagram.config.width=f64::NAN;
            assert_eq!(try_layout_wardley(&diagram),Err(WardleyLayoutError::Width));
        }
    }
    for offset in [-65536.0,-1.0,0.0,1.0,65536.0] {
        let mut diagram=wardley();diagram.nodes[0].label_offset_x=Some(offset);diagram.nodes[0].label_offset_y=Some(offset);
        let layout=try_layout_wardley(&diagram).unwrap();
        assert_eq!((layout.nodes[0].label_offset_x,layout.nodes[0].label_offset_y),(Some(offset),Some(offset)));
    }
}

#[test]
fn ishikawa_font_normalization_preserves_complete_default_geometry() {
    let mut diagram=IshikawaDiagram{effect:"E".into(),causes:vec![IshikawaCause{id:"a".into(),label:"A".into(),parent_id:None,depth:1}],config:IshikawaConfig::default()};
    let expected=layout_ishikawa(&diagram,900.0);
    for bad in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,1e308,65537.0,0.0001,0.0,-1.0] {
        diagram.config.font_size=bad;
        assert_eq!(layout_ishikawa(&diagram,900.0),expected,"unsupported typed font {bad}");
    }
    for good in [1.0,16.0,22.0,65536.0] {
        diagram.config.font_size=good;let layout=layout_ishikawa(&diagram,900.0);
        assert_eq!(layout.config.font_size,good);
        for bone in &layout.bones {
            assert!(bone.label_width.is_finite()&&bone.label_width>0.0&&bone.label_height.is_finite()&&bone.label_height>0.0);
            assert!(bone.label_outline.iter().chain(&bone.path).chain(&bone.arrowhead).all(|p|p.x.is_finite()&&p.y.is_finite()));
        }
        assert!(layout.effect_outline.iter().chain(&layout.spine_path).all(|p|p.x.is_finite()&&p.y.is_finite()));
    }
}
