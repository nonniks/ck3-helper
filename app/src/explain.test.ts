import type { TFunction } from 'i18next';
import { beforeAll, expect, test } from 'vitest';
import type { ConflictView } from './api/bindings/ConflictView';
import type { SourceView } from './api/bindings/SourceView';
import type { WinReasonView } from './api/bindings/WinReasonView';
import { explain } from './explain';
import { i18n, initI18n } from './i18n';

let t: TFunction;
beforeAll(async () => {
  await initI18n('ru');
  t = i18n.getFixedT('ru');
});

const A: SourceView = { kind: 'mod', id: 'a', name: 'Base A' };
const B: SourceView = { kind: 'mod', id: 'b', name: 'Base B' };
const VANILLA: SourceView = { kind: 'vanilla', id: null, name: '' };

const objectConflict: ConflictView = {
  level: 'object',
  target: 'common/traits::brave',
  database: 'common/traits',
  key: 'brave',
  winner: { source: A, file: 'common/traits/zz_base_a.txt', text: null },
  losers: [{ source: B, file: 'common/traits/00_base_b.txt', text: null }],
  verdict: 'suspicious',
  reason: 'laterFilePath',
};

const vanillaWins: ConflictView = {
  ...objectConflict,
  winner: { source: VANILLA, file: 'common/traits/zz_vanilla.txt', text: null },
  losers: [{ source: A, file: 'common/traits/00_base_a.txt', text: null }],
};

const reasons: WinReasonView[] = [
  'lowerInPlayset',
  'laterFilePath',
  'earlierFilePath',
  'replaceFolder',
];
const withReason = (reason: WinReasonView): ConflictView => ({ ...objectConflict, reason });

test('object conflict explains file order and suggests a compatch', () => {
  const lines = explain(t, objectConflict);
  expect(lines.join(' ')).toContain('zz_base_a.txt');
  expect(lines.join(' ')).toContain('00_base_b.txt');
  expect(lines.join(' ')).toContain('compatch');
});

test('vanilla winner adds the not-in-effect line', () => {
  expect(explain(t, vanillaWins)).toContain(t('explain.vanillaWins'));
  expect(explain(t, objectConflict)).not.toContain(t('explain.vanillaWins'));
});

test('each win reason has its own text', () => {
  expect(new Set(reasons.map((r) => explain(t, withReason(r))[0])).size).toBe(4);
});

test('file conflict names the winning mod', () => {
  expect(explain(t, withReason('lowerInPlayset'))[0]).toBe(
    'Загружается файл мода Base A: он ниже в плейсете; копии остальных модов не загружаются',
  );
});
