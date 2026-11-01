# 平台验证矩阵

第一阶段只交付 Rust 核心/IO/应用服务的只读能力，不包含桌面端或打包。

| GitHub runner | 第一阶段目标 | 后续桌面目标 |
|---|---|---|
| ubuntu-24.04 x64 | Rust 1.90，核心/IO/应用服务 | Linux x64 AppImage/Debian 与 CLI |
| windows-2022 x64 | 同上，Windows 路径与链接行为 | Windows x64 桌面/CLI |
| macos-15 arm64 | 同上，原生 arm64 运行 | macOS arm64 桌面/CLI |
| macos-15-intel x64 | 同上，原生 Intel 运行 | macOS x64 桌面/CLI |

核心 manifest 的最低 Rust 声明为 1.85；初始统一验证工具链为 1.90.0。MSRV 独立验证与最终 OS/WebView/系统库基线将在对应构建阶段补齐，不能把 manifest 声明说成已经证明最低版本兼容。

runner 名称和架构核对自 [GitHub 官方说明](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)。Linux 桌面及所有打包只在后续 GitHub 工作流准备相应依赖；不在本机安装。
