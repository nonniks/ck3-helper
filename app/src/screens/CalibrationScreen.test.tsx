import { MantineProvider } from '@mantine/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { beforeEach, expect, test, vi } from 'vitest';
import type { CalibrationView } from '../api/bindings/CalibrationView';
import type { SetupView } from '../api/bindings/SetupView';
import { initI18n } from '../i18n';
import { ru } from '../i18n/ru';
import { CalibrationScreen } from './CalibrationScreen';

const setup: SetupView = {
  settings: {
    userDir: null,
    gameDir: null,
    analysisLang: null,
    uiLang: null,
    showVanilla: false,
    checkUpdates: true,
    launcherProcessNames: null,
    updateCheckedAt: null,
  },
  settingsError: null,
  userDir: '/u',
  gameDir: '/game',
  gameLanguage: 'english',
  defaultUserDir: null,
  defaultGameDir: null,
  launcherDbFound: true,
  appVersion: '0.1.0',
};

const view: CalibrationView = {
  matched: 1370,
  mismatched: [],
  logUnknown: [],
  loggedSingleDefinition: [],
  predictedNotLogged: [],
  truncated: false,
  lastRun: null,
};

type Args = Record<string, unknown>;
let handlers: Record<string, (args: Args) => unknown> = {};
const openSettings = vi.fn();

beforeEach(async () => {
  openSettings.mockClear();
  await initI18n('ru');
  handlers = { get_setup: () => setup, calibrate: () => view };
  mockIPC((cmd, args) => {
    const handler = handlers[cmd];
    return handler ? handler(args as Args) : null;
  });
});

function renderScreen() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={client}>
      <MantineProvider>
        <CalibrationScreen onOpenSettings={openSettings} />
      </MantineProvider>
    </QueryClientProvider>,
  );
}

const runButton = () => screen.findByRole('button', { name: ru.calibration.run });

test('no log explains how to get one', async () => {
  handlers.calibrate = () => Promise.reject({ code: 'logNotFound', items: [] });
  renderScreen();
  await userEvent.click(await runButton());
  expect(await screen.findByText(ru.calibration.noLog)).toBeInTheDocument();
});

test('calibration summary', async () => {
  renderScreen();
  await userEvent.click(await runButton());
  expect(await screen.findByText(/1370/)).toBeInTheDocument();
});

test('without a game folder the screen links to settings', async () => {
  handlers.get_setup = () => ({ ...setup, gameDir: null });
  renderScreen();
  expect(await screen.findByText(ru.calibration.noGameDir)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: ru.calibration.run })).toBeNull();
  await userEvent.click(screen.getByRole('button', { name: ru.calibration.openSettings }));
  expect(openSettings).toHaveBeenCalled();
});

test('warnings name the mods of the last run', async () => {
  handlers.calibrate = () => ({
    ...view,
    truncated: true,
    lastRun: { notInLastRun: ['New Mod'], onlyInLastRun: ['Old Mod'] },
  });
  renderScreen();
  await userEvent.click(await runButton());
  expect(await screen.findByText(ru.calibration.truncated)).toBeInTheDocument();
  expect(screen.getByText(/New Mod/)).toBeInTheDocument();
  expect(screen.getByText(/Old Mod/)).toBeInTheDocument();
});
