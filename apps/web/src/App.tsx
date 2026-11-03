import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import type { AppInfo, Detection, HistoryItem, ImportChoice, LibraryView, PackageDocument, Profile, Recovery, SettingsView, TargetStatus } from './types';
import { call } from './api';
import { errorCode, SessionController } from './session';
import { selectionFromCells } from './selection';
import { AppShell } from './components/AppShell';
import type { Page } from './components/AppShell';
import { Icon } from './components/Icon';
import { Library } from './pages/Library';
import { Agents } from './pages/Agents';
import { History } from './pages/History';
import { Settings } from './pages/Settings';
import { Import } from './pages/Import';
import { SkillEditor } from './pages/SkillEditor';
import { AgentProfileEditor } from './pages/AgentProfileEditor';
import { Preview } from './pages/Preview';
import { message } from './messages';
import './styles.css';
type Modal={kind:'import';source:string}|{kind:'edit';packageId:string}|{kind:'profile';profile:Profile|null}|null;
type LastRequest={kind:string;args:Record<string,unknown>;source?:string};
function profileSignature(profiles:Profile[]){return JSON.stringify(profiles.map(({revision,...p})=>({...p,roots:p.roots.map(({exists,...r})=>r)})).sort((a,b)=>(a.id||'').localeCompare(b.id||'')));}
export default function App() {
  const [session]=useState(()=>new SessionController(call));const state=useSyncExternalStore(session.subscribe,session.snapshot);
  const [page,setPage]=useState<Page>('library'),[info,setInfo]=useState<AppInfo|null>(null),[settings,setSettings]=useState<SettingsView|null>(null),[settingsError,setSettingsError]=useState<string|null>(null),[library,setLibrary]=useState<LibraryView|null>(null),[libraryError,setLibraryError]=useState<string|null>(null),[profiles,setProfiles]=useState<Profile[]>([]),[profilesError,setProfilesError]=useState<string|null>(null),[statuses,setStatuses]=useState<TargetStatus[]>([]),[statusError,setStatusError]=useState<string|null>(null),[detections,setDetections]=useState<Detection[]>([]),[history,setHistory]=useState<HistoryItem[]>([]),[historyError,setHistoryError]=useState<string|null>(null),[recoveries,setRecoveries]=useState<Recovery[]>([]),[recoveryError,setRecoveryError]=useState<string|null>(null);
  const [selectedId,setSelectedId]=useState(''),[document,setDocument]=useState<PackageDocument|null>(null),[documentError,setDocumentError]=useState<string|null>(null),[ready,setReady]=useState(false),[refreshing,setRefreshing]=useState(false),[scopeChanging,setScopeChanging]=useState(false),[modal,setModal]=useState<Modal>(null),[announcement,setAnnouncement]=useState(''),[decisions,setDecisions]=useState<Record<string,string>>({});
  const [theme,setTheme]=useState(()=>{try{return localStorage.getItem('skill-hub-theme')||(matchMedia('(prefers-color-scheme:dark)').matches?'dark':'light');}catch{return 'light';}});
  const readEpoch=useRef(0),navEpoch=useRef(0),profileRevision=useRef(''),lastRequest=useRef<LastRequest|null>(null),scopeLock=useRef(false),leave=useRef<()=>boolean>(()=>true);
  const registerGuard=useCallback((guard:(()=>boolean)|null)=>{leave.current=guard||(()=>true);},[]);
  const busy=state.phase==='confirming'||scopeChanging;
  useEffect(()=>{documentTheme(theme);},[theme]);
  function documentTheme(value:string){globalThis.document.documentElement.dataset.theme=value;try{localStorage.setItem('skill-hub-theme',value);}catch{/* Theme persistence is optional. */}}
  const refresh=useCallback(async()=>{
    const epoch=++readEpoch.current,generation=session.state.generation;setRefreshing(true);
    const query=async<T,>(command:string,store:(value:T)=>void,fail:(code:string|null)=>void)=>{try{const value=await session.read<T>(command);if(value!==null&&epoch===readEpoch.current&&generation===session.state.generation){store(value);fail(null);}}catch(e){if(epoch===readEpoch.current&&generation===session.state.generation)fail(errorCode(e));}};
    await Promise.all([
      query<SettingsView>('load_settings',setSettings,setSettingsError),
      query<LibraryView>('load_library',v=>{setLibrary(v);setSelectedId(old=>v.records.some(r=>r.id===old)?old:v.records[0]?.id||'');},setLibraryError),
      query<Profile[]>('list_agent_profiles',v=>{const signature=profileSignature(v);if(profileRevision.current&&profileRevision.current!==signature)void session.clear();profileRevision.current=signature;setProfiles(v);},setProfilesError),
      query<TargetStatus[]>('library_statuses',setStatuses,setStatusError),
      query<Detection[]>('detect_agents',setDetections,()=>{}),
      query<HistoryItem[]>('list_history',setHistory,setHistoryError),query<Recovery[]>('list_recoveries',setRecoveries,setRecoveryError),
    ]);
    if(epoch===readEpoch.current&&generation===session.state.generation){setRefreshing(false);setReady(true);}
  },[session]);
  useEffect(()=>{let active=true;void(async()=>{
    try{const data=await call<AppInfo>('app_info');if(active)setInfo(data);}catch{if(active)setAnnouncement('无法连接本机服务，请重新打开桌面窗口。');}
    try{const initial=await call<SettingsView>('load_settings');if(!active)return;setSettings(initial);await session.context({libraryPath:initial.settings.libraryPath,scope:'user',projectRoot:null});if(initial.settings.defaultScope==='project'&&initial.settings.lastProjectDir)await session.context({scope:'project',projectRoot:initial.settings.lastProjectDir});}
    catch(e){if(active)setSettingsError(errorCode(e));}
    if(active)await refresh();
  })();return()=>{active=false;++readEpoch.current;};},[refresh,session]);
  useEffect(()=>{let active=true;setDocument(null);setDocumentError(null);if(selectedId&&!libraryError)void session.read<PackageDocument>('read_package',{packageId:selectedId}).then(value=>{if(active&&value)setDocument(value);}).catch(e=>{if(active)setDocumentError(errorCode(e));});return()=>{active=false;};},[selectedId,library?.records,libraryError,state.generation,session]);
  useEffect(()=>{const timer=setInterval(()=>session.expire(),1000);return()=>clearInterval(timer);},[session]);
  useEffect(()=>{if(state.phase==='result')void refresh();},[state.phase,state.receipt?.operationId,refresh]);
  async function navigate(next:Page){if(busy||!leave.current())return;const epoch=++navEpoch.current;await session.cancel();if(epoch===navEpoch.current){setPage(next);setAnnouncement('');}}
  async function scope(scope:'user'|'project'){
    if(busy||scopeLock.current||!leave.current())return;scopeLock.current=true;setScopeChanging(true);
    try{const changed=scope==='project'?await session.pickProject():await session.context({scope:'user'});if(changed)await refresh();}catch(e){setAnnouncement(message(errorCode(e)));}finally{scopeLock.current=false;setScopeChanging(false);}
  }
  async function configurationChanged(){const view=await call<SettingsView>('load_settings');await session.context({libraryPath:view.settings.libraryPath});setSettings(view);await refresh();setAnnouncement('配置已保存，本次选择与旧预览已清空。');}
  async function librarySaved(){await session.cancel();await refresh();setAnnouncement('已保存到库，尚未分发。');}
  async function openTask(next:Modal){if(busy||!leave.current())return;await session.cancel();setModal(next);setAnnouncement('');}
  async function startPreview(kind:string,args:Record<string,unknown>,source?:string){setModal(null);lastRequest.current={kind,args,source};if(kind!=='distribution')setDecisions({});await session.prepare(kind,args);}
  function distribute(){setDecisions({});void startPreview('distribution',{selection:selectionFromCells(state.selection),conflictDecisions:{}});}
  function changeDecision(key:string,value:string){const last=lastRequest.current;if(!last||last.kind!=='distribution')return;const next={...decisions,[key]:value};setDecisions(next);void startPreview('distribution',{...last.args,conflictDecisions:next});}
  function repreview(){const last=lastRequest.current;if(!last)return;if(last.kind==='import'){void session.cancel().then(()=>setModal({kind:'import',source:last.source||''}));}else void session.prepare(last.kind,last.args);}
  const selectedRecord=modal?.kind==='edit'?library?.records.find(r=>r.id===modal.packageId):null;
  const mismatch=!!settings&&!!state.libraryPath&&settings.settings.libraryPath!==state.libraryPath;
  return <AppShell page={page} onPage={p=>void navigate(p)} info={info} state={scopeChanging?{...state,phase:'confirming'}:state} theme={theme} onTheme={()=>setTheme(theme==='dark'?'light':'dark')} onScope={s=>void scope(s)}><div className="main-content" data-ready={ready}>
    {announcement&&<div className="notice error-block" role="status">{announcement}</div>}{settingsError&&<div className="notice error error-block" role="alert">{message(settingsError)}</div>}{state.error&&state.phase==='idle'&&<div className="notice error error-block" role="alert">{message(state.error)}</div>}{mismatch&&<div className="notice warning error-block"><span>配置库位置已在外部改变，请重新载入工作台后继续。</span><button className="button" type="button" onClick={()=>void configurationChanged()}>重新载入</button></div>}{recoveries.length>0&&page!=='history'&&<div className="notice warning error-block"><Icon name="warning" size={17}/><span>有 {recoveries.length} 项事务需要处理。</span><button type="button" className="footer-ghost" onClick={()=>void navigate('history')}>查看待恢复任务</button></div>}
    {page==='library'&&<Library library={library} error={libraryError} statusError={statusError} profiles={profiles} statuses={statuses} selectedId={selectedId} onSelect={setSelectedId} document={document} documentError={documentError} state={state} onTarget={(key,value)=>session.select(key,value)} onImport={()=>void openTask({kind:'import',source:''})} onAgents={()=>void navigate('agents')} onEdit={()=>{if(selectedId)void openTask({kind:'edit',packageId:selectedId});}} onRefresh={()=>void refresh()} history={history}/>}
    {page==='agents'&&<Agents profiles={profiles} detections={detections} error={profilesError} session={session} busy={busy} onProfile={p=>void openTask({kind:'profile',profile:p})} onImport={source=>void openTask({kind:'import',source})} onChanged={configurationChanged}/>}
    {page==='history'&&<History items={history} recoveries={recoveries} error={historyError} recoveryError={recoveryError} busy={busy} onRestore={id=>void startPreview('restore',{operationId:id})} onRefresh={()=>void refresh()}/>}
    {page==='settings'&&<Settings key={state.generation} call={call} onSaved={configurationChanged} onHistory={()=>void navigate('history')} registerGuard={registerGuard}/>}
    </div>{page==='library'&&<footer className="selection-bar"><div><p className="selection-count"><b>{state.selection.length}</b> 个目标配对</p><p className="selection-hint">仅记录本次选择，预览后手动应用</p></div><div className="row"><button type="button" className="button quiet" disabled={!state.selection.length||busy} onClick={()=>void session.clear()}>清空</button><button type="button" className="button primary" data-action="preview-distribution" disabled={!state.selection.length||busy||refreshing||mismatch||!!settingsError||!!libraryError||!!state.error} onClick={distribute}>预览变更<Icon name="arrow" size={16}/></button></div></footer>}
    {modal?.kind==='import'&&<Import source={modal.source} records={library?.records||[]} call={call} onClose={()=>setModal(null)} onPreview={(scanId:string,choices:ImportChoice[],source:string)=>startPreview('import',{scanId,choices},source)}/>}
    {modal?.kind==='edit'&&selectedRecord&&<SkillEditor record={selectedRecord} call={call} onClose={()=>setModal(null)} onSaved={librarySaved}/>}
    {modal?.kind==='profile'&&<AgentProfileEditor profile={modal.profile} info={info} scope={state.scope} projectRoot={state.projectRoot} call={call} onClose={()=>setModal(null)} onSaved={configurationChanged}/>}
    {state.kind&&state.phase!=='idle'&&<Preview state={state} session={session} profiles={profiles} decisions={decisions} onDecision={changeDecision} onRepreview={repreview}/>}
  </AppShell>;
}
