//! Fail-closed parser for D18 Level 1 `SKILL.md` agents.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_json_serializer::{serialize_pretty, JsonSerializerError, SerializerConfig};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use document_ast::{BlockNode, InlineNode, ListChildNode};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

const MANIFEST_SCHEMA: &str = "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/agent_manifest.schema.json";

/// One validated operating-system capability from an agent manifest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Capability {
    /// Capability taxonomy category.
    pub category: String,
    /// Operation within the category.
    pub action: String,
    /// Narrow resource selected by the operation.
    pub target: String,
    /// Human-readable reason for the access.
    pub justification: String,
}

/// Declared channel access for one agent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChannelAccess {
    /// Channels consumed by the agent.
    pub reads: Vec<String>,
    /// Channels produced by the agent.
    pub writes: Vec<String>,
}

/// Typed schema-v1 manifest generated from one Level 1 skill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentManifest {
    /// Stable lowercase agent identifier.
    pub agent: String,
    /// Reviewer-facing purpose statement.
    pub description: String,
    /// D18 privilege tier in the inclusive range zero through three.
    pub privilege_tier: u8,
    /// Declared input and output channels.
    pub channels: ChannelAccess,
    /// Validated OS capability profile.
    pub capabilities: Vec<Capability>,
    /// Supervisor behavior: `always`, `on-failure`, or `never`.
    pub restart_policy: String,
    /// Overall capability-profile justification.
    pub justification: String,
}

impl AgentManifest {
    /// Render deterministic, schema-shaped pretty JSON with a trailing newline.
    pub fn to_json(&self) -> Result<String, JsonSerializerError> {
        serialize_pretty(
            &manifest_json(self),
            &SerializerConfig {
                sort_keys: false,
                trailing_newline: true,
                ..SerializerConfig::default()
            },
        )
    }
}

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
    /// The required capability section is absent.
    MissingCapabilitiesSection,
    /// The capability section has no list.
    MissingCapabilitiesList,
    /// Capability declarations were split across repeated sections or lists.
    AmbiguousCapabilitiesSection,
    /// A capability bullet is malformed or outside the taxonomy.
    InvalidCapability(String),
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
            Self::MissingCapabilitiesSection => {
                formatter.write_str("SKILL.md requires a Capabilities needed section")
            }
            Self::MissingCapabilitiesList => {
                formatter.write_str("SKILL.md capabilities section requires a list")
            }
            Self::AmbiguousCapabilitiesSection => {
                formatter.write_str("SKILL.md capabilities section is ambiguous")
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
    let capabilities = parse_capabilities(&document.children, &agent)?;
    let deno_permissions = deno_permissions(&capabilities);
    let manifest = AgentManifest {
        agent: agent.clone(),
        description,
        privilege_tier,
        channels,
        capabilities,
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
            "agent" | "description" | "privilege_tier" | "reads" | "writes" | "restart_policy"
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

fn parse_capabilities(
    nodes: &[BlockNode],
    agent: &str,
) -> Result<Vec<Capability>, SkillParseError> {
    let sections = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            BlockNode::Heading(heading) if heading.level == 2 => inline_text(&heading.children)
                .trim()
                .eq_ignore_ascii_case("capabilities needed")
                .then_some(index),
            _ => None,
        })
        .collect::<Vec<_>>();
    let section = match sections.as_slice() {
        [] => return Err(SkillParseError::MissingCapabilitiesSection),
        [section] => *section,
        _ => return Err(SkillParseError::AmbiguousCapabilitiesSection),
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
        [] => return Err(SkillParseError::MissingCapabilitiesList),
        [list] => *list,
        _ => return Err(SkillParseError::AmbiguousCapabilitiesSection),
    };
    let mut capabilities = Vec::new();
    let mut seen = BTreeSet::new();
    for child in &list.children {
        let blocks = match child {
            ListChildNode::ListItem(item) => &item.children,
            ListChildNode::TaskItem(item) => &item.children,
        };
        let value = block_text(blocks).trim().to_string();
        if value.eq_ignore_ascii_case("none") {
            if list.children.len() != 1 {
                return Err(SkillParseError::InvalidCapability(value));
            }
            return Ok(Vec::new());
        }
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

fn manifest_json(manifest: &AgentManifest) -> JsonValue {
    let strings = |values: &[String]| {
        JsonValue::Array(values.iter().cloned().map(JsonValue::String).collect())
    };
    JsonValue::Object(vec![
        (
            "$schema".to_string(),
            JsonValue::String(MANIFEST_SCHEMA.to_string()),
        ),
        (
            "version".to_string(),
            JsonValue::Number(JsonNumber::Integer(1)),
        ),
        (
            "agent".to_string(),
            JsonValue::String(manifest.agent.clone()),
        ),
        (
            "description".to_string(),
            JsonValue::String(manifest.description.clone()),
        ),
        (
            "privilege_tier".to_string(),
            JsonValue::Number(JsonNumber::Integer(i64::from(manifest.privilege_tier))),
        ),
        (
            "channels".to_string(),
            JsonValue::Object(vec![
                ("reads".to_string(), strings(&manifest.channels.reads)),
                ("writes".to_string(), strings(&manifest.channels.writes)),
            ]),
        ),
        (
            "capabilities".to_string(),
            JsonValue::Array(
                manifest
                    .capabilities
                    .iter()
                    .map(|capability| {
                        JsonValue::Object(vec![
                            (
                                "category".to_string(),
                                JsonValue::String(capability.category.clone()),
                            ),
                            (
                                "action".to_string(),
                                JsonValue::String(capability.action.clone()),
                            ),
                            (
                                "target".to_string(),
                                JsonValue::String(capability.target.clone()),
                            ),
                            (
                                "justification".to_string(),
                                JsonValue::String(capability.justification.clone()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "restart_policy".to_string(),
            JsonValue::String(manifest.restart_policy.clone()),
        ),
        (
            "justification".to_string(),
            JsonValue::String(manifest.justification.clone()),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use document_ast::{
        AutolinkNode, BlockquoteNode, CodeSpanNode, EmphasisNode, HardBreakNode, HeadingNode,
        ImageNode, LinkNode, ListItemNode, ListNode, ParagraphNode, RawInlineNode, SoftBreakNode,
        StrikethroughNode, StrongNode, TaskItemNode, TextNode,
    };

    const MINIMAL: &str = "# Weather Reporter\n\nYou are a weather reporting agent for friendly local forecasts.\n\n## Capabilities needed\n- net:connect:api.weather.gov:443\n\n## Output format\nBe brief.\n";

    #[test]
    fn issue_example_infers_safe_defaults_and_permissions() {
        let skill = parse_skill(MINIMAL).unwrap();
        assert_eq!(skill.title, "Weather Reporter");
        assert_eq!(skill.manifest.agent, "weather-reporter");
        assert_eq!(skill.manifest.privilege_tier, 0);
        assert_eq!(skill.manifest.channels, ChannelAccess::default());
        assert_eq!(skill.manifest.restart_policy, "on-failure");
        assert_eq!(skill.deno_permissions, ["--allow-net=api.weather.gov:443"]);
        assert!(skill.instructions.starts_with("# Weather Reporter"));
    }

    #[test]
    fn frontmatter_overrides_metadata_and_sorts_runtime_access() {
        let source = "---\nagent: 'forecast-agent'\ndescription: \"Produces precise forecasts for subscribed cities.\"\nprivilege_tier: 1\nreads: [weather-requests]\nwrites: [weather-reports]\nrestart_policy: always\n---\n# Forecast\n\nIgnored description because frontmatter wins.\n\n## Capabilities needed\n- fs:write:/tmp/cache | Stores short-lived forecast cache data.\n- net:connect:api.weather.gov:443\n- fs:read:/tmp/cache\n- net:dns:api.weather.gov\n";
        let skill = parse_skill(source).unwrap();
        assert_eq!(skill.manifest.agent, "forecast-agent");
        assert_eq!(skill.manifest.channels.reads, ["weather-requests"]);
        assert_eq!(skill.manifest.channels.writes, ["weather-reports"]);
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
        assert!(json.contains("\"reads\": [\n      \"weather-requests\"\n    ]"));
    }

    #[test]
    fn explicit_none_produces_empty_profile_and_schema_json() {
        let source = "# Greeter\n\nGreets the user without external operating-system access.\n\n## Capabilities needed\n- none\n";
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
        let source = "# Operator\n\nExercises every supported Deno permission mapping safely.\n\n## Capabilities needed\n- proc:exec:git\n- env:read:HOME\n- ffi:load:libdemo\n- stdin:read:stdin\n- stdout:write:stdout\n- time:sleep:clock\n";
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
            parse_skill("# Valid Name\n\nshort\n\n## Capabilities needed\n- none\n"),
            Err(SkillParseError::InvalidDescription)
        );
        let bad_tier = "---\nprivilege_tier: 4\n---\n# Valid Name\n\nLong enough description.\n\n## Capabilities needed\n- none\n";
        assert_eq!(
            parse_skill(bad_tier),
            Err(SkillParseError::InvalidPrivilegeTier)
        );
    }

    #[test]
    fn rejects_invalid_channels_and_restart_policy() {
        let both = "---\nreads: [same-channel]\nwrites: [same-channel]\n---\n# Valid Agent\n\nLong enough description.\n\n## Capabilities needed\n- none\n";
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
    fn rejects_capability_failures() {
        let base = "# Valid Agent\n\nLong enough description for a valid agent.\n\n## Capabilities needed\n";
        assert_eq!(
            parse_skill(base),
            Err(SkillParseError::MissingCapabilitiesList)
        );
        assert!(matches!(
            parse_skill(&format!("{base}- net:write:example.com\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert!(matches!(
            parse_skill(&format!("{base}- net:connect:x\n- net:connect:x\n")),
            Err(SkillParseError::DuplicateCapability(_))
        ));
        assert!(matches!(
            parse_skill(&format!("{base}- none\n- net:connect:x\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert!(matches!(
            parse_skill(&format!("{base}- net:connect:x | short\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert_eq!(
            parse_skill("# Valid Agent\n\nLong enough description for a valid agent.\n"),
            Err(SkillParseError::MissingCapabilitiesSection)
        );
        assert_eq!(
            parse_skill("# Valid Agent\n\nLong enough description for a valid agent.\n\n# Second Agent\n\n## Capabilities needed\n- none\n"),
            Err(SkillParseError::MultipleTitles)
        );
        assert!(matches!(
            parse_skill(&format!("{base}- fs:read:/tmp/one,/tmp/two\n")),
            Err(SkillParseError::InvalidCapability(_))
        ));
        assert_eq!(
            parse_skill(&format!("{base}- none\n\n## Capabilities needed\n- none\n")),
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
