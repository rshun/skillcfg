import type { Transport } from './types';
import { mockCall } from './mock';
import { errorCode } from './session';
export const call: Transport = async <T>(command: string, request: Record<string, unknown> = {}): Promise<T> => {
  try {
    if ('__TAURI_INTERNALS__' in window) {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>('execute', { command, request });
    }
    return await mockCall<T>(command, request);
  } catch (error) { throw { code: errorCode(error) }; }
};
