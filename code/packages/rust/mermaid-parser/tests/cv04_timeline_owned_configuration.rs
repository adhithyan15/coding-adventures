//! Whole source/semantic IR authority controls, committed before reader repair.
//! Expected defaults and every18-slot palette are declared independently here.
use diagram_ir::{TimelineConfig, TimelineDiagram, TimelineDirection, TimelinePeriod, TimelineSection};
use mermaid_parser::parse_timeline;

macro_rules! complete_source_cell {
    ($name:ident, $source:expr, $config:expr, $title:expr, $description:expr, $direction:ident) => {
        #[test]
        fn $name() {
            let source = $source;
            let actual = parse_timeline(source).unwrap_or_else(|error| panic!("{}: {source:?}: {error:?}", stringify!($name)));
            let expected = ($title, TimelineDiagram {
                accessibility_title: None,
                accessibility_description: $description,
                config: $config,
                direction: TimelineDirection::$direction,
                sections: vec![TimelineSection { label: None, periods: vec![TimelinePeriod {
                    label: "2025".into(), events: vec!["event".into()],
                }] }],
            });
            assert_eq!(actual, expected, "{}: complete source/IR", stringify!($name));
            eprintln!("timeline-owned COMPLETE cell={}", stringify!($name));
        }
    };
}

complete_source_cell!(defaults,
    "timeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(full18_yaml,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(full18_init,
    "%%{init: {\"timeline\":{\"padding\":72,\"leftMargin\":80,\"useMaxWidth\":true,\"disableMulticolor\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(full18_init_lr,
    "%%{init: {\"timeline\":{\"padding\":72,\"leftMargin\":80,\"useMaxWidth\":true,\"disableMulticolor\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\ntimeline LR\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, LeftRight);

complete_source_cell!(empty_timeline_suppresses_yaml_only,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":{}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(empty_theme_suppresses_yaml_only,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"themeVariables\":{}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(absent_timeline_keeps_yaml,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(absent_theme_keeps_yaml,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":{\"padding\":64,\"leftMargin\":0,\"useMaxWidth\":true}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 64.0,
        left_margin: 0.0,
        use_max_width: true,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(first_root_absent_not_later_root,
    "%%{init: {}}%%\n%%{init: {\"timeline\":{\"padding\":72,\"leftMargin\":80,\"useMaxWidth\":true,\"disableMulticolor\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(first_root_absent_keeps_yaml,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {}}%%\n%%{init: {\"timeline\":{\"padding\":2,\"leftMargin\":3}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_timeline_false,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":false}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_theme_false,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"themeVariables\":false}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_timeline_null,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":null}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_theme_null,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"themeVariables\":null}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_timeline_array,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":[]}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_theme_array,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"themeVariables\":[]}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_timeline_string,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":\"invalid\"}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(present_invalid_theme_string,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"themeVariables\":\"invalid\"}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(duplicate_direct_first_wins,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":false,\"timeline\":{\"padding\":2,\"leftMargin\":3}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(comment_lookalike,
    "%% ignored \"timeline\":{\"padding\":409,\"leftMargin\":409},\"themeVariables\":{\"cScale0\":\"#badbad\"}\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(nested_lookalike,
    "%%{init: {\"other\":{\"timeline\":{\"padding\":72,\"leftMargin\":80,\"useMaxWidth\":true,\"disableMulticolor\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(title_lookalike,
    "timeline TD\ntitle {\"timeline\":{\"padding\":409,\"leftMargin\":409},\"themeVariables\":{\"cScale0\":\"#badbad\"}}\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("{\"timeline\":{\"padding\":409,\"leftMargin\":409},\"themeVariables\":{\"cScale0\":\"".into()), None, TopDown);

complete_source_cell!(yaml_unknown_parent,
    "---\nconfig:\n  other:\n    timeline:\n      padding: 409\n      leftMargin: 409\nother:\n  themeVariables:\n    cScale0: \"#badbad\"\n---\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(yaml_sequence_parent,
    "---\nconfig:\n  timeline:\n    - hidden:\n        padding: 409\n        leftMargin: 409\nthemeVariables:\n  - hidden:\n      cScale0: \"#badbad\"\n---\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(yaml_block_parent,
    "---\nconfig:\n  timeline: |\n    padding: 409\n    leftMargin: 409\nthemeVariables: |\n  cScale0: \"#badbad\"\n---\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(yaml_literal_init_is_data,
    "---\nnotes: |\n  %%{init: {\"timeline\":{\"padding\":409,\"leftMargin\":409}}}%%\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(quoted_prefix_not_scalar,
    "%%{init: {\"timeline\":{\"padding\":\"80\"junk,\"leftMargin\":\"90\"junk,\"useMaxWidth\":\"true\"junk,\"disableMulticolor\":\"true\"junk},\"themeVariables\":{\"cScale0\":\"#badbad\"junk}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(complete_active_quotes,
    "%%{init: {\"timeline\":{\"padding\":'64',\"leftMargin\":\"0\",\"useMaxWidth\":'TrUe',\"disableMulticolor\":\"FaLsE\"}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 64.0,
        left_margin: 0.0,
        use_max_width: true,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(zero_distinct_from_absent,
    "%%{init: {\"timeline\":{\"padding\":0,\"leftMargin\":0}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 0.0,
        left_margin: 0.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(invalid_source_scalars_negative,
    "%%{init: {\"timeline\":{\"padding\":-1,\"leftMargin\":-1}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(invalid_source_scalars_nan,
    "%%{init: {\"timeline\":{\"padding\":\"NaN\",\"leftMargin\":\"NaN\"}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(invalid_source_scalars_infinite,
    "%%{init: {\"timeline\":{\"padding\":\"inf\",\"leftMargin\":\"inf\"}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(invalid_source_scalars_trailing,
    "%%{init: {\"timeline\":{\"padding\":\"64junk\",\"leftMargin\":\"64junk\"}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(huge_finite_is_ir_not_geometry_acceptance,
    "%%{init: {\"timeline\":{\"padding\":1e+308,\"leftMargin\":1e+308}}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 1e+308,
        left_margin: 1e+308,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(accessibility_init_is_data,
    "timeline TD\naccDescr {\npréface\n%%{init: {\"timeline\":{\"padding\":409,\"leftMargin\":409,\"useMaxWidth\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\n尾\n}\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), Some("préface\n尾".into()), TopDown);

complete_source_cell!(accessibility_crlf_init_is_data,
    "timeline TD\r\naccDescr {\r\npréface\r\n%%{init: {\"timeline\":{\"padding\":409,\"leftMargin\":409,\"useMaxWidth\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\r\n尾\r\n}\r\ntitle Roadmap\r\n2025 : event\r\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), Some("préface\n尾".into()), TopDown);

complete_source_cell!(accessibility_does_not_consume_first_root,
    "timeline TD\naccDescr {\npréface\n%%{init: {\"timeline\":{\"padding\":409,\"leftMargin\":409,\"useMaxWidth\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\n尾\n}\n%%{init: {\"timeline\":{\"padding\":64,\"leftMargin\":0},\"themeVariables\":{\"cScale0\":\"#abcdef\"}}}%%\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 64.0,
        left_margin: 0.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#abcdef".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), Some("préface\n尾".into()), TopDown);

complete_source_cell!(adjacent_accessibility_closing_boundary,
    "timeline TD\naccDescr {\n%%{init: {\"timeline\":{\"padding\":409,\"leftMargin\":409,\"useMaxWidth\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\n}\naccDescr {\nrésumé\n%%{init: {\"timeline\":{\"padding\":409,\"leftMargin\":409,\"useMaxWidth\":true},\"themeVariables\":{\"cScale0\":\"#112230\",\"cScale1\":\"#112231\",\"cScale2\":\"#112232\",\"cScale3\":\"#112233\",\"cScale4\":\"#112234\",\"cScale5\":\"#112235\",\"cScaleLabel0\":\"#223340\",\"cScaleLabel1\":\"#223341\",\"cScaleLabel2\":\"#223342\",\"cScaleLabel3\":\"#223343\",\"cScaleLabel4\":\"#223344\",\"cScaleLabel5\":\"#223345\",\"cScaleInv0\":\"#334450\",\"cScaleInv1\":\"#334451\",\"cScaleInv2\":\"#334452\",\"cScaleInv3\":\"#334453\",\"cScaleInv4\":\"#334454\",\"cScaleInv5\":\"#334455\"}}}%%\n}\n%%{init: {\"timeline\":{\"padding\":64,\"leftMargin\":0},\"themeVariables\":{\"cScale0\":\"#abcdef\"}}}%%\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 64.0,
        left_margin: 0.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#abcdef".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), Some("résumé".into()), TopDown);

complete_source_cell!(malformed_timeline_object_tail,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"timeline\":{\"padding\":64,\"leftMargin\":0}junk}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#112230".into(),"#112231".into(),"#112232".into(),"#112233".into(),"#112234".into(),"#112235".into()],
        section_label_colors: vec!["#223340".into(),"#223341".into(),"#223342".into(),"#223343".into(),"#223344".into(),"#223345".into()],
        section_inverse_colors: vec!["#334450".into(),"#334451".into(),"#334452".into(),"#334453".into(),"#334454".into(),"#334455".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(malformed_theme_object_tail,
    "---\nconfig:\n  timeline:\n    padding: 72\n    leftMargin: 80\n    useMaxWidth: true\n    disableMulticolor: true\nthemeVariables:\n  cScale0: \"#112230\"\n  cScale1: \"#112231\"\n  cScale2: \"#112232\"\n  cScale3: \"#112233\"\n  cScale4: \"#112234\"\n  cScale5: \"#112235\"\n  cScaleLabel0: \"#223340\"\n  cScaleLabel1: \"#223341\"\n  cScaleLabel2: \"#223342\"\n  cScaleLabel3: \"#223343\"\n  cScaleLabel4: \"#223344\"\n  cScaleLabel5: \"#223345\"\n  cScaleInv0: \"#334450\"\n  cScaleInv1: \"#334451\"\n  cScaleInv2: \"#334452\"\n  cScaleInv3: \"#334453\"\n  cScaleInv4: \"#334454\"\n  cScaleInv5: \"#334455\"\n---\n%%{init: {\"themeVariables\":{\"cScale0\":\"#badbad\"}junk}}%%\ntimeline TD\ntitle Roadmap\n2025 : event\n",
    TimelineConfig {
        padding: 72.0,
        left_margin: 80.0,
        use_max_width: true,
        disable_multicolor: true,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("Roadmap".into()), None, TopDown);

complete_source_cell!(title_lookalike_without_hash,
    "timeline TD\ntitle {\"timeline\":{\"padding\":409,\"leftMargin\":409},\"themeVariables\":{\"cScale0\":\"red\"}}\n2025 : event\n",
    TimelineConfig {
        padding: 50.0,
        left_margin: 50.0,
        use_max_width: false,
        disable_multicolor: false,
        section_colors: vec!["#dbeafe".into(),"#dcfce7".into(),"#fef3c7".into(),"#fae8ff".into(),"#ffe4e6".into(),"#cffafe".into()],
        section_label_colors: vec!["#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into(),"#0f172a".into()],
        section_inverse_colors: vec!["#2563eb".into(),"#16a34a".into(),"#d97706".into(),"#a21caf".into(),"#e11d48".into(),"#0891b2".into()],
    }, Some("{\"timeline\":{\"padding\":409,\"leftMargin\":409},\"themeVariables\":{\"cScale0\":\"red\"}}".into()), None, TopDown);

