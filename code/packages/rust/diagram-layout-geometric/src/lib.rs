use diagram_ir::{GeoElement, GeometricDiagram, LayoutedGeometricDiagram};

pub const VERSION: &str = "0.1.0";
const MARGIN: f64 = 20.0;

pub fn layout_geometric_diagram(diagram: &GeometricDiagram) -> LayoutedGeometricDiagram {
    let (w, h) = match (diagram.width, diagram.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => { let (_,_,_,my) = bounding_box(&diagram.elements); (w, my+MARGIN) }
        (None, Some(h)) => { let (_,_,mx,_) = bounding_box(&diagram.elements); (mx+MARGIN, h) }
        (None, None) => { let (_,_,mx,my) = bounding_box(&diagram.elements); ((mx+MARGIN).max(100.0),(my+MARGIN).max(100.0)) }
    };
    LayoutedGeometricDiagram { width:w, height:h, elements:diagram.elements.clone() }
}

fn bounding_box(elements: &[GeoElement]) -> (f64,f64,f64,f64) {
    if elements.is_empty() { return (0.0,0.0,200.0,100.0); }
    let mut mn_x=f64::INFINITY; let mut mn_y=f64::INFINITY; let mut mx_x=f64::NEG_INFINITY; let mut mx_y=f64::NEG_INFINITY;
    for el in elements { let (x0,y0,x1,y1) = element_aabb(el); mn_x=mn_x.min(x0); mn_y=mn_y.min(y0); mx_x=mx_x.max(x1); mx_y=mx_y.max(y1); }
    (mn_x-MARGIN, mn_y-MARGIN, mx_x+MARGIN, mx_y+MARGIN)
}

fn element_aabb(el: &GeoElement) -> (f64,f64,f64,f64) {
    match el {
        GeoElement::Box{x,y,w,h,..} => (*x,*y,x+w,y+h),
        GeoElement::Circle{cx,cy,r,..} => (cx-r,cy-r,cx+r,cy+r),
        GeoElement::Line{x1,y1,x2,y2,..} => (x1.min(*x2),y1.min(*y2),x1.max(*x2),y1.max(*y2)),
        GeoElement::Arc{cx,cy,r,..} => (cx-r,cy-r,cx+r,cy+r),
        GeoElement::Text{x,y,text,..} => (*x,y-14.0,x+text.len() as f64*7.5,*y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{GeoElement,GeometricDiagram,TextAlign};
    fn make_diagram() -> GeometricDiagram {
        GeometricDiagram { title:None, width:None, height:None, elements:vec![
            GeoElement::Box{id:"a".into(),x:50.0,y:50.0,w:120.0,h:60.0,corner_radius:0.0,label:Some("Input".into()),fill:None,stroke:None},
            GeoElement::Circle{id:"b".into(),cx:300.0,cy:80.0,r:40.0,label:Some("Proc".into()),fill:None,stroke:None},
            GeoElement::Line{id:"l1".into(),x1:170.0,y1:80.0,x2:260.0,y2:80.0,arrow_end:true,arrow_start:false,stroke:None},
        ] }
    }
    #[test] fn version_exists() { assert_eq!(VERSION, "0.1.0"); }
    #[test] fn auto_size_includes_all_elements() { let out = layout_geometric_diagram(&make_diagram()); assert!(out.width >= 340.0); assert!(out.height >= 110.0); }
    #[test] fn explicit_size_respected() { let mut d = make_diagram(); d.width=Some(800.0); d.height=Some(600.0); let out = layout_geometric_diagram(&d); assert_eq!(out.width,800.0); assert_eq!(out.height,600.0); }
    #[test] fn elements_pass_through() { let out = layout_geometric_diagram(&make_diagram()); assert_eq!(out.elements.len(), 3); }
    #[test] fn empty_gets_min_canvas() { let d = GeometricDiagram{title:None,width:None,height:None,elements:vec![]}; let out = layout_geometric_diagram(&d); assert!(out.width >= 100.0); }
    #[test] fn text_aabb_estimated() { let el = GeoElement::Text{id:"t".into(),x:10.0,y:50.0,text:"Hello".into(),align:TextAlign::Left}; let(_,y0,x1,y1) = element_aabb(&el); assert!(x1>10.0); assert!(y0<50.0); assert_eq!(y1,50.0); }
}
