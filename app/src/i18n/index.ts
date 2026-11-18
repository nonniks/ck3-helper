import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { en } from './en';
import { ru } from './ru';

export type Lang = 'ru' | 'en';

export function detectLanguage(
  setting: string | null | undefined,
  navigatorLanguage: string,
): Lang {
  if (setting === 'ru' || setting === 'en') return setting;
  return navigatorLanguage.toLowerCase().startsWith('ru') ? 'ru' : 'en';
}

export async function initI18n(lang: Lang): Promise<void> {
  await i18n.use(initReactI18next).init({
    lng: lang,
    fallbackLng: 'en',
    resources: { ru: { translation: ru }, en: { translation: en } },
    interpolation: { escapeValue: false },
  });
}

export { i18n };
