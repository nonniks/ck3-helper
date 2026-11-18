import { MantineProvider } from '@mantine/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { beforeEach, expect, test, vi } from 'vitest';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { LauncherStatus } from '../api/bindings/LauncherStatus';
import type { ProposalView } from '../api/bindings/ProposalView';
import { queryKeys } from '../api/queries';
import { initI18n } from '../i18n';
import { ru } from '../i18n/ru';
import { OrderScreen } from './OrderScreen';

const analysis: AnalysisView = {
  playset: { id: 'main-id', name: 'Main', active: true, mods: 2, enabled: 2 },
  lang: 'english',
  vanilla: false,
  mods: [],
  counts: { suspicious: 0, intended: 0, vanilla: 0, files: 0, objects: 0, loc: 0 },
  pairs: [],
  rulesError: null,
};

const proposal: ProposalView = {
  rows: [
    { id: 'b', name: 'Base B', enabled: true, from: 1, to: 0, moved: true },
    { id: 'a', name: 'Base A', enabled: true, from: 0, to: 1, moved: true },
  ],
  moves: [
    {
      id: 'b',
      name: 'Base B',
      from: 1,
      to: 0,
      reasons: [
        { kind: 'userRule', other: 'Base A', before: true, note: 'B first' },
        { kind: 'dependency', on: 'Lib' },
      ],
    },
    {
      id: 'a',
      name: 'Base A',
      from: 0,
      to: 1,
      reasons: [{ kind: 'requiredBy', by: 'Base B' }],
    },
  ],
  winnerChanges: [],
  unresolvedRules: [],
  alreadySorted: false,
};

type Args = Record<string, unknown>;
let calls: [string, Args][] = [];
let handlers: Record<string, (args: Args) => unknown> = {};
const invoked = (cmd: string) => calls.find(([c]) => c === cmd)?.[1];

beforeEach(async () => {
  calls = [];
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('ru-RU');
  await initI18n('ru');
  handlers = {
    propose: () => proposal,
    launcher_status: () => ({ state: 'notRunning' }) satisfies LauncherStatus,
    apply_preview: () => ({ newPlaysetName: 'Main (CK3 Helper)', backupDir: '/db/dir' }),
    apply: () => ({ newPlaysetName: 'Main (CK3 Helper)', backupPath: '/db/dir/backup.sqlite' }),
    analyze: () => analysis,
  };
  mockIPC((cmd, args) => {
    calls.push([cmd, args as Args]);
    const handler = handlers[cmd];
    return handler ? handler(args as Args) : null;
  });
});

function renderScreen() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(queryKeys.summary, analysis);
  render(
    <QueryClientProvider client={client}>
      <MantineProvider>
        <OrderScreen />
      </MantineProvider>
    </QueryClientProvider>,
  );
}

const saveButton = () => screen.findByRole('button', { name: ru.apply.save });

test('running launcher disables saving and names the process', async () => {
  handlers.launcher_status = () => ({ state: 'running', processes: ['Paradox Launcher'] });
  renderScreen();
  expect(await saveButton()).toBeDisabled();
  expect(await screen.findByText(/Paradox Launcher/)).toBeInTheDocument();
});

test('unknown process list disables saving with its own text', async () => {
  handlers.launcher_status = () => ({ state: 'unknown' });
  renderScreen();
  expect(await screen.findByText(ru.apply.processListUnavailable)).toBeInTheDocument();
  expect(await saveButton()).toBeDisabled();
});

test('already sorted shows no save button', async () => {
  handlers.propose = () => ({ ...proposal, rows: [], moves: [], alreadySorted: true });
  renderScreen();
  expect(await screen.findByText(ru.order.alreadySorted)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: ru.apply.save })).toBeNull();
});

test('cycle shows the mod names and the hint, no table, no button', async () => {
  handlers.propose = () => Promise.reject({ code: 'cycle', items: ['Mod X', 'Mod Y'] });
  renderScreen();
  expect(await screen.findByText(/Mod X, Mod Y/)).toBeInTheDocument();
  expect(screen.getByText(ru.order.cycleHint)).toBeInTheDocument();
  expect(screen.queryByRole('table')).toBeNull();
  expect(screen.queryByRole('button', { name: ru.apply.save })).toBeNull();
});

test('table shows 1-based new positions and previous positions of moved mods', async () => {
  renderScreen();
  await screen.findByRole('table');
  expect(screen.getByText(ru.order.was.replace('{{from}}', '2'))).toBeInTheDocument();
});

test('apply flow shows the copy name and reveals the backup', async () => {
  const user = userEvent.setup();
  renderScreen();
  await user.click(await saveButton());
  expect(await screen.findByText('Main (CK3 Helper)')).toBeInTheDocument();
  expect(screen.getByText('/db/dir')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: ru.apply.confirm }));
  await user.click(await screen.findByRole('button', { name: ru.apply.revealBackup }));
  expect(invoked('plugin:opener|reveal_item_in_dir')).toBeTruthy();
});

test('changed playset offers to recalculate', async () => {
  handlers.apply = () => Promise.reject({ code: 'playsetChanged', items: [] });
  const user = userEvent.setup();
  renderScreen();
  await user.click(await saveButton());
  await user.click(await screen.findByRole('button', { name: ru.apply.confirm }));
  await user.click(await screen.findByRole('button', { name: ru.apply.recalculate }));
  await waitFor(() =>
    expect(invoked('analyze')).toEqual(expect.objectContaining({ playsetId: 'main-id' })),
  );
});

test('user rule reason shows side and note', async () => {
  renderScreen();
  expect(await screen.findByText(/раньше Base A — B first/)).toBeInTheDocument();
  expect(screen.getByText(/зависит от Lib/)).toBeInTheDocument();
  expect(screen.getByText(/нужен для Base B/)).toBeInTheDocument();
});
