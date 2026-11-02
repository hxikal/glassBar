<script lang="ts">
  // The month grid (Hijri days, week numbers) and public holidays: on/off, state, how far ahead the pill shows one.
  import OptionsPopover from '../../lib/OptionsPopover.svelte';
  import type { ClockSettings } from './index';
  import { stateOf, STATES } from './holidays';
  import { useLocation } from '../../lib/location.svelte';

  let { settings, save }: { settings: ClockSettings; save: (p: Partial<ClockSettings>) => void } = $props();
  const here = useLocation();
  const found = $derived(stateOf(here.s?.place?.name));
</script>

<OptionsPopover title="Calendar & holidays" values={settings} save={(p) => save(p as Partial<ClockSettings>)} options={[
  { type: 'toggle', key: 'calHijri', label: 'Hijri dates in the calendar', hint: 'Umm al-Qura day under each date' },
  { type: 'toggle', key: 'calWeeks', label: 'Week numbers in the calendar' },
  { type: 'toggle', key: 'holidays', label: 'Show the next holiday on the clock', hint: 'Next to the clock when one is coming up' },
  { type: 'toggle', key: 'holidayList', label: 'Show the holiday list in the panel', hint: 'Upcoming holidays under the calendar' },
  { type: 'choice', key: 'holidayState', label: 'State', menu: true,
    choices: [{ value: 'auto', label: found ? `From my location (${found})` : 'From my location (national only)' }, ...STATES.map((s) => ({ value: s, label: s }))] },
  ...(settings.holidays ? [
    { type: 'choice' as const, key: 'holidayDays', label: 'Show when within', choices: [{ value: '7', label: '1 week' }, { value: '14', label: '2 weeks' }, { value: '30', label: '1 month' }] },
  ] : []),
]} />
