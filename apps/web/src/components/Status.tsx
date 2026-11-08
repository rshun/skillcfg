import { label } from '../messages';

export function Status({ state, pill = false }: { state: string; pill?: boolean }) {
  return (
    <span className={`${pill ? 'pill-status ' : ''}status status-${state}`}>
      {label(state)}
    </span>
  );
}
