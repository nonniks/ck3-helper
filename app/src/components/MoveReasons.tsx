import { List } from '@mantine/core';
import { useTranslation } from 'react-i18next';
import type { MoveReasonView } from '../api/bindings/MoveReasonView';

export function MoveReasons({ reasons }: { reasons: MoveReasonView[] }) {
  const { t } = useTranslation();
  const text = (r: MoveReasonView): string => {
    switch (r.kind) {
      case 'dependency':
        return t('order.reason.dependency', { on: r.on });
      case 'requiredBy':
        return t('order.reason.requiredBy', { by: r.by });
      case 'userRule':
        return (
          t(r.before ? 'order.reason.ruleBefore' : 'order.reason.ruleAfter', { other: r.other }) +
          (r.note ? t('order.reason.note', { note: r.note }) : '')
        );
    }
  };
  return (
    <List size="xs" c="dimmed">
      {reasons.map((r, i) => (
        <List.Item key={i}>{text(r)}</List.Item>
      ))}
    </List>
  );
}
