import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { beforeEach, expect, test, vi } from 'vitest';
import App from '../App';
import type { SetupView } from '../api/bindings/SetupView';
import { initI18n } from '../i18n';
import { ru } from '../i18n/ru';

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
  gameDir: null,
  gameLanguage: null,
  defaultUserDir: null,
  defaultGameDir: null,
  launcherDbFound: true,
  appVersion: '0.1.0',
};

beforeEach(async () => {
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('ru-RU');
  await initI18n('ru');
});

test('first run without launcher db opens settings', async () => {
  mockIPC((cmd) => (cmd === 'get_setup' ? { ...setup, launcherDbFound: false } : null));
  render(<App />);
  expect(await screen.findByText(ru.setup.noLauncherDb)).toBeInTheDocument();
});

test('analysis screens are disabled until analysis finishes', async () => {
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'analyze') return new Promise(() => {});
    return null;
  });
  render(<App />);
  expect(await screen.findByText(ru.progress.starting)).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Конфликты' })).toHaveAttribute('data-disabled', 'true');
  expect(screen.getByRole('link', { name: 'Плейсет' })).not.toHaveAttribute('data-disabled');
});

test('update banner links to the release', async () => {
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'analyze') return new Promise(() => {});
    if (cmd === 'check_update')
      return { version: '0.2.0', url: 'https://github.com/o/r/releases/tag/v0.2.0' };
    return null;
  });
  render(<App />);
  expect(await screen.findByText(/0\.2\.0/)).toBeInTheDocument();
});

test('a broken rules file is shown app-wide, outside settings', async () => {
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'list_playsets') return [];
    if (cmd === 'analyze')
      return {
        playset: { id: 'p1', name: 'Main', active: true, mods: 0, enabled: 0 },
        lang: 'english',
        vanilla: false,
        mods: [],
        counts: { suspicious: 0, intended: 0, vanilla: 0, files: 0, objects: 0, loc: 0 },
        pairs: [],
        rulesError: 'expected `=` at line 3',
      };
    return null;
  });
  render(<App />);
  expect(
    await screen.findByText(ru.banner.rulesError.replace('{{detail}}', 'expected `=` at line 3')),
  ).toBeInTheDocument();
});

test('analysis started from the playset list shows shared progress and disables refresh', async () => {
  const user = userEvent.setup();
  const first = {
    playset: { id: 'p1', name: 'Main', active: true, mods: 0, enabled: 0 },
    lang: 'english',
    vanilla: false,
    mods: [],
    counts: { suspicious: 0, intended: 0, vanilla: 0, files: 0, objects: 0, loc: 0 },
    pairs: [],
    rulesError: null,
  };
  let calls = 0;
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'list_playsets')
      return [first.playset, { id: 'p2', name: 'Other', active: false, mods: 0, enabled: 0 }];
    if (cmd === 'analyze') return ++calls === 1 ? first : new Promise(() => {});
    return null;
  });
  render(<App />);
  await user.click(await screen.findByText('Other'));
  expect(await screen.findByText(ru.progress.starting)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: ru.header.refresh })).toBeDisabled();
});
