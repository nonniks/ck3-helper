import { Alert, Anchor, Button, Group, Stack, Text } from '@mantine/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { pluginCall } from '../plugins';
import { useTranslation } from 'react-i18next';
import type { UpdateView } from '../api/bindings/UpdateView';

type Props = {
  playsetName: string | null;
  busy: boolean;
  canRefresh: boolean;
  onRefresh: () => void;
  settingsError: string | null;
  update: UpdateView | null;
};

export function Header({ playsetName, busy, canRefresh, onRefresh, settingsError, update }: Props) {
  const { t } = useTranslation();
  return (
    <Stack gap="xs" p="xs">
      <Group justify="space-between">
        <Text fw={600}>{playsetName ?? t('header.noPlayset')}</Text>
        <Button size="xs" onClick={onRefresh} disabled={busy || !canRefresh}>
          {t('header.refresh')}
        </Button>
      </Group>
      {settingsError && (
        <Alert color="yellow">{t('banner.settingsError', { detail: settingsError })}</Alert>
      )}
      {update && (
        <Alert color="blue">
          <Group gap="xs">
            <Text>{t('update.available', { version: update.version })}</Text>
            <Anchor
              component="button"
              type="button"
              onClick={() => {
                void pluginCall(() => openUrl(update.url));
              }}
            >
              {t('update.open')}
            </Anchor>
          </Group>
        </Alert>
      )}
    </Stack>
  );
}
