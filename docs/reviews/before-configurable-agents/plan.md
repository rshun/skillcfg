# Skill Hub Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建能读取、统一管理并在 Codex、Claude Code、Antigravity、Grok Build 之间手动分发完整 Skill 包的本地工具。

**Architecture:** Rust 纯核心、目录 IO 引擎和宿主无关应用服务分层，CLI 与 Tauri 调用同一应用服务。React 使用已确认的蓝白工作台设计，真实文件写入只接受后端一次性预览。按物理 Skill 目录执行备份、归档、激活和恢复，明确部分成功与并发边界。

**Tech Stack:** 候选为 Rust、serde/serde_json/serde_yaml/toml_edit/thiserror/clap、React 19、TypeScript、Vite、Tauri 2；确切版本和新增依赖须在实施前获用户确认，不默认安装。

**Spec:** [完整设计方案](../specs/2026-11-01-skill-hub-design.md)

状态：实施前计划，全部任务未执行；只有视觉方向获确认。本轮不生成产品代码、依赖清单或 CI 文件。

## Global Constraints

- 每次预览后手动应用；禁止自动同步。保存、编辑、导入和启动均不自动分发。
- 源包入口可以是符号链接或 junction；包内部链接、设备文件和其他特殊条目首版阻止导入/应用。
- 单文件 4 MiB，单包 16 MiB，最多 1,024 个文件/目录条目，相对深度不超过 16，单次预览原始内容总和不超过 64 MiB。
- 一次性预览有效期 600 秒；取消、新预览、过期及任何确认尝试消费旧预览。
- 完整目录复制，原文和资源按字节保留；旧包通过归档移出加载目录，不自动永久删除。
- 先备份后覆盖；共享实际目录只备份和应用一次；不承诺跨多个目标的全局原子性。
- Rust 核心/CLI 至少 1.85，桌面至少 1.90；Node 最低版本在前置核实时固定，不猜测。
- 不在 master/main 修改项目，不新增/升级/安装未经批准的依赖，不执行删除或危险 Git 操作。
- 自动化仅使用隔离 HOME/APPDATA/项目与演示资料；不输出或上传真实凭据。
- 未获授权不 commit/push、打标签、发布或部署；计划里的提交信息只是候选。
- 用户要求与红线优先于任务模板；锁文件恢复、覆盖与任何例外均不能被计划视作预授权。

## Review Focus

1. 同一个物理根同时被多个 Agent 或不同别名读取时，只改一次并列出额外影响；由 Task 2/3/6 验证。
2. 目标在预览后增加未列出的文件、改变符号链接或仅改变二进制内容时，旧确认必须被拒绝；由 Task 2/6 验证。
3. 原目标不存在、旧包已有额外文件或进程在两次 rename 之间退出时，可保留全部资料并明确恢复；由 Task 4/6 验证。
4. 中文/长路径、Windows 大小写碰撞、Unix 执行位和混合换行不能造成错误比较、丢文件或越界写入；由 Task 1/2/4/12 验证。
5. 前端切换范围、项目或 Agent 时，旧异步结果、旧选择和旧预览不能污染新上下文；由 Task 8/9/10 验证。

## 0. 开始实施前的门槛

- [ ] 用户审阅完整设计与本计划，记录 D1–D9 是否调整；本轮授权只覆盖设计和计划。
- [ ] 用户在 skillcfg 新建/切换开发分支，例如 `git switch -c dev/skill-hub-design`。
- [ ] 重新运行 `git branch --show-current`、`git status --short`、`git branch -vv`；若仍为 master/main 则停止仓库修改。
- [ ] 核对远端及 upstream，在授权的开发分支同步最新代码；只能快进，不能隐式 merge/rebase/stash。新分支无 upstream 时先核对来源，不盲目 `git pull`。
- [ ] 将本提案的 `docs/` 归档进仓库；同名文件已存在则先比较和备份，不能覆盖无关修改。
- [ ] 读取 mcpconfig 的 Cargo/package 锁文件和 CI 版本，列出本项目需要的确切依赖、用途、来源和安装影响；请求用户批准依赖清单与锁文件生成方式。
- [ ] 核对现有工具和缓存；用户没有批准前不创建含新增依赖的 manifest、不安装依赖，不触发隐式下载。
- [ ] 明确是否已授权推送/运行 GitHub CI。下文 Run 命令是 CI 验证命令，不代表现在已运行或已有权限执行。

每项实现遵循“先失败测试 → 验证失败 → 最小实现 → 验证通过 → 独立评审”。小型纯文案/样式调整用交互/截图核对，不为镜像实现凑测试。失败证据由临时测试提交或 CI 制品保存，不把故意失败的代码留在已验收基线。

## 1. 预期文件结构

```text
Cargo.toml / Cargo.lock
crates/
  skill-core/src/{lib,model,package,compatibility,diff,plan,error}.rs
  skill-core/tests/{packages,planning}.rs
  skill-io/src/{lib,snapshot,paths,backup,transaction,journal,lock,audit}.rs
  skill-io/tests/{snapshot,transactions,recovery}.rs
  skill-app/src/{lib,env,adapters,detect,library,settings,preview,importing,distribution,restore,history}.rs
  skill-app/tests/{discovery,library,importing,distribution,restore}.rs
  skill-cli/src/main.rs
  skill-cli/tests/commands.rs
apps/web/
  src/{App,main}.tsx
  src/{api,types,mock,selection,styles}.ts|css
  src/components/{Icon,Dialog,LocationForm,Status,AppShell}.tsx
  src/pages/{Library,Agents,Import,SkillEditor,Preview,History,Restore,Settings}.tsx
  tests/{selection,contract,gui}.test.mjs
  src-tauri/src/{lib,commands,dto,directory}.rs
  src-tauri/{Cargo.toml,build.rs,tauri.conf.json,capabilities/default.json}
tests/fixtures/{agents,packages,contracts}/
.github/workflows/{ci,ui-review,release}.yml
docs/{AGENT-COMPATIBILITY,DEPENDENCIES,SAFETY,MANUAL-VALIDATION}.md
docs/prototype/skill-hub-design.html
README.md
```

仅在对应任务需要时创建文件。测试 fixtures 全为人工合成，不复制真实用户配置。产品类型、命令和路径名称如下文冻结；实现者可拆分内部函数，不能随意变更相邻任务依赖的接口。

## 2. 跨任务接口约定

核心类型放 `skill-core/src/model.rs`；仅内部使用，禁止整体 Serialize 到 IPC：

```rust
pub enum AgentId { Codex, ClaudeCode, Antigravity, Grok }
pub enum Scope { User, Project }
pub enum Platform { Windows, Macos, Linux }
pub struct PackageId(pub String);
pub struct SkillMetadata { pub name: String, pub description: String }
pub enum EntryKind { Directory, File { bytes: Vec<u8>, unix_executable: bool } }
pub struct TreeEntry { pub relative_path: String, pub kind: EntryKind }
pub struct PackageTree { pub entries: Vec<TreeEntry> } // 相对路径有序且唯一
pub struct LibraryPackage { pub id: PackageId, pub metadata: SkillMetadata, pub tree: PackageTree }
pub struct SelectionPair { pub package_id: PackageId, pub agent: AgentId }
pub enum ConflictDecision { Skip, Replace }
```

`TargetKey` 是规范化物理目标键，由 IO 生成。`TargetSnapshot` 包含 `key/path/tree: Option<PackageTree>/selected_agents/possible_readers`；`None` 表示原本不存在，空目录为 `Some(empty)`，必须区分。

`DistributionPlan` 含按 TargetKey 去重的 `DirectoryChange`、脱敏前的内部 diff、阻止与跳过原因。`DirectoryChange` 固定 `key/path/before: Option<PackageTree>/after: Option<PackageTree>`；after=None 只用于明确恢复“不存在”，由归档实现，不删除。

`LibraryRecord` 固定 id/current_revision/name/display_name/description/source_kind/source_agent/source_path/imported_at；库中的完整树保存在 `packages/<id>/revisions/<revision_id>/`。catalog 字段必须与引用版本的 SKILL.md 一致，否则阻止分发。`DistributionRecord` 保存 package_id/revision_id/name_at_write/scope/project/physical_path/operation_id，供更名后的旧位置提示和按原路径恢复。

IO 的 `MutationPlan` 同时承载目录变更和少量 JSON 设置/索引文件的 `FileChange { path, before: Option<Vec<u8>>, after: Option<Vec<u8>> }`，所有路径都必须先通过注册根校验。after=None 仅用于明确恢复原先不存在的文件，通过归档实现；普通文件变更采用原子文件写。多个独立目录组按回执逐组执行，不能隐含全局事务。

`AppContext { scope, project_root: Option<PathBuf>, platform, agent_profiles }` 由后端构建和校验；前端只传作用域和用户明确选定的项目目录，不能伪造平台或最终写入根。

公开 `PreviewDto` 只含 `previewId/expiresAt/groups/warnings/blocked`；group 含真实安全路径、selectedAgents、possibleReaders、changeType、fileDiffs 和 counts，绝不包含原始快照字节。`ReceiptDto` 含 operationId、backupLocation、outcomes，每项使用设计中的七类状态。

请求/查看类型在相应用例模块中定义，并以 contract fixtures 固定：

- `ImportRequest { library_path, source: Agent(AgentId) | Directory(PathBuf) }`；`ImportChoice { source_item_id, action: Skip | Add | Replace(PackageId) | KeepSeparate }`。
- `ImportScan { scan_id, expires_at, items }`，item 含后台扫描条目 ID、元数据、安全来源路径、文件清单和 issues，不含 raw bytes。
- `EditDocumentRequest { package_id, edit_token, document: String }`；`SafePackageView` 返回 revision_id、元数据、安全原文、文件清单、issues 和 edit_token。token 在后端绑定整个编辑基线；疑似敏感原文不返回可保存的掩码文本，改为禁止编辑并提示本地处理。
- `GuiSettings { version: 1, library_path, backup_path, default_scope, last_project_dir }`；`SettingsRequest { expected_revision, settings: GuiSettings }`，基线变化拒绝保存。
- `HistoryItem { operation_id, kind, created_at, scope, project, outcomes, backup_location }`；安全错误一律 code/message，不包含 YAML 解析器原文。
- 上述 Rust 字段经 serde 统一映射 camelCase，enum 的 IPC 字面量在 fixture 中固定；原始内部树不实现 IPC 序列化。

## Task 1：完整包模型、原文保留与可移植性校验

**Files:** Create `Cargo.toml`、`crates/skill-core/{Cargo.toml,src/lib.rs,src/model.rs,src/package.rs,src/compatibility.rs,src/error.rs}`、`crates/skill-core/tests/packages.rs`、`tests/fixtures/packages/`、`docs/DEPENDENCIES.md`；在依赖获批后创建锁文件和基础 Rust CI。

**Interfaces:**
- Produces `parse_skill_document(bytes: &[u8]) -> Result<SkillMetadata, CoreError>`。
- Produces `validate_package(tree: &PackageTree, platform: Platform) -> Vec<PackageIssue>`；issue 固定 code/path/severity，不能夹带原文。
- Produces `compare_packages(left: &PackageTree, right: &PackageTree) -> PackageComparison`，包含新增/变化/缺失项和执行位差异。
- `PackageIssue` 与 `PackageComparison` 定义在 model.rs，消费上文核心类型。

- [ ] 先写失败用例 `keeps_unknown_frontmatter_and_crlf_bytes`、`rejects_duplicate_yaml_keys`、`rejects_non_string_name`、`rejects_65_char_name`、`detects_binary_and_executable_change`、`rejects_windows_case_collision`；断言原输入 bytes 不变、同样时间戳不影响比较、错误不含测试凭据。
- [ ] Run `cargo test --locked -p skill-core --test packages`；确认因接口缺失/断言失败而红，不把下载失败当功能红灯。
- [ ] 最小实现纯解析和校验；4 MiB/16 MiB/1,024/16 的边界同时测试等于上限通过、超一单位拒绝。标准库做字节比较，不引入额外摘要依赖。
- [ ] 再运行相同测试；确认 UTF-8 BOM、中文、LF/CRLF、未知 YAML 字段和二进制样例通过，并在批准的 CI 留证。
- [ ] 评审包边界、正则/name 规则、错误脱敏和未授权依赖；候选提交 `feat(core): model and validate complete skill packages`，仅获提交授权后暂存精确文件并做敏感检查。

## Task 2：安全快照与四个 Agent 的只读发现

**Files:** Create `crates/skill-io/{Cargo.toml,src/lib.rs,src/snapshot.rs,src/paths.rs}`、`crates/skill-app/{Cargo.toml,src/lib.rs,src/env.rs,src/adapters.rs,src/detect.rs}`、`crates/skill-io/tests/snapshot.rs`、`crates/skill-app/tests/discovery.rs`、`tests/fixtures/agents/`、`docs/AGENT-COMPATIBILITY.md`。

**Interfaces:**
- Consumes Task 1 `PackageTree/parse_skill_document/validate_package`。
- Produces `read_package_snapshot(root: &Path, limits: &ScanLimits) -> Result<PackageSnapshot, IoError>`；snapshot 包含实际路径、物理键和完整树。
- Produces `resolve_profile_roots(profile: &AgentProfile, ctx: &AppContext) -> Result<ResolvedRoots, AppError>` 与 `App::list_agent_skills(agent: AgentId) -> Result<AgentInventory, AppError>`。
- `AgentProfile/ResolvedRoots/AgentInventory` 定义在 adapters.rs；profile 含版本证据、surface、read_roots/write_root/possible_readers 规则，inventory 包含全部来源和别名。

- [ ] 先写 `root_alias_is_one_package_with_two_sources`、`internal_link_is_blocked`、`junction_target_is_not_writable`、`invalid_primary_root_is_not_empty`、`project_parent_is_read_only`；断言扫描不创建任何库/备份/目标文件。
- [ ] Run `cargo test --locked -p skill-io --test snapshot` 与 `cargo test --locked -p skill-app --test discovery`；红灯要落在指定失败行为。
- [ ] 最小实现有界扫描、同一实际路径去重、包入口链接只读解析、内部链接阻止、权限错误分类。读取前后检查目录树变化，完整读取后才能标为有效快照。
- [ ] 记录四个 Agent 的官方证据、样例版本、当前/兼容目录和读写差异；Antigravity 档案区分 App/CLI/IDE，无证据的旧配置只读。
- [ ] 运行同一组测试，补充中文长路径、空目录、丢失根、二进制、父目录边界、Windows 实际 reparse point 与 Unix symlink。跨平台缺少权限时明确跳过，不虚报通过。
- [ ] 独立评审发现能力与写入能力分离；候选提交 `feat(discovery): inspect four agent skill inventories safely`。

## Task 3：逐对选择、物理目标合并与差异计划

**Files:** Create `crates/skill-core/src/{plan.rs,diff.rs}`、`crates/skill-core/tests/planning.rs`；Modify core model.rs/lib.rs。

**Interfaces:**
- Consumes Task 1 核心类型和 Task 2 的已规范化目标快照。
- Produces `plan_distribution(packages: &[LibraryPackage], targets: &[TargetSnapshot], selection: &[SelectionPair], decisions: &BTreeMap<TargetKey, ConflictDecision>) -> Result<DistributionPlan, CoreError>`。
- Produces `redacted_plan_dto(plan: &DistributionPlan) -> SafePlanView`；SafePlanView 只用于展示，不可反序列化成可执行计划。

- [ ] 写失败用例 `two_pairs_do_not_expand_to_four`、`shared_target_has_one_change_and_all_possible_readers`、`two_versions_for_same_target_block`、`unmanaged_different_package_defaults_to_skip`、`identical_target_has_no_write`。
- [ ] Run `cargo test --locked -p skill-core --test planning`，验证红灯。
- [ ] 实现逐对编译、按物理键合并、默认跳过/明确替换、额外旧文件归档清单；不随意混合目录树或合并 SKILL.md。
- [ ] 实现逐文件脱敏 diff：文本展示结构变化，二进制仅展示大小/变化类型；公开 fixtures 不使用真实凭据。
- [ ] 再跑 planning，断言原目标多出的用户文件出现在替换影响清单，已跳过项从 MutationPlan 消失，不变项不产生备份请求。
- [ ] 独立评审共享影响与冲突语义；候选提交 `feat(planning): preserve exact skill-agent pairs and shared targets`。

## Task 4：备份、目录事务与中断恢复引擎

**Files:** Create `crates/skill-io/src/{backup.rs,transaction.rs,journal.rs,lock.rs,audit.rs}`、`crates/skill-io/tests/{transactions.rs,recovery.rs}`、`docs/SAFETY.md`；Modify io lib.rs/paths.rs。

**Interfaces:**
- Consumes Task 3 `DistributionPlan` 转换得到的 `MutationPlan` 和 Task 2 快照。
- Produces `IoEngine::prepare(plan: &MutationPlan) -> Result<PreparedMutation, IoError>`。
- Produces `IoEngine::apply(prepared: PreparedMutation) -> MutationReceipt`；prepared 为引擎内部类型，不能从 IPC 构造。
- Produces `IoEngine::list_recoveries() -> Result<Vec<RecoveryRecord>, IoError>`、`IoEngine::plan_restore(id: &str) -> Result<MutationPlan, IoError>`。
- `PreparedMutation/MutationReceipt/RecoveryRecord` 在 io lib.rs 定义；支持设计的七类 outcome，包含恢复路径且不含原始内容。

- [ ] 写失败用例 `backup_failure_means_zero_target_mutations`、`no_staging_inside_any_skill_root`、`extra_old_files_leave_load_path_but_are_archived`、`restore_original_absence_without_delete`。
- [ ] Run `cargo test --locked -p skill-io --test transactions`，验证红灯；使用引擎注入的故障点，禁止对真实目录制造故障。
- [ ] 实现所有 original/planned 完整树和 journal 持久化后才写；同卷 staging 在扫描根之外；逐组二次核对 → 归档旧包 → 激活新包 → 完整校验。锁 acquire 用 create_new，释放用 rename，不删除锁文件。
- [ ] 写并跑 `crash_after_archive_is_recovery_required`、`crash_after_activation_does_not_overwrite_later_edits`、`one_group_failure_does_not_hide_other_success`、`audit_failure_keeps_real_receipt`；Run `cargo test --locked -p skill-io --test recovery`。
- [ ] 增加 `external_edit_after_activation_prevents_auto_rollback` 和 `cannot_read_target_keeps_current_directory`：激活后出现非 planned 字节或无法证明归属时，断言当前目标保持原样、结果 recovery_required、original/planned/当前现场全部保留。仅在字节和路径身份仍属于本事务时才自动恢复。
- [ ] 验证全阶段 journal 检查、原本不存在与空目录差异、中央备份跨卷与激活同卷、权限不足、Unix 执行位和 Windows rename 失败；残留文件保留供查，不自动清理。
- [ ] 独立评审先备份后覆盖、可恢复的中断状态及“不保证全局原子”的边界；候选提交 `feat(io): back up and activate skill directories with recovery journals`。

## Task 5：库、设置与导入/编辑用例

**Files:** Create `crates/skill-app/src/{library.rs,settings.rs,preview.rs,importing.rs}`、`crates/skill-app/tests/{library.rs,importing.rs}`；Modify app lib.rs；Create `tests/fixtures/contracts/import.json`。

**Interfaces:**
- Consumes Task 1/2 包与扫描、Task 4 IO 引擎。
- Produces `App::load_library(path: &Path) -> Result<LibraryView, AppError>`、`App::read_package(id: &PackageId) -> Result<SafePackageView, AppError>`。
- Produces `App::scan_import(req: ImportRequest) -> Result<ImportScan, AppError>`、`App::preview_import(scan_id: &str, choices: &[ImportChoice]) -> Result<PreviewDto, AppError>`、`App::commit_import(preview_id: &str) -> Result<ReceiptDto, AppError>`。
- Produces `App::save_skill_document(req: EditDocumentRequest) -> Result<ReceiptDto, AppError>`；请求含 packageId、编辑基线 ID 与正文，最终路径只能由后端库记录决定。
- Produces `App::load_settings() -> Result<SettingsView, AppError>` 与 `App::save_settings(req: SettingsRequest) -> Result<SettingsView, AppError>`；SettingsView 固定 settings/revision/warnings，settings 破损禁止隐式回默认并覆盖。
- preview.rs 提供 `PreviewStore`：窗口单当前会话、可注入 Clock、600 秒、每次确认即消费、64 MiB 总限制；导入和分发/恢复共用生命周期。

- [ ] 写 `missing_library_is_first_use_not_created`、`broken_library_is_not_empty`、`import_preserves_source_and_full_tree`、`import_or_edit_never_touches_agent_roots`、`same_name_different_content_requires_choice`。
- [ ] Run `cargo test --locked -p skill-app --test library` 与 `cargo test --locked -p skill-app --test importing`；确认红灯。
- [ ] 实现 version:1 catalog 与追加式 revisions：新版本完整复制并持久化 → 备份旧索引 → 原子切换 catalog 引用。工具不原地覆盖已引用版本；未引用 revision 中断后不进入列表，不自动删除。外部编辑冲突必须阻止覆盖。
- [ ] 实现导入扫描/预览/确认，源和库均有快照；编辑保持原文格式与未知字段，校验不通过保留输入，不带入源 Agent 的插件安装或开关。
- [ ] 跑同样测试并补充目录重叠、导入超限、敏感字段/错误脱敏、损坏设置、备份目录变更、未完成库任务恢复。新备份目录只影响未来任务，不迁移历史数据。
- [ ] 增加 `catalog_metadata_mismatch_blocks_distribution`、`crash_before_catalog_commit_keeps_old_revision`、`rename_keeps_package_id_and_tracks_previous_paths`；断言 name/description 来自 SKILL.md 且与索引一致后才可用，保存更名不移动任何 Agent 目录。
- [ ] 独立评审库一致性和“只改库”；候选提交 `feat(library): import and edit complete skill packages without distribution`。

## Task 6：分发、历史与恢复的一次性确认服务

**Files:** Create `crates/skill-app/src/{distribution.rs,restore.rs,history.rs}`、`crates/skill-app/tests/{distribution.rs,restore.rs}`、`tests/fixtures/contracts/{distribution.json,receipt.json}`；Modify preview.rs/app lib.rs。

**Interfaces:**
- Consumes Task 3 planner、Task 4 engine、Task 5 library/settings/PreviewStore。
- Produces `App::preview_distribution(req: DistributionRequest) -> Result<PreviewDto, AppError>`、`App::commit_distribution(preview_id: &str) -> Result<ReceiptDto, AppError>`、`App::cancel_preview(preview_id: &str)`。
- Produces `App::list_history() -> Result<Vec<HistoryItem>, AppError>`、`App::preview_restore(operation_id: &str) -> Result<PreviewDto, AppError>`、`App::commit_restore(preview_id: &str) -> Result<ReceiptDto, AppError>`、`App::list_recoveries() -> Result<Vec<RecoveryDto>, AppError>`；RecoveryDto 固定 operationId/state/paths/reason，无原文。
- DistributionRequest 固定 libraryPath/scope/projectRoot/selection/conflictDecisions；commit 不接受它的副本或任意目标路径。

- [ ] 写 `expired_preview_writes_nothing`、`failed_confirm_consumes_token`、`added_target_file_invalidates_whole_preview`、`binary_change_invalidates_preview`、`link_redirect_invalidates_preview`、`selection_or_settings_change_invalidates_preview`。
- [ ] Run `cargo test --locked -p skill-app --test distribution`；确认红灯不是环境问题。
- [ ] 实现后端完整快照和上下文绑定；600 秒用假 Clock 测，无真实 sleep。确认前重新扫描所有目标，整批预检通过后才进入 engine。
- [ ] 跑 distribution，断言同目录只有一组 mutation、一份备份；预检后逐组竞态分别记录 blocked/实际成功，不把部分成功显示为整批成功。
- [ ] 写并跑 `restore_previews_current_vs_backup`、`restore_backs_up_later_edits`、`restore_missing_target_archives_instead_of_deletes`、`restart_requires_explicit_recovery_confirmation`；Run `cargo test --locked -p skill-app --test restore`。
- [ ] 增加 `rename_new_target_does_not_hide_old_location` 与 `restore_uses_name_at_write_path`：foo 改成 bar 后应用 bar，foo 继续存在并留在历史位置提示；恢复旧操作仍作用于原 foo 路径，不重新拼接当前 bar。
- [ ] 独立评审令牌消费、路径/树复核、恢复前备份和审计失败；候选提交 `feat(app): enforce one-time preview for distribution and restore`。

## Task 7：可用的 CLI 闭环

**Files:** Create `crates/skill-cli/{Cargo.toml,src/main.rs,tests/commands.rs}`；Modify workspace Cargo.toml/README.md/CI CLI jobs。

**Interfaces:**
- Consumes Task 2/5/6 App 用例；CLI 不直接绕过应用层写盘。
- Produces `skill-hub agents|doctor|list|import|diff|apply|backups|restore`；支持 `--home` 隔离 HOME 与 APPDATA、显式 `--project`、`--scope user|project`、精确配对选择。
- `apply` 无 --yes 退出 2，只展示计划；有 --yes 现场生成并消费预览。--dry-run 优先于写入许可，不创建备份或审计。import/restore 的写入也要求 --yes。

- [ ] 先写子进程用例 `apply_without_yes_is_read_only_exit_2`、`dry_run_has_no_backups_or_audit`、`home_isolation_prevents_real_profile_reads`、`restore_requires_yes`。
- [ ] Run `cargo test --locked -p skill-cli --test commands`，确认失败后接线最小命令，不复制 planner/engine。
- [ ] 实现错误码：0=完成且无失败，2=需要确认/只展示计划，1=错误或部分失败；dry-run 成功用 0。明确区分未检测到与读取失败。
- [ ] 再跑命令测试及隔离目录的 import → diff → apply --yes → restore --yes 流程，核对原文件和回执；全程只用 fixture。
- [ ] 独立评审无写入默认和 CLI 选项优先级；候选提交 `feat(cli): expose isolated manual skill distribution workflow`。

## Task 8：已确认视觉的 UI 外壳、库列表与契约

**Files:** Create `apps/web/{package.json,package-lock.json,index.html,tsconfig.json,vite.config.ts}`、`src/{App.tsx,main.tsx,styles.css,api.ts,types.ts,mock.ts,selection.ts}`、`src/components/{AppShell.tsx,Icon.tsx,Status.tsx,Dialog.tsx}`、`src/pages/Library.tsx`、`tests/{selection.test.mjs,contract.test.mjs}`；Modify 前端 CI。

**Interfaces:**
- Consumes Task 5/6 contract fixtures，mock 返回同结构 DTO，不能生成假“真实配置”状态。
- Produces typed `api` 方法，名称与设计关键命令一一对应；desktop invoke 与 mockCall 分支统一。
- Produces `selectionFromCells(cells: Iterable<string>): SelectionPair[]`，校验 key 并保留逐对关系。
- Root 状态持有 library/scope/project/selection/contextGeneration；所有预览只存后端 ID 与安全 DTO，不存可执行 raw plan。

- [ ] 先写 selection 精确配对用例和 JSON fixture/TS 字段契约检查；Run `npm --prefix apps/web test`，在批准锁定依赖已就位的 CI 中确认红灯。
- [ ] 按已认可原型实现主题 tokens、166px 侧栏、列表/260px 详情和底部选择栏；导航为我的 Skills/Agents/变更历史/软件设置。保留中文排版、色彩语义和真实目标状态。
- [ ] 实现搜索、全部/待更新/未分发筛选与选择；空库、损坏库、无匹配、读取失败分开呈现。切 Skill 保留本次其他配对，切库/范围/项目清空配对并使预览失效。
- [ ] Run `npm --prefix apps/web test` 与 `npm --prefix apps/web run build`；TS 无错误、fixture 契约通过、无真实 IO。
- [ ] 以 1024/850/650/390/320px 截图核对：不缩放整个桌面、不依赖在线字体、不出现页面水平溢出。样式用视觉核对，不写镜像 CSS 测试。
- [ ] 独立设计/可访问性评审；候选提交 `feat(ui): build approved skill library workbench`。

## Task 9：Agent 扫描、导入、编辑和设置交互

**Files:** Create `apps/web/src/pages/{Agents.tsx,Import.tsx,SkillEditor.tsx,Settings.tsx}`、`src/components/LocationForm.tsx`；Modify App.tsx/api.ts/mock.ts/types.ts；Create gui.test.mjs 的对应场景。

**Interfaces:**
- Consumes Task 5 导入/编辑/设置用例、Task 8 dialog/api/contextGeneration。
- Import 成功只刷新库；SkillEditor 保存只调用 save_skill_document；两者绝不调用 commit_distribution。
- 项目目录选择支持取消返回 null，空值不得保存成当前工作目录；旧请求以 generation 丢弃。

- [ ] 先写浏览器场景 `import_never_applies`、`editor_keeps_dirty_input_on_error`、`cancel_project_picker_keeps_context`、`slow_old_agent_response_is_ignored`。
- [ ] Run `npm --prefix apps/web run test:gui`，用内存 IPC fixture 确认红灯；没有批准的浏览器依赖时停止而非安装。
- [ ] 实现 Agent 列表/来源路径/只读详情、Agent 与本地目录导入、同名选择、SKILL.md 编辑和字段校验；来源链接与实际路径都可见。name 改变后单列历史名称位置并提示可能同时加载，不提供自动卸载。
- [ ] 实现库/备份目录设置保存的明确生效范围，不提供自动同步开关；损坏设置在用户明确修复前不写库或 Agent。
- [ ] 再跑场景，补充 Escape、焦点回到触发按钮、未保存确认、权限失败、超限包、长路径和旧请求竞争。
- [ ] 独立评审表单与状态保真；候选提交 `feat(ui): add safe agent import and library editing tasks`。

## Task 10：预览确认、逐组回执、历史与恢复 UI

**Files:** Create `apps/web/src/pages/{Preview.tsx,History.tsx,Restore.tsx}`；Modify App.tsx/api.ts/mock.ts/types.ts、tests/gui.test.mjs。

**Interfaces:**
- Consumes Task 6 PreviewDto/ReceiptDto/HistoryItem；confirm 仅传当前 previewId。
- UI 状态固定 idle/planning/ready/confirming/expired/result。任何上下文变化清 ready；confirming 期间锁定上下文且不允许重入。

- [ ] 先写 `shared_target_lists_extra_readers`、`stale_preview_has_no_confirm_button`、`double_confirm_is_not_replayed`、`partial_failure_retains_successful_groups`、`restore_has_independent_preview`。
- [ ] Run `npm --prefix apps/web run test:gui`，确认上述场景红灯。
- [ ] 实现按物理目录的新增/更新/不变/阻止展示、文本 diff/二进制摘要、额外文件归档影响、冲突策略；修改策略重新预览，不直接改前端计划。
- [ ] 实现逐组结果和 history/recovery 入口，保留失败/备份/归档资料；审计失败不显示为“全部失败已回滚”，成功文案不声称 Agent 已加载。
- [ ] 跑完整 gui 场景，另测库编辑后旧预览不可用、任务取消、恢复失败、切页与重启后待恢复任务提示。
- [ ] 独立评审确认含义与失败展示；候选提交 `feat(ui): preview and confirm distribution with recovery history`。

## Task 11：Tauri 薄桥接和原生路径选择

**Files:** Create `apps/web/src-tauri/{Cargo.toml,build.rs,tauri.conf.json,capabilities/default.json}`、`src/{lib.rs,commands.rs,dto.rs,directory.rs}`；Modify api.ts/CI 桌面 jobs。

**Interfaces:**
- Consumes Task 2/5/6 App；注册设计列出的命令。每个窗口持有独立 PreviewStore，App/Engine 统一注册根与数据目录。
- `choose_directory(initial: Option<String>) -> Result<Option<String>, SafeError>`，原生取消=none，不落盘。
- `commit_distribution(preview_id: String)`、`commit_import(preview_id: String)`、`commit_restore(preview_id: String)` 不接受前端任意文件路径/内容计划。
- `read_package` 等只读命令同样校验 packageId 和注册根；无任意文件读写、shell 执行或泛化 fs IPC。

- [ ] 写原生命令用例 `forged_target_path_is_rejected`、`unknown_package_id_cannot_read_arbitrary_file`、`window_preview_is_not_shared`、`cancel_native_picker_returns_none`。
- [ ] 在 Windows CI Run `cargo test --locked -p skill-desktop` 与 `cargo check --locked -p skill-desktop`；确认红灯来自缺失契约。
- [ ] 接线薄命令和 DTO、选择目录能力；权限能力最小化，不为打开目录开放任意命令执行。前端 mock 与 desktop 契约保持一致。
- [ ] 重跑原生命令测试，静态检查 raw snapshots 未实现 Debug/Serialize、日志无原文、Capabilities 没有任意写盘/远程 shell。
- [ ] 独立评审 IPC 边界；候选提交 `feat(desktop): connect typed native commands to skill services`。

## Task 12：完整验证、安装包与操作手册

**Files:** Create/Modify `.github/workflows/{ci.yml,ui-review.yml,release.yml}`、`docs/{SAFETY.md,MANUAL-VALIDATION.md}`、README.md、受控合成 fixtures；按批准版本维护 Cargo/package 锁文件。

**Interfaces:**
- Consumes 前面所有稳定接口；不在此任务增加新功能或升级依赖。
- Produces 版本化测试证据、截图、安装包与明确的人工验收清单；发布只在另有授权时执行。

- [ ] 先建立真正的端到端场景：同一个临时 HOME 中导入 Claude 包 → 保存库 → 逐对选 Codex/Grok → 预览 → 应用 → 写后比较 → 再编辑库但不应用 → 预览恢复。验证未选独立目标不变，共享可见性按实际根单独断言。
- [ ] Run `cargo fmt --all --check`、`cargo clippy --workspace --exclude skill-desktop --all-targets --locked -- -D warnings`、`cargo test --workspace --exclude skill-desktop --locked`、前端 test/test:gui/build；Windows/macOS 另外运行 `cargo test --locked -p skill-desktop` 与桌面构建，不要求 Linux 安装桌面依赖。
- [ ] 验证 Windows/macOS 的重命名与故障恢复、Linux CLI、Windows 保留名/中文/长路径、Unix 执行位、二进制和 CRLF；每项注明真实平台执行、模拟或跳过。
- [ ] 用真实浏览器核对四页、全部任务弹窗、暗色、320–1024px、键盘和200%缩放；截图与交互日志作为制品，不把原型静态检查当成 GUI 通过。
- [ ] 构建 Windows/macOS 桌面包与三平台 CLI，生成产物校验摘要；不自动发布。用户先在隔离 profile 人工验收，再明确批准真实 Agent 手动应用；记录版本与实际发现/加载结果。
- [ ] 全分支独立评审 A1–A16。若获提交授权，精确暂存本任务文件，检查 `git diff --cached --check`、已暂存内容和敏感关键词后再提交；不输出任何匹配到的真实凭据。
- [ ] 候选提交 `docs(release): document verified manual skill workflows and platform limits`；标签/Release/部署各自单独授权。

## 3. 阶段交付、验收映射与回退

| 阶段 | 任务 | 可独立检查的成果 | 验收 |
|---|---|---|---|
| 基础与只读能力 | 1–3 | 完整包校验、四个 Agent 只读清单、无副作用分发计划 | A1/A2/A4/A5/A10 |
| 文件与用例闭环 | 4–7 | 隔离目录中可备份、应用、恢复的 CLI 与一致性保护 | A3/A6/A7/A8/A9/A12/A13/A15/A16 |
| 产品界面 | 8–11 | 已确认视觉的桌面四页和任务弹窗，真实 IPC 接线 | A3/A4/A5/A6/A11/A12/A16 |
| 可分发与人工验证 | 12 | CI 证据、安装包、说明与实际 Agent 核对记录 | A1–A16 |

阶段评审不通过时只修该阶段的问题，再进入依赖它的阶段。先完成上游接口与模型，不让多个实现同时发明 DTO。允许在接口冻结后分开进行无共享写入的 UI 与 CLI 工作，但必须由用户选定执行方式。

回退策略：代码使用新开发分支保留现状，不用 reset --hard/clean/rebase。文件内容通过已持久化备份与明确恢复预览回退，不删除数据；新版本与旧版本的 journal/catalog schema 不兼容时阻止自动回退。安装包回退不能代替数据恢复。

## 4. 本计划自检

- [x] A1–A16 均有明确的实施任务和预期证据要求；未把安装包产出当作真实 Agent 加载验证。
- [x] 单个任务的类型、方法名、DTO 字段与第 2 节一致；源快照和可执行计划从不由前端传入。
- [x] 五项 Review Focus 都有具名失败用例与所属任务；用例尚未实现或执行。
- [x] 每个任务有验证命令、红绿条件和评审范围；当前没有声称这些命令已通过。
- [x] 已确认视觉与手动应用要求贯穿库、导入、设置、CLI 和恢复。
- [x] 本轮只写设计和计划；当前 master 仓库无修改；没有安装、删除、提交或推送。

执行方式在用户审阅后选择：逐任务实现并独立评审，或当前主代理连续实现、阶段和分支结束时独立评审。建议本项目采用逐任务实现与独立评审，因为目录替换、共享可见性和恢复协议的错误会影响真实 Agent 文件。
