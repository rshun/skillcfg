use crate::*;
use crate::{
    binding::{ContextOverride, FileBinding},
    distribution::DistributionRecord,
    preview::{error, TreeBinding},
    settings::Authority,
};
use skill_core::Scope;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub operation_id: String,
    pub kind: String,
    pub created_at: u64,
    pub backup_location: String,
    pub outcomes: Vec<SafeOutcome>,
    pub records: Vec<DistributionRecord>,
    pub warnings: Vec<String>,
}
impl App {
    pub(crate) fn history_authority(&self) -> Result<Authority, AppError> {
        let a = match self.authority() {
            Ok((a, _)) => a,
            Err(_) => {
                let raw = crate::binding::read_optional(&self.authority_path(), 4 * 1024 * 1024)?
                    .ok_or(error("recovery_authority_missing"))?;
                serde_json::from_slice::<Authority>(&raw)
                    .map_err(|_| error("invalid_state_roots"))?
            }
        };
        if a.version != 1 || a.libraries.len() > 64 || a.backups.len() > 64 {
            return Err(error("invalid_state_roots"));
        }
        for p in a.libraries.iter().chain(a.backups.iter()) {
            self.basic_location(p)?;
        }
        Ok(a)
    }
    pub(crate) fn history_engines(&self) -> Result<Vec<skill_io::IoEngine>, AppError> {
        let authority = self.history_authority()?;
        let over = ContextOverride {
            scope: self.context.scope,
            project: self.context.env.project_root.clone(),
            recovery: true,
            extra_paths: vec![],
        };
        let ctx = self.context_for(Some(&over))?;
        let scan_roots = self.known_roots(&ctx)?;
        let mut write_roots = BTreeSet::new();
        for entry in &ctx.registry.entries {
            if !matches!(
                entry.state,
                ProfileState::Builtin | ProfileState::Enabled | ProfileState::Disabled
            ) {
                continue;
            }
            if let Some(p) = &entry.profile {
                for variant in &p.variants {
                    for scope in [Scope::User, Scope::Project] {
                        if scope == Scope::Project && ctx.env.project_root.is_none() {
                            continue;
                        }
                        let mut local = ctx.clone();
                        local.scope = scope;
                        local
                            .registry
                            .selected_variants
                            .insert(p.id.clone(), variant.clone());
                        if self.profile_authorized(p, &local, true).is_err() {
                            continue;
                        }
                        for root in crate::roots::resolve_profile_roots(p, &local)?.roots {
                            if root.writable {
                                write_roots.insert(skill_io::disk::resolved_path(&root.path)?);
                            }
                        }
                    }
                }
            }
        }
        let mut files = vec![
            self.settings_path(),
            self.authority_path(),
            self.source
                .approvals_path
                .clone()
                .ok_or(error("missing_approvals_path"))?,
        ];
        for library in &authority.libraries {
            let library = self.basic_location(library)?;
            write_roots.insert(library.join("packages"));
            files.push(library.join("catalog.json"));
        }
        let mut roots = authority.backups;
        roots.sort();
        roots.dedup();
        let mut engines = Vec::new();
        for backup in roots {
            engines.push(skill_io::IoEngine {
                backup_root: skill_io::disk::resolved_path(&backup)?,
                audit_path: self.context.env.home.join(".skill-hub/audit/events.jsonl"),
                write_roots: write_roots.iter().cloned().collect(),
                write_files: files.clone(),
                write_file_roots: vec![self.source.custom_dir.clone()],
                scan_roots: scan_roots.clone(),
                protected_roots: vec![self.source.custom_dir.clone()],
                observer: std::sync::Arc::new(skill_io::Quiet),
            });
        }
        Ok(engines)
    }
    pub fn list_history(&self) -> Result<Vec<HistoryItem>, AppError> {
        let mut items = Vec::new();
        let mut ids = BTreeSet::new();
        for engine in self.history_engines()? {
            for (receipt, metadata) in engine.history_details()? {
                if !ids.insert(receipt.operation_id.clone()) {
                    return Err(error("duplicate_operation_id"));
                }
                let kind = metadata.get("kind").cloned().unwrap_or_else(|| {
                    if metadata.contains_key("restoreBackupRoot") {
                        "restore".into()
                    } else {
                        "mutation".into()
                    }
                });
                let created_at = metadata
                    .get("createdAt")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| {
                        receipt
                            .operation_id
                            .split('-')
                            .nth(2)
                            .and_then(|s| u128::from_str_radix(s, 16).ok())
                            .map(|n| (n / 1_000_000_000) as u64)
                            .unwrap_or(0)
                    });
                let mut warnings = Vec::new();
                let mut records = if let Some(raw) = metadata.get("records") {
                    match serde_json::from_str::<Vec<DistributionRecord>>(raw) {
                        Ok(r) if r.len() <= 4096 => r,
                        _ => {
                            warnings.push("distribution_metadata_unreadable".into());
                            vec![]
                        }
                    }
                } else {
                    vec![]
                };
                for r in &mut records {
                    r.name_at_write = skill_core::compatibility::safe_text(&r.name_at_write);
                    r.physical_path = PathBuf::from(skill_core::compatibility::safe_text(
                        &r.physical_path.to_string_lossy(),
                    ));
                    r.project = r.project.as_ref().map(|p| {
                        PathBuf::from(skill_core::compatibility::safe_text(&p.to_string_lossy()))
                    });
                }
                let safe = crate::preview::safe_receipt(receipt);
                items.push(HistoryItem {
                    operation_id: safe.operation_id,
                    kind,
                    created_at,
                    backup_location: safe.backup_location,
                    outcomes: safe.outcomes,
                    records,
                    warnings,
                });
            }
        }
        items.sort_by(|a, b| a.operation_id.cmp(&b.operation_id));
        Ok(items)
    }
    pub fn list_recoveries(&self) -> Result<Vec<skill_io::RecoveryRecord>, AppError> {
        let mut result = Vec::new();
        for engine in self.history_engines()? {
            for mut r in engine.list_recoveries()? {
                r.paths = r
                    .paths
                    .iter()
                    .map(|p| {
                        PathBuf::from(skill_core::compatibility::safe_text(&p.to_string_lossy()))
                    })
                    .collect();
                result.push(r);
            }
        }
        Ok(result)
    }
    pub fn preview_restore(&self, id: &str) -> Result<PreviewDto, AppError> {
        self.state.borrow_mut().preview = None;
        let over = ContextOverride {
            scope: self.context.scope,
            project: self.context.env.project_root.clone(),
            recovery: true,
            extra_paths: vec![],
        };
        let stamp = self.snapshot(Some(&over))?;
        let mut found = None;
        for engine in self.history_engines()? {
            let operation = engine.backup_root.join(id);
            if operation.exists() {
                if found.is_some() {
                    return Err(error("duplicate_operation_id"));
                }
                let plan = engine.plan_restore(id)?;
                let sources = engine.restore_sources(id)?;
                found = Some((engine, plan, operation, sources));
            }
        }
        let (mut engine, mut plan, operation, sources) = found.ok_or(error("unknown_operation"))?;
        // The authority index is monotonic: restoring an older operation must not
        // make later approved library/backup locations disappear from history.
        plan.files.retain(|f| f.path != self.authority_path());
        let authority=self.history_authority()?;
        if plan.metadata.get("restoredKind").is_some_and(|k|k=="distribution"){
          for c in &plan.directories {for protected in authority.libraries.iter().chain(&authority.backups){if crate::settings::overlap(&c.path,&skill_io::disk::resolved_path(protected)?){return Err(error("protected_target"));}}}
        }
        if plan.metadata.get("restoredKind").is_some_and(|k|k=="import"){
          for file in &mut plan.files {if file.after.is_none() && file.path.file_name().is_some_and(|n|n=="catalog.json") && authority.libraries.iter().any(|p|skill_io::disk::resolved_path(p).is_ok_and(|p|file.path.parent()==Some(p.as_path()))){file.after=Some(br#"{"version":1,"records":[]}"#.to_vec());}}
        }
        let mut warnings =
            vec!["恢复前会完整备份当前内容；原先不存在的目录通过归档移出加载路径。".into()];
        if let Ok((settings, _, _)) = self.settings_data() {
            engine.backup_root = skill_io::disk::resolved_path(&settings.backup_path)?;
        } else {
            warnings.push("设置损坏，恢复前的备份将写入该历史备份根。".into());
        }
        let mut auxiliary = vec![FileBinding::read(operation.join("manifest.json"))?];
        for path in sources.files {
            if !skill_io::disk::resolved_path(&path)?.starts_with(&operation) {
                return Err(error("invalid_backup"));
            }
            auxiliary.push(FileBinding::read(path)?);
        }
        let mut bindings = Vec::new();
        let mut source_bytes = 0usize;
        for path in sources.directories {
            if skill_io::paths::is_link(&path)?
                || !skill_io::disk::resolved_path(&path)?.starts_with(&operation)
            {
                return Err(error("invalid_backup"));
            }
            let binding = TreeBinding::read(&path)?;
            if binding.link || !binding.actual.starts_with(&operation) {
                return Err(error("invalid_backup"));
            }
            source_bytes = source_bytes.saturating_add(crate::preview::tree_bytes(&binding.tree));
            if source_bytes > 64 * 1024 * 1024 {
                return Err(error("preview_too_large"));
            }
            bindings.push(binding);
        }
        let mut display = skill_core::plan::DistributionPlan::default();
        for change in &plan.directories {
            display.groups.push(crate::importing::display_group(
                change.path.clone(),
                change.before.as_ref(),
                change.after.as_ref(),
            ));
        }
        for file in &plan.files {
            display.groups.push(file_group(
                &file.path,
                file.before.as_deref(),
                file.after.as_deref(),
            ));
        }
        let groups = skill_core::diff::redacted_plan_dto(&display).groups;
        let backup_location =
            skill_core::compatibility::safe_text(&engine.backup_root.to_string_lossy());
        plan.metadata
            .insert("createdAt".into(), self.clock.now().to_string());
        let mut target_bindings=Vec::new();
        for change in &plan.directories {target_bindings.push(crate::preview::TargetBinding::capture(&change.path,change.before.clone())?);}
        for file in &plan.files{let binding=FileBinding::read(file.path.clone())?;if binding.data!=file.before{return Err(error("preview_changed"));}auxiliary.push(binding);}
        let (preview_id, expires_at) = self.install_bound(
            "restore",
            plan,
            bindings,
            engine,
            vec![],
            false,
            stamp,
            Some(over),
            auxiliary,
            target_bindings,
        )?;
        Ok(PreviewDto {
            preview_id,
            expires_at,
            groups,
            warnings,
            blocked: false,
            backup_location,
        })
    }
    pub fn commit_restore(&self, id: &str) -> Result<ReceiptDto, AppError> {
        self.commit(id, "restore")
    }
}
fn file_group(
    path: &Path,
    before: Option<&[u8]>,
    after: Option<&[u8]>,
) -> skill_core::plan::PlanGroup {
    let tree = |bytes: Option<&[u8]>| {
        bytes.map(|b| skill_core::PackageTree {
            entries: vec![skill_core::TreeEntry {
                relative_path: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                kind: skill_core::EntryKind::File {
                    bytes: b.to_vec(),
                    unix_executable: false,
                },
            }],
        })
    };
    crate::importing::display_group(path.into(), tree(before).as_ref(), tree(after).as_ref())
}
