import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  Diagnostics,
  SettingsDto,
  SettingsPatch,
  UsageRequest,
  UsageResponse,
} from "./types";

export function getStatus(): Promise<AppStatus> {
  return invoke("get_status");
}

export function getSettings(): Promise<SettingsDto> {
  return invoke("get_settings");
}

export function updateSettings(patch: SettingsPatch): Promise<SettingsDto> {
  return invoke("update_settings", { patch });
}

export function queryUsage(request: UsageRequest): Promise<UsageResponse> {
  return invoke("query_usage", { request });
}

export function clearCache(): Promise<void> {
  return invoke("clear_cache");
}

export function runDiagnostics(): Promise<Diagnostics> {
  return invoke("run_diagnostics");
}

