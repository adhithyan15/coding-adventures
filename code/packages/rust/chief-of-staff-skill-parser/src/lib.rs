//! Fail-closed parser for D18 Level 1 `SKILL.md` agents.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub use chief_of_staff_agent_manifest::{
    AgentManifest, Capability, ChannelAccess, MANIFEST_VERSION, MAX_ALLOWED_TOOLS,
    MAX_TOOL_CAPABILITIES,
};
use document_ast::{BlockNode, InlineNode, ListChildNode};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

/// Complete parsed Level 1 skill and its derived runtime plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSkill {
    /// Display title from the document H1.
    pub title: String,
    /// Original instructions after optional frontmatter removal.
    pub instructions: String,
    /// Generated typed agent manifest.
    pub manifest: AgentManifest,
    /// Sorted and deduplicated Deno permission arguments.
    pub deno_permissions: Vec<String>,
}

/// Stable classes of invalid `SKILL.md` input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkillParseError {
    /// Frontmatter opened but did not close.
    UnterminatedFrontmatter,
    /// A frontmatter line was not one supported key-value pair.
    InvalidFrontmatter(String),
    /// A frontmatter key appeared more than once.
    DuplicateFrontmatterKey(String),
    /// A frontmatter key is not in the bounded Level 1 contract.
    UnknownFrontmatterKey(String),
    /// No non-empty H1 title was present.
    MissingTitle,
    /// More than one non-empty H1 title was present.
    MultipleTitles,
    /// The inferred or explicit agent identifier is invalid.
    InvalidAgent,
    /// The inferred or explicit description violates schema bounds.
    InvalidDescription,
    /// The privilege tier is not zero through three.
    InvalidPrivilegeTier,
    /// The restart policy is not one of the schema values.
    InvalidRestartPolicy,
    /// A channel identifier is invalid or is both read and written.
    InvalidChannel(String),
    /// A per-channel message-schema version declaration is malformed or incomplete.
    InvalidMessageSchemaVersion(String),
    /// The required capability section is absent.
    MissingCapabilitiesSection,
    /// The capability section has no list.
    MissingCapabilitiesList,
    /// Capability declarations were split across repeated sections or lists.
    AmbiguousCapabilitiesSection,
    /// A capability bullet is malformed or outside the taxonomy.
    InvalidCapability(String),
    /// The required tool section is absent.
    MissingToolsSection,
    /// The tool section has no list.
    MissingToolsList,
    /// The tool section appears more than once, or holds more than one list.
    AmbiguousToolsSection,
    /// A tool bullet is not a namespaced D18D tool identifier.
    InvalidTool(String),
    /// The required tool-capability section is absent.
    MissingToolCapabilitiesSection,
    /// The tool-capability section has no list.
    MissingToolCapabilitiesList,
    /// The tool-capability section appears more than once.
    AmbiguousToolCapabilitiesSection,
    /// A tool-capability bullet is malformed.
    InvalidToolCapability(String),
    /// The same tool capability was declared more than once.
    DuplicateToolCapability(String),
    /// The same tool was declared more than once.
    DuplicateTool(String),
    /// The same capability was declared more than once.
    DuplicateCapability(String),
}

impl Display for SkillParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedFrontmatter => {
                formatter.write_str("unterminated SKILL.md frontmatter")
            }
            Self::InvalidFrontmatter(value) => {
                write!(formatter, "invalid SKILL.md frontmatter: {value}")
            }
            Self::DuplicateFrontmatterKey(value) => {
                write!(formatter, "duplicate SKILL.md frontmatter key: {value}")
            }
            Self::UnknownFrontmatterKey(value) => {
                write!(formatter, "unknown SKILL.md frontmatter key: {value}")
            }
            Self::MissingTitle => formatter.write_str("SKILL.md requires one H1 title"),
            Self::MultipleTitles => formatter.write_str("SKILL.md allows only one H1 title"),
            Self::InvalidAgent => formatter.write_str("SKILL.md agent identifier is invalid"),
            Self::InvalidDescription => {
                formatter.write_str("SKILL.md description must contain 10 to 200 characters")
            }
            Self::InvalidPrivilegeTier => {
                formatter.write_str("SKILL.md privilege_tier must be 0 through 3")
            }
            Self::InvalidRestartPolicy => formatter.write_str("SKILL.md restart_policy is invalid"),
            Self::InvalidChannel(value) => write!(formatter, "invalid SKILL.md channel: {value}"),
            Self::InvalidMessageSchemaVersion(value) => {
                write!(
                    formatter,
                    "invalid SKILL.md message schema version: {value}"
                )
            }
            Self::MissingCapabilitiesSection => {
                formatter.write_str("SKILL.md requires a Capabilities needed section")
            }
            Self::MissingCapabilitiesList => {
                formatter.write_str("SKILL.md capabilities section requires a list")
            }
            Self::AmbiguousCapabilitiesSection => {
                formatter.write_str("SKILL.md capabilities section is ambiguous")
            }
            Self::MissingToolsSection => {
                formatter.write_str("SKILL.md requires a Tools needed section")
            }
            Self::MissingToolsList => formatter.write_str("SKILL.md tools section requires a list"),
            Self::AmbiguousToolsSection => {
                formatter.write_str("SKILL.md tools section is ambiguous")
            }
            Self::InvalidTool(value) => {
                write!(formatter, "invalid SKILL.md tool: {value}")
            }
            Self::MissingToolCapabilitiesSection => {
                formatter.write_str("SKILL.md requires a Tool capabilities needed section")
            }
            Self::MissingToolCapabilitiesList => {
                formatter.write_str("SKILL.md tool capabilities section requires a list")
            }
            Self::AmbiguousToolCapabilitiesSection => {
                formatter.write_str("SKILL.md tool capabilities section is ambiguous")
            }
            Self::InvalidToolCapability(value) => {
                write!(formatter, "invalid SKILL.md tool capability: {value}")
            }
            Self::DuplicateToolCapability(value) => {
                write!(formatter, "duplicate SKILL.md tool capability: {value}")
            }
            Self::DuplicateTool(value) => {
                write!(formatter, "duplicate SKILL.md tool: {value}")
            }
            Self::InvalidCapability(value) => {
                write!(formatter, "invalid SKILL.md capability: {value}")
            }
            Self::DuplicateCapability(value) => {
                write!(formatter, "duplicate SKILL.md capability: {value}")
            }
        }
    }
}

impl std::error::Error for SkillParseError {}

/// Parse one caller-provided Level 1 `SKILL.md` document without OS access.
pub fn parse_skill(source: &str) -> Result<ParsedSkill, SkillParseError> {
    let normalized = source.replace("\r\n", "\n");
    let (metadata, body) = split_frontmatter(&normalized)?;
    let document = commonmark_parser::parse(&body);
    let titles = document
        .children
        .iter()
        .filter_map(|node| match node {
            BlockNode::Heading(heading) if heading.level == 1 => {
                Some(inline_text(&heading.children))
            }
            _ => None,
        })
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    let title = match titles.as_slice() {
        [] => return Err(SkillParseError::MissingTitle),
        [title] => title.clone(),
        _ => return Err(SkillParseError::MultipleTitles),
    };
    let inferred_description = first_description(&document.children);
    let agent = metadata
        .get("agent")
        .cloned()
        .unwrap_or_else(|| slugify(&title));
    if !valid_identifier(&agent) || agent.len() > 64 {
        return Err(SkillParseError::InvalidAgent);
    }
    let description = metadata
        .get("description")
        .cloned()
        .or(inferred_description)
        .ok_or(SkillParseError::InvalidDescription)?;
    if !(10..=200).contains(&description.chars().count()) {
        return Err(SkillParseError::InvalidDescription);
    }
    let privilege_tier = metadata
        .get("privilege_tier")
        .map(|value| {
            value
                .parse::<u8>()
                .map_err(|_| SkillParseError::InvalidPrivilegeTier)
        })
        .transpose()?
        .unwrap_or(0);
    if privilege_tier > 3 {
        return Err(SkillParseError::InvalidPrivilegeTier);
    }
    let restart_policy = metadata
        .get("restart_policy")
        .cloned()
        .unwrap_or_else(|| "on-failure".to_string());
    if !matches!(restart_policy.as_str(), "always" | "on-failure" | "never") {
        return Err(SkillParseError::InvalidRestartPolicy);
    }
    let channels = parse_channels(&metadata)?;
    let message_schema_versions = parse_message_schema_versions(&metadata, &channels)?;
    let capabilities = parse_capabilities(&document.children, &agent)?;
    let allowed_tools = parse_allowed_tools(&document.children)?;
    let tool_capabilities = parse_tool_capabilities(&document.children)?;
    let deno_permissions = deno_permissions(&capabilities);
    let manifest = AgentManifest {
        version: MANIFEST_VERSION,
        agent: agent.clone(),
        description,
        privilege_tier,
        channels,
        message_schema_versions,
        vault_access: None,
        capabilities,
        allowed_tools,
        tool_capabilities,
        restart_policy,
        justification: format!(
            "Level 1 agent {agent} requests only the access declared in its SKILL.md."
        ),
    };
    Ok(ParsedSkill {
        title: title.trim().to_string(),
        instructions: body,
        manifest,
        deno_permissions,
    })
}

fn split_frontmatter(source: &str) -> Result<(BTreeMap<String, String>, String), SkillParseError> {
    let mut metadata = BTreeMap::new();
    if !source.starts_with("---\n") {
        return Ok((metadata, source.to_string()));
    }
    let remainder = &source[4..];
    let end = remainder
        .find("\n---\n")
        .ok_or(SkillParseError::UnterminatedFrontmatter)?;
    for raw in remainder[..end].lines() {
        let (key, value) = raw
            .split_once(':')
            .ok_or_else(|| SkillParseError::InvalidFrontmatter(raw.to_string()))?;
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            return Err(SkillParseError::InvalidFrontmatter(raw.to_string()));
        }
        if !matches!(
            key,
            "agent"
                | "description"
                | "privilege_tier"
                | "reads"
                | "writes"
                | "message_schema_versions"
                | "restart_policy"
        ) {
            return Err(SkillParseError::UnknownFrontmatterKey(key.to_string()));
        }
        if metadata.insert(key.to_string(), unquote(value)).is_some() {
            return Err(SkillParseError::DuplicateFrontmatterKey(key.to_string()));
        }
    }
    Ok((metadata, remainder[end + 5..].to_string()))
}

fn unquote(value: &str) -> String {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn first_description(nodes: &[BlockNode]) -> Option<String> {
    let mut saw_title = false;
    for node in nodes {
        match node {
            BlockNode::Heading(heading) if heading.level == 1 => saw_title = true,
            BlockNode::Heading(_) if saw_title => return None,
            BlockNode::Paragraph(paragraph) if saw_title => {
                let value = inline_text(&paragraph.children);
                if !value.trim().is_empty() {
                    return Some(value.trim().to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_channels(metadata: &BTreeMap<String, String>) -> Result<ChannelAccess, SkillParseError> {
    let mut reads = parse_list(metadata.get("reads"))?;
    let mut writes = parse_list(metadata.get("writes"))?;
    reads.sort();
    writes.sort();
    if reads.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SkillParseError::InvalidChannel(
            reads
                .windows(2)
                .find(|pair| pair[0] == pair[1])
                .map_or_else(String::new, |pair| pair[0].clone()),
        ));
    }
    if writes.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SkillParseError::InvalidChannel(
            writes
                .windows(2)
                .find(|pair| pair[0] == pair[1])
                .map_or_else(String::new, |pair| pair[0].clone()),
        ));
    }
    for channel in reads.iter().chain(&writes) {
        if !valid_identifier(channel) {
            return Err(SkillParseError::InvalidChannel(channel.clone()));
        }
    }
    if let Some(channel) = reads.iter().find(|channel| writes.contains(channel)) {
        return Err(SkillParseError::InvalidChannel(channel.clone()));
    }
    Ok(ChannelAccess { reads, writes })
}

fn parse_message_schema_versions(
    metadata: &BTreeMap<String, String>,
    channels: &ChannelAccess,
) -> Result<BTreeMap<String, u32>, SkillParseError> {
    let declared_channels = channels
        .reads
        .iter()
        .chain(&channels.writes)
        .cloned()
        .collect::<BTreeSet<_>>();
    let Some(raw) = metadata.get("message_schema_versions") else {
        return Ok(declared_channels
            .into_iter()
            .map(|channel| (channel, 1))
            .collect());
    };
    let mut versions = BTreeMap::new();
    for declaration in parse_list(Some(raw))? {
        let (channel, version) = declaration
            .split_once('=')
            .ok_or_else(|| SkillParseError::InvalidMessageSchemaVersion(declaration.clone()))?;
        if !valid_identifier(channel) || !declared_channels.contains(channel) {
            return Err(SkillParseError::InvalidMessageSchemaVersion(declaration));
        }
        let version = version
            .parse::<u32>()
            .map_err(|_| SkillParseError::InvalidMessageSchemaVersion(declaration.clone()))?;
        if version == 0 || versions.insert(channel.to_string(), version).is_some() {
            return Err(SkillParseError::InvalidMessageSchemaVersion(declaration));
        }
    }
    if versions.keys().collect::<BTreeSet<_>>() != declared_channels.iter().collect::<BTreeSet<_>>()
    {
        return Err(SkillParseError::InvalidMessageSchemaVersion(raw.clone()));
    }
    Ok(versions)
}

fn parse_list(value: Option<&String>) -> Result<Vec<String>, SkillParseError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if !(value.starts_with('[') && value.ends_with(']')) {
        return Err(SkillParseError::InvalidFrontmatter(value.clone()));
    }
    let inner = &value[1..value.len() - 1];
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(inner.split(',').map(|part| unquote(part.trim())).collect())
}

/// Parse the required `## Tools needed` section into sorted D18D tool ids.
///
/// Required rather than optional, and `- none` is the way to say "calls no
/// tools" -- the same shape `## Capabilities needed` already uses. Manifest
/// schema v3 requires `allowed_tools` precisely so that "calls no tools" is
/// declared instead of defaulted into; letting an absent section mean an empty
/// list here would put the default back one layer up and undo that.
/// Parse the required `## Tool capabilities needed` section.
///
/// These are D18D tool-capability names matched against a `ToolDefinition`'s
/// `required_capabilities` (`smart_home`, `smart_home.events`) -- a different
/// namespace from both the operating-system capabilities above and the tool
/// identifiers below. Single-segment names are legal here and are not legal as
/// tool ids.
fn parse_tool_capabilities(nodes: &[BlockNode]) -> Result<Vec<String>, SkillParseError> {
    let items = section_items(
        nodes,
        "tool capabilities needed",
        SkillParseError::MissingToolCapabilitiesSection,
        SkillParseError::MissingToolCapabilitiesList,
        SkillParseError::AmbiguousToolCapabilitiesSection,
        SkillParseError::InvalidToolCapability,
    )?;
    let Some(items) = items else {
        return Ok(Vec::new());
    };
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for value in items {
        // Colon-delimited scope (`smart_home:read`), matching
        // `chief-of-staff-host-runtime`'s `validate_capability`. Note the
        // separator differs from a tool identifier's `.` -- these are two
        // namespaces that look alike and are not interchangeable.
        if !(1..=128).contains(&value.len()) || !valid_capability_scope(&value) {
            return Err(SkillParseError::InvalidToolCapability(value));
        }
        if !seen.insert(value.clone()) {
            return Err(SkillParseError::DuplicateToolCapability(value));
        }
        if names.len() >= MAX_TOOL_CAPABILITIES {
            return Err(SkillParseError::InvalidToolCapability(value));
        }
        names.push(value);
    }
    names.sort();
    Ok(names)
}

/// Read one required section's literal bullet texts.
///
/// `None` means the list was exactly `- none`. Shared so a third section does
/// not copy the scan a third time -- the tools section already showed that the
/// copied shape carries a copied bug.
fn section_items(
    nodes: &[BlockNode],
    heading: &str,
    missing_section: SkillParseError,
    missing_list: SkillParseError,
    ambiguous: SkillParseError,
    invalid: fn(String) -> SkillParseError,
) -> Result<Option<Vec<String>>, SkillParseError> {
    if count_headings(nodes, heading) > 1 {
        return Err(ambiguous);
    }
    let sections = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            BlockNode::Heading(node) if node.level == 2 => literal_inline_text(&node.children)
                .is_some_and(|text| text.trim().eq_ignore_ascii_case(heading))
                .then_some(index),
            _ => None,
        })
        .collect::<Vec<_>>();
    let section = match sections.as_slice() {
        [] => return Err(missing_section),
        [section] => *section,
        _ => return Err(ambiguous),
    };
    let lists = nodes[section + 1..]
        .iter()
        .take_while(|node| !matches!(node, BlockNode::Heading(_)))
        .filter_map(|node| match node {
            BlockNode::List(list) => Some(list),
            _ => None,
        })
        .collect::<Vec<_>>();
    let list = match lists.as_slice() {
        [] => return Err(missing_list),
        [list] => *list,
        _ => return Err(ambiguous),
    };
    let mut items = Vec::new();
    for child in &list.children {
        let blocks = match child {
            ListChildNode::ListItem(item) => &item.children,
            ListChildNode::TaskItem(item) => &item.children,
        };
        let Some(value) = literal_item_text(blocks).map(|text| text.trim().to_string()) else {
            return Err(invalid(block_text(blocks).trim().to_string()));
        };
        if value.eq_ignore_ascii_case("none") {
            if list.children.len() != 1 {
                return Err(invalid(value));
            }
            return Ok(None);
        }
        items.push(value);
    }
    Ok(Some(items))
}

fn count_headings(nodes: &[BlockNode], heading: &str) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            BlockNode::Heading(value) if value.level == 2 => usize::from(
                literal_inline_text(&value.children)
                    .is_some_and(|text| text.trim().eq_ignore_ascii_case(heading)),
            ),
            BlockNode::Blockquote(value) => count_headings(&value.children, heading),
            BlockNode::List(value) => value
                .children
                .iter()
                .map(|child| match child {
                    ListChildNode::ListItem(item) => count_headings(&item.children, heading),
                    ListChildNode::TaskItem(item) => count_headings(&item.children, heading),
                })
                .sum(),
            _ => 0,
        })
        .sum()
}

fn valid_capability_scope(value: &str) -> bool {
    !value.is_empty()
        && value.split(':').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        })
}

fn parse_allowed_tools(nodes: &[BlockNode]) -> Result<Vec<String>, SkillParseError> {
    let Some(items) = section_items(
        nodes,
        "tools needed",
        SkillParseError::MissingToolsSection,
        SkillParseError::MissingToolsList,
        SkillParseError::AmbiguousToolsSection,
        SkillParseError::InvalidTool,
    )?
    else {
        return Ok(Vec::new());
    };
    let mut tools = Vec::new();
    let mut seen = BTreeSet::new();
    for value in items {
        if !valid_tool_id(&value) || !(3..=128).contains(&value.len()) {
            return Err(SkillParseError::InvalidTool(value));
        }
        if !seen.insert(value.clone()) {
            return Err(SkillParseError::DuplicateTool(value));
        }
        if tools.len() >= MAX_ALLOWED_TOOLS {
            return Err(SkillParseError::InvalidTool(value));
        }
        tools.push(value);
    }
    // The manifest stores tools sorted and validates that invariant, so two
    // skills listing the same tools generate byte-identical manifests.
    tools.sort();
    Ok(tools)
}

/// A D18D tool identifier: two or more dot-separated segments, each starting
/// with a lowercase letter. A bare namespace is rejected -- it names no tool,
/// and would invite prefix matching in the broker.
fn valid_tool_id(value: &str) -> bool {
    let mut count = 0usize;
    for segment in value.split('.') {
        count += 1;
        if !segment
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            return false;
        }
    }
    count >= 2
}

fn parse_capabilities(
    nodes: &[BlockNode],
    agent: &str,
) -> Result<Vec<Capability>, SkillParseError> {
    let Some(items) = section_items(
        nodes,
        "capabilities needed",
        SkillParseError::MissingCapabilitiesSection,
        SkillParseError::MissingCapabilitiesList,
        SkillParseError::AmbiguousCapabilitiesSection,
        SkillParseError::InvalidCapability,
    )?
    else {
        return Ok(Vec::new());
    };
    let mut capabilities = Vec::new();
    let mut seen = BTreeSet::new();
    for value in items {
        let (specification, justification) = value
            .split_once(" | ")
            .map_or((value.as_str(), None), |(left, right)| {
                (left, Some(right.trim()))
            });
        let mut parts = specification.splitn(3, ':');
        let category = parts.next().unwrap_or_default().trim();
        let action = parts.next().unwrap_or_default().trim();
        let target = parts.next().unwrap_or_default().trim();
        if !valid_capability(category, action)
            || target.is_empty()
            || target
                .bytes()
                .any(|byte| byte == b',' || byte.is_ascii_control())
        {
            return Err(SkillParseError::InvalidCapability(value));
        }
        let key = format!("{category}:{action}:{target}");
        if !seen.insert(key.clone()) {
            return Err(SkillParseError::DuplicateCapability(key));
        }
        let justification = match justification {
            Some(text) if text.chars().count() < 10 => {
                return Err(SkillParseError::InvalidCapability(value));
            }
            Some(text) => text.to_string(),
            None => format!("Declared by the {agent} SKILL.md agent."),
        };
        capabilities.push(Capability {
            category: category.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            justification,
        });
    }
    capabilities.sort();
    Ok(capabilities)
}

fn valid_capability(category: &str, action: &str) -> bool {
    matches!(
        (category, action),
        ("fs", "read" | "write" | "create" | "delete" | "list")
            | ("net", "connect" | "listen" | "dns")
            | ("proc", "exec" | "fork" | "signal")
            | ("env", "read" | "write")
            | ("ffi", "call" | "load")
            | ("time", "read" | "sleep")
            | ("stdin", "read")
            | ("stdout", "write")
    )
}

fn deno_permissions(capabilities: &[Capability]) -> Vec<String> {
    let mut groups: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for capability in capabilities {
        let flag = match (capability.category.as_str(), capability.action.as_str()) {
            ("fs", "read" | "list") => Some("--allow-read"),
            ("fs", _) => Some("--allow-write"),
            ("net", _) => Some("--allow-net"),
            ("proc", _) => Some("--allow-run"),
            ("env", _) => Some("--allow-env"),
            ("ffi", _) => Some("--allow-ffi"),
            _ => None,
        };
        if let Some(flag) = flag {
            groups.entry(flag).or_default().insert(&capability.target);
        }
    }
    groups
        .into_iter()
        .map(|(flag, targets)| {
            format!(
                "{flag}={}",
                targets.into_iter().collect::<Vec<_>>().join(",")
            )
        })
        .collect()
}

fn valid_identifier(value: &str) -> bool {
    value.len() >= 2
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.ends_with('-')
}

fn slugify(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.is_empty() && !output.ends_with('-') {
            output.push('-');
        }
    }
    output.trim_end_matches('-').to_string()
}

fn block_text(nodes: &[BlockNode]) -> String {
    nodes
        .iter()
        .map(|node| match node {
            BlockNode::Paragraph(value) => inline_text(&value.children),
            BlockNode::Heading(value) => inline_text(&value.children),
            BlockNode::List(value) => value
                .children
                .iter()
                .map(|child| match child {
                    ListChildNode::ListItem(item) => block_text(&item.children),
                    ListChildNode::TaskItem(item) => block_text(&item.children),
                })
                .collect::<Vec<_>>()
                .join(" "),
            BlockNode::Blockquote(value) => block_text(&value.children),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract inline text ONLY when every node is literal, else `None`.
///
/// `inline_text` is for prose. It is unsafe for anything that becomes an
/// authorization decision, because it resolves an image to its **alt text**,
/// drops raw inline HTML while keeping the text around it, and concatenates
/// with no separator. Each of those lets a tool identifier be authorized that
/// a reader cannot see:
///
/// ```text
///   - ![&#97;dmin&#46;exec&#95;all](pixel.png)     -> "admin.exec_all"
///   - context.read<span hidden>_write_all</span>  -> "context.read_write_all"
///   - artifact<span></span>.<span></span>write    -> "artifact.write"
/// ```
///
/// The first renders as a picture and the second renders as `context.read`, so
/// the rendered document and a source diff are defeated at the same time.
///
/// Only `Text` and `CodeSpan` survive here. A code span is allowed because it
/// renders as its own literal content; everything else -- images, raw HTML,
/// links, autolinks, emphasis, breaks -- is rejected rather than flattened.
///
/// One residual is accepted knowingly: HTML entity references decode into
/// `Text` before this sees them, so `admin&period;exec_all` still yields
/// `admin.exec_all`. That case renders faithfully, so a reader of the rendered
/// document sees the real identifier; only a raw-bytes grep is fooled. The
/// answer to that is to scan the signed manifest's `allowed_tools` rather than
/// the Markdown, which is what declaring it in the manifest is for.
fn literal_inline_text(nodes: &[InlineNode]) -> Option<String> {
    let mut text = String::new();
    for node in nodes {
        match node {
            InlineNode::Text(value) => text.push_str(&value.value),
            InlineNode::CodeSpan(value) => text.push_str(&value.value),
            _ => return None,
        }
    }
    Some(text)
}

/// A list item's literal text, if it is exactly one paragraph of literal inlines.
fn literal_item_text(blocks: &[BlockNode]) -> Option<String> {
    match blocks {
        [BlockNode::Paragraph(paragraph)] => literal_inline_text(&paragraph.children),
        _ => None,
    }
}

fn inline_text(nodes: &[InlineNode]) -> String {
    nodes
        .iter()
        .map(|node| match node {
            InlineNode::Text(value) => value.value.clone(),
            InlineNode::Emphasis(value) => inline_text(&value.children),
            InlineNode::Strong(value) => inline_text(&value.children),
            InlineNode::Strikethrough(value) => inline_text(&value.children),
            InlineNode::CodeSpan(value) => value.value.clone(),
            InlineNode::Link(value) => inline_text(&value.children),
            InlineNode::Image(value) => value.alt.clone(),
            InlineNode::Autolink(value) => value.destination.clone(),
            InlineNode::HardBreak(_) | InlineNode::SoftBreak(_) => " ".to_string(),
            InlineNode::RawInline(_) => String::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use document_ast::{
        AutolinkNode, BlockquoteNode, CodeSpanNode, EmphasisNode, HardBreakNode, HeadingNode,
        ImageNode, LinkNode, ListItemNode, ListNode, ParagraphNode, RawInlineNode, SoftBreakNode,
        StrikethroughNode, StrongNode, TaskItemNode, TextNode,
    };

    const MINIMAL: &str = "# Weather Reporter\n\nYou are a weather reporting agent for friendly local forecasts.\n\n## Capabilities needed\n- net:connect:api.weather.gov:443\n\n## Output format\nBe brief.\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";

    #[test]
    fn issue_example_infers_safe_defaults_and_permissions() {
        let skill = parse_skill(MINIMAL).unwrap();
        assert_eq!(skill.title, "Weather Reporter");
        assert_eq!(skill.manifest.agent, "weather-reporter");
        assert_eq!(skill.manifest.version, MANIFEST_VERSION);
        assert_eq!(skill.manifest.privilege_tier, 0);
        assert_eq!(skill.manifest.channels, ChannelAccess::default());
        assert!(skill.manifest.message_schema_versions.is_empty());
        assert_eq!(skill.manifest.restart_policy, "on-failure");
        assert_eq!(skill.deno_permissions, ["--allow-net=api.weather.gov:443"]);
        assert!(skill.instructions.starts_with("# Weather Reporter"));
    }

    #[test]
    fn frontmatter_overrides_metadata_and_sorts_runtime_access() {
        let source = "---\nagent: 'forecast-agent'\ndescription: \"Produces precise forecasts for subscribed cities.\"\nprivilege_tier: 1\nreads: [weather-requests]\nwrites: [weather-reports]\nmessage_schema_versions: [weather-requests=1, weather-reports=2]\nrestart_policy: always\n---\n# Forecast\n\nIgnored description because frontmatter wins.\n\n## Capabilities needed\n- fs:write:/tmp/cache | Stores short-lived forecast cache data.\n- net:connect:api.weather.gov:443\n- fs:read:/tmp/cache\n- net:dns:api.weather.gov\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";
        let skill = parse_skill(source).unwrap();
        assert_eq!(skill.manifest.agent, "forecast-agent");
        assert_eq!(skill.manifest.channels.reads, ["weather-requests"]);
        assert_eq!(skill.manifest.channels.writes, ["weather-reports"]);
        assert_eq!(
            skill.manifest.message_schema_version("weather-requests"),
            Some(1)
        );
        assert_eq!(
            skill.manifest.message_schema_version("weather-reports"),
            Some(2)
        );
        assert_eq!(
            skill.deno_permissions,
            [
                "--allow-net=api.weather.gov,api.weather.gov:443",
                "--allow-read=/tmp/cache",
                "--allow-write=/tmp/cache",
            ]
        );
        assert!(!skill.instructions.contains("privilege_tier"));
        let json = skill.manifest.to_json().unwrap();
        assert!(json.contains("\"target\": \"/tmp/cache\""));
        assert!(json.contains("\"reads\": {\n      \"weather-requests\": 1\n    }"));
        assert!(json.contains("\"weather-reports\": 2"));
    }

    #[test]
    fn explicit_none_produces_empty_profile_and_schema_json() {
        let source = "# Greeter\n\nGreets the user without external operating-system access.\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";
        let skill = parse_skill(source).unwrap();
        assert!(skill.manifest.capabilities.is_empty());
        assert!(skill.deno_permissions.is_empty());
        let json = skill.manifest.to_json().unwrap();
        assert!(json.contains("\"agent\": \"greeter\""));
        assert!(json.contains("\"capabilities\": []"));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn taxonomy_permissions_cover_every_mapped_category() {
        let source = "# Operator\n\nExercises every supported Deno permission mapping safely.\n\n## Capabilities needed\n- proc:exec:git\n- env:read:HOME\n- ffi:load:libdemo\n- stdin:read:stdin\n- stdout:write:stdout\n- time:sleep:clock\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";
        let skill = parse_skill(source).unwrap();
        assert_eq!(
            skill.deno_permissions,
            ["--allow-env=HOME", "--allow-ffi=libdemo", "--allow-run=git",]
        );
    }

    #[test]
    fn rejects_frontmatter_failures() {
        assert!(matches!(
            parse_skill("---\nagent: a\n"),
            Err(SkillParseError::UnterminatedFrontmatter)
        ));
        assert!(matches!(
            parse_skill("---\nunknown: x\n---\n# A\n"),
            Err(SkillParseError::UnknownFrontmatterKey(_))
        ));
        assert!(matches!(
            parse_skill("---\nagent: one\nagent: two\n---\n# A\n"),
            Err(SkillParseError::DuplicateFrontmatterKey(_))
        ));
        assert!(matches!(
            parse_skill("---\nagent one\n---\n# A\n"),
            Err(SkillParseError::InvalidFrontmatter(_))
        ));
        assert!(matches!(
            parse_skill("---\nagent:\n---\n# A\n"),
            Err(SkillParseError::InvalidFrontmatter(_))
        ));
    }

    #[test]
    fn rejects_missing_structure_and_bad_metadata() {
        assert_eq!(parse_skill("paragraph"), Err(SkillParseError::MissingTitle));
        assert_eq!(
            parse_skill("# X\n\nLong enough description.\n"),
            Err(SkillParseError::InvalidAgent)
        );
        assert_eq!(
            parse_skill("# Valid Name\n\nshort\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n"),
            Err(SkillParseError::InvalidDescription)
        );
        let bad_tier = "---\nprivilege_tier: 4\n---\n# Valid Name\n\nLong enough description.\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";
        assert_eq!(
            parse_skill(bad_tier),
            Err(SkillParseError::InvalidPrivilegeTier)
        );
    }

    #[test]
    fn rejects_invalid_channels_and_restart_policy() {
        let both = "---\nreads: [same-channel]\nwrites: [same-channel]\n---\n# Valid Agent\n\nLong enough description.\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";
        assert_eq!(
            parse_skill(both),
            Err(SkillParseError::InvalidChannel("same-channel".to_string()))
        );
        let restart = both.replace(
            "reads: [same-channel]\nwrites: [same-channel]",
            "restart_policy: sometimes",
        );
        assert_eq!(
            parse_skill(&restart),
            Err(SkillParseError::InvalidRestartPolicy)
        );
        let duplicate = both.replace(
            "reads: [same-channel]\nwrites: [same-channel]",
            "reads: [one-channel, one-channel]",
        );
        assert_eq!(
            parse_skill(&duplicate),
            Err(SkillParseError::InvalidChannel("one-channel".to_string()))
        );
        let duplicate_write = both.replace(
            "reads: [same-channel]\nwrites: [same-channel]",
            "writes: [one-channel, one-channel]",
        );
        assert_eq!(
            parse_skill(&duplicate_write),
            Err(SkillParseError::InvalidChannel("one-channel".to_string()))
        );
        let malformed = both.replace(
            "reads: [same-channel]\nwrites: [same-channel]",
            "reads: not-a-list",
        );
        assert!(matches!(
            parse_skill(&malformed),
            Err(SkillParseError::InvalidFrontmatter(_))
        ));
        let invalid = both.replace(
            "reads: [same-channel]\nwrites: [same-channel]",
            "reads: [Bad_Channel]",
        );
        assert_eq!(
            parse_skill(&invalid),
            Err(SkillParseError::InvalidChannel("Bad_Channel".to_string()))
        );
    }

    #[test]
    fn message_schema_versions_default_to_one_and_fail_closed() {
        let base = "---\nreads: [request-channel]\nwrites: [response-channel]\n---\n# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";
        let skill = parse_skill(base).unwrap();
        assert_eq!(
            skill.manifest.message_schema_version("request-channel"),
            Some(1)
        );
        assert_eq!(
            skill.manifest.message_schema_version("response-channel"),
            Some(1)
        );

        for declarations in [
            "[request-channel=1]",
            "[request-channel=0, response-channel=1]",
            "[request-channel=x, response-channel=1]",
            "[request-channel=1, extra-channel=1]",
            "[request-channel=1, request-channel=2, response-channel=1]",
        ] {
            let source = base.replace(
                "writes: [response-channel]",
                &format!("writes: [response-channel]\nmessage_schema_versions: {declarations}"),
            );
            assert!(matches!(
                parse_skill(&source),
                Err(SkillParseError::InvalidMessageSchemaVersion(_))
            ));
        }
    }

    #[test]
    fn declares_tools_and_emits_a_current_schema_manifest() {
        let source = "# Weather Reporter\n\nYou are a weather reporting agent for friendly local forecasts.\n\n## Capabilities needed\n- none\n\n## Tools needed\n- context.append_entry\n- artifact.write\n- artifact.create_v2\n\n## Tool capabilities needed\n- smart_home:read\n- smart_home:write\n";
        let skill = parse_skill(source).unwrap();
        assert_eq!(skill.manifest.version, MANIFEST_VERSION);
        // Sorted here, not in SKILL.md order, so two skills listing the same
        // tools generate byte-identical manifests.
        assert_eq!(
            skill.manifest.allowed_tools,
            vec![
                "artifact.create_v2".to_string(),
                "artifact.write".to_string(),
                "context.append_entry".to_string(),
            ]
        );
        // The generated manifest must satisfy the codec that will verify it.
        let json = skill.manifest.to_json().unwrap();
        assert!(json.contains("\"allowed_tools\""));
    }

    #[test]
    fn explicit_none_declares_an_empty_tool_surface() {
        let source = "# Greeter\n\nGreets the user without external operating-system access.\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n";
        let skill = parse_skill(source).unwrap();
        assert_eq!(skill.manifest.version, MANIFEST_VERSION);
        assert!(skill.manifest.allowed_tools.is_empty());
        // v3 requires the field, so an empty surface still renders it.
        assert!(skill.manifest.to_json().unwrap().contains("allowed_tools"));
    }

    #[test]
    fn rejects_tool_section_failures() {
        let head = "# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n- none\n";
        let with = |tools: &str| {
            format!("{head}\n## Tools needed\n{tools}\n## Tool capabilities needed\n- none\n")
        };

        // Required, not optional. An absent section would put "calls no tools"
        // back to a default, which is what manifest v3 exists to prevent.
        assert_eq!(parse_skill(head), Err(SkillParseError::MissingToolsSection));
        assert_eq!(
            parse_skill(&format!(
                "{head}\n## Tools needed\n\n## Tool capabilities needed\n- none\n"
            )),
            Err(SkillParseError::MissingToolsList)
        );
        assert_eq!(
            parse_skill(&format!(
                "{head}\n## Tools needed\n- artifact.write\n\n## Tools needed\n- a.b\n\n## Tool capabilities needed\n- none\n"
            )),
            Err(SkillParseError::AmbiguousToolsSection)
        );
        assert!(matches!(
            parse_skill(&with("- artifact.write\n- artifact.write\n")),
            Err(SkillParseError::DuplicateTool(_))
        ));
        assert!(matches!(
            parse_skill(&with("- none\n- artifact.write\n")),
            Err(SkillParseError::InvalidTool(_))
        ));
        // A bare namespace names no tool and would invite prefix matching.
        for bad in [
            "- artifact\n",
            "- Artifact.create\n",
            "- artifact..create\n",
            "- .create\n",
            "- artifact.\n",
            "- artifact.create!\n",
            "- 2artifact.create\n",
            "- ab\n",
        ] {
            assert!(
                matches!(
                    parse_skill(&with(bad)),
                    Err(SkillParseError::InvalidTool(_))
                ),
                "should have rejected {bad:?}"
            );
        }
    }

    #[test]
    fn a_tool_a_reader_cannot_see_is_never_authorized() {
        // Security review, 2026-09-02. `block_text` resolved an image to its
        // ALT TEXT, dropped raw inline HTML while keeping surrounding text, and
        // concatenated with no separator. Each case below produced a signed
        // `allowed_tools` entry that the rendered document does not show.
        let head = "# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n- none\n";
        let with = |tools: &str| {
            format!("{head}\n## Tools needed\n{tools}\n## Tool capabilities needed\n- none\n")
        };

        for attack in [
            // Renders as a picture. The identifier is entity-encoded, so it is
            // absent from the source bytes AND invisible when rendered.
            "- ![&#97;dmin&#46;exec&#95;all](https://example.com/pixel.png)\n",
            // Renders as `context.read`; the suffix is hidden HTML.
            "- context.read<span style=\"display:none\">_write_all</span>\n",
            // Empty tags fuse fragments into an identifier present nowhere.
            "- artifact<span></span>.<span></span>write\n",
            // Alt text concatenated with following text.
            "- ![admin.](x.png)exec_all\n",
            // A link renders as its label, not necessarily its target.
            "- [artifact.write](https://evil.example)\n",
            "- <https://evil.example/artifact.write>\n",
        ] {
            assert!(
                matches!(
                    parse_skill(&with(attack)),
                    Err(SkillParseError::InvalidTool(_))
                ),
                "should have rejected non-literal bullet {attack:?}"
            );
        }

        // A code span renders as its own literal content, so it stays legal.
        let skill = parse_skill(&with("- `artifact.write`\n")).unwrap();
        assert_eq!(skill.manifest.allowed_tools, vec!["artifact.write"]);
    }

    #[test]
    fn a_capability_a_reader_cannot_see_is_never_granted() {
        // The identical hole existed in the capabilities section and was
        // strictly worse: `- ![fs:write:/](x.png)` granted filesystem write
        // while rendering as a picture.
        let make = |caps: &str| {
            format!("# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n{caps}\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n")
        };
        for attack in [
            "- ![fs:write:/](https://example.com/pixel.png)\n",
            "- net:connect<span>:evil.example:443</span>\n",
            "- [fs:write:/](https://evil.example)\n",
        ] {
            assert!(
                matches!(
                    parse_skill(&make(attack)),
                    Err(SkillParseError::InvalidCapability(_))
                ),
                "should have rejected non-literal capability {attack:?}"
            );
        }
    }

    #[test]
    fn a_heading_that_does_not_read_as_tools_needed_does_not_declare_tools() {
        let tail = "\n- admin.exec_all\n";
        let head = "# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n- none\n";
        // Renders as "Tools 🖼" / as a link to an attacker URL.
        for heading in [
            "## Tools ![needed](x.png)",
            "## [Tools needed](https://evil.example)",
        ] {
            assert_eq!(
                parse_skill(&format!(
                    "{head}\n{heading}{tail}\n## Tool capabilities needed\n- none\n"
                )),
                Err(SkillParseError::MissingToolsSection),
                "heading {heading:?} must not match"
            );
        }
    }

    #[test]
    fn a_decoy_tools_section_makes_the_document_ambiguous() {
        // A prominent quoted "Tools needed / none" above a real section was
        // silently ignored, so the visible declaration was not the effective
        // one. Nested occurrences now count.
        let source = "# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n- none\n\n> ## Tools needed\n>\n> - none\n\nSome prose here.\n\n## Tools needed\n- admin.exec_all\n";
        assert_eq!(
            parse_skill(source),
            Err(SkillParseError::AmbiguousToolsSection)
        );
    }

    #[test]
    fn rejects_more_tools_than_the_manifest_bound_allows() {
        let head = "# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n- none\n";
        let bullets = (0..=MAX_ALLOWED_TOOLS)
            .map(|index| format!("- ns.tool_{index}\n"))
            .collect::<String>();
        assert!(matches!(
            parse_skill(&format!(
                "{head}\n## Tools needed\n{bullets}\n## Tool capabilities needed\n- none\n"
            )),
            Err(SkillParseError::InvalidTool(_))
        ));
    }

    #[test]
    fn rejects_capability_failures() {
        // Capability bullets are appended AFTER `base`, so the tools section
        // has to be supplied by the closure rather than baked into `base` --
        // otherwise the appended bullets land under `## Tools needed`.
        let base = "# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n";
        let with = |bullets: &str| {
            format!(
                "{base}{bullets}\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n"
            )
        };
        assert_eq!(
            parse_skill(&with("")),
            Err(SkillParseError::MissingCapabilitiesList)
        );
        assert!(matches!(
            parse_skill(&with("- net:write:example.com\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert!(matches!(
            parse_skill(&with("- net:connect:x\n- net:connect:x\n")),
            Err(SkillParseError::DuplicateCapability(_))
        ));
        assert!(matches!(
            parse_skill(&with("- none\n- net:connect:x\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert!(matches!(
            parse_skill(&with("- net:connect:x | short\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert_eq!(
            parse_skill("# Valid Agent\n\nLong enough description for a valid agent.\n"),
            Err(SkillParseError::MissingCapabilitiesSection)
        );
        assert_eq!(
            parse_skill("# Valid Agent\n\nLong enough description for a valid agent.\n\n# Second Agent\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n"),
            Err(SkillParseError::MultipleTitles)
        );
        assert!(matches!(
            parse_skill(&with("- fs:read:/tmp/one,/tmp/two\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert_eq!(
            parse_skill(&with(
                "- none\n\n## Capabilities needed\n- none\n\n## Tools needed\n- none\n\n## Tool capabilities needed\n- none\n"
            )),
            Err(SkillParseError::AmbiguousCapabilitiesSection)
        );
        assert_eq!(
            parse_skill(&format!(
                "{base}- net:connect:one\n\nA separator paragraph.\n\n- net:connect:two\n"
            )),
            Err(SkillParseError::AmbiguousCapabilitiesSection)
        );
    }

    #[test]
    fn every_error_has_stable_public_text() {
        let errors = [
            SkillParseError::UnterminatedFrontmatter,
            SkillParseError::InvalidFrontmatter("line".to_string()),
            SkillParseError::DuplicateFrontmatterKey("agent".to_string()),
            SkillParseError::UnknownFrontmatterKey("other".to_string()),
            SkillParseError::MissingTitle,
            SkillParseError::MultipleTitles,
            SkillParseError::InvalidAgent,
            SkillParseError::InvalidDescription,
            SkillParseError::InvalidPrivilegeTier,
            SkillParseError::InvalidRestartPolicy,
            SkillParseError::InvalidChannel("channel".to_string()),
            SkillParseError::InvalidMessageSchemaVersion("channel=0".to_string()),
            SkillParseError::MissingCapabilitiesSection,
            SkillParseError::MissingCapabilitiesList,
            SkillParseError::AmbiguousCapabilitiesSection,
            SkillParseError::InvalidCapability("capability".to_string()),
            SkillParseError::DuplicateCapability("capability".to_string()),
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
            assert!(std::error::Error::source(&error).is_none());
        }
    }

    #[test]
    fn text_helpers_cover_nested_commonmark_shapes() {
        let rich = vec![
            InlineNode::Text(TextNode {
                value: "text ".to_string(),
            }),
            InlineNode::Emphasis(EmphasisNode {
                children: vec![InlineNode::Text(TextNode {
                    value: "em ".to_string(),
                })],
            }),
            InlineNode::Strong(StrongNode {
                children: vec![InlineNode::Text(TextNode {
                    value: "strong ".to_string(),
                })],
            }),
            InlineNode::Strikethrough(StrikethroughNode {
                children: vec![InlineNode::Text(TextNode {
                    value: "strike ".to_string(),
                })],
            }),
            InlineNode::CodeSpan(CodeSpanNode {
                value: "code ".to_string(),
            }),
            InlineNode::Link(LinkNode {
                destination: "ignored".to_string(),
                title: None,
                children: vec![InlineNode::Text(TextNode {
                    value: "link ".to_string(),
                })],
            }),
            InlineNode::Image(ImageNode {
                destination: "image".to_string(),
                title: None,
                alt: "alt ".to_string(),
            }),
            InlineNode::Autolink(AutolinkNode {
                destination: "https://example.test ".to_string(),
                is_email: false,
            }),
            InlineNode::HardBreak(HardBreakNode),
            InlineNode::SoftBreak(SoftBreakNode),
            InlineNode::RawInline(RawInlineNode {
                format: "html".to_string(),
                value: "ignored".to_string(),
            }),
        ];
        let text = inline_text(&rich);
        assert!(text.contains("strong"));
        assert!(text.contains("https://example.test"));
        assert!(!text.contains("ignored"));

        let paragraph = BlockNode::Paragraph(ParagraphNode { children: rich });
        let heading = BlockNode::Heading(HeadingNode {
            level: 3,
            children: vec![InlineNode::Text(TextNode {
                value: "heading".to_string(),
            })],
        });
        let nested = BlockNode::List(ListNode {
            ordered: false,
            start: None,
            tight: true,
            children: vec![
                ListChildNode::ListItem(ListItemNode {
                    children: vec![paragraph.clone()],
                }),
                ListChildNode::TaskItem(TaskItemNode {
                    checked: true,
                    children: vec![heading.clone()],
                }),
            ],
        });
        let quoted = BlockNode::Blockquote(BlockquoteNode {
            children: vec![nested],
        });
        assert!(block_text(&[heading, quoted]).contains("heading"));
    }
}
