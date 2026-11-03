# CLI 手动工作流

CLI 与后续桌面端共用应用服务。使用 GitHub Actions 的 skill-hub 平台制品；本机不需要安装 Rust 或构建依赖。没有自动同步。

首次先在临时目录用 `--home` 隔离 HOME、APPDATA、XDG 和可执行路径检测。所有示例中的路径和 ID 都是占位符；请替换为自己的绝对路径/列表中的 ID。PowerShell 使用 `skill-hub.exe`，Linux/macOS 使用 `./skill-hub`。

```text
skill-hub --help
skill-hub --home HOME_PATH agents list
skill-hub --home HOME_PATH doctor
skill-hub --home HOME_PATH import --source SOURCE_PATH --dry-run
skill-hub --home HOME_PATH import --source SOURCE_PATH --new --yes
skill-hub --home HOME_PATH list
skill-hub --home HOME_PATH diff --pair PACKAGE_ID=codex
skill-hub --home HOME_PATH apply --pair PACKAGE_ID=codex --yes
skill-hub --home HOME_PATH backups
skill-hub --home HOME_PATH restore OPERATION_ID --dry-run
skill-hub --home HOME_PATH restore OPERATION_ID --yes
```

`--pair` 可重复，保持精确 Skill/Agent 配对，不扩展笛卡尔积。项目范围需同时提供 `--scope project --project PROJECT_PATH`。可用 `--library LIBRARY_PATH` 显式选库。

已有不同内容的目标默认跳过；`apply --replace` 才明确替换，预览列出目标额外旧文件。导入同名包默认跳过；`--new` 新建独立包，`--replace` 替换唯一同名记录，或 `--replace-id PACKAGE_ID` 指定原记录。

写入命令必须 `--yes`，否则只展示计划并退出 2；`--dry-run` 优先于 `--yes`，成功预览退出 0，不创建备份或审计。`apply --yes` 和 `restore --yes` 均现场建立、校验并消费预览，无法拿另一个进程的预览 ID 直接执行。

## 可配置 Agent

档案格式见 resources/agents 与 resources/schema。普通新增 Agent 只添加 skill-md-v1 JSON 数据，不改 CLI 或前端枚举。

```text
skill-hub --home HOME_PATH agents validate --file PROFILE_PATH
skill-hub --home HOME_PATH agents preview --file PROFILE_PATH
skill-hub --home HOME_PATH agents save --file PROFILE_PATH --enable --yes
skill-hub --home HOME_PATH agents disable custom.demo --yes
skill-hub --home HOME_PATH agents enable custom.demo --yes
skill-hub --home HOME_PATH agents select-variant AGENT_ID VARIANT --yes
```

`--root ROOT_ID=ABSOLUTE_PATH` 设置本机覆盖；内置档案用 `--builtin AGENT_ID`。授权展示当前平台所有 variant 根及探针路径；未授权探针不读内容。外部编辑档案后旧授权失效，需重新预览保存。档案停用不删除文件、不移除已分发包。

JSON 结果区分实际回执与预览。退出码：0 成功；1 错误或部分失败；2 需要明确确认。读取失败不等于空库或未安装。遇到 RecoveryRequired，保留现场，使用 backups 查看，再显式 restore 预览；不要删除锁或临时目录。

Linux/macOS/Windows 原生 runner 会以同一已构建可执行文件，运行第五个 Agent 的添加、授权、导入、diff、apply 与 restore；测试比较二进制字节不变，证明不需要重编译。
