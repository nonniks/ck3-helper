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
  playset: {
    list: 'Playsets',
    activeInLauncher: 'Active playset in the launcher',
    summaryMods: 'Mods',
    summaryEnabled: 'Enabled',
    summaryMissing: 'Not found on disk',
    summaryDepIssues: 'Dependency issues',
    summaryCompatch: 'Compatch',
    colPosition: '#',
    colName: 'Mod',
    colStatus: 'Status',
    colActions: 'Actions',
    missing: 'not found on disk',
    depMissing: 'missing dependency {{name}}',
    depAmbiguous: 'ambiguous dependency {{name}}',
    compatch: 'compatch',
    replacePath: 'replaces folder {{path}}',
    disabled: 'disabled',
    warnings: 'Index warnings: {{count}}',
    openFolder: 'Open folder',
    warningsTitle: 'Index warnings: {{name}}',
    warningsEmpty: 'No warnings.',
  },
  counts: {
    files_one: '{{count}} file',
    files_other: '{{count}} files',
    objects_one: '{{count}} object',
    objects_other: '{{count}} objects',
    loc_one: '{{count}} localization line',
    loc_other: '{{count}} localization lines',
  },
  source: {
    vanilla: 'Vanilla',
  },
  verdict: {
    suspicious: 'suspicious',
    intended: 'intended',
    vanilla: 'vanilla',
  },
  level: {
    file: 'file',
    object: 'object',
    loc: 'localization',
  },
  conflicts: {
    vanilla: 'Include vanilla',
    vanillaNoGameDir: 'Needs the game folder: set it in settings.',
    verdicts: 'Verdicts',
    levels: 'Level',
    tabPairs: 'Mod pairs',
    tabAll: 'All conflicts',
    search: 'Search by path or key',
    pair: '{{winner}} → {{loser}}',
    noPairs: 'No mod pairs with these verdicts.',
    noConflicts: 'No conflicts found.',
    total: 'Conflicts: {{count}}',
    winner: 'Winner',
    loser: 'Overridden',
  },
  explain: {
    fileLower:
      'The file of mod {{winner}} is loaded: it is lower in the playset; the copies of the other mods are not loaded',
    objectLater:
      'The game reads the database files in path order and takes the last definition: {{winnerFile}} comes after {{loserFile}}. The mod order does not decide here — a compatch is needed',
    locEarlier: 'Among regular localization files the first definition in path order wins',
    locReplace: 'A file from the replace folder overrides regular ones',
    vanillaWins: "The mod's change does not take effect in the game",
  },
  markIntended: {
    button: 'Intended',
    cycle:
      '{{loser}} already depends on {{winner}} (rules included): a rule “{{loser}} before {{winner}}” would create a cycle',
  },
  confirm: {
    markIntendedTitle: 'Mark as intended?',
    markIntended:
      '{{loser}} will load before {{winner}}; if {{winner}} is above {{loser}} now, the “Order” screen will show a move',
    ok: 'Confirm',
    cancel: 'Cancel',
  },
  rules: {
    intendedNote: 'intended: {{winner}} overrides {{loser}}',
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
