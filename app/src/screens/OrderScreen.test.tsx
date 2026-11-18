import { MantineProvider } from '@mantine/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, within } from '@testing-library/react';
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
  return client;
}

const saveButton = () => screen.findByRole('button', { name: ru.apply.save });
const doneTitle = ru.apply.doneTitle.replace('{{name}}', 'Main (CK3 Helper)');

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

test('a mod that only shifted is not shown as moved', async () => {
  handlers.propose = () => ({
    ...proposal,
    rows: [
      { id: 'c', name: 'Shifted C', enabled: true, from: 2, to: 0, moved: false },
      ...proposal.rows.map((r) => ({ ...r, to: r.to + 1 })),
    ],
  });
  renderScreen();
  await screen.findByRole('table');
  const shifted = screen.getByRole('row', { name: /Shifted C/ });
  expect(within(shifted).queryByText(/было/)).toBeNull();
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

test('after a successful apply the screen shows the new playset and offers no second save', async () => {
  let proposals = 0;
  handlers.propose = () => {
    proposals++;
    return proposal;
  };
  const user = userEvent.setup();
  renderScreen();
  await user.click(await saveButton());
  await user.click(await screen.findByRole('button', { name: ru.apply.confirm }));
  const dialog = await screen.findByRole('dialog');
  expect(await within(dialog).findByText(doneTitle)).toBeInTheDocument();
  await user.click(within(dialog).getAllByRole('button', { name: ru.apply.close }).at(-1)!);
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.getByText(doneTitle)).toBeInTheDocument();
  expect(screen.getByText(ru.apply.pickInLauncher)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: ru.apply.save })).toBeNull();
  expect(screen.queryByRole('table')).toBeNull();
  expect(proposals).toBe(1);
});

test('a failing proposal refetch keeps the apply dialog and shows the error', async () => {
  const user = userEvent.setup();
  const client = renderScreen();
  await user.click(await saveButton());
  expect(await screen.findByText('Main (CK3 Helper)')).toBeInTheDocument();
  handlers.propose = () => Promise.reject({ code: 'io', items: [], detail: 'proposal gone' });
  await client.invalidateQueries({ queryKey: queryKeys.proposal });
  expect(await screen.findByText('proposal gone')).toBeInTheDocument();
  expect(screen.getByRole('dialog')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: ru.apply.confirm }));
  expect(await screen.findByRole('button', { name: ru.apply.revealBackup })).toBeInTheDocument();
});

test('the apply dialog cannot be closed while saving', async () => {
  let finish: (result: unknown) => void = () => {};
  handlers.apply = () => new Promise((resolve) => (finish = resolve));
  const user = userEvent.setup();
  renderScreen();
  await user.click(await saveButton());
  await user.click(await screen.findByRole('button', { name: ru.apply.confirm }));
  const dialog = screen.getByRole('dialog');
  await waitFor(() =>
    expect(within(dialog).getByRole('button', { name: ru.apply.cancel })).toBeDisabled(),
  );
  expect(within(dialog).queryByRole('button', { name: ru.common.close })).toBeNull();
  await user.keyboard('{Escape}');
  finish({ newPlaysetName: 'Main (CK3 Helper)', backupPath: '/db/dir/backup.sqlite' });
  expect(await screen.findByRole('button', { name: ru.apply.revealBackup })).toBeInTheDocument();
});

test('a failed recalculation does not come back when the dialog is reopened', async () => {
  handlers.apply = () => Promise.reject({ code: 'playsetChanged', items: [], detail: '' });
  handlers.analyze = () => Promise.reject({ code: 'io', items: [], detail: 'disk full' });
  const user = userEvent.setup();
  renderScreen();
  await user.click(await saveButton());
  await user.click(await screen.findByRole('button', { name: ru.apply.confirm }));
  await user.click(await screen.findByRole('button', { name: ru.apply.recalculate }));
  expect(await screen.findByText('disk full')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: ru.apply.cancel }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await user.click(await saveButton());
  await user.click(await screen.findByRole('button', { name: ru.apply.confirm }));
  expect(await screen.findByRole('button', { name: ru.apply.recalculate })).toBeInTheDocument();
  expect(screen.queryByText('disk full')).toBeNull();
});

test('a failed recalculation is shown inside the dialog with its detail', async () => {
  handlers.apply = () => Promise.reject({ code: 'playsetChanged', items: [], detail: '' });
  handlers.analyze = () => Promise.reject({ code: 'io', items: [], detail: 'disk full' });
  const user = userEvent.setup();
  renderScreen();
  await user.click(await saveButton());
  await user.click(await screen.findByRole('button', { name: ru.apply.confirm }));
  await user.click(await screen.findByRole('button', { name: ru.apply.recalculate }));
  const dialog = await screen.findByRole('dialog');
  expect(await within(dialog).findByText('disk full')).toBeInTheDocument();
  expect(within(dialog).getByText(ru.errorsPlain.io)).toBeInTheDocument();
});

test('user rule reason shows side and note', async () => {
  renderScreen();
  expect(await screen.findByText(/раньше Base A — B first/)).toBeInTheDocument();
  expect(screen.getByText(/зависит от Lib/)).toBeInTheDocument();
  expect(screen.getByText(/нужен для Base B/)).toBeInTheDocument();
});
