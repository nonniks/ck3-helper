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
