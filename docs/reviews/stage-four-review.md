# 第四阶段全分支独立审阅与回归（A1–A20）

审阅范围：`6c4a93a..508c0bd` 全分支实现与第四阶段（Task 12）交付内容；由一名全新只读上下文 reviewer（`Stage 4 Whole-Branch Auditor`）对照设计规范第 13 节 `A1–A20`、计划书 Task 12 及全部红线规则执行独立审阅，共识别 1 项 `Critical`、3 项 `Important` 与 4 项 `Minor`。

候选提交 `508c0bd` 在 GitHub Actions 运行期间已同步暴露 `C1` 红灯（`Stage CI run 42446908336` 中 Windows 拒绝 `CON.txt` 而 Linux/macOS 未拒绝）以及 `end_to_end.rs` 独立目标 `possibleReaders` 断言和 `rustfmt` 格式差异（`Desktop and UI run 42446908309`）。全部发现项已在代码修复批次 `eb1cea5` / `a441284` 收敛并通过三条四平台流水线全绿验证。

| 编号 | 严重程度 | 关联验收项 | 发现及可见影响 | 回归与当前状态 |
|---|---|---|---|---|
| C1 | Critical | A4 / A19 / §5.3 | `crates/skill-core/src/package.rs` 中 `valid_relative_path` 与 `validate_package` 大小写折叠仅在 `Platform::Windows` 生效，导致 Linux/macOS 导入含 Windows 保留名（如 `CON.txt`、`NUL.txt`）、控制字符或大小写折叠冲突（如 `readme` 与 `README`）的包时未拦截，破坏跨平台可移植性 | `508c0bd` / `run 42446908336` 实际观察 Linux/macOS 红灯；移除平台短路并在 `packages.rs` 与 `commands.rs` 覆盖三平台断言，RED→GREEN 四平台全绿 |
| I1 | Important | A18 / A20 | `508c0bd` 新增测试文件未通过云端 `cargo fmt --all --check`（`Stage CI` `quality` 与 `Desktop and UI` `native-format` 失败）；后续 `eb1cea5` 在 Windows 上暴露 `end_to_end.rs` 仅 Unix 使用的 `Path` 导入触发 Clippy `-D warnings` | 下载并应用 GitHub 生成的格式补丁，修正 `end_to_end.rs` 条件导入；`a441284` 全部格式与 Clippy 作业通过 |
| I2 | Important | A16 / Task 12 | `scripts/smoke_desktop.py` 最初仅在 CLI 上验证第五个 Agent 的正向导入/分发/恢复，未翻转 `.fifth/config.json` 布尔探针，也未在已启用第五个 Agent 的隔离 `HOME` 上启动已打包桌面可执行文件；`MANUAL-VALIDATION.md` 需区分内存预览失效与打包后进程核验的证据归属 | `smoke_desktop.py` 新增探针 `true -> false`（`configEnabled -> configDisabled`）动态重算及已构建 `skill-desktop` 进程存活校验，前后比对二进制 SHA-256 完全不变；更新文档证据说明 |
| I3 | Important | A20 / Task 12 | 第四阶段审阅记录 `stage-four-review.md`、`PROGRESS.md`、`README.md` 与实施计划 Task 12 勾选状态需在代码验收全绿后同步归档，并再次核对最终提交 SHA 的三条必需流水线 | 本文件与阶段文档统一归档，并以阶段提交触发最终 SHA 全套 CI 核验 |
| M1 | Minor | Task 12 | `ci.yml` 保留第二阶段兼容的 `skill-hub-v0.1.0-<platform>.zip`，而 `release.yml` 产出 `skill-hub-cli-v0.1.0-*` 与 `skill-hub-desktop-v0.1.0-*`，文档需说明两套制品命名来源避免混淆 | 已在 `MANUAL-VALIDATION.md` 与 `PLATFORMS.md` 明确区分日常 CI 归档与发布候选归档 |
| M2 | Minor | A19 | `scripts/package_desktop.py` 中 `ldd --version` 空输出索引存在防御性缺口，且 `.app.tar.gz` 归档校验未显式断言 `Contents/MacOS/*` 可执行位 | 已增加空行兜底与 `.app.tar.gz` 内 `Contents/MacOS/*` 执行位（`0o111`）强制校验 |
| M3 | Minor | A11 | 第三阶段遗留 Minor：`apps/web/src/pages/Preview.tsx` 在零写入回执（无 `backupLocation` / `operationId`）时渲染空的“备份：/操作：”标签 | 已改为按字段非空条件渲染，与 `MutationReceipt.tsx` 一致，浏览器 `no-op receipt` 用例通过 |
| M4 | Minor | A6 / Task 12 | CLI 端到端测试可在恢复前补充“更新库记录但不执行 `apply` 时，已写入 Agent 目录字节保持不变”的断言，与桌面 `end_to_end.rs` 对齐 | 已在 `crates/skill-cli/tests/commands.rs` 补充 `--replace-id` 更新库后核对 `.agents/skills/code-review/SKILL.md` 原字节不变 |

## A1–A20 逐项核对结论

- **A1–A13、A15–A18（自动化闭环通过）**：内置四档案 + 运行时第五档案（`custom.fifth.json`）零重编译接入、只读启动零写入、精确 `package_id -> agent_id[]` 配对（不求笛卡尔积）、共享目录合并展示与布尔探针（JSON/TOML）重算、原文/未知 frontmatter/CRLF/二进制/空目录/Unix 执行位保留、同名冲突默认跳过、先备份再切换与中断/改名恢复、脱敏与真实凭据阻断、真实 Chrome 四页/五弹窗/暗色/320–1024px/键盘 `Escape` 焦点回收/200% 缩放截图与交互日志均已在 GitHub Actions 验证通过。
- **A14、A19、A20（诚实边界说明）**：
  - 四平台构建与原生测试（`ubuntu-24.04`、`windows-2022`、`macos-15` arm64、`macos-15-intel` x64，不以交叉编译冒充运行验证）、13 个桌面/CLI 安装包与归档及 SHA-256 摘要已由 GitHub Actions 产出。
  - Linux 桌面启动在无物理显示器的 GitHub runner 上通过 `xvfb-run` 虚拟显示验证原生进程存活与零启动写盘；Linux 安装包明确限定 `glibc >= 2.39`（Ubuntu 24.04 同代基线），不宣称兼容旧版 glibc 发行版；Windows 与 macOS 产物当前未签名/未公证。
  - **系统原生目录对话框人工点击、安装向导图形点击以及四个内置真实 Agent 的实际发现与加载（`A14`）属于宿主机人工验收范围，未在无外部 Agent 的 CI 环境伪造为“已通过”，已在 [MANUAL-VALIDATION.md](../MANUAL-VALIDATION.md) 提供完整核对步骤与待填记录表。**

## 代码验收证据（归档前代码提交）

代码验收提交：`a44128460d9e54b3ae246bb4c021f4a061579491`（21 个作业全部成功）：
- [Stage CI run 42450350844](https://github.com/rshun/skillcfg/actions/runs/42450350844)：7 个作业全部成功（Linux 114、macOS arm64 与 Intel 各 113、Windows 108 项测试，`rustfmt`、Clippy `-D warnings`、锁文件与 CLI 归档全部通过）。
- [Desktop and UI run 42450350883](https://github.com/rshun/skillcfg/actions/runs/42450350883)：8 个作业全部成功（前端 16 项单元/契约测试、真实 Chrome 34 项测试〔33 个交互场景，含 200% 缩放截图与 `gui-interaction-log.json`〕、原生格式检查、四平台各 9 项原生桥接/端到端/内嵌资源测试、Clippy 及桌面候选构建通过）。
- [Release Bundles and Smoke run 42450350841](https://github.com/rshun/skillcfg/actions/runs/42450350841)：6 个作业全部成功（四平台原生桌面启动 smoke、零重编译 `custom.fifth.json` CLI+桌面双端校验与二进制 SHA-256 前后一致证明，以及全部 13 个桌面安装包/免安装归档/CLI 归档、`SHA256SUMS.txt` 与 `BUILD-BASELINE.md` 汇总上传为 `skill-hub-release-bundles`）。

归档提交后将继续核对最终 HEAD SHA 在上述三条流水线的完整绿灯状态；不自动打 tag、不创建 GitHub Release、不合并 `master`。
