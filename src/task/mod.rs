use std::path::PathBuf;

use crate::error::SrrResult;
use crate::storage::StorageManager;
use crate::types::{TaskIntent, IntentType, TaskPlan, PlanStep};

pub struct TaskEngine {
    storage: StorageManager,
}

impl TaskEngine {
    pub fn new(storage: StorageManager) -> Self {
        Self { storage }
    }

    pub fn parse_intent(&self, query: &str) -> TaskIntent {
        let lower = query.to_lowercase();
        let intent_type = classify_intent(&lower);
        let targets = extract_targets(query);
        let constraints = extract_constraints(&lower);
        TaskIntent {
            intent_type,
            targets,
            constraints,
            query: query.to_string(),
        }
    }

    pub fn find_relevant_files(&self, query: &str, max_files: usize) -> SrrResult<Vec<PathBuf>> {
        let intent = self.parse_intent(query);
        let mut combined = Vec::new();

        let terms: Vec<&str> = query.split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        for term in &terms {
            let results = self.storage.search_files(term, max_files / terms.len().max(1))?;
            for p in results {
                if !combined.contains(&p) {
                    combined.push(p);
                }
            }
        }

        if combined.is_empty() {
            for target in &intent.targets {
                let results = self.storage.search_files(target, max_files)?;
                for p in results {
                    if !combined.contains(&p) {
                        combined.push(p);
                    }
                }
            }
        }

        combined.truncate(max_files);
        Ok(combined)
    }

    pub fn generate_plan(&self, query: &str, max_files: usize) -> SrrResult<TaskPlan> {
        let intent = self.parse_intent(query);
        let relevant_files = self.find_relevant_files(query, max_files)?;
        let steps = generate_steps(&intent, &relevant_files);

        Ok(TaskPlan {
            intent,
            steps,
            relevant_files,
            confidence: 0.7,
        })
    }
}

fn classify_intent(lower: &str) -> IntentType {
    let refactor_words = ["refactor", "rename", "extract", "move", "restructure", "reorganize", "clean up", "simplify"];
    let feature_words = ["add", "implement", "create", "new", "feature", "support", "introduce"];
    let bug_words = ["fix", "bug", "error", "crash", "issue", "broken", "incorrect", "wrong", "problem"];
    let explain_words = ["explain", "understand", "what does", "how does", "why is", "describe", "summarize"];
    let optimize_words = ["optimize", "performance", "slow", "speed", "faster", "memory", "efficient"];
    let test_words = ["test", "unit test", "integration test", "coverage", "testing"];
    let doc_words = ["document", "docs", "readme", "comment", "documentation"];

    if refactor_words.iter().any(|w| lower.contains(w)) { return IntentType::Refactor; }
    if feature_words.iter().any(|w| lower.contains(w)) { return IntentType::AddFeature; }
    if bug_words.iter().any(|w| lower.contains(w)) { return IntentType::FixBug; }
    if explain_words.iter().any(|w| lower.contains(w)) { return IntentType::Explain; }
    if optimize_words.iter().any(|w| lower.contains(w)) { return IntentType::Optimize; }
    if test_words.iter().any(|w| lower.contains(w)) { return IntentType::Test; }
    if doc_words.iter().any(|w| lower.contains(w)) { return IntentType::Document; }
    IntentType::Unknown
}

fn extract_targets(query: &str) -> Vec<String> {
    let re = regex::Regex::new(r#""([^"]+)"|'([^']+)'"#).unwrap();
    let mut targets = Vec::new();
    for cap in re.captures_iter(query) {
        if let Some(t) = cap.get(1).or_else(|| cap.get(2)) {
            targets.push(t.as_str().to_string());
        }
    }
    targets
}

fn extract_constraints(lower: &str) -> Vec<String> {
    let mut constraints = Vec::new();
    if lower.contains("backward") || lower.contains("backwards") {
        constraints.push("backward compatible".to_string());
    }
    if lower.contains("without breaking") {
        constraints.push("no breaking changes".to_string());
    }
    if lower.contains("async") {
        constraints.push("async".to_string());
    }
    if lower.contains("safe") {
        constraints.push("memory safe".to_string());
    }
    constraints
}

fn generate_steps(intent: &TaskIntent, files: &[PathBuf]) -> Vec<PlanStep> {
    let mut steps = Vec::new();
    let base = match intent.intent_type {
        IntentType::Refactor => vec![
            ("Analyze current implementation", files.to_vec()),
            ("Design refactored structure", vec![]),
            ("Implement changes", files.to_vec()),
            ("Verify backward compatibility", vec![]),
        ],
        IntentType::AddFeature => vec![
            ("Understand existing codebase", files.to_vec()),
            ("Design feature interface", vec![]),
            ("Implement feature", files.to_vec()),
            ("Add tests", vec![]),
        ],
        IntentType::FixBug => vec![
            ("Reproduce bug", files.to_vec()),
            ("Identify root cause", files.to_vec()),
            ("Implement fix", files.to_vec()),
            ("Verify fix with tests", vec![]),
        ],
        IntentType::Explain => vec![
            ("Analyze relevant code", files.to_vec()),
            ("Generate explanation", vec![]),
        ],
        IntentType::Optimize => vec![
            ("Profile current performance", files.to_vec()),
            ("Identify bottlenecks", files.to_vec()),
            ("Implement optimizations", files.to_vec()),
            ("Benchmark improvements", vec![]),
        ],
        IntentType::Test => vec![
            ("Analyze existing tests", files.to_vec()),
            ("Identify coverage gaps", vec![]),
            ("Write new tests", files.to_vec()),
            ("Run test suite", vec![]),
        ],
        IntentType::Document => vec![
            ("Analyze public API", files.to_vec()),
            ("Generate documentation", vec![]),
        ],
        IntentType::Search => vec![
            ("Search codebase for query", files.to_vec()),
            ("Present results", vec![]),
        ],
        IntentType::Unknown => vec![
            ("Search codebase", files.to_vec()),
            ("Analyze project structure", vec![]),
        ],
    };
    for (i, (action, plan_files)) in base.iter().enumerate() {
        steps.push(PlanStep {
            order: i + 1,
            action: format!("Step {}", i + 1),
            description: action.to_string(),
            files: plan_files.clone(),
        });
    }
    steps
}
