//! Typed admission and exact expression/exit geometry, including empty IR.
use super::*;
use diagram_ir::{RailroadConfig,RailroadDiagram,RailroadExpression as E,RailroadRule};

fn input(definition: E, config: RailroadConfig, title: bool) -> RailroadDiagram {
    RailroadDiagram { title:title.then(||"Grammar".into()),accessibility_title:None,
        accessibility_description:None,config,rules:vec![RailroadRule{name:"r".into(),definition}] }
}

#[test]
fn railroad_spacing_checked_typed_admission_is_exact_and_never_panics() {
    let mut cells=0;
    for (field,index) in [("padding",0),("verticalSeparation",1),("horizontalSeparation",2)] {
        for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY,-1.0,65537.0,1e20,1e308] {
            for empty in [false,true] {for title in [false,true] {
                let mut config=RailroadConfig::default();
                match index {0=>config.padding=value,1=>config.vertical_separation=value,_=>config.horizontal_separation=value}
                let mut diagram=input(E::Terminal("A".into()),config,title);
                if empty {diagram.rules.clear();}
                let checked=std::panic::catch_unwind(||try_layout_railroad(&diagram));
                assert!(matches!(checked,Ok(Err(RailroadLayoutError::UnsupportedSpacing{field:actual})) if actual==field),"{field}={value}, empty={empty}, title={title}: {checked:?}");
                cells+=1;
            }}
        }
    }
    assert_eq!(cells,84);
}

#[test]
fn railroad_spacing_encloses_all_constructor_geometry_at_supported_boundaries() {
    let a=E::Terminal("A".into());
    let expressions=vec![a.clone(),E::NonTerminal("B".into()),E::Special("C".into()),
        E::Sequence(vec![]),E::Sequence(vec![a.clone()]),E::Sequence(vec![a.clone(),a.clone()]),
        E::Sequence(vec![E::Sequence(vec![a.clone()]),a.clone()]),
        E::Choice(vec![]),E::Choice(vec![a.clone(),a.clone()]),E::Optional(Box::new(a.clone())),
        E::Repetition{element:Box::new(a.clone()),min:0,max:None},
        E::Repetition{element:Box::new(a),min:1,max:None}];
    let mut cells=0;
    for config in [RailroadConfig{padding:0.0,vertical_separation:0.0,horizontal_separation:0.0,..Default::default()},
        RailroadConfig{padding:0.1,vertical_separation:0.5,horizontal_separation:0.3,..Default::default()},
        RailroadConfig::default(),RailroadConfig{padding:65536.0,vertical_separation:65536.0,horizontal_separation:65536.0,..Default::default()}] {
        for expression in &expressions {for title in [false,true] {
            let diagram=input(expression.clone(),config.clone(),title);
            let layout=try_layout_railroad(&diagram).unwrap();
            assert_eq!(layout.config,config);
            assert!([layout.width,layout.height].iter().all(|v|v.is_finite()&&*v>=0.0));
            for rule in &layout.rules {
                for e in &rule.elements {
                    assert!([e.x,e.y,e.width,e.height].iter().all(|v|v.is_finite()&&*v>=0.0));
                    assert!(e.x+e.width<=layout.width+1e-7&&e.y+e.height<=layout.height+1e-7,"{expression:?}: {e:?} in {layout:?}");
                }
                for p in rule.paths.iter().flat_map(|p|&p.points) {
                    assert!(p.x.is_finite()&&p.y.is_finite()&&p.x>=0.0&&p.y>=0.0&&p.x<=layout.width+1e-7&&p.y<=layout.height+1e-7);
                }
            }
            cells+=1;
        }}
    }
    assert_eq!(cells,96);
}

#[test]
fn railroad_singleton_and_empty_sequence_have_exact_exit_geometry() {
    for padding in [0.0,0.1,0.5,10.0,65536.0] {for gap in [0.0,0.3,10.0,65536.0] {
        let config=RailroadConfig{padding,vertical_separation:8.0,horizontal_separation:gap,..Default::default()};
        for empty in [false,true] {
            let expression=E::Sequence(if empty{vec![]}else{vec![E::Terminal("A".into())]});
            let layout=try_layout_railroad(&input(expression,config.clone(),false)).unwrap();
            let width=if empty{0.0}else{config.font_size*3.15+2.0*padding};
            let exit=layout.rules[0].paths.last().unwrap();
            assert_eq!(exit.points[0].x,padding+3.0*config.font_size*0.61+40.0+width,"padding={padding},gap={gap},empty={empty}");
            assert_eq!(exit.points[1].x,padding+3.0*config.font_size*0.61+40.0+width+5.0);
        }
    }}
    let normal=try_layout_railroad(&input(E::Terminal("A".into()),RailroadConfig::default(),false)).unwrap();
    assert_eq!((normal.width,normal.height),(420.0,116.1));
}

#[test]
fn railroad_sequences_preserve_complete_ordered_leaf_connector_and_exit_geometry() {
    for config in [RailroadConfig{padding:0.0,vertical_separation:0.0,horizontal_separation:0.0,..Default::default()},
        RailroadConfig::default(),RailroadConfig{padding:0.1,vertical_separation:0.5,horizontal_separation:0.3,..Default::default()},
        RailroadConfig{padding:65536.0,vertical_separation:65536.0,horizontal_separation:65536.0,..Default::default()}] {
        let p=config.padding;let gap=config.horizontal_separation;let width=config.font_size*3.15+2.0*p;
        let height=config.font_size*1.15+2.0*p;let sequence_height=height.max(36.0);
        let origin=p+3.0*config.font_size*0.61+40.0;let content_y=16.0+2.0*p;
        let leaf_y=content_y+(sequence_height-height)/2.0;let center=content_y+sequence_height/2.0;
        let close=|actual:f64,expected:f64|assert!((actual-expected).abs()<1e-7,"{actual} != {expected}, config={config:?}");
        for nested in [false,true] {
            let a=E::Terminal("A".into());let b=E::NonTerminal("B".into());let c=E::Special("C".into());
            let expression=if nested {E::Sequence(vec![a,E::Sequence(vec![b,c])])}else{E::Sequence(vec![a,b,c])};
            let layout=try_layout_railroad(&input(expression,config.clone(),false)).unwrap();let rule=&layout.rules[0];
            assert_eq!(rule.elements.len(),3);
            for (index,(element,(label,kind))) in rule.elements.iter().zip([
                ("A",diagram_ir::RailroadElementKind::Terminal),("B",diagram_ir::RailroadElementKind::NonTerminal),
                ("? C ?",diagram_ir::RailroadElementKind::Special)]).enumerate() {
                assert_eq!((&*element.label,&element.kind),(label,&kind));
                close(element.x,origin+index as f64*(width+gap));close(element.y,leaf_y);
                close(element.width,width);close(element.height,height);
            }
            // Recursive lowering emits the inner connector before the outer
            // connector. These whole ordered paths are an independent golden,
            // not just a count or a canvas-enclosure invariant.
            let pairs=if nested{[(1,2),(0,1)]}else{[(0,1),(1,2)]};
            let mut expected:Vec<Vec<(f64,f64)>>=pairs.into_iter().map(|(from,to)|vec![
                (origin+from as f64*(width+gap)+width,center),(origin+to as f64*(width+gap),center)]).collect();
            let right=origin+3.0*width+2.0*gap;
            expected.push(vec![(origin-20.0+config.marker_radius,center),(origin,center)]);
            expected.push(vec![(right,center),(right+10.0-config.marker_radius,center)]);
            assert_eq!(rule.paths.len(),4);
            for (path,points) in rule.paths.iter().zip(expected) {
                assert!(!path.loopback);assert_eq!(path.points.len(),points.len());
                for (actual,(x,y)) in path.points.iter().zip(points) {close(actual.x,x);close(actual.y,y);}
            }
        }
        // Two rows prove the vertical gap independently of recursive choices.
        let mut rows=input(E::Terminal("A".into()),config.clone(),false);
        rows.rules.push(RailroadRule{name:"s".into(),definition:E::Terminal("B".into())});
        let layout=try_layout_railroad(&rows).unwrap();let row_height=config.font_size*1.15+24.0+4.0*p;
        assert_eq!(layout.rules.iter().map(|r|r.name.as_str()).collect::<Vec<_>>(),["r","s"]);
        close(layout.rules[0].y,8.0+p);close(layout.rules[1].y,8.0+p+row_height+config.vertical_separation);
        close(layout.height,8.0+2.0*p+2.0*row_height+2.0*config.vertical_separation);
    }
}
