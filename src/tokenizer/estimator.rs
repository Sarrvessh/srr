use super::TokenEstimator;

pub struct TiktokenEstimator;

impl TokenEstimator for TiktokenEstimator {
    fn count_tokens(&self, content: &str) -> usize {
        match tiktoken_rs::cl100k_base() {
            Ok(bpe) => bpe.encode_ordinary(content).len(),
            Err(_) => content.len().div_ceil(4),
        }
    }

    fn model_name(&self) -> &'static str {
        "cl100k_base (GPT-4/GPT-3.5)"
    }
}

impl TiktokenEstimator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TiktokenEstimator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn count_tokens_fast(content: &str) -> usize {
    match tiktoken_rs::cl100k_base() {
        Ok(bpe) => bpe.encode_ordinary(content).len(),
        Err(_) => content.len().div_ceil(4),
    }
}
