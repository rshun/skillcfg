import { useEffect, useState } from 'react';
import type { LibraryRecord, PackageDocument, Receipt, Transport } from '../types';
import { useDiscardConfirmation } from '../components/DiscardConfirmation';
import { MutationReceipt, incompleteReceipt } from '../components/MutationReceipt';
import { Dialog } from '../components/Dialog';
import { message } from '../messages';
import { errorCode } from '../session';
export function SkillEditor({record,call,onClose,onSaved}:{record:LibraryRecord;call:Transport;onClose:()=>void;onSaved:()=>Promise<void>}) {
  const ask=useDiscardConfirmation();const [receipt,setReceipt]=useState<Receipt|null>(null);
  const [value,setValue]=useState(''),[original,setOriginal]=useState(''),[data,setData]=useState<PackageDocument|null>(null),[error,setError]=useState<string|null>(null),[busy,setBusy]=useState(false);
  useEffect(()=>{let active=true;void call<PackageDocument>('read_package',{packageId:record.id}).then(v=>{if(active){setData(v);setOriginal(v.document||'');setValue(v.document||'');}}).catch(e=>{if(active)setError(errorCode(e));});return()=>{active=false;};},[record.id,call]);
  const dirty=value!==original;
  async function close(){if(busy)return;if(!dirty||await ask('放弃尚未保存的 SKILL.md 修改？'))onClose();}
  async function save(){
    if(!data?.editToken)return;setBusy(true);setError(null);setReceipt(null);
    try{
      const fresh=await call<PackageDocument>('read_package',{packageId:record.id});
      if(fresh.packageId!==data.packageId||fresh.revisionId!==data.revisionId||fresh.document!==original||JSON.stringify(fresh.files)!==JSON.stringify(data.files)||!fresh.editToken)throw {code:'edit_baseline_changed'};
      const result=await call<Receipt>('save_skill_document',{packageId:record.id,editToken:fresh.editToken,document:value});
      if(incompleteReceipt(result)){setReceipt(result);return;}
      setOriginal(value);await onSaved();onClose();
    }catch(e){setError(errorCode(e));}finally{setBusy(false);}
  }
  return <Dialog title={`编辑 ${record.name}`} wide onClose={close} locked={busy}><div className="dialog-body"><p className="muted">编辑库中的 SKILL.md 原文，包内其他资源随当前 revision 保留。保存时会校验 frontmatter 与完整包。</p>{error&&<div className="notice error" data-editor-error role="alert">{message(error)}</div>}<MutationReceipt receipt={receipt}/>{!data&&!error?<p className="loading">正在读取文档…</p>:data?.document===null?<div className="notice warning">当前文档无法安全编辑，请检查来源包后重新导入。</div>:data&&<label className="field"><span>SKILL.md</span><textarea name="document" className="code editor" value={value} onChange={e=>setValue(e.target.value)} spellCheck={false} disabled={busy}/></label>}<details><summary>完整包文件（{data?.files.length||0}）</summary><ul className="file-list section-space">{data?.files.map(f=><li key={f}>{f}</li>)}</ul></details>{data?.issues.map(i=><p className="form-hint" key={i}>{message(i)}</p>)}<div className="notice">保存到库后，目标中的内容差异会重新计算；分发需要另行预览并确认。</div></div><footer className="dialog-footer"><span className="footer-copy">{dirty?'有未保存的修改':'仅编辑库内文档'}</span><button type="button" className="button" disabled={busy} onClick={close}>取消</button><button type="button" className="button primary" data-action="save-document" disabled={busy||!dirty||!data?.editToken} onClick={()=>void save()}>{busy?'正在保存…':'保存到库'}</button></footer></Dialog>;
}
