pub mod estimator;
pub mod pricing;

pub trait TokenEstimator: Send + Sync {
    fn count_tokens(&self, content: &str) -> usize;
    fn model_name(&self) -> &'static str;
}

pub trait PricingProvider: Send + Sync {
    fn cost_per_input_token(&self) -> f64;
    fn cost_per_output_token(&self) -> f64;
    fn name(&self) -> &'static str;
    fn estimate_cost(&self, tokens: usize) -> f64;
}
