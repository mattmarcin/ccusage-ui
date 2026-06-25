export function formatTokens(value: number): string {
  if (!Number.isFinite(value)) return "0";
  if (value >= 1_000_000_000) return `${(value / 1_000_000_000).toFixed(2)}B`;
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`;
  if (value >= 10_000) return `${(value / 1_000).toFixed(1)}K`;
  return Math.round(value).toLocaleString();
}

export function formatCost(microUsd: number | null | undefined): string {
  if (microUsd === null || microUsd === undefined) return "n/a";
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: microUsd < 100_000 ? 4 : 2,
    maximumFractionDigits: microUsd < 100_000 ? 4 : 2,
  }).format(microUsd / 1_000_000);
}

export function formatDateTime(value: string | null | undefined): string {
  if (!value) return "Never";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit",
    month: "short",
    day: "numeric",
  }).format(date);
}

export function percent(part: number | null | undefined, whole: number | null | undefined): number {
  if (!part || !whole) return 0;
  return Math.max(0, Math.min(100, (part / whole) * 100));
}

