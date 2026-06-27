export type RangeKind = "today" | "last7Days" | "last30Days" | "all" | "custom";

export interface UsageRequest {
  range: RangeKind;
  since?: string | null;
  until?: string | null;
  timezone?: string | null;
  forceRefresh?: boolean;
}

export interface TokenTotals {
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
  reasoningOutputTokens: number;
  totalTokens: number;
  costMicroUsd: number | null;
}

export interface ModelUsage extends TokenTotals {
  modelName: string;
  agent: string;
}

export interface DailyUsage extends TokenTotals {
  period: string;
}

export interface ApiError {
  code: string;
  message: string;
  details?: string | null;
  exitCode?: number | null;
  retryable: boolean;
}

export interface UsageResponse {
  totals: TokenTotals;
  models: ModelUsage[];
  daily: DailyUsage[];
  generatedAt: string;
  lastRefreshed: string;
  stale: boolean;
  fromCache: boolean;
  ccusageVersion?: string | null;
  command: string[];
  warning?: ApiError | null;
}

export interface SettingsDto {
  ccusagePath?: string | null;
  claudeConfigDirs?: string | null;
  timezone?: string | null;
  cacheTtlSeconds: number;
  offline: boolean;
  autoRefreshSeconds?: number | null;
  includeRawJson: boolean;
}

export interface SettingsPatch {
  ccusagePath?: string | null;
  claudeConfigDirs?: string | null;
  timezone?: string | null;
  cacheTtlSeconds?: number;
  offline?: boolean;
  autoRefreshSeconds?: number | null;
  includeRawJson?: boolean;
}

export interface AppStatus {
  ccusageFound: boolean;
  ccusagePath?: string | null;
  ccusageVersion?: string | null;
  settingsPath: string;
  cachePath: string;
}

export interface Diagnostics {
  status: AppStatus;
  command: string[];
  stdoutExcerpt?: string | null;
  stderrExcerpt?: string | null;
  error?: ApiError | null;
}

