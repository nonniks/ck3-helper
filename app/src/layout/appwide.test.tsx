import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { beforeEach, expect, test, vi } from 'vitest';
import App from '../App';
import { initI18n } from '../i18n';
import { ru } from '../i18n/ru';

const setup = {
  settings: { uiLang: null },
  settingsError: null,
  launcherDbFound: true,
  appVersion: '0.1.0',
};
const analysis = {
  playset: { id: 'p', name: 'Мой плейсет', active: true, mods: 0, enabled: 0 },
  lang: 'l_english',
  vanilla: false,
  mods: [],
  counts: { suspicious: 0, intended: 0, vanilla: 0, files: 0, objects: 0, loc: 0 },
  pairs: [],
  rulesError: null,
};

beforeEach(async () => {
  vi.spyOn(navigator, 'language', 'get').mockReturnValue('ru-RU');
  await initI18n('ru');
});

test('a failed header refresh shows the error with detail and keeps the analysis and screens', async () => {
  const results: unknown[] = [analysis, { code: 'io', items: [], detail: 'disk full' }, analysis];
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'list_playsets') return [];
    if (cmd === 'analyze') {
      const r = results.shift();
      return r === analysis ? r : Promise.reject(r);
    }
    return null;
  });
  const user = userEvent.setup();
  render(<App />);
  const refresh = await screen.findByRole('button', { name: ru.header.refresh });
  await waitFor(() => expect(refresh).toBeEnabled());
  expect(screen.getByRole('link', { name: 'Конфликты' })).not.toHaveAttribute('data-disabled');

  await user.click(refresh);
  expect(await screen.findByText(ru.errorsPlain.io)).toBeInTheDocument();
  expect(screen.getByText('disk full')).toBeInTheDocument();
  expect(screen.queryByText(/ввода-вывода: $/)).toBeNull();
  expect(screen.getByRole('link', { name: 'Конфликты' })).not.toHaveAttribute('data-disabled');
  expect(screen.getByRole('button', { name: ru.header.refresh })).toBeEnabled();
});

test('a failed re-analysis after a folder change in settings disables the analysis screens', async () => {
  const fullSetup = {
    ...setup,
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
    userDir: '/u',
    gameDir: null,
    gameLanguage: null,
  };
  let analyses = 0;
  mockIPC((cmd, args) => {
    if (cmd === 'get_setup') return fullSetup;
    if (cmd === 'save_settings')
      return { ...fullSetup, settings: (args as { settings: object }).settings };
    if (cmd === 'list_playsets' || cmd === 'rules')
      return cmd === 'rules' ? { path: '', error: null, rules: [] } : [];
    if (cmd === 'analyze') {
      return analyses++ === 0
        ? analysis
        : Promise.reject({ code: 'io', items: [], detail: 'gone' });
    }
    return null;
  });
  const user = userEvent.setup();
  render(<App />);
  const refresh = await screen.findByRole('button', { name: ru.header.refresh });
  await waitFor(() => expect(refresh).toBeEnabled());
  await user.click(screen.getByRole('link', { name: 'Настройки' }));
  await user.type(
    await screen.findByRole('combobox', { name: ru.settings.analysisLang }),
    'l_russian',
  );
  await user.click(await screen.findByRole('button', { name: ru.settings.save }));
  expect(await screen.findByText('gone')).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Конфликты' })).toHaveAttribute('data-disabled', 'true');
});

test('Enter on a playset while an analysis runs does not start another one', async () => {
  let analyses = 0;
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'list_playsets')
      return [
        { id: 'p', name: 'Первый', active: true, mods: 0, enabled: 0 },
        { id: 'q', name: 'Второй', active: false, mods: 0, enabled: 0 },
      ];
    if (cmd === 'analyze') {
      analyses++;
      return new Promise(() => {});
    }
    return null;
  });
  const user = userEvent.setup();
  render(<App />);
  const other = await screen.findByText('Второй');
  const control = other.closest('[aria-disabled]') as HTMLElement;
  expect(control).not.toBeNull();
  control.focus();
  await user.keyboard('{Enter}');
  expect(analyses).toBe(1);
});

test('a later successful analysis clears the startup error', async () => {
  const results: unknown[] = [{ code: 'io', items: [], detail: 'boom' }, analysis];
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'list_playsets')
      return [{ id: 'p', name: 'Мой плейсет', active: true, mods: 0, enabled: 0 }];
    if (cmd === 'analyze') {
      const r = results.shift();
      return r === analysis ? r : Promise.reject(r);
    }
    return null;
  });
  const user = userEvent.setup();
  render(<App />);
  expect(await screen.findByText('boom')).toBeInTheDocument();
  await user.click(await screen.findByText('Мой плейсет'));
  await waitFor(() => expect(screen.queryByText('boom')).toBeNull());
});

test('disabled nav items ignore keyboard activation and expose aria-disabled', async () => {
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'analyze') return new Promise(() => {});
    return null;
  });
  const user = userEvent.setup();
  render(<App />);
  const link = await screen.findByRole('link', { name: 'Сверка' });
  expect(link).toHaveAttribute('aria-disabled', 'true');
  link.focus();
  await user.keyboard('{Enter}');
  await user.keyboard(' ');
  expect(screen.queryByRole('heading', { name: ru.screens.calibration })).toBeNull();
});

test('a failing plugin call is reported to the user', async () => {
  mockIPC((cmd) => {
    if (cmd === 'get_setup') return setup;
    if (cmd === 'analyze') return new Promise(() => {});
    if (cmd === 'check_update') return { version: '0.2.0', url: 'https://github.com/o/r' };
    if (cmd === 'plugin:opener|open_url') return Promise.reject('no browser');
    return null;
  });
  const user = userEvent.setup();
  render(<App />);
  await user.click(await screen.findByText(ru.update.open));
  expect(await screen.findByText(/no browser/)).toBeInTheDocument();
});

test('document language follows the UI language', async () => {
  await initI18n('en');
  expect(document.documentElement.lang).toBe('en');
  await initI18n('ru');
  expect(document.documentElement.lang).toBe('ru');
});
