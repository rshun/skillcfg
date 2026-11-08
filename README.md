# Skill Hub

本地 Skill 管理与跨 Agent 手动分发工具。内置 Codex、Claude Code、Antigravity 和 Grok Build，普通新增 Agent 通过 JSON 档案接入。

当前版本为 **v0.3.0**（历史版本见 [v0.2.0](docs/releases/v0.2.0.md)、[v0.1.0](docs/releases/v0.1.0.md)），提供 Windows x64（NSIS `-setup.exe`、`.msi`、`-portable.zip` 与 CLI `.zip`）、macOS Apple Silicon arm64 与 Intel x64（各含 `.dmg`、`.app.tar.gz` 与 CLI `.tar.gz`）以及 Linux x64（`.AppImage`、`.deb` 与 CLI `.tar.gz`，基线 `glibc >= 2.39`），均附 SHA-256 摘要与运行环境基线；见 [v0.3.0 发布说明](docs/releases/v0.3.0.md)、[平台验证矩阵](docs/PLATFORMS.md) 与 [人工验收手册](docs/MANUAL-VALIDATION.md)。所有制品由 GitHub Actions 在完整验证通过后生成。

## 当前状态

第一至第四阶段（Tasks 1–12）的代码实现、全分支 `A1–A20` 独立审阅修复、四平台原生测试/桌面启动 smoke/零重编译第五个 Agent 验证、真实 Chrome 界面回归（含 200% 缩放）及三平台四架构安装包构建已在 `dev/skill-hub` 完成自动化验收；宿主机系统对话框点击与四个真实外部 Agent 的实际发现/加载核对（`A14`）按 [人工验收手册](docs/MANUAL-VALIDATION.md) 在目标宿主机执行。

- [完整设计](docs/superpowers/specs/2026-11-01-skill-hub-design.md)
- [实施计划](docs/superpowers/plans/2026-11-01-skill-hub.md)
- [阶段进度](docs/PROGRESS.md)
- [CLI 使用](docs/CLI.md)
- [桌面工作台说明](docs/DESKTOP.md)
- [平台验证矩阵](docs/PLATFORMS.md)
- [安装包与人工验收手册](docs/MANUAL-VALIDATION.md)
- [写入与恢复边界](docs/SAFETY.md)
- [依赖清单](docs/DEPENDENCIES.md)
- [设计审阅](docs/reviews/design-review.md)
- [第四阶段全分支审阅](docs/reviews/stage-four-review.md)
- [已确认的界面原型](docs/prototype/skill-hub-design.html)

每次预览后手动应用，不自动同步。Linux、macOS、Windows 均规划桌面端与 CLI；全部编译和测试只在 GitHub Actions 执行，每阶段提交到开发分支。

## 第一阶段

1. 完整包模型、原文保留与可移植性校验。
2. 声明式 Agent 档案、安全快照与只读发现。
3. 精确 Skill/Agent 配对、共享目标和差异计划。

第一阶段不写入 Agent。第二阶段的写入要求每次手动确认；不自动执行包内脚本。第三阶段继续已确认的桌面界面设计。

## 第二阶段验收

代码验收提交 9df300c，[GitHub run 41637391199](https://github.com/rshun/skillcfg/actions/runs/41637391199) 的 6 个必需作业全部通过。Linux 109、macOS 两个架构各 108、Windows 103 项测试；平台专属用例按实际系统运行。CLI 制品可从工作流 Artifacts 获取，保留 14 天。归档提交仍由同一工作流检查；[开发分支最新 CI](https://github.com/rshun/skillcfg/actions/workflows/ci.yml?query=branch%3Adev%2Fskill-hub) 应与当前提交匹配。

[第二阶段审阅与修复记录](docs/reviews/stage-two-review.md) 包含六类回归。

## 第三阶段验收

代码验收提交 58d6cd6，[Stage CI run 42443949755](https://github.com/rshun/skillcfg/actions/runs/42443949755) 的 7 个作业与 [Desktop and UI run 42443949757](https://github.com/rshun/skillcfg/actions/runs/42443949757) 的 8 个作业全部通过。核心矩阵 Linux 113、macOS 两个架构各 112、Windows 107 项测试；前端 16 项单元/契约测试与真实 Chrome 32 项交互场景通过；四个平台原生 Tauri 桥接测试、内嵌生产前端资源校验与桌面候选构建通过。归档提交继续由同一双流水线检查；[桌面工作流最新 CI](https://github.com/rshun/skillcfg/actions/workflows/desktop.yml?query=branch%3Adev%2Fskill-hub) 应与当前提交匹配。

[第三阶段审阅与修复记录](docs/reviews/stage-three-review.md) 包含八类回归。

## 第四阶段验收与安装包产物

代码验收提交 `a441284`，[Stage CI run 42450350844](https://github.com/rshun/skillcfg/actions/runs/42450350844)（7 项作业）、[Desktop and UI run 42450350883](https://github.com/rshun/skillcfg/actions/runs/42450350883)（8 项作业）与 [Release Bundles and Smoke run 42450350841](https://github.com/rshun/skillcfg/actions/runs/42450350841)（6 项作业）共 21 个作业全部通过：

- 四平台核心矩阵：Linux 114、macOS arm64 与 Intel 各 113、Windows 108 项测试；四平台原生桌面桥接与端到端各 9 项测试。
- 界面与浏览器回归：前端 16 项单元/契约测试，真实 Chrome 34 项测试（33 个交互场景，覆盖四页、全部五个任务弹窗、320–1024px、暗色主题、键盘 `Escape` 焦点回收与 200% 缩放截图及交互日志）。
- 三平台安装包与零重编译验证：`skill-hub-release-bundles` 制品包含全部 13 个桌面/CLI 安装包与免安装归档、`SHA256SUMS.txt`、`BUILD-BASELINE.md` 及四平台原生启动与 `custom.fifth.json` 零重编译运行报告（前后比对二进制 SHA-256 完全一致）。
- 边界与人工核对：GitHub CI 不代替宿主机原生目录对话框点击、安装向导图形点击或外部真实 Agent 发现/加载验证（`A14`）；操作前请按 [人工验收手册](docs/MANUAL-VALIDATION.md) 先在 `--home` 隔离目录核对，再逐项记录真实 Agent 加载结果。[发布包工作流最新 CI](https://github.com/rshun/skillcfg/actions/workflows/release.yml?query=branch%3Adev%2Fskill-hub) 应与当前分支最新提交匹配。
