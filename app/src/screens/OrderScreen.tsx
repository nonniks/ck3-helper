import { Button, List, Loader, Stack, Text, Title } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { ApplyView } from '../api/bindings/ApplyView';
import type { ProposalView } from '../api/bindings/ProposalView';
import { launcherStatus, propose } from '../api/client';
import { queryKeys, useAnalysisRunning } from '../api/queries';
import { ApplyDialog } from '../components/ApplyDialog';
import { OrderTable } from '../components/OrderTable';
import { ErrorAlert } from '../components/ErrorAlert';

function Details({ proposal }: { proposal: ProposalView }) {
  const { t } = useTranslation();
  return (
    <>
      {proposal.winnerChanges.length > 0 && (
        <Stack gap="xs">
          <Title order={4}>{t('order.winnerChangesTitle')}</Title>
          <List size="sm">
            {proposal.winnerChanges.map((w) => (
              <List.Item key={`${w.level}:${w.target}`}>
                {t('order.winnerChange', {
                  target: w.target,
                  before: w.before.name || t('source.vanilla'),
                  after: w.after.name || t('source.vanilla'),
                })}
              </List.Item>
            ))}
          </List>
        </Stack>
      )}
      {proposal.unresolvedRules.length > 0 && (
        <Stack gap="xs">
          <Title order={4}>{t('order.unresolvedTitle')}</Title>
          <List size="sm">
            {proposal.unresolvedRules.map((r) => (
              <List.Item key={r.index}>
                {t('order.unresolvedRule', { before: r.before, after: r.after })}
                {r.note ? t('order.reason.note', { note: r.note }) : ''}
              </List.Item>
            ))}
          </List>
        </Stack>
      )}
    </>
  );
}

export function OrderScreen() {
  const { t } = useTranslation();
  const [dialog, setDialog] = useState(false);
  const { running: analysisRunning } = useAnalysisRunning();
  const summary = useQuery<AnalysisView>({
    queryKey: queryKeys.summary,
    queryFn: () => Promise.reject(new Error('summary is filled by analyze')),
    enabled: false,
  });
  const applied = useQuery<ApplyView | null>({
    queryKey: queryKeys.applied,
    queryFn: () => null,
    enabled: false,
    // Only written by a successful save; a garbage-collected entry would offer the save again.
    gcTime: Infinity,
  }).data;
  // After a save the same proposal must not be offered again (Rust spent it).
  const proposal = useQuery({
    queryKey: queryKeys.proposal,
    queryFn: propose,
    retry: false,
    enabled: !applied,
  });
  const launcher = useQuery({
    queryKey: queryKeys.launcher,
    queryFn: launcherStatus,
    refetchInterval: 2000,
    retry: false,
  });

  const body = () => {
    if (applied)
      return (
        <Stack gap="xs">
          <Text fw={600}>{t('apply.doneTitle', { name: applied.newPlaysetName })}</Text>
          <Text size="sm">{t('apply.pickInLauncher')}</Text>
        </Stack>
      );
    if (proposal.isPending) return <Loader size="sm" />;
    if (proposal.isError) {
      const cycle =
        typeof proposal.error === 'object' &&
        proposal.error !== null &&
        (proposal.error as { code?: string }).code === 'cycle';
      return (
        <ErrorAlert error={proposal.error} color={cycle ? 'yellow' : 'red'}>
          {cycle && <Text size="sm">{t('order.cycleHint')}</Text>}
        </ErrorAlert>
      );
    }

    const data = proposal.data;
    if (data.alreadySorted)
      return (
        <>
          <Text>{t('order.alreadySorted')}</Text>
          <Details proposal={data} />
        </>
      );
    const status = launcher.data;
    let blocked: string | null = null;
    if (!status) blocked = '';
    else if (status.state === 'running')
      blocked = t('errors.launcherRunning', { items: status.processes.join(', ') });
    else if (status.state === 'unknown') blocked = t('apply.processListUnavailable');
    return (
      <>
        <OrderTable rows={data.rows} moves={data.moves} />
        <Details proposal={data} />
        <Stack gap="xs" align="flex-start">
          <Button disabled={blocked !== null || analysisRunning} onClick={() => setDialog(true)}>
            {t('apply.save')}
          </Button>
          {blocked && (
            <Text size="sm" c="red">
              {blocked}
            </Text>
          )}
        </Stack>
      </>
    );
  };

  return (
    <Stack>
      <Title order={2}>{t('screens.order')}</Title>
      {body()}
      {/* Outside the proposal states: a refetch must not take the result of a save with it. */}
      <ApplyDialog
        opened={dialog}
        onClose={() => setDialog(false)}
        playsetId={summary.data?.playset.id ?? null}
      />
    </Stack>
  );
}
