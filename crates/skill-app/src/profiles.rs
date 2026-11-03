use crate::*;
use crate::{
    preview::{error, file_change},
    registry::{approval_matches, proof_current, RootProof},
    settings::overlap,
};
use skill_core::{
    profile::{parse_agent_profile, AgentProfile, Base},
    AgentId, Scope,
};
use std::{collections::BTreeSet, path::PathBuf};
type ApprovalData = (Vec<Approval>, Option<Vec<u8>>, PathBuf);
pub(crate) struct ProfileBaseline {
    pub id: AgentId,
    pub path: Option<PathBuf>,
    pub raw: Option<Vec<u8>>,
}
impl App {
    fn profile_file(&self, id: &AgentId) -> Result<ProfileBaseline, AppError> {
        if !id.as_str().starts_with("custom.") {
            let raw = self
                .source
                .builtins
                .iter()
                .find(|b| parse_agent_profile(b).is_ok_and(|p| p.id == *id))
                .ok_or(error("unknown_agent"))?
                .clone();
            return Ok(ProfileBaseline {
                id: id.clone(),
                path: None,
                raw: Some(raw),
            });
        }
        if !id
            .as_str()
            .strip_prefix("custom.")
            .is_some_and(skill_core::package::is_slug)
        {
            return Err(error("invalid_namespace"));
        }
        let canonical = self.source.custom_dir.join(format!("{}.json", id.as_str()));
        let mut found = None;
        match std::fs::read_dir(&self.source.custom_dir) {
            Ok(entries) => {
                for (n, entry) in entries.enumerate() {
                    if n >= 256 {
                        return Err(error("too_many_profiles"));
                    }
                    let path = entry.map_err(|_| error("registry_read_failed"))?.path();
                    if path.extension().is_none_or(|e| e != "json") {
                        continue;
                    }
                    let bytes = match skill_io::snapshot::read_bounded_file(&path, 256 * 1024) {
                        Ok(b) => b,
                        Err(e) if path == canonical => return Err(e.into()),
                        Err(_) => continue,
                    };
                    let parsed = parse_agent_profile(&bytes);
                    let same = parsed.as_ref().is_ok_and(|p| p.id == *id);
                    if path == canonical && parsed.as_ref().is_ok_and(|p| p.id != *id) {
                        return Err(error("profile_filename_conflict"));
                    }
                    if same || path == canonical {
                        if found.is_some() {
                            return Err(error("duplicate_id"));
                        }
                        found = Some((path, bytes));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(error("registry_read_failed")),
        }
        let (path, raw) = found.map_or((canonical, None), |(p, b)| (p, Some(b)));
        Ok(ProfileBaseline {
            id: id.clone(),
            path: Some(path),
            raw,
        })
    }
    pub fn profile_revision(&self, id: &AgentId) -> Result<String, AppError> {
        let baseline = self.profile_file(id)?;
        let revision = skill_io::disk::operation_id();
        let mut state = self.state.borrow_mut();
        state.profile_tokens.retain(|_, b| b.id != *id);
        state.profile_tokens.insert(revision.clone(), baseline);
        Ok(revision)
    }
    pub fn read_agent_profile_document(&self, id: &AgentId) -> Result<(String, String), AppError> {
        let baseline=self.profile_file(id)?;
        let raw=baseline.raw.as_ref().ok_or(error("unknown_agent"))?;
        parse_agent_profile(raw)?;
        let document=String::from_utf8(raw.clone()).map_err(|_|error("invalid_profile"))?;
        let document=skill_core::compatibility::safe_text(&document);
        let revision=skill_io::disk::operation_id();let mut state=self.state.borrow_mut();
        state.profile_tokens.retain(|_,b|b.id!=*id);state.profile_tokens.insert(revision.clone(),baseline);
        Ok((document,revision))
    }
    fn approval_data(&self) -> Result<ApprovalData, AppError> {
        let path = self
            .source
            .approvals_path
            .clone()
            .ok_or(error("missing_approvals_path"))?;
        let raw = skill_io::disk::optional_file(&path)?;
        let approvals = if let Some(bytes) = &raw {
            let a: Vec<Approval> =
                serde_json::from_slice(bytes).map_err(|_| error("invalid_approvals"))?;
            if a.len() > 256 {
                return Err(error("too_many_approvals"));
            }
            a
        } else {
            vec![]
        };
        Ok((approvals, raw, path))
    }
    fn proof(
        &self,
        path: PathBuf,
        probe: bool,
        id: String,
        variant: String,
        scope: Scope,
    ) -> Result<RootProof, AppError> {
        let actual = skill_io::disk::resolved_path(&path)?;
        let mut anchor = if probe {
            actual
                .parent()
                .ok_or(error("invalid_probe_path"))?
                .to_owned()
        } else {
            actual.clone()
        };
        while !anchor.exists() {
            if !anchor.pop() {
                return Err(error("invalid_root"));
            }
        }
        let anchor = skill_io::disk::resolved_path(&anchor)?;
        let anchor_key = skill_io::paths::physical_key(&anchor)?;
        Ok(RootProof {
            probe,
            root_id: id,
            variant,
            scope,
            project: if scope == Scope::Project {
                self.context
                    .env
                    .project_root
                    .as_ref()
                    .map(|p| skill_io::disk::resolved_path(p))
                    .transpose()?
            } else {
                None
            },
            logical_path: path,
            actual_path: actual,
            anchor,
            anchor_key,
        })
    }
    pub fn preview_agent_profile(
        &self,
        req: ProfileChangeRequest,
    ) -> Result<ProfilePreviewDto, AppError> {
        self.state.borrow_mut().preview = None;
        let profile = parse_agent_profile(&req.profile_json)?;
        let existing = self.profile_file(&profile.id)?;
        if let Some(revision) = &req.expected_revision {
            let baseline = self
                .state
                .borrow_mut()
                .profile_tokens
                .remove(revision)
                .ok_or(error("profile_revision_unknown"))?;
            if baseline.id != profile.id
                || baseline.path != existing.path
                || baseline.raw != existing.raw
            {
                return Err(error("profile_changed"));
            }
        } else if existing.raw.is_some() {
            return Err(error("profile_revision_required"));
        }
        let document = if existing.path.is_none() {
            let raw = existing.raw.clone().ok_or(error("unknown_agent"))?;
            if parse_agent_profile(&raw)? != profile {
                return Err(error("builtin_definition_immutable"));
            }
            raw
        } else {
            req.profile_json
        };
        let stamp = self.stamp()?;
        let mut seen = BTreeSet::new();
        let (settings, backups, _) = self.settings_data()?;
        let (authority, _) = self.authority()?;
        let private_roots = authority
            .libraries
            .into_iter()
            .chain(authority.backups)
            .chain(backups.iter().cloned())
            .chain([settings.library_path.clone()])
            .map(|p| skill_io::disk::resolved_path(&p))
            .collect::<Result<Vec<_>, _>>()?;
        for o in &req.local_root_overrides {
            if o.platform != self.context.env.platform
                || !seen.insert((o.root_id.clone(), o.scope))
                || !profile.roots.iter().any(|r| {
                    r.root_id == o.root_id
                        && r.scope == o.scope
                        && r.platforms.contains(&o.platform)
                })
            {
                return Err(error("invalid_root_override"));
            }
            let actual = self.basic_location(&o.path)?;
            if overlap(
                &actual,
                &skill_io::disk::resolved_path(&settings.library_path)?,
            ) || private_roots.iter().any(|b| overlap(&actual, b))
            {
                return Err(error("protected_root"));
            }
            if o.scope == Scope::Project
                && (o.project.as_ref() != self.context.env.project_root.as_ref()
                    || o.project.is_none())
            {
                return Err(error("override_project_required"));
            }
        }
        let mut approval = Approval {
            id: profile.id.clone(),
            document: document.clone(),
            platform: self.context.env.platform,
            home: self.context.env.home.clone(),
            appdata: self.context.env.appdata.clone(),
            xdg_config: self.context.env.xdg_config.clone(),
            enabled: req.enable_after_save,
            overrides: req.local_root_overrides,
            root_proofs: None,
        };
        let mut ctx = self.runtime_context()?;
        ctx.registry
            .entries
            .retain(|e| e.profile.as_ref().is_none_or(|p| p.id != profile.id));
        ctx.registry.entries.push(ProfileEntry {
            profile: Some(profile.clone()),
            state: ProfileState::Enabled,
            raw: document.clone(),
            issues: vec![],
        });
        ctx.registry.approvals.retain(|a| a.id != profile.id);
        ctx.registry.approvals.push(approval.clone());
        let mut proofs = Vec::new();
        let mut resolved_roots = Vec::new();
        let mut probe_paths = Vec::new();
        let mut warnings = Vec::new();
        for variant in &profile.variants {
            for scope in [Scope::User, Scope::Project] {
                if scope == Scope::Project && ctx.env.project_root.is_none() {
                    continue;
                }
                let mut local = ctx.clone();
                local.scope = scope;
                local
                    .registry
                    .selected_variants
                    .insert(profile.id.clone(), variant.clone());
                let roots = crate::roots::resolve_profile_roots(&profile, &local)?;
                if roots.issues.contains(&"protected_root") {
                    return Err(error("protected_root"));
                }
                warnings.extend(roots.issues.iter().map(|s| s.to_string()));
                for r in roots.roots {
                    let actual = self.basic_location(&r.path)?;
                    if overlap(
                        &actual,
                        &skill_io::disk::resolved_path(&settings.library_path)?,
                    ) || private_roots.iter().any(|b| overlap(&actual, b))
                    {
                        return Err(error("protected_root"));
                    }
                    resolved_roots.push(skill_core::compatibility::safe_text(
                        &actual.to_string_lossy(),
                    ));
                    proofs.push(self.proof(r.path, false, r.root_id, variant.clone(), scope)?);
                }
            }
        }
        for p in &profile.probes {
            if let Some(base) = ctx.env.base(p.base) {
                let path = base.join(&p.relative);
                probe_paths.push(skill_core::compatibility::safe_text(
                    &path.to_string_lossy(),
                ));
                proofs.push(self.proof(
                    path,
                    true,
                    p.probe_id.clone(),
                    String::new(),
                    if p.base == Base::Project {
                        Scope::Project
                    } else {
                        Scope::User
                    },
                )?);
            }
        }
        if proofs.len() > 1024 {
            return Err(error("too_many_root_proofs"));
        }
        let extra_paths = proofs.iter().map(|p| p.logical_path.clone()).collect();
        if proofs.iter().any(|p| !proof_current(p)) {
            return Err(error("profile_context_changed"));
        }
        approval.root_proofs = Some(proofs);
        let (mut approvals, before, approval_path) = self.approval_data()?;
        approvals.retain(|a| a.id != profile.id);
        approvals.push(approval);
        let after = serde_json::to_vec(&approvals).map_err(|_| error("invalid_approvals"))?;
        let mut plan = skill_io::MutationPlan {
            kind: "profile".into(),
            ..Default::default()
        };
        let mut files = vec![approval_path.clone()];
        if let Some(path) = existing.path {
            files.push(path.clone());
            plan.files
                .push(file_change(path, existing.raw, Some(document)));
        }
        plan.files
            .push(file_change(approval_path, before, Some(after)));
        let engine = self.engine(vec![], files, None)?;
        if stamp != self.stamp()? {
            return Err(error("profile_context_changed"));
        }
        let over = crate::binding::ContextOverride {
            scope: self.context.scope,
            project: self.context.env.project_root.clone(),
            recovery: false,
            extra_paths,
        };
        let bound_stamp = self.snapshot(Some(&over))?;
        let (preview_id, _) = self.install_bound(
            "profile",
            plan,
            vec![],
            engine,
            vec![],
            false,
            bound_stamp,
            Some(over),
            vec![],
            vec![],
        )?;
        Ok(ProfilePreviewDto {
            preview_id,
            profile_id: profile.id,
            resolved_roots,
            probe_paths,
            variants: profile.variants,
            blocked: false,
            warnings,
        })
    }
    pub fn commit_agent_profile(&self, id: &str) -> Result<ReceiptDto, AppError> {
        self.commit(id, "profile")
    }
    pub fn set_agent_profile_enabled(
        &self,
        req: ProfileEnableRequest,
    ) -> Result<ReceiptDto, AppError> {
        self.state.borrow_mut().preview = None;
        let baseline = self
            .state
            .borrow_mut()
            .profile_tokens
            .remove(&req.expected_revision)
            .ok_or(error("profile_revision_unknown"))?;
        let current = self.profile_file(&req.agent_id)?;
        if baseline.id != current.id || baseline.path != current.path || baseline.raw != current.raw
        {
            return Err(error("profile_changed"));
        }
        let raw = current.raw.ok_or(error("unknown_agent"))?;
        let profile = parse_agent_profile(&raw)?;
        let (mut approvals, before, path) = self.approval_data()?;
        let existing = approvals.iter().position(|a| a.id == req.agent_id);
        if req.enabled {
            let a = existing
                .map(|i| &approvals[i])
                .ok_or(error("profile_authorization_required"))?;
            let entry = ProfileEntry {
                profile: Some(profile),
                state: ProfileState::Enabled,
                raw: raw.clone(),
                issues: vec![],
            };
            if !approval_matches(a, &entry, &self.context.env)
                || (a.root_proofs.is_none() && current.path.is_some())
            {
                return Err(error("profile_authorization_required"));
            }
        }
        if let Some(i) = existing {
            approvals[i].enabled = req.enabled;
        } else if current.path.is_none() {
            approvals.push(Approval {
                id: req.agent_id,
                document: raw,
                platform: self.context.env.platform,
                home: self.context.env.home.clone(),
                appdata: self.context.env.appdata.clone(),
                xdg_config: self.context.env.xdg_config.clone(),
                enabled: false,
                overrides: vec![],
                root_proofs: None,
            });
        } else {
            return Err(error("profile_authorization_required"));
        }
        let plan = skill_io::MutationPlan {
            files: vec![file_change(
                path.clone(),
                before,
                Some(serde_json::to_vec(&approvals).map_err(|_| error("invalid_approvals"))?),
            )],
            kind: "profile".into(),
            ..Default::default()
        };
        let engine = self.engine(vec![], vec![path], None)?;
        Ok(crate::preview::safe_receipt(
            engine.apply(engine.prepare(&plan)?),
        ))
    }
    pub(crate) fn profile_authorized(
        &self,
        profile: &AgentProfile,
        ctx: &AppContext,
        write: bool,
    ) -> Result<(), AppError> {
        if self
            .source
            .builtins
            .iter()
            .any(|b| parse_agent_profile(b).is_ok_and(|p| p.id == profile.id))
        {
            return Ok(());
        }
        let approval = ctx
            .registry
            .approvals
            .iter()
            .find(|a| a.id == profile.id)
            .ok_or(error("profile_authorization_required"))?;
        let Some(proofs) = &approval.root_proofs else {
            if write {
                return Err(error("profile_authorization_required"));
            }
            return Ok(());
        };
        let variant = ctx
            .registry
            .selected_variants
            .get(&profile.id)
            .unwrap_or(&profile.default_variant);
        let project = if ctx.scope == Scope::Project {
            ctx.env
                .project_root
                .as_ref()
                .map(|p| skill_io::disk::resolved_path(p))
                .transpose()?
        } else {
            None
        };
        for root in crate::roots::resolve_profile_roots(profile, ctx)?.roots {
            if !proofs.iter().any(|p| {
                !p.probe
                    && p.root_id == root.root_id
                    && p.variant == *variant
                    && p.scope == ctx.scope
                    && p.project == project
                    && p.logical_path == root.path
                    && proof_current(p)
            }) {
                return Err(error("profile_root_reauthorization_required"));
            }
        }
        for probe in &profile.probes {
            if let Some(base) = ctx.env.base(probe.base) {
                let scope = if probe.base == Base::Project {
                    Scope::Project
                } else {
                    Scope::User
                };
                if !proofs.iter().any(|p| {
                    p.probe
                        && p.root_id == probe.probe_id
                        && p.scope == scope
                        && p.logical_path == base.join(&probe.relative)
                        && proof_current(p)
                }) {
                    return Err(error("profile_probe_reauthorization_required"));
                }
            }
        }
        Ok(())
    }
}
