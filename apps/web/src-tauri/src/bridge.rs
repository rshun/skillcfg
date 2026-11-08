use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use skill_app::*;
use skill_core::{
    compatibility::safe_text,
    plan::{ConflictDecision, PackageId, SelectionPair, TargetKey},
    AgentId, Scope,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Debug, Serialize)]
pub struct SafeError {
    pub code: String,
    pub message: String,
}
impl SafeError {
    pub fn new(code: &str) -> Self {
        Self {
            code: code.into(),
            message: "操作未完成，请检查位置与状态后重试。".into(),
        }
    }
}
impl From<AppError> for SafeError {
    fn from(e: AppError) -> Self {
        Self::new(e.code)
    }
}
impl From<skill_io::IoError> for SafeError {
    fn from(e: skill_io::IoError) -> Self {
        Self::new(e.code)
    }
}
fn decode<T: DeserializeOwned>(value: Value) -> Result<T, SafeError> {
    serde_json::from_value(value).map_err(|_| SafeError::new("invalid_request"))
}
fn encode<T: Serialize>(value: T) -> Result<Value, SafeError> {
    serde_json::to_value(value).map_err(|_| SafeError::new("invalid_response"))
}
fn clean(value: Value) -> Value {
    match value {
        Value::String(s) => Value::String(safe_text(&s)),
        Value::Array(a) => Value::Array(a.into_iter().map(clean).collect()),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, clean(v))).collect()),
        v => v,
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Id {
    preview_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Agent {
    agent_id: AgentId,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Package {
    package_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    scope: Scope,
    project_root: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    source: PathBuf,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Choice {
    item_id: String,
    action: String,
    replace_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Import {
    scan_id: String,
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Edit {
    package_id: String,
    edit_token: String,
    document: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pair {
    package_id: String,
    agent: AgentId,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Distribution {
    selection: Vec<Pair>,
    #[serde(default)]
    conflict_decisions: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Restore {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    expected_revision: String,
    settings: GuiSettings,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Profile {
    profile_json: String,
    expected_revision: Option<String>,
    #[serde(default)]
    local_root_overrides: Vec<registry::LocalOverride>,
    enable_after_save: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Enable {
    agent_id: AgentId,
    enabled: bool,
    expected_revision: String,
}
pub fn isolated_env(home: PathBuf) -> HostEnv {
    HostEnv {
        appdata: Some(home.join("AppData/Roaming")),
        xdg_config: Some(home.join(".config")),
        home,
        project_root: None,
        search_path: vec![],
        platform: if cfg!(windows) {
            skill_core::Platform::Windows
        } else if cfg!(target_os = "macos") {
            skill_core::Platform::Macos
        } else {
            skill_core::Platform::Linux
        },
    }
}
pub struct DesktopSession {
    app: App,
    library: Option<PathBuf>,
}
impl DesktopSession {
    pub fn new(home: PathBuf) -> Result<Self, SafeError> {
        Self::with_env(isolated_env(home))
    }
    pub fn with_env(env: HostEnv) -> Result<Self, SafeError> {
        if !env.home.is_absolute() {
            return Err(SafeError::new("invalid_home"));
        }
        let source = RegistrySource {
            builtins: registry::builtin_profiles(),
            custom_dir: env.home.join(".skill-hub/agents"),
            approvals_path: Some(env.home.join(".skill-hub/agent-approvals.json")),
        };
        let registry = load_registry(&source, &env)?;
        let app = App::new(AppContext {
            env,
            registry,
            scope: Scope::User,
        });
        let library = app.load_settings().ok().map(|v| v.settings.library_path);
        Ok(Self { app, library })
    }
    fn library(&self) -> Result<PathBuf, SafeError> {
        self.library
            .clone()
            .ok_or(SafeError::new("settings_not_loaded"))
    }
    pub fn execute(&mut self, command: &str, request: Value) -> Result<Value, SafeError> {
        if request.to_string().len() > 32 * 1024 * 1024 {
            return Err(SafeError::new("request_too_large"));
        }
        let result = match command {
            "app_info" => {
                let _: Empty = decode(request)?;
                Ok(
                    json!({"version":env!("CARGO_PKG_VERSION"),"platform":if cfg!(windows){"windows"}else if cfg!(target_os="macos"){"macos"}else{"linux"},"mode":"desktop"}),
                )
            }
            "set_context" => {
                let r: Context = decode(request)?;
                if r.scope == Scope::Project && r.project_root.is_none() {
                    return Err(SafeError::new("invalid_project"));
                }
                if let Some(p) = &r.project_root {
                    if !p.is_absolute() || !p.is_dir() {
                        return Err(SafeError::new("invalid_project"));
                    }
                }
                let mut context = self.app.context.clone();
                context.scope = r.scope;
                context.env.project_root = r.project_root;
                self.app = App::new(context);
                self.library = self
                    .app
                    .load_settings()
                    .ok()
                    .map(|v| v.settings.library_path);
                Ok(Value::Null)
            }
            "load_settings" => {
                let _: Empty = decode(request)?;
                let v = self.app.load_settings()?;
                self.library = Some(v.settings.library_path.clone());
                Ok(json!({"settings":v.settings,"revision":v.revision,"warnings":v.warnings}))
            }
            "save_settings" => {
                let r: Settings = decode(request)?;
                let v = self.app.save_settings(SettingsRequest {
                    expected_revision: r.expected_revision,
                    settings: r.settings,
                })?;
                self.library = Some(v.settings.library_path.clone());
                Ok(json!({"settings":v.settings,"revision":v.revision,"warnings":v.warnings}))
            }
            "load_library" => {
                let _: Empty = decode(request)?;
                encode(self.app.load_library(&self.library()?)?)
            }
            "library_statuses" => {
                let _: Empty = decode(request)?;
                encode(self.app.library_statuses(&self.library()?)?)
            }
            "read_package" => {
                let r: Package = decode(request)?;
                let id = package_id(r.package_id)?;
                self.app.load_library(&self.library()?)?;
                let v = self.app.read_package(&id)?;
                Ok(
                    json!({"packageId":v.package_id,"revisionId":v.revision_id,"document":v.document,"editToken":v.edit_token,"files":v.files,"issues":v.issues}),
                )
            }
            "list_agent_profiles" => {
                let _: Empty = decode(request)?;
                let views = self.app.list_agent_profiles()?;
                Ok(Value::Array(views.into_iter().map(|v|json!({"id":v.id,"displayName":v.display_name,"revision":v.revision,"source":v.source,"state":match v.state {ProfileState::Builtin=>"builtin",ProfileState::Enabled=>"enabled",ProfileState::Pending=>"pending",ProfileState::Disabled=>"disabled",ProfileState::Invalid=>"invalid"},"variants":v.variants,"currentVariant":v.current_variant,"roots":v.roots.iter().map(root_view).collect::<Vec<_>>(),"issues":v.issues})).collect()))
            }
            "list_agent_skills" => {
                let r: Agent = decode(request)?;
                let inventory = self.app.list_agent_skills(r.agent_id)?;
                Ok(
                    json!({"agentId":inventory.agent,"installed":inventory.installed,"roots":inventory.roots.iter().map(root_view).collect::<Vec<_>>(),"items":inventory.items.into_iter().map(|i|json!({"name":i.metadata.as_ref().map(|m|m.name.clone()),"description":i.metadata.as_ref().map(|m|m.description.clone()),"sources":i.sources.iter().map(|p|source_view(p)).collect::<Vec<_>>(),"files":i.files,"issues":i.issues})).collect::<Vec<_>>(),"errors":inventory.errors}),
                )
            }
            "detect_agents" => {
                let _: Empty = decode(request)?;
                let env = &self.app.context.env;
                let registry = load_registry(
                    &RegistrySource {
                        builtins: registry::builtin_profiles(),
                        custom_dir: env.home.join(".skill-hub/agents"),
                        approvals_path: Some(env.home.join(".skill-hub/agent-approvals.json")),
                    },
                    env,
                )?;
                Ok(Value::Array(
                    registry
                        .entries
                        .iter()
                        .filter_map(|e| {
                            e.profile.as_ref().map(|p| {
                                let installed = matches!(
                                    e.state,
                                    ProfileState::Builtin | ProfileState::Enabled
                                ) && env.search_path.iter().any(|dir| {
                                    p.binary_names.iter().any(|n| {
                                        if cfg!(windows) {
                                            [".exe", ".cmd", ".bat", ""]
                                                .iter()
                                                .any(|s| dir.join(format!("{n}{s}")).is_file())
                                        } else {
                                            dir.join(n).is_file()
                                        }
                                    })
                                });
                                json!({"agentId":p.id,"installed":installed,"issues":e.issues})
                            })
                        })
                        .collect(),
                ))
            }
            "scan_import" => {
                let r: Source = decode(request)?;
                encode(self.app.scan_import(ImportRequest {
                    source: r.source,
                    library_path: self.library()?,
                })?)
            }
            "preview_import" => {
                let r: Import = decode(request)?;
                if r.choices.len() > 1024 {
                    return Err(SafeError::new("too_many_choices"));
                }
                let choices = r
                    .choices
                    .into_iter()
                    .map(|c| {
                        Ok(ImportChoice {
                            item_id: c.item_id,
                            action: match c.action.as_str() {
                                "skip" => ImportAction::Skip,
                                "new" => ImportAction::New,
                                "replace" => ImportAction::Replace(package_id(
                                    c.replace_id.ok_or(SafeError::new("invalid_request"))?,
                                )?),
                                _ => return Err(SafeError::new("invalid_request")),
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, SafeError>>()?;
                encode(self.app.preview_import(&r.scan_id, &choices)?)
            }
            "save_skill_document" => {
                let r: Edit = decode(request)?;
                encode(self.app.save_skill_document(EditDocumentRequest {
                    package_id: package_id(r.package_id)?,
                    edit_token: r.edit_token,
                    document: r.document,
                })?)
            }
            "preview_distribution" => {
                let r: Distribution = decode(request)?;
                if r.selection.len() > 4096 || r.conflict_decisions.len() > 4096 {
                    return Err(SafeError::new("too_many_pairs"));
                }
                let selection = r
                    .selection
                    .into_iter()
                    .map(|p| {
                        Ok(SelectionPair {
                            package_id: package_id(p.package_id)?,
                            agent: p.agent,
                        })
                    })
                    .collect::<Result<Vec<_>, SafeError>>()?;
                let decisions = r
                    .conflict_decisions
                    .into_iter()
                    .map(|(k, v)| {
                        Ok((
                            TargetKey(k),
                            match v.as_str() {
                                "skip" => ConflictDecision::Skip,
                                "replace" => ConflictDecision::Replace,
                                _ => return Err(SafeError::new("invalid_request")),
                            },
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>, SafeError>>()?;
                encode(self.app.preview_distribution(DistributionRequest {
                    library_path: self.library()?,
                    scope: self.app.context.scope,
                    project_root: self.app.context.env.project_root.clone(),
                    selection,
                    conflict_decisions: decisions,
                })?)
            }
            "preview_restore" => {
                let r: Restore = decode(request)?;
                encode(self.app.preview_restore(&r.operation_id)?)
            }
            "commit_distribution" | "commit_import" | "commit_restore" | "commit_agent_profile" => {
                let r: Id = decode(request)?;
                let receipt = match command {
                    "commit_distribution" => self.app.commit_distribution(&r.preview_id),
                    "commit_import" => self.app.commit_import(&r.preview_id),
                    "commit_restore" => self.app.commit_restore(&r.preview_id),
                    _ => self.app.commit_agent_profile(&r.preview_id),
                }?;
                encode(receipt)
            }
            "cancel_preview" => {
                let r: Id = decode(request)?;
                self.app.cancel_preview(&r.preview_id);
                Ok(Value::Null)
            }
            "read_agent_profile" => {
                let r: Agent = decode(request)?;
                let (document, revision) = self.app.read_agent_profile_document(&r.agent_id)?;
                let env = &self.app.context.env;
                let registry = load_registry(
                    &RegistrySource {
                        builtins: registry::builtin_profiles(),
                        custom_dir: env.home.join(".skill-hub/agents"),
                        approvals_path: Some(env.home.join(".skill-hub/agent-approvals.json")),
                    },
                    env,
                )?;
                let overrides = registry
                    .approvals
                    .into_iter()
                    .find(|a| a.id == r.agent_id && a.platform == env.platform)
                    .map(|a| a.overrides)
                    .unwrap_or_default();
                Ok(
                    json!({"profileJson":document,"revision":revision,"localRootOverrides":overrides}),
                )
            }
            "preview_agent_profile" => {
                let r: Profile = decode(request)?;
                encode(self.app.preview_agent_profile(ProfileChangeRequest {
                    profile_json: r.profile_json.into_bytes(),
                    expected_revision: r.expected_revision,
                    local_root_overrides: r.local_root_overrides,
                    enable_after_save: r.enable_after_save,
                })?)
            }
            "set_agent_profile_enabled" => {
                let r: Enable = decode(request)?;
                encode(self.app.set_agent_profile_enabled(ProfileEnableRequest {
                    agent_id: r.agent_id,
                    enabled: r.enabled,
                    expected_revision: r.expected_revision,
                })?)
            }
            "list_history" => {
                let _: Empty = decode(request)?;
                encode(self.app.list_history()?)
            }
            "list_recoveries" => {
                let _: Empty = decode(request)?;
                Ok(Value::Array(self.app.list_recoveries()?.into_iter().map(|r|json!({"operationId":r.operation_id,"kind":r.kind,"state":r.state,"paths":r.paths,"reason":r.reason})).collect()))
            }
            _ => Err(SafeError::new("unknown_command")),
        }?;
        Ok(clean(result))
    }
}
fn package_id(id: String) -> Result<PackageId, SafeError> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        Err(SafeError::new("invalid_request"))
    } else {
        Ok(PackageId(id))
    }
}
fn source_view(p: &Path) -> Value {
    let actual = skill_io::disk::resolved_path(p).ok();
    let linked =
        actual.as_ref().is_some_and(|a| a != p) || skill_io::paths::is_link(p).unwrap_or(false);
    json!({"path":p,"actualPath":actual,"rootWasLink":linked,"exists":p.is_dir()})
}
fn root_view(r: &roots::ResolvedRoot) -> Value {
    let s = source_view(&r.path);
    json!({"agent":r.agent,"variant":r.variant,"rootId":r.root_id,"path":r.path,"actualPath":s["actualPath"],"rootWasLink":s["rootWasLink"],"exists":s["exists"],"writable":r.writable,"priority":r.priority})
}
