# Skill Hub 设计审阅记录

日期：2026-11-01。

审阅方式：独立子代理只读 builder check；主代理进行实施计划的覆盖、类型和比例自检。没有运行产品测试或构建，产品尚未实现。

首轮审阅发现三处重要未决事项，均已写入最终设计与任务：

| 问题 | 已采纳的规则 | 实施验证 |
|---|---|---|
| 写后失败自动恢复可能撤下外部编辑 | 只有内容与实际路径仍可证明属于本事务时自动恢复；否则保留现场，进入 recovery_required | Task 4：external_edit_after_activation_prevents_auto_rollback、cannot_read_target_keeps_current_directory |
| catalog 与包内容缺少共同提交边界 | 追加完整 revision 后原子切换 catalog；索引与 SKILL.md 不一致则阻止分发 | Task 5：catalog_metadata_mismatch_blocks_distribution、crash_before_catalog_commit_keeps_old_revision |
| name 改变后丢失旧分发位置 | 保存 name_at_write 与原物理路径，旧目录继续提示、不自动卸载；恢复不按新名称定位 | Task 5/6：rename_keeps_package_id_and_tracks_previous_paths、rename_new_target_does_not_hide_old_location、restore_uses_name_at_write_path |

独立审阅者重新读取修订后的完整设计，结论：`nothing important left`。这表示没有发现仍需补问的重要设计问题，不表示安全审计、实现或测试已经通过。

主代理自检：16 项验收条件均映射到任务；五项主要失败类别均有具名用例；共享目录、恢复、库版本和原型视觉保持一致；所有实施任务仍未勾选。

保留首轮文档于 `initial/design.md` 与 `initial/plan.md`，不覆盖历史草稿。产品设计和计划以 `../docs/superpowers/` 下的文件为准。

本轮边界：skillcfg 仍为 master；文档只写入聊天专属目录。未修改两份仓库、未安装依赖、未删除文件、未 commit/push。
