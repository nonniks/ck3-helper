import { render, screen } from '@testing-library/react';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import App from './App';
import { detectLanguage, initI18n } from './i18n';

beforeEach(() => {
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return { settings: { uiLang: null }, launcherDbFound: true };
    if (cmd === 'analyze') return new Promise(() => {});
    return null;
  });
});
afterEach(() => vi.restoreAllMocks());

test('navigation is in Russian for a Russian system', async () => {
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('ru-RU');
  await initI18n(detectLanguage(null, 'ru-RU'));
  render(<App />);
  expect(await screen.findByText('Конфликты')).toBeInTheDocument();
});

test('navigation is in English otherwise', async () => {
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('de-DE');
  await initI18n(detectLanguage(null, 'de-DE'));
  render(<App />);
  expect(await screen.findByText('Conflicts')).toBeInTheDocument();
});

test('setting overrides the system language', () => {
  expect(detectLanguage('en', 'ru-RU')).toBe('en');
});
