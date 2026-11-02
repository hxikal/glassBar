// Water amounts are stored in ml; shown in ml or US fluid ounces.
export type WaterUnit = 'ml' | 'oz';
const ML_PER_OZ = 29.5735;

/** "750 ml" / "1,250 ml" / "25 fl oz". */
export function fmtWater(ml: number, unit: WaterUnit = 'ml'): string {
  return unit === 'oz' ? `${Math.round(ml / ML_PER_OZ)} fl oz` : `${Math.round(ml).toLocaleString('en-US')} ml`;
}
/** Today's progress, 0..1. */
export const progressOf = (total: number, goal: number): number => (goal > 0 ? Math.max(0, Math.min(1, total / goal)) : 0);
/** Time left until a reminder, short: "now", "12m", "1h 05m". */
export function untilText(ms: number): string {
  const m = Math.ceil(ms / 60_000);
  if (m <= 0) return 'now';
  return m < 60 ? `${m}m` : `${Math.floor(m / 60)}h ${String(m % 60).padStart(2, '0')}m`;
}
