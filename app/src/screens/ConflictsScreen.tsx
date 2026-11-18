import { Title } from '@mantine/core';
import { useTranslation } from 'react-i18next';

export function ConflictsScreen() {
  const { t } = useTranslation();
  return <Title order={2}>{t('screens.conflicts')}</Title>;
}
