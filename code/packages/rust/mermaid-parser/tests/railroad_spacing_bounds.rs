//! Each syntax dialect must reach the same exact numeric admission boundary.
use mermaid_parser::parse_railroad;

const BODIES: [&str; 4] = [
    "railroad-beta\nr = terminal(\"A\");",
    "railroad-ebnf-beta\nr = 'A';",
    "railroad-abnf-beta\nr = \"A\";",
    "railroad-peg-beta\nr <- \"A\";",
];

fn source(body: &str, field: &str, value: &str, yaml: bool) -> String {
    if yaml { format!("---\nconfig:\n  railroad:\n    {field}: {value}\n---\n{body}") }
    else { format!("%%{{init: {{\"railroad\": {{\"{field}\": {value}}}}}}}%%\n{body}") }
}

#[test]
fn railroad_spacing_refuses_exact_unsupported_values_in_every_source_form() {
    let mut failures = Vec::new(); let mut cells = 0;
    for body in BODIES { for yaml in [false,true] {
        for field in ["padding","verticalSeparation","horizontalSeparation"] {
            for value in ["NaN","inf","-inf","-1","65537","1e20","1e308"] {
                let result = parse_railroad(&source(body,field,value,yaml));
                let expected = format!("Mermaid Railroad {field} must be finite and in 0..=65536");
                if !matches!(&result,Err(error) if error.message==expected && error.line==1 && error.col==1) {
                    failures.push(format!("{} yaml={yaml} {field}={value}: {result:?}",body.lines().next().unwrap()));
                }
                cells += 1;
            }
        }
    }}
    assert_eq!(cells,168);
    assert!(failures.is_empty(),"exact parser admission failures {}/168: {failures:?}",failures.len());
}

#[test]
fn railroad_spacing_preserves_complete_accepted_config_without_clamping() {
    let mut cells=0;
    for body in BODIES { for yaml in [false,true] {
        for (field,index) in [("padding",0),("verticalSeparation",1),("horizontalSeparation",2)] {
            for value in [0.0,0.5,4.0,10.0,65536.0] {
                let parsed=parse_railroad(&source(body,field,&value.to_string(),yaml)).unwrap();
                let mut expected=[10.0,8.0,10.0];expected[index]=value;
                assert_eq!([parsed.config.padding,parsed.config.vertical_separation,parsed.config.horizontal_separation],expected);
                assert_eq!(parsed.rules.len(),1);cells+=1;
            }
        }
    }}
    assert_eq!(cells,120);
}
