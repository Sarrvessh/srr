use srr::tokenizer::estimator::count_tokens_fast;
use srr::tokenizer::PricingProvider;

#[test]
fn test_token_count_empty() {
    assert_eq!(count_tokens_fast(""), 0);
}

#[test]
fn test_token_count_known() {
    let count = count_tokens_fast("hello world");
    assert!(count > 0);
}

#[test]
fn test_token_count_unicode() {
    let count = count_tokens_fast("日本語テスト");
    assert!(count > 0);
}

#[test]
fn test_count_tokens_fast_works() {
    let text = "This is a test sentence that should be tokenized correctly.";
    let count = count_tokens_fast(text);
    assert!(count > 0);
    assert!(count < text.len());
}

#[test]
fn test_pricing_gpt4o() {
    use srr::tokenizer::pricing::get_pricing;
    use srr::types::ModelType;

    let pricing = get_pricing(&ModelType::Gpt4o);
    assert!(!pricing.is_empty());
    assert!(pricing[0].cost_per_input_token() > 0.0);
    assert!(pricing[0].cost_per_output_token() > 0.0);

    let cost = pricing[0].estimate_cost(1000000);
    assert!((cost - 2.50).abs() < 0.01);
}
