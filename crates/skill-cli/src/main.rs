use serde_json::{json, Value};
use skill_app::*;
use skill_core::{
    plan::{ConflictDecision, Disposition, PackageId, SelectionPair, TargetKey},
    AgentId, Platform, Scope,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
const HELP:&str="Skill Hub — 手动管理与分发 Skills\n\n全局选项：--home PATH --library PATH --project PATH --scope user|project\n只读：agents list|validate|preview，doctor，list [--agent ID]，diff --pair PACKAGE=AGENT，backups\n写入：import --source PATH [--new|--replace|--replace-id ID] --yes\n      apply --pair PACKAGE=AGENT [--pair ...] [--replace] --yes\n      restore OPERATION --yes\n      agents save --file PROFILE.json [--enable] --yes\n      agents enable|disable ID --yes\n      agents select-variant ID VARIANT --yes\n任何 --dry-run 均优先于 --yes，只预览，不生成备份或审计。\nrestore OPERATION --dry-run 查看实际恢复计划。\nagents preview|save 支持 --builtin ID 和 --root ROOT_ID=PATH。\n退出码：0 完成/成功预览，2 需要明确确认，1 错误或部分失败。";
#[derive(Default)]
struct Options {
    positionals: Vec<String>,
    home: Option<PathBuf>,
    library: Option<PathBuf>,
    project: Option<PathBuf>,
    scope: Option<Scope>,
    file: Option<PathBuf>,
    source: Option<PathBuf>,
    agent: Option<String>,
    builtin: Option<String>,
    pairs: Vec<String>,
    roots: Vec<String>,
    replace_id: Option<String>,
    yes: bool,
    dry: bool,
    replace: bool,
    new: bool,
    enable: bool,
    help: bool,
}
fn error(code: &'static str) -> AppError {
    AppError { code }
}
fn parse() -> Result<Options, AppError> {
    let raw = std::env::args_os()
        .skip(1)
        .map(|v| v.into_string().map_err(|_| error("argument_encoding")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut args = raw.into_iter();
    let mut o = Options::default();
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .filter(|s| !s.starts_with("--"))
                .ok_or(error("missing_option_value"))
        };
        match arg.as_str() {
            "--home" => o.home = Some(value()?.into()),
            "--library" => o.library = Some(value()?.into()),
            "--project" => o.project = Some(value()?.into()),
            "--scope" => {
                o.scope = Some(match value()?.as_str() {
                    "user" => Scope::User,
                    "project" => Scope::Project,
                    _ => return Err(error("invalid_scope")),
                })
            }
            "--file" => o.file = Some(value()?.into()),
            "--source" => o.source = Some(value()?.into()),
            "--agent" => o.agent = Some(value()?),
            "--builtin" => o.builtin = Some(value()?),
            "--pair" => o.pairs.push(value()?),
            "--root" => o.roots.push(value()?),
            "--replace-id" => o.replace_id = Some(value()?),
            "--yes" => o.yes = true,
            "--dry-run" => o.dry = true,
            "--replace" => o.replace = true,
            "--new" => o.new = true,
            "--enable" => o.enable = true,
            "--help" | "-h" => o.help = true,
            _ if arg.starts_with('-') => return Err(error("unknown_option")),
            _ => o.positionals.push(arg),
        }
    }
    if o.new && (o.replace || o.replace_id.is_some()) {
        return Err(error("conflicting_import_options"));
    }
    Ok(o)
}
fn absolute(path: &Path) -> Result<PathBuf, AppError> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| error("current_directory_unavailable"))?
            .join(path)
    };
    Ok(skill_io::disk::resolved_path(&path)?)
}
fn application(o: &Options) -> Result<App, AppError> {
    let platform = if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Macos
    } else {
        Platform::Linux
    };
    let isolated = o.home.is_some();
    let home = absolute(
        &o.home
            .clone()
            .or_else(|| {
                std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                    .map(PathBuf::from)
            })
            .ok_or(error("home_unavailable"))?,
    )?;
    if home.parent().is_none() {
        return Err(error("invalid_home"));
    }
    let project_root = o.project.as_ref().map(|p| absolute(p)).transpose()?;
    if project_root.as_ref().is_some_and(|p| !p.is_dir()) {
        return Err(error("invalid_project"));
    }
    let appdata = if isolated {
        if cfg!(windows) {
            Some(home.join("AppData/Roaming"))
        } else {
            None
        }
    } else {
        std::env::var_os("APPDATA").map(PathBuf::from)
    };
    let xdg_config = if isolated {
        Some(home.join(".config"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| Some(home.join(".config")))
    };
    let search_path = if isolated {
        vec![]
    } else {
        std::env::var_os("PATH")
            .map(|v| std::env::split_paths(&v).collect())
            .unwrap_or_default()
    };
    let env = HostEnv {
        home: home.clone(),
        appdata,
        xdg_config,
        project_root,
        search_path,
        platform,
    };
    let registry = load_registry(
        &RegistrySource {
            builtins: skill_app::registry::builtin_profiles(),
            custom_dir: home.join(".skill-hub/agents"),
            approvals_path: Some(home.join(".skill-hub/agent-approvals.json")),
        },
        &env,
    )?;
    let mut app = App::new(AppContext {
        env,
        registry,
        scope: o.scope.unwrap_or(Scope::User),
    });
    if o.scope.is_none() {
        if let Ok(v) = app.load_settings() {
            app.context.scope = v.settings.default_scope;
        }
    }
    Ok(app)
}
fn encode<T: serde::Serialize>(v: T) -> Result<Value, AppError> {
    serde_json::to_value(v).map_err(|_| error("output_encoding"))
}
fn receipt(r: ReceiptDto) -> Result<(Value, i32), AppError> {
    let failed = r.outcomes.iter().any(|i| {
        !matches!(
            i.outcome,
            skill_io::Outcome::Written | skill_io::Outcome::Unchanged | skill_io::Outcome::Skipped
        )
    });
    Ok((encode(r)?, if failed { 1 } else { 0 }))
}
fn library(app: &App, o: &Options) -> Result<PathBuf, AppError> {
    if let Some(p) = &o.library {
        absolute(p)
    } else {
        Ok(app.load_settings()?.settings.library_path)
    }
}
fn profiles(app: &App) -> Result<Value, AppError> {
    Ok(Value::Array(app.list_agent_profiles()?.into_iter().map(|p|json!({"id":p.id,"displayName":p.display_name,"source":p.source,"state":format!("{:?}",p.state).to_ascii_lowercase(),"revision":p.revision,"variants":p.variants,"currentVariant":p.current_variant,"roots":p.roots.iter().map(|r|json!({"rootId":r.root_id,"variant":r.variant,"path":skill_core::compatibility::safe_text(&r.path.to_string_lossy()),"writable":r.writable})).collect::<Vec<_>>(),"issues":p.issues})).collect()))
}
fn profile_document(o: &Options) -> Result<Vec<u8>, AppError> {
    if o.file.is_some() && o.builtin.is_some() {
        return Err(error("conflicting_profile_source"));
    }
    if let Some(file) = &o.file {
        Ok(skill_io::snapshot::read_bounded_file(
            &absolute(file)?,
            256 * 1024,
        )?)
    } else if let Some(id) = &o.builtin {
        let id = AgentId::parse(id)?;
        skill_app::registry::builtin_profiles()
            .into_iter()
            .find(|b| skill_core::profile::parse_agent_profile(b).is_ok_and(|p| p.id == id))
            .ok_or(error("unknown_builtin"))
    } else {
        Err(error("profile_source_required"))
    }
}
fn agents(app: &App, o: &Options) -> Result<(Value, i32), AppError> {
    let command = o.positionals.get(1).map(String::as_str).unwrap_or("list");
    match command {
        "list" => Ok((profiles(app)?, 0)),
        "validate" => {
            let p = skill_core::profile::parse_agent_profile(&profile_document(o)?)?;
            Ok((
                json!({"valid":true,"id":p.id,"displayName":p.display_name,"variants":p.variants}),
                0,
            ))
        }
        "preview" | "save" => {
            let doc = profile_document(o)?;
            let p = skill_core::profile::parse_agent_profile(&doc)?;
            let revision = app
                .list_agent_profiles()?
                .iter()
                .find(|v| v.id.as_ref() == Some(&p.id))
                .and_then(|v| v.revision.clone());
            let mut overrides = Vec::new();
            for raw in &o.roots {
                let (id, path) = raw.split_once('=').ok_or(error("invalid_root_override"))?;
                let root = p
                    .roots
                    .iter()
                    .find(|r| r.root_id == id)
                    .ok_or(error("unknown_root"))?;
                overrides.push(skill_app::registry::LocalOverride {
                    root_id: id.into(),
                    platform: app.context.env.platform,
                    scope: root.scope,
                    project: if root.scope == Scope::Project {
                        app.context.env.project_root.clone()
                    } else {
                        None
                    },
                    path: absolute(Path::new(path))?,
                });
            }
            let preview = app.preview_agent_profile(ProfileChangeRequest {
                profile_json: doc,
                expected_revision: revision,
                local_root_overrides: overrides,
                enable_after_save: o.enable,
            })?;
            if command == "preview" || o.dry || !o.yes {
                Ok((
                    encode(preview)?,
                    if command == "save" && !o.dry { 2 } else { 0 },
                ))
            } else {
                receipt(app.commit_agent_profile(&preview.preview_id)?)
            }
        }
        "enable" | "disable" => {
            let id = AgentId::parse(o.positionals.get(2).ok_or(error("agent_required"))?)?;
            let preview =
                json!({"agentId":id,"enabled":command=="enable","requiresConfirmation":!o.yes});
            if o.dry || !o.yes {
                return Ok((preview, if o.dry { 0 } else { 2 }));
            }
            let revision = app.profile_revision(&id)?;
            receipt(app.set_agent_profile_enabled(ProfileEnableRequest {
                agent_id: id,
                enabled: command == "enable",
                expected_revision: revision,
            })?)
        }
        "select-variant" => {
            let id = AgentId::parse(o.positionals.get(2).ok_or(error("agent_required"))?)?;
            let variant = o.positionals.get(3).ok_or(error("variant_required"))?;
            let profile = app
                .list_agent_profiles()?
                .into_iter()
                .find(|p| p.id.as_ref() == Some(&id))
                .ok_or(error("unknown_agent"))?;
            if !profile.variants.contains(variant) {
                return Err(error("invalid_variant"));
            }
            let mut view = app.load_settings()?;
            view.settings.agent_variants.insert(id, variant.clone());
            if o.dry || !o.yes {
                return Ok((
                    json!({"settings":view.settings,"requiresConfirmation":!o.yes}),
                    if o.dry { 0 } else { 2 },
                ));
            }
            let v = app.save_settings(SettingsRequest {
                expected_revision: view.revision,
                settings: view.settings,
            })?;
            Ok((
                json!({"settings":v.settings,"revision":v.revision,"warnings":v.warnings}),
                0,
            ))
        }
        _ => Err(error("unknown_agent_command")),
    }
}
fn execute(app: &App, o: &Options) -> Result<(Value, i32), AppError> {
    match o.positionals.first().map(String::as_str).unwrap_or("help") {
        "agents" => agents(app, o),
        "doctor" => {
            let profiles = profiles(app)?;
            let library = app.load_library(&library(app, o)?)?;
            Ok((
                json!({"profiles":profiles,"library":library,"binaryProbeExecutesCommands":false}),
                0,
            ))
        }
        "list" => {
            if let Some(id) = &o.agent {
                let inventory = app.list_agent_skills(AgentId::parse(id)?)?;
                let failed = !inventory.errors.is_empty();
                let items:Vec<_>=inventory.items.iter().map(|i|json!({"physicalKey":i.physical_key,"metadata":i.metadata.as_ref().map(|m|json!({"name":m.name,"description":m.description})),"sources":i.sources.iter().map(|p|skill_core::compatibility::safe_text(&p.to_string_lossy())).collect::<Vec<_>>(),"files":i.files,"issues":i.issues})).collect();
                Ok((
                    json!({"agent":inventory.agent,"installed":inventory.installed,"items":items,"errors":inventory.errors}),
                    if failed { 1 } else { 0 },
                ))
            } else {
                Ok((encode(app.load_library(&library(app, o)?)?)?, 0))
            }
        }
        "import" => {
            let source = absolute(o.source.as_ref().ok_or(error("source_required"))?)?;
            let lib = library(app, o)?;
            let existing = app.load_library(&lib)?;
            let scan = app.scan_import(ImportRequest {
                source,
                library_path: lib,
            })?;
            let mut choices = Vec::new();
            for item in &scan.items {
                let same: Vec<_> = existing
                    .records
                    .iter()
                    .filter(|r| r.name == item.name)
                    .collect();
                let action = if let Some(id) = &o.replace_id {
                    ImportAction::Replace(PackageId(id.clone()))
                } else if o.replace {
                    if same.len() != 1 {
                        return Err(error("ambiguous_import_replacement"));
                    }
                    ImportAction::Replace(PackageId(same[0].id.clone()))
                } else if o.new || same.is_empty() {
                    ImportAction::New
                } else {
                    ImportAction::Skip
                };
                choices.push(ImportChoice {
                    item_id: item.item_id.clone(),
                    action,
                });
            }
            let preview = app.preview_import(&scan.scan_id, &choices)?;
            if o.dry || !o.yes {
                Ok((encode(preview)?, if o.dry { 0 } else { 2 }))
            } else {
                receipt(app.commit_import(&preview.preview_id)?)
            }
        }
        "diff" | "apply" => {
            let mut selection = Vec::new();
            for raw in &o.pairs {
                let (package, agent) = raw.split_once('=').ok_or(error("invalid_pair"))?;
                if !skill_core::package::is_slug(package) {
                    return Err(error("invalid_package_id"));
                }
                selection.push(SelectionPair {
                    package_id: PackageId(package.into()),
                    agent: AgentId::parse(agent)?,
                });
            }
            let lib = library(app, o)?;
            let request = || DistributionRequest {
                library_path: lib.clone(),
                scope: app.context.scope,
                project_root: app.context.env.project_root.clone(),
                selection: selection.clone(),
                conflict_decisions: BTreeMap::new(),
            };
            let mut preview = app.preview_distribution(request())?;
            if o.replace {
                let decisions = preview
                    .groups
                    .iter()
                    .filter(|g| g.disposition == Disposition::Skipped)
                    .map(|g| (TargetKey(g.target_key.clone()), ConflictDecision::Replace))
                    .collect();
                let mut req = request();
                req.conflict_decisions = decisions;
                preview = app.preview_distribution(req)?;
            }
            let apply = o.positionals[0] == "apply";
            if o.dry || !apply || !o.yes {
                return Ok((encode(preview)?, if apply && !o.dry { 2 } else { 0 }));
            }
            receipt(app.commit_distribution(&preview.preview_id)?)
        }
        "backups" => Ok((
            json!({"history":app.list_history()?,"recoveries":app.list_recoveries()?}),
            0,
        )),
        "restore" => {
            let id = o.positionals.get(1).ok_or(error("operation_required"))?;
            if !o.yes && !o.dry {
                return Ok((
                    json!({"operationId":id,"requiresConfirmation":true,"hint":"使用 --dry-run 查看恢复计划，再以 --yes 现场重新预览并确认。"}),
                    2,
                ));
            }
            let p = app.preview_restore(id)?;
            if o.dry {
                Ok((encode(p)?, 0))
            } else {
                receipt(app.commit_restore(&p.preview_id)?)
            }
        }
        _ => Err(error("unknown_command")),
    }
}
fn main() {
    let result = (|| {
        let options = parse()?;
        if options.help || options.positionals.is_empty() || options.positionals[0] == "help" {
            println!("{HELP}");
            return Ok(0);
        }
        let app = application(&options)?;
        let (output, code) = execute(&app, &options)?;
        println!("{output}");
        Ok::<_, AppError>(code)
    })();
    match result {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("{}", json!({"error":e.code}));
            std::process::exit(1)
        }
    }
}
