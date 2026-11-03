const paths: Record<string,string> = {
  skills:'M5 3h9l5 5v13H5z M14 3v5h5 M8 13h8 M8 17h6', agents:'M8 4h8v5H8z M4 15h6v5H4z M14 15h6v5h-6z M12 9v3 M7 12h10 M7 12v3 M17 12v3',
  history:'M4 8a8 8 0 1 1-1 7 M3 3v6h6 M12 7v5l3 2', settings:'M4 6h16 M4 12h16 M4 18h16 M8 3v6 M16 9v6 M10 15v6', search:'M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14 M15 15l6 6',
  plus:'M12 4v16 M4 12h16', sun:'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8 M12 2v2 M12 20v2 M2 12h2 M20 12h2 M5 5l1 1 M18 18l1 1 M5 19l1-1 M18 6l1-1', moon:'M20 15a8 8 0 0 1-11-11 8 8 0 1 0 11 11',
  close:'M6 6l12 12 M18 6 6 18', folder:'M3 6h7l2 2h9v12H3z', code:'M8 7l-5 5 5 5 M16 7l5 5-5 5 M14 4l-4 16', check:'M5 12l4 4L19 6', chevron:'m9 5 7 7-7 7', refresh:'M20 9a8 8 0 0 0-14-5L3 7 M3 2v5h5 M4 15a8 8 0 0 0 14 5l3-3 M21 22v-5h-5', warning:'m12 3 10 18H2z M12 9v5 M12 17h.01', link:'m10 14 4-4 M8 16l-1 1a4 4 0 0 1-6-6l4-4a4 4 0 0 1 6 0 M16 8l1-1a4 4 0 0 1 6 6l-4 4a4 4 0 0 1-6 0', arrow:'M4 12h15 M13 6l6 6-6 6', download:'M12 3v12 M7 10l5 5 5-5 M4 17v4h16v-4',
};
export function Icon({name,size=20}:{name:string;size?:number}) { return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false"><path d={paths[name] || paths.skills}/></svg>; }
export function AgentMark({name}:{name:string}) { return <span className="agent-mark" aria-hidden="true">{name.replace(/[^\p{L}\p{N}]/gu,'').slice(0,1).toUpperCase() || '?'}</span>; }
