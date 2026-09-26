//! §13.2.4.1 The insertion mode.
//!
//! Tree construction is a state machine whose state is the insertion mode.
//! Each token is handled by the rules of the current mode; a rule may switch
//! modes and ask for the same token to be *reprocessed* in the new one. A
//! plain document walks the first few in order:
//!
//! ```text
//!   Initial ──doctype──▶ BeforeHtml ──<html>──▶ BeforeHead ──<head>──▶ InHead
//!      ──</head>──▶ AfterHead ──<body>──▶ InBody ──</body>──▶ AfterBody
//!      ──</html>──▶ AfterAfterBody ──EOF──▶ stop
//! ```
//!
//! and every other mode is entered from `InBody` or `InHead` when a table,
//! template, frameset or raw-text element starts.

/// The specification's 21 insertion modes, in the specification's order.
///
/// Older editions had 23: "in select" and "in select in table" were removed
/// when `<select>` became customizable (2025), and select parsing moved into
/// the "in body" rules. The html5lib corpus follows the current rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}
