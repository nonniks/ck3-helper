import { expect, test } from 'vitest';
import type { ErrorCode } from './api/bindings/ErrorCode';
import { errorMessage } from './errors';
import { en } from './i18n/en';
import { i18n, initI18n } from './i18n';
import { ru } from './i18n/ru';

test('launcherRunning message names the processes', async () => {
  await initI18n('ru');
  expect(
    errorMessage(i18n.t, { code: 'launcherRunning', items: ['Paradox Launcher'], detail: '' }),
  ).toContain('Paradox Launcher');
});

test('non-AppError gives the unknown text', async () => {
  await initI18n('ru');
  expect(errorMessage(i18n.t, new Error('x'))).toBe(ru.errors.unknown);
});

test('missing launcher process names do not blame the settings file', async () => {
  const error = { code: 'settings', items: [], detail: 'no launcher process names configured' };
  await initI18n('ru');
  expect(errorMessage(i18n.t, error)).toMatch(/имена процесса лаунчера/i);
  expect(errorMessage(i18n.t, error)).not.toMatch(/не читается/);
  await initI18n('en');
  expect(errorMessage(i18n.t, error)).toMatch(/launcher process names/i);
  expect(errorMessage(i18n.t, error)).not.toMatch(/cannot be read/);
});

test('a rule or mod that is gone does not blame the rules file', async () => {
  const gone = { code: 'rules', items: ['3'], detail: 'no rule with index 3' };
  const unreadable = { code: 'rules', items: [], detail: 'expected `=`' };
  await initI18n('ru');
  expect(errorMessage(i18n.t, gone)).toMatch(/не найдены/);
  expect(errorMessage(i18n.t, gone)).not.toMatch(/не читается/);
  expect(errorMessage(i18n.t, unreadable)).toBe(ru.errorsPlain.rules);
  await initI18n('en');
  expect(errorMessage(i18n.t, gone)).toMatch(/not found/);
  expect(errorMessage(i18n.t, gone)).not.toMatch(/cannot be read/);
  expect(errorMessage(i18n.t, unreadable)).toBe(en.errorsPlain.rules);
});

test('every ErrorCode has a text in both languages', () => {
  const codes: Record<ErrorCode, true> = {
    noUserDir: true,
    launcherDbNotFound: true,
    schema: true,
    playsetNotFound: true,
    noActivePlayset: true,
    launcherRunning: true,
    processListUnavailable: true,
    playsetChanged: true,
    nothingToApply: true,
    cycle: true,
    notGameDir: true,
    logNotFound: true,
    rules: true,
    settings: true,
    notAnalyzed: true,
    io: true,
    db: true,
  };
  for (const code of Object.keys(codes)) {
    expect(ru.errors).toHaveProperty(code);
    expect(en.errors).toHaveProperty(code);
  }
  expect(Object.keys(ru.errors).sort()).toEqual([...Object.keys(codes), 'unknown'].sort());
});
