# Skill Hub 阶段进度

分支：dev/skill-hub。基线：6c4a93a。只在 GitHub Actions 编译与测试。

| 阶段 | 范围 | 状态 | 验证证据 |
|---|---|---|---|
| 设计归档 | 设计、计划、原型、审阅记录 | 已提交并推送 b0e19ce | 当前文档静态核对，历史快照原文保留；不是产品测试 |
| 第一阶段 | Tasks 1–3：完整包、档案/快照/发现、分发计划 | 已验收 | 9e02750 / run 41289351712，6 个必需作业全部成功 |
| 第二阶段 | Tasks 4–7：事务、库/导入、预览/恢复、CLI | 已实现、审阅修复及代码验收；归档提交继续 CI | 9df300c / run 41637391199，6 个必需作业全部成功 |
| 第三阶段 | Tasks 8–11：界面与三平台桌面桥接 | 已验收 | 3f2057e / Stage CI 42445134470（7 项成功）、Desktop and UI 42445134493（8 项成功） |
| 第四阶段 | Task 12：全流程、三平台安装包与人工验收手册 | 已实现、全分支审阅修复及代码验收；归档提交继续三条 CI；真实 Agent 加载（`A14`）留待宿主机人工核对 | a441284 / Stage CI 42450350844（7 项成功）、Desktop and UI 42450350883（8 项成功）、Release Bundles and Smoke 42450350841（6 项成功） |

阶段完成需要远端 commit、匹配最终 SHA 的必需 CI 绿灯与独立审阅。未开始、等待 CI 或失败不能写作完成。

## 执行记录

- 2026-11-01：用户授权创建/使用 dev/skill-hub、归档文档并继续第一阶段。
- 当前分支已为 dev/skill-hub；通过 git pull --ff-only origin master 核对为最新。
- 归档设计、计划、原型及历史审阅记录；未安装依赖或在本机编译/测试。
- 用户明确要求依赖及所有编译/测试/打包仅在 GitHub 执行，按此授权继续，避免重复询问同一执行边界。
- Task 1 已先写真实行为测试和最小未实现接口，提交到 GitHub 观察红灯后再完成实现；本机不执行 cargo。
- Task 1 红灯：42587a2，GitHub run 41268223837；最小实现：4af6cf1，run 41268519941 四平台行为测试通过，格式补丁从 GitHub 返回后应用。
- Task 2 红灯：7c4aba4，run 41268845418；Task 3 红灯：615a594，run 41269079843。已读取实际失败断言，未将依赖/格式错误冒充功能红灯。
- 独立审阅发现 3 项 Important、1 项 Minor；30e07dd / run 41287675767 的回归测试已观察三个重要问题红灯，4c7d27c / run 41287977699 四平台行为测试转绿。详见 [第一阶段审阅](reviews/stage-one-review.md)。
- 0a030ac / [run 41288863551](https://github.com/rshun/skillcfg/actions/runs/41288863551)：6 个 job 全部成功，覆盖 Linux x64、Windows x64、macOS arm64/Intel 的测试与 Clippy，以及格式和锁文件一致性。
- 归档收尾补充超过 260 字符的中文资源路径、库存入口别名合并、缺失探针/不完整覆盖证据保留已知读取者的用例。最终验收必须核对 [当前开发分支 CI](https://github.com/rshun/skillcfg/actions/workflows/ci.yml?query=branch%3Adev%2Fskill-hub) 的 headSha 与远端提交一致。

## 实施判断

- 使用用户明确指定的干净开发分支在当前 checkout 实施，不另建 worktree；无其他工作区改动需要迁移。
- TDD 红/绿证据均由 GitHub Actions 提供；本机不执行技能辅助脚本中会运行测试的步骤。
- 临时执行账本保留在 .superpowers 目录，不自动删除；正式进度和阶段证据记录在本文件。
- 规划源树经完整预算预检后使用 Arc 共享；最多 4,096 个配对，公开文本每侧 64 KiB、每视图 1 MiB，截断有标记，原始字节不截断；大批次需分批。
- Tasks 1–3 的部分测试合并为场景用例，名称不逐条照抄提案。Task 3 仅验证计划的 Skipped/Unchanged；不产生备份或实际 MutationPlan 的转换验证属于 Task 4，不在只读阶段伪造写入证据。
- 保留开发分支与现场，不自动合并、发布或清理。当前执行 Tasks 4–7，沿用已授权 GitHub 验证边界。

## 第二阶段执行记录

- 用户授权完成 Tasks 4–7，完成后提交 dev/skill-hub；基线 9e02750。
- IO、库、分发与 CLI 均先通过 GitHub 观察缺失功能的失败，再实现；新增回归覆盖真实进程退出、活跃锁、准备阶段放弃、旧编辑基线和替换前完整备份。
- 37488f6 / run 41448764980：四平台真实进程退出恢复、活跃锁拒绝接管、锁争用与项目读取者用例通过；替换旧 revision 备份和历史库根保护负例按预期失败后修复。
- acb93ba / run 41449424031 Linux job 125290772854：确认来源 URL 脱敏、已验证事务残留锁、标量提交顺序三处负例；修复后继续完整矩阵。
- 第二阶段只有一次新的整体验收审阅；不将之前第一阶段审阅冒充本阶段审阅。六类 Important 已经具名回归在 GitHub 复现并修复；最终代码提交 9df300c 的 6 个必需作业全部通过，尚未进入第三阶段。

实施判断：标准库 CLI 参数解析，复用现有锁定依赖；保留原 checkout 和全部中断现场，不自动清理；已知历史位置索引只追加，恢复不撤销它；preparing 日志只允许明确放弃，不以不完整备份覆盖后续编辑。详细边界见 [SAFETY](SAFETY.md)。

- de8b88a / run 41451315947 的 Linux 日志已确认六类审阅回归及两个目标绑定子场景全部按预期失败。f2eaaf6 / run 41634382261 的 Linux 107 项测试全部通过，包含 /tmp 与 /dev/shm 不同设备上的锁释放和真实崩溃恢复。中央备份跨文件系统实测目前仅声明 Linux，其他平台通过原生事务与锁测试。

## 第二阶段验收与交付

- 代码验收：9df300c3d02484639a8208045ee4c0d5ebe81ea9 / [run 41637391199](https://github.com/rshun/skillcfg/actions/runs/41637391199)，4 个平台测试/Clippy/release 作业与 lockfile、quality 共 6 项成功。
- 测试：Linux 109、macOS arm64 与 Intel 各 108、Windows 103。Unix 执行位/权限、Linux 跨设备与 Windows 共享句柄失败测试按系统区分，不将条件跳过计为覆盖。
- 四个 CLI 制品由 GitHub 构建上传；同一可执行文件以运行时 JSON 接入第五个 Agent，完成 list/import/diff/apply/restore，测试比较二进制字节未变。
- 独立审阅六类 Important 通过 RED→GREEN 关闭，详见 [审阅记录](reviews/stage-two-review.md)。库存实际路径/link 展示这一既有 Minor 留到桌面契约阶段。
- 保留 dev/skill-hub 与所有现场，不合并主干、不发布、不删除备份/旧 revision/临时日志。库/Agent 实际内容的恢复须再次预览并明确确认，不能靠回退代码替代恢复。
- 本机只阅读、编辑、静态检查和 Git；依赖、格式、测试、构建全部在 GitHub。第二阶段没有新增第三方依赖。

归档提交会再运行同一完整工作流；最终交付核对远端 headSha 与最新成功 CI 一致。第三阶段 Tasks 8–11 尚未开始。

- 归档边界补测：b0c3b08 / run 41636945159 复现 preparing 首次导入放弃后 missing_catalog；9df300c / run 41637391199 全部 6 个作业成功。显式恢复可补齐缺失 catalog，保留 staging、未引用文件及已有 catalog。四平台日志实际计数为 Linux 109、macOS 各 108、Windows 103。此前 c445bcb / run 41635052515 为补测前验收记录。

## 第三阶段执行记录（Tasks 8–11）

基线为已发布 CLI 的 master 合并提交 6bf1c5e，开发继续 dev/skill-hub。沿用已确认的雾蓝工作台，增加四页、导入/编辑/档案任务、手动预览/逐组回执/历史恢复及 Tauri 薄桥接，未进入 Task 12 安装包与实际 Agent 加载验收。

- 1f6a565 / run 41642529625：selection 精确配对、第五个 Agent、Rust/TS 预览契约按缺失行为失败。初次 Session 测试被 Node strip-only 不支持的参数属性阻断，先改 scaffold，不把语法错误当功能红灯。
- b9568f5 / run 41643139953：八项 Session 契约与六项原生命令契约均在实际 runner 编译后按缺失行为失败。原生用例覆盖伪造目标、未知包、窗口隔离、取消选择器、未授权档案和第五档案。
- 6e9c6cf / run 41644177305：只读分发状态、档案文档/版本绑定与迟到取消三个 App 回归失败后修复。状态读取不消费已就绪预览。
- 40274e4 / run 41647267240：四个原生桌面作业通过测试/check/Clippy/release；后端状态与原生源格式由 GitHub 补丁统一处理，未本机运行格式化器。
- 411fae0：完成四页和任务流，新增“拒绝范围切换必须回退”和“档案刷新保留已完成回执”负例。run 41650385034 实际复现两个失败，83440a8 / run 41821993804 的前端 15 项单元/契约测试与 12 项浏览器场景通过。
- 扩展浏览器验收覆盖损坏档案、停用语义、未保存确认、库切换、运行时自定义共享读取者、恢复失败消费与重载后待恢复提示。两个 Settings 初次错误来自测试辅助函数在异步字段挂载前输入，补齐等待条件后继续；不将其算为产品红灯。

已下载并人工查看 GitHub 截图：1024px、320px 和暗色库界面，中文可读、布局不靠整体缩放，保留 166px/64px 侧栏及窄窗口详情下移。最初 runner 无 CJK 字体导致方框，已只在 GitHub 安装系统中文字库；应用无在线字体依赖。

实施判断：独立原生 Cargo 工作区隔离 WebView 依赖，根核心/CLI 与桌面两条流水线都必须通过；代价是分别维护锁文件。目录选择用 Tauri 原生对话框，所有实际写入走已有应用服务。Task 10 的 Restore 复用 Preview 的恢复模式与 History 入口，避免复制一次性确认状态机。最终安装包、最低 OS、实际原生窗口/选择器和真实 Agent 加载仍属 Task 12。

补充视觉核对：已查看 Agents 1024px、History 1024px、Settings 320px 与 Preview 1024px 的 GitHub PNG；四页风格一致，窄设置表单换行，预览按实际目录分组。截图仅证明合成浏览器界面，不作为原生加载证明。

## 第三阶段验收与交付

同一 reviewer 已完成 6bf1c5e..17e80d1 的审阅，收到 2 类 Critical、6 类 Important 和 1 类 Minor；详见 [第三阶段审阅记录](reviews/stage-three-review.md)。全部 8 类问题接纳为一个修复批次，Minor（无写入回执空标签）暂不扩展。

- 回归复现：59ab380 曾遇 GitHub 账单上限未分配 runner，恢复账单后 f295cca 在 GitHub 实际观察到 R1–R8 全部八类目标断言红灯（核心 diff、session 精确取消、浏览器 11 项新交互与原生 release_assets custom-protocol 检查）。
- 修复与收尾：b0b4b91 完成全部八类行为修复并通过四平台核心与原生桌面构建；58d6cd6 应用 GitHub 生成的 rustfmt 补丁并补齐 gui.test.mjs 设置校验提示的布尔转换。
- 代码验收：58d6cd6c07391f13f475ab0096b6811013f3ed8f。
  - [Stage CI run 42443949755](https://github.com/rshun/skillcfg/actions/runs/42443949755)：7 个作业全部成功（Linux 113、macOS arm64 与 Intel 各 112、Windows 107 项测试，Clippy、格式、锁文件及 CLI 制品通过）。
  - [Desktop and UI run 42443949757](https://github.com/rshun/skillcfg/actions/runs/42443949757)：8 个作业全部成功（前端 16 项单元/契约测试、真实 Chrome 32 项交互场景与响应式/暗色截图、原生格式检查、四个平台各 6 项原生命令契约与 1 项内嵌生产资源测试、Clippy 及四个 `desktop-candidate-*` 制品上传）。
- 保留 dev/skill-hub 与所有现场，不合并主干、不自动发布、不删除任何备份或临时记录。归档提交 `3f2057e` 已通过 Stage CI `42445134470`（7/7）与 Desktop and UI `42445134493`（8/8），随后进入第四阶段 Task 12。

## 第四阶段执行记录与验收（Task 12）

用户授权实现第四阶段（基线 `3f2057e`），继续严格遵守“零新增依赖、零删除文件、不在本机编译/测试、不触碰 `master`、不自动打标签或发布 Release”红线。

- 新增端到端与回归验证：
  - `tests/fixtures/agents/custom.fifth.json`：标准第五个 Agent 运行时档案（含 `user-native`、`project-native`、`.agents/skills` 兼容读取根及 JSON 布尔探针 `.fifth/config.json`）。
  - `apps/web/src-tauri/tests/end_to_end.rs`：单隔离 `HOME` 真实端到端闭环（导入 Claude 多包 → 保存库 → 精确配对 Codex/Grok 且断言不扩展为 4 项 → 预览与共享可见性按物理根独立断言 → 应用并核对 UTF-8 中文/未知 frontmatter/CRLF/二进制字节/Unix 执行位 → 编辑库及改名但不自动应用 → 预览并确认恢复至加载目录干净状态），以及第五个 Agent 非法档案、未授权拦截、探针翻转使旧预览失效与恢复闭环。
  - `crates/skill-cli/tests/commands.rs`：CLI 端到端闭环、跨平台 `CON.txt` 保留名拦截、Unix 包内符号链接拦截、更新库记录不改动已分发目录，以及第五个 Agent 负例与恢复。
  - `apps/web/tests/gui.test.mjs`：补齐真实 Chrome 200% 缩放（`512x450` CSS 视口、`deviceScaleFactor: 2`）遍历四页与全部五个任务弹窗无横向溢出，产出 `library-zoom-200.png`、`agents-zoom-200.png`、`preview-zoom-200.png` 与 `gui-interaction-log.json`，下载后已人工核对中文字体清晰、弹窗头部/底栏固定正常。
- 三平台四架构安装包与启动/零重编译验证：
  - 新增 `apps/web/src-tauri/tauri.bundle.json`、`scripts/package_desktop.py`、`scripts/smoke_desktop.py` 与 `.github/workflows/release.yml`，在 `ubuntu-24.04`、`windows-2022`、`macos-15`（arm64）、`macos-15-intel`（x64）原生构建并校验全部 13 个桌面/CLI 安装包与免安装归档，执行原生桌面启动 smoke（Linux 配合 `xvfb-run`）以及对同一已编译二进制零重编译接入 `custom.fifth.json`（含二进制运行前后 SHA-256 完全相等断言）。
- 全分支独立审阅与 RED→GREEN 闭环：
  - `508c0bd` 在 `Stage CI run 42446908336` 实际复现 `C1` 红灯（`package.rs` 的 `valid_relative_path` 在非 Windows 平台提前返回 `true`，导致 Linux/macOS 未拦截 `CON.txt` 等不可移植保留名与控制字符），同时独立只读审阅对照 `A1–A20` 输出 `C1`、`I1–I3`、`M1–M4`（详见 [第四阶段审阅记录](reviews/stage-four-review.md)）。
  - `eb1cea5` 完成 `C1`、`I2`、`M2–M4` 修复并使 `Stage CI`（`42448143220`）与 `Release Bundles and Smoke`（`42448143228`）转绿；`a441284` 消除 `end_to_end.rs` 在 Windows 下的条件导入告警。
- 代码验收：`a44128460d9e54b3ae246bb4c021f4a061579491`（三条工作流共 21 个作业 100% 成功）：
  - [Stage CI run 42450350844](https://github.com/rshun/skillcfg/actions/runs/42450350844)：7 个作业全部成功（Linux 114、macOS arm64 与 Intel 各 113、Windows 108 项测试）。
  - [Desktop and UI run 42450350883](https://github.com/rshun/skillcfg/actions/runs/42450350883)：8 个作业全部成功（前端 16 项单元/契约测试、真实 Chrome 34 项测试〔33 个交互子场景〕、四平台各 9 项原生桌面测试与候选构建）。
  - [Release Bundles and Smoke run 42450350841](https://github.com/rshun/skillcfg/actions/runs/42450350841)：6 个作业全部成功（汇总上传 `skill-hub-release-bundles`，内含 13 个安装包/归档、`SHA256SUMS.txt`、`BUILD-BASELINE.md` 与四份平台运行报告）。
- 诚实边界：系统原生目录对话框人工点击、安装向导图形点击以及四个真实外部 Agent 的实际发现/加载（`A14`）须由操作人员在宿主机按 [人工验收手册](MANUAL-VALIDATION.md) 执行并填写记录表，不以 CI 通过代替真实 Agent 加载证明。阶段归档提交将继续在 GitHub 跑完上述全部三条流水线并核对最终 HEAD SHA 全绿。
