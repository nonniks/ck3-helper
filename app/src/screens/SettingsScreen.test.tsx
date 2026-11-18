import { MantineProvider } from '@mantine/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { beforeEach, expect, test, vi } from 'vitest';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { ModView } from '../api/bindings/ModView';
import type { RulesView } from '../api/bindings/RulesView';
import type { Settings } from '../api/bindings/Settings';
import type { SetupView } from '../api/bindings/SetupView';
import { queryKeys } from '../api/queries';
import { i18n, initI18n } from '../i18n';
import { ru } from '../i18n/ru';
import { SettingsScreen } from './SettingsScreen';

const settings: Settings = {
  userDir: null,
  gameDir: null,
  analysisLang: null,
  uiLang: null,
  showVanilla: false,
  checkUpdates: true,
  launcherProcessNames: null,
  updateCheckedAt: null,
};

const setup: SetupView = {
  settings,
  settingsError: null,
  userDir: '/detected/ck3',
  gameDir: null,
  gameLanguage: 'english',
  launcherDbFound: true,
  appVersion: '0.1.0',
};

function mod(id: string, name: string, position: number): ModView {
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
  };
}

const analysis: AnalysisView = {
  playset: { id: 'main-id', name: 'Main', active: true, mods: 2, enabled: 2 },
  lang: 'english',
  vanilla: false,
  mods: [mod('a', 'Base A', 0), mod('b', 'Base B', 1)],
  counts: { suspicious: 0, intended: 0, vanilla: 0, files: 0, objects: 0, loc: 0 },
  pairs: [],
  rulesError: null,
};

const noRules: RulesView = { path: '/cfg/rules.toml', error: null, rules: [] };

type Args = Record<string, unknown>;
let calls: [string, Args][] = [];
let handlers: Record<string, (args: Args) => unknown> = {};
const invoked = (cmd: string) => calls.find(([c]) => c === cmd)?.[1];

beforeEach(async () => {
  calls = [];
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('ru-RU');
  await initI18n('ru');
  handlers = {
    get_setup: () => setup,
    save_settings: (args) => ({ ...setup, settings: args.settings as Settings }),
    rules: () => noRules,
    summary: () => analysis,
    analyze: () => analysis,
    'plugin:dialog|open': () => '/tmp/ck3',
  };
  mockIPC((cmd, args) => {
    calls.push([cmd, args as Args]);
    const handler = handlers[cmd];
    return handler ? handler(args as Args) : null;
  });
});

function renderScreen(withAnalysis = true) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  if (withAnalysis) client.setQueryData(queryKeys.summary, analysis);
  render(
    <QueryClientProvider client={client}>
      <MantineProvider>
        <SettingsScreen />
      </MantineProvider>
    </QueryClientProvider>,
  );
}

const saveButton = () => screen.findByRole('button', { name: ru.settings.save });
const addRuleButton = () => screen.findByRole('button', { name: ru.rules.add });

async function pickMod(label: string, name: string) {
  const box = await screen.findByRole('combobox', { name: label });
  await waitFor(() => expect(box).toBeEnabled());
  await userEvent.click(box);
  await userEvent.type(box, `${name}{ArrowDown}{Enter}`);
}

test('folder picker fills user dir and saves', async () => {
  renderScreen();
  const [pickUserDir] = await screen.findAllByRole('button', { name: ru.settings.pick });
  await userEvent.click(pickUserDir);
  await userEvent.click(await saveButton());
  await screen.findByRole('button', { name: ru.settings.save });
  expect((invoked('save_settings')?.settings as Settings).userDir).toBe('/tmp/ck3');
});

test('cancelled picker changes nothing', async () => {
  handlers['plugin:dialog|open'] = () => null;
  renderScreen();
  const [pickUserDir] = await screen.findAllByRole('button', { name: ru.settings.pick });
  await userEvent.click(pickUserDir);
  await userEvent.click(await saveButton());
  await screen.findByRole('button', { name: ru.settings.save });
  expect((invoked('save_settings')?.settings as Settings).userDir).toBeNull();
});

test('changing analysis language re-analyzes the analysed playset', async () => {
  renderScreen();
  await userEvent.type(
    await screen.findByRole('combobox', { name: ru.settings.analysisLang }),
    'l_russian',
  );
  await userEvent.click(await saveButton());
  await screen.findByRole('button', { name: ru.settings.save });
  expect((invoked('save_settings')?.settings as Settings).analysisLang).toBe('l_russian');
  expect(invoked('analyze')).toEqual(expect.objectContaining({ playsetId: 'main-id' }));
});

test('saving without a relevant change does not re-analyze', async () => {
  renderScreen();
  await userEvent.click(await screen.findByRole('switch', { name: ru.settings.checkUpdates }));
  await userEvent.click(await saveButton());
  await screen.findByRole('button', { name: ru.settings.save });
  expect(invoked('save_settings')).toBeTruthy();
  expect(invoked('analyze')).toBeUndefined();
});

test('unapplied rule is marked', async () => {
  handlers.rules = () => ({
    ...noRules,
    rules: [{ index: 0, before: 'Nope', after: 'Base A', note: null, applied: false }],
  });
  renderScreen();
  expect(await screen.findByText(ru.rules.unapplied)).toBeInTheDocument();
});

test('broken rules file disables editing', async () => {
  handlers.rules = () => ({ ...noRules, error: 'expected ]' });
  renderScreen();
  expect(await screen.findByText(/expected \]/)).toBeInTheDocument();
  expect(await addRuleButton()).toBeDisabled();
});

test('add rule sends mod names', async () => {
  renderScreen();
  await pickMod(ru.rules.before, 'Base B');
  await pickMod(ru.rules.after, 'Base A');
  await userEvent.type(screen.getByRole('textbox', { name: ru.rules.note }), 'B first');
  await userEvent.click(await addRuleButton());
  await screen.findByRole('button', { name: ru.rules.add });
  expect(invoked('add_rule')).toEqual({ before: 'Base B', after: 'Base A', note: 'B first' });
});

test('add form is disabled without an analysis', async () => {
  renderScreen(false);
  expect(await addRuleButton()).toBeDisabled();
});

test('removing a rule asks for confirmation', async () => {
  handlers.rules = () => ({
    ...noRules,
    rules: [{ index: 3, before: 'Base B', after: 'Base A', note: 'B first', applied: true }],
  });
  renderScreen();
  await userEvent.click(
    await screen.findByRole('button', {
      name: i18n.t('rules.remove', { before: 'Base B', after: 'Base A' }),
    }),
  );
  expect(invoked('remove_rule')).toBeUndefined();
  await userEvent.click(await screen.findByRole('button', { name: ru.confirm.ok }));
  await screen.findByRole('button', { name: ru.rules.add });
  expect(invoked('remove_rule')).toEqual({ index: 3 });
});
