# 第二阶段独立审阅

范围：9e02750..32015b181fc96835029c2d55cca9598cab8223e2，Tasks 4–7。按执行技能的单次整体验收审阅，由新的只读上下文完成；未修改源码或执行本地编译测试。

## 发现与修复

无已确认 Critical。六类 Important 全部接受，进入同一轮修复。

| 问题 | 回归用例 | 验证 |
|---|---|---|
| 跨卷中央备份导致锁释放和恢复 rename 失败 | cross_filesystem_backup_releases_lock_and_recovers_real_crash | RED→GREEN，最终矩阵通过 |
| 跳过/不变目标树与恢复目标物理身份未绑定 | changed_skipped_or_unchanged_target_invalidates_other_writes；same_bytes_new_target_identity_invalidates_restore | 同上 |
| 内置根链接可覆盖外置历史库或旧备份 | builtin_alias_cannot_overwrite_historical_external_library | 同上 |
| 恢复首次导入后库无法再次加载 | restoring_first_import_keeps_library_usable；abandoning_first_import_preparation_keeps_library_usable | 同上 |
| 可以写出第 1,025 条 catalog 记录 | importing_beyond_catalog_limit_is_rejected_before_writes | 同上 |
| 共享影响会读取未经授权的项目探针 | unapproved_project_probe_does_not_affect_shared_reader_evidence | 同上 |

回归提交 de8b88a，GitHub run 41451315947，Linux job 125297189647：已读取全部具名失败断言；不是依赖、编译或环境错误。修复通过具名回归和完整矩阵验收，不派第二次复审。

没有新增 Minor。第一阶段库存 DTO 的实际路径/link 展示保留到桌面契约阶段，底层快照信息仍在。

## 范围裁定

GUI 和桌面 IPC 属于 Tasks 8–11；真实 Agent 加载、最终桌面安装包和人工平台验收属于 Task 12。Rust 1.85 是 manifest 声明，当前统一验证 1.90，不能视为独立 MSRV 证明。这些不作为第二阶段实现遗漏，也不宣称已交付。

进度文档按最终证据更新。审阅本身不证明 CI 通过；最终 SHA 对应的完整 GitHub 工作流必须验收。锁 inode 在原卷私有目录归档，中央备份保留持久化元数据副本，以支持跨卷。首次导入恢复保留有效的空 catalog 和所有未引用现场，避免删除，也允许再次导入。

修复 f2eaaf6 的全部六类回归通过；最终代码验收 9df300c / [run 41637391199](https://github.com/rshun/skillcfg/actions/runs/41637391199)：6 个必需作业成功，Linux 109、两种 macOS 各 108、Windows 103 项测试，Clippy、格式、锁文件及四个 CLI 构建通过。跨文件系统真实设备测试只在 Linux /tmp 与 /dev/shm 运行。归档提交继续同一工作流，不将旧 SHA 的结果冒充新 SHA。

归档复核补充首次导入在 preparing 阶段中断的子场景：b0c3b08 / run 41636945159 的 Linux job 125920566645 在库重新加载处以 missing_catalog 失败。9df300c 通过保留当前标量快照，让显式恢复仅为已登记库补齐缺失 catalog；不回放不完整备份，不修改已有 catalog 或目标目录。run 41637391199 的四个原生日志均确认该具名用例转绿，属于同一 Important 第四类的修复收尾。
