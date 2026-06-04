// Geometric 1.6-stroke icon set, ported from the design's wizard/ui.jsx.
// Values are the inner SVG markup for a 0 0 24 24 viewBox.
export const ICONS: Record<string, string> = {
  check: '<polyline points="4 12 9 17 20 6"/>',
  arrowR: '<line x1="4" y1="12" x2="20" y2="12"/><polyline points="14 6 20 12 14 18"/>',
  arrowL: '<line x1="20" y1="12" x2="4" y2="12"/><polyline points="10 6 4 12 10 18"/>',
  eye: '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/>',
  shield: '<path d="M12 3l7 3v5c0 4.4-3 7.6-7 9-4-1.4-7-4.6-7-9V6l7-3Z"/><polyline points="9 11.5 11.2 14 15 9.5"/>',
  lock: '<rect x="5" y="11" width="14" height="9" rx="1.5"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  bolt: '<polygon points="13 2 4 14 11 14 10 22 19 9 12 9 13 2"/>',
  warn: '<path d="M12 3 1.8 20.5h20.4L12 3Z"/><line x1="12" y1="10" x2="12" y2="14.5"/><circle cx="12" cy="17.6" r="0.4" fill="currentColor"/>',
  info: '<circle cx="12" cy="12" r="9"/><line x1="12" y1="11" x2="12" y2="16.5"/><circle cx="12" cy="7.8" r="0.5" fill="currentColor"/>',
  key: '<circle cx="8" cy="8" r="4"/><line x1="11" y1="11" x2="20" y2="20"/><line x1="17" y1="17" x2="19" y2="15"/>',
  wallet: '<rect x="3" y="6" width="18" height="13" rx="2"/><path d="M3 9h18"/><circle cx="16.5" cy="13.5" r="1.2" fill="currentColor" stroke="none"/>',
  copy: '<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"/>',
  pen: '<path d="M4 20h4L19 9l-4-4L4 16v4Z"/><line x1="14" y1="6" x2="18" y2="10"/>',
  download: '<line x1="12" y1="4" x2="12" y2="15"/><polyline points="7 11 12 16 17 11"/><path d="M5 20h14"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-.5 4"/><polyline points="20 5 20 11 14 11"/>',
  link: '<path d="M9 15l6-6"/><path d="M11 6l1-1a4 4 0 0 1 6 6l-1 1"/><path d="M13 18l-1 1a4 4 0 0 1-6-6l1-1"/>',
  spark: '<path d="M12 3l1.8 5.7L19.5 10l-5.7 1.8L12 17.5l-1.8-5.7L4.5 10l5.7-1.3L12 3Z"/>',
  enclave: '<rect x="4" y="4" width="16" height="16" rx="3"/><rect x="9" y="9" width="6" height="6" rx="1"/>',
  cog: '<circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.4-2.3 1a7 7 0 0 0-1.7-1L14.5 2h-5l-.4 2.6a7 7 0 0 0-1.7 1l-2.3-1-2 3.4 2 1.5a7 7 0 0 0 0 2l-2 1.5 2 3.4 2.3-1a7 7 0 0 0 1.7 1l.4 2.6h5l.4-2.6a7 7 0 0 0 1.7-1l2.3 1 2-3.4-2-1.5a7 7 0 0 0 .1-1Z"/>',
};
