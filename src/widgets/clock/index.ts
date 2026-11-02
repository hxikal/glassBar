import { Clock } from '@lucide/svelte';
import { defineWidget } from '../types';
import Pill from './Pill.svelte';
import Popup from './Popup.svelte';
import SettingsForm from './SettingsForm.svelte';
import type { Zone } from './zones';
import type { AmPm, DateOpts, DateStyle } from './format';

export interface ClockSettings {
  /** 'system' follows Windows' regional setting (24-hour if unknown). Unset in older configs:
   *  then the old `hour12` switch decides. */
  timeFormat?: 'system' | '24' | '12';
  /** Older 12/24-hour switch, still read when `timeFormat` is unset. */
  hour12: boolean;
  ampm: AmPm;
  ampmSize: 'small' | 'same' | 'hidden';
  /** 'auto' = 02:05 in 24-hour, 2:05 in 12-hour. */
  leadingZero: 'auto' | 'on' | 'off';
  showSeconds: boolean;
  worldSeconds: boolean;
  dateStyle: DateStyle;
  dateSep: DateOpts['sep'];
  weekday: DateOpts['weekday'];
  datePattern: string;
  /** What the pill shows (the small size always shows the time only). */
  show: 'time' | 'date' | 'both';
  layout: 'inline' | 'stacked';
  /** Side by side: time on the left or right (stacked: time on top or below). */
  timeSide: 'left' | 'right';
  /** Stacked: line up time and date on the left, centre or right. */
  stackAlign: 'left' | 'center' | 'right';
  /** Sun / moon next to each world clock. */
  dayNight: boolean;
  /** standard pill · prominent (bigger, bolder, accent panel) · popup (hover shows a large clock) */
  appearance: 'standard' | 'prominent' | 'popup';
  /** World clocks, in the user's order. */
  zones: Zone[];
  /** Older single "second time zone" setting; folded into `zones` when read. */
  secondTz: string;
  /** Weather on the left of the pill (was the Weather widget). */
  showWeather: boolean;
  weatherUnit: 'c' | 'f';
  weatherColorful: boolean;
  /** Next Malaysian public holiday beside the clock (the panel always lists them). */
  holidays: boolean;
  /** The list of upcoming holidays in the clock's panel. */
  holidayList: boolean;
  /** 'auto' = the state of the shared location; else a state name (holidays.ts STATES). */
  holidayState: string;
  /** Show it on the clock when it is at most this many days away. */
  holidayDays: '7' | '14' | '30';
}

export default defineWidget<ClockSettings>({
  id: 'clock',
  name: 'Clock & Date',
  description: 'Time and date in your format, calendar, world clocks with day/night, weather, Malaysian public holidays',
  category: 'productivity',
  icon: Clock,
  sizes: ['small', 'medium', 'large'],
  defaultSize: 'medium',
  defaults: {
    hour12: false, ampm: 'PM', ampmSize: 'small', leadingZero: 'auto', showSeconds: false, worldSeconds: false,
    dateStyle: 'weekday', dateSep: '/', weekday: 'none', datePattern: 'ddd, D MMM', show: 'both', layout: 'inline', timeSide: 'left', stackAlign: 'center',
    dayNight: true, appearance: 'standard', zones: [], secondTz: '', showWeather: true, weatherUnit: 'c', weatherColorful: true,
    holidays: true, holidayList: true, holidayState: 'auto', holidayDays: '14',
  },
  Pill,
  Popup: { component: Popup, width: 340, height: 640 },
  SettingsForm,
});
