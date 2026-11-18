import { Button, Group, Loader, Modal, Stack, Text } from '@mantine/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { pluginCall } from '../plugins';
import { useTranslation } from 'react-i18next';
import { apply, applyPreview, isAppError } from '../api/client';
import { queryKeys, useAnalyze } from '../api/queries';
import { ErrorAlert } from './ErrorAlert';

export function ApplyDialog({
  opened,
  onClose,
  playsetId,
}: {
  opened: boolean;
  onClose: () => void;
  playsetId: string | null;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const preview = useQuery({
    queryKey: ['applyPreview'],
    queryFn: applyPreview,
    enabled: opened,
    gcTime: 0,
    retry: false,
  });
  const analyze = useAnalyze();
  const save = useMutation({
    mutationFn: apply,
    // Rust spends its proposal on success; the screen shows the result until the next analysis.
    onSuccess: (result) => queryClient.setQueryData(queryKeys.applied, result),
  });
  const close = () => {
    save.reset();
    analyze.reset();
    onClose();
  };
  const changed = isAppError(save.error) && save.error.code === 'playsetChanged';
  const done = save.data;
  // The copy and the backup are written even if the dialog goes away, so it stays until done.
  const saving = save.isPending;

  return (
    <Modal
      opened={opened}
      onClose={close}
      title={t('apply.dialogTitle')}
      closeOnEscape={!saving}
      closeOnClickOutside={!saving}
      withCloseButton={!saving}
      closeButtonProps={{ 'aria-label': t('common.close') }}
    >
      {done ? (
        <Stack>
          <Text fw={600}>{t('apply.doneTitle', { name: done.newPlaysetName })}</Text>
          <Text size="sm">{t('apply.pickInLauncher')}</Text>
          <Group justify="flex-end">
            <Button
              variant="default"
              onClick={() => void pluginCall(() => revealItemInDir(done.backupPath))}
            >
              {t('apply.revealBackup')}
            </Button>
            <Button onClick={close}>{t('apply.close')}</Button>
          </Group>
        </Stack>
      ) : (
        <Stack>
          {preview.isPending && <Loader size="sm" />}
          {preview.isError && <ErrorAlert error={preview.error} />}
          {preview.data && (
            <>
              <div>
                <Text size="xs" c="dimmed">
                  {t('apply.copyName')}
                </Text>
                <Text>{preview.data.newPlaysetName}</Text>
              </div>
              <div>
                <Text size="xs" c="dimmed">
                  {t('apply.backupDir')}
                </Text>
                <Text>{preview.data.backupDir}</Text>
              </div>
              <Text size="sm">{t('apply.sourceUntouched')}</Text>
            </>
          )}
          {save.isError && <ErrorAlert error={save.error} />}
          {changed && analyze.isError && <ErrorAlert error={analyze.error} />}
          <Group justify="flex-end">
            <Button variant="default" onClick={close} disabled={saving}>
              {t('apply.cancel')}
            </Button>
            {changed ? (
              <Button
                loading={analyze.isPending}
                onClick={() => analyze.mutate({ playsetId }, { onSuccess: close })}
              >
                {t('apply.recalculate')}
              </Button>
            ) : (
              <Button
                loading={save.isPending}
                disabled={!preview.data}
                onClick={() => save.mutate()}
              >
                {t('apply.confirm')}
              </Button>
            )}
          </Group>
        </Stack>
      )}
    </Modal>
  );
}
