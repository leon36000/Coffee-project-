import '@testing-library/jest-dom/vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import * as api from './api';

vi.mock('./api', () => ({
  sendChat: vi.fn(),
  getEvidence: vi.fn(),
}));

const sendChat = vi.mocked(api.sendChat);
const getEvidence = vi.mocked(api.getEvidence);

describe('HermesClaw mission cockpit', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('renders a workspace read response and sanitized evidence', async () => {
    sendChat.mockResolvedValue({
      trace_id: 'read-trace',
      mission_id: 'read-mission',
      mission_state: 'completed',
      response: 'Contents of alpha.txt:\nalpha secret text',
    });
    getEvidence.mockResolvedValue([
      {
        kind: 'policy_decision',
        capability_id: 'workspace.read',
        status: 'allowed',
      },
      {
        kind: 'capability_execution',
        capability_id: 'workspace.read',
        status: 'succeeded',
        payload: {
          path: 'alpha.txt',
          bytes: 17,
          sha256: 'a'.repeat(64),
        },
      },
    ]);

    const user = userEvent.setup();
    render(<App />);

    expect(
      screen.getByPlaceholderText('List this workspace or read alpha.txt'),
    ).toBeInTheDocument();
    await user.type(screen.getByLabelText('Message'), 'Read alpha.txt');
    await user.click(screen.getByRole('button', { name: 'Run mission' }));

    expect(await screen.findByText('Completed')).toBeInTheDocument();
    expect(screen.getByText(/Contents of alpha.txt/)).toBeInTheDocument();
    expect(screen.getByText(/alpha secret text/)).toBeInTheDocument();

    const summary = screen.getByText(/Evidence/);
    await user.click(summary);
    const evidencePanel = summary.closest('details');
    expect(evidencePanel).not.toBeNull();
    expect(within(evidencePanel as HTMLElement).getAllByText('workspace.read').length).toBeGreaterThan(0);
    expect(within(evidencePanel as HTMLElement).queryByText(/alpha secret text/)).not.toBeInTheDocument();
    expect(sendChat).toHaveBeenCalledWith('Read alpha.txt');
    expect(getEvidence).toHaveBeenCalledWith('read-trace');
  });

  it('shows completed mission, observe autonomy, response and evidence', async () => {
    sendChat.mockResolvedValue({
      trace_id: '0198-trace',
      mission_id: '0198-mission',
      mission_state: 'completed',
      response: 'Workspace entries: alpha.txt',
    });
    getEvidence.mockResolvedValue([
      {
        kind: 'policy_decision',
        capability_id: 'workspace.list',
        status: 'allowed',
      },
      {
        kind: 'capability_execution',
        capability_id: 'workspace.list',
        status: 'succeeded',
      },
    ]);

    const user = userEvent.setup();
    render(<App />);

    expect(screen.getByText('Observe')).toBeInTheDocument();
    await user.type(screen.getByLabelText('Message'), 'List this workspace');
    await user.click(screen.getByRole('button', { name: 'Run mission' }));

    expect(await screen.findByText('Completed')).toBeInTheDocument();
    expect(screen.getByText('Workspace entries: alpha.txt')).toBeInTheDocument();
    expect(screen.getByText('0198-trace')).toBeInTheDocument();

    await user.click(screen.getByText(/Evidence/));
    expect(screen.getAllByText('workspace.list').length).toBeGreaterThan(0);
    expect(sendChat).toHaveBeenCalledWith('List this workspace');
    expect(getEvidence).toHaveBeenCalledWith('0198-trace');
  });
});
