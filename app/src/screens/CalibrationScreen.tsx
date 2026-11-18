import {
  Accordion,
  Alert,
  Button,
  Progress as ProgressBar,
  Stack,
  Text,
  Title,
} from '@mantine/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CalibrationView } from '../api/bindings/CalibrationView';
import { calibrate, getSetup, isAppError, type Progress } from '../api/client';
import { queryKeys, useAnalysisRunning } from '../api/queries';
import { ErrorAlert } from '../components/ErrorAlert';

const LIMIT = 200;

function Lines({ lines }: { lines: string[] }) {
  const { t } = useTranslation();
  return (
    <Stack gap={2}>
      {lines.slice(0, LIMIT).map((line, i) => (
        <Text key={i} size="sm">
          {line}
        </Text>
      ))}
      {lines.length > LIMIT && (
        <Text size="sm" c="dimmed">
          {t('calibration.more', { count: lines.length - LIMIT })}
        </Text>
      )}
    </Stack>
  );
}

export function CalibrationScreen({ onOpenSettings }: { onOpenSettings: () => void }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { running: analyzing } = useAnalysisRunning();
  const [progress, setProgress] = useState<Progress | null>(null);

  const setup = useQuery({ queryKey: queryKeys.setup, queryFn: getSetup });
  const result = useQuery<CalibrationView>({
    queryKey: queryKeys.calibration,
    queryFn: () => Promise.reject(new Error('calibration is filled by calibrate')),
    enabled: false,
  }).data;
  const run = useMutation({
    mutationFn: () => {
      setProgress(null);
      return calibrate(setProgress);
    },
    onSuccess: (data) => queryClient.setQueryData(queryKeys.calibration, data),
  });

  if (!setup.data) {
    return setup.isError ? <ErrorAlert error={setup.error} /> : null;
  }

  const progressText =
    progress === null
      ? t('progress.starting')
      : progress.stage === 'mods'
        ? t('progress.mods', { done: progress.done, total: progress.total })
        : t(`progress.${progress.stage}`);
  const percent =
    progress?.stage === 'mods' && progress.total > 0 ? (progress.done / progress.total) * 100 : 100;
  const list = (names: string[]) => (names.length === 0 ? t('calibration.none') : names.join(', '));

  return (
    <Stack>
      <Title order={2}>{t('screens.calibration')}</Title>
      {setup.data.gameDir === null ? (
        <Stack align="flex-start">
          <Text>{t('calibration.noGameDir')}</Text>
          <Button onClick={onOpenSettings}>{t('calibration.openSettings')}</Button>
        </Stack>
      ) : (
        <Button
          style={{ alignSelf: 'flex-start' }}
          loading={run.isPending}
          disabled={analyzing}
          onClick={() => run.mutate()}
        >
          {t('calibration.run')}
        </Button>
      )}
      {run.isPending && (
        <div>
          <Text size="sm">{progressText}</Text>
          <ProgressBar value={percent} animated />
        </div>
      )}
      {run.isError &&
        (isAppError(run.error) && run.error.code === 'logNotFound' ? (
          <Alert color="yellow">{t('calibration.noLog')}</Alert>
        ) : (
          <ErrorAlert error={run.error} />
        ))}
      {result && setup.data.gameDir !== null && (
        <Stack>
          <Text fw={600}>
            {t('calibration.summary', {
              matched: result.matched,
              mismatched: result.mismatched.length,
            })}
          </Text>
          {result.truncated && <Alert color="yellow">{t('calibration.truncated')}</Alert>}
          {result.lastRun && (
            <Alert color="yellow">
              {t('calibration.lastRun', {
                missing: list(result.lastRun.notInLastRun),
                extra: list(result.lastRun.onlyInLastRun),
              })}
            </Alert>
          )}
          <Accordion variant="contained" multiple>
            {result.mismatched.length > 0 && (
              <Accordion.Item value="mismatched">
                <Accordion.Control>
                  {t('calibration.mismatches', { count: result.mismatched.length })}
                </Accordion.Control>
                <Accordion.Panel>
                  <Lines lines={result.mismatched.map((m) => t('calibration.mismatch', m))} />
                </Accordion.Panel>
              </Accordion.Item>
            )}
            {result.logUnknown.length > 0 && (
              <Accordion.Item value="logUnknown">
                <Accordion.Control>
                  {t('calibration.logUnknown', { count: result.logUnknown.length })}
                </Accordion.Control>
                <Accordion.Panel>
                  <Lines lines={result.logUnknown.map((d) => t('calibration.databaseCount', d))} />
                </Accordion.Panel>
              </Accordion.Item>
            )}
            {result.loggedSingleDefinition.length > 0 && (
              <Accordion.Item value="single">
                <Accordion.Control>
                  {t('calibration.singleDefinition', {
                    count: result.loggedSingleDefinition.length,
                  })}
                </Accordion.Control>
                <Accordion.Panel>
                  <Lines
                    lines={result.loggedSingleDefinition.map((d) =>
                      t('calibration.databaseCount', d),
                    )}
                  />
                </Accordion.Panel>
              </Accordion.Item>
            )}
            {result.predictedNotLogged.length > 0 && (
              <Accordion.Item value="predicted">
                <Accordion.Control>
                  {t('calibration.predictedNotLogged', {
                    count: result.predictedNotLogged.length,
                  })}
                </Accordion.Control>
                <Accordion.Panel>
                  <Lines lines={result.predictedNotLogged.map((o) => t('calibration.object', o))} />
                </Accordion.Panel>
              </Accordion.Item>
            )}
          </Accordion>
        </Stack>
      )}
    </Stack>
  );
}
