import { browser } from '$app/environment';

const KEY = 'ocean-ln-waitlist';
const TOTAL_SPOTS = 500;
const BASE_TAKEN = TOTAL_SPOTS - 214; // 214 remaining at baseline

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function isEmail(value: string): boolean {
  return EMAIL_RE.test(value);
}

/**
 * Client-side early-access waitlist. This mirrors the design prototype: it
 * persists signups to localStorage so the "spots remaining" counter reflects
 * the visitor's own submissions. Wiring this to a real backend endpoint is a
 * follow-up — swap `add()` for a POST and keep the same public surface.
 */
class Waitlist {
  emails = $state<string[]>([]);
  lastEmail = $state('');
  private loaded = false;

  /** Hydrate from localStorage. Safe to call repeatedly and during SSR. */
  load(): void {
    if (this.loaded || !browser) return;
    this.loaded = true;
    try {
      const raw = localStorage.getItem(KEY);
      const parsed: unknown = raw ? JSON.parse(raw) : [];
      if (Array.isArray(parsed)) {
        this.emails = parsed.filter((e): e is string => typeof e === 'string');
      }
    } catch {
      // Corrupt/unavailable storage — start empty rather than crashing.
      this.emails = [];
    }
  }

  /** Record a signup. Returns false if the email is invalid. */
  add(email: string): boolean {
    const value = email.trim();
    if (!isEmail(value)) return false;
    this.lastEmail = value;
    if (!this.emails.includes(value)) {
      this.emails = [...this.emails, value];
      if (browser) {
        try {
          localStorage.setItem(KEY, JSON.stringify(this.emails));
        } catch {
          // Non-fatal: keep the in-memory list even if persistence fails.
        }
      }
    }
    return true;
  }

  get spotsLeft(): number {
    return Math.max(1, TOTAL_SPOTS - BASE_TAKEN - this.emails.length);
  }

  get totalSpots(): number {
    return TOTAL_SPOTS;
  }
}

export const waitlist = new Waitlist();
