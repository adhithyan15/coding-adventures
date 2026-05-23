use coding_adventures_html_parser::{
    parse_browser_document, BrowserDocument, BrowserForm, BrowserFormControl, BrowserHeading,
    BrowserImage, BrowserLink, BrowserTable,
};
use serde::Deserialize;

const BROWSER_READINESS_FIXTURE: &str = include_str!("fixtures/html-browser-readiness.json");

#[derive(Debug, Deserialize)]
struct BrowserReadinessSuite {
    format: String,
    suite: String,
    cases: Vec<BrowserReadinessCase>,
}

#[derive(Debug, Deserialize)]
struct BrowserReadinessCase {
    id: String,
    input: String,
    expected: ExpectedBrowserDocument,
}

#[derive(Debug, Deserialize)]
struct ExpectedBrowserDocument {
    title: Option<String>,
    base_href: Option<String>,
    body_text: String,
    headings: Vec<ExpectedHeading>,
    links: Vec<ExpectedLink>,
    images: Vec<ExpectedImage>,
    forms: Vec<ExpectedForm>,
    tables: Vec<ExpectedTable>,
}

#[derive(Debug, Deserialize)]
struct ExpectedHeading {
    level: u8,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedLink {
    href: Option<String>,
    name: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedImage {
    src: Option<String>,
    alt: Option<String>,
    width: Option<String>,
    height: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedForm {
    action: Option<String>,
    method: String,
    controls: Vec<ExpectedFormControl>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFormControl {
    control_type: String,
    name: Option<String>,
    value: Option<String>,
    text: String,
    options: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedTable {
    caption: Option<String>,
    row_count: usize,
    cell_count: usize,
    header_cell_count: usize,
}

#[test]
fn browser_readiness_cases_extract_browser_document_facts() {
    let suite: BrowserReadinessSuite = serde_json::from_str(BROWSER_READINESS_FIXTURE)
        .expect("browser readiness fixture should parse");

    assert_eq!(suite.format, "venture-html-browser-readiness/v1");
    assert_eq!(suite.suite, "browser-readiness");
    assert!(!suite.cases.is_empty(), "fixture should contain cases");

    for case in suite.cases {
        let actual = parse_browser_document(&case.input)
            .unwrap_or_else(|error| panic!("{} should parse: {error}", case.id));

        assert_eq!(
            actual,
            case.expected.into_browser_document(),
            "{} extracted browser facts should match",
            case.id
        );
    }
}

impl ExpectedBrowserDocument {
    fn into_browser_document(self) -> BrowserDocument {
        BrowserDocument {
            title: self.title,
            base_href: self.base_href,
            body_text: self.body_text,
            headings: self
                .headings
                .into_iter()
                .map(ExpectedHeading::into_browser_heading)
                .collect(),
            links: self
                .links
                .into_iter()
                .map(ExpectedLink::into_browser_link)
                .collect(),
            images: self
                .images
                .into_iter()
                .map(ExpectedImage::into_browser_image)
                .collect(),
            forms: self
                .forms
                .into_iter()
                .map(ExpectedForm::into_browser_form)
                .collect(),
            tables: self
                .tables
                .into_iter()
                .map(ExpectedTable::into_browser_table)
                .collect(),
        }
    }
}

impl ExpectedHeading {
    fn into_browser_heading(self) -> BrowserHeading {
        BrowserHeading {
            level: self.level,
            text: self.text,
        }
    }
}

impl ExpectedLink {
    fn into_browser_link(self) -> BrowserLink {
        BrowserLink {
            href: self.href,
            name: self.name,
            text: self.text,
        }
    }
}

impl ExpectedImage {
    fn into_browser_image(self) -> BrowserImage {
        BrowserImage {
            src: self.src,
            alt: self.alt,
            width: self.width,
            height: self.height,
        }
    }
}

impl ExpectedForm {
    fn into_browser_form(self) -> BrowserForm {
        BrowserForm {
            action: self.action,
            method: self.method,
            controls: self
                .controls
                .into_iter()
                .map(ExpectedFormControl::into_browser_form_control)
                .collect(),
        }
    }
}

impl ExpectedFormControl {
    fn into_browser_form_control(self) -> BrowserFormControl {
        BrowserFormControl {
            control_type: self.control_type,
            name: self.name,
            value: self.value,
            text: self.text,
            options: self.options,
        }
    }
}

impl ExpectedTable {
    fn into_browser_table(self) -> BrowserTable {
        BrowserTable {
            caption: self.caption,
            row_count: self.row_count,
            cell_count: self.cell_count,
            header_cell_count: self.header_cell_count,
        }
    }
}
