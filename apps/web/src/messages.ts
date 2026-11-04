const messages: Record<string,string> = {
  edit_baseline_changed: '库中的版本已变化，草稿已保留。请复制草稿并重新打开，核对新版本后再保存。',
  settings_changed: '当前设置已变化，输入已保留。请核对已保存设置后再编辑，不能直接覆盖新版本。',
  invalid_settings: '设置文件损坏。请在变更历史中预览恢复，修复前不会写入库或 Agent。',
  invalid_catalog: '库索引无法解析，请核对位置或从历史备份恢复。', missing_catalog: '此目录缺少有效库索引。保留现场，检查历史备份。',
  preview_expired: '预览已过期，请重新预览。', preview_consumed: '此预览已使用或取消，请重新预览。', invalid_preview: '预览与当前操作不匹配，请重新预览。',
  source_changed: '来源内容已变化，请重新扫描。', target_changed: '目标内容已变化，请重新预览。', library_changed: '配置库已变化，请刷新后重试。',
  profile_changed: 'Agent 档案已变化，请重新读取并确认目录。', profile_revision_unknown: '档案编辑凭据已失效，请重新读取。',
  invalid_project: '请选择存在的项目目录。', invalid_home: '用户目录不可用。', request_too_large: '输入超出大小限制，请缩小本次操作。', preview_too_large: '本次内容超过 64 MiB，请分批处理。',
  write_failed: '保存失败，输入已保留。请检查目录权限后重试。', permission_denied: '目录权限不足。', profile_not_enabled: '此档案尚未启用，请先预览并确认其目录。',
  protected_root: '该目录与库或备份重叠，不能作为 Agent 目录。', protected_target: '目标与受保护的本地资料重叠，已阻止写入。',
  builtin_definition_immutable: '内置档案定义只读，可配置本机目录覆盖；新增 Agent 请使用 custom. 命名空间。',
  sensitive_content: '内容包含认证信息，已阻止操作。请先移除认证信息。', recovery_required: '存在未完成事务，请在变更历史中预览恢复。',
  operation_failed: '操作未完成，请核对目录、权限和当前状态后重试。', demo_profile_readonly: '此档案是演示资料；可用“添加 Agent”查看通用配置流程。',
};
export function message(code: string): string { return messages[code] || `操作未完成（${code}）。请检查当前输入后重试。`; }
export const statusLabels: Record<string,string> = { equal:'内容一致',different:'存在差异',missing:'未分发',unsupported:'不支持当前范围',read_error:'读取失败',unknown:'尚未检查',new:'新增',update:'替换',unchanged:'内容不变',skipped:'跳过',blocked:'已阻止',written:'已写入',failed_restored:'失败，已恢复',recovery_required:'需要恢复',audit_failed:'审计记录失败',builtin:'内置',enabled:'已启用',pending:'待确认目录',disabled:'已停用',invalid:'档案无效',distribution:'分发',import:'导入到库',edit:'编辑库',restore:'恢复',profile:'Agent 档案' };
export function label(key: string): string { return statusLabels[key] || key; }
