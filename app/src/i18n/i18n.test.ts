import { expect, test } from 'vitest';
import { en } from './en';
import { i18n, initI18n } from './index';
import { ru } from './ru';

function keys(obj: object, prefix = ''): string[] {
  return Object.entries(obj)
    .flatMap(([k, v]) =>
      typeof v === 'object' && v !== null
        ? keys(v, `${prefix}${k}.`)
        : [`${prefix}${k}`.replace(/_(zero|one|two|few|many|other)$/, '')],
    )
    .sort()
    .filter((k, i, a) => a.indexOf(k) === i);
}

test('ru and en have the same keys, plural suffixes folded', () => {
  expect(keys(ru)).toEqual(keys(en));
});

test('Russian plurals', async () => {
  await initI18n('ru');
  expect(i18n.t('counts.objects', { count: 1 })).toBe('1 объект');
  expect(i18n.t('counts.objects', { count: 3 })).toBe('3 объекта');
  expect(i18n.t('counts.objects', { count: 5 })).toBe('5 объектов');
});
