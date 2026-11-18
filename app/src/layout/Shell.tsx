import { Alert, AppShell, NavLink, Progress as ProgressBar, Stack, Text } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import { checkUpdate, getSetup } from '../api/client';
import { queryKeys, useAnalysisRunning, useAnalyze } from '../api/queries';
import { errorMessage } from '../errors';
import { detectLanguage } from '../i18n';
import { CalibrationScreen } from '../screens/CalibrationScreen';
import { ConflictsScreen } from '../screens/ConflictsScreen';
import { OrderScreen } from '../screens/OrderScreen';
import { PlaysetScreen } from '../screens/PlaysetScreen';
import { SettingsScreen } from '../screens/SettingsScreen';
import { Header } from './Header';

const screens = ['playset', 'conflicts', 'order', 'calibration', 'settings'] as const;
type ScreenId = (typeof screens)[number];
const needsAnalysis: ScreenId[] = ['conflicts', 'order', 'calibration'];

const views: Record<ScreenId, ReactNode> = {
  playset: <PlaysetScreen />,
  conflicts: <ConflictsScreen />,
  order: <OrderScreen />,
  calibration: <CalibrationScreen />,
  settings: <SettingsScreen />,
};

export function Shell() {
  const { t, i18n } = useTranslation();
  const [chosen, setActive] = useState<ScreenId | null>(null);
  const analyze = useAnalyze();
  const { running: busy, progress } = useAnalysisRunning();
  const started = useRef(false);

  const setup = useQuery({ queryKey: queryKeys.setup, queryFn: getSetup, retry: false });
  const summary = useQuery<AnalysisView>({
    queryKey: queryKeys.summary,
    queryFn: () => Promise.reject(new Error('summary is filled by analyze')),
    enabled: false,
  });
  const update = useQuery({
    queryKey: ['update'],
    queryFn: () => checkUpdate().catch(() => null),
    staleTime: Infinity,
    retry: false,
  });

  const setupData = setup.data;
  const lang = detectLanguage(setupData?.settings.uiLang, navigator.language);
  const runAnalyze = analyze.mutate;
  useEffect(() => {
    if (setupData) void i18n.changeLanguage(lang);
  }, [setupData, lang, i18n]);
  useEffect(() => {
    if (!setupData || started.current) return;
    started.current = true;
    if (!setupData.launcherDbFound) return;
    runAnalyze({ playsetId: null });
  }, [setupData, runAnalyze]);

  const active: ScreenId =
    chosen ?? (setupData && !setupData.launcherDbFound ? 'settings' : 'playset');
  const analysis = summary.data;
  const progressText =
    progress === null
      ? t('progress.starting')
      : progress.stage === 'mods'
        ? t('progress.mods', { done: progress.done, total: progress.total })
        : t(`progress.${progress.stage}`);
  const percent =
    progress?.stage === 'mods' && progress.total > 0 ? (progress.done / progress.total) * 100 : 100;

  return (
    <AppShell navbar={{ width: 220, breakpoint: 0 }} padding="md">
      <AppShell.Navbar p="xs">
        {screens.map((screen) => (
          <NavLink
            key={screen}
            href={`#${screen}`}
            label={t(`nav.${screen}`)}
            active={active === screen}
            disabled={needsAnalysis.includes(screen) && analysis === undefined}
            onClick={(e) => {
              e.preventDefault();
              setActive(screen);
            }}
          />
        ))}
      </AppShell.Navbar>
      <AppShell.Main>
        <Header
          playsetName={analysis?.playset.name ?? null}
          busy={busy}
          canRefresh={analysis !== undefined}
          onRefresh={() =>
            analysis && runAnalyze({ playsetId: analysis.playset.id })
          }
          settingsError={setupData?.settingsError ?? null}
          update={update.data ?? null}
        />
        <Stack>
          {setupData && !setupData.launcherDbFound && (
            <Alert color="yellow">{t('setup.noLauncherDb')}</Alert>
          )}
          {setup.isError && <Alert color="red">{errorMessage(t, setup.error)}</Alert>}
          {analyze.isError && <Alert color="red">{errorMessage(t, analyze.error)}</Alert>}
          {busy && (
            <div>
              <Text size="sm">{progressText}</Text>
              <ProgressBar value={percent} animated />
            </div>
          )}
          {views[active]}
        </Stack>
      </AppShell.Main>
    </AppShell>
  );
}
