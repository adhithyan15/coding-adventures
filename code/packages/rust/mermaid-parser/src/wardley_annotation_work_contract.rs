//! Actual comparison counts and complete stable-group IR, before lookup repair.
use super::*;
fn expected()->WardleyDiagram {WardleyDiagram {title:None,config:WardleyConfig {
 width:900.0,height:600.0,padding:48.0,node_radius:6.0,node_label_offset:8.0,axis_font_size:12.0,label_font_size:10.0,show_grid:false,use_max_width:true,
 theme:WardleyTheme {background_color:"#ffffff".into(),axis_color:"#000000".into(),axis_text_color:"#222222".into(),grid_color:"rgba(100, 100, 100, 0.2)".into(),component_fill:"#ffffff".into(),component_stroke:"#000000".into(),component_label_color:"#222222".into(),link_stroke:"#000000".into(),evolution_stroke:"#dc3545".into()}},
 stages:vec!["Genesis".into(),"Custom Built".into(),"Product".into(),"Commodity".into()],nodes:vec![],links:vec![],evolves:vec![],notes:vec![],annotations_box:None,annotations:vec![],forces:vec![]}}

fn distinct(count:usize,limit:usize) {
 for descending in [false,true] {
 let order:Vec<_>=if descending {(1..=count).rev().collect()}else{(1..=count).collect()};
 let mut source=String::from("wardley-beta\n");for number in &order {source.push_str(&format!("annotation {number},[0.5,0.5] \"A\"\n"));}
 let mut want=expected();want.annotations=order.iter().map(|number|WardleyAnnotation{number:*number,text:"A".into(),coordinates:vec![(0.5,0.5)]}).collect();
 WARDLEY_ANNOTATION_COMPARE_COUNT.with(|c|c.set(0));let got=parse_wardley(&source).unwrap();let work=WARDLEY_ANNOTATION_COMPARE_COUNT.with(std::cell::Cell::get);
 assert_eq!(got,want,"wholeIR distinct{count}/descending{descending}");
 eprintln!("WARDLEY-ANNOTATION-WORK distinct={count} descending={descending} comparisons={work} limit={limit}");
 assert!(work>0&&work<=limit,"distinct{count}/descending{descending}: actual{work} limit{limit}");
 }
}
#[test]
fn wardley_annotation_32_distinct_ids_bind_actual_comparisons_and_complete_ir() {distinct(32,384);}
#[test]
fn wardley_annotation_128_distinct_ids_bind_actual_comparisons_and_complete_ir() {distinct(128,3072);}
#[test]
fn wardley_annotation_interleaving_keeps_first_group_and_first_nonempty_text() {
 let source="wardley-beta\nannotation 3,[0.0,0.0] \"\"\nannotation 1,[0.25,0.75] \"One\"\nannotation 3,[0.5,0.5] \"Three\"\nannotation 2,[1.0,1.0] \"Two\"\nannotation 1,[0.75,0.25] \"Ignored\"\n";
 let mut want=expected();want.annotations=vec![WardleyAnnotation{number:3,text:"Three".into(),coordinates:vec![(0.0,0.0),(0.5,0.5)]},WardleyAnnotation{number:1,text:"One".into(),coordinates:vec![(0.25,0.75),(0.75,0.25)]},WardleyAnnotation{number:2,text:"Two".into(),coordinates:vec![(1.0,1.0)]}];
 WARDLEY_ANNOTATION_COMPARE_COUNT.with(|c|c.set(0));assert_eq!(parse_wardley(source).unwrap(),want);let work=WARDLEY_ANNOTATION_COMPARE_COUNT.with(std::cell::Cell::get);
 assert!(work>0&&work<=48);eprintln!("WARDLEY-ANNOTATION-WORK interleaved5 comparisons={work} limit48 wholeIR");
}
