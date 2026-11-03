# Skill Hub 设计审阅记录

日期：2026-11-01。

审阅方式：独立子代理只读 builder check；主代理进行实施计划的覆盖、类型和比例自检。没有运行产品测试或构建，产品尚未实现。以下先保留首版记录，最新可配置档案修订的审阅见后半部分。

首轮审阅发现三处重要未决事项，均已写入最终设计与任务：

| 问题 | 已采纳的规则 | 实施验证 |
|---|---|---|
| 写后失败自动恢复可能撤下外部编辑 | 只有内容与实际路径仍可证明属于本事务时自动恢复；否则保留现场，进入 recovery_required | Task 4：external_edit_after_activation_prevents_auto_rollback、cannot_read_target_keeps_current_directory |
| catalog 与包内容缺少共同提交边界 | 追加完整 revision 后原子切换 catalog；索引与 SKILL.md 不一致则阻止分发 | Task 5：catalog_metadata_mismatch_blocks_distribution、crash_before_catalog_commit_keeps_old_revision |
| name 改变后丢失旧分发位置 | 保存 name_at_write 与原物理路径，旧目录继续提示、不自动卸载；恢复不按新名称定位 | Task 5/6：rename_keeps_package_id_and_tracks_previous_paths、rename_new_target_does_not_hide_old_location、restore_uses_name_at_write_path |

独立审阅者重新读取修订后的完整设计，结论：`nothing important left`。这表示没有发现仍需补问的重要设计问题，不表示安全审计、实现或测试已经通过。

主代理自检：16 项验收条件均映射到任务；五项主要失败类别均有具名用例；共享目录、恢复、库版本和原型视觉保持一致；所有实施任务仍未勾选。

保留首轮文档于 `initial/design.md` 与 `initial/plan.md`，不覆盖历史草稿。产品设计和计划以 `../superpowers/` 下的文件为准。

本轮边界：skillcfg 仍为 master；文档只写入聊天专属目录。未修改两份仓库、未安装依赖、未删除文件、未 commit/push。

## 可配置 Agent 档案修订的审阅

新增用户要求：内置四个 Agent + 运行时 JSON 档案扩展；普通新增不改 Rust/TS、不重编译；Linux/macOS/Windows 均支持；每阶段提交到仓库；所有编译在 GitHub Actions。

本次按三平台均提供桌面端和 CLI 设计，架构目标为 Windows/Linux x64、macOS arm64/x64。阶段内必要验证提交与阶段完成提交推送到开发分支，最终提交对应 GitHub CI 绿灯后才可标为完成；不自动合并主干或发布。

新独立审阅者发现 variant 与根/探针的选择关系未冻结。已明确：

- appliesToVariants 省略代表全部，指定时必须非空且引用有效；每平台/范围/当前 variant 只有一个静态写根。
- readEvidence 的 false/unknown 只改变读取状态提示，不创建授权、不改选写根，保留潜在读取者。
- 启用时展示并授权当前平台全部 variants 的根与探针；切换已授权 variant 清选择和预览，不重复询问相同授权；新根/新规则必须重新确认。
- 共享影响合并所有已知 variants，保留 agentId/variantId/rootId/evidence；不硬编码四个 Agent。

复核修订设计后，独立审阅结论：`nothing important left`。主代理另补 settings.agentVariants 和 CLI select-variant 的保存接口，避免 UI 的变体选择缺少后端入口。

实施计划仍为 4 阶段、12 个未执行任务，新增 A17–A20，现有 20 项验收均有任务和预期证据。当前没有产品测试、构建、平台兼容或阶段 CI 已通过的声明。

变更前的设计/计划/审阅记录保留于 `before-configurable-agents/`。本次只更新提案目录，master 仓库未改；后续开发阶段的提交授权已记录，不再沿用“提交须另行泛化授权”的旧约定。

## 归档到开发分支

2026-11-01，用户授权在 dev/skill-hub 归档并继续第一阶段。当前文档位于仓库 docs，以上 master/提案目录相关陈述属于此前审阅时的历史边界；正式执行状态以 ../PROGRESS.md 为准。
