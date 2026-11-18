import { Button, Group, Modal, Stack, Text, Tooltip, VisuallyHidden } from '@mantine/core';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { PairView } from '../api/bindings/PairView';
import { markIntended } from '../api/client';
import { queryKeys, refreshAfterRuleEdit, useAnalysisRunning } from '../api/queries';
import { ErrorAlert } from './ErrorAlert';

/** «Задумано» for a suspicious pair of two mods: a rule «loser before winner» after a confirmation. */
export function MarkIntendedButton({ pair }: { pair: PairView }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { running } = useAnalysisRunning();
  const [confirming, setConfirming] = useState(false);
  const hintId = useId();
  const names = { winner: pair.winner.name, loser: pair.loser.name };

  const mark = useMutation({
    mutationFn: () =>
      markIntended(
        pair.winner.id as string,
        pair.loser.id as string,
        t('rules.intendedNote', names),
      ),
    onSuccess: (view) => {
      queryClient.setQueryData(queryKeys.rules, view);
      setConfirming(false);
      void refreshAfterRuleEdit(queryClient);
    },
  });

  if (pair.mark === 'notApplicable') return null;

  if (pair.mark === 'cycle') {
    const hint = t('markIntended.cycle', names);
    return (
      <Tooltip label={hint} multiline maw={320}>
        <span>
          <Button
            size="compact-xs"
            variant="light"
            disabled
            aria-describedby={hintId}
            style={{ pointerEvents: 'none' }}
          >
            {t('markIntended.button')}
          </Button>
          <VisuallyHidden id={hintId}>{hint}</VisuallyHidden>
        </span>
      </Tooltip>
    );
  }

  return (
    <>
      <Button
        size="compact-xs"
        variant="light"
        disabled={running}
        onClick={() => {
          mark.reset();
          setConfirming(true);
        }}
      >
        {t('markIntended.button')}
      </Button>
      <Modal
        opened={confirming}
        onClose={() => setConfirming(false)}
        title={t('confirm.markIntendedTitle')}
        closeButtonProps={{ 'aria-label': t('common.close') }}
      >
        <Stack>
          <Text size="sm">{t('confirm.markIntended', names)}</Text>
          {mark.isError && <ErrorAlert error={mark.error} />}
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setConfirming(false)}>
              {t('confirm.cancel')}
            </Button>
            <Button loading={mark.isPending} disabled={running} onClick={() => mark.mutate()}>
              {t('confirm.ok')}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </>
  );
}
