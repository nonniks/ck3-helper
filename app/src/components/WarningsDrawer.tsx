import { Drawer, Stack, Text } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { modWarnings } from '../api/client';
import { errorMessage } from '../errors';

export function WarningsDrawer({
  modId,
  modName,
  onClose,
}: {
  modId: string | null;
  modName: string;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const warnings = useQuery({
    queryKey: ['warnings', modId],
    queryFn: () => modWarnings(modId as string),
    enabled: modId !== null,
  });
  return (
    <Drawer
      opened={modId !== null}
      onClose={onClose}
      position="right"
      size="lg"
      title={t('playset.warningsTitle', { name: modName })}
    >
      <Stack>
        {warnings.isError && <Text c="red">{errorMessage(t, warnings.error)}</Text>}
        {warnings.data?.length === 0 && <Text c="dimmed">{t('playset.warningsEmpty')}</Text>}
        {warnings.data?.map((w, i) => (
          <div key={`${w.file}:${i}`}>
            <Text ff="monospace" size="sm">
              {w.file}
            </Text>
            <Text size="sm">{w.message}</Text>
          </div>
        ))}
      </Stack>
    </Drawer>
  );
}
