// Formatting and safety helpers for the DATUM tab (Datum.svelte). Pure, so
// they're unit-tested on their own. Hashrates arrive in TH/s: that's the
// unit DATUM Gateway computes them in.

export const fmtInt = (n: number): string => Math.round(n).toLocaleString("en-US");

/// TH/s, scaled up to PH/s or EH/s so a farm doesn't read "123456.00 Th/s".
export function fmtThs(ths: number): { v: string; u: string } {
  const n = Number.isFinite(ths) ? ths : 0;
  if (n >= 1e6) return { v: (n / 1e6).toFixed(2), u: "Eh/s" };
  if (n >= 1e3) return { v: (n / 1e3).toFixed(2), u: "Ph/s" };
  return { v: n.toFixed(2), u: "Th/s" };
}

/// Seconds → the two largest units: "3d 4h", "5h 12m", "42m", "30s".
export function fmtUptime(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}

/// Rejected share difficulty as a percentage of all submitted difficulty,
/// the same ratio DATUM's own dashboard shows. 0 before anything was sent.
export function rejectPct(acceptedDiff: number, rejectedDiff: number): number {
  const total = acceptedDiff + rejectedDiff;
  return total > 0 ? (rejectedDiff / total) * 100 : 0;
}

export const fmtBtc = (sats: number): string => (sats / 1e8).toFixed(8);

/// Network difficulty in the short form explorers use ("146.47 T").
export function fmtDifficulty(d: number): string {
  if (d >= 1e12) return `${(d / 1e12).toFixed(2)} T`;
  if (d >= 1e9) return `${(d / 1e9).toFixed(2)} G`;
  return fmtInt(d);
}

/// "f21f2f0e…3003": enough to recognize a key or hash, short enough to fit.
export function shortHex(hex: string, keep = 8): string {
  return hex.length > keep * 2 + 1 ? `${hex.slice(0, keep)}…${hex.slice(-keep)}` : hex;
}

/// True when `url` is plain http:// to a host outside the local network, so
/// the gateway's answers cross the internet unencrypted. LAN addresses
/// (RFC 1918, link-local, CGNAT/Tailscale, ULA), single-label and `.local`
/// names, and `.onion` (Tor encrypts) don't count.
export function isCleartextRemote(url: string): boolean {
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return false;
  }
  if (u.protocol !== "http:") return false;
  const host = u.hostname.toLowerCase().replace(/^\[|\]$/g, "");
  if (host === "localhost") return false;
  const v4 = host.match(/^(\d+)\.(\d+)\.(\d+)\.(\d+)$/);
  if (v4) {
    const [a, b] = [Number(v4[1]), Number(v4[2])];
    const local =
      a === 10 ||
      a === 127 ||
      (a === 172 && b >= 16 && b <= 31) ||
      (a === 192 && b === 168) ||
      (a === 169 && b === 254) ||
      (a === 100 && b >= 64 && b <= 127);
    return !local;
  }
  if (host.includes(":")) return !(host === "::1" || /^f[cd]/.test(host) || /^fe[89ab]/.test(host));
  if (!host.includes(".")) return false; // single-label LAN name ("umbrel")
  return !/\.(local|lan|home\.arpa|internal|onion)$/.test(host);
}
