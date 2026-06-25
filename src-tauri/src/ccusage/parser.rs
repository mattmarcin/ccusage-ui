use crate::{
    ccusage::models::{DailyUsage, ModelUsage, TokenTotals, UsageResponse},
    errors::AppError,
};
use serde_json::Value;
use std::collections::HashMap;

pub fn parse_usage_json(raw: &str) -> Result<UsageResponse, AppError> {
    let root: Value = serde_json::from_str(raw).map_err(|err| AppError::JsonParse {
        details: err.to_string(),
    })?;

    let rows = report_rows(&root);
    let reasoning_reported = reasoning_reported(&root);
    let mut model_map: HashMap<(String, String), ModelUsage> = HashMap::new();
    let mut daily_map: HashMap<String, DailyUsage> = HashMap::new();

    for row in rows {
        let period = string_field(row, &["period", "date", "month", "week"])
            .unwrap_or_else(|| "unknown".to_string());
        let agent = string_field(row, &["agent", "source", "type"]).unwrap_or_else(|| {
            if row.get("models").is_some() {
                "codex".to_string()
            } else {
                "all".to_string()
            }
        });
        let row_totals = totals_from_value(row);

        daily_map
            .entry(period.clone())
            .and_modify(|daily| daily.totals.add(&row_totals))
            .or_insert_with(|| DailyUsage {
                period: period.clone(),
                totals: row_totals.clone(),
            });

        if let Some(breakdowns) = row.get("modelBreakdowns").and_then(Value::as_array) {
            for breakdown in breakdowns {
                let model_name = string_field(breakdown, &["modelName", "model", "name"])
                    .unwrap_or_else(|| "unknown".to_string());
                let totals = totals_from_value(breakdown);
                add_model(&mut model_map, &agent, &model_name, totals);
            }
        } else if let Some(models) = row.get("models") {
            parse_models_container(models, &agent, &mut model_map);
        }
    }

    let mut daily = daily_map.into_values().collect::<Vec<_>>();
    daily.sort_by(|a, b| a.period.cmp(&b.period));

    let mut models = model_map.into_values().collect::<Vec<_>>();
    models.sort_by(|a, b| {
        b.totals
            .cost_micro_usd
            .unwrap_or(-1)
            .cmp(&a.totals.cost_micro_usd.unwrap_or(-1))
            .then_with(|| b.totals.total_tokens.cmp(&a.totals.total_tokens))
            .then_with(|| a.model_name.cmp(&b.model_name))
    });

    let totals = root
        .get("totals")
        .map(totals_from_value)
        .unwrap_or_else(|| {
            daily
                .iter()
                .fold(TokenTotals::default(), |mut totals, row| {
                    totals.add(&row.totals);
                    totals
                })
        });

    Ok(UsageResponse {
        totals,
        models,
        daily,
        reasoning_reported,
        generated_at: String::new(),
        last_refreshed: String::new(),
        stale: false,
        from_cache: false,
        ccusage_version: None,
        command: Vec::new(),
        warning: None,
    })
}

fn reasoning_reported(root: &Value) -> bool {
    if root.get("totals").is_some_and(has_reasoning_field) {
        return true;
    }

    report_rows(root).into_iter().any(|row| {
        has_reasoning_field(row)
            || row
                .get("modelBreakdowns")
                .and_then(Value::as_array)
                .is_some_and(|items| items.iter().any(has_reasoning_field))
            || row.get("models").is_some_and(|models| match models {
                Value::Object(map) => map.values().any(has_reasoning_field),
                Value::Array(items) => items.iter().any(has_reasoning_field),
                _ => false,
            })
    })
}

fn has_reasoning_field(value: &Value) -> bool {
    value.get("reasoningOutputTokens").is_some() || value.get("reasoning_output_tokens").is_some()
}
fn report_rows(root: &Value) -> Vec<&Value> {
    for key in ["daily", "weekly", "monthly", "sessions", "blocks"] {
        if let Some(rows) = root.get(key).and_then(Value::as_array) {
            return rows.iter().collect();
        }
    }

    if let Some(projects) = root.get("projects").and_then(Value::as_object) {
        return projects
            .values()
            .flat_map(|value| value.as_array().into_iter().flatten())
            .collect();
    }

    Vec::new()
}

fn parse_models_container(
    models: &Value,
    fallback_agent: &str,
    model_map: &mut HashMap<(String, String), ModelUsage>,
) {
    if let Some(object) = models.as_object() {
        for (model_name, value) in object {
            add_model(
                model_map,
                fallback_agent,
                model_name,
                totals_from_value(value),
            );
        }
        return;
    }

    if let Some(array) = models.as_array() {
        for value in array {
            let model_name = string_field(value, &["modelName", "model", "name"])
                .unwrap_or_else(|| "unknown".to_string());
            add_model(
                model_map,
                fallback_agent,
                &model_name,
                totals_from_value(value),
            );
        }
    }
}

fn add_model(
    model_map: &mut HashMap<(String, String), ModelUsage>,
    agent: &str,
    model_name: &str,
    totals: TokenTotals,
) {
    model_map
        .entry((agent.to_string(), model_name.to_string()))
        .and_modify(|row| row.totals.add(&totals))
        .or_insert_with(|| ModelUsage {
            model_name: model_name.to_string(),
            agent: agent.to_string(),
            totals,
        });
}

fn totals_from_value(value: &Value) -> TokenTotals {
    let input_tokens = u64_field(value, &["inputTokens", "input_tokens"]);
    let output_tokens = u64_field(value, &["outputTokens", "output_tokens"]);
    let cache_creation_tokens = u64_field(value, &["cacheCreationTokens", "cache_creation_tokens"]);
    let cache_read_tokens = u64_field(value, &["cacheReadTokens", "cache_read_tokens"]);
    let reasoning_output_tokens =
        u64_field(value, &["reasoningOutputTokens", "reasoning_output_tokens"]);
    let computed_total = input_tokens
        + output_tokens
        + cache_creation_tokens
        + cache_read_tokens
        + reasoning_output_tokens;
    let total_tokens = u64_field(value, &["totalTokens", "total_tokens"]).max(computed_total);
    let cost_micro_usd = cost_field(value, &["totalCost", "costUSD", "cost", "costUsd"]);

    TokenTotals {
        input_tokens,
        output_tokens,
        cache_creation_tokens,
        cache_read_tokens,
        reasoning_output_tokens,
        total_tokens,
        cost_micro_usd,
    }
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn u64_field(value: &Value, names: &[&str]) -> u64 {
    names
        .iter()
        .find_map(|name| value.get(*name))
        .and_then(|field| {
            field
                .as_u64()
                .or_else(|| field.as_f64().map(|value| value.max(0.0).round() as u64))
                .or_else(|| field.as_str().and_then(|value| value.parse::<u64>().ok()))
        })
        .unwrap_or(0)
}

fn cost_field(value: &Value, names: &[&str]) -> Option<i64> {
    names
        .iter()
        .find_map(|name| value.get(*name))
        .and_then(|field| {
            field
                .as_f64()
                .or_else(|| field.as_i64().map(|value| value as f64))
                .or_else(|| field.as_str().and_then(|value| value.parse::<f64>().ok()))
                .map(|dollars| (dollars * 1_000_000.0).round() as i64)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_global_daily_model_breakdowns() {
        let json = r#"{
          "daily": [{
            "agent": "codex",
            "period": "2026-06-24",
            "inputTokens": 100,
            "outputTokens": 40,
            "cacheCreationTokens": 5,
            "cacheReadTokens": 10,
            "totalTokens": 155,
            "totalCost": 0.0123,
            "modelBreakdowns": [{
              "modelName": "gpt-5.1-codex-max",
              "inputTokens": 100,
              "outputTokens": 40,
              "cacheCreationTokens": 5,
              "cacheReadTokens": 10,
              "cost": 0.0123
            }]
          }],
          "totals": {
            "inputTokens": 100,
            "outputTokens": 40,
            "cacheCreationTokens": 5,
            "cacheReadTokens": 10,
            "totalTokens": 155,
            "totalCost": 0.0123
          }
        }"#;

        let parsed = parse_usage_json(json).unwrap();
        assert!(!parsed.reasoning_reported);
        assert_eq!(parsed.totals.cost_micro_usd, Some(12_300));
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].model_name, "gpt-5.1-codex-max");
        assert_eq!(parsed.models[0].totals.total_tokens, 155);
    }

    #[test]
    fn parses_codex_models_object() {
        let json = r#"{
          "daily": [{
            "date": "2026-06-24",
            "inputTokens": 30,
            "outputTokens": 10,
            "reasoningOutputTokens": 7,
            "totalTokens": 47,
            "costUSD": 0.0042,
            "models": {
              "gpt-5": {
                "inputTokens": 30,
                "outputTokens": 10,
                "reasoningOutputTokens": 7,
                "totalTokens": 47,
                "isFallback": false
              }
            }
          }],
          "totals": {
            "inputTokens": 30,
            "outputTokens": 10,
            "reasoningOutputTokens": 7,
            "totalTokens": 47,
            "costUSD": 0.0042
          }
        }"#;

        let parsed = parse_usage_json(json).unwrap();
        assert!(parsed.reasoning_reported);
        assert_eq!(parsed.totals.cost_micro_usd, Some(4_200));
        assert_eq!(parsed.models[0].agent, "codex");
        assert_eq!(parsed.models[0].totals.reasoning_output_tokens, 7);
        assert_eq!(parsed.models[0].totals.cost_micro_usd, None);
    }

    #[test]
    fn handles_empty_report() {
        let json = r#"{
          "daily": [],
          "totals": {
            "cacheCreationTokens": 0,
            "cacheReadTokens": 0,
            "inputTokens": 0,
            "outputTokens": 0,
            "totalCost": 0,
            "totalTokens": 0
          }
        }"#;

        let parsed = parse_usage_json(json).unwrap();
        assert!(parsed.models.is_empty());
        assert!(parsed.daily.is_empty());
        assert_eq!(parsed.totals.total_tokens, 0);
    }

    #[test]
    fn treats_missing_cost_as_none() {
        let json = r#"{
          "daily": [{
            "period": "2026-06-24",
            "inputTokens": 1,
            "outputTokens": 2,
            "modelBreakdowns": [{
              "modelName": "model-a",
              "inputTokens": 1,
              "outputTokens": 2,
              "unexpected": "ignored"
            }]
          }],
          "totals": {
            "inputTokens": 1,
            "outputTokens": 2,
            "totalTokens": 3
          }
        }"#;

        let parsed = parse_usage_json(json).unwrap();
        assert_eq!(parsed.totals.cost_micro_usd, None);
        assert_eq!(parsed.models[0].totals.cost_micro_usd, None);
    }

    #[test]
    fn rejects_malformed_json() {
        let error = parse_usage_json("{nope").unwrap_err();
        assert!(matches!(error, AppError::JsonParse { .. }));
    }
}
