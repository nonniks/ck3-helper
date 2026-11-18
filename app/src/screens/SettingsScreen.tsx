import {
  Accordion,
  Autocomplete,
  Button,
  Group,
  Select,
  Stack,
  Switch,
  TextInput,
  Title,
} from '@mantine/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { open } from '@tauri-apps/plugin-dialog';
import { pluginCall } from '../plugins';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { Settings } from '../api/bindings/Settings';
import { getSetup, saveSettings } from '../api/client';
import { queryKeys, useAnalysisRunning, useAnalyze } from '../api/queries';
import { RulesEditor } from '../components/RulesEditor';
import { ErrorAlert } from '../components/ErrorAlert';
import { detectLanguage } from '../i18n';

const analysisLanguages = [
  'l_english',
  'l_french',
  'l_german',
  'l_spanish',
  'l_russian',
  'l_korean',
  'l_simp_chinese',
];

const emptyToNull = (value: string) => (value.trim() === '' ? null : value);

function parseNames(text: string): string[] | null {
  const names = text
    .split(',')
    .map((n) => n.trim())
    .filter((n) => n !== '');
  return names.length === 0 ? null : names;
}

type DirField = 'userDir' | 'gameDir';

export function SettingsScreen() {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const analyze = useAnalyze();
  const { running } = useAnalysisRunning();
  const setup = useQuery({ queryKey: queryKeys.setup, queryFn: getSetup });
  const analysis = useQuery<AnalysisView>({
    queryKey: queryKeys.summary,
    queryFn: () => Promise.reject(new Error('summary is filled by analyze')),
    enabled: false,
  }).data;
  const [draft, setDraft] = useState<Settings | null>(null);
  const [namesText, setNamesText] = useState<string | null>(null);

  const saved = setup.data?.settings;
  const current = draft ?? saved;

  const save = useMutation({
    mutationFn: saveSettings,
    onSuccess: (view, next) => {
      const before = saved;
      queryClient.setQueryData(queryKeys.setup, view);
      void queryClient.invalidateQueries({ queryKey: queryKeys.setup });
      setDraft(null);
      setNamesText(null);
      if (next.uiLang !== before?.uiLang) {
        void i18n.changeLanguage(detectLanguage(view.settings.uiLang, navigator.language));
      }
      if (next.userDir !== before?.userDir) {
        analyze.mutate({ playsetId: null });
      } else if (next.analysisLang !== before?.analysisLang || next.gameDir !== before?.gameDir) {
        analyze.mutate({ playsetId: analysis?.playset.id ?? null });
      }
    },
  });

  if (!setup.data || !current) {
    return setup.isError ? <ErrorAlert error={setup.error} /> : null;
  }
  const view = setup.data;
  const edit = (patch: Partial<Settings>) => setDraft({ ...current, ...patch });
  const names = namesText ?? (current.launcherProcessNames ?? []).join(', ');

  const pick = async (field: DirField) => {
    const dir = await pluginCall(() => open({ directory: true }));
    if (typeof dir === 'string') edit({ [field]: dir });
  };

  const dirField = (field: DirField, label: string) => (
    <Group align="flex-end">
      <TextInput
        style={{ flex: 1 }}
        label={label}
        value={current[field] ?? ''}
        placeholder={view[field] ?? t('settings.notFound')}
        description={
          current[field] === null
            ? t('settings.usingDefault', { path: view[field] ?? t('settings.notFound') })
            : undefined
        }
        onChange={(e) => edit({ [field]: emptyToNull(e.currentTarget.value) })}
      />
      <Button variant="default" onClick={() => void pick(field)}>
        {t('settings.pick')}
      </Button>
      <Button variant="default" onClick={() => edit({ [field]: null })}>
        {t('settings.useDefault')}
      </Button>
    </Group>
  );

  return (
    <Stack>
      <Title order={2}>{t('screens.settings')}</Title>
      {dirField('userDir', t('settings.userDir'))}
      {dirField('gameDir', t('settings.gameDir'))}
      <Autocomplete
        label={t('settings.analysisLang')}
        description={
          current.analysisLang === null
            ? t('settings.analysisLangDefault', { lang: view.gameLanguage ?? 'l_english' })
            : undefined
        }
        data={analysisLanguages}
        value={current.analysisLang ?? ''}
        onChange={(v) => edit({ analysisLang: emptyToNull(v) })}
      />
      <Select
        label={t('settings.uiLang')}
        allowDeselect={false}
        data={[
          { value: 'system', label: t('settings.uiLangSystem') },
          { value: 'ru', label: 'Русский' },
          { value: 'en', label: 'English' },
        ]}
        value={current.uiLang ?? 'system'}
        onChange={(v) => edit({ uiLang: v === 'ru' || v === 'en' ? v : null })}
      />
      <Switch
        label={t('settings.checkUpdates')}
        checked={current.checkUpdates}
        onChange={(e) => edit({ checkUpdates: e.currentTarget.checked })}
      />
      <Accordion variant="contained">
        <Accordion.Item value="advanced">
          <Accordion.Control>{t('settings.advanced')}</Accordion.Control>
          <Accordion.Panel>
            <Group align="flex-end">
              <TextInput
                style={{ flex: 1 }}
                label={t('settings.launcherProcessNames')}
                description={t('settings.launcherProcessNamesHint')}
                value={names}
                onChange={(e) => setNamesText(e.currentTarget.value)}
              />
              <Button variant="default" onClick={() => setNamesText('')}>
                {t('settings.useDefault')}
              </Button>
            </Group>
          </Accordion.Panel>
        </Accordion.Item>
      </Accordion>
      {save.isError && <ErrorAlert error={save.error} />}
      <Group>
        <Button
          loading={save.isPending}
          disabled={running}
          onClick={() => save.mutate({ ...current, launcherProcessNames: parseNames(names) })}
        >
          {t('settings.save')}
        </Button>
      </Group>
      <RulesEditor />
    </Stack>
  );
}
