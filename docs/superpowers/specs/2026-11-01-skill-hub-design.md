# Skill Hub 完整设计方案

日期：2026-11-01。修订：可配置 Agent 档案版。状态：第一阶段实施与 GitHub 验证进行中，后续阶段尚未开始。用户已明确所有依赖下载、编译、测试与打包均在 GitHub 执行，本机只编辑与静态检查。

项目目录：`C:\Dev\skillcfg`。产品名暂定 **Skill Hub**，命令名暂定 `skill-hub`。

## 1. 已确认的要求与本方案的决策

用户已确认：

- 参考 `C:\Dev\mcpconfig` 的功能和界面，管理对象由 MCP 改为 Skill。
- 首版支持 Codex、Claude、Antigravity、Grok；本文按 Claude Code 和 xAI Grok Build 处理。
- 从本机已有 Agent 读取 Skills，收集到统一库，再按 Skill 独立选择目标 Agent。
- 每次预览后手动应用；禁止自动同步。保存、编辑、导入和启动均不自动分发。
- 采用已展示的蓝白工作台视觉，要求美观、精致，遵循 frontend-design 技能。
- 内置四个 Agent，同时支持可配置 Agent 档案；普通新增 Agent 尽量只加配置，不改代码或重新编译。
- Linux、macOS、Windows 均支持；本方案按三平台均提供桌面端和 CLI 落实。
- 每开发完一个阶段，必须提交到仓库；所有编译在 GitHub Actions 中完成。
- 本轮更新设计和实施计划，不开始产品实现、不安装依赖。后续阶段的 commit/push 和必要 CI 验证提交纳入工程流程；不授权直接修改主干或自动发布。

以下是设计者的建议，供本次方案审阅，不冒充用户已经逐项确认：

| 编号 | 建议决策 | 理由 |
|---|---|---|
| D1 | Rust 核心与应用服务 + React/TypeScript + Tauri 2 | 延续 mcpconfig，便于复用经验和界面部件 |
| D2 | 用完整目录复制分发，首版不创建用于实时同步的符号链接 | 每个目标保留明确副本，更新仍需预览确认 |
| D3 | 同时提供用户级与项目级范围 | 延续参考项目；四个 Agent 均有对应发现机制 |
| D4 | 首版接收本地 Agent 和本地目录，不接入 GitHub 安装、市场、云端同步 | 先完整做好已有 Skill 的管理闭环 |
| D5 | Linux/macOS/Windows 均提供桌面端与 CLI | 按用户三平台要求执行，Windows 优先人工验收不代替另外两平台验证 |
| D6 | 每阶段验收后保留 commit 并 push 到开发分支；所有编译在 GitHub Actions | 用户已明确要求阶段提交和云端编译，不在本机编译 |
| D7 | 首版不管理原生启停开关，不自动执行 Skill 脚本或安装其依赖 | 各 Agent 的启停与权限语义不同；“分发”不等于“运行” |
| D8 | 库内采用追加式包版本，catalog 原子切换当前版本引用 | 包与索引中断时不会出现半包或新内容配旧名称 |
| D9 | 修改 name 后保留并提示旧分发位置，不自动卸载旧名称 | 避免更名隐式删除文件，也不掩盖两个名称可能同时被加载 |
| D10 | 内置四个 Agent 与用户档案走相同的声明式注册表 | 普通新增 Agent 只增加本机 JSON 档案，无需修改 Rust/TS 或重编译 |
| D11 | 档案只表达有限的目录/发现规则，不运行脚本或加载动态代码 | 配置扩展复用现有备份、路径校验和确认边界 |

D5、D6、D10 依据用户最新要求；其他技术细化为设计者的实施建议。

## 2. 产品目标与范围

用户维护一份自己的工作方法库，在不同 Agent 间有选择地应用；能看清来源、文件、差异、实际写入位置和其他可能受影响的 Agent。

首版必须闭环：只读扫描 → 查看 → 导入到库 → 编辑 SKILL.md → 选择目标 → 预览 → 手动应用 → 写后核对 → 历史与恢复。另须打通自定义 Agent 档案的添加、目录授权、启用与通用分发闭环。

不纳入首版：远程主机推送、自动同步、后台自动更新、公共市场、依赖自动安装、Skill 内容自动改写为其他 Agent 的语义、插件安装管理、账户同步、自动运行验证、自动卸载或永久删除。

不得发生的结果：启动即改配置、只复制 SKILL.md 导致资源丢失、同名静默覆盖、预览后内容变了仍写入、共享目录被当作独立目标、错误提示泄露正文凭据、把复制成功描述为“Skill 可正常执行”。

## 3. 参考项目与复用方式

以当前实现为准，不把 mcpconfig 的历史架构稿中的规划当成现有功能。

| 参考文件 | 可借鉴内容 | Skill 版的区别 |
|---|---|---|
| `apps/web/src/App.tsx` | 导航、工作范围、任务弹窗 | 加入目录包和兼容性展示 |
| `apps/web/src/pages/Library.tsx` | 列表、详情、逐项目标选择 | 主要对象变为 Skill 包 |
| `apps/web/src/pages/Panels.tsx` | Agent 查看和历史 | 展示根目录、来源别名和文件清单 |
| `apps/web/src/components/Dialog.tsx` | 焦点管理、取消与未保存确认 | 复用交互原则，不复制旧数据模型 |
| `crates/mcp-app/src/distribution.rs` | 一次性预览、快照、上下文复核 | 快照扩展到完整目录树和二进制文件 |
| `crates/mcp-app/src/planning.rs` | 物理路径去重和共享影响 | 写入单位改为 Skill 目录 |
| `crates/mcp-io/src/lib.rs` | 备份、回执、审计、失败区分 | 现有文本文件写入引擎不能直接当作目录事务 |

保持两个项目独立：不修改 mcpconfig，不建立跨仓库运行时路径依赖，不整仓复制和替换字符串。复制具体部件前核对适用性并保留 MIT 声明。

## 4. Agent 发现、适配与写入目录

下表来源为 2026-11-01 查阅的官方文档/上游文档。它是适配基线，不证明本机安装版本支持全部规则；Task 1 必须补齐版本化样例与证据。

| Agent | 用户级主要根目录 | 项目级主要根目录 | 兼容读取与注意事项 |
|---|---|---|---|
| Codex | `~/.agents/skills/` | `<project>/.agents/skills/` | 兼容安装常见的 `~/.codex/skills/` 作为候选只读来源，写入前核对版本；系统内置目录不作为写入目标 |
| Claude Code | `~/.claude/skills/` | `<project>/.claude/skills/` | 插件命名空间、账户同步和 commands 不等于普通 Skill 目录 |
| Antigravity | `~/.gemini/config/skills/` | `<project>/.agents/skills/` | 旧版 `.agent/skills`、`~/.gemini/antigravity/skills` 等作为兼容候选；CLI 独有根目录不冒充 App/IDE 通用路径 |
| Grok Build | `~/.grok/skills/` | `<project>/.grok/skills/` | 还会读取 `.agents/skills`，默认兼容 Claude/Cursor 的 Skills；须展示兼容开关未知或当前状态 |

注册表与目录解析器应区分 **read_roots**、**write_root**、**possible_readers**，不能用一张固定目录表承担三种含义。四个内置 Agent 是默认档案，不是代码允许列表；自定义档案复用同一发现与分发流程。

- 只扫描明确登记的根目录及其 Skill 子目录；不递归扫描整个 HOME、磁盘或全部插件缓存。
- 项目范围必须明确选取已存在的绝对目录；不以应用启动目录作为默认项目。
- Codex/Grok/Antigravity 的父目录发现按从选定目录至 Git 仓库根目录枚举；找不到 Git 根时只使用明确选择的项目目录，不向 HOME 无限上溯。
- 祖先目录中的 Skill 作为只读来源展示；项目写入只到明确选定项目的 write_root，不偷偷回写父目录。
- 多个根目录有同名 Skill 时全部保留来源，不将某个 Agent 的优先级规则强套给其他 Agent。
- 插件 Skill 首版可通过“本地目录”单包导入；自动枚举插件缓存、账户同步目录和旧 commands 转换留待后续。
- 只用文件存在不能证明 Agent 已安装；安装检测、Skill 目录是否存在、单个 Skill 是否有效分别展示。
- 不明确的版本/兼容状态标为“可能读取”或“需要核对”，不默认为支持，也不自动修改兼容开关。
- Antigravity 的 App/CLI/IDE 由所选配置档案区分；无证据确认的旧版配置只读，未核实写入根时禁用应用并给出原因。

主要来源：

- [OpenAI：Build skills](https://learn.chatgpt.com/docs/build-skills)
- [Anthropic：Extend Claude with skills](https://code.claude.com/docs/en/skills)
- [Google：Agent skills](https://antigravity.google/docs/skills)
- [xAI Grok Build：Skills](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/08-skills.md)

### 4.1 配置扩展的契约

`AgentId` 改为经校验的字符串，不使用固定四项枚举。词法构造器只校验格式，不包含内置 ID 名单；加载器再按来源检查命名空间。内置 ID 为 `codex`、`claude-code`、`antigravity`、`grok`；用户 ID 使用 `custom.<slug>`，slug 符合小写字母/数字与单个连字符分段，完整 ID 不超过 64 字符。ID 不可重复或覆盖内置 ID；显示名可以修改，ID 创建后固定，历史记录用 ID 与档案快照关联。

内置定义放源码 `profiles/builtin/*.json` 并随软件打包；用户档案放 `~/.skill-hub/agents/*.json`，运行时加载，无需编译。两者通过同一个 JSON schema v1 解析和校验，Rust、TypeScript、CLI、UI 均不得列举四个 Agent 的分支来决定通用流程。

档案至少包含 schemaVersion、id、displayName、packageFormat、variants/defaultVariant、roots；可选 iconKey/monogram、发现提示、证据备注与有限的兼容条件。iconKey 从产品内置图标集中选择，未知图标使用通用图标；不接受 SVG/HTML/脚本、远程图标或代码 URL。

有限规则集如下；增加规则枚举属于引擎开发，使用现有规则不需要开发：

| 字段/能力 | 首版允许内容 |
|---|---|
| packageFormat | `skill-md-v1`，完整 SKILL.md 目录包；未知格式阻止应用 |
| 平台 | `windows`、`macos`、`linux`；每个根显式声明适用平台 |
| 路径 | `base: home/project/appdata/xdg-config` + 非空 relative，不执行表达式、任意环境变量插值或命令 |
| 发现 | `children`（根目录的包）、`ancestors-to-repo`（项目祖先根）；始终应用包大小/深度上限 |
| 范围与访问 | user/project；read 表示扫描来源/已声明读取根，write 表示可作为 Hub 目标；write=true 必须同时 read=true，每平台/范围/variant 最多一个静态写入根 |
| 来源语义 | native/compatibility，根有稳定 rootId；发现优先级用整数 priority 表达，冲突展示不由优先级消除 |
| variant | 顶层 variants/defaultVariant；RootSpec.appliesToVariants 关联根，省略表示适用于全部已声明 variant，当前选择进入上下文与预览快照 |
| 读取状态证据 | RootSpec.readEvidence 为 always 或 bool-config-v1 探针引用；只改变读取提示，不开启/关闭写入资格 |

根的 variant 选择规则固定：appliesToVariants 为非空、不重复的已声明 variant ID 数组，未知引用或空数组无效；省略时等于全部 variants。defaultVariant 必须属于 variants，每个任务每个 Agent 只选一个 currentVariant。按当前平台、范围和 currentVariant 选写根；多个候选不靠 priority 猜选，而是判档案无效。项目祖先产生的其他读取路径从不成为隐式写根；当前 variant 无写根时阻止写入，不回退到另一个 variant。

档案可选 probes 数组，每个探针固定 probeId/base/relative/format(json|toml)/keyPath(字符串数组)/expected(布尔)/onMissing(true|false|unknown)。RootSpec.readEvidence 只能是 `{kind: "always"}` 或 `{kind: "bool-config-v1", probeId: "..."}`，省略为 always，引用缺失探针无效。只返回匹配结果 true/false/unknown，不向 UI 暴露其他字段；探针路径同样纳入档案授权，缺失按 onMissing，解析失败/非布尔/超限一律 unknown，单个文件上限 4 MiB。

读取证据不充当文件访问授权或写入条件，规则为：

| readEvidence 结果 | 来源/共享提示 | 写入资格 |
|---|---|---|
| always 或 true | 已声明的潜在读取者；true 可加“配置中启用” | 只由平台/范围/variant 的唯一 write 根、档案/目录授权和路径校验决定 |
| false | 标为“配置中未启用，实际进程仍可能受其他来源覆盖”，保留潜在影响提示 | 不因 false 自动禁用，也不另选写根；预览提示可能不被 Agent 加载 |
| unknown | 标为“读取状态未知”，保守保留潜在影响提示 | 不因 unknown 扩大授权或自动启用；同样按静态写根规则判断 |

对已启用且根已获授权的档案，有限扫描可列出当前 variant 根内已有包，并保留上述证据状态，不把“发现文件”说成实际加载。探针只证明可读到的配置状态，不证明真实进程状态；首版不把任意环境变量读取、shell、正则脚本或插件代码作为条件语言。需要按运行时值切换写入根的 Agent，必须由用户显式选择 variant/覆盖；不做隐式条件写入。

档案保护上限：单文件 256 KiB、JSON 嵌套深度 16；注册表最多 256 个档案；每档案最多 32 个根、8 个 variant 和 8 个布尔探针。rootId/variant/probeId 为 1–64 字符的合法 slug，displayName 为 1–80 字符纯文本。超限阻止新档案启用，不截断规则；不通过引入脚本逃避规则上限。

普通新 Agent 的最小档案示例（仅演示，非已实现的产品配置）：

```json
{
  "schemaVersion": 1,
  "id": "custom.my-agent",
  "displayName": "My Agent",
  "packageFormat": "skill-md-v1",
  "variants": ["default"],
  "defaultVariant": "default",
  "roots": [
    {
      "rootId": "user-skills",
      "platforms": ["windows", "macos", "linux"],
      "scope": "user",
      "base": "home",
      "relative": ".my-agent/skills",
      "discovery": "children",
      "role": "native",
      "read": true,
      "write": true,
      "priority": 100
    },
    {
      "rootId": "project-skills",
      "platforms": ["windows", "macos", "linux"],
      "scope": "project",
      "base": "project",
      "relative": ".my-agent/skills",
      "discovery": "ancestors-to-repo",
      "role": "native",
      "read": true,
      "write": true,
      "priority": 200
    }
  ]
}
```

relative 使用 `/` 作为配置分隔符，由后端按平台解析；拒绝绝对路径、盘符、UNC、`..`、空相对根及未识别 base。APPDATA/XDG 等 base 在当前平台不存在时禁用对应根，不猜测替代位置。若实际安装位于上述基目录以外，可通过系统目录选择器登记本机 rootId 的绝对路径覆盖；覆盖不来自外部档案，仍需用户确认，不自动扩大到整个磁盘/HOME。

本地覆盖分别保存到 root 授权记录，包含 AgentId/rootId/platform/明确选定目录。项目范围只覆盖当前明确选择的项目；不会把某项目覆盖变成所有项目通用写入路径。整个规则语义由解析器统一执行，不为某个新 ID 加特殊代码。

### 4.2 档案生命周期与扩展限制

Agents 页提供“添加 Agent”，任务弹窗可填写表单或导入 JSON；先校验，再展示当前平台全部 variants 的读取根、写入根、探针文件、共享影响，最后明确启用。其他平台只展示声明，不扫描本机无法映射的路径；授权记录绑定本机平台和这些实际根/项目模板。新用户档案默认未启用；放入 agents 目录不立即扫描新位置或触发写入。启用只授权指定根内的扫描与后续手动预览，不直接应用 Skill。

授权覆盖当时已展示的当前平台全部 variants，而不仅是默认 variant；项目模板仍要求每次使用明确选定项目目录，本机绝对路径覆盖只对已确认的项目/范围生效。切换到已授权 variant 不重问相同目录授权，但必须清选择、作废预览并重读状态；涉及新根或新规则时重新确认整个变更。迁移到另一平台/本机根变化时，不直接复用另一台机器的路径授权。

当前 variant 选择保存在软件设置的 agentVariants 映射，通过现有 save_settings 的基线检查保存；没有选择时使用档案 defaultVariant。保存只接受已登记 AgentId 和该档案已声明的 variant，不接受任意根路径。授权定义与当前选择分开，切换已批准 variant 不改写档案或扩大授权。

用户档案的结构/规则或本机根覆盖变化时，重新校验与授权，失效涉及的旧预览并清理本次选择。非法 JSON、未知 schema/格式/字段、重复 ID、矛盾写根、跨目录逃逸均阻止该档案启用，不破坏其他有效档案。原文件不被覆盖成默认配置。

保存档案先备份，再原子写入；授权记录保存当时的规范化完整档案及覆盖。外部编辑后与已授权内容不一致时暂停该档案的扫描/应用，提示重新确认，不静默沿用旧授权。停用不删除文件，不卸载任何 Skill，也不关闭真实 Agent 的兼容读取。

共享影响从全部已知读取规则和实际路径计算，不硬编码“Codex → Antigravity”，且合并所有已知 variants 的潜在读取根，不只计算任务当前 variant。每个影响条目保留 agentId/variantId/rootId/读取证据，UI 可按 Agent 合并但能展开具体 variant。Hub 中停用的已知 Agent 仍可能读取共享目录；对已授权的既有档案保留静态潜在影响提示，但不扫描新未授权路径。提示明确不是对所有未登记 Agent 的完备可见性证明。

UI 目标列表、计数、状态、CLI Agent 列表及 DTO 校验都由注册表生成。用户自定义档案显示“用户自定义，加载兼容性未核实”；明确目录可授权分发，不以“复制成功”代替实际 Agent 加载验证。

未知包格式、数据库存储、归档/插件安装协议、目录规则无法表达的发现机制，需要新增引擎适配。首版不提供任意代码插件系统；不承诺所有未来 Agent 都能零开发接入。

## 5. Skill 包、标识与库的存储

### 5.1 完整包与元数据

一个 Skill 包以包含 `SKILL.md` 的目录为边界，包含边界内全部普通文件、子目录和空目录。正文、frontmatter、资源和二进制文件保持原始字节与相对路径；只解析展示字段，不重排或删掉未知元数据。

首版可以编辑 SKILL.md 原文并校验，其他文件只读预览或在系统目录中自行编辑。后端只打开目录，不执行其中的脚本。

最小可移植条件：SKILL.md 为 UTF-8，可有 UTF-8 BOM，存在 YAML frontmatter，`name` 和 `description` 为非空字符串；name 长度 1–64，符合 `[a-z0-9]+(-[a-z0-9]+)*`。不合条件的来源仍可查看，但阻止直接分发并说明具体字段；不自动修正文案或名称。

内部包 ID 与 name 分开：`id` 在库内唯一，`name` 决定目标目录。允许保留同名不同内容的多个包作为独立库记录；同一次分发中，一个物理目标的同名包只能选一个版本。显示标题可自定义，不改变 frontmatter name。

首版不提供会自动改写 frontmatter 的“跨 Agent 改名转换”。同名冲突用跳过/明确替换处理；用户需要另一个 name 时自行编辑原文并重新校验。

### 5.2 目录布局

```text
~/.skill-hub/
  settings.json
  agents/<custom-agent-id>.json     # 运行时用户档案，默认待启用
  agent-approvals.json              # 已确认档案内容、variant 与本机根覆盖
  library/
    catalog.json
    packages/<package-id>/revisions/<revision-id>/...
                                   # 每个版本都是完整包，SKILL.md 保持原文
  backups/<operation-id>/
    manifest.json
    original/<item-id>/...         # 变更前内容，可跨卷复制保存
    planned/<item-id>/...          # 确认后的预期完整内容，用于中断恢复比对
    previous-library/...          # 导入、编辑等库操作的恢复资料
  audit/events.jsonl
```

库可切换到另一个本地绝对目录；根目录包含 catalog.json 和 packages。首次显式保存/导入才创建；不存在与损坏分别处理。损坏库禁止当空库覆盖。库 schema 固定 `version: 1`，更新通过明确迁移，不静默降级。

catalog 至少记录：`id/current_revision/name/display_name/description/source_kind/source_agent/source_path/imported_at`。来源路径仅存在本机数据，不进入诊断上传或公开样例。适配性从包原文和目标能力计算，不写死为“通用兼容”。

包原文是 name/description 的语义来源，catalog 的对应字段只是当前版本的缓存。读取时必须校验引用完整性及字段与所引用 SKILL.md 一致；不一致则阻止分发，提供明确的“预览重新导入/修复索引”入口，不悄悄选择其中一个值决定写入路径。

导入和编辑都先生成新的完整 revision，备份旧索引，将新 revision 持久化并复核后，才用带基线校验的原子文件替换提交 catalog；catalog 提交是库可见性边界。工具不原地覆盖已引用 revision。中断留下的未引用 revision 不进入列表，可保留现场并在恢复页处理，不自动删除；catalog 仍指向旧 revision 时旧内容继续可读。

导入即复制到自持库；源目录之后变化不自动更新库。用户再次导入同一来源时比较内容，明确选择更新或保留独立记录。库内包与目标分别扫描比较，不用时间戳替代内容比较。

### 5.3 资源与安全限制

首版上限：单文件 4 MiB，单包 16 MiB，最多 1,024 个文件/目录条目，子目录相对深度不超过 16，单次预览合计原始内容不超过 64 MiB。超限明确阻止对应操作，不截断后继续复制；这些值为初始保护阈值，后续用实际包评估调整。

拒绝绝对包内路径、`..`、路径逃逸、目标平台不合法文件名、Windows 保留名及大小写折叠冲突；不按 Git ignore 静默删资源。

源包入口可以是符号链接或 junction：只读解析并展示原路径与实际路径，去重后读取。包内部链接、设备文件和其他特殊条目首版阻止导入/应用，不隐式追随外部资源。目标包为链接或 reparse point 时阻止写入；已登记根目录可解析到实际根，但实际路径必须进入预览并被锁定复核。

禁止向库、备份、事务目录或其他目标包的祖先/后代写入；路径重叠校验在规范化物理路径上进行。文件复制保留常规权限/Unix 可执行位，不保留所有者或提升权限，不复制特殊权限位。Windows 不宣称自动加固 ACL。

## 6. 导入、编辑与冲突

导入流程：选 Agent/本地目录 → 只读扫描 → 选择包 → 处理库中同名冲突 → 预览库变化 → 明确保存。来源和库都进入快照，确认前变化则拒绝旧预览。源文件保持不变，保存库不分发。

库内同名包：内容完全相同则默认跳过；不同内容默认跳过，可明确替换某个库记录或另存为新记录。替换先备份旧记录与包；一个导入任务内冲突选择互相矛盾时阻止确认。

编辑只改库内选定包的 SKILL.md：保存前校验、展示影响的元数据字段、先备份，并核对库与包是否被外部编辑。保存后仅标记目标“存在差异”；编辑失败保留输入，取消/切页提示未保存更改。

若编辑 name，PackageId 保持不变，但新名字解析为新的目标目录。成功分发记录必须保存 `package_id/revision_id/name_at_write/scope/project/physical_path/operation_id`；详情分开列出“当前名称目标”与“历史名称仍存在的位置”。旧名称目录不删除、不迁移、不自动跟随，继续存在时醒目提示可能同时加载；新名称应用成功不能清掉旧位置提示。恢复始终绑定那次操作的原实际路径与备份，不通过当前 name 重新计算路径。

依赖识别只作静态提示：Claude 专属 frontmatter/动态命令、Codex agents/openai.yaml、工具名称、外部文件引用、系统命令和插件依赖均可能不跨 Agent 通用。原文不重写；“有兼容性提示”不等于可执行保证。

敏感信息：后端在原文离开进程前脱敏；高置信真实密钥/认证内容阻止导入、编辑保存和分发，不把匹配原文写入错误、日志或报告。检测不是无凭据证明；普通描述不冒充安全审计。已有配置与备份都按私密本地数据处理，不上传。

## 7. 范围、选择与状态模型

库独立于范围；切换用户/项目不会隐藏或删除库记录。状态按 `package_id + agent_profile + scope + canonical_project` 计算，不能复用另一个项目的状态。

安装状态：已检测到 Agent / 仅目录存在 / 未发现 Agent。包状态：目录不存在 / 内容一致 / 内容不同 / 同名冲突 / 不支持 / 读取失败。无效包与空目录不等同于未配置。

目标选择保存精确配对 `package_id → agent_id[]`；不能把选中行与选中列求笛卡尔积。agent_id 从有效注册表校验，不使用四项固定允许列表。选择属于当前任务，不恢复上次未完成的分发；切库、切范围、切项目或变更相关档案/variant/根覆盖时清空选择和预览。切选中 Skill 保留其他 Skill 的本次配对。

“待更新”只表示库与目标内容不同；不据此认定库更新或目标更新。详情同时展示来源和差异，防止覆盖目标中的手工修改。

## 8. 分发计划、共享目录与确认协议

### 8.1 预览必须包含

- 所选 Skill、目标 Agent、范围/项目、各物理目录及受影响的文件数。
- 按物理目录分组的新增/更新/不变/冲突/阻止结果。
- 文本按文件展示脱敏 diff；二进制只展示路径、大小与变化类型，不假造文本 diff。
- 目标已有额外文件时列出“旧包归档后移出加载目录”的影响；不得偷偷混合两版目录。
- 主动选择的 Agent 与“可能额外读取”的 Agent 分开列出。
- 每个冲突的跳过或明确替换决定；默认跳过，跳过项不写入。
- 备份位置、共享影响提示、目录可能短暂不可用的说明和手动确认按钮。

同一物理目录只生成一次计划、一次备份、一次变更。不同 Agent 对该路径要求不同包内容时阻止该组；用户调整选择后重预览。只勾选 Codex 仍可能影响 Antigravity/Grok，应在确认页列出；不自动替用户关闭兼容读取。

确认语义是“同意这些实际目录与其已列出的影响”，不能承诺未勾选的 Agent 一定看不到共享目录。

### 8.2 一次性预览

后端生成 preview_id，绑定库和包的字节快照、源与目标树（含原本不存在的目录）、实际路径、完整已授权档案与注册表版本、variant/根覆盖/条件探针结果、选择、冲突策略、范围/项目和设置。预览阶段的原始快照只在后端内存，绝不通过 IPC 或日志发送；明确确认后，实际变更所需 original/planned 树才作为私密本地备份持久化，不生成包含原文的公开日志或草稿。

有效期 600 秒；一个窗口只保留一个当前预览。新预览、取消、过期、任何确认尝试均消费旧预览。更改选择/策略/编辑/库/范围/项目后必须重预览。确认接口只收 preview_id，不接收前端可编辑的文件计划或目标路径。

确认前重读完整树和上下文，任何改变、增加、消失或链接重定向均整批停止，尚未开始写入。文件时间戳不能代替逐字节比较。全量预检后仍在每组写入前复核；期间发生外部修改时，该组拒绝并保留其他已完成组的结果。

### 8.3 更新和无变化

同内容不备份、不写文件。目录级替换是完整包替换；旧版多余文件跟随旧目录被归档，不永久删除。此策略避免新版 SKILL.md 配到旧脚本，也明确告知目标手工增补文件会移出加载目录。用户必须在预览明确选择替换才可执行。

## 9. 目录事务、恢复与并发边界

不能把跨多个 Agent 的分发描述为全局原子事务。写入单位是一个物理 Skill 目录；独立组可部分成功，UI 必须逐组报告。

单次任务流程：

1. 获得 Skill Hub 自身的协作锁，锁文件以 create_new 创建；不修改 Agent 的全局权限或服务。
2. 验证全部快照、路径和容量。所有待写组的 original/planned 完整树、清单和事务日志成功持久化后，才允许任何目标改动。
3. 每组在与目标同卷、且位于所有 Skill 扫描根之外的事务目录复制 staging，核对全部内容与可执行位。
4. 对该组再次核对目标。已有目标先 rename 到同卷事务归档目录，再将 staging rename 到目标位置；目标原本不存在则直接激活。
5. 写后重读完整包并比较。仅当当前目标仍等于本事务 planned 树、旧归档仍等于 original 树，且实际路径身份未变时，才可在内部失败后自动归档本次产物并恢复旧目录。若检测到外部编辑、无法读全树或无法证明当前内容归属，保留当前目标和所有备份，进入 recovery_required，等待用户预览恢复，不撤下外部内容。激活 rename 失败且目标仍不存在时，可以在核实旧归档和路径后恢复旧目录。新创建包的回退同样要求归属核对，再归档成“不存在”；不自动永久删除。
6. 记录每组结果和保留下来的路径；协作锁通过 rename 移到本次事务记录中释放，不删除文件。

建议事务位置是目标根的兄弟目录，例如 `~/.agents/.skill-hub-transactions/<id>/`，而不是 `.agents/skills/` 内部。执行前检查其不在任何已知/自定义读取根内；无法找到可写、同卷且不被扫描的位置时阻止该组。跨卷中央备份用复制，激活归档只在同卷 rename。

两次 rename 之间可能有目标短暂不存在的窗口。GUI 提示避免在 Agent 正在使用该 Skill 时更新；不自动停止 Agent。不得宣称 Linux/macOS/Windows 的整个目录替换是单步原子操作。

journal 持久状态至少有 `prepared / archived / activated / verified / rolled_back / recovery_required`，每次 rename 前后记录预期和实际路径，以及 original/planned 树的受控备份引用。进程重启时只读列出未完成任务，不自动覆盖目标；用户进入恢复预览，核对现状后继续恢复。备份/journal schema、引用路径或文件不完整时禁止猜测恢复。

锁只协调 Skill Hub 进程，不能阻止 Agent/编辑器写文件。持续外部写入或恶意路径竞态不保证完全消除；复核不通过时停止，不以陈旧内容覆盖。遗留锁不自动删除或接管，界面展示恢复入口和影响。

备份、旧 revision 和事务现场不自动清理。界面说明保留策略和占用；空间不足时阻止新操作或报告失败，由用户自行管理保留资料，不悄悄删除旧数据腾空间。

回执：`written / unchanged / skipped / blocked / failed_restored / recovery_required / audit_failed`。审计失败不抹掉真实写入结果。恢复失败明确给出可用备份/归档路径，不笼统显示“已回滚”。

恢复同样必须预览、备份当前内容、一次性确认和状态复核。原先不存在的目录通过归档当前目录恢复为不存在；原先存在的目录用备份 staging 激活。用户后续编辑不会在无提示下丢失。库和设置的恢复也走对应的同一机制。

## 10. 界面信息架构与视觉规范

视觉基准：[已确认的交互原型](../../prototype/skill-hub-design.html)。该原型仅演示交互，不能当作文件服务或事务实现。

### 10.1 基础视觉

- 浅色基础：雾蓝背景 `#F3F6FC`、白色工作面 `#FFFFFF`、侧栏 `#EDF1F9`、深蓝文字 `#253453`、次级文字 `#64718A`、分隔线 `#E0E6F1`。
- 主要操作：钴蓝 `#355AF4`，浅蓝选中 `#EAF0FF`；绿色 `#25785F` 表示内容一致/写入成功，琥珀 `#956116` 表示差异或需要处理。
- 使用明确的颜色语义、文字和图标，不单靠颜色表达状态。暗色使用原型中的对应主题色，不仅反转背景。
- 字体优先本机 Segoe UI Variable / Microsoft YaHei UI / PingFang SC；路径和代码使用 Cascadia Code/系统等宽字体；不依赖在线字体。
- 页标题约 29px，详情约 19–20px，正文 13–14px，次要说明不小于 11px；用清晰层级和留白建立重点。
- 侧栏 166px，内容边距约 27px，详情宽 260px；面板圆角约 11px，按钮 7px。桌面以列表和按需详情组织，不堆砌统计卡片或装饰性渐变。
- 光标动作响应克制，尊重减少动态效果偏好；正文对比目标不低于 4.5:1。

### 10.2 页面与任务

| 页面/区域 | 职责 |
|---|---|
| 我的 Skills | 全部/待更新/未分发筛选、搜索、名称描述、分发状态、选中项详情 |
| Skill 详情 | 包名与来源、文件清单、兼容提示、当前目标状态、本次 Agent 勾选 |
| 底部选择栏 | 精确配对数量、清空、预览变更；未选中时禁用预览 |
| Agents | 注册表动态列表、内置/用户档案标记、安装与根目录状态、只读查看、Skill 导入、添加/编辑/启停档案 |
| 变更历史 | 导入/编辑/分发/恢复记录、逐组结果、可用备份和恢复入口 |
| 软件设置 | 库目录、备份目录、默认范围、最近项目；“手动应用”固定说明，不提供自动同步开关 |
| 导入弹窗 | Agent/本地来源选择、扫描结果、勾选、库冲突和确认 |
| 编辑弹窗 | SKILL.md 原文、字段校验、文件列表、保存到库 |
| 预览弹窗 | 目录与文件 diff、受影响 Agent、冲突策略、明确确认 |
| 恢复弹窗 | 当前内容与备份差异、额外备份位置、确认恢复 |
| Agent 档案弹窗 | 通用表单/JSON、三平台根规则、variant、本机路径覆盖、校验与目录授权；不增加主导航页 |

中文用户文案统一：导入到库、保存到库、预览变更、确认应用、重新预览、查看备份、预览恢复。成功提示写“文件已应用，请在目标 Agent 核对加载”，不写“Skill 已运行/已生效”。

空库提供“从 Agent 导入”和“从本地目录导入”；首次启动不创建库。损坏库提供检查位置/重试，不出现可以覆盖的空白编辑器。

需要覆盖的状态：加载、空、扫描失败、权限不足、无效项目、无效 Skill、超限、同名冲突、共享目录、预览过期、写入部分失败、恢复失败、审计失败、未保存编辑、遗留事务。

切 Agent/范围后的异步旧响应不能覆盖当前页面。写入期间锁定上下文；原生 dialog 管理焦点和 Escape，取消只使预览失效，不能中断已激活目录的事务。

### 10.3 响应式与可访问性

以 1024px 桌面宽度为基准：850px 以下侧栏折叠为带可访问名称的图标；650px 以下详情移到列表下方；390px 以下按钮和工具栏按需换行。支持 320px 窗口，不将桌面整体缩小。

键盘可完成导航、选择、搜索、导入、预览和确认；焦点可见，弹窗不落到背景；结果用 aria-live，错误用明确文本。触屏有效点击区域约 44px。长名称、中文、长路径、200% 缩放和空搜索不溢出主布局。

## 11. 技术架构与接口边界

```text
React UI → typed IPC → Tauri command adapter → skill-app
CLI argument adapter ────────────────────────→ skill-app
skill-app → skill-core（纯解析/比较/计划）+ skill-io（文件/事务）
```

- `skill-core` 无文件 IO、不依赖 tauri/clap，处理模型、frontmatter、可移植性校验、内容比较、计划与 diff。
- `skill-io` 负责安全扫描、完整快照、备份、目录事务、锁、journal、恢复和审计。
- `skill-app` 负责声明式 Agent 注册表、环境/根目录解析、档案与根授权、库生命周期、逐对选择、预览会话与所有用例编排。
- Tauri 只做 DTO 转换、窗口状态与系统目录选择；前端不能提交可写路径或任意 IO 计划。
- CLI 调用同一应用服务。GUI 使用一次性会话；CLI apply 在同一进程即时生成并确认计划，不能用旧 diff 文件绕过复核。

关键命令：`app_info / load_settings / save_settings / choose_directory / list_agent_profiles / preview_agent_profile / commit_agent_profile / set_agent_profile_enabled / detect_agents / list_agent_skills / load_library / read_package / scan_import / preview_import / commit_import / save_skill_document / preview_distribution / commit_distribution / cancel_preview / list_history / preview_restore / commit_restore / list_recoveries`。档案保存/启用通过安全应用服务，不能绕过根授权直接注册前端传入的可写路径。

CLI 范围：`agents list|validate|preview|save|enable|disable|select-variant / doctor / list / import / diff / apply / backups / restore`；档案预览、保存和启用与 GUI 使用同一后端规则，select-variant 经 save_settings 保存有效选择。`apply` 无 `--yes` 仅输出脱敏计划并退出 2；`--dry-run` 不写用户配置、备份或审计；只有明确 `--yes` 执行且仍做快照/路径复核。import/restore/档案保存、启停与 variant 设置的写操作同样要求确认。

DTO 字段固定 camelCase；Rust snake_case。首版手写 TS 类型并用契约 fixture 验证，避免为自动生成类型额外引入未经批准的依赖。IPC 错误为安全 code + 中文消息，不含原始 YAML/路径内容中的凭据。

## 12. 依赖、版本和构建约束

参考项目已使用：serde、serde_json、serde_yaml、toml_edit、thiserror、clap、Tauri 2、React 19、TypeScript、Vite、Node 自带测试运行器。这里只提出候选清单；skillcfg 目前没有任何依赖，不能把“参考项目已有”当作本项目的安装授权。

实施 Task 1 须给出确切版本、来源、用途和锁文件方案，用户确认后才可新增依赖清单或安装。优先复用参考项目已锁定版本，不顺带升级；缺失测试工具同样不自动安装。

版本基线建议：Rust 核心/CLI 至少 1.85；桌面至少 1.90（参考项目约束）；Node 版本从 mcpconfig CI/锁文件和 Vite 实际要求核实后固定。锁文件必须入库，离线/locked 检查不得触发自动补装。

内容一致性直接比较有序路径清单与原始字节，不使用不稳定 std 哈希作为持久完整性凭据。首版无需为 SHA256 再新增依赖；如果发布产物需要校验摘要，由已授权 CI 的系统工具生成。

所有 Rust 编译/检查、Clippy、cargo test、前端 TypeScript/Vite 构建、浏览器测试与 Tauri 打包都在 GitHub Actions 的 GitHub-hosted runners 上执行；不使用本机编译或自托管 runner 变相编译。本地只做阅读、文档/配置静态校验、差异和敏感信息检查，不安装构建依赖、不运行会编译的测试。

CI 从第一阶段建立，不等最后才接入。每阶段验证当前已实现的组件；未到桌面阶段不把缺少桌面任务标作“三平台桌面通过”。依赖安装清单与 CI 环境安装方案先列明用途/版本并按用户红线确认，之后仅在批准的 GitHub 临时 runner 环境恢复或安装，不自动安装到用户电脑。

首版平台/架构与产物目标：

| 平台 | 桌面端 | CLI | 主要验证 |
|---|---|---|---|
| Windows x64 | 安装程序、免安装 ZIP | ZIP | 原生目录选择、reparse point、保留名/大小写、长路径、WebView2 |
| macOS arm64 与 x64 | DMG/App bundle，分架构产物 | tar.gz，分架构 | 原生目录、执行位、rename、系统权限与未签名安装边界 |
| Linux x64 | AppImage、Debian 包 | tar.gz | 原生桌面启动、GTK/WebKit 运行依赖、权限/执行位、符号链接与 rename |

Windows/Linux arm64 和更多 Linux 包格式为后续扩展，不因支持三平台而默认承诺全部 CPU/发行版。最低 OS/发行版、WebView/系统库要求在 Task 1 核对官方约束后写入平台矩阵；不能把一个 Ubuntu runner 通过说成所有 Linux 发行版兼容。macOS 两架构既要构建，也要有对应架构运行检查；只有交叉编译不算运行验证。

CI 矩阵覆盖 Linux/macOS/Windows 核心、IO、应用服务、CLI 与桌面编译；UI 浏览器回归覆盖通用交互，原生桌面能力另有三平台人工/可自动化的 smoke 检查。Linux 桌面必需系统依赖只在批准的 CI 环境配置，不在用户本机执行安装命令。所有自动化使用 fixture HOME/APPDATA/项目，不能读取或修改操作人员真实配置。

平台实施参考：[Tauri 分发](https://v2.tauri.app/distribute/)、[Tauri 前置条件](https://v2.tauri.app/start/prerequisites/)、[GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)。runner 镜像和工具链按阶段记录确切版本；不凭 latest 标签推断用户最低系统版本。

发布构建与安装后人工验证分开记录。不因 CI 通过就宣称目标 Agent 已加载；不自动打标签、发布或部署。

### 12.1 阶段提交与 GitHub 验证流程

用户已授权后续开发阶段提交到仓库，并要求云端编译。本方案将其落实为每阶段至少一份可审阅 commit，push 到当前授权的开发分支以运行 GitHub Actions；必要的红灯测试/验证快照与修复也可在阶段内 commit/push，不等所有阶段结束才提交。不自动合并主干或创建 Release。

阶段顺序：同步开发分支 → 写测试/实现 → 本地差异和敏感检查 → 精确暂存 → commit/push → GitHub 验证 → 修复并再次提交 → 独立评审 → 对同一最终提交确认应有 CI 全绿 → 报告阶段 commit/分支/工作流链接及剩余限制。代码或配置再次改变后必须验证新提交，不能沿用旧 SHA 的绿灯。

`docs/PROGRESS.md` 记录阶段状态、验证范围和证据入口；提交号与 CI run 信息可在阶段报告中给出，不要求文件包含其自身尚未生成的 commit SHA。CI 失败的阶段不能标为完成；网络或 CI 不可用时保留已提交工作，报告等待验证，不转到本机编译。

本轮只更新提案文档，不是产品开发阶段完成；仓库仍在 master 时不进行归档或提交。阶段提交授权不取消主干红线、依赖确认、精确暂存、凭据扫描和禁止危险 Git 操作等要求。

## 13. 验收标准

| 编号 | 可验收的结果 |
|---|---|
| A1 | 四个 Agent 的有效、缺失、损坏、多根来源均可分辨；版本/路径有证据 |
| A2 | 导入完整包后，源目录未变；未知 frontmatter、中文、二进制和执行位按规则保留 |
| A3 | 编辑/导入只改库，四个 Agent 的目录逐字节保持不变 |
| A4 | 精确配对正确；两条 Skill 分别选不同 Agent 不会扩成四组 |
| A5 | 预览路径、文本/二进制变化、冲突与共享影响正确；不变项无写入 |
| A6 | 选择/内容/设置/路径/目标树变化或预览过期后，确认被拒绝；无开始写入 |
| A7 | 共享目录只改一次；同路径不同包内容阻止；外部修改不被静默覆盖 |
| A8 | 备份失败时零目标变更；各事务中断点可识别并经明确恢复处理 |
| A9 | 更新、失败恢复、手动恢复均无永久删除；原本不存在的目标恢复后不被加载；外部编辑发生时不自动撤下当前内容 |
| A10 | 路径逃逸、内部链接、目标链接、大小写冲突、超限与敏感值被安全处理 |
| A11 | 四页、任务弹窗、键盘、窄窗、长内容和异步竞态通过交互验证 |
| A12 | 所有真实文件操作经应用服务；默认无写入；CLI dry-run 无副作用 |
| A13 | 错误、审计、IPC、公开示例和提交检查均无真实凭据输出 |
| A14 | 安装包人工验证完成后，另行记录四个 Agent 的实际发现/加载结果 |
| A15 | 库内容/索引共同提交可恢复；中断不出现半包，元数据不一致禁止分发 |
| A16 | 更名后旧路径持续提示；历史恢复使用原实际路径，不按新 name 重新定位 |
| A17 | 在既有程序中仅添加并授权第五个标准 Agent JSON 档案，即可扫描、选择、预览、应用和恢复；不修改 Rust/TS、不重新编译 |
| A18 | 非法/未授权档案不扫描不写入；档案、variant、根覆盖或条件变化使相关旧预览失效；共享提示不硬编码四个 ID |
| A19 | Linux/macOS/Windows 均有桌面和 CLI 构建产物、相应平台验证证据；所有编译来自 GitHub Actions |
| A20 | 四个阶段分别有远端开发分支 commit 和匹配最终 SHA 的 GitHub CI/评审记录；失败或未验证不标完成 |

## 14. 本轮交付与后续审阅

配套：[实施计划](../plans/2026-11-01-skill-hub.md)。原型已获用户视觉认可；本次将用户确认的可配置 Agent、阶段提交、GitHub 编译和三平台要求落实到 D5/D6/D10，其他细化及限制值一起供审阅。

用户已明确授权创建/使用 dev/skill-hub、归档文档并实施第一阶段。2026-11-01 已核对当前开发分支并快进同步 origin/master，文档现归档在仓库 docs。当前仅推进 Tasks 1–3；按 §12.1 进行验证提交和阶段提交，所有编译与测试只在 GitHub Actions。

建议开发分支命令：`git switch -c dev/skill-hub-design`。执行归档前重新确认分支、工作区和 upstream；尚未配置 upstream 的新分支不能猜测 pull 来源，先核对 origin/master 的更新，再做可快进同步。

设计审阅通过后再实施。实施按依赖顺序分阶段，每个阶段有测试证据和独立评审；如遇需要新增依赖、删除文件、覆盖用户配置或高风险操作，继续按用户红线单独处理。
