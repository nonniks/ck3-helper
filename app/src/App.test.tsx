import { render, screen } from '@testing-library/react';
import { expect, test } from 'vitest';
import App from './App';
import { detectLanguage, initI18n } from './i18n';

test('navigation is in Russian for a Russian system', async () => {
  await initI18n(detectLanguage(null, 'ru-RU'));
  render(<App />);
  expect(screen.getByText('Конфликты')).toBeInTheDocument();
});

test('navigation is in English otherwise', async () => {
  await initI18n(detectLanguage(null, 'de-DE'));
  render(<App />);
  expect(screen.getByText('Conflicts')).toBeInTheDocument();
});

test('setting overrides the system language', () => {
  expect(detectLanguage('en', 'ru-RU')).toBe('en');
});
