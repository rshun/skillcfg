# 平台验证矩阵

第一阶段交付 Rust 核心/只读 IO；第二阶段交付写入事务、应用服务和 CLI；第三阶段交付桌面工作台与 Tauri 原生桥接；第四阶段交付三平台桌面安装包、CLI 归档、启动与零重编译验证及人工验收手册。

| GitHub runner | 构建与测试架构 | 桌面产物 | CLI 产物 | 原生运行与自动化范围 |
|---|---|---|---|---|
| `ubuntu-24.04` | Linux `x86_64` | `.AppImage`、`.deb` | `.tar.gz`（及 `ci.yml` `.zip`） | 核心/IO/应用/CLI 测试、真实 Chrome 33 项交互与 200% 缩放截图、Tauri 原生桥接测试、`xvfb-run` 桌面启动 smoke、零重编译第五个 Agent 验证 |
| `windows-2022` | Windows `x86_64` | NSIS `-setup.exe`、`.msi`、免安装 `-portable.zip` | `.zip` | 核心/IO/应用/CLI 测试（含 Windows 保留名、junction、共享句柄冲突、长路径）、Tauri 原生桥接测试、Win32/WebView2 桌面进程启动 smoke、零重编译第五个 Agent 验证 |
| `macos-15` | macOS `arm64`（Apple Silicon） | `.dmg`、`.app.tar.gz` | `.tar.gz`（及 `ci.yml` `.zip`） | 原生 `arm64` 编译与运行测试（非交叉编译替代）、Unix 执行位与 rename 恢复、Tauri 原生桥接测试、桌面进程启动 smoke、零重编译第五个 Agent 验证 |
| `macos-15-intel` | macOS `x86_64`（Intel） | `.dmg`、`.app.tar.gz` | `.tar.gz`（及 `ci.yml` `.zip`） | 原生 `x86_64` 编译与运行测试（非交叉编译替代）、Unix 执行位与 rename 恢复、Tauri 原生桥接测试、桌面进程启动 smoke、零重编译第五个 Agent 验证 |

核心 manifest 的最低 Rust 声明为 1.85；统一验证与打包工具链固定为 Rust `1.90.0`、Node `24`。runner 名称和架构核对自 [GitHub 官方说明](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)。所有系统构建依赖仅在 GitHub Actions 临时 runner 安装，不在用户或开发者本机安装。

## 操作系统与运行库最低基线

- **Windows x64**：64 位 Windows 10（1803+）或 Windows 11 x64，需 Microsoft Edge WebView2 Runtime。当前产物未做 Authenticode 代码签名。
- **macOS arm64 与 macOS x64**：分架构独立原生产物，最低目标为 macOS 11+（构建 SDK 为 macOS 15），使用系统自带 `WKWebView`。当前产物未做 Apple Developer 签名与公证（Notarization）。
- **Linux x64**：基于 `ubuntu-24.04` 构建，**仅声明支持 `glibc >= 2.39` 的 x86_64 发行版**（Ubuntu 24.04 LTS 及同代 Debian 系），运行需 `libwebkit2gtk-4.1-0`、`libgtk-3-0`、`libayatana-appindicator3-1`、`librsvg2-2` 与图形会话；**不把 Ubuntu 24.04 runner 通过宣称为兼容所有 Linux 发行版或旧版 glibc**。

## 真实平台执行、受控模拟与人工验收边界

- **真实平台自动执行**：四平台核心/IO/应用/CLI 测试、`end_to_end.rs` 端到端导入/精确配对/共享可见性/编辑不自动分发/改名恢复测试、零重编译接入 `custom.fifth.json` 全流程与二进制 SHA-256 不变校验。Unix 执行位仅在 Linux/macOS 实测；跨设备中央备份仅在 Linux（`/tmp` 与 `/dev/shm`）实测；Windows 共享句柄占用仅在 Windows 实测。
- **受控模拟与无头自动化**：真实 Chrome 界面回归（四页、全部任务弹窗、320–1024px、暗色、键盘 `Escape` 焦点、200% 缩放截图与交互日志）在 Ubuntu 执行内存 IPC fixture；Linux 桌面启动 smoke 使用 `xvfb-run` 虚拟显示验证原生进程存活与零启动写入。
- **宿主机人工核对项**：系统原生目录选择器窗口点击、图形安装向导/Gatekeeper/SmartScreen 放行，以及四个真实 Agent 的实际发现与加载结果（`A14`），详见 [人工验收手册](MANUAL-VALIDATION.md)。
