// Widget categories for Settings › Widgets. A widget names its category in its manifest;
// one without a known category lands in "Other", so new widgets always show up.
export const CATEGORIES = [
  { id: 'system', name: 'System & Hardware' },
  { id: 'productivity', name: 'Productivity & Time' },
  { id: 'apps', name: 'Applications & Shortcuts' },
  { id: 'connectivity', name: 'Connectivity & Quick Controls' },
  { id: 'media', name: 'Media & Notifications' },
  { id: 'info', name: 'Weather & Information' },
  { id: 'ai', name: 'AI' },
  { id: 'other', name: 'Other' },
] as const;

export type CategoryId = (typeof CATEGORIES)[number]['id'];

export const categoryOf = (m: { category?: string }): CategoryId =>
  CATEGORIES.some((c) => c.id === m.category) ? (m.category as CategoryId) : 'other';
