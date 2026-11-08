# 安装包、平台基线与人工验收手册

本文档提供第四阶段（Task 12）的产物说明、操作系统与运行库基线、自动化与人工验证边界矩阵，以及隔离 `--home` 验收和真实 Agent 发现/加载（`A14`）记录模板。

> 重要边界：GitHub Actions 全绿仅证明代码、CLI、原生桥接、无头/虚拟显示桌面进程启动、真实 Chrome 界面交互与安装包构建通过；**绝不等于系统原生目录对话框已完成人工点击，也不等于外部真实 Agent 已实际发现或加载 Skill**。标签、GitHub Release 与生产部署在未获单独授权前不自动执行。

---

## 1. 三平台构建产物与运行基线

所有二进制与安装包均由 `.github/workflows/ci.yml`、`.github/workflows/desktop.yml` 与 `.github/workflows/release.yml` 在 GitHub-hosted runners 上以 `--locked` 构建。`release.yml` 将桌面端与 CLI 发布候选统一汇总到 `skill-hub-release-bundles` 制品（内含 `SHA256SUMS.txt`、`BUILD-BASELINE.md` 及 `evidence/smoke-report-*.json`）；`ci.yml` 同时保留兼容命名的 `skill-hub-v0.2.0-<platform>.zip` CLI 归档供日常流水线快速验证。

| 平台与架构 | 桌面端产物 | CLI 产物 | 构建与原生运行 Runner | 最低 OS / 系统运行库基线与未签名边界 |
|---|---|---|---|---|
| Windows x64 | `skill-hub-desktop-v0.2.0-windows-x64-setup.exe`（NSIS）<br>`skill-hub-desktop-v0.2.0-windows-x64.msi`（MSI）<br>`skill-hub-desktop-v0.2.0-windows-x64-portable.zip` | `skill-hub-cli-v0.2.0-windows-x64.zip` | `windows-2022`（原生 x64 编译 + 原生进程启动） | Windows 10 1803+ 或 Windows 11 x64；需 Microsoft Edge WebView2 Runtime（Windows 11 与现代 Windows 10 已内置）。当前未配置 Authenticode 代码签名证书，首次运行可能出现 SmartScreen 提示。 |
| macOS Apple Silicon（arm64） | `skill-hub-desktop-v0.2.0-macos-arm64.dmg`<br>`skill-hub-desktop-v0.2.0-macos-arm64.app.tar.gz` | `skill-hub-cli-v0.2.0-macos-arm64.tar.gz` | `macos-15`（原生 arm64 编译 + 原生 arm64 启动） | macOS 11+（Apple Silicon），使用系统 Cocoa 与 `WKWebView`；构建基线为 macOS 15 SDK。当前未配置 Apple Developer 签名与公证（Notarization），Gatekeeper 默认拦截从浏览器下载的未公证应用，需在“系统设置 -> 隐私与安全性”手动允许。 |
| macOS Intel（x64） | `skill-hub-desktop-v0.2.0-macos-x64.dmg`<br>`skill-hub-desktop-v0.2.0-macos-x64.app.tar.gz` | `skill-hub-cli-v0.2.0-macos-x64.tar.gz` | `macos-15-intel`（原生 x64 编译 + 原生 x64 启动） | macOS 11+（Intel x86_64），使用系统 Cocoa 与 `WKWebView`；独立原生 x64 构建与运行验证，未以交叉编译替代运行验证。同样未签名/未公证。 |
| Linux x64 | `skill-hub-desktop-v0.2.0-linux-x64.AppImage`<br>`skill-hub-desktop-v0.2.0-linux-x64.deb` | `skill-hub-cli-v0.2.0-linux-x64.tar.gz` | `ubuntu-24.04`（原生 x64 编译 + `xvfb-run` 启动） | **仅声明兼容 glibc >= 2.39 的 x86_64 发行版**（如 Ubuntu 24.04 LTS 及同代 Debian 系），不宣称兼容旧 glibc 发行版（如 Ubuntu 22.04 glibc 2.35 / Debian 12 glibc 2.36）。桌面运行需 `libwebkit2gtk-4.1-0`、`libgtk-3-0`、`libayatana-appindicator3-1`、`librsvg2-2` 及图形会话；无 FUSE 环境运行 AppImage 可传 `--appimage-extract-and-run` 或设置 `APPIMAGE_EXTRACT_AND_RUN=1`。 |

不支持的架构（如 Windows arm64、Linux arm64、RPM/Flatpak）不在 v0.2.0 承诺范围内。

---

## 2. 验证覆盖分类矩阵（真实平台执行 vs 受控模拟 vs 人工核对）

任何验证项均按实际执行方式如实归类，不将模拟或无头检查冒充人工交互或真实 Agent 加载：

| 验证项 | 执行分类 | 覆盖平台 | 证据入口与边界说明 |
|---|---|---|---|
| 核心包校验、共享可见性、精确配对计划 | **真实平台自动执行** | Linux x64、Windows x64、macOS arm64、macOS x64 | `crates/skill-core` 单元测试与集成测试（`ci.yml` 四平台矩阵） |
| Claude 导入 → 保存库 → 逐对选 Codex/Grok → 预览 → 应用 → 写后比较 → 编辑库不应用 → 预览恢复 | **真实平台自动执行** | Linux x64、Windows x64、macOS arm64、macOS x64 | `apps/web/src-tauri/tests/end_to_end.rs` 与 `crates/skill-cli/tests/commands.rs`，在隔离临时 `HOME` 读写实际文件系统 |
| 重命名后历史恢复定位、真实子进程崩溃残留锁恢复、备份与归档不删除 | **真实平台自动执行** | Linux x64、Windows x64、macOS arm64、macOS x64 | `crates/skill-io`、`crates/skill-app`、`end_to_end.rs`（Windows 额外实测共享句柄占用回退；Linux 额外实测 `/tmp` 与 `/dev/shm` 跨设备中央备份） |
| Windows 保留名（`CON`/`NUL` 等）、UTF-8 中文、超 260 字符长路径、二进制字节、CRLF 保留 | **真实平台自动执行** | 四平台全跑（含 `windows-2022` 原生 NTFS 实测） | `crates/skill-core`、`crates/skill-io`、`crates/skill-cli/tests/commands.rs`、`end_to_end.rs` |
| Unix 执行位（`0755`）与私有权限（`0700`/`0600`） | **真实平台自动执行（仅 Unix）** | Linux x64、macOS arm64、macOS x64（Windows 按平台语义不声明 POSIX 执行位） | `crates/skill-io`、`crates/skill-cli/tests/commands.rs`、`end_to_end.rs` |
| 同一已编译二进制零重编译接入第五个 Agent（含非法档案、未授权、共享根、探针变更、桌面启动与恢复） | **真实平台自动执行** | Linux x64、Windows x64、macOS arm64、macOS x64 | `scripts/smoke_desktop.py` 在已构建产物上完成 CLI 全流程、探针布尔值翻转（`configEnabled -> configDisabled`）及已启用第五档案的原生桌面启动，前后比对 `skill-hub` 与 `skill-hub-desktop` 的 SHA-256 完全一致；内存态预览因探针变化自动失效由 `end_to_end.rs` 四平台实测 |
| 原生桌面可执行程序启动、动态库链接与只读零落盘启动（`--home`） | **真实平台进程执行（Linux 配合 `xvfb-run` 虚拟显示）** | Windows x64、macOS arm64、macOS x64 原生图形子系统；Linux x64 使用 `xvfb-run` | `scripts/smoke_desktop.py` 验证进程存活、非法相对路径退出码 `1`、启动不创建 `.skill-hub` |
| 四页导航、全部任务弹窗、暗色主题、320–1024px 宽度、键盘 `Escape` 焦点回收、200% 缩放 | **真实 Chrome 浏览器自动化（受控内存 IPC fixture）** | Ubuntu 24.04（Google Chrome CDP） | `apps/web/tests/gui.test.mjs` 产出 PNG 截图（含 `library-zoom-200.png`、`agents-zoom-200.png`、`preview-zoom-200.png`）与 `gui-interaction-log.json` |
| 原生目录选择器取消语义与非法路径边界 | **桥接契约测试 + 待宿主机人工点击** | 四平台自动跑 `DirectoryPicker` 取消/校验单测；真实 OS 文件对话框需宿主机人工点击 | `apps/web/src-tauri/tests/commands.rs` 验证取消不改上下文；真实系统窗口点击列于第 3 节人工清单 |
| 安装程序图形向导安装/升级与系统拦截放行 | **待宿主机人工核对** | Windows x64、macOS arm64/x64、Linux x64 | 构建完整性与头部魔数已在 `package_desktop.py` 自动校验；图形向导点击列于第 3 节人工清单 |
| 四个内置 Agent 的真实发现与加载（`A14`） | **待宿主机人工核对（严禁由 CI 代宣称通过）** | 用户目标宿主机 | 必须在完成第 3 节隔离验收后，按第 4 节模板逐个核对并记录真实 Agent 行为 |

---

## 3. 宿主机隔离环境（`--home`）人工验收步骤

在接触任何真实 Agent 配置前，先在全新隔离目录验证安装包与桌面/CLI 交互，确保零污染真实用户目录：

### 3.1 校验安装包摘要

```bash
# Linux / macOS：核对下载目录中的 SHA256SUMS.txt
shasum -a 256 -c SHA256SUMS.txt
```

```powershell
# Windows PowerShell：查看单个产物 SHA-256 并与 SHA256SUMS.txt 比对
Get-FileHash .\skill-hub-desktop-v0.2.0-windows-x64-portable.zip -Algorithm SHA256
```

### 3.2 创建空隔离目录并启动桌面端

```powershell
# Windows PowerShell（使用免安装版或已安装可执行文件，不触碰真实 USERPROFILE）
New-Item -ItemType Directory -Force C:\Temp\skill-hub-isolated-home
.\skill-hub-desktop.exe --home C:\Temp\skill-hub-isolated-home
```

```bash
# Linux / macOS（使用隔离绝对路径启动）
mkdir -p /tmp/skill-hub-isolated-home
./skill-hub-desktop --home /tmp/skill-hub-isolated-home
```

### 3.3 桌面端宿主机人工核对项

- [ ] **窗口渲染与缩放**：确认主窗口正常显示雾蓝工作台，切换明暗主题、拖窄窗口及调整系统显示缩放（如 150%/200%）无文字截断或按钮不可点。
- [ ] **原生目录选择器交互**：在“我的 Skills -> 导入本地包”或切换到“项目范围”时点击浏览按钮，先点系统原生对话框的“取消”（确认当前项目与页面状态不变），再重新选择测试目录（确认路径正确回填）。
- [ ] **手动预览与确认闭环**：导入一个测试 Skill 包后，核对 `.agents/skills` 等目录尚未产生文件；勾选目标 Agent、点击“预览分发变更”，核对物理路径、共享读取提示与文本/二进制差异后点击确认，验证写入回执。
- [ ] **编辑与恢复不删文件**：在库内编辑该 Skill 保存（确认已分发目录未自动变化，状态变为“内容不同”）；随后进入“变更历史”预览并确认恢复，确认加载目录中的包已归档移出且备份目录保留完整副本。
- [ ] **第五个 Agent 动态接入**：在“Agents -> 添加 Agent”粘贴或填写 `custom.fifth` 档案，勾选根目录与探针确认后启用，确认无需重启或重编译即可在“我的 Skills”勾选、预览、应用与恢复。

---

## 4. 真实 Agent 实际发现与加载验收记录表（`A14`）

完成第 3 节隔离验收并确认无误后，方可由操作人员在已备份前提下对本机真实安装的四个 Agent 逐一验证 Skill 是否被目标 Agent 实际发现和加载。

> 操作前请确认：仅分发已知安全、无敏感凭据的测试 Skill（例如只包含纯文本格式说明的 `skill-hub-smoke-note`），并在测试结束后通过“变更历史 -> 预览恢复”验证可无损撤出加载目录。

| Agent | 验证日期 | 宿主机 OS / 架构 | Agent 实际版本 | 选择范围与 Variant | 实际写入路径 | Agent 是否发现/列出该 Skill | 调用验证现象 | 历史恢复撤出验证 | 记录人 |
|---|---|---|---|---|---|---|---|---|---|
| Codex | 待填写 | 待填写 | 待填写 | `user` / `project`（`default`） | `~/.agents/skills/<name>` 或 `<project>/.agents/skills/<name>` | 待核对 | 待核对 | 待核对 | 待填写 |
| Claude Code | 待填写 | 待填写 | 待填写 | `user` / `project`（`default`） | `~/.claude/skills/<name>` 或 `<project>/.claude/skills/<name>` | 待核对 | 待核对 | 待核对 | 待填写 |
| Antigravity | 待填写 | 待填写 | 待填写 | `user` / `project`（`app` / `cli` / `ide`） | `~/.gemini/config/skills/<name>` 或 `<project>/.agents/skills/<name>` | 待核对 | 待核对 | 待核对 | 待填写 |
| Grok Build | 待填写 | 待填写 | 待填写 | `user` / `project`（`default`） | `~/.grok/skills/<name>`（并核对 `.agents` / `.claude` 兼容读取表现） | 待核对 | 待核对 | 待核对 | 待填写 |
