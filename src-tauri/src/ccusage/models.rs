use crate::{errors::AppError, settings::Settings};
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RangeKind {
    Today,
    Last7Days,
    Last30Days,
    All,
    Custom,
}

impl Default for RangeKind {
    fn default() -> Self {
        Self::Last30Days
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageRequest {
    pub range: RangeKind,
    pub since: Option<String>,
    pub until: Option<String>,
    pub timezone: Option<String>,
    pub force_refresh: bool,
}

impl Default for UsageRequest {
    fn default() -> Self {
        Self {
            range: RangeKind::Last30Days,
            since: None,
            until: None,
            timezone: None,
            force_refresh: false,
        }
    }
}

impl UsageRequest {
    pub fn normalize(
        self,
        settings: &Settings,
        today: NaiveDate,
    ) -> Result<NormalizedUsageRequest, AppError> {
        let (since, until) = match self.range {
            RangeKind::Today => (Some(today), Some(today)),
            RangeKind::Last7Days => (Some(today - Duration::days(6)), Some(today)),
            RangeKind::Last30Days => (Some(today - Duration::days(29)), Some(today)),
            RangeKind::All => (None, None),
            RangeKind::Custom => (
                parse_optional_date(self.since.as_deref(), "since")?,
                parse_optional_date(self.until.as_deref(), "until")?,
            ),
        };

        if let (Some(start), Some(end)) = (since, until) {
            if start > end {
                return Err(AppError::InvalidRequest {
                    details: "since must be on or before until".to_string(),
                });
            }
        }

        let timezone = self
            .timezone
            .filter(|value| !value.trim().is_empty())
            .or_else(|| settings.timezone.clone())
            .unwrap_or_else(|| "UTC".to_string());

        Ok(NormalizedUsageRequest {
            since: since.map(|date| date.format("%Y-%m-%d").to_string()),
            until: until.map(|date| date.format("%Y-%m-%d").to_string()),
            timezone,
            force_refresh: self.force_refresh,
        })
    }
}

fn parse_optional_date(value: Option<&str>, label: &str) -> Result<Option<NaiveDate>, AppError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| AppError::InvalidRequest {
                details: format!("{label} must use YYYY-MM-DD"),
            }),
        None => Ok(None),
    }
}

#[derive(Debug, Clone)]
pub struct NormalizedUsageRequest {
    pub since: Option<String>,
    pub until: Option<String>,
    pub timezone: String,
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
    pub cost_micro_usd: Option<i64>,
}

impl TokenTotals {
    pub fn add(&mut self, other: &TokenTotals) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_creation_tokens += other.cache_creation_tokens;
        self.cache_read_tokens += other.cache_read_tokens;
        self.reasoning_output_tokens += other.reasoning_output_tokens;
        self.total_tokens += other.total_tokens;
        self.cost_micro_usd = add_optional_cost(self.cost_micro_usd, other.cost_micro_usd);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model_name: String,
    pub agent: String,
    #[serde(flatten)]
    pub totals: TokenTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub period: String,
    #[serde(flatten)]
    pub totals: TokenTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageResponse {
    pub totals: TokenTotals,
    pub models: Vec<ModelUsage>,
    pub daily: Vec<DailyUsage>,
    pub generated_at: String,
    pub last_refreshed: String,
    pub stale: bool,
    pub from_cache: bool,
    pub ccusage_version: Option<String>,
    pub command: Vec<String>,
    pub warning: Option<crate::errors::ApiError>,
}

impl UsageResponse {
    pub fn with_cache_flags(
        mut self,
        from_cache: bool,
        stale: bool,
        warning: Option<crate::errors::ApiError>,
    ) -> Self {
        self.from_cache = from_cache;
        self.stale = stale;
        self.warning = warning;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub ccusage_found: bool,
    pub ccusage_path: Option<String>,
    pub ccusage_version: Option<String>,
    pub settings_path: String,
    pub cache_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub status: AppStatus,
    pub command: Vec<String>,
    pub stdout_excerpt: Option<String>,
    pub stderr_excerpt: Option<String>,
    pub error: Option<crate::errors::ApiError>,
}

fn add_optional_cost(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a + b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use chrono::NaiveDate;

    #[test]
    fn normalizes_last_7_days() {
        let request = UsageRequest {
            range: RangeKind::Last7Days,
            timezone: Some("America/Los_Angeles".to_string()),
            ..UsageRequest::default()
        };
        let normalized = request
            .normalize(
                &Settings::default(),
                NaiveDate::from_ymd_opt(2026, 6, 24).unwrap(),
            )
            .unwrap();

        assert_eq!(normalized.since.as_deref(), Some("2026-06-18"));
        assert_eq!(normalized.until.as_deref(), Some("2026-06-24"));
        assert_eq!(normalized.timezone, "America/Los_Angeles");
    }

    #[test]
    fn rejects_reversed_custom_range() {
        let request = UsageRequest {
            range: RangeKind::Custom,
            since: Some("2026-06-25".to_string()),
            until: Some("2026-06-24".to_string()),
            ..UsageRequest::default()
        };

        assert!(request
            .normalize(
                &Settings::default(),
                NaiveDate::from_ymd_opt(2026, 6, 24).unwrap()
            )
            .is_err());
    }
}
