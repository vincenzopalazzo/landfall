import { describe, it, expect } from "vitest";
import { fmtThs, fmtUptime, rejectPct, fmtBtc, fmtDifficulty, shortHex, isCleartextRemote } from "./datumFormat";

describe("DATUM formatting helpers", () => {
  it("scales TH/s up to PH/s and EH/s", () => {
    expect(fmtThs(215.371)).toEqual({ v: "215.37", u: "Th/s" });
    expect(fmtThs(1234.5)).toEqual({ v: "1.23", u: "Ph/s" });
    expect(fmtThs(2_500_000)).toEqual({ v: "2.50", u: "Eh/s" });
    expect(fmtThs(NaN)).toEqual({ v: "0.00", u: "Th/s" });
  });

  it("shows uptime as its two largest units", () => {
    expect(fmtUptime(30)).toBe("30s");
    expect(fmtUptime(42 * 60 + 5)).toBe("42m");
    expect(fmtUptime(5 * 3600 + 12 * 60)).toBe("5h 12m");
    expect(fmtUptime(3 * 86400 + 4 * 3600 + 59)).toBe("3d 4h");
    expect(fmtUptime(-1)).toBe("0s");
  });

  it("computes the reject rate by difficulty, 0 before any share", () => {
    expect(rejectPct(0, 0)).toBe(0);
    expect(rejectPct(990, 10)).toBeCloseTo(1);
  });

  it("formats sats, difficulty and long hex", () => {
    expect(fmtBtc(313512345)).toBe("3.13512345");
    expect(fmtDifficulty(146472570619930.8)).toBe("146.47 T");
    expect(shortHex("f21f2f0ef0aa1970468f22bad9bb7f45")).toBe("f21f2f0e…d9bb7f45");
    expect(shortHex("abcd")).toBe("abcd");
  });

  it("flags plain http only when it leaves the local network", () => {
    for (const local of [
      "http://127.0.0.1:7152",
      "http://localhost:7152",
      "http://192.168.1.10:7152",
      "http://10.0.0.5:7152",
      "http://172.20.0.2:7152",
      "http://100.101.102.103:7152", // Tailscale / CGNAT
      "http://umbrel:7152",
      "http://umbrel.local:7152",
      "http://[fd00::10]:7152",
      "http://[::1]:7152",
      "https://datum.example.com", // TLS
      "not a url",
    ]) {
      expect(isCleartextRemote(local), local).toBe(false);
    }
    for (const remote of ["http://203.0.113.7:7152", "http://datum.example.com:7152", "http://[2001:db8::1]:7152"]) {
      expect(isCleartextRemote(remote), remote).toBe(true);
    }
  });
});
