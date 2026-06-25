use super::PricingProvider;

pub struct ModelPricing {
    pub name: &'static str,
    pub input_price_per_1m: f64,
    pub output_price_per_1m: f64,
}

impl PricingProvider for ModelPricing {
    fn cost_per_input_token(&self) -> f64 {
        self.input_price_per_1m / 1_000_000.0
    }

    fn cost_per_output_token(&self) -> f64 {
        self.output_price_per_1m / 1_000_000.0
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn estimate_cost(&self, tokens: usize) -> f64 {
        tokens as f64 * self.cost_per_input_token()
    }
}

pub fn get_pricing(model: &crate::types::ModelType) -> Vec<ModelPricing> {
    match model {
        crate::types::ModelType::Gpt4o => vec![
            ModelPricing { name: "GPT-4o", input_price_per_1m: 2.50, output_price_per_1m: 10.00 },
            ModelPricing { name: "GPT-4o-mini", input_price_per_1m: 0.15, output_price_per_1m: 0.60 },
        ],
        crate::types::ModelType::Claude35Sonnet => vec![
            ModelPricing { name: "Claude 3.5 Sonnet", input_price_per_1m: 3.00, output_price_per_1m: 15.00 },
            ModelPricing { name: "Claude 3.5 Haiku", input_price_per_1m: 0.80, output_price_per_1m: 4.00 },
        ],
        crate::types::ModelType::Gemini15Pro => vec![
            ModelPricing { name: "Gemini 1.5 Pro", input_price_per_1m: 1.25, output_price_per_1m: 5.00 },
            ModelPricing { name: "Gemini 1.5 Flash", input_price_per_1m: 0.075, output_price_per_1m: 0.30 },
        ],
    }
}
