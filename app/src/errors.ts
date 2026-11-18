import type { TFunction } from 'i18next';
import { isAppError } from './api/client';

export function errorMessage(t: TFunction, e: unknown): string {
  if (!isAppError(e)) return t('errors.unknown');
  if (e.items.length === 0 && t(`errorsPlain.${e.code}`, { defaultValue: '' }) !== '') {
    return t(`errorsPlain.${e.code}`);
  }
  return t(`errors.${e.code}`, { items: e.items.join(', ') });
}

export function errorDetail(e: unknown): string | null {
  return isAppError(e) && e.detail !== '' ? e.detail : null;
}
