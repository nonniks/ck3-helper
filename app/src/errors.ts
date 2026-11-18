import type { TFunction } from 'i18next';
import { isAppError } from './api/client';

export function errorMessage(t: TFunction, e: unknown): string {
  if (isAppError(e)) return t(`errors.${e.code}`, { items: e.items.join(', ') });
  return t('errors.unknown');
}
