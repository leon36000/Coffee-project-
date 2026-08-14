export type ChatResponse = {
  trace_id: string;
  mission_id: string;
  mission_state: string;
  response: string;
};

export type EvidenceRecord = {
  kind: string;
  capability_id?: string | null;
  status: string;
  policy_decision?: unknown;
  payload?: unknown;
  recorded_at?: string;
};

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function decode<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw new Error(`HermesClaw API request failed with HTTP ${response.status}`);
  }
  return (await response.json()) as T;
}

export async function sendChat(message: string): Promise<ChatResponse> {
  if (isTauriRuntime()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<ChatResponse>('chat', { message });
  }

  return decode<ChatResponse>(
    await fetch('/api/chat', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ message }),
    }),
  );
}

export async function getEvidence(traceId: string): Promise<EvidenceRecord[]> {
  if (isTauriRuntime()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<EvidenceRecord[]>('evidence', { traceId });
  }

  return decode<EvidenceRecord[]>(await fetch(`/api/evidence/${encodeURIComponent(traceId)}`));
}
