//! The update check: the only network code of the app (decisions Q17, Q29). It runs only when
//! enabled, at most once a day, with a 5 s timeout, and sends nothing but the app version.

use crate::dto::UpdateView;
use crate::settings::Settings;
use std::time::Duration;

const DAY: i64 = 86_400;

/// `https://github.com/<owner>/<repo>` (a trailing `/` or `.git` is fine) to the API URL of the
/// latest release; anything else gives `None`.
pub fn releases_api(repository: &str) -> Option<String> {
    let path = repository.strip_prefix("https://github.com/")?;
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return None;
    }
    Some(format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/latest"
    ))
}

/// `tag` (with or without a leading `v`) is a higher semver than `current`; unparseable is `false`.
pub fn newer_release(current: &str, tag: &str) -> bool {
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    match (semver::Version::parse(current), semver::Version::parse(tag)) {
        (Ok(current), Ok(tag)) => tag > current,
        _ => false,
    }
}

/// Updates are enabled and the last check was at least a day ago (or never).
pub fn due(settings: &Settings, now: i64) -> bool {
    settings.check_updates
        && settings
            .update_checked_at
            .is_none_or(|t| now.saturating_sub(t) >= DAY)
}

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

/// Asks the releases API; `Some` only for a release newer than `current`.
pub fn fetch_latest(api: &str, current: &str) -> Result<Option<UpdateView>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();
    let body = agent
        .get(api)
        .header("User-Agent", &format!("ck3-helper/{current}"))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let release: Release = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(
        newer_release(current, &release.tag_name).then(|| UpdateView {
            version: release.tag_name.trim_start_matches('v').to_string(),
            url: release.html_url,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_api_from_repository_url() {
        assert_eq!(
            releases_api("https://github.com/o/ck3-helper.git").as_deref(),
            Some("https://api.github.com/repos/o/ck3-helper/releases/latest")
        );
        assert_eq!(
            releases_api("https://github.com/o/ck3-helper/").as_deref(),
            Some("https://api.github.com/repos/o/ck3-helper/releases/latest")
        );
        assert_eq!(releases_api(""), None);
        assert_eq!(releases_api("https://gitlab.com/o/r"), None);
    }

    #[test]
    fn newer_release_compares_semver() {
        assert!(newer_release("0.1.0", "v0.2.0"));
        assert!(!newer_release("0.2.0", "v0.2.0"));
        assert!(!newer_release("0.1.0", "garbage"));
    }

    #[test]
    fn due_once_a_day_when_enabled() {
        let mut s = Settings::default();
        assert!(due(&s, 100_000));
        s.update_checked_at = Some(100_000);
        assert!(!due(&s, 100_000 + 86_399));
        assert!(due(&s, 100_000 + 86_400));
        s.check_updates = false;
        assert!(!due(&s, 999_999));
        s.check_updates = true;
        s.update_checked_at = Some(i64::MIN);
        assert!(due(&s, 100_000));
    }
}
