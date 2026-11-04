# Skill Hub

本地 Skill 管理与跨 Agent 手动分发工具。内置 Codex、Claude Code、Antigravity 和 Grok Build，普通新增 Agent 通过 JSON 档案接入。

首版为 v0.1.0 CLI，提供 Linux、Windows、macOS 两个架构的 ZIP 和 SHA-256；见 [发布说明](docs/releases/v0.1.0.md)。发布包由 GitHub Actions 在完整验证通过后生成。

## 当前状态

第一阶段核心和只读发现已验收。第二阶段的完整包备份、库管理、手动预览/应用/恢复及 CLI 已通过独立审阅修复与四平台 GitHub 验收。第三阶段正在完成桌面工作台与原生桥接验收，安装包与实际 Agent 加载属于第四阶段。

- [完整设计](docs/superpowers/specs/2026-11-01-skill-hub-design.md)
- [实施计划](docs/superpowers/plans/2026-11-01-skill-hub.md)
- [阶段进度](docs/PROGRESS.md)
- [CLI 使用](docs/CLI.md)
- [桌面工作台说明](docs/DESKTOP.md)
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

## 第二阶段验收

代码验收提交 9df300c，[GitHub run 41637391199](https://github.com/rshun/skillcfg/actions/runs/41637391199) 的 6 个必需作业全部通过。Linux 109、macOS 两个架构各 108、Windows 103 项测试；平台专属用例按实际系统运行。CLI 制品可从工作流 Artifacts 获取，保留 14 天。归档提交仍由同一工作流检查；[开发分支最新 CI](https://github.com/rshun/skillcfg/actions/workflows/ci.yml?query=branch%3Adev%2Fskill-hub) 应与当前提交匹配。

[第二阶段审阅与修复记录](docs/reviews/stage-two-review.md) 包含六类回归。桌面界面、安装包和真实 Agent 加载验收仍属于后续阶段；当前 CLI 不代表 Agent 已加载 Skill。
