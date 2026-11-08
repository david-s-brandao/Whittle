//! A small GitHub REST client: issues, pull requests, comments and reviews.
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum ForgeError {
    Auth,
    RateLimited(Option<i64>),
    Forbidden(String),
    NotFound,
    Invalid(String),
    Network(String),
    Other(u16, String),
}

impl std::fmt::Display for ForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth => write!(f, "GitHub rejected your login: the token is invalid or has expired. Reconnect to fix it."),
            Self::RateLimited(reset) => {
                let when = reset
                    .map(|r| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let mins = ((r - now).max(0) + 59) / 60;
                        format!(" It resets in about {mins} minute{}.", if mins == 1 { "" } else { "s" })
                    })
                    .unwrap_or_default();
                write!(f, "GitHub's request limit is used up.{when} Signing in gives you a much higher limit.")
            }
            Self::Forbidden(m) => write!(f, "GitHub says you're not allowed to do that ({m}). Your token may be missing the “repo” permission."),
            Self::NotFound => write!(f, "Not found. If the repository is private, sign in with an account that can see it."),
            Self::Invalid(m) => write!(f, "{m}"),
            Self::Network(m) => write!(f, "Could not reach GitHub: {m}"),
            Self::Other(c, m) => write!(f, "GitHub returned an error ({c}): {m}"),
        }
    }
}

impl From<ForgeError> for String {
    fn from(e: ForgeError) -> String {
        e.to_string()
    }
}

type Res<T> = Result<T, ForgeError>;

/// Percent-encodes everything except unreserved characters.
fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub struct Client {
    base: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(base: &str, token: Option<String>) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(25))
            .user_agent("Whittle")
            .build();
        Self {
            base: base.trim_end_matches('/').to_string(),
            token,
            agent,
        }
    }

    /// Sends a request; returns the JSON body and whether another page follows.
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Res<(Value, bool)> {
        let url = format!("{}{}", self.base, path);
        let mut req = self
            .agent
            .request(method, &url)
            .set("Accept", "application/vnd.github+json")
            .set("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = &self.token {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        let result = match body {
            Some(b) => req
                .set("Content-Type", "application/json")
                .send_string(&b.to_string()),
            None => req.call(),
        };
        match result {
            Ok(resp) => {
                let next = resp
                    .header("link")
                    .is_some_and(|l| l.contains("rel=\"next\""));
                let text = resp
                    .into_string()
                    .map_err(|e| ForgeError::Network(e.to_string()))?;
                let v = if text.trim().is_empty() {
                    Value::Null
                } else {
                    serde_json::from_str(&text)
                        .map_err(|e| ForgeError::Other(200, format!("unreadable answer: {e}")))?
                };
                Ok((v, next))
            }
            Err(ureq::Error::Status(code, resp)) => {
                let remaining = resp.header("x-ratelimit-remaining").map(str::to_string);
                let reset = resp
                    .header("x-ratelimit-reset")
                    .and_then(|r| r.parse::<i64>().ok());
                let text = resp.into_string().unwrap_or_default();
                let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
                let mut message = v["message"].as_str().unwrap_or("").to_string();
                if let Some(errs) = v["errors"].as_array() {
                    let extra: Vec<String> = errs
                        .iter()
                        .filter_map(|e| {
                            e["message"]
                                .as_str()
                                .map(str::to_string)
                                .or_else(|| e.as_str().map(str::to_string))
                        })
                        .collect();
                    if !extra.is_empty() {
                        message = format!("{message}: {}", extra.join("; "));
                    }
                }
                Err(match code {
                    401 => ForgeError::Auth,
                    403 | 429
                        if remaining.as_deref() == Some("0")
                            || message.to_lowercase().contains("rate limit") =>
                    {
                        ForgeError::RateLimited(reset)
                    }
                    403 => ForgeError::Forbidden(message),
                    404 => ForgeError::NotFound,
                    422 => ForgeError::Invalid(if message.is_empty() {
                        "GitHub could not accept that.".into()
                    } else {
                        message
                    }),
                    c => ForgeError::Other(c, message),
                })
            }
            Err(ureq::Error::Transport(t)) => Err(ForgeError::Network(t.to_string())),
        }
    }

    fn get(&self, path: &str) -> Res<(Value, bool)> {
        self.call("GET", path, None)
    }

    /// All pages of a list endpoint (up to `max_pages` × 100 items).
    fn get_all(&self, path: &str, max_pages: u32) -> Res<Vec<Value>> {
        let sep = if path.contains('?') { '&' } else { '?' };
        let mut out = Vec::new();
        for page in 1..=max_pages {
            let (v, next) = self.get(&format!("{path}{sep}per_page=100&page={page}"))?;
            out.extend(v.as_array().cloned().unwrap_or_default());
            if !next {
                break;
            }
        }
        Ok(out)
    }

    /// The signed-in user's login (also validates the token).
    pub fn whoami(&self) -> Res<String> {
        let (v, _) = self.get("/user")?;
        Ok(v["login"].as_str().unwrap_or("").to_string())
    }
}

// ---- models ----------------------------------------------------------------------------

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct Label {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct IssueSummary {
    pub number: u64,
    pub title: String,
    /// "open" or "closed".
    pub state: String,
    pub author: String,
    pub labels: Vec<Label>,
    pub assignees: Vec<String>,
    pub comments: u64,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct PullSummary {
    pub number: u64,
    pub title: String,
    /// "open", "closed" or "merged".
    pub state: String,
    pub draft: bool,
    pub author: String,
    pub head_ref: String,
    pub base_ref: String,
    pub labels: Vec<Label>,
    /// Start of the description; enough to find "fixes #12" style links.
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct Comment {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub url: String,
    /// OWNER, MEMBER, CONTRIBUTOR, …
    pub association: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct Review {
    pub id: u64,
    pub author: String,
    /// APPROVED, CHANGES_REQUESTED, COMMENTED, DISMISSED or PENDING.
    pub state: String,
    pub body: String,
    pub submitted_at: String,
    pub url: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct ReviewComment {
    pub id: u64,
    pub author: String,
    pub body: String,
    pub path: String,
    /// Line in the file; `None` when the comment is outdated.
    pub line: Option<u64>,
    /// LEFT (old file) or RIGHT (new file).
    pub side: String,
    pub created_at: String,
    pub in_reply_to: Option<u64>,
    /// The review this comment was submitted with, if any.
    pub review_id: Option<u64>,
    pub url: String,
    pub commit_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct IssueDetail {
    pub summary: IssueSummary,
    pub body: String,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PullDetail {
    pub summary: PullSummary,
    pub body: String,
    /// clean, dirty (conflicts), unstable (checks failing), blocked, behind, draft, unknown.
    pub mergeable_state: Option<String>,
    pub additions: u64,
    pub deletions: u64,
    pub changed_files: u64,
    pub commits: u64,
    pub head_sha: String,
    pub comments: Vec<Comment>,
    pub reviews: Vec<Review>,
    pub review_comments: Vec<ReviewComment>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PullFile {
    pub filename: String,
    pub previous_filename: Option<String>,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    /// Unified diff of the file; absent for binary or very large files.
    pub patch: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_page: Option<u32>,
}

// ---- JSON -> models ------------------------------------------------------------------

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}
fn author(v: &Value) -> String {
    v["user"]["login"].as_str().unwrap_or("ghost").to_string()
}
fn labels(v: &Value) -> Vec<Label> {
    v["labels"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|l| Label {
                    name: s(&l["name"]),
                    color: s(&l["color"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn issue_summary(v: &Value) -> IssueSummary {
    IssueSummary {
        number: v["number"].as_u64().unwrap_or(0),
        title: s(&v["title"]),
        state: s(&v["state"]),
        author: author(v),
        labels: labels(v),
        assignees: v["assignees"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|u| u["login"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        comments: v["comments"].as_u64().unwrap_or(0),
        created_at: s(&v["created_at"]),
        updated_at: s(&v["updated_at"]),
        url: s(&v["html_url"]),
    }
}

fn pull_summary(v: &Value) -> PullSummary {
    // list items carry `merged_at`; search results hide it under `pull_request`
    let merged = !v["merged_at"].is_null() || !v["pull_request"]["merged_at"].is_null();
    let state = match (s(&v["state"]).as_str(), merged) {
        (_, true) => "merged".to_string(),
        (st, false) => st.to_string(),
    };
    PullSummary {
        number: v["number"].as_u64().unwrap_or(0),
        title: s(&v["title"]),
        state,
        draft: v["draft"].as_bool().unwrap_or(false),
        author: author(v),
        head_ref: s(&v["head"]["ref"]),
        base_ref: s(&v["base"]["ref"]),
        labels: labels(v),
        body: s(&v["body"]).chars().take(2000).collect(),
        created_at: s(&v["created_at"]),
        updated_at: s(&v["updated_at"]),
        url: s(&v["html_url"]),
    }
}

fn comment(v: &Value) -> Comment {
    Comment {
        id: v["id"].as_u64().unwrap_or(0),
        author: author(v),
        body: s(&v["body"]),
        created_at: s(&v["created_at"]),
        url: s(&v["html_url"]),
        association: s(&v["author_association"]),
    }
}

fn review(v: &Value) -> Review {
    Review {
        id: v["id"].as_u64().unwrap_or(0),
        author: author(v),
        state: s(&v["state"]),
        body: s(&v["body"]),
        submitted_at: s(&v["submitted_at"]),
        url: s(&v["html_url"]),
    }
}

fn review_comment(v: &Value) -> ReviewComment {
    ReviewComment {
        id: v["id"].as_u64().unwrap_or(0),
        author: author(v),
        body: s(&v["body"]),
        path: s(&v["path"]),
        line: v["line"].as_u64().or_else(|| v["original_line"].as_u64()),
        side: if v["side"].is_null() {
            "RIGHT".into()
        } else {
            s(&v["side"])
        },
        created_at: s(&v["created_at"]),
        in_reply_to: v["in_reply_to_id"].as_u64(),
        review_id: v["pull_request_review_id"].as_u64(),
        url: s(&v["html_url"]),
        commit_id: s(&v["commit_id"]),
    }
}

// ---- API ------------------------------------------------------------------------------

pub struct GitHub {
    pub client: Client,
    pub owner: String,
    pub name: String,
}

const PER_PAGE: u32 = 30;

fn check_state(state: &str) -> Res<()> {
    if ["open", "closed", "all"].contains(&state) {
        Ok(())
    } else {
        Err(ForgeError::Invalid("Unknown state filter.".into()))
    }
}

impl GitHub {
    pub fn new(client: Client, owner: &str, name: &str) -> Self {
        Self {
            client,
            owner: owner.into(),
            name: name.into(),
        }
    }

    fn repo(&self) -> String {
        format!("/repos/{}/{}", enc(&self.owner), enc(&self.name))
    }

    /// Search qualifier text for the repo and the state filter.
    fn search_q(&self, kind: &str, state: &str, query: &str) -> String {
        let st = if state == "all" {
            String::new()
        } else {
            format!(" state:{state}")
        };
        format!(
            "repo:{}/{} is:{kind}{st} {}",
            self.owner,
            self.name,
            query.trim()
        )
    }

    pub fn issues(&self, state: &str, query: &str, page: u32) -> Res<Page<IssueSummary>> {
        check_state(state)?;
        let page = page.max(1);
        let (v, next) = if query.trim().is_empty() {
            self.client.get(&format!(
                "{}/issues?state={state}&per_page={PER_PAGE}&page={page}&sort=updated&direction=desc",
                self.repo()
            ))?
        } else {
            let (res, next) = self.client.get(&format!(
                "/search/issues?q={}&per_page={PER_PAGE}&page={page}&sort=updated&order=desc",
                enc(&self.search_q("issue", state, query))
            ))?;
            (res["items"].clone(), next)
        };
        // the issues endpoint also returns pull requests: leave those to the other tab
        let items = v
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|i| i.get("pull_request").is_none())
                    .map(issue_summary)
                    .collect()
            })
            .unwrap_or_default();
        Ok(Page {
            items,
            next_page: next.then_some(page + 1),
        })
    }

    pub fn issue(&self, number: u64) -> Res<IssueDetail> {
        let (v, _) = self
            .client
            .get(&format!("{}/issues/{number}", self.repo()))?;
        let comments = self
            .client
            .get_all(&format!("{}/issues/{number}/comments", self.repo()), 5)?
            .iter()
            .map(comment)
            .collect();
        Ok(IssueDetail {
            summary: issue_summary(&v),
            body: s(&v["body"]),
            comments,
        })
    }

    pub fn pulls(&self, state: &str, query: &str, page: u32) -> Res<Page<PullSummary>> {
        check_state(state)?;
        let page = page.max(1);
        let (v, next) = if query.trim().is_empty() {
            self.client.get(&format!(
                "{}/pulls?state={state}&per_page={PER_PAGE}&page={page}&sort=updated&direction=desc",
                self.repo()
            ))?
        } else {
            let (res, next) = self.client.get(&format!(
                "/search/issues?q={}&per_page={PER_PAGE}&page={page}&sort=updated&order=desc",
                enc(&self.search_q("pr", state, query))
            ))?;
            (res["items"].clone(), next)
        };
        let items = v
            .as_array()
            .map(|a| a.iter().map(pull_summary).collect())
            .unwrap_or_default();
        Ok(Page {
            items,
            next_page: next.then_some(page + 1),
        })
    }

    pub fn pull(&self, number: u64) -> Res<PullDetail> {
        let r = self.repo();
        let (v, _) = self.client.get(&format!("{r}/pulls/{number}"))?;
        let comments = self
            .client
            .get_all(&format!("{r}/issues/{number}/comments"), 5)?
            .iter()
            .map(comment)
            .collect();
        let reviews = self
            .client
            .get_all(&format!("{r}/pulls/{number}/reviews"), 5)?
            .iter()
            .map(review)
            .collect();
        let review_comments = self
            .client
            .get_all(&format!("{r}/pulls/{number}/comments"), 5)?
            .iter()
            .map(review_comment)
            .collect();
        Ok(PullDetail {
            summary: pull_summary(&v),
            body: s(&v["body"]),
            mergeable_state: v["mergeable_state"].as_str().map(str::to_string),
            additions: v["additions"].as_u64().unwrap_or(0),
            deletions: v["deletions"].as_u64().unwrap_or(0),
            changed_files: v["changed_files"].as_u64().unwrap_or(0),
            commits: v["commits"].as_u64().unwrap_or(0),
            head_sha: s(&v["head"]["sha"]),
            comments,
            reviews,
            review_comments,
        })
    }

    pub fn pull_files(&self, number: u64) -> Res<Vec<PullFile>> {
        Ok(self
            .client
            .get_all(&format!("{}/pulls/{number}/files", self.repo()), 10)?
            .iter()
            .map(|f| PullFile {
                filename: s(&f["filename"]),
                previous_filename: f["previous_filename"].as_str().map(str::to_string),
                status: s(&f["status"]),
                additions: f["additions"].as_u64().unwrap_or(0),
                deletions: f["deletions"].as_u64().unwrap_or(0),
                patch: f["patch"].as_str().map(str::to_string),
            })
            .collect())
    }

    /// Comment on an issue or on a pull request's conversation.
    pub fn add_comment(&self, number: u64, body: &str) -> Res<Comment> {
        if body.trim().is_empty() {
            return Err(ForgeError::Invalid("Write something first.".into()));
        }
        let (v, _) = self.client.call(
            "POST",
            &format!("{}/issues/{number}/comments", self.repo()),
            Some(&json!({ "body": body })),
        )?;
        Ok(comment(&v))
    }

    /// Close or reopen an issue or pull request.
    pub fn set_state(&self, number: u64, state: &str) -> Res<()> {
        if !["open", "closed"].contains(&state) {
            return Err(ForgeError::Invalid("Unknown state.".into()));
        }
        self.client.call(
            "PATCH",
            &format!("{}/issues/{number}", self.repo()),
            Some(&json!({ "state": state })),
        )?;
        Ok(())
    }

    /// Submit a review. `event`: APPROVE, REQUEST_CHANGES or COMMENT.
    pub fn review(&self, number: u64, event: &str, body: &str) -> Res<()> {
        if !["APPROVE", "REQUEST_CHANGES", "COMMENT"].contains(&event) {
            return Err(ForgeError::Invalid("Unknown review type.".into()));
        }
        if event != "APPROVE" && body.trim().is_empty() {
            return Err(ForgeError::Invalid(
                "Say what you think: this kind of review needs a comment.".into(),
            ));
        }
        let mut payload = json!({ "event": event });
        if !body.trim().is_empty() {
            payload["body"] = json!(body);
        }
        self.client.call(
            "POST",
            &format!("{}/pulls/{number}/reviews", self.repo()),
            Some(&payload),
        )?;
        Ok(())
    }

    /// A comment on one line of the diff, or (with `in_reply_to`) a reply in a thread.
    #[allow(clippy::too_many_arguments)]
    pub fn review_comment(
        &self,
        number: u64,
        commit_id: &str,
        path: &str,
        line: u64,
        side: &str,
        body: &str,
        in_reply_to: Option<u64>,
    ) -> Res<ReviewComment> {
        if body.trim().is_empty() {
            return Err(ForgeError::Invalid("Write something first.".into()));
        }
        let payload = match in_reply_to {
            Some(id) => json!({ "body": body, "in_reply_to": id }),
            None => {
                if !["LEFT", "RIGHT"].contains(&side) {
                    return Err(ForgeError::Invalid("Unknown side of the diff.".into()));
                }
                json!({ "body": body, "commit_id": commit_id, "path": path, "line": line, "side": side })
            }
        };
        let (v, _) = self.client.call(
            "POST",
            &format!("{}/pulls/{number}/comments", self.repo()),
            Some(&payload),
        )?;
        Ok(review_comment(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    struct Recorded {
        method: String,
        path: String,
        headers: HashMap<String, String>,
        body: String,
    }
    struct Canned(u16, Vec<(&'static str, String)>, String);
    fn ok(body: Value) -> Canned {
        Canned(200, vec![], body.to_string())
    }

    /// A one-connection-per-response fake API. Returns its base URL and the recorded requests.
    fn fake(responses: Vec<Canned>) -> (String, Arc<Mutex<Vec<Recorded>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();
        std::thread::spawn(move || {
            for canned in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buf = Vec::new();
                let mut chunk = [0u8; 1024];
                let head_end = loop {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break buf.len();
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break p + 4;
                    }
                };
                let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                let mut lines = head.lines();
                let first = lines.next().unwrap_or("").to_string();
                let mut parts = first.split_whitespace();
                let (method, path) = (
                    parts.next().unwrap_or("").to_string(),
                    parts.next().unwrap_or("").to_string(),
                );
                let headers: HashMap<String, String> = lines
                    .filter_map(|l| l.split_once(':'))
                    .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
                    .collect();
                let want: usize = headers
                    .get("content-length")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                while buf.len() < head_end + want {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                let body = String::from_utf8_lossy(&buf[head_end..]).to_string();
                log2.lock().unwrap().push(Recorded {
                    method,
                    path,
                    headers,
                    body,
                });
                let mut resp = format!("HTTP/1.1 {} X\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n", canned.0, canned.2.len());
                for (k, v) in canned.1 {
                    resp.push_str(&format!("{k}: {v}\r\n"));
                }
                resp.push_str("\r\n");
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.write_all(canned.2.as_bytes());
            }
        });
        (base, log)
    }

    fn gh(base: &str, token: Option<&str>) -> GitHub {
        GitHub::new(Client::new(base, token.map(String::from)), "octo", "hello")
    }

    fn issue_json(n: u64, title: &str) -> Value {
        json!({"number": n, "title": title, "state": "open", "user": {"login": "ana"}, "comments": 2,
               "labels": [{"name": "bug", "color": "d73a4a"}], "assignees": [{"login": "miguel"}],
               "created_at": "2026-11-01T10:00:00Z", "updated_at": "2026-11-02T10:00:00Z",
               "html_url": format!("https://github.com/octo/hello/issues/{n}")})
    }

    #[test]
    fn lists_issues_without_prs_with_pagination_and_auth() {
        let mut pr = issue_json(2, "A pull request");
        pr["pull_request"] = json!({"url": "x"});
        let (base, log) = fake(vec![Canned(
            200,
            vec![(
                "Link",
                "<https://api.github.com/x?page=2>; rel=\"next\"".into(),
            )],
            json!([issue_json(1, "Crash on start"), pr, issue_json(3, "Typo")]).to_string(),
        )]);
        let page = gh(&base, Some("secret-token"))
            .issues("open", "", 1)
            .unwrap();
        assert_eq!(
            page.items.iter().map(|i| i.number).collect::<Vec<_>>(),
            vec![1, 3],
            "PRs are filtered out"
        );
        assert_eq!(page.next_page, Some(2));
        assert_eq!(
            page.items[0].labels,
            vec![Label {
                name: "bug".into(),
                color: "d73a4a".into()
            }]
        );
        assert_eq!(page.items[0].assignees, vec!["miguel"]);
        let req = &log.lock().unwrap()[0];
        assert_eq!(req.method, "GET");
        assert!(
            req.path
                .starts_with("/repos/octo/hello/issues?state=open&per_page=30&page=1"),
            "{}",
            req.path
        );
        assert_eq!(req.headers["authorization"], "Bearer secret-token");
        assert_eq!(req.headers["accept"], "application/vnd.github+json");
        assert!(req.headers["user-agent"].starts_with("Whittle"));
    }

    #[test]
    fn no_token_means_no_authorization_header_and_search_is_encoded() {
        let (base, log) = fake(vec![ok(
            json!({"items": [issue_json(7, "login bug")], "total_count": 1}),
        )]);
        let page = gh(&base, None)
            .issues("closed", "login & \"bug\"", 3)
            .unwrap();
        assert_eq!(page.items[0].number, 7);
        assert_eq!(page.next_page, None);
        let req = &log.lock().unwrap()[0];
        assert!(!req.headers.contains_key("authorization"));
        assert!(req.path.starts_with("/search/issues?q=repo%3Aocto%2Fhello%20is%3Aissue%20state%3Aclosed%20login%20%26%20%22bug%22"), "{}", req.path);
        assert!(req.path.contains("page=3"));
    }

    #[test]
    fn pull_list_derives_merged_state_and_refs() {
        let pr = |n: u64, state: &str, merged: Value, draft: bool| {
            json!({"number": n, "title": format!("PR {n}"), "state": state, "merged_at": merged, "draft": draft,
                   "user": {"login": "ana"}, "head": {"ref": "feature/x", "sha": "abc"}, "base": {"ref": "main"},
                   "labels": [], "created_at": "t", "updated_at": "t", "html_url": "u"})
        };
        let (base, _) = fake(vec![ok(json!([
            pr(1, "open", Value::Null, true),
            pr(2, "closed", json!("2026-01-01T00:00:00Z"), false),
            pr(3, "closed", Value::Null, false),
        ]))]);
        let page = gh(&base, None).pulls("all", "", 1).unwrap();
        let states: Vec<_> = page
            .items
            .iter()
            .map(|p| (p.state.as_str(), p.draft))
            .collect();
        assert_eq!(
            states,
            vec![("open", true), ("merged", false), ("closed", false)]
        );
        assert_eq!(
            (
                page.items[0].head_ref.as_str(),
                page.items[0].base_ref.as_str()
            ),
            ("feature/x", "main")
        );
    }

    #[test]
    fn pull_detail_gathers_conversation_reviews_and_line_comments() {
        let (base, log) = fake(vec![
            ok(
                json!({"number": 5, "title": "Add search", "state": "open", "merged_at": null, "draft": false, "body": "Closes #1",
                      "user": {"login": "ana"}, "head": {"ref": "feat", "sha": "deadbeef"}, "base": {"ref": "main"}, "labels": [],
                      "mergeable_state": "clean", "additions": 10, "deletions": 2, "changed_files": 3, "commits": 4,
                      "created_at": "t", "updated_at": "t", "html_url": "u"}),
            ),
            ok(
                json!([{"id": 1, "user": {"login": "bob"}, "body": "LGTM so far", "created_at": "t", "html_url": "u", "author_association": "MEMBER"}]),
            ),
            ok(
                json!([{"id": 9, "user": {"login": "bob"}, "state": "CHANGES_REQUESTED", "body": "Please fix", "submitted_at": "t", "html_url": "u"}]),
            ),
            ok(
                json!([{"id": 30, "pull_request_review_id": 9, "user": {"login": "bob"}, "body": "typo", "path": "src/a.rs", "line": 12, "side": "RIGHT", "created_at": "t", "html_url": "u", "commit_id": "deadbeef"},
                      {"id": 31, "user": null, "body": "reply", "path": "src/a.rs", "line": null, "original_line": 8, "side": "LEFT", "in_reply_to_id": 30, "created_at": "t", "html_url": "u", "commit_id": "old"}]),
            ),
        ]);
        let d = gh(&base, Some("t")).pull(5).unwrap();
        assert_eq!(
            (
                d.summary.number,
                d.body.as_str(),
                d.mergeable_state.as_deref()
            ),
            (5, "Closes #1", Some("clean"))
        );
        assert_eq!(
            (d.additions, d.deletions, d.changed_files, d.commits),
            (10, 2, 3, 4)
        );
        assert_eq!(d.head_sha, "deadbeef");
        assert_eq!(d.comments[0].association, "MEMBER");
        assert_eq!(d.reviews[0].state, "CHANGES_REQUESTED");
        assert_eq!(d.review_comments[0].line, Some(12));
        assert_eq!(
            d.review_comments[0].review_id,
            Some(9),
            "inline comments know their review"
        );
        assert_eq!(d.review_comments[1].review_id, None);
        assert_eq!(d.review_comments[1].in_reply_to, Some(30));
        assert_eq!(
            d.review_comments[1].author, "ghost",
            "deleted users do not break parsing"
        );
        assert_eq!(
            d.review_comments[1].line,
            Some(8),
            "outdated comments fall back to the original line"
        );
        let paths: Vec<String> = log
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.path.split('?').next().unwrap().to_string())
            .collect();
        assert_eq!(
            paths,
            [
                "/repos/octo/hello/pulls/5",
                "/repos/octo/hello/issues/5/comments",
                "/repos/octo/hello/pulls/5/reviews",
                "/repos/octo/hello/pulls/5/comments"
            ]
        );
    }

    #[test]
    fn files_follow_pagination_until_the_last_page() {
        let file = |n: &str| json!({"filename": n, "status": "modified", "additions": 1, "deletions": 0, "patch": "@@ -1 +1 @@\n-a\n+b"});
        let (base, log) = fake(vec![
            Canned(
                200,
                vec![("Link", "<x>; rel=\"next\"".into())],
                json!([file("a.rs")]).to_string(),
            ),
            ok(json!([
                file("b.rs"),
                json!({"filename": "img.png", "status": "added", "additions": 0, "deletions": 0})
            ])),
        ]);
        let files = gh(&base, None).pull_files(8).unwrap();
        assert_eq!(
            files
                .iter()
                .map(|f| f.filename.as_str())
                .collect::<Vec<_>>(),
            ["a.rs", "b.rs", "img.png"]
        );
        assert!(files[2].patch.is_none(), "binary files have no patch");
        let log = log.lock().unwrap();
        assert!(log[0].path.contains("page=1") && log[1].path.contains("page=2"));
    }

    #[test]
    fn writes_send_the_right_payloads() {
        let (base, log) = fake(vec![
            ok(
                json!({"id": 77, "user": {"login": "me"}, "body": "Hello", "created_at": "t", "html_url": "u", "author_association": "OWNER"}),
            ),
            ok(json!({})),
            ok(json!({})),
            ok(
                json!({"id": 5, "user": {"login": "me"}, "body": "nit", "path": "a.rs", "line": 3, "side": "RIGHT", "created_at": "t", "html_url": "u", "commit_id": "sha"}),
            ),
            ok(
                json!({"id": 6, "user": {"login": "me"}, "body": "agreed", "path": "a.rs", "line": 3, "side": "RIGHT", "in_reply_to_id": 5, "created_at": "t", "html_url": "u", "commit_id": "sha"}),
            ),
        ]);
        let g = gh(&base, Some("t"));
        assert_eq!(g.add_comment(4, "Hello").unwrap().id, 77);
        g.set_state(4, "closed").unwrap();
        g.review(4, "APPROVE", "").unwrap();
        let rc = g
            .review_comment(4, "sha", "a.rs", 3, "RIGHT", "nit", None)
            .unwrap();
        assert_eq!(rc.line, Some(3));
        g.review_comment(4, "sha", "a.rs", 3, "RIGHT", "agreed", Some(5))
            .unwrap();

        let log = log.lock().unwrap();
        let body = |i: usize| serde_json::from_str::<Value>(&log[i].body).unwrap();
        assert_eq!(
            (log[0].method.as_str(), log[0].path.as_str()),
            ("POST", "/repos/octo/hello/issues/4/comments")
        );
        assert_eq!(body(0), json!({"body": "Hello"}));
        assert_eq!(
            (log[1].method.as_str(), body(1)),
            ("PATCH", json!({"state": "closed"}))
        );
        assert_eq!(
            (log[2].path.as_str(), body(2)),
            (
                "/repos/octo/hello/pulls/4/reviews",
                json!({"event": "APPROVE"})
            )
        );
        assert_eq!(
            body(3),
            json!({"body": "nit", "commit_id": "sha", "path": "a.rs", "line": 3, "side": "RIGHT"})
        );
        assert_eq!(body(4), json!({"body": "agreed", "in_reply_to": 5}));
        assert_eq!(log[0].headers["content-type"], "application/json");
    }

    #[test]
    fn invalid_writes_are_refused_before_any_network_call() {
        let (base, log) = fake(vec![]);
        let g = gh(&base, Some("t"));
        assert!(g.add_comment(1, "   ").is_err());
        assert!(g
            .review(1, "REQUEST_CHANGES", "")
            .unwrap_err()
            .to_string()
            .contains("needs a comment"));
        assert!(g.review(1, "MERGE", "x").is_err());
        assert!(g.set_state(1, "deleted").is_err());
        assert!(g.review_comment(1, "s", "p", 1, "UP", "x", None).is_err());
        assert!(g.issues("everything", "", 1).is_err());
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn errors_are_translated_into_plain_language() {
        let reset = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 600)
            .to_string();
        let (base, _) = fake(vec![
            Canned(401, vec![], json!({"message": "Bad credentials"}).to_string()),
            Canned(404, vec![], json!({"message": "Not Found"}).to_string()),
            Canned(403, vec![("X-RateLimit-Remaining", "0".into()), ("X-RateLimit-Reset", reset)], json!({"message": "API rate limit exceeded"}).to_string()),
            Canned(403, vec![], json!({"message": "Resource not accessible by personal access token"}).to_string()),
            Canned(422, vec![], json!({"message": "Validation Failed", "errors": [{"message": "Can not approve your own pull request"}]}).to_string()),
            Canned(500, vec![], "oops".into()),
        ]);
        let g = gh(&base, Some("t"));
        let e = |r: Res<Page<IssueSummary>>| r.unwrap_err();
        assert_eq!(e(g.issues("open", "", 1)), ForgeError::Auth);
        assert!(e(g.issues("open", "", 1)).to_string().contains("private"));
        let rl = e(g.issues("open", "", 1));
        assert!(matches!(rl, ForgeError::RateLimited(Some(_))), "{rl:?}");
        assert!(rl.to_string().contains("10 minutes"), "{rl}");
        assert!(e(g.issues("open", "", 1))
            .to_string()
            .contains("not allowed"));
        assert!(g
            .review(1, "APPROVE", "")
            .unwrap_err()
            .to_string()
            .contains("own pull request"));
        assert!(matches!(
            e(g.issues("open", "", 1)),
            ForgeError::Other(500, _)
        ));

        // nothing listening: a clear network error, not a hang or a panic
        let dead = gh("http://127.0.0.1:1", None)
            .issues("open", "", 1)
            .unwrap_err();
        assert!(
            matches!(dead, ForgeError::Network(_)) && dead.to_string().contains("Could not reach"),
            "{dead}"
        );
    }

    #[test]
    fn whoami_validates_a_token() {
        let (base, _) = fake(vec![
            ok(json!({"login": "david"})),
            Canned(401, vec![], "{}".into()),
        ]);
        assert_eq!(
            Client::new(&base, Some("good".into())).whoami().unwrap(),
            "david"
        );
        assert_eq!(
            Client::new(&base, Some("bad".into())).whoami().unwrap_err(),
            ForgeError::Auth
        );
    }
}
