import { MantineProvider } from '@mantine/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { Channel } from '@tauri-apps/api/core';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeAll, beforeEach, expect, test, vi } from 'vitest';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { ConflictQuery } from '../api/bindings/ConflictQuery';
import type { ConflictView } from '../api/bindings/ConflictView';
import type { Page } from '../api/bindings/Page';
import type { PairView } from '../api/bindings/PairView';
import type { Progress } from '../api/bindings/Progress';
import type { SetupView } from '../api/bindings/SetupView';
import type { SourceView } from '../api/bindings/SourceView';
import { queryKeys } from '../api/queries';
import { i18n, initI18n } from '../i18n';
import { ru } from '../i18n/ru';
import { ConflictsScreen } from './ConflictsScreen';

const A: SourceView = { kind: 'mod', id: 'a', name: 'Base A' };
const B: SourceView = { kind: 'mod', id: 'b', name: 'Base B' };
const VANILLA: SourceView = { kind: 'vanilla', id: null, name: '' };

function pair(winner: SourceView, loser: SourceView, extra: Partial<PairView> = {}): PairView {
  return {
    winner,
    loser,
    verdict: 'suspicious',
    files: 0,
    objects: 0,
    loc: 0,
    mark: 'notApplicable',
    ...extra,
  };
}

const analysis: AnalysisView = {
  playset: { id: 'main-id', name: 'Main', active: true, mods: 2, enabled: 2 },
  lang: 'english',
  vanilla: true,
  mods: [],
  counts: { suspicious: 1099, intended: 0, vanilla: 2, files: 2, objects: 1, loc: 1098 },
  pairs: [
    pair(A, B, { objects: 1, loc: 1098, mark: 'available' }),
    pair(A, VANILLA, { verdict: 'vanilla', files: 2 }),
  ],
  rulesError: null,
};

const setup: SetupView = {
  settings: {
    userDir: null,
    gameDir: null,
    analysisLang: null,
    uiLang: null,
    showVanilla: true,
    checkUpdates: false,
    launcherProcessNames: null,
    updateCheckedAt: null,
  },
  settingsError: null,
  userDir: '/u',
  gameDir: '/game',
  gameLanguage: 'english',
  launcherDbFound: true,
  appVersion: '0.1.0',
};

const locConflict: ConflictView = {
  level: 'loc',
  target: 'english::shared_key',
  database: null,
  key: 'shared_key',
  winner: { source: A, file: 'localization/english/a_l_english.yml', text: 'From A' },
  losers: [{ source: B, file: 'localization/english/b_l_english.yml', text: 'From B' }],
  verdict: 'suspicious',
  reason: 'earlierFilePath',
};

function objectConflict(i: number): ConflictView {
  return {
    level: 'object',
    target: `common/traits::trait_${i}`,
    database: 'common/traits',
    key: `trait_${i}`,
    winner: { source: A, file: 'common/traits/zz_base_a.txt', text: null },
    losers: [{ source: B, file: 'common/traits/00_base_b.txt', text: null }],
    verdict: 'suspicious',
    reason: 'laterFilePath',
  };
}

function searchPage(offset: number, limit: number, total: number): Page<ConflictView> {
  const end = Math.min(offset + limit, total);
  const items = Array.from({ length: Math.max(end - offset, 0) }, (_, i) =>
    objectConflict(offset + i),
  );
  return { items, total, offset };
}

type SearchArgs = { query: ConflictQuery; offset: number; limit: number };
type Args = Record<string, unknown>;

let calls: [string, Args][] = [];
let handlers: Record<string, (args: Args) => unknown> = {};

function allInvoked<T = Args>(cmd: string): T[] {
  return calls.filter(([c]) => c === cmd).map(([, args]) => args as T);
}
function lastInvoked<T = Args>(cmd: string): T {
  return allInvoked<T>(cmd).at(-1) as T;
}
const invoked = (cmd: string) => lastInvoked(cmd);

// jsdom has no layout: scroll containers get a 600px viewport and rows the conflict list's
// estimated height, so the virtualizer renders a bounded window of rows as in the app.
const ROW_HEIGHT = 120;
const ROW_STEP = ROW_HEIGHT + 8;
beforeAll(() => {
  vi.spyOn(Element.prototype, 'clientHeight', 'get').mockReturnValue(600);
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(ROW_HEIGHT);
});

beforeEach(async () => {
  calls = [];
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('ru-RU');
  await initI18n('ru');
  handlers = {
    get_setup: () => setup,
    pair_conflicts: () => ({ items: [locConflict], total: 1, offset: 0 }),
    search_conflicts: (args) => searchPage(args.offset as number, args.limit as number, 3),
    mark_intended: () => ({ path: '/c/rules.toml', error: null, rules: [] }),
    summary: () => ({
      ...analysis,
      pairs: [pair(A, B, { verdict: 'intended', objects: 1, loc: 1098 }), analysis.pairs[1]],
    }),
  };
  mockIPC((cmd, args) => {
    calls.push([cmd, args as Args]);
    return handlers[cmd]?.(args as Args) ?? null;
  });
});

afterEach(() => {
  vi.useRealTimers();
});

function renderScreen(view: AnalysisView = analysis) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(queryKeys.summary, view);
  render(
    <QueryClientProvider client={client}>
      <MantineProvider>
        <ConflictsScreen />
      </MantineProvider>
    </QueryClientProvider>,
  );
  return client;
}

const pairRow = (winner: string, loser: string) =>
  screen.getByRole('button', { name: i18n.t('conflicts.pair', { winner, loser }) });
const markButtonFor = (winner: string, loser: string) =>
  within(pairRow(winner, loser).closest('[data-pair]') as HTMLElement).getByRole('button', {
    name: 'Задумано',
  });
const searchBox = () => screen.getByRole('textbox', { name: ru.conflicts.search });
const vanillaSwitch = () => screen.getByRole('switch', { name: ru.conflicts.vanilla });

test('vanilla pairs are hidden by default', async () => {
  const user = userEvent.setup();
  renderScreen();
  expect(pairRow('Base A', 'Base B')).toBeInTheDocument();
  expect(screen.queryByText(ru.source.vanilla)).toBeNull();
  await user.click(screen.getByRole('checkbox', { name: ru.verdict.vanilla }));
  expect(screen.getByText(ru.source.vanilla)).toBeInTheDocument();
});

test('pair row shows counts with plurals', () => {
  renderScreen();
  expect(screen.getByText(/1 объект/)).toBeInTheDocument();
  expect(screen.getByText(/1098 строк/)).toBeInTheDocument();
});

test('mark intended sends a localized note', async () => {
  const user = userEvent.setup();
  const client = renderScreen();
  await user.click(screen.getByRole('button', { name: 'Задумано' }));
  expect(
    await screen.findByText(
      'Base B будет грузиться раньше Base A; если сейчас Base A выше Base B, на экране «Порядок» появится перестановка',
    ),
  ).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Подтвердить' }));
  await waitFor(() =>
    expect(invoked('mark_intended')).toEqual({
      winnerId: 'a',
      loserId: 'b',
      note: 'задумано: Base A перекрывает Base B',
    }),
  );
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Задумано' })).toBeNull());
  expect(client.getQueryData<AnalysisView>(queryKeys.summary)?.pairs[0].verdict).toBe('intended');
});

test('cycle pair has a disabled button', () => {
  renderScreen({
    ...analysis,
    pairs: [...analysis.pairs, pair(B, A, { files: 3, mark: 'cycle' })],
  });
  expect(markButtonFor('Base B', 'Base A')).toBeDisabled();
  expect(markButtonFor('Base B', 'Base A')).toHaveAccessibleDescription(
    i18n.t('markIntended.cycle', { winner: 'Base B', loser: 'Base A' }),
  );
  expect(markButtonFor('Base A', 'Base B')).toBeEnabled();
});

test('loc conflict shows both texts', async () => {
  const user = userEvent.setup();
  renderScreen();
  await user.click(pairRow('Base A', 'Base B'));
  expect(await screen.findByText('From A')).toBeInTheDocument();
  expect(screen.getByText('From B')).toBeInTheDocument();
  expect(screen.getByText(ru.explain.locEarlier)).toBeInTheDocument();
  expect(invoked('pair_conflicts')).toEqual({ winnerId: 'a', loserId: 'b', offset: 0, limit: 100 });
});

test('search is debounced and sends the text', async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
  renderScreen();
  await user.click(screen.getByRole('tab', { name: ru.conflicts.tabAll }));
  await user.type(searchBox(), 'shared');
  await act(async () => {
    vi.advanceTimersByTime(300);
  });
  await waitFor(() =>
    expect(lastInvoked<SearchArgs>('search_conflicts').query.text).toBe('shared'),
  );
  expect(allInvoked<SearchArgs>('search_conflicts').map((a) => a.query.text)).toEqual([
    '',
    'shared',
  ]);
});

test('long lists render only visible rows', async () => {
  handlers.search_conflicts = (args) =>
    searchPage(args.offset as number, args.limit as number, 5000);
  const user = userEvent.setup();
  renderScreen();
  await user.click(screen.getByRole('tab', { name: ru.conflicts.tabAll }));
  const rows = await screen.findAllByTestId('conflict-row');
  expect(rows.length).toBeLessThan(100);
  expect(screen.getByText(i18n.t('conflicts.total', { count: 5000 }))).toBeInTheDocument();
  expect(allInvoked('search_conflicts')).toEqual([
    {
      query: { text: '', levels: [], verdicts: ['suspicious', 'intended'] },
      offset: 0,
      limit: 100,
    },
  ]);
});

test('scrolling near the loaded end fetches the next page', async () => {
  handlers.search_conflicts = (args) =>
    searchPage(args.offset as number, args.limit as number, 5000);
  const user = userEvent.setup();
  renderScreen();
  await user.click(screen.getByRole('tab', { name: ru.conflicts.tabAll }));
  const rows = await screen.findAllByTestId('conflict-row');
  const list = rows[0].closest('[data-conflict-list]') as HTMLElement;
  fireEvent.scroll(list, { target: { scrollTop: 85 * ROW_STEP } });
  await waitFor(() => expect(lastInvoked<SearchArgs>('search_conflicts').offset).toBe(100));
  fireEvent.scroll(list, { target: { scrollTop: 105 * ROW_STEP } });
  expect(await screen.findByText('trait_105')).toBeInTheDocument();
  expect(allInvoked<SearchArgs>('search_conflicts').map((a) => a.offset)).toEqual([0, 100]);
});

test('vanilla switch is disabled without a game folder', async () => {
  handlers.get_setup = () => ({ ...setup, gameDir: null });
  renderScreen();
  await waitFor(() => expect(invoked('get_setup')).toBeDefined());
  expect(vanillaSwitch()).toBeDisabled();
  expect(vanillaSwitch()).toHaveAccessibleDescription(ru.conflicts.vanillaNoGameDir);
});

test('turning vanilla on stores the new summary', async () => {
  handlers.set_vanilla = () => ({
    ...analysis,
    pairs: [...analysis.pairs, pair(VANILLA, B, { objects: 4 })],
  });
  const user = userEvent.setup();
  const client = renderScreen({ ...analysis, vanilla: false, pairs: [analysis.pairs[0]] });
  await waitFor(() => expect(vanillaSwitch()).toBeEnabled());
  await user.click(vanillaSwitch());
  expect(await screen.findByText(ru.source.vanilla)).toBeInTheDocument();
  expect(invoked('set_vanilla')).toMatchObject({ on: true });
  expect(vanillaSwitch()).toBeChecked();
  expect(client.getQueryData<AnalysisView>(queryKeys.summary)?.vanilla).toBe(true);
});

test('while vanilla is applied its progress is shared and actions wait', async () => {
  handlers.set_vanilla = (args) => {
    (args.onProgress as Channel<Progress>).onmessage({ stage: 'vanilla' });
    return new Promise(() => {});
  };
  const user = userEvent.setup();
  const client = renderScreen({ ...analysis, vanilla: false });
  await waitFor(() => expect(vanillaSwitch()).toBeEnabled());
  await user.click(vanillaSwitch());
  await waitFor(() => expect(vanillaSwitch()).toBeDisabled());
  expect(vanillaSwitch()).toBeChecked();
  expect(screen.getByRole('button', { name: 'Задумано' })).toBeDisabled();
  expect(client.getQueryData(queryKeys.analysisProgress)).toEqual({ stage: 'vanilla' });
});
