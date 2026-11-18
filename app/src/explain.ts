import type { TFunction } from 'i18next';
import type { ConflictView } from './api/bindings/ConflictView';
import type { SourceView } from './api/bindings/SourceView';

export function sourceName(t: TFunction, source: SourceView): string {
  return source.kind === 'vanilla' ? t('source.vanilla') : source.name;
}

function reasonLine(t: TFunction, c: ConflictView): string {
  switch (c.reason) {
    case 'lowerInPlayset':
      return t('explain.fileLower', { winner: sourceName(t, c.winner.source) });
    case 'laterFilePath':
      return t('explain.objectLater', { winnerFile: c.winner.file, loserFile: c.losers[0].file });
    case 'earlierFilePath':
      return t('explain.locEarlier');
    case 'replaceFolder':
      return t('explain.locReplace');
  }
}

/** Why the winner won (spec §4.8.2 «Объяснения»); one string per line. */
export function explain(t: TFunction, c: ConflictView): string[] {
  const lines = [reasonLine(t, c)];
  if (c.winner.source.kind === 'vanilla') lines.push(t('explain.vanillaWins'));
  return lines;
}
