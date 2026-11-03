export type AgentId = string;
export type Scope = 'user' | 'project';
export type Pair = { packageId: string; agent: AgentId };
export type FileDiff = { path: string; binary: boolean; before: string | null; after: string | null; beforeBytes: number; afterBytes: number; truncated: boolean };
export type Reader = { agent: string; rootId: string; variant: string; scope: Scope; project: string | null; state: string };
export type PlanGroup = { targetKey: string; path: string; disposition: string; selectedAgents: string[]; possibleReaders: Reader[]; files: FileDiff[]; reason: string | null };
export type Preview = { previewId: string; expiresAt: number; groups: PlanGroup[]; warnings: string[]; blocked: boolean; backupLocation: string };
export type Receipt = { operationId: string; backupLocation: string; outcomes: { path: string; outcome: string; reason: string | null }[] };
export type Transport = <T>(command: string, request?: Record<string, unknown>) => Promise<T>;
