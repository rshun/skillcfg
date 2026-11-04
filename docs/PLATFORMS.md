# 平台验证矩阵

第一阶段交付 Rust 核心/只读 IO；第二阶段新增写入事务、应用服务和 CLI。桌面端仍在后续阶段。

| GitHub runner | 第一阶段目标 | 后续桌面目标 |
|---|---|---|
| ubuntu-24.04 x64 | Rust 1.90，核心/IO/应用服务 | Linux x64 AppImage/Debian 与 CLI |
| windows-2022 x64 | 同上，Windows 路径与链接行为 | Windows x64 桌面/CLI |
| macos-15 arm64 | 同上，原生 arm64 运行 | macOS arm64 桌面/CLI |
| macos-15-intel x64 | 同上，原生 Intel 运行 | macOS x64 桌面/CLI |

核心 manifest 的最低 Rust 声明为 1.85；初始统一验证工具链为 1.90.0。MSRV 独立验证与最终 OS/WebView/系统库基线将在对应构建阶段补齐，不能把 manifest 声明说成已经证明最低版本兼容。

runner 名称和架构核对自 [GitHub 官方说明](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)。Linux 桌面及所有打包只在后续 GitHub 工作流准备相应依赖；不在本机安装。

## 第二阶段 CLI 制品

同一矩阵运行完整 workspace 测试、平台 Clippy 与 CLI release 构建，上传 skill-hub-ubuntu-24.04-X64、skill-hub-windows-2022-X64、skill-hub-macos-15-ARM64、skill-hub-macos-15-intel-X64。制品保留 14 天；它们是分支验证产物，不等于正式发布或桌面安装包。

Unix 执行位用例只在 Linux/macOS 声明通过；Windows 使用原生共享句柄制造 rename 失败，并使用 junction 验证链接防护。真实崩溃用例在独立 fixture 子进程调用 exit，验证未执行 Drop 时的残留锁；不用真实 Agent 目录制造故障。

第二阶段代码验收 c445bcb / run 41635052515：6 个必需作业成功，Linux 108、macOS arm64/Intel 各 107、Windows 102 项；四个 CLI release 制品已上传。跨设备测试以 Linux 的设备 ID 不同为前提实测，未声称在 macOS/Windows 分别配置第二块卷实测。

## 第三阶段桌面候选

`desktop.yml` 独立原生工作区在以上四个 runner 原生运行桥接测试、cargo check、Clippy、release 构建；不是只交叉编译。原生编译包含 React/Vite 前端资源。`desktop-candidate-ubuntu-24.04-X64`、`desktop-candidate-windows-2022-X64`、`desktop-candidate-macos-15-ARM64`、`desktop-candidate-macos-15-intel-X64` 为可执行候选，完整安装包在第四阶段。

Linux runner 安装 WebKitGTK 4.1 开发库和 AppIndicator 等，Windows 使用系统 WebView2，macOS 使用 WebKit。当前只声明 runner OS 构建基线；最低系统版本、GUI 启动、原生目录选择器人工交互、签名、公证、安装/升级及真实 Agent 加载尚未验收。跨平台桥接单元测试验证路径授权、选择器取消返回与窗口间预览隔离，不能替代真实系统对话框操作。

真实 Chrome 界面测试目前在 Ubuntu 执行，覆盖动态第五个 Agent、导入/编辑、档案根确认、范围切换、逐组结果与恢复，以及 320–1024px 和暗色截图；未声称在三个系统分别执行同一浏览器 GUI 自动化。
