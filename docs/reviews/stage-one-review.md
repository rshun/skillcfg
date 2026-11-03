# 第一阶段独立代码审阅

审阅范围：`b0e19ce..132a89eda5610a703156f87b17cfd89478b5d30f`，对应实施计划 Tasks 1–3。独立审阅仅静态读取，没有本机编译、测试或安装依赖。

结论：初审发现 3 项 Important、1 项 Minor，未确认 Critical。初审不能作为阶段完成证据；重要问题须经 GitHub 回归测试修复并取得最终 SHA 的完整绿灯。

| 等级 | 问题与复现 | 处理状态 |
|---|---|---|
| Important | 单行/嵌套 JSON 的 token/password/Authorization 字段可能漏过按行检测，并进入公开差异 DTO | 已修复：结构化与内联字段检查；红灯 run 41287675767 → 完整绿灯 run 41288863551 |
| Important | 未授权 `.json` 档案或配置探针指向 FIFO 时，打开文件可能无限阻塞 | 已修复：普通文件检查、Unix 非阻塞/不跟随入口链接打开与句柄类型复核；同一红/绿 run 验证，Windows 不冒充具有 Unix FIFO 测试 |
| Important | 64 MiB 预览预算在克隆全部源树后才拒绝，超限请求可能造成内存压力 | 已修复：预算完整预检后再复制；独立分配计数用例在同一红/绿 run 验证 |
| Minor | AgentInventory 仅保留源路径与物理键，丢失入口链接的实际路径和链接标记 | 暂缓到展示契约接入时补齐；底层 PackageSnapshot 已保留该信息，当前阶段不执行写入 |

审阅确认了原始字节/执行位比较、待授权档案禁止扫描、外部编辑使重新加载后的授权失效、数据驱动 variant/写根校验、精确配对与共享目标冲突检查，以及原始包树不实现 Debug/Serialize。

未判断后续事务、备份、恢复、CLI、桌面功能；未验证真实 Agent 当前版本是否加载对应目录。持续运行时的重新加载和确认时撤销旧授权仍需后续应用服务验证。手工伪造内部 TargetSnapshot 不作为外部漏洞结论。

## 修复验证

`4c7d27c` / [GitHub run 41287977699](https://github.com/rshun/skillcfg/actions/runs/41287977699)：四平台行为测试通过，三个 Important 的回归用例由红转绿；该 run 的 Clippy 和格式仍未通过，因此不是阶段完整完成证据。后续继续完成质量检查，不重复派发第二轮独立审阅。

`0a030ac` / [GitHub run 41288863551](https://github.com/rshun/skillcfg/actions/runs/41288863551)：四平台测试/Clippy、格式、锁文件检查全部通过。当前没有未处理的 Important/Critical；上述 Minor 保留为后续展示契约工作，不影响当前只读组件验收。完整应用预览授权与真实 Agent 加载兼容性仍不属于该次验证。
