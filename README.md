# Skill Hub

本地 Skill 管理与跨 Agent 手动分发工具。内置 Codex、Claude Code、Antigravity 和 Grok Build，普通新增 Agent 通过 JSON 档案接入。

## 当前状态

第一阶段核心和只读发现已验收。第二阶段实现了完整包备份、库管理、手动预览/应用/恢复及 CLI，正在进行最终 GitHub 验证和独立审阅。桌面界面属于第三阶段。

- [完整设计](docs/superpowers/specs/2026-11-01-skill-hub-design.md)
- [实施计划](docs/superpowers/plans/2026-11-01-skill-hub.md)
- [阶段进度](docs/PROGRESS.md)
- [CLI 使用](docs/CLI.md)
- [写入与恢复边界](docs/SAFETY.md)
- [依赖清单](docs/DEPENDENCIES.md)
- [设计审阅](docs/reviews/design-review.md)
- [已确认的界面原型](docs/prototype/skill-hub-design.html)

每次预览后手动应用，不自动同步。Linux、macOS、Windows 均规划桌面端与 CLI；全部编译和测试只在 GitHub Actions 执行，每阶段提交到开发分支。

## 第一阶段

1. 完整包模型、原文保留与可移植性校验。
2. 声明式 Agent 档案、安全快照与只读发现。
3. 精确 Skill/Agent 配对、共享目标和差异计划。

第一阶段不写入 Agent。第二阶段的写入要求每次手动确认；不自动执行包内脚本。第三阶段继续已确认的桌面界面设计。
