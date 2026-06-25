pub mod calculator;

use crate::types::CompressionMetrics;
use crate::types::ProjectState;
use crate::config::Config;

pub fn calculate(state: &ProjectState, output_text: &str, config: &Config) -> CompressionMetrics {
    calculator::calculate_metrics(state, output_text, config)
}
