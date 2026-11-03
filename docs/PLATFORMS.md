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
