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
