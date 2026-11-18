import {
  Alert,
  Chip,
  Group,
  Stack,
  Switch,
  Tabs,
  Text,
  TextInput,
  Title,
  Tooltip,
  VisuallyHidden,
} from '@mantine/core';
import { useDebouncedValue } from '@mantine/hooks';
import { IconSearch } from '@tabler/icons-react';
import { useQuery } from '@tanstack/react-query';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { ConflictQuery } from '../api/bindings/ConflictQuery';
import type { LevelView } from '../api/bindings/LevelView';
import type { VerdictView } from '../api/bindings/VerdictView';
import { getSetup, searchConflicts } from '../api/client';
import { queryKeys, useAnalysisRunning, useSetVanilla } from '../api/queries';
import { ConflictList } from '../components/ConflictList';
import { PairList } from '../components/PairList';
import { errorMessage } from '../errors';

const VERDICTS: VerdictView[] = ['suspicious', 'intended', 'vanilla'];
const LEVELS: LevelView[] = ['file', 'object', 'loc'];

function VanillaSwitch({ on }: { on: boolean }) {
  const { t } = useTranslation();
  const hintId = useId();
  const setup = useQuery({ queryKey: queryKeys.setup, queryFn: getSetup, retry: false });
  const vanilla = useSetVanilla();
  const { running } = useAnalysisRunning();
  const hasGame = Boolean(setup.data?.gameDir);
  return (
    <Stack gap="xs">
      <Tooltip label={t('conflicts.vanillaNoGameDir')} disabled={hasGame}>
        <div style={{ width: 'fit-content' }}>
          <Switch
            label={t('conflicts.vanilla')}
            checked={vanilla.isPending ? vanilla.variables : on}
            disabled={!hasGame || running}
            aria-describedby={hasGame ? undefined : hintId}
            onChange={(e) => vanilla.mutate(e.currentTarget.checked)}
          />
          {!hasGame && (
            <VisuallyHidden id={hintId}>{t('conflicts.vanillaNoGameDir')}</VisuallyHidden>
          )}
        </div>
      </Tooltip>
      {vanilla.isError && <Alert color="red">{errorMessage(t, vanilla.error)}</Alert>}
    </Stack>
  );
}

export function ConflictsScreen() {
  const { t } = useTranslation();
  const summary = useQuery<AnalysisView>({
    queryKey: queryKeys.summary,
    queryFn: () => Promise.reject(new Error('summary is filled by analyze')),
    enabled: false,
  });
  const [tab, setTab] = useState<string | null>('pairs');
  const [verdicts, setVerdicts] = useState<VerdictView[]>(['suspicious', 'intended']);
  const [levels, setLevels] = useState<LevelView[]>([]);
  const [text, setText] = useState('');
  const [debouncedText] = useDebouncedValue(text, 300);
  const [openPair, setOpenPair] = useState<string | null>(null);
  const analysis = summary.data;

  if (!analysis) return <Title order={2}>{t('screens.conflicts')}</Title>;

  const pairs =
    verdicts.length === 0
      ? analysis.pairs
      : analysis.pairs.filter((p) => verdicts.includes(p.verdict));
  const query: ConflictQuery = { text: debouncedText, levels, verdicts };

  return (
    <Stack>
      <Title order={2}>{t('screens.conflicts')}</Title>
      <Group justify="space-between" align="flex-start">
        <VanillaSwitch on={analysis.vanilla} />
        <Group gap="xs">
          <Text size="sm">{t('conflicts.verdicts')}</Text>
          <Chip.Group multiple value={verdicts} onChange={setVerdicts}>
            <Group gap={4} role="group" aria-label={t('conflicts.verdicts')}>
              {VERDICTS.map((v) => (
                <Chip key={v} value={v} size="xs">
                  {t(`verdict.${v}`)}
                </Chip>
              ))}
            </Group>
          </Chip.Group>
        </Group>
      </Group>
      <Tabs value={tab} onChange={setTab} keepMounted={false}>
        <Tabs.List>
          <Tabs.Tab value="pairs">{t('conflicts.tabPairs')}</Tabs.Tab>
          <Tabs.Tab value="all">{t('conflicts.tabAll')}</Tabs.Tab>
        </Tabs.List>
        <Tabs.Panel value="pairs" pt="sm">
          <PairList pairs={pairs} openKey={openPair} onToggle={setOpenPair} />
        </Tabs.Panel>
        <Tabs.Panel value="all" pt="sm">
          <Stack gap="sm">
            <Group>
              <TextInput
                w={360}
                leftSection={<IconSearch size={16} />}
                placeholder={t('conflicts.search')}
                aria-label={t('conflicts.search')}
                value={text}
                onChange={(e) => setText(e.currentTarget.value)}
              />
              <Text size="sm">{t('conflicts.levels')}</Text>
              <Chip.Group multiple value={levels} onChange={setLevels}>
                <Group gap={4} role="group" aria-label={t('conflicts.levels')}>
                  {LEVELS.map((l) => (
                    <Chip key={l} value={l} size="xs">
                      {t(`level.${l}`)}
                    </Chip>
                  ))}
                </Group>
              </Chip.Group>
            </Group>
            <ConflictList
              key={JSON.stringify(query)}
              queryKey={queryKeys.search(query)}
              fetchPage={(offset, limit) => searchConflicts(query, offset, limit)}
              maxHeight="calc(100vh - 300px)"
            />
          </Stack>
        </Tabs.Panel>
      </Tabs>
    </Stack>
  );
}
