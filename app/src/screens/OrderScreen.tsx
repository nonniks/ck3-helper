import { Title } from '@mantine/core';
import { useTranslation } from 'react-i18next';

export function OrderScreen() {
  const { t } = useTranslation();
  return <Title order={2}>{t('screens.order')}</Title>;
}
