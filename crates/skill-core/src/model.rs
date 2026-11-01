#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentId(pub(crate) String);

impl AgentId {
    pub fn parse(_value: &str) -> Result<Self, crate::CoreError> {
        Err(crate::CoreError::new("not_implemented"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
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
    File { bytes: Vec<u8>, unix_executable: bool },
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
