use crate::*;
use skill_core::PackageTree;
use skill_io::{DirectoryChange, FileChange, IoEngine, MutationPlan};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Default)]
pub(crate) struct Session {
    pub library: Option<PathBuf>,
    pub scan: Option<crate::importing::ScanState>,
    pub preview: Option<Pending>,
    pub settings_tokens: BTreeMap<String, Option<Vec<u8>>>,
    pub profile_tokens: BTreeMap<String, crate::profiles::ProfileBaseline>,
    pub edits: BTreeMap<String, EditBaseline>,
}
pub(crate) struct EditBaseline {
    pub catalog: Option<Vec<u8>>,
    pub library: PathBuf,
    pub id: String,
    pub stamp: Stamp,
    pub bindings: Vec<TreeBinding>,
}
pub(crate) struct Pending {
    pub id: String,
    pub created: u64,
    pub kind: &'static str,
    pub stamp: Stamp,
    pub bindings: Vec<TreeBinding>,
    pub plan: MutationPlan,
    pub engine: IoEngine,
    pub outcomes: Vec<SafeOutcome>,
    pub blocked: bool,
}
#[derive(PartialEq, Eq)]
pub(crate) struct Stamp {
    pub settings: Option<Vec<u8>>,
    pub authority: Option<Vec<u8>>,
    pub registry: Vec<(String, Option<Vec<u8>>)>,
    pub approvals: Option<Vec<u8>>,
    pub context: String,
    pub probes: Vec<(PathBuf, Result<Option<Vec<u8>>, String>)>,
}
pub(crate) struct TreeBinding {
    pub path: PathBuf,
    pub actual: PathBuf,
    pub key: String,
    pub link: bool,
    pub tree: PackageTree,
}
impl TreeBinding {
    pub fn read(path: &Path) -> Result<Self, AppError> {
        let s = skill_io::read_package_snapshot(path, &skill_io::ScanLimits::default())?;
        Ok(Self {
            path: path.into(),
            actual: s.actual_path,
            key: s.physical_key,
            link: s.root_was_link,
            tree: s.tree,
        })
    }
    pub fn matches(&self) -> bool {
        Self::read(&self.path).is_ok_and(|s| {
            s.actual == self.actual
                && s.key == self.key
                && s.link == self.link
                && s.tree == self.tree
        })
    }
}
pub(crate) fn error(code: &'static str) -> AppError {
    AppError { code }
}
impl App {
    pub fn with_clock(context: AppContext, clock: Arc<dyn Clock>) -> Self {
        let builtins = context.registry.builtin_documents.clone();
        let source = RegistrySource {
            builtins,
            custom_dir: context.env.home.join(".skill-hub/agents"),
            approvals_path: Some(context.env.home.join(".skill-hub/agent-approvals.json")),
        };
        Self {
            context,
            state: Default::default(),
            clock,
            source,
        }
    }
    pub(crate) fn runtime_context(&self) -> Result<AppContext, AppError> {
        let mut registry = load_registry(&self.source, &self.context.env)?;
        registry.selected_variants = self.context.registry.selected_variants.clone();
        let (settings, _, _) = self.settings_data()?;
        for (id, variant) in settings.agent_variants {
            let profile = registry
                .entries
                .iter()
                .find_map(|e| e.profile.as_ref().filter(|p| p.id == id))
                .ok_or(error("unknown_variant_agent"))?;
            if !profile.variants.contains(&variant) {
                return Err(error("invalid_variant"));
            }
            registry.selected_variants.insert(id, variant);
        }
        Ok(AppContext {
            env: self.context.env.clone(),
            scope: self.context.scope,
            registry,
        })
    }
    pub(crate) fn stamp(&self) -> Result<Stamp, AppError> {
        let settings = skill_io::disk::optional_file(&self.settings_path())?;
        let ctx = self.runtime_context()?;
        let mut registry = Vec::new();
        match std::fs::read_dir(&self.source.custom_dir) {
            Ok(entries) => {
                for (n, entry) in entries.enumerate() {
                    if n > 256 {
                        return Err(error("too_many_profiles"));
                    }
                    let entry = entry.map_err(|_| error("registry_read_failed"))?;
                    if entry.path().extension().is_some_and(|e| e == "json") {
                        let name = entry
                            .file_name()
                            .into_string()
                            .map_err(|_| error("invalid_profile_path"))?;
                        let bytes =
                            skill_io::snapshot::read_bounded_file(&entry.path(), 256 * 1024).ok();
                        registry.push((name, bytes));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(error("registry_read_failed")),
        }
        registry.sort();
        let approvals = skill_io::disk::optional_file(
            self.source
                .approvals_path
                .as_ref()
                .ok_or(error("missing_approvals_path"))?,
        )?;
        let context = format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            ctx.env.home,
            ctx.env.appdata,
            ctx.env.xdg_config,
            ctx.env.project_root,
            ctx.env.platform,
            ctx.scope,
            ctx.registry.selected_variants,
            self.context.registry.selected_variants
        );
        let mut probes = Vec::new();
        for entry in &ctx.registry.entries {
            if !matches!(
                entry.state,
                ProfileState::Builtin | ProfileState::Enabled | ProfileState::Disabled
            ) {
                continue;
            }
            if let Some(profile) = &entry.profile {
                for probe in &profile.probes {
                    if let Some(base) = ctx.env.base(probe.base) {
                        let path = base.join(&probe.relative);
                        let value = skill_io::snapshot::read_bounded_file(&path, 4 * 1024 * 1024)
                            .map(Some)
                            .or_else(|e| {
                                if e.code == "not_found" {
                                    Ok(None)
                                } else {
                                    Err(e.code.to_owned())
                                }
                            });
                        probes.push((path, value));
                    }
                }
            }
        }
        Ok(Stamp {
            settings,
            authority: skill_io::disk::optional_file(&self.authority_path())?,
            registry,
            approvals,
            context,
            probes,
        })
    }
    pub(crate) fn install(
        &self,
        kind: &'static str,
        plan: MutationPlan,
        bindings: Vec<TreeBinding>,
        engine: IoEngine,
        outcomes: Vec<SafeOutcome>,
        blocked: bool,
    ) -> Result<(String, u64), AppError> {
        self.state.borrow_mut().preview = None;
        let mut bytes = 0usize;
        for b in &bindings {
            bytes = bytes.saturating_add(tree_bytes(&b.tree));
        }
        for c in &plan.directories {
            bytes = bytes
                .saturating_add(c.before.as_ref().map_or(0, tree_bytes))
                .saturating_add(c.after.as_ref().map_or(0, tree_bytes));
        }
        for f in &plan.files {
            bytes = bytes
                .saturating_add(f.before.as_ref().map_or(0, Vec::len))
                .saturating_add(f.after.as_ref().map_or(0, Vec::len));
        }
        if bytes > 64 * 1024 * 1024 {
            return Err(error("preview_too_large"));
        }
        let id = skill_io::disk::operation_id();
        let created = self.clock.now();
        let stamp = self.stamp()?;
        self.state.borrow_mut().preview = Some(Pending {
            id: id.clone(),
            created,
            kind,
            stamp,
            bindings,
            plan,
            engine,
            outcomes,
            blocked,
        });
        Ok((id, created.saturating_add(600)))
    }
    pub(crate) fn commit(&self, id: &str, kind: &str) -> Result<ReceiptDto, AppError> {
        let p = self
            .state
            .borrow_mut()
            .preview
            .take()
            .ok_or(error("preview_consumed"))?;
        let now = self.clock.now();
        if p.id != id || p.kind != kind {
            return Err(error("invalid_preview"));
        }
        if now < p.created || now >= p.created.saturating_add(600) {
            return Err(error("preview_expired"));
        }
        if p.blocked {
            return Err(error("preview_blocked"));
        }
        if p.stamp != self.stamp()? || p.bindings.iter().any(|b| !b.matches()) {
            return Err(error("preview_changed"));
        }
        if p.plan.directories.is_empty() && p.plan.files.is_empty() {
            return Ok(ReceiptDto {
                operation_id: String::new(),
                backup_location: String::new(),
                outcomes: p.outcomes,
            });
        }
        let mut receipt = safe_receipt(p.engine.apply(p.engine.prepare(&p.plan)?));
        receipt.outcomes.extend(p.outcomes);
        Ok(receipt)
    }
    pub fn cancel_preview(&self, _id: &str) {
        self.state.borrow_mut().preview = None;
    }
    pub(crate) fn engine(
        &self,
        write_roots: Vec<PathBuf>,
        write_files: Vec<PathBuf>,
        library: Option<&Path>,
    ) -> Result<IoEngine, AppError> {
        let (settings, _, _) = self.settings_data()?;
        let ctx = self.runtime_context()?;
        let scan_roots = self.known_roots(&ctx)?;
        let mut protected_roots = vec![self.context.env.home.join(".skill-hub/agents")];
        if let Some(library) = library {
            protected_roots.push(library.into());
        }
        Ok(IoEngine {
            backup_root: settings.backup_path,
            audit_path: self.context.env.home.join(".skill-hub/audit/events.jsonl"),
            write_roots,
            write_files,
            write_file_roots: vec![self.source.custom_dir.clone()],
            scan_roots,
            protected_roots,
            observer: Arc::new(skill_io::Quiet),
        })
    }
    pub(crate) fn known_roots(&self, ctx: &AppContext) -> Result<Vec<PathBuf>, AppError> {
        let mut roots = Vec::new();
        for entry in &ctx.registry.entries {
            if !matches!(
                entry.state,
                ProfileState::Builtin | ProfileState::Enabled | ProfileState::Disabled
            ) {
                continue;
            }
            if let Some(profile) = &entry.profile {
                for variant in &profile.variants {
                    for scope in [skill_core::Scope::User, skill_core::Scope::Project] {
                        if scope == skill_core::Scope::Project && ctx.env.project_root.is_none() {
                            continue;
                        }
                        let mut local = ctx.clone();
                        local.scope = scope;
                        local
                            .registry
                            .selected_variants
                            .insert(profile.id.clone(), variant.clone());
                        for r in crate::roots::resolve_profile_roots(profile, &local)?.roots {
                            roots.push(skill_io::disk::resolved_path(&r.path)?);
                        }
                    }
                }
            }
        }
        roots.sort();
        roots.dedup();
        Ok(roots)
    }
}
pub(crate) fn tree_bytes(tree: &PackageTree) -> usize {
    tree.entries.iter().fold(0usize, |n, e| {
        n.saturating_add(match &e.kind {
            skill_core::EntryKind::File { bytes, .. } => bytes.len(),
            _ => 0,
        })
    })
}
pub(crate) fn safe_receipt(receipt: skill_io::MutationReceipt) -> ReceiptDto {
    ReceiptDto {
        operation_id: receipt.operation_id,
        backup_location: skill_core::compatibility::safe_text(
            &receipt.backup_location.to_string_lossy(),
        ),
        outcomes: receipt
            .outcomes
            .into_iter()
            .map(|o| SafeOutcome {
                path: skill_core::compatibility::safe_text(&o.path.to_string_lossy()),
                outcome: o.outcome,
                reason: o.reason,
            })
            .collect(),
    }
}
pub(crate) fn file_change(
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
) -> FileChange {
    FileChange {
        path,
        before,
        after,
    }
}
pub(crate) fn directory_change(
    path: PathBuf,
    before: Option<PackageTree>,
    after: Option<PackageTree>,
) -> DirectoryChange {
    DirectoryChange {
        path,
        before,
        after,
    }
}
