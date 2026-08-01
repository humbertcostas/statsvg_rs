use std::collections::HashMap;

use chrono::Datelike;
use futures::future::try_join_all;
use serde::Deserialize;

use crate::error::AppError;

const STATS_QUERY: &str = r#"
query($login: String!) {
  user(login: $login) {
    name login bio company location avatarUrl createdAt
    followers { totalCount }
    following  { totalCount }
    repositories(
      first: 100,
      ownerAffiliations: [OWNER, ORGANIZATION_MEMBER],
      isFork: false,
      orderBy: { field: STARGAZERS, direction: DESC }
    ) {
      totalCount
      nodes {
        name description stargazerCount forkCount isPrivate
        primaryLanguage { name color }
        languages(first: 10) {
          edges { size node { name color } }
        }
      }
    }
    contributionsCollection {
      totalCommitContributions
      totalPullRequestContributions
      totalIssueContributions
      totalPullRequestReviewContributions
      contributionCalendar {
        totalContributions
        weeks {
          contributionDays { contributionCount date }
        }
      }
    }
    pinnedItems(first: 6, types: [REPOSITORY]) {
      nodes {
        ... on Repository {
          name description stargazerCount forkCount
          primaryLanguage { name color }
        }
      }
    }
    repositoriesContributedTo(
      first: 20,
      contributionTypes: [COMMIT, PULL_REQUEST, PULL_REQUEST_REVIEW],
      includeUserRepositories: false,
      orderBy: { field: STARGAZERS, direction: DESC }
    ) {
      nodes {
        nameWithOwner stargazerCount
        primaryLanguage { name color }
      }
    }
  }
}
"#;

const YEAR_STATS_QUERY: &str = r#"
query($login: String!, $from: DateTime!, $to: DateTime!) {
  user(login: $login) {
    contributionsCollection(from: $from, to: $to) {
      totalCommitContributions
      totalPullRequestContributions
      totalIssueContributions
      totalPullRequestReviewContributions
      contributionCalendar { totalContributions }
    }
  }
}
"#;

#[derive(Debug, Deserialize)]
struct GqlResponse {
    data: Option<GqlData>,
}

#[derive(Debug, Deserialize)]
struct GqlData {
    user: Option<GqlUser>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlUser {
    name: Option<String>,
    login: String,
    bio: Option<String>,
    #[allow(dead_code)]
    company: Option<String>,
    location: Option<String>,
    avatar_url: String,
    created_at: String,
    followers: Count,
    following: Count,
    repositories: RepoConnection,
    contributions_collection: ContributionsCollection,
    pinned_items: PinnedItems,
    repositories_contributed_to: ContributedConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Count {
    total_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepoConnection {
    #[serde(default)]
    total_count: u32,
    nodes: Vec<RepoNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepoNode {
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
    stargazer_count: u32,
    fork_count: u32,
    is_private: bool,
    #[allow(dead_code)]
    primary_language: Option<Language>,
    languages: LanguageConnection,
}

#[derive(Debug, Deserialize, Clone)]
struct Language {
    name: String,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LanguageConnection {
    edges: Vec<LanguageEdge>,
}

#[derive(Debug, Deserialize)]
struct LanguageEdge {
    size: u64,
    node: Language,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionsCollection {
    total_commit_contributions: u32,
    total_pull_request_contributions: u32,
    total_issue_contributions: u32,
    total_pull_request_review_contributions: u32,
    contribution_calendar: ContributionCalendar,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionCalendar {
    total_contributions: u32,
    #[serde(default)]
    weeks: Vec<ContributionWeek>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionWeek {
    contribution_days: Vec<ContributionDay>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributionDay {
    contribution_count: u32,
    date: String,
}

#[derive(Debug, Deserialize)]
struct PinnedItems {
    nodes: Vec<PinnedNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PinnedNode {
    name: Option<String>,
    description: Option<String>,
    stargazer_count: Option<u32>,
    fork_count: Option<u32>,
    primary_language: Option<Language>,
}

#[derive(Debug, Deserialize)]
struct ContributedConnection {
    nodes: Vec<ContributedNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContributedNode {
    name_with_owner: String,
    stargazer_count: u32,
    primary_language: Option<Language>,
}

#[derive(Debug, Deserialize)]
struct YearGqlResponse {
    data: Option<YearGqlData>,
}

#[derive(Debug, Deserialize)]
struct YearGqlData {
    user: Option<YearUser>,
}

#[derive(Debug, Deserialize)]
struct YearUser {
    #[serde(rename = "contributionsCollection")]
    contributions_collection: ContributionsCollection,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GitHubStats {
    pub login: String,
    pub name: String,
    pub bio: String,
    pub location: String,
    pub avatar_url: String,
    pub avatar_bytes: Vec<u8>,
    pub avatar_mime: String,
    pub created_year: i32,
    pub created_iso: String,
    pub followers: u32,
    pub following: u32,

    // last-year totals (from contributions_collection without date range)
    pub total_contributions: u32,
    pub total_commits: u32,
    pub total_prs: u32,
    pub total_issues: u32,
    pub total_reviews: u32,

    // lifetime totals (summed across all years on GitHub)
    pub lifetime_contributions: u32,
    pub lifetime_commits: u32,
    pub lifetime_prs: u32,
    pub lifetime_issues: u32,
    pub lifetime_reviews: u32,

    pub current_streak: u32,
    pub longest_streak: u32,

    pub total_stars: u32,
    pub total_forks: u32,
    pub public_repos: u32,
    pub private_repos: u32,

    pub top_languages: Vec<(String, String, f64)>,

    pub contribution_grid: Vec<Vec<(String, u32)>>,

    pub pinned: Vec<PinnedRepo>,
    pub contributed_to: Vec<ContributedRepo>,
    pub most_starred: Option<MostStarredRepo>,
}

#[derive(Debug, Clone)]
pub struct PinnedRepo {
    pub name: String,
    pub description: String,
    pub stars: u32,
    pub forks: u32,
    pub language: String,
    pub lang_color: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ContributedRepo {
    pub name_with_owner: String,
    pub stars: u32,
    pub language: String,
    pub lang_color: String,
}

#[derive(Debug, Clone)]
pub struct MostStarredRepo {
    pub name: String,
    pub stars: u32,
    pub forks: u32,
    pub language: String,
    pub lang_color: String,
}

pub struct FetchOptions {
    pub include_lifetime: bool,
    pub fetch_avatar: bool,
}

pub async fn fetch_stats(
    client: &reqwest::Client,
    token: &str,
    login: &str,
    options: &FetchOptions,
) -> Result<GitHubStats, AppError> {
    let body = serde_json::json!({
        "query": STATS_QUERY,
        "variables": { "login": login },
    });

    let mut req = client
        .post("https://api.github.com/graphql")
        .header("User-Agent", "statsvg-rs/0.1")
        .json(&body);
    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp: GqlResponse = req.send().await?.error_for_status()?.json().await?;

    let user = resp
        .data
        .and_then(|d| d.user)
        .ok_or_else(|| AppError::NotFound(login.to_string()))?;

    let repos = &user.repositories.nodes;
    let total_stars: u32 = repos.iter().map(|r| r.stargazer_count).sum();
    let total_forks: u32 = repos.iter().map(|r| r.fork_count).sum();
    let private_repos = repos.iter().filter(|r| r.is_private).count() as u32;
    // total_count covers all matching repos, not just the first page of 100
    let public_repos = user.repositories.total_count.saturating_sub(private_repos);

    let top_languages = compute_languages(repos);
    let calendar = &user.contributions_collection.contribution_calendar;
    let (current_streak, longest_streak) = compute_streaks(calendar);
    let contribution_grid = build_grid(calendar);

    let most_starred = repos
        .iter()
        .max_by_key(|r| r.stargazer_count)
        .filter(|r| r.stargazer_count > 0)
        .map(|r| MostStarredRepo {
            name: r.name.clone(),
            stars: r.stargazer_count,
            forks: r.fork_count,
            language: r
                .primary_language
                .as_ref()
                .map(|l| l.name.clone())
                .unwrap_or_default(),
            lang_color: r
                .primary_language
                .as_ref()
                .and_then(|l| l.color.clone())
                .unwrap_or_else(|| "#888".into()),
        });

    let login_owned = user.login.clone();
    let pinned: Vec<PinnedRepo> = user
        .pinned_items
        .nodes
        .into_iter()
        .filter_map(|p| {
            Some(PinnedRepo {
                name: p.name?,
                description: p.description.unwrap_or_default(),
                stars: p.stargazer_count.unwrap_or(0),
                forks: p.fork_count.unwrap_or(0),
                language: p
                    .primary_language
                    .as_ref()
                    .map(|l| l.name.clone())
                    .unwrap_or_default(),
                lang_color: p
                    .primary_language
                    .and_then(|l| l.color)
                    .unwrap_or_else(|| "#888".into()),
            })
        })
        .collect();

    let contributed_to: Vec<ContributedRepo> = user
        .repositories_contributed_to
        .nodes
        .into_iter()
        .map(|c| ContributedRepo {
            name_with_owner: c.name_with_owner,
            stars: c.stargazer_count,
            language: c
                .primary_language
                .as_ref()
                .map(|l| l.name.clone())
                .unwrap_or_default(),
            lang_color: c
                .primary_language
                .and_then(|l| l.color)
                .unwrap_or_else(|| "#888".into()),
        })
        .collect();

    let created_year = user
        .created_at
        .get(..4)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or_else(|| chrono::Utc::now().year());

    let (
        lifetime_contributions,
        lifetime_commits,
        lifetime_prs,
        lifetime_issues,
        lifetime_reviews,
    ) = if options.include_lifetime {
        fetch_lifetime_totals(client, token, login, created_year).await?
    } else {
        (0, 0, 0, 0, 0)
    };

    let avatar_url = if user.avatar_url.contains('?') {
        format!("{}&s=160", user.avatar_url)
    } else {
        format!("{}?s=160", user.avatar_url)
    };
    let (avatar_bytes, avatar_mime) = if options.fetch_avatar {
        fetch_avatar(client, &avatar_url).await.unwrap_or_default()
    } else {
        (Vec::new(), String::new())
    };

    Ok(GitHubStats {
        name: user.name.unwrap_or_else(|| login_owned.clone()),
        login: login_owned,
        bio: user.bio.unwrap_or_default(),
        location: user.location.unwrap_or_default(),
        avatar_url: user.avatar_url,
        avatar_bytes,
        avatar_mime,
        created_year,
        created_iso: user.created_at,
        followers: user.followers.total_count,
        following: user.following.total_count,

        total_contributions: calendar.total_contributions,
        total_commits: user.contributions_collection.total_commit_contributions,
        total_prs: user.contributions_collection.total_pull_request_contributions,
        total_issues: user.contributions_collection.total_issue_contributions,
        total_reviews: user
            .contributions_collection
            .total_pull_request_review_contributions,

        lifetime_contributions,
        lifetime_commits,
        lifetime_prs,
        lifetime_issues,
        lifetime_reviews,

        current_streak,
        longest_streak,
        total_stars,
        total_forks,
        public_repos,
        private_repos,
        top_languages,
        contribution_grid,
        pinned,
        contributed_to,
        most_starred,
    })
}

async fn fetch_lifetime_totals(
    client: &reqwest::Client,
    token: &str,
    login: &str,
    created_year: i32,
) -> Result<(u32, u32, u32, u32, u32), AppError> {
    let current_year = chrono::Utc::now().year();
    let years: Vec<i32> = (created_year..=current_year).collect();

    let futures: Vec<_> = years
        .iter()
        .map(|year| fetch_year_stats(client, token, login, *year))
        .collect();

    let results = try_join_all(futures).await?;
    let mut totals = (0u32, 0u32, 0u32, 0u32, 0u32);
    for r in results {
        totals.0 = totals.0.saturating_add(r.0);
        totals.1 = totals.1.saturating_add(r.1);
        totals.2 = totals.2.saturating_add(r.2);
        totals.3 = totals.3.saturating_add(r.3);
        totals.4 = totals.4.saturating_add(r.4);
    }
    Ok(totals)
}

async fn fetch_year_stats(
    client: &reqwest::Client,
    token: &str,
    login: &str,
    year: i32,
) -> Result<(u32, u32, u32, u32, u32), AppError> {
    let from = format!("{year}-01-01T00:00:00Z");
    let to = format!("{year}-12-31T23:59:59Z");
    let body = serde_json::json!({
        "query": YEAR_STATS_QUERY,
        "variables": { "login": login, "from": from, "to": to },
    });

    let mut req = client
        .post("https://api.github.com/graphql")
        .header("User-Agent", "statsvg-rs/0.1")
        .json(&body);
    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp: YearGqlResponse = req.send().await?.error_for_status()?.json().await?;
    let collection = match resp.data.and_then(|d| d.user) {
        Some(u) => u.contributions_collection,
        None => return Ok((0, 0, 0, 0, 0)),
    };
    Ok((
        collection.contribution_calendar.total_contributions,
        collection.total_commit_contributions,
        collection.total_pull_request_contributions,
        collection.total_issue_contributions,
        collection.total_pull_request_review_contributions,
    ))
}

async fn fetch_avatar(
    client: &reqwest::Client,
    url: &str,
) -> Result<(Vec<u8>, String), AppError> {
    let resp = client
        .get(url)
        .header("User-Agent", "statsvg-rs/0.1")
        .send()
        .await?
        .error_for_status()?;
    let mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();
    let bytes = resp.bytes().await?.to_vec();
    Ok((bytes, mime))
}

fn compute_languages(repos: &[RepoNode]) -> Vec<(String, String, f64)> {
    // Comma-separated language names to exclude from the chart,
    // e.g. STATSVG_HIDE_LANGS="HTML,CSS". Empty/unset hides nothing.
    let hidden: Vec<String> = std::env::var("STATSVG_HIDE_LANGS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let mut totals: HashMap<String, (u64, String)> = HashMap::new();
    for repo in repos {
        for edge in &repo.languages.edges {
            if hidden.contains(&edge.node.name.to_lowercase()) {
                continue;
            }
            let color = edge
                .node
                .color
                .clone()
                .unwrap_or_else(|| "#888".into());
            totals
                .entry(edge.node.name.clone())
                .and_modify(|(bytes, _)| *bytes += edge.size)
                .or_insert((edge.size, color));
        }
    }
    let total_bytes: u64 = totals.values().map(|(b, _)| *b).sum();
    if total_bytes == 0 {
        return Vec::new();
    }
    let mut langs: Vec<(String, String, f64)> = totals
        .into_iter()
        .map(|(name, (bytes, color))| (name, color, (bytes as f64 / total_bytes as f64) * 100.0))
        .collect();
    langs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    langs.truncate(5);
    langs
}

fn compute_streaks(calendar: &ContributionCalendar) -> (u32, u32) {
    let counts: Vec<u32> = calendar
        .weeks
        .iter()
        .flat_map(|w| w.contribution_days.iter().map(|d| d.contribution_count))
        .collect();

    let mut current = 0u32;
    for &c in counts.iter().rev() {
        if c > 0 {
            current += 1;
        } else {
            break;
        }
    }

    let mut longest = 0u32;
    let mut run = 0u32;
    for &c in &counts {
        if c > 0 {
            run += 1;
            if run > longest {
                longest = run;
            }
        } else {
            run = 0;
        }
    }

    (current, longest)
}

fn build_grid(calendar: &ContributionCalendar) -> Vec<Vec<(String, u32)>> {
    let weeks = &calendar.weeks;
    let start = weeks.len().saturating_sub(18);
    weeks[start..]
        .iter()
        .map(|w| {
            w.contribution_days
                .iter()
                .map(|d| (d.date.clone(), d.contribution_count))
                .collect()
        })
        .collect()
}