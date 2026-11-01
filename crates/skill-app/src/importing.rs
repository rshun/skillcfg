use crate::*;
use crate::{
    library::{metadata, validate, Loaded},
    preview::{directory_change, error, file_change, TreeBinding},
    settings::overlap,
};
use skill_core::{
    plan::{Disposition, DistributionPlan, PackageId, PlanGroup, TargetKey},
    PackageTree,
};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
pub(crate) struct ScanItem {
    pub id: String,
    pub source: PathBuf,
    pub binding: TreeBinding,
}
pub(crate) struct ScanState {
    pub id: String,
    pub library: Loaded,
    pub items: Vec<ScanItem>,
}
impl App {
    pub fn scan_import(&self, req: ImportRequest) -> Result<ImportScan, AppError> {
        self.state.borrow_mut().preview = None;
        let library = self.checked_library(&req.library_path)?;
        let source = skill_io::disk::resolved_path(&req.source)?;
        let (settings, backups, _) = self.settings_data()?;
        if overlap(&source, &library.path)
            || backups
                .iter()
                .chain(std::iter::once(&settings.backup_path))
                .any(|p| skill_io::disk::resolved_path(p).is_ok_and(|p| overlap(&source, &p)))
        {
            return Err(error("import_overlap"));
        }
        let mut paths = Vec::new();
        if req.source.join("SKILL.md").exists() {
            paths.push(req.source);
        } else {
            for (n, entry) in std::fs::read_dir(&req.source)
                .map_err(|_| error("source_unreadable"))?
                .enumerate()
            {
                if n >= 1024 {
                    return Err(error("too_many_import_items"));
                }
                let entry = entry.map_err(|_| error("source_unreadable"))?;
                if entry.path().join("SKILL.md").exists() {
                    paths.push(entry.path());
                }
            }
        }
        paths.sort();
        let mut items = Vec::new();
        let mut views = Vec::new();
        let mut bytes = 0usize;
        let mut seen = BTreeSet::new();
        for path in paths {
            let binding = TreeBinding::read(&path)?;
            if !seen.insert(binding.key.clone()) {
                continue;
            }
            bytes = bytes.saturating_add(crate::preview::tree_bytes(&binding.tree));
            if bytes > 64 * 1024 * 1024 {
                return Err(error("import_too_large"));
            }
            validate(&binding.tree, self.context.env.platform)?;
            let parsed = metadata(&binding.tree)?;
            let id = skill_io::disk::operation_id();
            views.push(ImportItem {
                item_id: id.clone(),
                name: parsed.name,
                source_path: skill_core::compatibility::safe_text(&path.to_string_lossy()),
                actual_path: skill_core::compatibility::safe_text(
                    &binding.actual.to_string_lossy(),
                ),
                issues: vec![],
            });
            items.push(ScanItem {
                id,
                source: path,
                binding,
            });
        }
        let id = skill_io::disk::operation_id();
        self.state.borrow_mut().library = Some(library.path.clone());
        self.state.borrow_mut().scan = Some(ScanState {
            id: id.clone(),
            library,
            items,
        });
        Ok(ImportScan {
            scan_id: id,
            items: views,
        })
    }
    pub fn preview_import(
        &self,
        id: &str,
        choices: &[ImportChoice],
    ) -> Result<PreviewDto, AppError> {
        self.state.borrow_mut().preview = None;
        let scan = self
            .state
            .borrow_mut()
            .scan
            .take()
            .ok_or(error("scan_consumed"))?;
        if scan.id != id {
            return Err(error("invalid_scan"));
        }
        if scan.library.raw != self.checked_library(&scan.library.path)?.raw
            || scan.library.bindings.iter().any(|b| !b.matches())
        {
            return Err(error("library_changed"));
        }
        let mut catalog = scan.library.catalog.clone();
        let original_ids: BTreeSet<_> = catalog.records.iter().map(|r| r.id.clone()).collect();
        let mut plan = skill_io::MutationPlan {
            kind: "import".into(),
            ..Default::default()
        };
        let mut view = DistributionPlan::default();
        let mut outcomes = Vec::new();
        let mut chosen = BTreeSet::new();
        let mut replaced = BTreeSet::new();
        let mut bindings = scan.library.bindings;
        for choice in choices {
            if !chosen.insert(choice.item_id.clone()) {
                return Err(error("duplicate_import_choice"));
            }
            let item = scan
                .items
                .iter()
                .find(|i| i.id == choice.item_id)
                .ok_or(error("unknown_import_item"))?;
            if !item.binding.matches() {
                return Err(error("source_changed"));
            }
            let parsed = metadata(&item.binding.tree)?;
            if matches!(choice.action, ImportAction::Skip) {
                outcomes.push(SafeOutcome {
                    path: skill_core::compatibility::safe_text(&item.source.to_string_lossy()),
                    outcome: skill_io::Outcome::Skipped,
                    reason: None,
                });
                continue;
            }
            let index = match &choice.action {
                ImportAction::Replace(id) => {
                    if !original_ids.contains(&id.0) {
                        return Err(error("unknown_package"));
                    }
                    if !replaced.insert(id.0.clone()) {
                        return Err(error("conflicting_import_choices"));
                    }
                    Some(
                        catalog
                            .records
                            .iter()
                            .position(|r| r.id == id.0)
                            .ok_or(error("unknown_package"))?,
                    )
                }
                _ => None,
            };
            let package_id = index.map_or_else(skill_io::disk::operation_id, |i| {
                catalog.records[i].id.clone()
            });
            let revision = skill_io::disk::operation_id();
            let destination = scan
                .library
                .path
                .join("packages")
                .join(&package_id)
                .join("revisions")
                .join(&revision);
            let before = index.map(|i| bindings[i].tree.clone());
            if before.as_ref() == Some(&item.binding.tree) {
                outcomes.push(SafeOutcome {
                    path: destination.to_string_lossy().into(),
                    outcome: skill_io::Outcome::Unchanged,
                    reason: None,
                });
                continue;
            }
            view.groups.push(display_group(
                destination.clone(),
                before.as_ref(),
                Some(&item.binding.tree),
            ));
            plan.directories.push(directory_change(
                destination,
                None,
                Some(item.binding.tree.clone()),
            ));
            let record = LibraryRecord {
                id: package_id,
                current_revision: revision,
                name: parsed.name.clone(),
                display_name: parsed.name,
                description: parsed.description,
                source_kind: "local".into(),
                source_agent: None,
                source_path: item.source.clone(),
                imported_at: self.clock.now(),
            };
            if let Some(i) = index {
                catalog.records[i] = record;
            } else {
                catalog.records.push(record);
            }
        }
        if !plan.directories.is_empty() {
            let catalog_path = scan.library.path.join("catalog.json");
            plan.files
                .push(self.authority_change(Some(&scan.library.path), None)?);
            plan.files.push(file_change(
                catalog_path,
                scan.library.raw,
                Some(serde_json::to_vec(&catalog).map_err(|_| error("invalid_library"))?),
            ));
        }
        for item in scan.items {
            if chosen.contains(&item.id) {
                bindings.push(item.binding);
            }
        }
        let engine = self.engine(
            vec![scan.library.path.join("packages")],
            vec![
                self.authority_path(),
                scan.library.path.join("catalog.json"),
            ],
            None,
        )?;
        let (preview_id, expires_at) =
            self.install("import", plan, bindings, engine, outcomes, false)?;
        Ok(PreviewDto {
            preview_id,
            expires_at,
            groups: skill_core::diff::redacted_plan_dto(&view).groups,
            warnings: vec![],
            blocked: false,
        })
    }
    pub fn commit_import(&self, id: &str) -> Result<ReceiptDto, AppError> {
        self.commit(id, "import")
    }
}
pub(crate) fn display_group(
    path: PathBuf,
    before: Option<&PackageTree>,
    after: Option<&PackageTree>,
) -> PlanGroup {
    let empty = PackageTree::default();
    let desired = after.unwrap_or(&empty);
    PlanGroup {
        key: TargetKey(path.to_string_lossy().into()),
        path,
        package_ids: vec![PackageId("view".into())],
        selected_agents: vec![],
        possible_readers: vec![],
        before: before.cloned(),
        after: Arc::new(desired.clone()),
        disposition: if before.is_none() {
            Disposition::New
        } else {
            Disposition::Update
        },
        reason: None,
        comparison: skill_core::compare_packages(before.unwrap_or(&empty), desired),
    }
}
