//! Element categories the tree construction rules name (§13.2.4.2), and the
//! fixed tables the rules consult: the quirks-mode doctype list (§13.2.6.4.1)
//! and the SVG / MathML / foreign attribute adjustments (§13.2.6.1).
//!
//! These are lists in the specification, so they are lists here: each
//! function is one `matches!` over the names the specification gives, in the
//! order it gives them, so the two can be compared line by line.

use crate::arena::Namespace;

/// §13.2.4.2 "special" category. An end tag that meets one of these while
/// walking the stack stops looking ("any other end tag"), and the adoption
/// agency's furthest block is the first one below the formatting element.
pub fn is_special(namespace: Namespace, name: &str) -> bool {
    match namespace {
        Namespace::Html => matches!(
            name,
            "address"
                | "applet"
                | "area"
                | "article"
                | "aside"
                | "base"
                | "basefont"
                | "bgsound"
                | "blockquote"
                | "body"
                | "br"
                | "button"
                | "caption"
                | "center"
                | "col"
                | "colgroup"
                | "dd"
                | "details"
                | "dir"
                | "div"
                | "dl"
                | "dt"
                | "embed"
                | "fieldset"
                | "figcaption"
                | "figure"
                | "footer"
                | "form"
                | "frame"
                | "frameset"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "head"
                | "header"
                | "hgroup"
                | "hr"
                | "html"
                | "iframe"
                | "img"
                | "input"
                | "keygen"
                | "li"
                | "link"
                | "listing"
                | "main"
                | "marquee"
                | "menu"
                | "meta"
                | "nav"
                | "noembed"
                | "noframes"
                | "noscript"
                | "object"
                | "ol"
                | "p"
                | "param"
                | "plaintext"
                | "pre"
                | "script"
                | "search"
                | "section"
                | "select"
                | "source"
                | "style"
                | "summary"
                | "table"
                | "tbody"
                | "td"
                | "template"
                | "textarea"
                | "tfoot"
                | "th"
                | "thead"
                | "title"
                | "tr"
                | "track"
                | "ul"
                | "wbr"
                | "xmp"
        ),
        Namespace::MathMl => matches!(name, "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml"),
        Namespace::Svg => matches!(name, "foreignObject" | "desc" | "title"),
    }
}

/// §13.2.4.2 formatting elements: the ones the list of active formatting
/// elements tracks and the adoption agency repairs.
pub fn is_formatting(name: &str) -> bool {
    matches!(
        name,
        "a" | "b"
            | "big"
            | "code"
            | "em"
            | "font"
            | "i"
            | "nobr"
            | "s"
            | "small"
            | "strike"
            | "strong"
            | "tt"
            | "u"
    )
}

/// §13.2.4.2 "has an element in scope": the elements that bound the default
/// scope. The other scopes add to this list. (`select` joined it, and the
/// separate "select scope" went, with customizable select.)
pub fn bounds_default_scope(namespace: Namespace, name: &str) -> bool {
    match namespace {
        Namespace::Html => matches!(
            name,
            "applet"
                | "caption"
                | "html"
                | "table"
                | "td"
                | "th"
                | "marquee"
                | "object"
                | "select"
                | "template"
        ),
        Namespace::MathMl => matches!(name, "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml"),
        Namespace::Svg => matches!(name, "foreignObject" | "desc" | "title"),
    }
}

/// §13.2.6.3 "generate implied end tags": elements whose end tag may be left
/// out, so an enclosing end tag closes them.
pub fn has_implied_end_tag(name: &str) -> bool {
    matches!(
        name,
        "dd" | "dt" | "li" | "optgroup" | "option" | "p" | "rb" | "rp" | "rt" | "rtc"
    )
}

/// §13.2.6.3 "generate all implied end tags thoroughly": the above plus the
/// table parts. Used when a `</template>` closes everything inside it.
pub fn has_implied_end_tag_thoroughly(name: &str) -> bool {
    has_implied_end_tag(name)
        || matches!(
            name,
            "caption" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead" | "tr"
        )
}

pub fn is_heading(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

pub fn is_html_whitespace(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\u{000C}' | '\r' | ' ')
}

/// §13.2.6.4.1: public identifiers that start with one of these put the
/// document in quirks mode. Compared ASCII case-insensitively.
const QUIRKS_PUBLIC_PREFIXES: &[&str] = &[
    "+//silmaril//dtd html pro v0r11 19970101//",
    "-//as//dtd html 3.0 aswedit + extensions//",
    "-//advasoft ltd//dtd html 3.0 aswedit + extensions//",
    "-//ietf//dtd html 2.0 level 1//",
    "-//ietf//dtd html 2.0 level 2//",
    "-//ietf//dtd html 2.0 strict level 1//",
    "-//ietf//dtd html 2.0 strict level 2//",
    "-//ietf//dtd html 2.0 strict//",
    "-//ietf//dtd html 2.0//",
    "-//ietf//dtd html 2.1e//",
    "-//ietf//dtd html 3.0//",
    "-//ietf//dtd html 3.2 final//",
    "-//ietf//dtd html 3.2//",
    "-//ietf//dtd html 3//",
    "-//ietf//dtd html level 0//",
    "-//ietf//dtd html level 1//",
    "-//ietf//dtd html level 2//",
    "-//ietf//dtd html level 3//",
    "-//ietf//dtd html strict level 0//",
    "-//ietf//dtd html strict level 1//",
    "-//ietf//dtd html strict level 2//",
    "-//ietf//dtd html strict level 3//",
    "-//ietf//dtd html strict//",
    "-//ietf//dtd html//",
    "-//metrius//dtd metrius presentational//",
    "-//microsoft//dtd internet explorer 2.0 html strict//",
    "-//microsoft//dtd internet explorer 2.0 html//",
    "-//microsoft//dtd internet explorer 2.0 tables//",
    "-//microsoft//dtd internet explorer 3.0 html strict//",
    "-//microsoft//dtd internet explorer 3.0 html//",
    "-//microsoft//dtd internet explorer 3.0 tables//",
    "-//netscape comm. corp.//dtd html//",
    "-//netscape comm. corp.//dtd strict html//",
    "-//o'reilly and associates//dtd html 2.0//",
    "-//o'reilly and associates//dtd html extended 1.0//",
    "-//o'reilly and associates//dtd html extended relaxed 1.0//",
    "-//sq//dtd html 2.0 hotmetal + extensions//",
    "-//softquad software//dtd hotmetal pro 6.0::19990601::extensions to html 4.0//",
    "-//softquad//dtd hotmetal pro 4.0::19971010::extensions to html 4.0//",
    "-//spyglass//dtd html 2.0 extended//",
    "-//sun microsystems corp.//dtd hotjava html//",
    "-//sun microsystems corp.//dtd hotjava strict html//",
    "-//w3c//dtd html 3 1995-03-24//",
    "-//w3c//dtd html 3.2 draft//",
    "-//w3c//dtd html 3.2 final//",
    "-//w3c//dtd html 3.2//",
    "-//w3c//dtd html 3.2s draft//",
    "-//w3c//dtd html 4.0 frameset//",
    "-//w3c//dtd html 4.0 transitional//",
    "-//w3c//dtd html experimental 19960712//",
    "-//w3c//dtd html experimental 970421//",
    "-//w3c//dtd w3 html//",
    "-//w3o//dtd w3 html 3.0//",
    "-//webtechs//dtd mozilla html 2.0//",
    "-//webtechs//dtd mozilla html//",
];

/// The document's mode (§13.2.6.4.1). Only `<table>` in body consults it
/// during tree construction; layout consults it far more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentMode {
    #[default]
    NoQuirks,
    LimitedQuirks,
    Quirks,
}

/// §13.2.6.4.1, the DOCTYPE token rule in the "initial" insertion mode.
pub fn document_mode_for_doctype(
    name: Option<&str>,
    public_identifier: Option<&str>,
    system_identifier: Option<&str>,
    force_quirks: bool,
) -> DocumentMode {
    let public = public_identifier.map(str::to_ascii_lowercase);
    let system = system_identifier.map(str::to_ascii_lowercase);
    let public_starts = |prefix: &str| public.as_deref().is_some_and(|id| id.starts_with(prefix));
    let transitional_or_frameset_401 = public_starts("-//w3c//dtd html 4.01 frameset//")
        || public_starts("-//w3c//dtd html 4.01 transitional//");

    if force_quirks
        || name != Some("html")
        || matches!(
            public.as_deref(),
            Some(
                "-//w3o//dtd w3 html strict 3.0//en//"
                    | "-/w3c/dtd html 4.0 transitional/en"
                    | "html"
            )
        )
        || system.as_deref() == Some("http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd")
        || QUIRKS_PUBLIC_PREFIXES
            .iter()
            .any(|prefix| public_starts(prefix))
        || (system.is_none() && transitional_or_frameset_401)
    {
        DocumentMode::Quirks
    } else if public_starts("-//w3c//dtd xhtml 1.0 frameset//")
        || public_starts("-//w3c//dtd xhtml 1.0 transitional//")
        || (system.is_some() && transitional_or_frameset_401)
    {
        DocumentMode::LimitedQuirks
    } else {
        DocumentMode::NoQuirks
    }
}

/// §13.2.6.1 "adjust MathML attributes".
pub fn adjust_mathml_attribute(name: &str) -> Option<&'static str> {
    (name == "definitionurl").then_some("definitionURL")
}

/// §13.2.6.1 "adjust SVG attributes": the tokenizer lowercases attribute
/// names, and SVG's are camelCase.
pub fn adjust_svg_attribute(name: &str) -> Option<&'static str> {
    Some(match name {
        "attributename" => "attributeName",
        "attributetype" => "attributeType",
        "basefrequency" => "baseFrequency",
        "baseprofile" => "baseProfile",
        "calcmode" => "calcMode",
        "clippathunits" => "clipPathUnits",
        "diffuseconstant" => "diffuseConstant",
        "edgemode" => "edgeMode",
        "filterunits" => "filterUnits",
        "glyphref" => "glyphRef",
        "gradienttransform" => "gradientTransform",
        "gradientunits" => "gradientUnits",
        "kernelmatrix" => "kernelMatrix",
        "kernelunitlength" => "kernelUnitLength",
        "keypoints" => "keyPoints",
        "keysplines" => "keySplines",
        "keytimes" => "keyTimes",
        "lengthadjust" => "lengthAdjust",
        "limitingconeangle" => "limitingConeAngle",
        "markerheight" => "markerHeight",
        "markerunits" => "markerUnits",
        "markerwidth" => "markerWidth",
        "maskcontentunits" => "maskContentUnits",
        "maskunits" => "maskUnits",
        "numoctaves" => "numOctaves",
        "pathlength" => "pathLength",
        "patterncontentunits" => "patternContentUnits",
        "patterntransform" => "patternTransform",
        "patternunits" => "patternUnits",
        "pointsatx" => "pointsAtX",
        "pointsaty" => "pointsAtY",
        "pointsatz" => "pointsAtZ",
        "preservealpha" => "preserveAlpha",
        "preserveaspectratio" => "preserveAspectRatio",
        "primitiveunits" => "primitiveUnits",
        "refx" => "refX",
        "refy" => "refY",
        "repeatcount" => "repeatCount",
        "repeatdur" => "repeatDur",
        "requiredextensions" => "requiredExtensions",
        "requiredfeatures" => "requiredFeatures",
        "specularconstant" => "specularConstant",
        "specularexponent" => "specularExponent",
        "spreadmethod" => "spreadMethod",
        "startoffset" => "startOffset",
        "stddeviation" => "stdDeviation",
        "stitchtiles" => "stitchTiles",
        "surfacescale" => "surfaceScale",
        "systemlanguage" => "systemLanguage",
        "tablevalues" => "tableValues",
        "targetx" => "targetX",
        "targety" => "targetY",
        "textlength" => "textLength",
        "viewbox" => "viewBox",
        "viewtarget" => "viewTarget",
        "xchannelselector" => "xChannelSelector",
        "ychannelselector" => "yChannelSelector",
        "zoomandpan" => "zoomAndPan",
        _ => return None,
    })
}

/// §13.2.6.1 "adjust foreign attributes": `xlink:href` and friends become
/// namespaced attributes. `dom_core` has no attribute namespaces, so the
/// adjusted name is written the way the html5lib format prints it —
/// `"xlink href"` — which is what `html-parser` stores today.
pub fn adjust_foreign_attribute(name: &str) -> Option<&'static str> {
    Some(match name {
        "xlink:actuate" => "xlink actuate",
        "xlink:arcrole" => "xlink arcrole",
        "xlink:href" => "xlink href",
        "xlink:role" => "xlink role",
        "xlink:show" => "xlink show",
        "xlink:title" => "xlink title",
        "xlink:type" => "xlink type",
        "xml:lang" => "xml lang",
        "xml:space" => "xml space",
        "xmlns" => "xmlns",
        "xmlns:xlink" => "xmlns xlink",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html5_doctype_is_no_quirks() {
        assert_eq!(
            document_mode_for_doctype(Some("html"), None, None, false),
            DocumentMode::NoQuirks
        );
    }

    #[test]
    fn missing_or_foreign_doctype_names_are_quirks() {
        assert_eq!(
            document_mode_for_doctype(None, None, None, false),
            DocumentMode::Quirks
        );
        assert_eq!(
            document_mode_for_doctype(Some("svg"), None, None, false),
            DocumentMode::Quirks
        );
        assert_eq!(
            document_mode_for_doctype(Some("html"), None, None, true),
            DocumentMode::Quirks
        );
    }

    #[test]
    fn html_401_transitional_depends_on_the_system_identifier() {
        let public = Some("-//W3C//DTD HTML 4.01 Transitional//EN");
        assert_eq!(
            document_mode_for_doctype(Some("html"), public, None, false),
            DocumentMode::Quirks
        );
        assert_eq!(
            document_mode_for_doctype(
                Some("html"),
                public,
                Some("http://www.w3.org/TR/html4/loose.dtd"),
                false
            ),
            DocumentMode::LimitedQuirks
        );
    }

    #[test]
    fn listed_public_prefixes_are_case_insensitive() {
        assert_eq!(
            document_mode_for_doctype(Some("html"), Some("-//IETF//DTD HTML 2.0//EN"), None, false),
            DocumentMode::Quirks
        );
        assert_eq!(
            document_mode_for_doctype(
                Some("html"),
                Some("-//W3C//DTD XHTML 1.0 Transitional//EN"),
                None,
                false
            ),
            DocumentMode::LimitedQuirks
        );
    }

    #[test]
    fn attribute_adjustments() {
        assert_eq!(adjust_svg_attribute("viewbox"), Some("viewBox"));
        assert_eq!(adjust_svg_attribute("width"), None);
        assert_eq!(
            adjust_mathml_attribute("definitionurl"),
            Some("definitionURL")
        );
        assert_eq!(adjust_foreign_attribute("xlink:href"), Some("xlink href"));
        assert_eq!(adjust_foreign_attribute("href"), None);
    }

    #[test]
    fn categories() {
        assert!(is_special(Namespace::Html, "div"));
        assert!(!is_special(Namespace::Html, "span"));
        assert!(is_special(Namespace::Svg, "foreignObject"));
        assert!(!is_special(Namespace::Svg, "div"));
        assert!(is_formatting("nobr"));
        assert!(!is_formatting("span"));
        assert!(has_implied_end_tag("p"));
        assert!(!has_implied_end_tag("td"));
        assert!(has_implied_end_tag_thoroughly("td"));
    }
}
