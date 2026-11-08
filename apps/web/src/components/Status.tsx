import { label } from '../messages';
export function Status({state}:{state:string}) { return <span className={`status status-${state}`}>{label(state)}</span>; }
