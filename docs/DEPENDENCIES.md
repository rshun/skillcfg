# 第一、二阶段依赖与执行边界

状态：用户已明确依赖、编译、测试和打包均在 GitHub 执行；按这一边界继续，不在本机安装。当前版本清单与本阶段 CI 一起提交，构建结果以 GitHub 为准。

## 直接依赖提案

以下版本从参考项目 mcpconfig 的 Cargo.lock 核对，不顺带升级。用户已授权在 GitHub 临时 runner 中处理这组锁定依赖；本机不安装。第二阶段仅复用 serde/serde_json 给 IO/CLI，没有新增第三方包，CLI 参数解析使用标准库。

| 包 | 精确版本 | 用途 |
|---|---|---|
| serde | 1.0.229 | 内部模型与 JSON 档案的安全结构化解析 |
| serde_json | 1.0.154 | JSON 档案、schema 与合成 fixtures |
| serde_yaml | 0.9.34（上游元数据标记 deprecated） | SKILL.md frontmatter 校验；未知字段和原文仍保留 |
| thiserror | 2.0.21 | 分类错误；不把原文凭据放进错误消息 |
| toml_edit | 0.22.27 | 只读布尔探针的 TOML 字段解析 |

serde_yaml 为参考项目所用版本，已知不再积极维护。本阶段仅处理有尺寸限制的本地 frontmatter，错误不外泄原文；本次不擅自替换或安装新的 YAML 库。若用户不同意该版本，先选择替代方案再继续。

本阶段不引入 Tauri、React、Node 包、动态插件或额外摘要库。后续阶段依赖另列清单，不视为本次批准。

## 运行地点

- 本机：阅读、编辑、文档/配置静态核对、Git 操作和读取 GitHub 结果；不安装依赖、不编译、不运行产品测试。
- GitHub-hosted 临时 runner：生成 Cargo.lock、按锁文件下载依赖、Rust 编译/测试/Clippy/格式检查。
- Cargo.lock 只在 GitHub 生成，通过工作流制品取回并精确暂存；本机不运行 cargo generate-lockfile。
- 首次生成锁文件时以锁文件制品作为所有矩阵作业共同输入，之后使用已提交锁文件；不依赖各平台各自解算出相同依赖。
- CI 的外部 action 复用参考项目已使用的固定 commit：actions/checkout 与 dtolnay/rust-toolchain；上传/下载制品使用 GitHub 官方 action。确切引用会随 CI 文件一起审阅。

阶段一面向 Linux/macOS/Windows 的核心与只读能力；桌面构建和安装包属于后续阶段，不将本阶段核心测试通过说成桌面已可用。

第二阶段生成的 CLI 为纯 Rust 命令行程序，四个原生 runner 在测试和 Clippy 后构建 release 二进制并上传 Actions 制品。锁文件图由 GitHub 更新后取回；桌面/Node 依赖不包含在本阶段。

## 第三阶段桌面依赖

用户已授权第三阶段继续在 GitHub 处理全部依赖、编译、测试和打包；本机仍不安装。前端沿用参考项目锁定版本，原生插件版本由 GitHub 解算后精确锁定。新增依赖仅服务已确认桌面界面与原生目录选择。

| 包 | 精确版本 | 用途 |
|---|---|---|
| react / react-dom | 19.3.1 | 工作台与任务弹窗 |
| @types/react / @types/react-dom | 19.3.0 | TypeScript 类型 |
| @tauri-apps/api / @tauri-apps/cli | 2.12.4 | 固定命令 IPC 与构建图标工具 |
| vite / @vitejs/plugin-react | 8.3.5 / 6.1.2 | 前端构建 |
| typescript | 5.9.3 | 严格类型检查 |
| tauri / tauri-build | 2.12.5 / 2.7.4 | 薄桌面外壳与资源嵌入 |
| tauri-plugin-dialog | 2.8.4 | 原生目录选择，取消返回 None |

`apps/web/package-lock.json` 与独立原生工作区 `apps/web/src-tauri/Cargo.lock` 均由 GitHub 生成后归档；原生锁图共 420 个包。原生工作区隔离系统 WebView 依赖，根工作区核心/CLI 流程维持独立；两条流水线均为阶段必需检查。

浏览器测试使用 runner 已有 Chrome 和 Node 标准库 CDP，不新增浏览器测试框架。Linux 系统构建依赖（WebKitGTK/AppIndicator/librsvg/patchelf）与截图中文字库 fonts-noto-cjk 只在 GitHub 临时 runner 安装；所有 Actions 固定 commit，不使用浮动 tag。
