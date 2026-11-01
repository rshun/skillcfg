#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct AgentId(pub(crate) String);

impl AgentId {
    pub fn parse(value: &str) -> Result<Self, crate::CoreError> {
        if value.len() > 64 || !value.split('.').all(crate::package::is_slug) {
            return Err(crate::CoreError::new("invalid_agent_id"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AgentId {
    type Error = crate::CoreError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<AgentId> for String {
    fn from(value: AgentId) -> Self {
        value.0
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
}

// No Debug or serialization: package bytes may contain private material.
#[derive(Clone, PartialEq, Eq)]
pub enum EntryKind {
    Directory,
    File {
        bytes: Vec<u8>,
        unix_executable: bool,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub relative_path: String,
    pub kind: EntryKind,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct PackageTree {
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadState {
    Declared,
    ConfigEnabled,
    ConfigDisabled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderExposure {
    pub agent: AgentId,
    pub variant: String,
    pub root_id: String,
    pub evidence: ReadState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIssue {
    pub code: &'static str,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageComparison {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

impl PackageComparison {
    pub fn is_equal(&self) -> bool {
        self.added.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}
