use crate::preview::{directory_change, error, file_change, EditBaseline, TreeBinding};
use crate::*;
use skill_core::{plan::PackageId, EntryKind, PackageTree};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Catalog {
    pub version: u32,
    pub records: Vec<LibraryRecord>,
}
pub(crate) struct Loaded {
    pub path: PathBuf,
    pub catalog: Catalog,
    pub raw: Option<Vec<u8>>,
    pub bindings: Vec<TreeBinding>,
}
pub(crate) fn metadata(tree: &PackageTree) -> Result<skill_core::SkillMetadata, AppError> {
    let bytes = tree
        .entries
        .iter()
        .find_map(|e| {
            if e.relative_path == "SKILL.md" {
                if let EntryKind::File { bytes, .. } = &e.kind {
                    Some(bytes.as_slice())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .ok_or(error("missing_skill_document"))?;
    Ok(skill_core::parse_skill_document(bytes)?)
}
pub(crate) fn validate(tree: &PackageTree, platform: skill_core::Platform) -> Result<(), AppError> {
    let issues = skill_core::validate_package(tree, platform);
    if let Some(issue) = issues.first() {
        return Err(error(issue.code));
    }
    Ok(())
}
impl App {
    pub(crate) fn checked_library(&self, path: &Path) -> Result<Loaded, AppError> {
        let path = self.check_library_location(path)?;
        let raw = skill_io::disk::optional_file(&path.join("catalog.json"))?;
        let catalog = if let Some(bytes) = &raw {
            let c: Catalog = serde_json::from_slice(bytes).map_err(|_| error("invalid_library"))?;
            if c.version != 1 || c.records.len() > 1024 {
                return Err(error("invalid_library"));
            }
            c
        } else {
            if path.exists() {
                let mut entries =
                    std::fs::read_dir(&path).map_err(|_| error("library_unreadable"))?;
                if entries.next().is_some() {
                    return Err(error("missing_catalog"));
                }
            }
            Catalog {
                version: 1,
                records: vec![],
            }
        };
        let mut seen = BTreeSet::new();
        let mut bindings = Vec::new();
        let mut bytes = 0usize;
        for record in &catalog.records {
            if !skill_core::package::is_slug(&record.id)
                || !skill_core::package::is_slug(&record.current_revision)
                || !seen.insert(record.id.clone())
            {
                return Err(error("invalid_catalog_record"));
            }
            let revision = path
                .join("packages")
                .join(&record.id)
                .join("revisions")
                .join(&record.current_revision);
            let binding = TreeBinding::read(&revision)?;
            if binding.link || binding.actual != revision {
                return Err(error("library_revision_link"));
            }
            bytes = bytes.saturating_add(crate::preview::tree_bytes(&binding.tree));
            if bytes > 64 * 1024 * 1024 {
                return Err(error("library_snapshot_too_large"));
            }
            validate(&binding.tree, self.context.env.platform)?;
            let parsed = metadata(&binding.tree)?;
            if parsed.name != record.name || parsed.description != record.description {
                return Err(error("catalog_metadata_mismatch"));
            }
            bindings.push(binding);
        }
        Ok(Loaded {
            path,
            catalog,
            raw,
            bindings,
        })
    }
    pub fn load_library(&self, path: &Path) -> Result<LibraryView, AppError> {
        let loaded = self.checked_library(path)?;
        let mut state = self.state.borrow_mut();
        if state.library.as_ref() != Some(&path.to_path_buf()) {
            state.preview = None;
            state.edits.clear();
            state.scan = None;
        }
        state.library = Some(path.to_path_buf());
        Ok(LibraryView {
            first_use: loaded.raw.is_none(),
            records: loaded.catalog.records.into_iter().map(|mut r|{r.display_name=skill_core::compatibility::safe_text(&r.display_name);r.source_path=PathBuf::from(skill_core::compatibility::safe_text(&r.source_path.to_string_lossy()));r.source_kind=skill_core::compatibility::safe_text(&r.source_kind);r.description=skill_core::compatibility::safe_text(&r.description);r}).collect(),
        })
    }
    pub fn read_package(&self, id: &PackageId) -> Result<SafePackageView, AppError> {
        let library = self
            .state
            .borrow()
            .library
            .clone()
            .ok_or(error("library_not_loaded"))?;
        let loaded = self.checked_library(&library)?;
        let index = loaded
            .catalog
            .records
            .iter()
            .position(|r| r.id == id.0)
            .ok_or(error("unknown_package"))?;
        let record = &loaded.catalog.records[index];
        let tree = &loaded.bindings[index].tree;
        let document = tree.entries.iter().find_map(|e| {
            if e.relative_path == "SKILL.md" {
                if let EntryKind::File { bytes, .. } = &e.kind {
                    String::from_utf8(bytes.clone()).ok()
                } else {
                    None
                }
            } else {
                None
            }
        });
        let files = tree
            .entries
            .iter()
            .map(|e| skill_core::compatibility::safe_text(&e.relative_path))
            .collect();
        let token = skill_io::disk::operation_id();
        let revision_id = record.current_revision.clone();
        let package_id = record.id.clone();
        let stamp = self.stamp()?;
        let mut state = self.state.borrow_mut();
        if state.edits.len() >= 16
            || state
                .edits
                .values()
                .flat_map(|e| e.bindings.iter())
                .map(|b| crate::preview::tree_bytes(&b.tree))
                .sum::<usize>()
                .saturating_add(
                    loaded
                        .bindings
                        .iter()
                        .map(|b| crate::preview::tree_bytes(&b.tree))
                        .sum::<usize>(),
                )
                > 64 * 1024 * 1024
        {
            state.edits.clear();
            state.scan = None;
        }
        state.edits.insert(
            token.clone(),
            EditBaseline {
                catalog: loaded.raw,
                library,
                id: id.0.clone(),
                stamp,
                bindings: loaded.bindings,
            },
        );
        Ok(SafePackageView {
            package_id,
            revision_id,
            document,
            edit_token: Some(token),
            files,
            issues: vec![],
        })
    }
    pub fn save_skill_document(&self, req: EditDocumentRequest) -> Result<ReceiptDto, AppError> {
        self.state.borrow_mut().preview = None;
        let baseline = self
            .state
            .borrow_mut()
            .edits
            .remove(&req.edit_token)
            .ok_or(error("edit_token_consumed"))?;
        if baseline.id != req.package_id.0
            || baseline.stamp != self.stamp()?
            || baseline.bindings.iter().any(|b| !b.matches())
        {
            return Err(error("edit_baseline_changed"));
        }
        let mut loaded = self.checked_library(&baseline.library)?;
        if loaded.raw != baseline.catalog {
            return Err(error("edit_baseline_changed"));
        }
        let index = loaded
            .catalog
            .records
            .iter()
            .position(|r| r.id == req.package_id.0)
            .ok_or(error("unknown_package"))?;
        let mut tree = loaded.bindings[index].tree.clone();
        let document = tree
            .entries
            .iter_mut()
            .find(|e| e.relative_path == "SKILL.md")
            .ok_or(error("missing_skill_document"))?;
        let executable = matches!(
            &document.kind,
            EntryKind::File {
                unix_executable: true,
                ..
            }
        );
        document.kind = EntryKind::File {
            bytes: req.document.into_bytes(),
            unix_executable: executable,
        };
        validate(&tree, self.context.env.platform)?;
        let parsed = metadata(&tree)?;
        let revision = skill_io::disk::operation_id();
        let record = &mut loaded.catalog.records[index];
        record.name = parsed.name;
        record.description = parsed.description;
        record.current_revision = revision.clone();
        let destination = loaded
            .path
            .join("packages")
            .join(&record.id)
            .join("revisions")
            .join(revision);
        let catalog_path = loaded.path.join("catalog.json");
        let authority = self.authority_change(Some(&loaded.path), None)?;
        let after = serde_json::to_vec(&loaded.catalog).map_err(|_| error("invalid_library"))?;
        let plan = skill_io::MutationPlan {
            retained: vec![(
                loaded.bindings[index].path.clone(),
                loaded.bindings[index].tree.clone(),
            )],
            directories: vec![directory_change(destination, None, Some(tree))],
            files: vec![
                authority,
                file_change(catalog_path.clone(), loaded.raw, Some(after)),
            ],
            kind: "edit".into(),
            ..Default::default()
        };
        let engine = self.engine(
            vec![loaded.path.join("packages")],
            vec![self.authority_path(), catalog_path],
            None,
        )?;
        Ok(crate::preview::safe_receipt(
            engine.apply(engine.prepare(&plan)?),
        ))
    }
}
