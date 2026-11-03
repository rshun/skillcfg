# 写入、备份与恢复边界

所有分发均为手动预览、明确确认。没有监听器或自动同步；导入和编辑只修改 Hub 库，档案启用不安装 Skill、不执行包内脚本、不修改 Agent 原生开关。

## 事务顺序

1. 校验完整包、目录授权和物理身份；冻结完整原始字节、执行位、目录和标量文件的基线。
2. 在中央备份根创建独占操作目录，持久化 manifest、original、planned，以及编辑/替换前的 retained revision。所有备份完成后才开始变更目标。
3. staging 位于同卷、所有已知 Skill 扫描根之外。逐目录复核基线，rename 归档旧目录，再激活完整新目录并复核。
4. 库 revision 先于 catalog；档案先于授权记录。前置写入失败时后续提交指针被阻止。没有删除旧 revision、额外文件或锁文件的自动清理。
5. 回执逐项记录 Written、Unchanged、Skipped、Blocked、FailedRestored、RecoveryRequired、AuditFailed。部分成功保留真实结果；不承诺多目录或跨卷全局原子性。

中央备份可以位于另一卷；原目标归档和激活必须同卷。原先不存在的目标在恢复时通过 rename 移出加载路径，不删除当前内容。新建 Unix 私有目录/文件使用 0700/0600，仅恢复执行位；现有目录权限不修改。不声称 Windows 目录 fsync 或 POSIX ACL 的完整保留。

## 手动确认

应用层仅返回安全 DTO 和一次性预览 ID，原始计划、快照和恢复锁证据保留在后端内存中。窗口单当前预览，600 秒过期；确认无论成功与否都会消费。切换库、范围、项目、配对、variant 或修改设置/档案后需重新预览。

确认时复核完整源/目标树、注册表、授权、探针原始结果、路径映射及物理身份。外部新增、删除、二进制修改或链接重定向均使整批预检失败。进入引擎后的竞态逐组报告；只在当前内容和路径身份仍属于本事务时自动回退，无法读取或存在后续外部编辑时保留现场供显式恢复。

快照原文限制 64 MiB，单包 16 MiB、单文件 4 MiB，最多 1,024 个包、4,096 个配对。应用层在复制重复目标前预算；超过上限需分批。公开文本 diff 每侧 64 KiB、整个视图 1 MiB，截断有标记；二进制只展示变化和大小。备份含原文，属于用户本机私有数据，不进入公开 DTO 或审计。

## 中断与锁

普通写入只尝试 create_new，全程持有内核独占能力。Unix 使用非阻塞 flock；Windows 使用拒绝其他读写句柄、允许 rename 的共享模式。进程退出时内核释放句柄，文件仍保留，普通写入不会自动夺取残留锁。

显式恢复预览确认锁不属于活跃进程，并绑定 operation ID、原始锁字节和物理身份。确认时重新取得排他能力并复核，先在中央备份持久化 recovered.lock 元数据副本，再将旧锁 inode rename 到锁所在卷的 .lock-records 私有目录，随后创建新锁。正常释放同样保留 released.lock 副本和原卷 inode，以支持跨卷中央备份。旧格式、无法证明归属或其他操作的锁均阻止恢复，不猜测。

preparing 状态表示引擎尚未开始目标提交；不完整备份只能显式放弃准备，保留后来编辑，不用不完整快照回写。已初始化日志才发布到正式操作目录，.initializing-* 残留保留供查且不作为可恢复事务扫描。

启动只列出待恢复任务，不自动恢复。恢复前再次备份当前完整现场；历史路径按写入时记录定位，不使用今天的 Skill 名字重新拼接。更换备份根只影响未来任务，旧根保留在单调追加的授权位置索引。设置损坏时禁止回默认后覆盖；仅使用有效历史位置索引进行明确的恢复。

目录只对当前已授权 Agent 根或已登记库开放；恢复旧项目操作需显式选择对应项目。应用不会为未知历史根自动扩大写入授权。

## 内核接口依据

- [Linux flock(2)](https://man7.org/linux/man-pages/man2/flock.2.html)：锁与打开描述关联，支持独占/非阻塞，关闭全部描述后释放。
- [Apple XNU fcntl.h](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/sys/fcntl.h)：macOS 的 flock 标志。
- [Rust Windows OpenOptionsExt](https://doc.rust-lang.org/stable/std/os/windows/fs/trait.OpenOptionsExt.html) 与 [Microsoft CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)：共享模式冲突与 FILE_SHARE_DELETE 的 rename 行为。

这是本地协作进程间的保守保护，不承诺防御拥有同一用户权限的恶意进程对所有父目录进行瞬间置换。

恢复首次导入后保留合法空 catalog，不删除未引用 revision 或 staging；库仍可继续导入。首次导入在准备阶段中断时，显式放弃也会为已登记的库补齐缺失 catalog；已有 catalog 保持当前内容，不回写不完整备份。所有预览目标（包括跳过和不变项）都绑定完整树与物理身份，历史库和备份根在实际分发/恢复时再次检查。共享读取影响与快照使用相同的单个探针授权判断，未授权探针只返回 Unknown。
