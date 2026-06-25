import {
  Activity,
  AlertTriangle,
  BarChart3,
  CheckCircle2,
  Clipboard,
  Moon,
  RefreshCw,
  Settings,
  Sun,
  Terminal,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  clearCache,
  getSettings,
  getStatus,
  queryUsage,
  runDiagnostics,
  updateSettings,
} from "./api";
import { formatCost, formatDateTime, formatTokens, percent } from "./format";
import type {
  ApiError,
  AppStatus,
  Diagnostics,
  ModelUsage,
  RangeKind,
  SettingsDto,
  UsageResponse,
} from "./types";

type SortKey = "cost" | "tokens" | "input" | "output" | "model";

const rangeOptions: Array<{ value: RangeKind; label: string }> = [
  { value: "today", label: "Today" },
  { value: "last7Days", label: "7 days" },
  { value: "last30Days", label: "30 days" },
  { value: "all", label: "All" },
  { value: "custom", label: "Custom" },
];
type Theme = "light" | "dark";

function initialTheme(): Theme {
  const stored = window.localStorage.getItem("ccusage-theme");
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function detectedTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

function emptySettings(): SettingsDto {
  return {
    ccusagePath: null,
    timezone: detectedTimezone(),
    cacheTtlSeconds: 300,
    offline: true,
    autoRefreshSeconds: null,
    includeRawJson: false,
  };
}

function App() {
  const [settings, setSettings] = useState<SettingsDto>(emptySettings);
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [usage, setUsage] = useState<UsageResponse | null>(null);
  const [range, setRange] = useState<RangeKind>("last30Days");
  const [customSince, setCustomSince] = useState("");
  const [customUntil, setCustomUntil] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("cost");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiError | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [diagnostics, setDiagnostics] = useState<Diagnostics | null>(null);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);


  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem("ccusage-theme", theme);
  }, [theme]);
  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await getStatus());
    } catch {
      setStatus(null);
    }
  }, []);

  const refreshUsage = useCallback(
    async (forceRefresh = false) => {
      setLoading(true);
      setError(null);
      try {
        const response = await queryUsage({
          range,
          since: range === "custom" ? customSince || null : null,
          until: range === "custom" ? customUntil || null : null,
          timezone: settings.timezone || detectedTimezone(),
          forceRefresh,
        });
        setUsage(response);
        await refreshStatus();
      } catch (err) {
        setError(normalizeError(err));
      } finally {
        setLoading(false);
      }
    },
    [customSince, customUntil, range, refreshStatus, settings.timezone],
  );

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      setLoading(true);
      try {
        const loaded = await getSettings();
        const merged = {
          ...emptySettings(),
          ...loaded,
          timezone: loaded.timezone || detectedTimezone(),
        };
        if (!cancelled) {
          setSettings(merged);
          setStatus(await getStatus());
          const response = await queryUsage({
            range: "last30Days",
            timezone: merged.timezone,
            forceRefresh: false,
          });
          setUsage(response);
        }
      } catch (err) {
        if (!cancelled) setError(normalizeError(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    boot();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!settings.autoRefreshSeconds || settings.autoRefreshSeconds < 30) return;
    const id = window.setInterval(() => {
      refreshUsage(true);
    }, settings.autoRefreshSeconds * 1000);
    return () => window.clearInterval(id);
  }, [refreshUsage, settings.autoRefreshSeconds]);

  useEffect(() => {
    if (!usage) return;
    refreshUsage(false);
  }, [range]);

  const sortedModels = useMemo(() => {
    const rows = [...(usage?.models ?? [])];
    rows.sort((a, b) => {
      if (sortKey === "model") return a.modelName.localeCompare(b.modelName);
      if (sortKey === "input") return b.inputTokens - a.inputTokens;
      if (sortKey === "output") return b.outputTokens - a.outputTokens;
      if (sortKey === "tokens") return b.totalTokens - a.totalTokens;
      return (b.costMicroUsd ?? -1) - (a.costMicroUsd ?? -1);
    });
    return rows;
  }, [sortKey, usage?.models]);

  const maxDailyTokens = useMemo(
    () => Math.max(1, ...(usage?.daily ?? []).map((row) => row.totalTokens)),
    [usage?.daily],
  );

  async function saveSettings(next: SettingsDto) {
    setError(null);
    try {
      const saved = await updateSettings(next);
      setSettings({ ...emptySettings(), ...saved, timezone: saved.timezone || detectedTimezone() });
      await refreshStatus();
    } catch (err) {
      setError(normalizeError(err));
    }
  }

  async function handleDiagnostics() {
    setDiagnosticsOpen(true);
    setDiagnostics(null);
    try {
      setDiagnostics(await runDiagnostics());
    } catch (err) {
      setDiagnostics({
        status:
          status ?? {
            ccusageFound: false,
            settingsPath: "",
            cachePath: "",
          },
        command: [],
        error: normalizeError(err),
      });
    }
  }

  async function handleClearCache() {
    try {
      await clearCache();
      await refreshUsage(true);
    } catch (err) {
      setError(normalizeError(err));
    }
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <div className="eyebrow">Local usage monitor</div>
          <h1>ccusage UI</h1>
        </div>
        <div className="toolbar">
          <StatusPill status={status} version={usage?.ccusageVersion} />
          <button
            className="icon-button"
            type="button"
            title={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
            aria-label={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          >
            {theme === "dark" ? <Sun size={18} /> : <Moon size={18} />}
          </button>
          <button className="icon-button" type="button" title="Run diagnostics" onClick={handleDiagnostics}>
            <Terminal size={18} />
          </button>
          <button className="icon-button" type="button" title="Settings" onClick={() => setShowSettings(true)}>
            <Settings size={18} />
          </button>
          <button
            className="primary-button"
            type="button"
            disabled={loading}
            onClick={() => refreshUsage(true)}
          >
            <RefreshCw className={loading ? "spin" : ""} size={17} />
            Refresh
          </button>
        </div>
      </header>

      <section className="control-strip">
        <div className="segmented" aria-label="Date range">
          {rangeOptions.map((option) => (
            <button
              key={option.value}
              className={range === option.value ? "active" : ""}
              type="button"
              onClick={() => setRange(option.value)}
            >
              {option.label}
            </button>
          ))}
        </div>
        {range === "custom" ? (
          <div className="date-fields">
            <input
              aria-label="Since"
              type="date"
              value={customSince}
              onChange={(event) => setCustomSince(event.target.value)}
            />
            <input
              aria-label="Until"
              type="date"
              value={customUntil}
              onChange={(event) => setCustomUntil(event.target.value)}
            />
            <button type="button" onClick={() => refreshUsage(true)}>
              Apply
            </button>
          </div>
        ) : null}
        <div className="refresh-meta">
          Last refreshed: {formatDateTime(usage?.lastRefreshed)}
          {usage?.fromCache ? <span>Cached</span> : null}
          {usage?.stale ? <span className="warning-text">Stale</span> : null}
        </div>
      </section>

      {error ? <ErrorBanner error={error} /> : null}
      {usage?.warning ? <ErrorBanner error={usage.warning} compact /> : null}

      <section className="summary-grid" aria-label="Usage summary">
        <Metric label="Total cost" value={formatCost(usage?.totals.costMicroUsd)} icon={<BarChart3 size={19} />} />
        <Metric label="Total tokens" value={formatTokens(usage?.totals.totalTokens ?? 0)} icon={<Activity size={19} />} />
        <Metric label="Input tokens" value={formatTokens(usage?.totals.inputTokens ?? 0)} />
        <Metric label="Output tokens" value={formatTokens(usage?.totals.outputTokens ?? 0)} />
        <Metric
          label="Cache tokens"
          value={formatTokens((usage?.totals.cacheCreationTokens ?? 0) + (usage?.totals.cacheReadTokens ?? 0))}
        />
        <Metric
          label="Reasoning tokens"
          value={
            usage?.reasoningReported === false
              ? "Not reported"
              : formatTokens(usage?.totals.reasoningOutputTokens ?? 0)
          }
        />
      </section>
      <p className="summary-note">
        Total tokens include input, output, cache creation/read, and reasoning tokens when ccusage reports them.
        The all-agent ccusage report may omit reasoning even when provider-specific Codex output includes it.
      </p>

      <section className="content-grid">
        <div className="panel models-panel">
          <div className="panel-heading">
            <div>
              <h2>Usage by model</h2>
              <p>{sortedModels.length} models detected</p>
            </div>
            <select value={sortKey} onChange={(event) => setSortKey(event.target.value as SortKey)}>
              <option value="cost">Sort by cost</option>
              <option value="tokens">Sort by tokens</option>
              <option value="input">Sort by input</option>
              <option value="output">Sort by output</option>
              <option value="model">Sort by model</option>
            </select>
          </div>
          <div className="legend-row" aria-label="Usage bar legend">
            <span><i className="legend-swatch token" />Token share</span>
            <span><i className="legend-swatch cost" />Cost share</span>
          </div>

          {loading && !usage ? <SkeletonRows /> : null}
          {!loading && usage && sortedModels.length === 0 ? (
            <div className="empty-state">No usage data found for this range.</div>
          ) : null}
          {sortedModels.length > 0 ? (
            <ModelTable
              rows={sortedModels}
              totalCost={usage?.totals.costMicroUsd ?? null}
              totalTokens={usage?.totals.totalTokens ?? 0}
            />
          ) : null}
        </div>

        <aside className="panel trend-panel">
          <div className="panel-heading">
            <div>
              <h2>Daily trend</h2>
              <p>Token volume and cost</p>
            </div>
          </div>
          {usage?.daily.length ? (
            <div className="trend-chart">
              {usage.daily.map((row) => (
                <div className="trend-column" key={row.period} title={`${row.period}: ${formatTokens(row.totalTokens)}`}>
                  <div
                    className="trend-bar"
                    style={{ height: `${Math.max(4, percent(row.totalTokens, maxDailyTokens))}%` }}
                  />
                  <span>{row.period.slice(5)}</span>
                </div>
              ))}
            </div>
          ) : (
            <div className="empty-state small">No trend data yet.</div>
          )}
          <div className="trend-total">
            <span>Range cost</span>
            <strong>{formatCost(usage?.totals.costMicroUsd)}</strong>
          </div>
        </aside>
      </section>

      {showSettings ? (
        <SettingsPanel
          settings={settings}
          onClose={() => setShowSettings(false)}
          onSave={saveSettings}
          onClearCache={handleClearCache}
        />
      ) : null}

      {diagnosticsOpen ? (
        <DiagnosticsPanel
          diagnostics={diagnostics}
          onClose={() => setDiagnosticsOpen(false)}
        />
      ) : null}
    </main>
  );
}

function normalizeError(err: unknown): ApiError {
  if (typeof err === "object" && err !== null && "code" in err && "message" in err) {
    return err as ApiError;
  }
  if (err instanceof Error) {
    return { code: "frontend", message: err.message, retryable: false };
  }
  return { code: "unknown", message: String(err), retryable: false };
}

function StatusPill({ status, version }: { status: AppStatus | null; version?: string | null }) {
  if (!status) {
    return <span className="status-pill neutral">Checking</span>;
  }
  return (
    <span className={`status-pill ${status.ccusageFound ? "ok" : "bad"}`}>
      {status.ccusageFound ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
      {status.ccusageFound ? version || status.ccusageVersion || "ccusage found" : "ccusage missing"}
    </span>
  );
}

function ErrorBanner({ error, compact = false }: { error: ApiError; compact?: boolean }) {
  return (
    <section className={`error-banner ${compact ? "compact" : ""}`}>
      <AlertTriangle size={18} />
      <div>
        <strong>{error.message}</strong>
        {error.details ? <p>{error.details}</p> : null}
      </div>
    </section>
  );
}

function Metric({ label, value, icon }: { label: string; value: string; icon?: React.ReactNode }) {
  return (
    <div className="metric-card">
      <div className="metric-icon">{icon}</div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function ModelTable({
  rows,
  totalCost,
  totalTokens,
}: {
  rows: ModelUsage[];
  totalCost: number | null;
  totalTokens: number;
}) {
  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Model</th>
            <th>Agent</th>
            <th>Input</th>
            <th>Output</th>
            <th>Total</th>
            <th>Cost</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const tokenShare = percent(row.totalTokens, totalTokens);
            const costShare = percent(row.costMicroUsd, totalCost);
            return (
              <tr key={`${row.agent}-${row.modelName}`}>
                <td>
                  <div className="model-name">{row.modelName}</div>
                  <div
                    className="token-split"
                    title={`${tokenShare.toFixed(1)}% of range tokens`}
                    aria-hidden="true"
                  >
                    <span className="token-share" style={{ width: `${tokenShare}%` }} />
                  </div>
                </td>
                <td>{row.agent}</td>
                <td>{formatTokens(row.inputTokens)}</td>
                <td>{formatTokens(row.outputTokens)}</td>
                <td>{formatTokens(row.totalTokens)}</td>
                <td>
                  <div className="cost-cell">
                    <span>{formatCost(row.costMicroUsd)}</span>
                    <div className="cost-bar" title={`${costShare.toFixed(1)}% of range cost`}>
                      <span style={{ width: `${costShare}%` }} />
                    </div>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function SkeletonRows() {
  return (
    <div className="skeleton-list">
      {Array.from({ length: 6 }).map((_, index) => (
        <div className="skeleton-row" key={index} />
      ))}
    </div>
  );
}

function SettingsPanel({
  settings,
  onClose,
  onSave,
  onClearCache,
}: {
  settings: SettingsDto;
  onClose: () => void;
  onSave: (settings: SettingsDto) => Promise<void>;
  onClearCache: () => Promise<void>;
}) {
  const [draft, setDraft] = useState<SettingsDto>(settings);
  const [saving, setSaving] = useState(false);

  async function submit() {
    setSaving(true);
    await onSave({
      ...draft,
      ccusagePath: draft.ccusagePath?.trim() || null,
      timezone: draft.timezone?.trim() || null,
    });
    setSaving(false);
    onClose();
  }

  return (
    <div className="overlay" role="dialog" aria-modal="true" aria-label="Settings">
      <section className="drawer">
        <div className="drawer-heading">
          <h2>Settings</h2>
          <button className="icon-button" type="button" title="Close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <label>
          ccusage path
          <input
            value={draft.ccusagePath ?? ""}
            placeholder="Auto-detect"
            onChange={(event) => setDraft({ ...draft, ccusagePath: event.target.value })}
          />
        </label>
        <label>
          Timezone
          <input
            value={draft.timezone ?? ""}
            placeholder={detectedTimezone()}
            onChange={(event) => setDraft({ ...draft, timezone: event.target.value })}
          />
        </label>
        <label>
          Cache TTL seconds
          <input
            type="number"
            min={0}
            value={draft.cacheTtlSeconds}
            onChange={(event) =>
              setDraft({ ...draft, cacheTtlSeconds: Number(event.target.value) || 0 })
            }
          />
        </label>
        <label>
          Auto refresh seconds
          <input
            type="number"
            min={0}
            value={draft.autoRefreshSeconds ?? ""}
            placeholder="Off"
            onChange={(event) =>
              setDraft({
                ...draft,
                autoRefreshSeconds: event.target.value ? Number(event.target.value) : null,
              })
            }
          />
        </label>
        <label className="switch-row">
          <span>Use offline pricing cache</span>
          <input
            type="checkbox"
            checked={draft.offline}
            onChange={(event) => setDraft({ ...draft, offline: event.target.checked })}
          />
        </label>
        <label className="switch-row">
          <span>Include raw JSON in cache</span>
          <input
            type="checkbox"
            checked={draft.includeRawJson}
            onChange={(event) => setDraft({ ...draft, includeRawJson: event.target.checked })}
          />
        </label>
        <div className="drawer-actions">
          <button className="danger-button" type="button" onClick={onClearCache}>
            <Trash2 size={16} />
            Clear cache
          </button>
          <button className="primary-button" type="button" disabled={saving} onClick={submit}>
            Save
          </button>
        </div>
      </section>
    </div>
  );
}

function DiagnosticsPanel({
  diagnostics,
  onClose,
}: {
  diagnostics: Diagnostics | null;
  onClose: () => void;
}) {
  const text = diagnostics ? JSON.stringify(diagnostics, null, 2) : "Running diagnostics...";

  return (
    <div className="overlay" role="dialog" aria-modal="true" aria-label="Diagnostics">
      <section className="drawer diagnostics-drawer">
        <div className="drawer-heading">
          <h2>Diagnostics</h2>
          <button className="icon-button" type="button" title="Close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>
        <pre>{text}</pre>
        <div className="drawer-actions">
          <button className="secondary-button" type="button" onClick={() => navigator.clipboard.writeText(text)}>
            <Clipboard size={16} />
            Copy
          </button>
        </div>
      </section>
    </div>
  );
}

export default App;

