<script lang="ts">
  import type { SettingsProps } from '../types';
  import type { ClockSettings } from './index';
  import FormatOptions from './FormatOptions.svelte';
  import WeatherOptions from './WeatherOptions.svelte';
  import HolidayOptions from './HolidayOptions.svelte';
  import LocationTag from '../../lib/LocationTag.svelte';
  import { zonesOf } from './zones';

  let { settings, save }: SettingsProps<ClockSettings> = $props();
  const zones = $derived(zonesOf(settings));
</script>

<FormatOptions {settings} {save} />
<WeatherOptions {settings} {save} />
<HolidayOptions {settings} {save} />
{#if settings.showWeather}<div>Weather location: <LocationTag /></div>{/if}
<p class="dim" style="font-size:12px;margin:0">
  World clocks: {zones.length ? zones.map((z) => z.label).join(', ') : 'none yet'}. Add, remove and reorder them in the clock's panel (click the clock, then +).
</p>
