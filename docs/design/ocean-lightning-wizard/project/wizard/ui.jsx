// wizard/ui.jsx — shared primitives: icon set, tooltip, copy field, callout.
const { useState, useRef, useEffect } = React;

// ── Icon set (geometric, 1.6 stroke, sharp — matches OCEAN's Material Sharp tone) ──
const ICONS = {
  check:   <polyline points="4 12 9 17 20 6" />,
  arrowR:  <g><line x1="4" y1="12" x2="20" y2="12"/><polyline points="14 6 20 12 14 18"/></g>,
  arrowL:  <g><line x1="20" y1="12" x2="4" y2="12"/><polyline points="10 6 4 12 10 18"/></g>,
  eye:     <g><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></g>,
  shield:  <g><path d="M12 3l7 3v5c0 4.4-3 7.6-7 9-4-1.4-7-4.6-7-9V6l7-3Z"/><polyline points="9 11.5 11.2 14 15 9.5"/></g>,
  lock:    <g><rect x="5" y="11" width="14" height="9" rx="1.5"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></g>,
  bolt:    <polygon points="13 2 4 14 11 14 10 22 19 9 12 9 13 2" />,
  warn:    <g><path d="M12 3 1.8 20.5h20.4L12 3Z"/><line x1="12" y1="10" x2="12" y2="14.5"/><circle cx="12" cy="17.6" r="0.4" fill="currentColor"/></g>,
  info:    <g><circle cx="12" cy="12" r="9"/><line x1="12" y1="11" x2="12" y2="16.5"/><circle cx="12" cy="7.8" r="0.5" fill="currentColor"/></g>,
  key:     <g><circle cx="8" cy="8" r="4"/><line x1="11" y1="11" x2="20" y2="20"/><line x1="17" y1="17" x2="19" y2="15"/></g>,
  wallet:  <g><rect x="3" y="6" width="18" height="13" rx="2"/><path d="M3 9h18"/><circle cx="16.5" cy="13.5" r="1.2" fill="currentColor" stroke="none"/></g>,
  copy:    <g><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"/></g>,
  pen:     <g><path d="M4 20h4L19 9l-4-4L4 16v4Z"/><line x1="14" y1="6" x2="18" y2="10"/></g>,
  download:<g><line x1="12" y1="4" x2="12" y2="15"/><polyline points="7 11 12 16 17 11"/><path d="M5 20h14"/></g>,
  refresh: <g><path d="M20 11a8 8 0 1 0-.5 4"/><polyline points="20 5 20 11 14 11"/></g>,
  link:    <g><path d="M9 15l6-6"/><path d="M11 6l1-1a4 4 0 0 1 6 6l-1 1"/><path d="M13 18l-1 1a4 4 0 0 1-6-6l1-1"/></g>,
  spark:   <path d="M12 3l1.8 5.7L19.5 10l-5.7 1.8L12 17.5l-1.8-5.7L4.5 10l5.7-1.3L12 3Z" />,
  enclave: <g><rect x="4" y="4" width="16" height="16" rx="3"/><rect x="9" y="9" width="6" height="6" rx="1"/></g>,
};
function Icon({ name, size = 18, stroke = 1.6, style }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none"
      stroke="currentColor" strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round" style={style}>
      {ICONS[name]}
    </svg>
  );
}

// ── "What's this?" inline tooltip (click to toggle, click-away to close) ──
function Tip({ label = "What's this?", children, enabled = true }) {
  const [open, setOpen] = useState(false);
  const ref = useRef(null);
  useEffect(() => {
    if (!open) return;
    const close = (e) => { if (ref.current && !ref.current.contains(e.target)) setOpen(false); };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [open]);
  if (!enabled) return null;
  return (
    <span className="wz-tip" ref={ref}>
      <button className="wz-tip-btn" onClick={() => setOpen((o) => !o)} type="button">
        <Icon name="info" size={13} />{label}
      </button>
      {open && <span className="wz-tip-pop">{children}</span>}
    </span>
  );
}

// ── Callout box ──
function Callout({ kind = "info", icon = "info", children, style }) {
  return (
    <div className={`wz-callout ${kind}`} style={style}>
      <Icon name={icon} size={17} />
      <div className="ct">{children}</div>
    </div>
  );
}

// ── Copy button with transient "Copied" state ──
function CopyBtn({ value, label = "Copy" }) {
  const [done, setDone] = useState(false);
  const copy = () => {
    try { navigator.clipboard?.writeText(value); } catch (e) {}
    setDone(true); setTimeout(() => setDone(false), 1600);
  };
  return (
    <button className={`wz-copybtn ${done ? "copied" : ""}`} onClick={copy} type="button">
      <Icon name={done ? "check" : "copy"} size={13} />{done ? "Copied" : label}
    </button>
  );
}

// ── Labeled copy field for an artifact ──
function CopyField({ label, chip, value, tip, mono = true }) {
  return (
    <div className="wz-copy">
      <div className="top">
        <span className="lbl">
          {label}
          {chip && <span className="chip">{chip}</span>}
          {tip && <Tip>{tip}</Tip>}
        </span>
        <CopyBtn value={value} />
      </div>
      <div className={`val ${mono ? "" : "muted"}`}>{value}</div>
    </div>
  );
}

// ── Small primary/secondary nav buttons ──
function Btn({ variant = "primary", icon, iconRight, children, ...rest }) {
  const cls = variant === "primary" ? "wz-btn"
    : variant === "ghost" ? "wz-btn ghost"
    : "wz-btn subtle";
  return (
    <button className={cls} type="button" {...rest}>
      {icon && <Icon name={icon} size={16} />}
      {children}
      {iconRight && <Icon name={iconRight} size={16} />}
    </button>
  );
}

Object.assign(window, { Icon, Tip, Callout, CopyBtn, CopyField, Btn });
