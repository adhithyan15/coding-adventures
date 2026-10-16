//! Whole causal Wardley IR expectations, specified before guard repair.
use diagram_ir::{WardleyConfig,WardleyDiagram};
use mermaid_parser::parse_wardley;

fn literal_config()->WardleyConfig { WardleyConfig {width:900.0,height:600.0,
padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,
label_font_size:10.0,show_grid:false,use_max_width:true,theme:diagram_ir::WardleyTheme {annotation_stroke:"#000000".into(),annotation_text_color:"#222222".into(),annotation_fill:"#ffffff".into(),background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}} }
fn expected(config:WardleyConfig)->WardleyDiagram { WardleyDiagram {accessibility_title:None,accessibility_description:None,title:None,
config,stages:vec!["Genesis".into(),"Custom Built".into(),"Product".into(),"Commodity".into()],
nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![],pipelines:vec![],stage_boundaries:vec![]} }

#[test]
fn wardley_scalars_have_complete_causal_ir() {
    for (label,source,config) in [
        ("width=1","%%{init: {\"wardley-beta\":{\"width\":\"1\"}}}%%\nwardley-beta\n",WardleyConfig {width:1.0,..literal_config()}),
        ("width=2","%%{init: {\"wardley-beta\":{\"width\":\"2\"}}}%%\nwardley-beta\n",WardleyConfig {width:2.0,..literal_config()}),
        ("width=0.5","%%{init: {\"wardley-beta\":{\"width\":\"0.5\"}}}%%\nwardley-beta\n",WardleyConfig {width:0.5,..literal_config()}),
        ("width=65536","%%{init: {\"wardley-beta\":{\"width\":\"65536\"}}}%%\nwardley-beta\n",WardleyConfig {width:65536.0,..literal_config()}),
        ("width=65537","%%{init: {\"wardley-beta\":{\"width\":\"65537\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("width=0","%%{init: {\"wardley-beta\":{\"width\":\"0\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("width=-1","%%{init: {\"wardley-beta\":{\"width\":\"-1\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("width=1.5e308","%%{init: {\"wardley-beta\":{\"width\":\"1.5e308\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("width=NaN","%%{init: {\"wardley-beta\":{\"width\":\"NaN\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("width=760'junk","%%{init: {\"wardley-beta\":{\"width\":\"760'junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("height=1","%%{init: {\"wardley-beta\":{\"height\":\"1\"}}}%%\nwardley-beta\n",WardleyConfig {height:1.0,..literal_config()}),
        ("height=2","%%{init: {\"wardley-beta\":{\"height\":\"2\"}}}%%\nwardley-beta\n",WardleyConfig {height:2.0,..literal_config()}),
        ("height=0.5","%%{init: {\"wardley-beta\":{\"height\":\"0.5\"}}}%%\nwardley-beta\n",WardleyConfig {height:0.5,..literal_config()}),
        ("height=65536","%%{init: {\"wardley-beta\":{\"height\":\"65536\"}}}%%\nwardley-beta\n",WardleyConfig {height:65536.0,..literal_config()}),
        ("height=65537","%%{init: {\"wardley-beta\":{\"height\":\"65537\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("height=0","%%{init: {\"wardley-beta\":{\"height\":\"0\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("height=-1","%%{init: {\"wardley-beta\":{\"height\":\"-1\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("height=1.5e308","%%{init: {\"wardley-beta\":{\"height\":\"1.5e308\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("height=NaN","%%{init: {\"wardley-beta\":{\"height\":\"NaN\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("height=760'junk","%%{init: {\"wardley-beta\":{\"height\":\"760'junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("padding=0","%%{init: {\"wardley-beta\":{\"padding\":\"0\"}}}%%\nwardley-beta\n",WardleyConfig {padding:0.0,..literal_config()}),
        ("padding=1","%%{init: {\"wardley-beta\":{\"padding\":\"1\"}}}%%\nwardley-beta\n",WardleyConfig {padding:1.0,..literal_config()}),
        ("padding=0.5","%%{init: {\"wardley-beta\":{\"padding\":\"0.5\"}}}%%\nwardley-beta\n",WardleyConfig {padding:0.5,..literal_config()}),
        ("padding=65536","%%{init: {\"wardley-beta\":{\"padding\":\"65536\"}}}%%\nwardley-beta\n",WardleyConfig {padding:65536.0,..literal_config()}),
        ("padding=65537","%%{init: {\"wardley-beta\":{\"padding\":\"65537\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("padding=-1","%%{init: {\"wardley-beta\":{\"padding\":\"-1\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("padding=1.5e308","%%{init: {\"wardley-beta\":{\"padding\":\"1.5e308\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("padding=NaN","%%{init: {\"wardley-beta\":{\"padding\":\"NaN\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("padding=Infinity","%%{init: {\"wardley-beta\":{\"padding\":\"Infinity\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("padding=760'junk","%%{init: {\"wardley-beta\":{\"padding\":\"760'junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeRadius=1","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"1\"}}}%%\nwardley-beta\n",WardleyConfig {node_radius:1.0,..literal_config()}),
        ("nodeRadius=2","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"2\"}}}%%\nwardley-beta\n",WardleyConfig {node_radius:2.0,..literal_config()}),
        ("nodeRadius=0.5","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"0.5\"}}}%%\nwardley-beta\n",WardleyConfig {node_radius:0.5,..literal_config()}),
        ("nodeRadius=65536","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"65536\"}}}%%\nwardley-beta\n",WardleyConfig {node_radius:65536.0,..literal_config()}),
        ("nodeRadius=65537","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"65537\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeRadius=0","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"0\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeRadius=-1","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"-1\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeRadius=1.5e308","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"1.5e308\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeRadius=NaN","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"NaN\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeRadius=760'junk","%%{init: {\"wardley-beta\":{\"nodeRadius\":\"760'junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeLabelOffset=0","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"0\"}}}%%\nwardley-beta\n",WardleyConfig {node_label_offset:0.0,..literal_config()}),
        ("nodeLabelOffset=1","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"1\"}}}%%\nwardley-beta\n",WardleyConfig {node_label_offset:1.0,..literal_config()}),
        ("nodeLabelOffset=0.5","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"0.5\"}}}%%\nwardley-beta\n",WardleyConfig {node_label_offset:0.5,..literal_config()}),
        ("nodeLabelOffset=65536","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"65536\"}}}%%\nwardley-beta\n",WardleyConfig {node_label_offset:65536.0,..literal_config()}),
        ("nodeLabelOffset=65537","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"65537\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeLabelOffset=-1","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"-1\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeLabelOffset=1.5e308","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"1.5e308\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeLabelOffset=NaN","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"NaN\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeLabelOffset=Infinity","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"Infinity\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nodeLabelOffset=760'junk","%%{init: {\"wardley-beta\":{\"nodeLabelOffset\":\"760'junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("axisFontSize=1","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"1\"}}}%%\nwardley-beta\n",WardleyConfig {axis_font_size:1.0,..literal_config()}),
        ("axisFontSize=2","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"2\"}}}%%\nwardley-beta\n",WardleyConfig {axis_font_size:2.0,..literal_config()}),
        ("axisFontSize=0.5","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"0.5\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("axisFontSize=65536","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"65536\"}}}%%\nwardley-beta\n",WardleyConfig {axis_font_size:65536.0,..literal_config()}),
        ("axisFontSize=65537","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"65537\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("axisFontSize=0","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"0\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("axisFontSize=-1","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"-1\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("axisFontSize=1.75e308","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"1.75e308\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("axisFontSize=Infinity","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"Infinity\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("axisFontSize=12'junk","%%{init: {\"wardley-beta\":{\"axisFontSize\":\"12'junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("labelFontSize=1","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"1\"}}}%%\nwardley-beta\n",WardleyConfig {label_font_size:1.0,..literal_config()}),
        ("labelFontSize=2","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"2\"}}}%%\nwardley-beta\n",WardleyConfig {label_font_size:2.0,..literal_config()}),
        ("labelFontSize=0.5","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"0.5\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("labelFontSize=65536","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"65536\"}}}%%\nwardley-beta\n",WardleyConfig {label_font_size:65536.0,..literal_config()}),
        ("labelFontSize=65537","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"65537\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("labelFontSize=0","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"0\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("labelFontSize=-1","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"-1\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("labelFontSize=1.75e308","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"1.75e308\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("labelFontSize=Infinity","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"Infinity\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("labelFontSize=12'junk","%%{init: {\"wardley-beta\":{\"labelFontSize\":\"12'junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
    ] {
        let actual=parse_wardley(source).unwrap_or_else(|e|panic!("{label}: {e:?}"));
        assert_eq!(actual,expected(config),"{label}: {source}");
        eprintln!("WARDLEY-IR {label}");
    }
}

#[test]
fn wardley_origins_have_complete_causal_ir() {
    for (label,source,config) in [
        ("default","wardley-beta\n",WardleyConfig {..literal_config()}),
        ("direct-init","%%{init: {\"wardley-beta\":{\"width\":760}}}%%\nwardley-beta\n",WardleyConfig {width:760.0,..literal_config()}),
        ("direct-header","---\nconfig:\n  wardley-beta:\n    width: 760\n---\nwardley-beta\n",WardleyConfig {width:760.0,..literal_config()}),
        ("empty-init-replaces-header","---\nconfig:\n  wardley-beta:\n    width: 760\n---\n%%{init: {\"wardley-beta\":{}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("invalid-init-replaces-header","---\nconfig:\n  wardley-beta:\n    width: 760\n---\n%%{init: {\"wardley-beta\":{\"width\":\"bad\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nested-init-object","%%{init: {\"other\":{\"wardley-beta\":{\"width\":760}}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nested-init-field","%%{init: {\"wardley-beta\":{\"other\":{\"width\":760}}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("array-target","%%{init: {\"wardley-beta\":[{\"width\":760}]}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("unknown-header-ancestor","---\nother:\n  config:\n    wardley-beta:\n      width: 760\n---\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("nested-header-field","---\nconfig:\n  wardley-beta:\n    other:\n      width: 760\n---\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("header-sequence","---\nconfig:\n  wardley-beta:\n    - width: 760\n---\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("header-block-scalar","---\nconfig:\n  wardley-beta: |\n    width: 760\n---\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("header-directive-literal","---\nnote: |\n  %%{init: {\"wardley-beta\":{\"width\":760}}}%%\n---\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("trailing-init-text","%%{init: {\"wardley-beta\":{\"width\":760}}}%% trailing\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("escaped-quote-number","%%{init: {\"wardley-beta\":{\"width\":\"760\\\"junk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("string-number","%%{init: {\"wardley-beta\":{\"width\":\"760\"}}}%%\nwardley-beta\n",WardleyConfig {width:760.0,..literal_config()}),
    ] {
        let actual=parse_wardley(source).unwrap_or_else(|e|panic!("{label}: {e:?}"));
        assert_eq!(actual,expected(config),"{label}: {source}");
        eprintln!("WARDLEY-IR {label}");
    }
}

#[test]
fn wardley_booleans_have_complete_causal_ir() {
    for (label,source,config) in [
        ("showGrid=true","%%{init: {\"wardley-beta\":{\"showGrid\":\"true\"}}}%%\nwardley-beta\n",WardleyConfig {show_grid:true,..literal_config()}),
        ("showGrid=FALSE","%%{init: {\"wardley-beta\":{\"showGrid\":\"FALSE\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("showGrid=truejunk","%%{init: {\"wardley-beta\":{\"showGrid\":\"truejunk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("useMaxWidth=true","%%{init: {\"wardley-beta\":{\"useMaxWidth\":\"true\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
        ("useMaxWidth=FALSE","%%{init: {\"wardley-beta\":{\"useMaxWidth\":\"FALSE\"}}}%%\nwardley-beta\n",WardleyConfig {use_max_width:false,..literal_config()}),
        ("useMaxWidth=truejunk","%%{init: {\"wardley-beta\":{\"useMaxWidth\":\"truejunk\"}}}%%\nwardley-beta\n",WardleyConfig {..literal_config()}),
    ] {
        let actual=parse_wardley(source).unwrap_or_else(|e|panic!("{label}: {e:?}"));
        assert_eq!(actual,expected(config),"{label}: {source}");
        eprintln!("WARDLEY-IR {label}");
    }
}

#[test]
fn wardley_body_have_complete_causal_ir() {
    for (label,source,config) in [
        ("body-overrides-init","%%{init: {\"wardley-beta\":{\"width\":1000,\"height\":700}}}%%\nwardley-beta\nsize [760,480]\n",WardleyConfig {width:760.0,height:480.0,..literal_config()}),
        ("body-at-cap","wardley-beta\nsize [65536,65536]\n",WardleyConfig {width:65536.0,height:65536.0,..literal_config()}),
    ] {
        let actual=parse_wardley(source).unwrap_or_else(|e|panic!("{label}: {e:?}"));
        assert_eq!(actual,expected(config),"{label}: {source}");
        eprintln!("WARDLEY-IR {label}");
    }
}

#[test]
fn wardley_body_size_over_cap_has_exact_source_error() {
    for source in ["wardley-beta\nsize [65537,600]\n","wardley-beta\nsize [900,65537]\n"] {
        let error=parse_wardley(source).expect_err("body size cannot bypass the scalar bound");
        assert_eq!((error.line,error.col,error.message.as_str()),
            (2,1,"size must be finite and at most 65536"),"{source}");
        eprintln!("WARDLEY-SIZE-ERROR {source:?}");
    }
}
