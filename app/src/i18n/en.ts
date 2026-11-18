export const en = {
  nav: {
    playset: 'Playset',
    conflicts: 'Conflicts',
    order: 'Order',
    calibration: 'Calibration',
    settings: 'Settings',
  },
  screens: {
    playset: 'Playset',
    conflicts: 'Conflicts',
    order: 'Order',
    calibration: 'Check against the game',
    settings: 'Settings',
  },
  header: {
    refresh: 'Refresh',
    noPlayset: 'No playset selected',
  },
  setup: {
    noLauncherDb:
      'The Paradox Launcher database was not found in the default folder. Set the CK3 user folder in settings.',
  },
  progress: {
    mods: 'Mod {{done}} of {{total}}',
    vanilla: 'Indexing the game',
    conflicts: 'Finding conflicts',
    starting: 'Starting analysis',
  },
  banner: {
    settingsError: 'Settings could not be read, defaults are used: {{detail}}',
  },
  update: {
    available: 'Version {{version}} is available',
    open: 'Open the release page',
  },
  counts: {
    objects_one: '{{count}} object',
    objects_other: '{{count}} objects',
  },
  errors: {
    unknown: 'Something went wrong.',
    noUserDir: 'The CK3 user folder was not found. Set it in settings.',
    launcherDbNotFound: 'The Paradox Launcher database was not found: {{items}}',
    schema: 'The launcher database schema is not supported.',
    playsetNotFound: 'Playset not found.',
    noActivePlayset: 'The launcher has no active playset.',
    launcherRunning: 'Close Paradox Launcher: {{items}}',
    processListUnavailable:
      'The process list is unavailable, so it cannot be checked that the launcher is closed.',
    playsetChanged:
      'The playset changed in the launcher after the analysis. Run the analysis again.',
    nothingToApply: 'The order is already right, nothing to apply.',
    cycle:
      'The dependencies form a cycle: {{items}}. A rule against a dependency usually causes it.',
    notGameDir: 'This is not a CK3 game folder: {{items}}',
    logNotFound: 'The log logs/database_conflicts.log was not found.',
    rules: 'The rules file cannot be read: {{items}}',
    settings: 'The settings file cannot be read: {{items}}',
    notAnalyzed: 'The playset has not been analyzed yet.',
    io: 'I/O error: {{items}}',
    db: 'Launcher database error.',
  },
};
