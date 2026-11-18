import { MantineProvider } from '@mantine/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { beforeEach, expect, test, vi } from 'vitest';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { ModView } from '../api/bindings/ModView';
import type { PlaysetRow } from '../api/bindings/PlaysetRow';
import { queryKeys } from '../api/queries';
import { initI18n } from '../i18n';
import { PlaysetScreen } from './PlaysetScreen';

const openerCalls: string[] = [];
vi.mock('@tauri-apps/plugin-opener', () => ({
  revealItemInDir: (p: string) => {
    openerCalls.push(p);
    return Promise.resolve();
  },
}));

function mod(id: string, name: string, position: number, extra: Partial<ModView> = {}): ModView {
  return {
    id,
    name,
    position,
    enabled: true,
    missing: false,
    compatch: false,
    steamId: null,
    dir: `/mods/${id}`,
    version: null,
    supportedVersion: null,
    depIssues: [],
    replacePaths: [],
    warnings: 0,
    ...extra,
  };
}

const main: PlaysetRow = { id: 'main-id', name: 'Main', active: true, mods: 5, enabled: 5 };
const other: PlaysetRow = { id: 'other-id', name: 'Other', active: false, mods: 1, enabled: 1 };

const analysis: AnalysisView = {
  playset: main,
  lang: 'english',
  vanilla: false,
  mods: [
    mod('a-id', 'Base A', 0, { version: '1.2' }),
    mod('b-id', 'Base B', 1),
    mod('ab-id', 'A+B Compatch', 2, { compatch: true, replacePaths: ['history/titles'] }),
    mod('broken-id', 'Broken', 3, {
      depIssues: [
        { dependency: 'Missing Mod', ambiguous: false },
        { dependency: 'Twin', ambiguous: true },
      ],
      warnings: 1,
    }),
    mod('ghost-id', 'Ghost', 4, { missing: true }),
  ],
  counts: { suspicious: 0, intended: 0, vanilla: 0, files: 0, objects: 0, loc: 0 },
  pairs: [],
  rulesError: null,
};

let calls: Record<string, unknown> = {};
const invoked = (cmd: string) => calls[cmd];

beforeEach(async () => {
  calls = {};
  openerCalls.length = 0;
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('ru-RU');
  await initI18n('ru');
  mockIPC((cmd, args) => {
    calls[cmd] = args;
    if (cmd === 'list_playsets') return [main, other];
    if (cmd === 'mod_warnings')
      return [{ file: 'common/traits/bad.txt', message: 'unbalanced braces' }];
    if (cmd === 'analyze') return { ...analysis, playset: other, mods: [] };
    return null;
  });
});

function renderScreen() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(queryKeys.summary, analysis);
  return render(
    <QueryClientProvider client={client}>
      <MantineProvider>
        <PlaysetScreen />
      </MantineProvider>
    </QueryClientProvider>,
  );
}

const rowFor = (name: string) => screen.getByText(name).closest('tr') as HTMLElement;

test('shows mod statuses', async () => {
  renderScreen();
  expect(await screen.findByText('не найден на диске')).toBeInTheDocument();
  expect(screen.getByText('нет зависимости Missing Mod')).toBeInTheDocument();
  expect(screen.getByText('неоднозначная зависимость Twin')).toBeInTheDocument();
  expect(screen.getByText('compatch')).toBeInTheDocument();
  expect(screen.getByText('заменяет папку history/titles')).toBeInTheDocument();
});

test('positions are 1-based', async () => {
  renderScreen();
  expect(await screen.findByText('Base A')).toBeInTheDocument();
  expect(rowFor('Base A').querySelector('td')).toHaveTextContent('1');
  expect(rowFor('Ghost').querySelector('td')).toHaveTextContent('5');
});

test('warnings drawer lists files', async () => {
  const user = userEvent.setup();
  renderScreen();
  await screen.findByText('Broken');
  await user.click(rowFor('Broken').querySelector('[data-warnings]') as HTMLElement);
  expect(await screen.findByText('common/traits/bad.txt')).toBeInTheDocument();
  expect(invoked('mod_warnings')).toEqual({ modId: 'broken-id' });
});

test('choosing another playset analyzes it', async () => {
  const user = userEvent.setup();
  renderScreen();
  await user.click(await screen.findByText('Other'));
  await vi.waitFor(() => expect(invoked('analyze')).toMatchObject({ playsetId: 'other-id' }));
});

test('open folder is disabled for a missing mod', async () => {
  const user = userEvent.setup();
  renderScreen();
  await screen.findByText('Ghost');
  expect(rowFor('Ghost').querySelector('[data-open]')).toBeDisabled();
  await user.click(rowFor('Base A').querySelector('[data-open]') as HTMLElement);
  expect(openerCalls).toEqual(['/mods/a-id']);
});
