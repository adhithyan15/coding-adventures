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
    for config in [RailroadConfig{padding:0.0,vertical_separation:0.0,horizontal_separation:0.0},
        RailroadConfig{padding:0.1,vertical_separation:0.5,horizontal_separation:0.3},
        RailroadConfig::default(),RailroadConfig{padding:65536.0,vertical_separation:65536.0,horizontal_separation:65536.0}] {
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
        let config=RailroadConfig{padding,vertical_separation:8.0,horizontal_separation:gap};
        for empty in [false,true] {
            let expression=E::Sequence(if empty{vec![]}else{vec![E::Terminal("A".into())]});
            let layout=try_layout_railroad(&input(expression,config.clone(),false)).unwrap();
            let width=if empty{0.0}else{44.0+2.0*padding};
            let exit=layout.rules[0].paths.last().unwrap();
            assert_eq!(exit.points[0].x,padding+130.0+width,"padding={padding},gap={gap},empty={empty}");
            assert_eq!(exit.points[1].x,padding+130.0+width+24.0);
        }
    }}
    let normal=try_layout_railroad(&input(E::Terminal("A".into()),RailroadConfig::default(),false)).unwrap();
    assert_eq!((normal.width,normal.height),(420.0,116.0));
}
