import { Alert, Button, Group, Loader, Modal, Stack, Text } from '@mantine/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { useTranslation } from 'react-i18next';
import { apply, applyPreview, isAppError } from '../api/client';
import { queryKeys, useAnalyze } from '../api/queries';
import { errorMessage } from '../errors';

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
    onSuccess: () => queryClient.invalidateQueries({ queryKey: queryKeys.proposal }),
  });
  const close = () => {
    save.reset();
    onClose();
  };
  const changed = isAppError(save.error) && save.error.code === 'playsetChanged';
  const done = save.data;

  return (
    <Modal opened={opened} onClose={close} title={t('apply.dialogTitle')}>
      {done ? (
        <Stack>
          <Text fw={600}>{t('apply.doneTitle', { name: done.newPlaysetName })}</Text>
          <Text size="sm">{t('apply.pickInLauncher')}</Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => void revealItemInDir(done.backupPath)}>
              {t('apply.revealBackup')}
            </Button>
            <Button onClick={close}>{t('apply.close')}</Button>
          </Group>
        </Stack>
      ) : (
        <Stack>
          {preview.isPending && <Loader size="sm" />}
          {preview.isError && <Alert color="red">{errorMessage(t, preview.error)}</Alert>}
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
          {save.isError && <Alert color="red">{errorMessage(t, save.error)}</Alert>}
          <Group justify="flex-end">
            <Button variant="default" onClick={close}>
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
