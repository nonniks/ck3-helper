import {
  Alert,
  Badge,
  Button,
  Group,
  Modal,
  Select,
  Stack,
  Text,
  TextInput,
  Title,
} from '@mantine/core';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AnalysisView } from '../api/bindings/AnalysisView';
import type { RuleView } from '../api/bindings/RuleView';
import { addRule, removeRule, rules, summary } from '../api/client';
import { queryKeys, useAnalysisRunning } from '../api/queries';
import { errorMessage } from '../errors';

export function RulesEditor() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { running } = useAnalysisRunning();
  const [before, setBefore] = useState<string | null>(null);
  const [after, setAfter] = useState<string | null>(null);
  const [note, setNote] = useState('');
  const [removing, setRemoving] = useState<RuleView | null>(null);

  const rulesQuery = useQuery({ queryKey: queryKeys.rules, queryFn: rules });
  const analysis = useQuery<AnalysisView>({
    queryKey: queryKeys.summary,
    queryFn: () => Promise.reject(new Error('summary is filled by analyze')),
    enabled: false,
  }).data;

  const refresh = async () => {
    queryClient.setQueryData(queryKeys.summary, await summary());
    for (const key of ['pair', 'search']) void queryClient.resetQueries({ queryKey: [key] });
    for (const key of ['rules', 'proposal']) {
      void queryClient.invalidateQueries({ queryKey: [key] });
    }
  };

  const add = useMutation({
    mutationFn: async (rule: { before: string; after: string; note: string }) => {
      await addRule(rule.before, rule.after, rule.note || null);
      await refresh();
    },
    onSuccess: () => {
      setBefore(null);
      setAfter(null);
      setNote('');
    },
  });
  const remove = useMutation({
    mutationFn: async (index: number) => {
      await removeRule(index);
      await refresh();
    },
    onSuccess: () => setRemoving(null),
  });

  const view = rulesQuery.data;
  const broken = view?.error != null;
  const names = [...new Set(analysis?.mods.map((m) => m.name))];
  const canEdit = view !== undefined && !broken && !running;
  const canAdd =
    canEdit && names.length > 0 && before !== null && after !== null && before !== after;

  return (
    <Stack>
      <Title order={3}>{t('rules.title')}</Title>
      {rulesQuery.isError && <Alert color="red">{errorMessage(t, rulesQuery.error)}</Alert>}
      {view && broken && (
        <Alert color="red">{t('rules.brokenFile', { path: view.path, detail: view.error })}</Alert>
      )}
      {view && !broken && view.rules.length === 0 && <Text size="sm">{t('rules.empty')}</Text>}
      {view?.rules.map((rule) => (
        <Group key={rule.index} justify="space-between" wrap="nowrap">
          <Group gap="xs">
            <Text>{t('rules.pair', { before: rule.before, after: rule.after })}</Text>
            {rule.note && (
              <Text size="sm" c="dimmed">
                {rule.note}
              </Text>
            )}
            {!rule.applied && (
              <Badge color="yellow" variant="light">
                {t('rules.unapplied')}
              </Badge>
            )}
          </Group>
          <Button
            size="compact-xs"
            variant="light"
            color="red"
            disabled={!canEdit}
            aria-label={t('rules.remove', { before: rule.before, after: rule.after })}
            onClick={() => {
              remove.reset();
              setRemoving(rule);
            }}
          >
            ×
          </Button>
        </Group>
      ))}
      {analysis === undefined && <Text size="sm">{t('rules.needAnalysis')}</Text>}
      <Group align="flex-end">
        <Select
          label={t('rules.before')}
          data={names}
          value={before}
          onChange={setBefore}
          searchable
          disabled={!canEdit || names.length === 0}
        />
        <Select
          label={t('rules.after')}
          data={names}
          value={after}
          onChange={setAfter}
          searchable
          disabled={!canEdit || names.length === 0}
        />
        <TextInput
          label={t('rules.note')}
          value={note}
          onChange={(e) => setNote(e.currentTarget.value)}
          disabled={!canEdit || names.length === 0}
        />
        <Button
          disabled={!canAdd}
          loading={add.isPending}
          onClick={() => before && after && add.mutate({ before, after, note })}
        >
          {t('rules.add')}
        </Button>
      </Group>
      {add.isError && <Alert color="red">{errorMessage(t, add.error)}</Alert>}
      <Modal
        opened={removing !== null}
        onClose={() => setRemoving(null)}
        title={t('rules.removeTitle')}
      >
        <Stack>
          <Text size="sm">
            {removing &&
              t('rules.removeConfirm', { before: removing.before, after: removing.after })}
          </Text>
          {remove.isError && <Alert color="red">{errorMessage(t, remove.error)}</Alert>}
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setRemoving(null)}>
              {t('confirm.cancel')}
            </Button>
            <Button
              color="red"
              loading={remove.isPending}
              disabled={running}
              onClick={() => removing && remove.mutate(removing.index)}
            >
              {t('confirm.ok')}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
