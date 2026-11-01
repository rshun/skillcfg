# Agent 档案 v1

内置定义来自 profiles/builtin/*.json，构建脚本自动打包目录内的 JSON，运行时统一校验；没有四项 AgentId 枚举。普通新增 Agent 将 custom.<slug> 档案放入 ~/.skill-hub/agents，由后续档案管理流程预览授权后启用，不需要修改 Rust/TypeScript 或重编译。

第一阶段提供读取与校验 API，不提供授权记录的写入界面或自动安装命令。测试使用独立临时目录内的合成授权记录，绝不扫描本机真实配置。

- schemaVersion=1、packageFormat=skill-md-v1；未知字段/规则拒绝，不运行表达式或命令。
- roots 按平台、范围和 variant 选择；appliesToVariants 省略代表所有 variants。
- 每个平台/范围/variant 最多一个静态写根；父项目根只读。
- readEvidence 仅提供读取证据，false/unknown 不切换写根，也不隐去共享影响。
- 用户档案默认 Pending，必须与已授权原文、当前平台及基目录完全匹配后才可扫描；外部修改后重新变为 Pending。
- root 入口链接可只读解析，包内链接阻止；不能将源链接视作可写目标。
- 第一阶段不实际写入 Agent。write 标记表示后续可候选的目标规则，不是现有写入功能。

JSON Schema 是形状说明；跨字段约束、命名空间、平台路径合法性、写根唯一性与实际根授权仍由统一 Rust 校验器执行。
