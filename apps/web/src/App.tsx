import { FormEvent, useState } from 'react';
import { getEvidence, sendChat, type ChatResponse, type EvidenceRecord } from './api';
import './app.css';

type UiState = 'idle' | 'working' | 'completed' | 'failed';

function stateLabel(state: string): string {
  return state
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function App() {
  const [message, setMessage] = useState('');
  const [uiState, setUiState] = useState<UiState>('idle');
  const [mission, setMission] = useState<ChatResponse | null>(null);
  const [evidence, setEvidence] = useState<EvidenceRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const prompt = message.trim();
    if (!prompt || uiState === 'working') return;

    setUiState('working');
    setMission(null);
    setEvidence([]);
    setError(null);

    try {
      const result = await sendChat(prompt);
      setMission(result);
      const rows = await getEvidence(result.trace_id);
      setEvidence(rows);
      setUiState(result.mission_state === 'completed' ? 'completed' : 'idle');
    } catch {
      setError('HermesClaw could not complete this mission.');
      setUiState('failed');
    }
  }

  const status = mission ? stateLabel(mission.mission_state) : stateLabel(uiState);

  return (
    <main className="shell">
      <header className="topbar">
        <div className="brand" aria-label="HermesClaw">
          <span className="brand-mark" aria-hidden="true">HC</span>
          <div>
            <strong>HermesClaw</strong>
            <span>Autonomous computer agent</span>
          </div>
        </div>
        <div className="autonomy" aria-label="Autonomy profile">
          <span className="pulse" aria-hidden="true" />
          <span>Observe</span>
        </div>
      </header>

      <section className="workspace" aria-label="Mission workspace">
        <div className="conversation">
          <div className="intro">
            <p className="eyebrow">Mission control</p>
            <h1>What should HermesClaw do?</h1>
            <p>
              Start with a request to list workspace entries or read an authorized UTF-8 text file.
              Actions remain constrained by the visible autonomy profile and every capability execution
              leaves evidence.
            </p>
          </div>

          {mission && (
            <article className="response-card" aria-live="polite">
              <span className="response-label">HermesClaw</span>
              <p>{mission.response}</p>
            </article>
          )}

          {error && <p className="error" role="alert">{error}</p>}

          <form className="composer" onSubmit={submit}>
            <label htmlFor="message">Message</label>
            <textarea
              id="message"
              name="message"
              rows={3}
              value={message}
              onChange={(event) => setMessage(event.target.value)}
              placeholder="List this workspace or read alpha.txt"
            />
            <div className="composer-actions">
              <span>Observe · read-only capabilities</span>
              <button type="submit" disabled={!message.trim() || uiState === 'working'}>
                {uiState === 'working' ? 'Working…' : 'Run mission'}
              </button>
            </div>
          </form>
        </div>

        <aside className="mission-panel" aria-label="Mission status">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Current mission</p>
              <h2>{mission ? 'Execution proof' : 'Ready'}</h2>
            </div>
            <span className={`status status-${uiState}`}>{status}</span>
          </div>

          {mission ? (
            <>
              <dl className="mission-meta">
                <div>
                  <dt>Trace</dt>
                  <dd>{mission.trace_id}</dd>
                </div>
                <div>
                  <dt>Mission</dt>
                  <dd>{mission.mission_id}</dd>
                </div>
              </dl>

              <details className="evidence-panel">
                <summary>Evidence · {evidence.length} records</summary>
                <div className="evidence-list">
                  {evidence.map((row, index) => (
                    <article className="evidence-row" key={`${row.kind}-${index}`}>
                      <div>
                        <strong>{row.capability_id ?? row.kind}</strong>
                        <span>{row.kind.replaceAll('_', ' ')}</span>
                      </div>
                      <span className="evidence-status">{stateLabel(row.status)}</span>
                    </article>
                  ))}
                </div>
              </details>
            </>
          ) : (
            <p className="empty-state">
              Mission state, trace identity, approvals and evidence will appear here when work begins.
            </p>
          )}
        </aside>
      </section>
    </main>
  );
}
