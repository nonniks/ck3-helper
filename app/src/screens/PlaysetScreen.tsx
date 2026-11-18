import { Group, NavLink, Paper, SimpleGrid, Stack, Text, Title } from '@mantine/core';
import { IconCircleCheck } from '@tabler/icons-react';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { ModView } from '../api/bindings/ModView';
import { listPlaysets } from '../api/client';
import { queryKeys, useAnalysisRunning, useAnalyze } from '../api/queries';
import { ModTable } from '../components/ModTable';
import { WarningsDrawer } from '../components/WarningsDrawer';
import { ErrorAlert } from '../components/ErrorAlert';

export function PlaysetScreen() {
  const { t } = useTranslation();
  const [warningsMod, setWarningsMod] = useState<ModView | null>(null);
  const analyze = useAnalyze();
  const { running } = useAnalysisRunning();
  const playsets = useQuery({ queryKey: queryKeys.playsets, queryFn: listPlaysets });
  const summary = useQuery<AnalysisView>({
    queryKey: queryKeys.summary,
    queryFn: () => Promise.reject(new Error('summary is filled by analyze')),
    enabled: false,
  });
  const analysis = summary.data;

  const stats =
    analysis &&
    ([
      [t('playset.summaryMods'), analysis.mods.length],
      [t('playset.summaryEnabled'), analysis.mods.filter((m) => m.enabled).length],
      [t('playset.summaryMissing'), analysis.mods.filter((m) => m.missing).length],
      [t('playset.summaryDepIssues'), analysis.mods.filter((m) => m.depIssues.length > 0).length],
      [t('playset.summaryCompatch'), analysis.mods.filter((m) => m.compatch).length],
    ] as const);

  return (
    <Stack>
      <Title order={2}>{t('screens.playset')}</Title>
      {playsets.isError && <ErrorAlert error={playsets.error} />}
      <div>
        <Text fw={500} size="sm">
          {t('playset.list')}
        </Text>
        {playsets.data?.map((p) => (
          <NavLink
            key={p.id}
            component="button"
            type="button"
            aria-current={analysis?.playset.id === p.id ? 'true' : undefined}
            label={p.name}
            active={analysis?.playset.id === p.id}
            disabled={running}
            aria-disabled={running || undefined}
            leftSection={
              p.active ? (
                <IconCircleCheck size={16} aria-label={t('playset.activeInLauncher')} />
              ) : undefined
            }
            onClick={() => {
              if (running) return;
              if (analysis?.playset.id !== p.id) analyze.mutate({ playsetId: p.id });
            }}
          />
        ))}
      </div>
      {analysis && stats && (
        <>
          <SimpleGrid cols={5}>
            {stats.map(([label, value]) => (
              <Paper key={label} withBorder p="xs">
                <Text size="xs" c="dimmed">
                  {label}
                </Text>
                <Group>
                  <Text fw={700}>{value}</Text>
                </Group>
              </Paper>
            ))}
          </SimpleGrid>
          <ModTable mods={analysis.mods} onShowWarnings={setWarningsMod} />
        </>
      )}
      <WarningsDrawer
        modId={warningsMod?.id ?? null}
        modName={warningsMod?.name ?? ''}
        onClose={() => setWarningsMod(null)}
      />
    </Stack>
  );
}
