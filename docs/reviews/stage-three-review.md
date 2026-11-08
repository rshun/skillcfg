# 第三阶段独立审阅与回归

审阅范围：6bf1c5e..17e80d1，Tasks 8–11；一名全新上下文 reviewer 只读审阅一次，包含源码、已确认原型、GitHub 截图与原生调用边界。中途曾遇模型配额限制，恢复同一 reviewer 的未完成审阅后已收到完整报告；没有启动第二 reviewer。

17e80d1 的核心/CLI 7 个作业、桌面/UI 8 个作业全部成功，但独立审阅仍发现下述问题，不能据此交付该候选。之后新增回归应先在 GitHub 观察失败，再进行一个修复批次；不在本机运行测试或构建。

| 编号 | 严重程度 | 发现及可见影响 | 回归与当前状态 |
|---|---|---|---|
| R1 | Critical | 普通 cargo release 未启用 Tauri custom-protocol，候选可能依赖开发服务器 | release_assets 断言非开发模式与嵌入 index；RED→GREEN，四平台原生验证通过 |
| R2 | Critical | 插件的异步 window.confirm 被当成布尔值，可能未经确认丢弃输入；enabled 未计入档案脏状态 | 原生异步 confirm 模拟、enabled 未保存确认；RED→GREEN，浏览器套件通过 |
| R3 | Important | 编辑/档案保存/停用忽略已 resolve 的失败回执，错误关闭并公告成功 | 三种失败 outcome、保存与停用保留弹窗/输入；RED→GREEN，浏览器套件通过 |
| R4 | Important | 校验失败后复用已消费 token；查看档案使停用 token 失效 | 修正后重试、外部基线变化拒绝、查看后停用；RED→GREEN，浏览器套件通过 |
| R5 | Important | 预览生成中取消后迟到结果未取消后端 ID；档案改字段未释放旧 ID | 精确取消迟到 ID 且保留新预览、档案 invalidation；RED→GREEN，单元与浏览器通过 |
| R6 | Important | 二进制移出、空目录和仅执行位变化缺少明确影响类型 | 核心安全 DTO 与浏览器预览明确类型；RED→GREEN，核心与浏览器通过 |
| R7 | Important | 新项目读取失败时保留旧范围状态；安装检测错误被吞掉 | 项目读取失败不复用一致/已安装结论；RED→GREEN，浏览器套件通过 |
| R8 | Important | 项目已移走时切用户范围仍带旧 projectRoot，导致无法退出该范围 | 用户范围显式空项目根；RED→GREEN，浏览器套件通过 |

Minor（保留）：无写入回执可能没有 operationId/backupLocation，界面仍显示空标签。它不影响逐组结果，记录后由用户决定是否纳入后续完善，不在本批扩展修复。

## 未重新判断的范围

- 安装包、各平台实际窗口启动、原生目录选择器人工交互与真实 Agent 加载属于 Task 12；R1 是本阶段候选构建错误，必须在本阶段修复。
- 前两阶段底层断电恢复和完整文件系统矩阵不重新验收；本次追踪桌面入口与结果保真。
- 浏览器演示与合成 IPC 不能证明原生写盘、WebView 焦点或系统辅助技术行为。

## 回归复现与最终验收

59ab380 曾因 GitHub 账单上限未分配 runner，不计为功能红灯。用户恢复账单后，回归提交 f295cca 在 GitHub 重跑实际观察到 R1–R8 全部八类目标断言失败（核心 diff、session 精确取消、浏览器 11 项新交互及原生 release_assets custom-protocol 断言）。

修复提交 b0b4b91 实现全部八类行为修复并完成四平台核心与原生桌面构建；58d6cd6 应用 GitHub 生成的 rustfmt 补丁并补齐 gui.test.mjs 设置校验提示的布尔转换断言。

最终代码验收：58d6cd6c07391f13f475ab0096b6811013f3ed8f。
- [Stage CI run 42443949755](https://github.com/rshun/skillcfg/actions/runs/42443949755)：7 个作业全部成功（Linux 113、macOS arm64 与 Intel 各 112、Windows 107 项测试，Clippy、rustfmt、锁文件及 CLI 打包通过）。
- [Desktop and UI run 42443949757](https://github.com/rshun/skillcfg/actions/runs/42443949757)：8 个作业全部成功（前端 16 项单元/契约测试、真实 Chrome 32 项场景及多尺寸/暗色截图、原生格式检查、Linux/Windows/macOS 双架构各 6 项命令测试与 release_assets 内嵌前端校验、Clippy 及四个桌面候选构建通过）。

审阅结论：R1–R8 已通过 RED→GREEN 关闭，第三阶段代码验收完成；归档提交继续同一双工作流，保留开发分支，不自动合并或发布。
