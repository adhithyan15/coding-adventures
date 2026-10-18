//! Normal-label contract, authored before the grid reconciliation.
//!
//! The tables come from independent Python model a36a0c46, created before any
//! production edits or returned Rust layout. Hex literals preserve operation
//! order (including the titled one-ULP witness); they are not display rounding.
//! No Cargo dependency or production cap is introduced by this test module.
use super::*;
use diagram_ir::{VennConfig, VennRegion, VennStyle, VennText};

fn assert_layout(actual: LayoutedVennDiagram, gold: LayoutedVennDiagram, context: &str) {
    // PartialEq preserves the complete owned structures. The separate bit
    // projection also distinguishes signed zero in every floating-point field.
    fn bits(layout: &LayoutedVennDiagram) -> Vec<u64> {
        let mut values = vec![
            layout.width,
            layout.height,
            layout.config.width,
            layout.config.height,
            layout.config.padding,
        ];
        for circle in &layout.circles {
            values.extend([circle.cx, circle.cy, circle.radius]);
            values.extend(circle.style.stroke_width);
            values.extend(circle.style.fill_opacity);
        }
        for label in &layout.labels {
            values.extend([label.x, label.y, label.font_size]);
            values.extend(label.style.stroke_width);
            values.extend(label.style.fill_opacity);
        }
        for circle in &layout.debug_circles {
            values.extend([circle.cx, circle.cy, circle.radius]);
        }
        for cell in &layout.debug_cells {
            values.extend([cell.x, cell.y, cell.width, cell.height]);
        }
        values.into_iter().map(f64::to_bits).collect()
    }
    assert_eq!(actual, gold, "{context}");
    assert_eq!(
        bits(&actual),
        bits(&gold),
        "{context}: exact binary64 fields"
    );
}

struct Prefix {
    circles: &'static [[u64; 3]],
    labels: &'static [[u64; 3]],
}
struct Grid {
    title: bool,
    labeled: bool,
    count: usize,
    centers: &'static [[u64; 2]],
    circles: &'static [[u64; 3]],
    cells: &'static [[u64; 4]],
}
fn f(bits: u64) -> f64 {
    f64::from_bits(bits)
}
const PREFIX: &[Prefix] = &[
    Prefix {
        circles: &[
            [0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000],
            [0x4073666666666666, 0x4072c00000000000, 0x4061800000000000],
        ],
        labels: &[
            [0x4081d66666666666, 0x4072c00000000000, 0x4038000000000000],
            [0x406ca66666666666, 0x4072c00000000000, 0x4038000000000000],
        ],
    },
    Prefix {
        circles: &[
            [0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000],
            [0x4073bd70a3d70a3e, 0x4073d00000000000, 0x4060700000000000],
        ],
        labels: &[
            [0x40818370a3d70a3d, 0x4073d00000000000, 0x4038000000000000],
            [0x406df23d70a3d70c, 0x4073d00000000000, 0x4038000000000000],
        ],
    },
];
const GRIDS: &[Grid] = &[
    Grid {
        title: false,
        labeled: false,
        count: 0,
        centers: &[],
        circles: &[],
        cells: &[],
    },
    Grid {
        title: true,
        labeled: false,
        count: 0,
        centers: &[],
        circles: &[],
        cells: &[],
    },
    Grid {
        title: false,
        labeled: true,
        count: 0,
        centers: &[],
        circles: &[],
        cells: &[],
    },
    Grid {
        title: true,
        labeled: true,
        count: 0,
        centers: &[],
        circles: &[],
        cells: &[],
    },
    Grid {
        title: false,
        labeled: false,
        count: 1,
        centers: &[[0x407e99999999999a, 0x4073b00000000000]],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[[
            0x407649999999999a,
            0x4066c00000000000,
            0x4070a00000000000,
            0x4070a00000000000,
        ]],
    },
    Grid {
        title: true,
        labeled: false,
        count: 1,
        centers: &[[0x407e428f5c28f5c2, 0x4074c00000000000]],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[[
            0x407673c28f5c28f5,
            0x4069e26666666666,
            0x406f3b3333333333,
            0x406f3b3333333333,
        ]],
    },
    Grid {
        title: false,
        labeled: true,
        count: 1,
        centers: &[[0x407e99999999999a, 0x4074b00000000000]],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[[
            0x407649999999999a,
            0x4068c00000000000,
            0x4070a00000000000,
            0x4070a00000000000,
        ]],
    },
    Grid {
        title: true,
        labeled: true,
        count: 1,
        centers: &[[0x407e428f5c28f5c2, 0x4075c00000000000]],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[[
            0x407673c28f5c28f5,
            0x406be26666666666,
            0x406f3b3333333333,
            0x406f3b3333333333,
        ]],
    },
    Grid {
        title: false,
        labeled: false,
        count: 2,
        centers: &[
            [0x407a71999999999a, 0x4073b00000000000],
            [0x408160cccccccccd, 0x4073b00000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4066c00000000000,
                0x4060a00000000000,
                0x4070a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4066c00000000000,
                0x4060a00000000000,
                0x4070a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: false,
        count: 2,
        centers: &[
            [0x407a5b28f5c28f5b, 0x4074c00000000000],
            [0x408114fae147ae14, 0x4074c00000000000],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x4069e26666666666,
                0x405f3b3333333333,
                0x406f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x4069e26666666666,
                0x405f3b3333333333,
                0x406f3b3333333333,
            ],
        ],
    },
    Grid {
        title: false,
        labeled: true,
        count: 2,
        centers: &[
            [0x407a71999999999a, 0x4074b00000000000],
            [0x408160cccccccccd, 0x4074b00000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4068c00000000000,
                0x4060a00000000000,
                0x4070a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4068c00000000000,
                0x4060a00000000000,
                0x4070a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: true,
        count: 2,
        centers: &[
            [0x407a5b28f5c28f5b, 0x4075c00000000000],
            [0x408114fae147ae14, 0x4075c00000000000],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x406be26666666666,
                0x405f3b3333333333,
                0x406f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x406be26666666666,
                0x405f3b3333333333,
                0x406f3b3333333333,
            ],
        ],
    },
    Grid {
        title: false,
        labeled: false,
        count: 3,
        centers: &[
            [0x407a71999999999a, 0x406d300000000000],
            [0x408160cccccccccd, 0x406d300000000000],
            [0x407a71999999999a, 0x4076e80000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4064e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4064e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407649999999999a,
                0x4072c00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: false,
        count: 3,
        centers: &[
            [0x407a5b28f5c28f5b, 0x406fd13333333333],
            [0x408114fae147ae14, 0x406fd13333333333],
            [0x407a5b28f5c28f5b, 0x4077b76666666666],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x4068026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x4068026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407673c28f5c28f5,
                0x4073d00000000000,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
        ],
    },
    Grid {
        title: false,
        labeled: true,
        count: 3,
        centers: &[
            [0x407a71999999999a, 0x406f300000000000],
            [0x408160cccccccccd, 0x406f300000000000],
            [0x407a71999999999a, 0x4077e80000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4066e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4066e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407649999999999a,
                0x4073c00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: true,
        count: 3,
        centers: &[
            [0x407a5b28f5c28f5b, 0x4070e89999999999],
            [0x408114fae147ae14, 0x4070e89999999999],
            [0x407a5b28f5c28f5b, 0x4078b76666666666],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x406a026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x406a026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407673c28f5c28f5,
                0x4074d00000000000,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
        ],
    },
    Grid {
        title: false,
        labeled: false,
        count: 4,
        centers: &[
            [0x407a71999999999a, 0x406d300000000000],
            [0x408160cccccccccd, 0x406d300000000000],
            [0x407a71999999999a, 0x4076e80000000000],
            [0x408160cccccccccd, 0x4076e80000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4064e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4064e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407649999999999a,
                0x4072c00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4072c00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: false,
        count: 4,
        centers: &[
            [0x407a5b28f5c28f5b, 0x406fd13333333333],
            [0x408114fae147ae14, 0x406fd13333333333],
            [0x407a5b28f5c28f5b, 0x4077b76666666666],
            [0x408114fae147ae14, 0x4077b76666666666],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x4068026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x4068026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407673c28f5c28f5,
                0x4073d00000000000,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x4073d00000000000,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
        ],
    },
    Grid {
        title: false,
        labeled: true,
        count: 4,
        centers: &[
            [0x407a71999999999a, 0x406f300000000000],
            [0x408160cccccccccd, 0x406f300000000000],
            [0x407a71999999999a, 0x4077e80000000000],
            [0x408160cccccccccd, 0x4077e80000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4066e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4066e00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407649999999999a,
                0x4073c00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
            [
                0x407e99999999999a,
                0x4073c00000000000,
                0x4060a00000000000,
                0x4060a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: true,
        count: 4,
        centers: &[
            [0x407a5b28f5c28f5b, 0x4070e89999999999],
            [0x408114fae147ae14, 0x4070e89999999999],
            [0x407a5b28f5c28f5b, 0x4078b76666666666],
            [0x408114fae147ae14, 0x4078b76666666666],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x406a026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x406a026666666666,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407673c28f5c28f5,
                0x4074d00000000000,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
            [
                0x407e428f5c28f5c2,
                0x4074d00000000000,
                0x405f3b3333333333,
                0x405f3b3333333333,
            ],
        ],
    },
    Grid {
        title: false,
        labeled: false,
        count: 5,
        centers: &[
            [0x40790eeeeeeeeeef, 0x406d300000000000],
            [0x407e99999999999a, 0x406d300000000000],
            [0x4082122222222223, 0x406d300000000000],
            [0x40790eeeeeeeeeef, 0x4076e80000000000],
            [0x407e99999999999a, 0x4076e80000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4064e00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x407bd44444444445,
                0x4064e00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x4080af7777777778,
                0x4064e00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x407649999999999a,
                0x4072c00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x407bd44444444445,
                0x4072c00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: false,
        count: 5,
        centers: &[
            [0x40790e06d3a06d39, 0x406fd13333333333],
            [0x407e428f5c28f5c2, 0x406fd13333333333],
            [0x4081bb8bf258bf25, 0x406fd13333333333],
            [0x40790e06d3a06d39, 0x4077b76666666666],
            [0x407e428f5c28f5c2, 0x4077b76666666666],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x4068026666666666,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x407ba84b17e4b17e,
                0x4068026666666666,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x40806e69d0369d03,
                0x4068026666666666,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x407673c28f5c28f5,
                0x4073d00000000000,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x407ba84b17e4b17e,
                0x4073d00000000000,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
        ],
    },
    Grid {
        title: false,
        labeled: true,
        count: 5,
        centers: &[
            [0x40790eeeeeeeeeef, 0x406f300000000000],
            [0x407e99999999999a, 0x406f300000000000],
            [0x4082122222222223, 0x406f300000000000],
            [0x40790eeeeeeeeeef, 0x4077e80000000000],
            [0x407e99999999999a, 0x4077e80000000000],
        ],
        circles: &[[0x407e99999999999a, 0x4072c00000000000, 0x4061800000000000]],
        cells: &[
            [
                0x407649999999999a,
                0x4066e00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x407bd44444444445,
                0x4066e00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x4080af7777777778,
                0x4066e00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x407649999999999a,
                0x4073c00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
            [
                0x407bd44444444445,
                0x4073c00000000000,
                0x40562aaaaaaaaaab,
                0x4060a00000000000,
            ],
        ],
    },
    Grid {
        title: true,
        labeled: true,
        count: 5,
        centers: &[
            [0x40790e06d3a06d39, 0x4070e89999999999],
            [0x407e428f5c28f5c2, 0x4070e89999999999],
            [0x4081bb8bf258bf25, 0x4070e89999999999],
            [0x40790e06d3a06d39, 0x4078b76666666666],
            [0x407e428f5c28f5c2, 0x4078b76666666666],
        ],
        circles: &[[0x407e428f5c28f5c2, 0x4073d00000000000, 0x4060700000000000]],
        cells: &[
            [
                0x407673c28f5c28f5,
                0x406a026666666666,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x407ba84b17e4b17e,
                0x406a026666666666,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x40806e69d0369d03,
                0x406a026666666666,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x407673c28f5c28f5,
                0x4074d00000000000,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
            [
                0x407ba84b17e4b17e,
                0x4074d00000000000,
                0x4054d22222222222,
                0x405f3b3333333333,
            ],
        ],
    },
];

fn fixture(
    title: bool,
    labeled: bool,
    count: usize,
    debug: bool,
) -> (VennDiagram, LayoutedVennDiagram) {
    let config = VennConfig {
        width: 800.0,
        height: 600.0,
        padding: 20.0,
        use_debug_layout: debug,
        ..Default::default()
    };
    let title = title.then(|| "T".to_string());
    let d = VennDiagram {
        title: title.clone(),
        config: config.clone(),
        regions: vec![
            VennRegion {
                sets: vec!["A".into()],
                size: 4.0,
                label: labeled.then(|| "A".into()),
                style: VennStyle::default(),
            },
            VennRegion {
                sets: vec!["B".into()],
                size: 4.0,
                label: None,
                style: VennStyle::default(),
            },
        ],
        texts: (0..count).map(|i| text(&["A"], &format!("A{i}"))).collect(),
    };
    let grid = GRIDS
        .iter()
        .find(|g| g.title == title.is_some() && g.labeled == labeled && g.count == count)
        .unwrap();
    let prefix = &PREFIX[usize::from(title.is_some())];
    let circles = prefix
        .circles
        .iter()
        .zip(["A", "B"])
        .map(|(c, id)| LayoutedVennCircle {
            id: id.into(),
            label: id.into(),
            cx: f(c[0]),
            cy: f(c[1]),
            radius: f(c[2]),
            style: VennStyle::default(),
        })
        .collect();
    let mut labels: Vec<_> = prefix
        .labels
        .iter()
        .zip(["A", "B"])
        .map(|(p, name)| label(name, f(p[0]), f(p[1]), f(p[2]), VennStyle::default()))
        .collect();
    labels.extend(grid.centers.iter().enumerate().map(|(i, p)| {
        label(
            &format!("A{i}"),
            f(p[0]),
            f(p[1]),
            20.0,
            VennStyle::default(),
        )
    }));
    let debug_circles = if debug {
        grid.circles
            .iter()
            .map(|c| LayoutedVennDebugCircle {
                cx: f(c[0]),
                cy: f(c[1]),
                radius: f(c[2]),
            })
            .collect()
    } else {
        vec![]
    };
    let debug_cells = if debug {
        grid.cells
            .iter()
            .map(|c| LayoutedVennDebugCell {
                x: f(c[0]),
                y: f(c[1]),
                width: f(c[2]),
                height: f(c[3]),
            })
            .collect()
    } else {
        vec![]
    };
    (
        d,
        LayoutedVennDiagram {
            width: 800.0,
            height: 600.0,
            title,
            config,
            circles,
            labels,
            debug_circles,
            debug_cells,
        },
    )
}
fn text(sets: &[&str], id: &str) -> VennText {
    VennText {
        sets: sets.iter().map(|s| s.to_string()).collect(),
        id: id.into(),
        label: None,
        style: VennStyle::default(),
    }
}
fn label(name: &str, x: f64, y: f64, font_size: f64, style: VennStyle) -> LayoutedVennLabel {
    LayoutedVennLabel {
        text: name.into(),
        x,
        y,
        font_size,
        style,
    }
}

#[test]
fn venn_normal_complete_real_layouts_match_predeclared_ieee_tables() {
    for title in [false, true] {
        for labeled in [false, true] {
            for count in 0..=5 {
                for debug in [false, true] {
                    let (d, gold) = fixture(title, labeled, count, debug);
                    assert_layout(
                        try_layout_venn(&d).unwrap(),
                        gold,
                        &format!(
                            "normal count={count} label={labeled} title={title} debug={debug}"
                        ),
                    );
                    println!("VENN_NORMAL_CASE count={count} label={labeled} title={title} debug={debug}");
                }
            }
        }
    }
}

#[test]
fn venn_normal_empty_texts_preserve_complete_existing_layouts() {
    for title in [false, true] {
        for labeled in [false, true] {
            for debug in [false, true] {
                let (d, gold) = fixture(title, labeled, 0, debug);
                assert_layout(try_layout_venn(&d).unwrap(), gold, "empty text control");
            }
        }
    }
}

#[test]
fn venn_normal_groups_keep_first_authored_order_and_complete_styles() {
    let (mut d, mut gold) = fixture(false, false, 0, false);
    let style = VennStyle {
        fill: Some("#112233".into()),
        stroke: Some("#445566".into()),
        stroke_width: Some(0.0),
        text_color: Some("#abcdef".into()),
        fill_opacity: Some(0.25),
    };
    let mut a0 = text(&["A"], "a0");
    a0.style = style.clone();
    a0.label = Some("explicit".into());
    let b = text(&["B"], "b1");
    let mut a2 = text(&["A"], "a2");
    a2.label = Some(String::new());
    d.texts = vec![a0, b, a2];
    let g = GRIDS
        .iter()
        .find(|g| !g.title && !g.labeled && g.count == 2)
        .unwrap();
    gold.labels.extend([
        label(
            "explicit",
            f(g.centers[0][0]),
            f(g.centers[0][1]),
            20.0,
            style,
        ),
        label(
            "",
            f(g.centers[1][0]),
            f(g.centers[1][1]),
            20.0,
            VennStyle::default(),
        ),
        label("b1", 310.4, 315.0, 20.0, VennStyle::default()),
    ]);
    assert_layout(try_layout_venn(&d).unwrap(), gold, "group order and styles");
}

#[test]
fn venn_normal_structural_nul_groups_are_distinct_even_with_unknown_members() {
    let (mut d, mut gold) = fixture(false, false, 0, false);
    d.regions[0].sets = vec!["A\0B".into()];
    d.regions[1].sets = vec!["A".into()];
    for (c, name) in gold.circles.iter_mut().zip(["A\0B", "A"]) {
        c.id = name.into();
        c.label = name.into();
    }
    for (l, name) in gold.labels.iter_mut().zip(["A\0B", "A"]) {
        l.text = name.into();
    }
    d.texts = vec![text(&["A\0B"], "nul"), text(&["A", "B"], "pair")];
    gold.labels.extend([
        label("nul", 489.6, 315.0, 20.0, VennStyle::default()),
        label("pair", 310.4, 315.0, 20.0, VennStyle::default()),
    ]);
    assert_layout(try_layout_venn(&d).unwrap(), gold, "structural NUL groups");
}

#[test]
fn venn_normal_ordered_repeated_partial_and_missing_members_keep_their_policy() {
    let (mut d, mut gold) = fixture(false, false, 0, false);
    d.texts = vec![
        text(&["B", "A"], "reverse"),
        text(&["A", "B"], "forward"),
        text(&["A", "A"], "repeat"),
        text(&["A", "unknown"], "partial"),
        text(&[], "empty"),
        text(&["unknown"], "missing"),
    ];
    gold.labels.extend([
        label("reverse", 400.0, 315.0, 20.0, VennStyle::default()),
        label("forward", 400.0, 315.0, 20.0, VennStyle::default()),
        label("repeat", 489.6, 315.0, 20.0, VennStyle::default()),
        label("partial", 489.6, 315.0, 20.0, VennStyle::default()),
    ]);
    assert_layout(
        try_layout_venn(&d).unwrap(),
        gold,
        "ordered membership vectors",
    );
}

#[test]
fn venn_normal_duplicate_ids_use_last_center_and_any_nonempty_region_label() {
    for last_label in [None, Some(String::new()), Some("later".into())] {
        let (mut d, mut gold) = fixture(false, false, 0, false);
        d.regions[1].sets = vec!["A".into()];
        d.regions[1].label = last_label.clone();
        gold.circles[1].id = "A".into();
        gold.circles[1].label = last_label.clone().unwrap_or_else(|| "A".into());
        gold.labels[1].text = gold.circles[1].label.clone();
        d.texts = vec![text(&["A"], "duplicate")];
        let y = if last_label.as_ref().is_some_and(|s| !s.is_empty()) {
            331.0
        } else {
            315.0
        };
        gold.labels
            .push(label("duplicate", 310.4, y, 20.0, VennStyle::default()));
        assert_layout(try_layout_venn(&d).unwrap(), gold, "duplicate-ID label OR");
    }
}

#[test]
fn venn_normal_retains_scalar_error_priority_and_derived_refusal() {
    let (base, _) = fixture(false, false, 1, false);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, 65_537.0] {
        let mut d = base.clone();
        d.config.width = bad;
        d.config.height = f64::NAN;
        d.config.padding = f64::NAN;
        assert_eq!(try_layout_venn(&d), Err(VennLayoutError::Width));
        d.config.width = 800.0;
        d.config.height = bad;
        assert_eq!(try_layout_venn(&d), Err(VennLayoutError::Height));
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 65_537.0] {
        let mut d = base.clone();
        d.config.padding = bad;
        assert_eq!(try_layout_venn(&d), Err(VennLayoutError::Padding));
    }
    let mut d = base;
    d.regions[0].size = f64::NAN;
    assert_eq!(try_layout_venn(&d), Err(VennLayoutError::DerivedGeometry));
}

#[test]
fn venn_normal_work_observes_real_visits_in_both_debug_modes() {
    // The same sixteen-visits-per-singleton ceiling is retained by the older
    // debug contract. These are actual iterator visits, not elapsed time,
    // allocator limits, Hash/Eq internals or a production input cap.
    for count in [8, 32, 128] {
        for debug in [false, true] {
            let ids: Vec<_> = (0..count).map(|i| format!("s{i}")).collect();
            let d = VennDiagram {
                title: None,
                config: VennConfig {
                    use_debug_layout: debug,
                    ..Default::default()
                },
                regions: ids
                    .iter()
                    .map(|id| VennRegion {
                        sets: vec![id.clone()],
                        size: 4.0,
                        label: None,
                        style: VennStyle::default(),
                    })
                    .collect(),
                texts: ids.iter().map(|id| text(&[id], id)).collect(),
            };
            VENN_DEBUG_VISITS.with(|v| v.set(0));
            let layout = try_layout_venn(&d).unwrap();
            let visits = VENN_DEBUG_VISITS.with(std::cell::Cell::get);
            assert_eq!(layout.circles.len(), count);
            assert_eq!(layout.labels.len(), 2 * count);
            assert_eq!(layout.debug_circles.len(), if debug { count } else { 0 });
            assert_eq!(layout.debug_cells.len(), if debug { count } else { 0 });
            assert!(
                visits > 0 && visits <= 16 * count,
                "{count} singleton groups debug={debug}: {visits} real visits"
            );
            println!("VENN_NORMAL_WORK groups={count} debug={debug} visits={visits}");
        }
    }
    // T authored keys of M repeated members contribute T*M input terms even
    // when there is only one structural group. The bound includes every key;
    // it makes no promise about unobserved standard-library hashing/equality.
    for (texts, members) in [(8, 4), (32, 16), (32, 128)] {
        for debug in [false, true] {
            let (mut d, _) = fixture(false, false, 0, debug);
            d.texts = (0..texts)
                .map(|i| VennText {
                    sets: vec!["A".into(); members],
                    id: format!("long{i}"),
                    label: None,
                    style: VennStyle::default(),
                })
                .collect();
            VENN_DEBUG_VISITS.with(|v| v.set(0));
            let layout = try_layout_venn(&d).unwrap();
            let visits = VENN_DEBUG_VISITS.with(std::cell::Cell::get);
            let input_and_output = 2
                + 2
                + texts
                + texts * members
                + members
                + layout.labels.len()
                + layout.debug_cells.len();
            assert_eq!(layout.labels.len(), 2 + texts);
            assert_eq!(layout.debug_circles.len(), usize::from(debug));
            assert_eq!(layout.debug_cells.len(), if debug { texts } else { 0 });
            assert!(
                visits > 0 && visits <= 16 * input_and_output,
                "{texts} authored keys of {members} members debug={debug}: {visits}"
            );
            println!("VENN_NORMAL_KEYS texts={texts} members={members} authored_terms={} debug={debug} visits={visits}", texts * members);
        }
    }
}
