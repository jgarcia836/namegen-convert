// Shared document model for both formats: an ordered list of named categories,
// each holding weighted text entries that may reference other categories with
// a `{name}` placeholder.

#[derive(Debug, Clone)]
pub struct Entry {
    pub text: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub start: Option<String>,
    pub categories: Vec<(String, Vec<Entry>)>,
}

// Fatal issues mean the input could not be understood at all and always abort,
// even with --lenient. Semantic issues are things like an unresolved reference
// or a duplicate category: the data is usable, just questionable, so
// --lenient downgrades them to warnings instead of stopping the conversion.
#[derive(Debug)]
pub enum Severity {
    Fatal,
    Semantic,
}

#[derive(Debug)]
pub struct Issue {
    pub message: String,
    pub severity: Severity,
}

impl Issue {
    pub fn fatal(message: impl Into<String>) -> Issue {
        Issue { message: message.into(), severity: Severity::Fatal }
    }

    pub fn semantic(message: impl Into<String>) -> Issue {
        Issue { message: message.into(), severity: Severity::Semantic }
    }
}

impl Document {
    pub fn has_category(&self, name: &str) -> bool {
        self.categories.iter().any(|(n, _)| n == name)
    }

    pub fn category_mut(&mut self, name: &str) -> Option<&mut Vec<Entry>> {
        self.categories.iter_mut().find(|(n, _)| n == name).map(|(_, e)| e)
    }

    // Adds entries to an existing category (merging) or creates a new one.
    // Returns true if the category already existed, so callers can warn
    // about the merge when they consider it noteworthy.
    pub fn add_category(&mut self, name: String, entries: Vec<Entry>) -> bool {
        if let Some(existing) = self.category_mut(&name) {
            existing.extend(entries);
            true
        } else {
            self.categories.push((name, entries));
            false
        }
    }
}

// Finds every `{identifier}` placeholder in a piece of entry text.
pub fn placeholders(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if text.as_bytes()[i] == b'{' {
            if let Some(end) = text[i + 1..].find('}') {
                let name = &text[i + 1..i + 1 + end];
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    result.push(name);
                }
                i += end + 2;
                continue;
            }
        }
        i += 1;
    }
    result
}

// Resolves the start category (falling back to a category literally named
// "name", or the first category defined) and checks that every placeholder
// reference points at a real category. Both checks are semantic, not syntax
// errors, so --lenient can downgrade them to warnings.
pub fn finalize(doc: &mut Document, issues: &mut Vec<Issue>) {
    if doc.start.is_none() {
        if doc.has_category("name") {
            doc.start = Some("name".to_string());
        } else if let Some((first, _)) = doc.categories.first() {
            let first = first.clone();
            issues.push(Issue::semantic(format!(
                "no start category specified; falling back to '{}'",
                first
            )));
            doc.start = Some(first);
        } else {
            issues.push(Issue::fatal("document has no categories".to_string()));
            return;
        }
    } else if let Some(name) = &doc.start {
        if !doc.has_category(name) {
            issues.push(Issue::semantic(format!("start category '{}' is not defined", name)));
        }
    }

    let names: Vec<String> = doc.categories.iter().map(|(n, _)| n.clone()).collect();
    for (cat, entries) in &doc.categories {
        for entry in entries {
            for ph in placeholders(&entry.text) {
                if !names.iter().any(|n| n.as_str() == ph) {
                    issues.push(Issue::semantic(format!(
                        "category '{}' references undefined category '{{{}}}'",
                        cat, ph
                    )));
                }
            }
        }
    }
}
