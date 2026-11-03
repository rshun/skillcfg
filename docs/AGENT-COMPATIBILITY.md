# 内置 Agent 档案证据

核对日期：2026-11-01。配置来源与实际运行验证分开；四个内置定义不代表任何 Skill 的工具依赖都可移植。

| Agent | 主要目录 | 官方证据 |
|---|---|---|
| Codex | 用户/项目 .agents/skills；.codex/skills 为兼容只读候选 | https://learn.chatgpt.com/docs/build-skills |
| Claude Code | 用户/项目 .claude/skills | https://code.claude.com/docs/en/skills |
| Antigravity | 用户 .gemini/config/skills，项目 .agents/skills；App/CLI/IDE variants | https://antigravity.google/docs/skills |
| Grok Build | 用户/项目 .grok/skills，以及 .agents/.claude/.cursor 兼容读取 | https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/08-skills.md |

Antigravity CLI 独有和旧路径仅作相应 variant 的兼容读取。Grok 的 TOML 布尔探针只给配置证据；环境变量或其他配置可能覆盖，不据此宣称互相隔离。

第一阶段自动化只使用上述结构的合成目录，不执行 Agent，不读取用户真实配置，不宣称已完成四个 Agent 的安装后加载验证。
