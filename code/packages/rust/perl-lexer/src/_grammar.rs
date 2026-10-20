// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: perl (release list from code/grammars/perl/releases.csv)
// Regenerate with: grammar-tools generate-rust-compiled-grammars perl
//
// 118 compiled releases share 5 distinct TokenGrammar value(s) (LANG84 deduplication:
// grammars equal apart from comments and line numbers share one module, whose
// line numbers come from the first release in the class).
// Call `token_grammar` instead of reading and parsing grammar files at runtime.

use grammar_tools::token_grammar::TokenGrammar;

/// Every release in the inventory, pending rows included, in canonical
/// inventory order. A pending release is known but has no grammar.
pub const INVENTORY_RELEASES: &[&str] = &[
    "1.000",
    "1.001",
    "1.002",
    "1.003",
    "1.004",
    "1.005",
    "1.006",
    "1.007",
    "1.008",
    "1.009",
    "1.010",
    "1.011",
    "1.012",
    "1.013",
    "1.014",
    "2.000",
    "2.001",
    "3.000",
    "3.001",
    "3.002",
    "3.003",
    "3.004",
    "3.005",
    "3.006",
    "3.007",
    "3.008",
    "3.009",
    "3.010",
    "3.011",
    "3.012",
    "3.013",
    "3.014",
    "3.015",
    "3.016",
    "3.017",
    "3.018",
    "3.019",
    "3.020",
    "3.021",
    "3.022",
    "3.023",
    "3.024",
    "3.025",
    "3.026",
    "3.027",
    "3.028",
    "3.029",
    "3.030",
    "3.031",
    "3.032",
    "3.033",
    "3.034",
    "3.035",
    "3.036",
    "3.037",
    "3.040",
    "3.041",
    "3.042",
    "3.043",
    "3.044",
    "4.000",
    "4.001",
    "4.002",
    "4.003",
    "4.004",
    "4.005",
    "4.006",
    "4.007",
    "4.008",
    "4.009",
    "4.010",
    "4.011",
    "4.012",
    "4.013",
    "4.014",
    "4.015",
    "4.016",
    "4.017",
    "4.018",
    "4.019",
    "4.020",
    "4.021",
    "4.022",
    "4.023",
    "4.024",
    "4.025",
    "4.026",
    "4.027",
    "4.028",
    "4.029",
    "4.030",
    "4.031",
    "4.032",
    "4.033",
    "4.034",
    "4.035",
    "4.036",
    "5.000alpha1",
    "5.000alpha2",
    "5.000alpha3",
    "5.000alpha4",
    "5.000alpha5",
    "5.000alpha6",
    "5.000alpha7",
    "5.000alpha8",
    "5.000alpha9",
    "5.000alpha10",
    "5.000alpha11",
    "5.000a11a",
    "5.000a11b",
    "5.000a11c",
    "5.000a11d",
    "5.000alpha12",
    "5.000a12a",
    "5.000a12b",
    "5.000a12c",
    "5.000a12d",
    "5.000a12e",
    "5.000a12f",
    "5.000a12g",
    "5.000a12h",
    "5.000beta1",
    "5.000b1a",
    "5.000beta2",
    "5.000b2a",
    "5.000b2b",
    "5.000b2c",
    "5.000beta3",
    "5.000b3a",
    "5.000b3b",
    "5.000b3c",
    "5.000b3d",
    "5.000b3e",
    "5.000b3f",
    "5.000b3g",
    "5.000b3h",
    "5.000gamma",
    "5.000",
    "5.000a",
    "5.000b",
    "5.000c",
    "5.000d",
    "5.000e",
    "5.000f",
    "5.000g",
    "5.000h",
    "5.000i",
    "5.000j",
    "5.000k",
    "5.000l",
    "5.000m",
    "5.000n",
    "5.000o",
    "5.001",
    "5.001a",
    "5.001b",
    "5.001c",
    "5.001d",
    "5.001e",
    "5.001g",
    "5.001h",
    "5.001i",
    "5.001f",
    "5.001j",
    "5.001k",
    "5.001l",
    "5.001m",
    "5.001n",
    "5.002beta1",
    "5.002b1a",
    "5.002b1b",
    "5.002b1c",
    "5.002b1d",
    "5.002b1e",
    "5.002b1f",
    "5.002b1g",
    "5.002b1h",
    "5.002b2",
    "5.002b3",
    "5.002gamma",
    "5.002delta",
    "5.002",
    "5.002_01",
    "5.003",
    "5.003_01",
    "5.003_02",
    "5.003_03",
    "5.003_04",
    "5.003_05",
    "5.003_06",
    "5.003_07",
    "5.003_08",
    "5.003_09",
    "5.003_10",
    "5.003_11",
    "5.003_12",
    "5.003_13",
    "5.003_14",
    "5.003_15",
    "5.003_16",
    "5.003_17",
    "5.003_18",
    "5.003_19",
    "5.003_20",
    "5.003_21",
    "5.003_22",
    "5.003_23",
    "5.003_24",
    "5.003_25",
    "5.003_26",
    "5.003_27",
    "5.003_28",
    "5.003_90",
    "5.003_91",
    "5.003_92",
    "5.003_93",
    "5.003_94",
    "5.003_95",
    "5.003_96",
    "5.003_97",
    "5.003_97a",
    "5.003_97b",
    "5.003_97c",
    "5.003_97d",
    "5.003_97e",
    "5.003_97f",
    "5.003_97g",
    "5.003_97h",
    "5.003_97i",
    "5.003_97j",
    "5.003_98",
    "5.003_99",
    "5.003_99a",
    "p54rc1",
    "p54rc2",
    "5.004",
    "5.004_01-t1",
    "5.004_01-t2",
    "5.004_01",
    "5.004_01_01",
    "5.004_01_02",
    "5.004_01_03",
    "5.004_02",
    "5.004_02_01",
    "5.004_03-t2",
    "5.004_03",
    "5.004_50",
    "5.004_04-t1",
    "5.004_04-t2",
    "5.004_51",
    "5.004_04-t3",
    "5.004_04-t4",
    "5.004_04",
    "5.004_52",
    "5.004_53",
    "5.004_54",
    "5.004_55",
    "5.004_56",
    "5.004_57",
    "5.004_58",
    "5.004_59",
    "5.004_60",
    "5.004_61",
    "5.004_04-m1",
    "5.004_62",
    "5.004_63",
    "5.004_64",
    "5.004_04-m2",
    "5.004_04-m3",
    "5.004_65",
    "5.004_04-m4",
    "5.004_66",
    "5.004_67",
    "5.004_68",
    "5.004_69",
    "5.004_70",
    "5.004_71",
    "5.004_72",
    "5.004_73",
    "5.004_74",
    "5.004_75",
    "5.004_05-MT5",
    "5.004_76",
    "5.005",
    "5.005_50",
    "5.005_01",
    "5.005_02-T1",
    "5.005_02-T2",
    "5.005_02",
    "5.005_51",
    "5.005_52",
    "5.004_05-MT6",
    "5.005_53",
    "5.004_05-MT7",
    "5.005_03-MT1",
    "5.005_54",
    "5.004_05-MT8",
    "5.005_03-MT2",
    "5.005_03-MT3",
    "5.005_03-MT4",
    "5.005_03-MT5",
    "5.005_55",
    "5.005_56",
    "5.005_03-MT6",
    "5.005_03",
    "5.004_05-MT9",
    "5.004_05",
    "5.005_57",
    "5.005_58",
    "5.005_59",
    "5.005_60",
    "5.005_61",
    "5.005_62",
    "5.005_63",
    "5.5.640",
    "5.5.650",
    "5.5.660",
    "5.5.670",
    "5.6.0-RC1",
    "5.6.0-RC2",
    "5.6.0-RC3",
    "5.6.0",
    "5.7.0",
    "5.6.1-TRIAL1",
    "5.6.1-TRIAL2",
    "5.6.1-TRIAL3",
    "5.6.1-foolish",
    "5.6.1",
    "5.7.1",
    "5.7.2",
    "5.7.3",
    "5.8.0-RC1",
    "5.8.0-RC2",
    "5.8.0-RC3",
    "5.8.0",
    "1.0.15",
    "5.8.1-RC1",
    "5.8.1-RC2",
    "5.8.1-RC3",
    "5.8.1-RC4",
    "5.8.1-RC5",
    "5.8.1",
    "5.8.2-RC1",
    "5.9.0",
    "5.8.2-RC2",
    "5.8.2",
    "5.6.2-RC1",
    "5.6.2",
    "1.0_16",
    "5.8.3-RC1",
    "5.8.3",
    "5.005_04-RC1",
    "5.005_04-RC2",
    "5.005_04",
    "5.9.1",
    "5.8.4-RC1",
    "5.8.4-RC2",
    "5.8.4",
    "5.8.5-RC1",
    "5.8.5-RC2",
    "5.8.5",
    "5.8.6-RC1",
    "5.8.6",
    "5.9.2",
    "5.8.7-RC1",
    "5.8.7",
    "5.8.8-RC1",
    "5.9.3",
    "5.8.8",
    "5.9.4",
    "5.9.5",
    "5.10.0-RC1",
    "5.10.0-RC2",
    "5.10.0",
    "5.8.9-RC1",
    "5.8.9-RC2",
    "5.8.9",
    "5.005_05-RC1",
    "5.10.1-RC1",
    "5.10.1-RC2",
    "5.10.1",
    "5.11.0",
    "5.11.1",
    "5.11.2",
    "5.11.3",
    "5.11.4",
    "5.11.5",
    "5.12.0-RC0",
    "5.12.0-RC1",
    "5.12.0-RC2",
    "5.12.0-RC3",
    "5.12.0-RC4",
    "5.12.0-RC5",
    "5.12.0",
    "5.13.0",
    "5.12.1-RC0",
    "5.12.1-RC1",
    "5.12.1-RC2",
    "5.12.1",
    "5.13.1",
    "5.13.2",
    "5.13.3",
    "5.13.4",
    "5.12.2-RC1",
    "5.12.2",
    "5.13.5",
    "5.13.6",
    "5.13.7",
    "5.13.8",
    "5.12.3-RC1",
    "5.12.3-RC2",
    "5.12.3-RC3",
    "5.13.9",
    "5.12.3",
    "5.13.10",
    "5.13.11",
    "5.14.0-RC1",
    "5.14.0-RC2",
    "5.14.0-RC3",
    "5.14.0",
    "5.12.4-RC1",
    "5.14.1-RC1",
    "5.12.4-RC2",
    "5.14.1",
    "5.12.4",
    "5.15.0",
    "5.15.1",
    "5.15.2",
    "5.14.2-RC1",
    "5.15.3",
    "5.14.2",
    "5.15.4",
    "5.15.5",
    "5.15.6",
    "5.15.7",
    "5.15.8",
    "5.15.9",
    "5.16.0-RC0",
    "5.16.0-RC1",
    "5.16.0-RC2",
    "5.16.0",
    "5.17.0",
    "5.17.1",
    "5.17.2",
    "5.16.1",
    "5.17.3",
    "5.17.4",
    "5.14.3-RC1",
    "5.14.3-RC2",
    "5.14.3",
    "5.17.5",
    "5.16.2",
    "5.12.5-RC1",
    "5.12.5-RC2",
    "5.12.5",
    "5.17.6",
    "5.17.7",
    "5.17.8",
    "5.17.9",
    "5.14.4-RC1",
    "5.16.3-RC1",
    "5.14.4-RC2",
    "5.14.4",
    "5.16.3",
    "5.17.10",
    "5.17.11",
    "5.18.0-RC1",
    "5.18.0-RC2",
    "5.18.0-RC3",
    "5.18.0-RC4",
    "5.18.0",
    "5.19.0",
    "5.19.1",
    "5.19.2",
    "5.18.1-RC1",
    "5.18.1-RC2",
    "5.18.1-RC3",
    "5.18.1",
    "5.19.3",
    "5.19.4",
    "5.19.5",
    "5.19.6",
    "5.19.7",
    "5.18.2-RC1",
    "5.18.2-RC2",
    "5.18.2-RC3",
    "5.18.2-RC4",
    "5.18.2",
    "5.19.8",
    "5.19.9",
    "5.19.10",
    "5.19.11",
    "5.20.0-RC1",
    "5.20.0",
    "5.21.0",
    "5.21.1",
    "5.21.2",
    "5.21.3",
    "5.20.1-RC1",
    "5.20.1-RC2",
    "5.20.1",
    "5.18.3-RC1",
    "5.21.4",
    "5.18.3-RC2",
    "5.18.3",
    "5.18.4",
    "5.21.5",
    "5.21.6",
    "5.21.7",
    "5.21.8",
    "5.20.2-RC1",
    "5.20.2",
    "5.21.9",
    "5.21.10",
    "5.21.11",
    "5.22.0-RC1",
    "5.22.0-RC2",
    "5.22.0",
    "5.23.0",
    "5.23.1",
    "5.23.2",
    "5.20.3-RC1",
    "5.20.3-RC2",
    "5.20.3",
    "5.23.3",
    "5.23.4",
    "5.22.1-RC1",
    "5.22.1-RC2",
    "5.23.5",
    "5.22.1-RC3",
    "5.22.1-RC4",
    "5.22.1",
    "5.23.6",
    "5.23.7",
    "5.23.8",
    "5.23.9",
    "5.22.2-RC1",
    "5.24.0-RC1",
    "5.24.0-RC2",
    "5.24.0-RC3",
    "5.22.2",
    "5.24.0-RC4",
    "5.24.0-RC5",
    "5.24.0",
    "5.25.0",
    "5.25.1",
    "5.25.2",
    "5.22.3-RC1",
    "5.24.1-RC1",
    "5.25.3",
    "5.22.3-RC2",
    "5.24.1-RC2",
    "5.22.3-RC3",
    "5.24.1-RC3",
    "5.25.4",
    "5.25.5",
    "5.22.3-RC4",
    "5.24.1-RC4",
    "5.25.6",
    "5.25.7",
    "5.25.8",
    "5.22.3-RC5",
    "5.24.1-RC5",
    "5.22.3",
    "5.24.1",
    "5.25.9",
    "5.25.10",
    "5.25.11",
    "5.25.12",
    "5.26.0-RC1",
    "5.26.0-RC2",
    "5.26.0",
    "5.27.0",
    "5.27.1",
    "5.22.4-RC1",
    "5.24.2-RC1",
    "5.22.4",
    "5.24.2",
    "5.27.2",
    "5.27.3",
    "5.24.3-RC1",
    "5.26.1-RC1",
    "5.27.4",
    "5.24.3",
    "5.26.1",
    "5.27.5",
    "5.27.6",
    "5.27.7",
    "5.27.8",
    "5.27.9",
    "5.27.10",
    "5.24.4-RC1",
    "5.26.2-RC1",
    "5.24.4",
    "5.26.2",
    "5.27.11",
    "5.28.0-RC1",
    "5.28.0-RC2",
    "5.28.0-RC3",
    "5.28.0-RC4",
    "5.28.0",
    "5.29.0",
    "5.29.1",
    "5.29.2",
    "5.29.3",
    "5.29.4",
    "5.26.3-RC1",
    "5.28.1-RC1",
    "5.29.5",
    "5.26.3",
    "5.28.1",
    "5.29.6",
    "5.29.7",
    "5.29.8",
    "5.29.9",
    "5.28.2-RC1",
    "5.28.2",
    "5.29.10",
    "5.30.0-RC1",
    "5.30.0-RC2",
    "5.30.0",
    "5.31.0",
    "5.31.1",
    "5.31.2",
    "5.31.3",
    "5.31.4",
    "5.31.5",
    "5.30.1-RC1",
    "5.30.1",
    "5.31.6",
    "5.31.7",
    "5.31.8",
    "5.31.9",
    "5.30.2-RC1",
    "5.30.2",
    "5.31.10",
    "5.31.11",
    "5.28.3-RC1",
    "5.30.3-RC1",
    "5.32.0-RC0",
    "5.28.3",
    "5.30.3",
    "5.32.0-RC1",
    "5.32.0",
    "5.33.0",
    "5.33.1",
    "5.33.2",
    "5.33.3",
    "5.33.4",
    "5.33.5",
    "5.32.1-RC1",
    "5.33.6",
    "5.32.1",
    "5.33.7",
    "5.33.8",
    "5.33.9",
    "5.34.0-RC1",
    "5.34.0-RC2",
    "5.34.0",
    "5.35.0",
    "5.35.1",
    "5.35.2",
    "5.35.3",
    "5.35.4",
    "5.35.5",
    "5.35.6",
    "5.35.7",
    "5.35.8",
    "5.35.9",
    "5.34.1-RC1",
    "5.34.1-RC2",
    "5.34.1",
    "5.35.10",
    "5.35.11",
    "5.36.0-RC1",
    "5.36.0-RC2",
    "5.36.0-RC3",
    "5.36.0",
    "5.37.0",
    "5.37.1",
    "5.37.2",
    "5.37.3",
    "5.37.4",
    "5.37.5",
    "5.37.6",
    "5.37.7",
    "5.37.8",
    "5.37.9",
    "5.37.10",
    "5.36.1-RC1",
    "5.36.1-RC2",
    "5.36.1-RC3",
    "5.37.11",
    "5.36.1",
    "5.38.0-RC1",
    "5.38.0-RC2",
    "5.39.0",
    "5.38.0",
    "5.39.1",
    "5.39.2",
    "5.39.3",
    "5.39.4",
    "5.39.5",
    "5.34.2",
    "5.36.2",
    "5.38.1",
    "5.34.3",
    "5.36.3",
    "5.38.2",
    "5.39.6",
    "5.39.7",
    "5.39.8",
    "5.39.9",
    "5.39.10",
    "5.40.0-RC1",
    "5.40.0-RC2",
    "5.40.0",
    "5.41.0",
    "5.41.1",
    "5.41.2",
    "5.41.3",
    "5.41.4",
    "5.41.5",
    "5.41.6",
    "5.41.7",
    "5.38.3-RC1",
    "5.40.1-RC1",
    "5.38.3",
    "5.40.1",
    "5.41.8",
    "5.41.9",
    "5.41.10",
    "5.38.4-RC1",
    "5.40.2-RC1",
    "5.38.4",
    "5.40.2",
    "5.41.11",
    "5.41.12",
    "5.41.13",
    "5.42.0-RC1",
    "5.42.0-RC2",
    "5.42.0-RC3",
    "5.42.0",
    "5.43.0",
    "5.43.1",
    "5.38.5-RC1",
    "5.40.3-RC1",
    "5.38.5",
    "5.40.3",
    "5.43.2",
    "5.43.3",
    "5.43.4",
    "5.43.5",
    "5.43.6",
    "5.43.7",
    "5.43.8",
    "5.42.1-RC1",
    "5.42.1",
    "5.43.9",
    "5.40.4-RC1",
    "5.42.2-RC1",
    "5.40.4",
    "5.42.2",
    "5.43.10",
    "5.43.11",
    "5.44.0-RC1",
    "5.44.0-RC2",
    "5.44.0",
    "5.45.0",
    "5.45.1",
    "5.40.5-RC1",
    "5.42.3-RC1",
    "5.40.5",
    "5.42.3",
    "5.45.2",
    "5.45.3",
];

/// Every release with a compiled grammar (non-pending rows), in inventory order.
pub const SUPPORTED_RELEASES: &[&str] = &[
    "1.000",
    "2.000",
    "2.001",
    "3.000",
    "3.044",
    "4.000",
    "4.036",
    "5.000",
    "5.001",
    "5.001n",
    "5.002",
    "5.002_01",
    "5.003",
    "5.003_01",
    "5.003_02",
    "5.003_03",
    "5.003_04",
    "5.003_05",
    "5.003_06",
    "5.003_07",
    "5.003_08",
    "5.003_09",
    "5.003_10",
    "5.003_11",
    "5.003_12",
    "5.003_13",
    "5.003_14",
    "5.003_15",
    "5.003_16",
    "5.003_17",
    "5.003_18",
    "5.003_19",
    "5.003_20",
    "5.003_21",
    "5.003_22",
    "5.003_23",
    "5.003_24",
    "5.003_25",
    "5.003_26",
    "5.003_27",
    "5.003_28",
    "5.003_90",
    "5.003_91",
    "5.003_92",
    "5.003_93",
    "5.003_94",
    "5.003_95",
    "5.003_96",
    "5.003_97",
    "5.003_97a",
    "5.003_97b",
    "5.003_97c",
    "5.003_97d",
    "5.003_97e",
    "5.003_97f",
    "5.003_97g",
    "5.003_97h",
    "5.003_97i",
    "5.003_97j",
    "5.003_98",
    "5.003_99",
    "5.003_99a",
    "p54rc1",
    "p54rc2",
    "5.004",
    "5.004_01-t2",
    "5.004_01",
    "5.004_01_01",
    "5.004_01_02",
    "5.004_01_03",
    "5.004_02",
    "5.004_02_01",
    "5.004_03-t2",
    "5.004_03",
    "5.004_50",
    "5.004_04-t1",
    "5.004_04-t2",
    "5.004_51",
    "5.004_04-t3",
    "5.004_04-t4",
    "5.004_04",
    "5.004_52",
    "5.004_53",
    "5.004_54",
    "5.004_55",
    "5.004_56",
    "5.004_57",
    "5.004_58",
    "5.004_59",
    "5.004_60",
    "5.004_61",
    "5.004_04-m1",
    "5.004_62",
    "5.004_63",
    "5.004_64",
    "5.004_04-m2",
    "5.004_04-m3",
    "5.004_65",
    "5.004_04-m4",
    "5.004_66",
    "5.004_67",
    "5.004_68",
    "5.004_69",
    "5.004_70",
    "5.004_71",
    "5.004_72",
    "5.004_05-MT5",
    "5.004_05-MT6",
    "5.004_05-MT7",
    "5.004_05-MT8",
    "5.004_05-MT9",
    "5.004_05",
    "1.0.15",
    "1.0_16",
    "5.38.0",
    "5.38.2",
    "5.44.0",
    "5.45.3",
];

/// The compiled TokenGrammar of exactly one inventory release, or `None` when the
/// release is pending or not in the inventory. There is no fallback.
pub fn token_grammar(release: &str) -> Option<TokenGrammar> {
    match release {
        "1.000"
        | "2.000"
        | "2.001"
        | "3.000"
        | "3.044"
        | "4.000"
        | "4.036"
        | "5.000"
        | "5.001"
        | "5.001n"
        | "5.002"
        | "5.002_01"
        | "5.003"
        | "5.003_01"
        | "5.003_02"
        | "5.003_03"
        | "5.003_04"
        | "5.003_05"
        | "5.003_06"
        | "5.003_07"
        | "5.003_08"
        | "5.003_09"
        | "5.003_10"
        | "5.003_11"
        | "5.003_12"
        | "5.003_13"
        | "5.003_14"
        | "5.003_15"
        | "5.003_16"
        | "5.003_17"
        | "5.003_18"
        | "5.003_19"
        | "5.003_20"
        | "5.003_21"
        | "5.003_22"
        | "5.003_23"
        | "5.003_24"
        | "5.003_25"
        | "1.0.15"
        | "1.0_16"
            => Some(g_296afecde0d2::token_grammar()),
        "5.003_26"
        | "5.003_27"
        | "5.003_28"
        | "5.003_90"
        | "5.003_91"
        | "5.003_92"
        | "5.003_93"
        | "5.003_94"
        | "5.003_95"
        | "5.003_96"
        | "5.003_97"
        | "5.003_97a"
        | "5.003_97b"
        | "5.003_97c"
        | "5.003_97d"
        | "5.003_97e"
        | "5.003_97f"
        | "5.003_97g"
        | "5.003_97h"
            => Some(g_70b390200cf5::token_grammar()),
        "5.003_97i"
        | "5.003_97j"
        | "5.003_98"
        | "5.003_99"
        | "5.003_99a"
        | "p54rc1"
        | "p54rc2"
        | "5.004"
        | "5.004_01-t2"
        | "5.004_01"
        | "5.004_01_01"
        | "5.004_01_02"
        | "5.004_01_03"
        | "5.004_02"
        | "5.004_02_01"
        | "5.004_03-t2"
        | "5.004_03"
        | "5.004_50"
        | "5.004_04-t1"
        | "5.004_04-t2"
        | "5.004_51"
        | "5.004_04-t3"
        | "5.004_04-t4"
        | "5.004_04"
        | "5.004_52"
        | "5.004_53"
        | "5.004_54"
        | "5.004_55"
        | "5.004_56"
        | "5.004_57"
        | "5.004_58"
        | "5.004_59"
        | "5.004_60"
        | "5.004_61"
        | "5.004_04-m1"
        | "5.004_62"
        | "5.004_63"
        | "5.004_64"
        | "5.004_04-m2"
        | "5.004_04-m3"
        | "5.004_65"
        | "5.004_04-m4"
        | "5.004_66"
        | "5.004_67"
        | "5.004_68"
        | "5.004_69"
        | "5.004_70"
        | "5.004_71"
        | "5.004_05-MT5"
        | "5.004_05-MT6"
        | "5.004_05-MT7"
        | "5.004_05-MT8"
        | "5.004_05-MT9"
        | "5.004_05"
            => Some(g_70013ef7ab94::token_grammar()),
        "5.004_72"
        | "5.38.0"
            => Some(g_a196e6b62667::token_grammar()),
        "5.38.2"
        | "5.44.0"
        | "5.45.3"
            => Some(g_d410b3a57cf4::token_grammar()),
        _ => None,
    }
}

/// The unversioned pair. It is not an inventory release and is never
/// returned for one; callers that use it must name it explicitly.
pub fn unversioned_token_grammar() -> TokenGrammar {
    generic::token_grammar()
}

mod generic {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl.tokens
    // Regenerate with: grammar-tools compile-tokens perl.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 5,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 6,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 7,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 8,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 16,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

// 40 release(s), first 1.000
mod g_296afecde0d2 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl1.000.tokens
    // Regenerate with: grammar-tools compile-tokens perl1.000.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 17,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 18,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 21,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

// 19 release(s), first 5.003_26
mod g_70b390200cf5 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.003_26.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.003_26.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 8,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 19,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

// 54 release(s), first 5.003_97i
mod g_70013ef7ab94 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.003_97i.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.003_97i.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]{0,249}"#.to_string(),
                    is_regex: true,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 17,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 20,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

// 2 release(s), first 5.004_72
mod g_a196e6b62667 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.004_72.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.004_72.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"PRINT"#.to_string(),
                    pattern: r#"print"#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]{0,249}"#.to_string(),
                    is_regex: true,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 16,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 17,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 18,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 19,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 20,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 21,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\n\r]+"#.to_string(),
                    is_regex: true,
                    line_number: 24,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

// 3 release(s), first 5.38.2
mod g_d410b3a57cf4 {
    // AUTO-GENERATED FILE — DO NOT EDIT
    // Source: perl5.38.2.tokens
    // Regenerate with: grammar-tools compile-tokens perl5.38.2.tokens
    //
    // This file embeds a TokenGrammar as native Rust data structures.
    // Call `token_grammar()` instead of reading and parsing the .tokens file.

    #[allow(unused_imports)]
    use grammar_tools::token_grammar::{ModeTransition, PatternGroup, TokenDefinition, TokenGrammar, TransitionAction};
    #[allow(unused_imports)]
    use std::collections::HashMap;

    pub fn token_grammar() -> TokenGrammar {
        TokenGrammar {
            definitions: vec![
                TokenDefinition {
                    name: r#"INT"#.to_string(),
                    pattern: r#"0|[1-9][0-9]*"#.to_string(),
                    is_regex: true,
                    line_number: 7,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"NAME"#.to_string(),
                    pattern: r#"[a-zA-Z_][a-zA-Z0-9_]*"#.to_string(),
                    is_regex: true,
                    line_number: 8,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"PLUS"#.to_string(),
                    pattern: r#"+"#.to_string(),
                    is_regex: false,
                    line_number: 9,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"DECREMENT"#.to_string(),
                    pattern: r#"--"#.to_string(),
                    is_regex: false,
                    line_number: 10,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"MINUS"#.to_string(),
                    pattern: r#"-"#.to_string(),
                    is_regex: false,
                    line_number: 11,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"STAR"#.to_string(),
                    pattern: r#"*"#.to_string(),
                    is_regex: false,
                    line_number: 12,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"LPAREN"#.to_string(),
                    pattern: r#"("#.to_string(),
                    is_regex: false,
                    line_number: 13,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"RPAREN"#.to_string(),
                    pattern: r#")"#.to_string(),
                    is_regex: false,
                    line_number: 14,
                    alias: None,
                },
                TokenDefinition {
                    name: r#"SEMICOLON"#.to_string(),
                    pattern: r#";"#.to_string(),
                    is_regex: false,
                    line_number: 15,
                    alias: None,
                },
            ],
            keywords: vec![],
            mode: None,
            skip_definitions: vec![
                TokenDefinition {
                    name: r#"WHITESPACE"#.to_string(),
                    pattern: r#"[ \t\r\n]+"#.to_string(),
                    is_regex: true,
                    line_number: 18,
                    alias: None,
                },
            ],
            reserved_keywords: vec![],
            escapes: None,
            error_definitions: vec![],
            groups: HashMap::new(),
            case_sensitive: true,
            version: 1,
            case_insensitive: false,
            context_keywords: vec![],
            soft_keywords: vec![],
            layout_keywords: vec![],
            start_mode: None,
            transitions: vec![],
        }
    }
}

