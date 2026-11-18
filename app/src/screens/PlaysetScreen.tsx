import { Title } from '@mantine/core';
import { useTranslation } from 'react-i18next';

export function PlaysetScreen() {
  const { t } = useTranslation();
  return <Title order={2}>{t('screens.playset')}</Title>;
}
