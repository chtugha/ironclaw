//! Project-scoped memory document operations.

use std::sync::Arc;

use tracing::warn;

use crate::traits::store::Store;
use crate::types::error::EngineError;
use crate::types::memory::{DocId, DocType, MemoryDoc};
use crate::types::project::ProjectId;
use crate::types::thread::ThreadId;
use crate::types::LEGACY_SHARED_OWNER_ID;

const PLAN_TEMPLATES: &[(&str, &str)] = &[
    (
        "install-software",
        include_str!("../../plan_templates/install-software.md"),
    ),
    (
        "web-search",
        include_str!("../../plan_templates/web-search.md"),
    ),
    (
        "calendar-events",
        include_str!("../../plan_templates/calendar-events.md"),
    ),
    (
        "notes",
        include_str!("../../plan_templates/notes.md"),
    ),
    (
        "file-search",
        include_str!("../../plan_templates/file-search.md"),
    ),
    (
        "git-operations",
        include_str!("../../plan_templates/git-operations.md"),
    ),
    (
        "system-info",
        include_str!("../../plan_templates/system-info.md"),
    ),
];

/// Parse a simple YAML frontmatter block (`---\n...\n---\n`) from a Markdown file.
///
/// Returns `(frontmatter_text, body_text)` or `None` if no frontmatter is present.
fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let content = content.trim_start();
    let open_len = if content.starts_with("---\r\n") {
        5
    } else if content.starts_with("---\n") {
        4
    } else {
        return None;
    };
    let after_open = &content[open_len..];
    let close_pos = after_open.find("\n---\n").or_else(|| after_open.find("\n---\r\n"))?;
    let frontmatter = &after_open[..close_pos];
    let close_len = if after_open[close_pos + 1..].starts_with("---\r\n") {
        5
    } else {
        4
    };
    let body = &after_open[close_pos + 1 + close_len..];
    Some((frontmatter, body))
}

/// Extract a string scalar value from a YAML line like `key: value`.
fn yaml_string_value<'a>(frontmatter: &'a str, key: &str) -> Option<&'a str> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key)
            && let Some(rest) = rest.strip_prefix(':')
        {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Extract a YAML list value from a line like `keywords: [a, b, c]`.
fn yaml_list_value(frontmatter: &str, key: &str) -> Vec<String> {
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key)
            && let Some(rest) = rest.strip_prefix(':')
        {
            let rest = rest.trim();
            if rest.starts_with('[') && rest.ends_with(']') {
                let inner = &rest[1..rest.len() - 1];
                return inner
                    .split(',')
                    .map(|s| {
                        s.trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string()
                    })
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Extract a float value from a YAML line like `confidence: 0.8`.
fn yaml_f64_value(frontmatter: &str, key: &str) -> Option<f64> {
    yaml_string_value(frontmatter, key).and_then(|v| v.parse().ok())
}

/// Extract a boolean value from a YAML line like `is_template: true`.
fn yaml_bool_value(frontmatter: &str, key: &str) -> Option<bool> {
    yaml_string_value(frontmatter, key).map(|v| matches!(v, "true" | "yes" | "1"))
}

/// Thin wrapper over the [`Store`] trait for project-scoped doc operations.
pub struct MemoryStore {
    store: Arc<dyn Store>,
}

impl MemoryStore {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }

    /// Create a new memory document.
    pub async fn create_doc(
        &self,
        project_id: ProjectId,
        user_id: &str,
        doc_type: DocType,
        title: &str,
        content: &str,
    ) -> Result<MemoryDoc, EngineError> {
        let doc = MemoryDoc::new(project_id, user_id, doc_type, title, content);
        self.store.save_memory_doc(&doc).await?;
        Ok(doc)
    }

    /// Create a doc linked to a source thread.
    pub async fn create_doc_from_thread(
        &self,
        project_id: ProjectId,
        user_id: &str,
        doc_type: DocType,
        title: &str,
        content: &str,
        source_thread_id: ThreadId,
    ) -> Result<MemoryDoc, EngineError> {
        let doc = MemoryDoc::new(project_id, user_id, doc_type, title, content)
            .with_source_thread(source_thread_id);
        self.store.save_memory_doc(&doc).await?;
        Ok(doc)
    }

    /// Load a single doc by ID.
    pub async fn get_doc(&self, id: DocId) -> Result<Option<MemoryDoc>, EngineError> {
        self.store.load_memory_doc(id).await
    }

    /// List all docs in a project, optionally filtered by type.
    pub async fn list_docs(
        &self,
        project_id: ProjectId,
        user_id: &str,
        doc_type: Option<DocType>,
    ) -> Result<Vec<MemoryDoc>, EngineError> {
        let all = self.store.list_memory_docs(project_id, user_id).await?;
        match doc_type {
            Some(dt) => Ok(all.into_iter().filter(|d| d.doc_type == dt).collect()),
            None => Ok(all),
        }
    }

    /// Seed the system plan-template collection from the embedded Markdown files.
    ///
    /// Each embedded template is parsed, converted to a `DocType::Plan` MemoryDoc,
    /// and upserted (by title) into a stable system project. Already-present docs
    /// are not overwritten. Returns the number of templates inserted.
    ///
    /// Emits a `warn!` for any template whose frontmatter is missing the
    /// `is_template` field; that template is still inserted with `is_template: false`.
    pub async fn ensure_system_docs(&self) -> usize {
        let project_id = ProjectId(uuid::Uuid::nil());

        let existing: std::collections::HashSet<String> = self
            .store
            .list_memory_docs(project_id, LEGACY_SHARED_OWNER_ID)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|d| d.title)
            .collect();

        let mut inserted = 0usize;

        for (slug, raw) in PLAN_TEMPLATES {
            let (frontmatter, body) = match split_frontmatter(raw) {
                Some(pair) => pair,
                None => {
                    warn!("plan template '{}' has no frontmatter — skipping", slug);
                    continue;
                }
            };

            let title = yaml_string_value(frontmatter, "title")
                .unwrap_or(slug)
                .to_string();

            if existing.contains(&title) {
                continue;
            }

            let is_template_opt = yaml_bool_value(frontmatter, "is_template");
            if is_template_opt.is_none() {
                warn!(
                    "plan template '{}' is missing 'is_template' frontmatter field — inserting with is_template: false",
                    slug
                );
            }
            let is_template = is_template_opt.unwrap_or(false);

            let confidence = yaml_f64_value(frontmatter, "confidence").unwrap_or(0.7);
            let keywords = yaml_list_value(frontmatter, "keywords");
            let tags = yaml_list_value(frontmatter, "tags");

            let steps: Vec<String> = body
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    if line.is_empty() {
                        return None;
                    }
                    let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
                    if rest.starts_with(". ") || rest.starts_with(") ") {
                        Some(rest[2..].trim().to_string())
                    } else {
                        None
                    }
                })
                .filter(|s| !s.is_empty())
                .collect();

            let metadata = serde_json::json!({
                "is_template": is_template,
                "is_decomposition": false,
                "confidence": confidence,
                "keywords": keywords,
                "steps": steps,
                "execution_count": 0u64,
                "failure_count": 0u64,
            });

            let mut doc =
                MemoryDoc::new(project_id, LEGACY_SHARED_OWNER_ID, DocType::Plan, &title, body.trim())
                    .with_tags(tags);
            doc.metadata = metadata;

            if let Err(e) = self.store.save_memory_doc(&doc).await {
                warn!("failed to seed plan template '{}': {}", slug, e);
            } else {
                inserted += 1;
            }
        }

        inserted
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::types::memory::{DocId, DocType};
    use crate::types::project::ProjectId;
    use crate::types::thread::ThreadId;

    use super::MemoryStore;

    fn make_store() -> MemoryStore {
        MemoryStore::new(Arc::new(crate::tests::InMemoryStore::new()))
    }

    // ── Tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn create_doc_and_get() {
        let store = make_store();
        let project_id = ProjectId::new();

        let doc = store
            .create_doc(
                project_id,
                "test-user",
                DocType::Summary,
                "Test Doc",
                "Some content",
            )
            .await
            .unwrap();

        assert_eq!(doc.title, "Test Doc");
        assert_eq!(doc.content, "Some content");
        assert_eq!(doc.doc_type, DocType::Summary);
        assert_eq!(doc.project_id, project_id);
        assert!(doc.source_thread_id.is_none());

        let loaded = store.get_doc(doc.id).await.unwrap();
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, doc.id);
        assert_eq!(loaded.title, "Test Doc");
        assert_eq!(loaded.content, "Some content");
    }

    #[tokio::test]
    async fn create_doc_from_thread_links_source() {
        let store = make_store();
        let project_id = ProjectId::new();
        let thread_id = ThreadId::new();

        let doc = store
            .create_doc_from_thread(
                project_id,
                "test-user",
                DocType::Lesson,
                "Thread Lesson",
                "Learned something",
                thread_id,
            )
            .await
            .unwrap();

        assert_eq!(doc.source_thread_id, Some(thread_id));
        assert_eq!(doc.doc_type, DocType::Lesson);

        let loaded = store.get_doc(doc.id).await.unwrap().unwrap();
        assert_eq!(loaded.source_thread_id, Some(thread_id));
    }

    #[tokio::test]
    async fn list_docs_by_project() {
        let store = make_store();
        let project_a = ProjectId::new();
        let project_b = ProjectId::new();

        store
            .create_doc(project_a, "test-user", DocType::Note, "A1", "content a1")
            .await
            .unwrap();
        store
            .create_doc(project_a, "test-user", DocType::Note, "A2", "content a2")
            .await
            .unwrap();
        store
            .create_doc(project_b, "test-user", DocType::Note, "B1", "content b1")
            .await
            .unwrap();

        let docs_a = store.list_docs(project_a, "test-user", None).await.unwrap();
        assert_eq!(docs_a.len(), 2);
        assert!(docs_a.iter().all(|d| d.project_id == project_a));

        let docs_b = store.list_docs(project_b, "test-user", None).await.unwrap();
        assert_eq!(docs_b.len(), 1);
        assert_eq!(docs_b[0].title, "B1");
    }

    #[tokio::test]
    async fn list_docs_filters_by_type() {
        let store = make_store();
        let project_id = ProjectId::new();

        store
            .create_doc(
                project_id,
                "test-user",
                DocType::Summary,
                "S1",
                "summary content",
            )
            .await
            .unwrap();
        store
            .create_doc(
                project_id,
                "test-user",
                DocType::Lesson,
                "L1",
                "lesson content",
            )
            .await
            .unwrap();
        store
            .create_doc(
                project_id,
                "test-user",
                DocType::Summary,
                "S2",
                "another summary",
            )
            .await
            .unwrap();

        let summaries = store
            .list_docs(project_id, "test-user", Some(DocType::Summary))
            .await
            .unwrap();
        assert_eq!(summaries.len(), 2);
        assert!(summaries.iter().all(|d| d.doc_type == DocType::Summary));

        let lessons = store
            .list_docs(project_id, "test-user", Some(DocType::Lesson))
            .await
            .unwrap();
        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].title, "L1");
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let store = make_store();
        let result = store.get_doc(DocId::new()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn ensure_system_docs_inserts_valid_templates() {
        let store = make_store();
        let inserted = store.ensure_system_docs().await;
        assert!(
            inserted >= 7,
            "expected at least 7 plan templates, got {inserted}"
        );

        let project_id = ProjectId(uuid::Uuid::nil());
        let docs = store
            .list_docs(project_id, crate::types::LEGACY_SHARED_OWNER_ID, Some(DocType::Plan))
            .await
            .unwrap();
        assert!(docs.len() >= 7);

        let install = docs.iter().find(|d| d.title == "Install Software");
        assert!(install.is_some(), "Install Software template not found");
        let install = install.unwrap();
        assert_eq!(
            install.metadata.get("is_template").and_then(|v| v.as_bool()),
            Some(true)
        );
        let steps = install
            .metadata
            .get("steps")
            .and_then(|v| v.as_array())
            .expect("steps array");
        assert!(!steps.is_empty());
    }

    #[tokio::test]
    async fn ensure_system_docs_missing_is_template_inserts_with_false() {
        let _store = make_store();

        let raw = "---\ntitle: Test Plan\nconfidence: 0.5\n---\n1. Do the thing\n2. Verify it\n";
        let (frontmatter, body) = super::split_frontmatter(raw).unwrap();

        let is_template_opt = super::yaml_bool_value(frontmatter, "is_template");
        assert!(is_template_opt.is_none(), "no is_template field expected");
        let is_template = is_template_opt.unwrap_or(false);
        assert!(!is_template);

        let steps: Vec<String> = body
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
                if rest.starts_with(". ") || rest.starts_with(") ") {
                    Some(rest[2..].trim().to_string())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(steps, vec!["Do the thing", "Verify it"]);
    }
}
