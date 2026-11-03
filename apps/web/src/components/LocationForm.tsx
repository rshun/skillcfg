import { Icon } from './Icon';
export function LocationForm({label,value,onChange,onPick,disabled=false}:{label:string;value:string;onChange:(v:string)=>void;onPick:()=>void;disabled?:boolean}) {
  return <label className="field"><span>{label}</span><span className="location-input"><input value={value} onChange={e=>onChange(e.target.value)} disabled={disabled} spellCheck={false}/><button type="button" className="button" disabled={disabled} onClick={onPick} aria-label={`选择${label}`}><Icon name="folder" size={16}/><span>选择目录</span></button></span></label>;
}
