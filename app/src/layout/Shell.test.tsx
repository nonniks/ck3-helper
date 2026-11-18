import { render, screen } from '@testing-library/react';
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
