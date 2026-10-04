//! Contained, crash-safe storage for the one native authoring profile.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use thiserror::Error;
use uuid::Uuid;

pub const MAX_PROJECT_BYTES: usize = 8 * 1024 * 1024;
const STORE_FILE: &str = "authoring-session.json";
const TEMP_PREFIX: &str = ".authoring-session.tmp-";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredProject {
    pub bytes: Vec<u8>,
    pub revision: String,
}

/// Fixed public failures. Paths and platform error details never cross IPC.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum StoreError {
    #[error("the authoring profile is not safe to use")]
    UnsafeProfile,
    #[error("the stored authoring project is malformed")]
    MalformedStore,
    #[error("the supplied authoring bytes are invalid")]
    InvalidBytes,
    #[error("the authoring project changed in another session")]
    Conflict,
    #[error("the storage operation failed before commit")]
    Io,
    #[error("the storage commit completed but durability could not be confirmed")]
    Indeterminate,
}

#[derive(Debug)]
pub struct NativeProjectStore {
    root: PathBuf,
    root_handle: File,
    transaction: Mutex<()>,
}

impl NativeProjectStore {
    /// Open the trusted application-provided profile root. Renderer input must
    /// never be used to construct `root`.
    pub fn open(root: PathBuf) -> Result<Self, StoreError> {
        prepare_root(&root)?;
        if !crate::security::ancestors_have_no_mutating_acl(&root) {
            return Err(StoreError::UnsafeProfile);
        }
        let root_handle = open_root_no_follow(&root)?;
        validate_open_directory(&root_handle)?;
        Ok(Self {
            root,
            root_handle,
            transaction: Mutex::new(()),
        })
    }

    pub fn load(&self) -> Result<Option<StoredProject>, StoreError> {
        let _guard = self.transaction.lock().map_err(|_| StoreError::Io)?;
        self.load_locked()
    }

    pub fn compare_and_swap(
        &self,
        expected_revision: Option<&str>,
        bytes: &[u8],
    ) -> Result<String, StoreError> {
        validate_canonical(bytes).map_err(|_| StoreError::InvalidBytes)?;
        let _guard = self.transaction.lock().map_err(|_| StoreError::Io)?;
        let current = self.load_locked()?;
        match (current.as_ref(), expected_revision) {
            (None, None) => {}
            (Some(stored), Some(expected)) if stored.revision == expected => {}
            _ => return Err(StoreError::Conflict),
        }
        self.commit_locked(bytes)
    }

    pub fn with_project_at_revision<T>(
        &self,
        expected_revision: &str,
        operation: impl FnOnce(serde_json::Value) -> T,
    ) -> Result<T, StoreError> {
        let _guard = self.transaction.lock().map_err(|_| StoreError::Io)?;
        let stored = self.load_locked()?.ok_or(StoreError::Conflict)?;
        if stored.revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        let envelope: serde_json::Value =
            serde_json::from_slice(&stored.bytes).map_err(|_| StoreError::MalformedStore)?;
        let cursor = envelope
            .get("cursor")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(StoreError::MalformedStore)?;
        let project = envelope
            .get("history")
            .and_then(serde_json::Value::as_array)
            .and_then(|history| history.get(cursor))
            .cloned()
            .ok_or(StoreError::MalformedStore)?;
        Ok(operation(project))
    }

    fn load_locked(&self) -> Result<Option<StoredProject>, StoreError> {
        if !crate::security::ancestors_have_no_mutating_acl(&self.root) {
            return Err(StoreError::UnsafeProfile);
        }
        validate_open_directory(&self.root_handle)?;
        #[cfg(unix)]
        let opened = open_existing_at(&self.root_handle, STORE_FILE)?;
        #[cfg(not(unix))]
        let opened = open_existing_no_follow(&self.root.join(STORE_FILE))?;
        let file = match opened {
            Some(file) => file,
            None => return Ok(None),
        };
        validate_open_file(&file)?;
        let length = file.metadata().map_err(|_| StoreError::Io)?.len();
        if length > MAX_PROJECT_BYTES as u64 {
            return Err(StoreError::MalformedStore);
        }
        let mut bytes = Vec::with_capacity(length as usize);
        file.take(MAX_PROJECT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| StoreError::Io)?;
        if bytes.len() > MAX_PROJECT_BYTES {
            return Err(StoreError::MalformedStore);
        }
        validate_canonical(&bytes).map_err(|_| StoreError::MalformedStore)?;
        Ok(Some(StoredProject {
            revision: revision(&bytes),
            bytes,
        }))
    }

    fn commit_locked(&self, bytes: &[u8]) -> Result<String, StoreError> {
        if !crate::security::ancestors_have_no_mutating_acl(&self.root) {
            return Err(StoreError::UnsafeProfile);
        }
        validate_open_directory(&self.root_handle)?;
        let temp_name = format!("{TEMP_PREFIX}{}", Uuid::now_v7().hyphenated());
        let temp_path = self.root.join(temp_name);
        #[cfg(not(unix))]
        let destination = self.root.join(STORE_FILE);
        let result = (|| {
            #[cfg(unix)]
            let mut temporary = create_new_private_at(
                &self.root_handle,
                temp_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or(StoreError::Io)?,
            )?;
            #[cfg(not(unix))]
            let mut temporary = create_new_private(&temp_path)?;
            validate_open_file(&temporary)?;
            temporary.write_all(bytes).map_err(|_| StoreError::Io)?;
            temporary.sync_all().map_err(|_| StoreError::Io)?;
            drop(temporary);
            validate_open_directory(&self.root_handle)?;
            #[cfg(unix)]
            atomic_replace_at(
                &self.root_handle,
                temp_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or(StoreError::Io)?,
                STORE_FILE,
            )
            .map_err(|_| StoreError::Io)?;
            #[cfg(not(unix))]
            atomic_replace(&temp_path, &destination).map_err(|_| StoreError::Io)?;
            self.root_handle
                .sync_all()
                .map_err(|_| StoreError::Indeterminate)?;
            Ok(revision(bytes))
        })();
        if result.is_err() {
            #[cfg(unix)]
            let _ = remove_at(
                &self.root_handle,
                temp_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or(""),
            );
            #[cfg(not(unix))]
            let _ = fs::remove_file(&temp_path);
        }
        result
    }
}

#[cfg(unix)]
fn open_root_no_follow(path: &Path) -> Result<File, StoreError> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| StoreError::UnsafeProfile)
}

#[cfg(not(unix))]
fn open_root_no_follow(path: &Path) -> Result<File, StoreError> {
    File::open(path).map_err(|_| StoreError::UnsafeProfile)
}

#[cfg(unix)]
fn open_existing_at(directory: &File, name: &str) -> Result<Option<File>, StoreError> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
    };
    let name = CString::new(name).map_err(|_| StoreError::Io)?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor >= 0 {
        return Ok(Some(unsafe { File::from_raw_fd(descriptor) }));
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::NotFound {
        Ok(None)
    } else if error.raw_os_error() == Some(libc::ELOOP) {
        Err(StoreError::UnsafeProfile)
    } else {
        Err(StoreError::Io)
    }
}

#[cfg(unix)]
fn create_new_private_at(directory: &File, name: &str) -> Result<File, StoreError> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
    };
    let name = CString::new(name).map_err(|_| StoreError::Io)?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if descriptor < 0 {
        Err(StoreError::Io)
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

#[cfg(unix)]
fn atomic_replace_at(directory: &File, source: &str, destination: &str) -> io::Result<()> {
    use std::{ffi::CString, os::fd::AsRawFd};
    let source = CString::new(source).map_err(|_| io::Error::other("invalid source"))?;
    let destination =
        CString::new(destination).map_err(|_| io::Error::other("invalid destination"))?;
    let result = unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            source.as_ptr(),
            directory.as_raw_fd(),
            destination.as_ptr(),
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn remove_at(directory: &File, name: &str) -> io::Result<()> {
    use std::{ffi::CString, os::fd::AsRawFd};
    let name = CString::new(name).map_err(|_| io::Error::other("invalid name"))?;
    let result = unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn revision(bytes: &[u8]) -> String {
    format!("sha256:{}", BASE64.encode(Sha256::digest(bytes)))
}

fn validate_canonical(bytes: &[u8]) -> Result<(), ()> {
    if bytes.is_empty() || bytes.len() > MAX_PROJECT_BYTES {
        return Err(());
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| ())?;
    if !value.is_object() || serde_json::to_vec(&value).map_err(|_| ())? != bytes {
        return Err(());
    }
    validate_session(&value)
}

fn exact_object<'a>(
    value: &'a serde_json::Value,
    keys: &[&str],
) -> Result<&'a serde_json::Map<String, serde_json::Value>, ()> {
    let object = value.as_object().ok_or(())?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(());
    }
    Ok(object)
}

fn integer(value: &serde_json::Value) -> Result<u64, ()> {
    let value = value.as_u64().ok_or(())?;
    (value <= 9_007_199_254_740_991).then_some(value).ok_or(())
}

fn checked_array(value: &serde_json::Value) -> Result<&Vec<serde_json::Value>, ()> {
    let array = value.as_array().ok_or(())?;
    (array.len() <= 100_000).then_some(array).ok_or(())
}

fn checked_string(
    value: &serde_json::Value,
    maximum: usize,
    nonempty: bool,
    trimmed: bool,
    newline: bool,
) -> Result<&str, ()> {
    let text = value.as_str().ok_or(())?;
    let count = text.chars().count();
    if count > maximum
        || (nonempty && text.is_empty())
        || (trimmed && text.trim_matches(is_ecmascript_whitespace) != text)
    {
        return Err(());
    }
    if text.chars().any(|character| {
        let code = character as u32;
        code < 0x20 && !(newline && matches!(character, '\t' | '\n' | '\r')) || code == 0x7f
    }) {
        return Err(());
    }
    Ok(text)
}

fn portable_name(value: &serde_json::Value, maximum: usize) -> Result<&str, ()> {
    let text = checked_string(value, maximum, true, true, false)?;
    let mut previous_separator = false;
    for (index, byte) in text.bytes().enumerate() {
        let alphanumeric = byte.is_ascii_lowercase() || byte.is_ascii_digit();
        let separator = matches!(byte, b'.' | b'_' | b'-');
        if !alphanumeric && !separator || separator && (index == 0 || previous_separator) {
            return Err(());
        }
        previous_separator = separator;
    }
    if previous_separator {
        Err(())
    } else {
        Ok(text)
    }
}

fn uuid_v7(value: &serde_json::Value) -> Result<String, ()> {
    let text = checked_string(value, 2048, true, true, false)?;
    let parsed = Uuid::parse_str(text).map_err(|_| ())?;
    if parsed.get_version_num() != 7 || parsed.hyphenated().to_string() != text {
        return Err(());
    }
    Ok(text.to_owned())
}

fn canonical_sha256(value: &serde_json::Value) -> Result<(), ()> {
    let text = checked_string(value, 44, true, false, false)?;
    let decoded = BASE64.decode(text).map_err(|_| ())?;
    if decoded.len() != 32 || BASE64.encode(decoded) != text {
        Err(())
    } else {
        Ok(())
    }
}

fn validate_session(value: &serde_json::Value) -> Result<(), ()> {
    let node = exact_object(
        value,
        &["schemaVersion", "historyLimit", "cursor", "history"],
    )?;
    if integer(&node["schemaVersion"])? != 1 {
        return Err(());
    }
    let limit = usize::try_from(integer(&node["historyLimit"])?).map_err(|_| ())?;
    let history = checked_array(&node["history"])?;
    let cursor = usize::try_from(integer(&node["cursor"])?).map_err(|_| ())?;
    if !(1..=200).contains(&limit)
        || history.is_empty()
        || history.len() > limit
        || cursor >= history.len()
    {
        return Err(());
    }
    let mut project_id = None;
    for project in history {
        let identity = validate_project(project)?;
        if project_id
            .as_ref()
            .is_some_and(|expected| expected != &identity)
        {
            return Err(());
        }
        project_id = Some(identity);
    }
    Ok(())
}

fn validate_project(value: &serde_json::Value) -> Result<String, ()> {
    let node = exact_object(
        value,
        &[
            "schemaVersion",
            "projectId",
            "title",
            "site",
            "workflow",
            "documents",
            "activeDocumentId",
        ],
    )?;
    if integer(&node["schemaVersion"])? != 1 {
        return Err(());
    }
    let project_id = uuid_v7(&node["projectId"])?;
    checked_string(&node["title"], 512, true, true, false)?;
    let site = exact_object(&node["site"], &["baseUrl", "themeId"])?;
    portable_name(&site["themeId"], 2048)?;
    if !site["baseUrl"].is_null() {
        let text = checked_string(&site["baseUrl"], 2048, true, true, false)?;
        let parsed = url::Url::parse(text).map_err(|_| ())?;
        if !matches!(parsed.scheme(), "http" | "https")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(());
        }
    }
    let workflow = exact_object(&node["workflow"], &["lastPublication"])?;
    if !workflow["lastPublication"].is_null() {
        let publication = exact_object(
            &workflow["lastPublication"],
            &["authoringRevision", "manifestSha256", "targetId"],
        )?;
        let revision = checked_string(&publication["authoringRevision"], 1024, true, false, false)?;
        if revision.len() > 2_048 || revision.chars().any(|character| {
            let code = character as u32;
            (0x80..=0x9f).contains(&code)
                || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        }) {
            return Err(());
        }
        canonical_sha256(&publication["manifestSha256"])?;
        portable_name(&publication["targetId"], 2048)?;
    }
    let documents = checked_array(&node["documents"])?;
    if documents.len() > 1000 {
        return Err(());
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut slugs = std::collections::BTreeSet::new();
    for document in documents {
        let document = exact_object(document, &["id", "slug", "title", "status", "body"])?;
        let id = uuid_v7(&document["id"])?;
        let slug = portable_name(&document["slug"], 2048)?.to_owned();
        checked_string(&document["title"], 512, true, true, false)?;
        if !matches!(document["status"].as_str(), Some("draft" | "published"))
            || !ids.insert(id)
            || !slugs.insert(slug)
        {
            return Err(());
        }
        let mut nodes = 0_usize;
        validate_document(&document["body"], &mut nodes)?;
    }
    if !node["activeDocumentId"].is_null() {
        let active = uuid_v7(&node["activeDocumentId"])?;
        if !ids.contains(&active) {
            return Err(());
        }
    }
    Ok(project_id)
}

fn count_node(nodes: &mut usize, depth: usize) -> Result<(), ()> {
    *nodes = nodes.checked_add(1).ok_or(())?;
    if *nodes > 100_000 || depth > 64 {
        Err(())
    } else {
        Ok(())
    }
}

fn validate_document(value: &serde_json::Value, nodes: &mut usize) -> Result<(), ()> {
    count_node(nodes, 0)?;
    let node = exact_object(value, &["type", "children"])?;
    if node["type"] != "document" {
        return Err(());
    }
    for child in checked_array(&node["children"])? {
        validate_block(child, 1, nodes)?;
    }
    Ok(())
}

fn validate_children(value: &serde_json::Value, depth: usize, nodes: &mut usize) -> Result<(), ()> {
    for child in checked_array(value)? {
        validate_block(child, depth, nodes)?;
    }
    Ok(())
}

fn validate_inline_children(
    value: &serde_json::Value,
    depth: usize,
    nodes: &mut usize,
    in_link: bool,
) -> Result<(), ()> {
    for child in checked_array(value)? {
        validate_inline(child, depth, nodes, in_link)?;
    }
    Ok(())
}

fn validate_inline(
    value: &serde_json::Value,
    depth: usize,
    nodes: &mut usize,
    in_link: bool,
) -> Result<(), ()> {
    count_node(nodes, depth)?;
    let kind = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or(())?;
    match kind {
        "text" | "code_span" => {
            let node = exact_object(value, &["type", "value"])?;
            checked_string(&node["value"], 1_048_576, false, false, true)?;
        }
        "emphasis" | "strong" | "strikethrough" => {
            let node = exact_object(value, &["type", "children"])?;
            validate_inline_children(&node["children"], depth + 1, nodes, in_link)?;
        }
        "link" => {
            if in_link {
                return Err(());
            }
            let node = exact_object(value, &["type", "destination", "title", "children"])?;
            safe_destination(&node["destination"], false)?;
            if !node["title"].is_null() {
                checked_string(&node["title"], 512, false, false, false)?;
            }
            validate_inline_children(&node["children"], depth + 1, nodes, true)?;
        }
        "image" => {
            let node = exact_object(value, &["type", "destination", "title", "alt"])?;
            safe_destination(&node["destination"], false)?;
            if !node["title"].is_null() {
                checked_string(&node["title"], 512, false, false, false)?;
            }
            checked_string(&node["alt"], 1_048_576, false, false, true)?;
        }
        "autolink" => {
            let node = exact_object(value, &["type", "destination", "isEmail"])?;
            let email = node["isEmail"].as_bool().ok_or(())?;
            safe_destination(&node["destination"], email)?;
        }
        "hard_break" | "soft_break" => {
            exact_object(value, &["type"])?;
        }
        _ => return Err(()),
    }
    Ok(())
}

fn safe_destination(value: &serde_json::Value, email: bool) -> Result<(), ()> {
    let text = checked_string(value, 2048, true, true, false)?;
    if email {
        let mut parts = text.split('@');
        if text.chars().any(is_ecmascript_whitespace)
            || parts.next().is_none_or(str::is_empty)
            || parts.next().is_none_or(str::is_empty)
            || parts.next().is_some()
        {
            return Err(());
        }
    } else if text.starts_with("//") || text.contains('\\') {
        return Err(());
    } else if let Some((scheme, _)) = text.split_once(':') {
        if scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "+.-".contains(character))
            && !matches!(
                scheme.to_ascii_lowercase().as_str(),
                "http" | "https" | "mailto"
            )
        {
            return Err(());
        }
    }
    Ok(())
}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

fn validate_block(value: &serde_json::Value, depth: usize, nodes: &mut usize) -> Result<(), ()> {
    count_node(nodes, depth)?;
    let kind = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or(())?;
    match kind {
        "heading" => {
            let node = exact_object(value, &["type", "level", "children"])?;
            if !(1..=6).contains(&integer(&node["level"])?) {
                return Err(());
            }
            validate_inline_children(&node["children"], depth + 1, nodes, false)?;
        }
        "paragraph" => {
            let node = exact_object(value, &["type", "children"])?;
            validate_inline_children(&node["children"], depth + 1, nodes, false)?;
        }
        "code_block" => {
            let node = exact_object(value, &["type", "language", "value"])?;
            if !node["language"].is_null() {
                portable_name(&node["language"], 2048)?;
            }
            if !checked_string(&node["value"], 1_048_576, false, false, true)?.ends_with('\n') {
                return Err(());
            }
        }
        "blockquote" | "list_item" => {
            let node = exact_object(value, &["type", "children"])?;
            validate_children(&node["children"], depth + 1, nodes)?;
        }
        "task_item" => {
            let node = exact_object(value, &["type", "checked", "children"])?;
            node["checked"].as_bool().ok_or(())?;
            validate_children(&node["children"], depth + 1, nodes)?;
        }
        "list" => {
            let node = exact_object(value, &["type", "ordered", "start", "tight", "children"])?;
            let ordered = node["ordered"].as_bool().ok_or(())?;
            node["tight"].as_bool().ok_or(())?;
            if ordered {
                if integer(&node["start"])? < 1 {
                    return Err(());
                }
            } else if !node["start"].is_null() {
                return Err(());
            }
            for child in checked_array(&node["children"])? {
                if !matches!(
                    child.get("type").and_then(serde_json::Value::as_str),
                    Some("list_item" | "task_item")
                ) {
                    return Err(());
                }
                validate_block(child, depth + 1, nodes)?;
            }
        }
        "thematic_break" => {
            exact_object(value, &["type"])?;
        }
        "table" => validate_table(value, depth, nodes)?,
        _ => return Err(()),
    }
    Ok(())
}

fn validate_table(value: &serde_json::Value, depth: usize, nodes: &mut usize) -> Result<(), ()> {
    let node = exact_object(value, &["type", "align", "children"])?;
    let align = checked_array(&node["align"])?;
    if align
        .iter()
        .any(|item| !item.is_null() && !matches!(item.as_str(), Some("left" | "right" | "center")))
    {
        return Err(());
    }
    for row in checked_array(&node["children"])? {
        count_node(nodes, depth + 1)?;
        let row = exact_object(row, &["type", "isHeader", "children"])?;
        if row["type"] != "table_row" {
            return Err(());
        }
        row["isHeader"].as_bool().ok_or(())?;
        let cells = checked_array(&row["children"])?;
        if cells.len() != align.len() {
            return Err(());
        }
        for cell in cells {
            count_node(nodes, depth + 2)?;
            let cell = exact_object(cell, &["type", "children"])?;
            if cell["type"] != "table_cell" {
                return Err(());
            }
            validate_inline_children(&cell["children"], depth + 3, nodes, false)?;
        }
    }
    Ok(())
}

fn prepare_root(root: &Path) -> Result<(), StoreError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => validate_root_metadata(&metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(root).map_err(|_| StoreError::Io)?;
            set_private_directory(root)?;
            validate_root(root)
        }
        Err(_) => Err(StoreError::Io),
    }
}

fn validate_root(root: &Path) -> Result<(), StoreError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| StoreError::UnsafeProfile)?;
    validate_root_metadata(&metadata)
}

fn validate_root_metadata(metadata: &fs::Metadata) -> Result<(), StoreError> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(StoreError::UnsafeProfile);
    }
    validate_owner(metadata)
}

fn validate_open_file(file: &File) -> Result<(), StoreError> {
    let metadata = file.metadata().map_err(|_| StoreError::Io)?;
    if !metadata.is_file() {
        return Err(StoreError::UnsafeProfile);
    }
    validate_owner(&metadata)?;
    if !crate::security::has_no_mutating_acl(file) {
        return Err(StoreError::UnsafeProfile);
    }
    Ok(())
}

fn validate_open_directory(directory: &File) -> Result<(), StoreError> {
    let metadata = directory.metadata().map_err(|_| StoreError::Io)?;
    validate_root_metadata(&metadata)?;
    if !crate::security::has_no_mutating_acl(directory) {
        return Err(StoreError::UnsafeProfile);
    }
    Ok(())
}

#[cfg(unix)]
fn validate_owner(metadata: &fs::Metadata) -> Result<(), StoreError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.permissions().mode() & 0o077 != 0 {
        return Err(StoreError::UnsafeProfile);
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_owner(_metadata: &fs::Metadata) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_directory(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| StoreError::Io)
}

#[cfg(not(unix))]
fn set_private_directory(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(not(unix))]
fn open_existing_no_follow(path: &Path) -> Result<Option<File>, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(StoreError::UnsafeProfile),
        Ok(_) => File::open(path).map(Some).map_err(|_| StoreError::Io),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(StoreError::Io),
    }
}

#[cfg(not(unix))]
fn create_new_private(path: &Path) -> Result<File, StoreError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| StoreError::Io)
}

#[cfg(all(not(unix), not(windows)))]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
