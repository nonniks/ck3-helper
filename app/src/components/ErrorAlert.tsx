import { Alert, Text } from '@mantine/core';
import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { errorDetail, errorMessage } from '../errors';

export function ErrorAlert({
  error,
  color = 'red',
  children,
}: {
  error: unknown;
  color?: string;
  children?: ReactNode;
}) {
  const { t } = useTranslation();
  const detail = errorDetail(error);
  return (
    <Alert color={color}>
      <Text size="sm">{errorMessage(t, error)}</Text>
      {detail && (
        <Text size="xs" c="dimmed">
          {detail}
        </Text>
      )}
      {children}
    </Alert>
  );
}
