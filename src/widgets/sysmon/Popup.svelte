<script lang="ts">
  import { updateWidgetSettings } from '../../lib/config.svelte';
  import OptionsPopover from '../../lib/OptionsPopover.svelte';
  import { fmtTemp, isHot } from './temp';
  import Sparkline from '../../lib/Sparkline.svelte';
  import type { PopupProps } from '../types';
  import type { SysmonSettings } from './index';
  import { liveSamples, rate } from './samples.svelte';
  import { Cpu, HardDrive, MemoryStick, ArrowDownUp, Gpu } from '@lucide/svelte';
  import { bytes, letter, level, levelColor, pickDrives, usedPct } from './disk';
  import { liveDrives } from './drives.svelte';
  import RecycleCard from './RecycleCard.svelte';
  import Boost from '../../lib/Boost.svelte';

  let { instance, settings }: PopupProps<SysmonSettings> = $props();
  const s = liveSamples();
  const last = $derived(s.list.at(-1));
  const netMax = $derived(Math.max(1, ...s.list.map((x) => Math.max(x.rxBps, x.txBps))));
  const drives = liveDrives(30_000);
  const onBar = $derived(new Set(pickDrives(drives.list, settings.drives).map((d) => d.mount)));
  function toggleDrive(mount: string) {
    const next = new Set(onBar);
    if (next.has(mount)) { if (next.size === 1) return; next.delete(mount); } else next.add(mount);
    updateWidgetSettings(instance, { drives: [...next] });
  }
</script>

<OptionsPopover title="Show on the bar" values={settings} save={(p) => updateWidgetSettings(instance, p)} options={[
  { type: 'checks', label: 'Items', min: 1, items: [{ key: 'cpu', label: 'CPU' }, { key: 'ram', label: 'RAM' }, { key: 'network', label: 'Network' }, { key: 'disk', label: 'Disk' }] },
  { type: 'toggle', key: 'temp', label: 'CPU temperature after CPU %', hint: 'From Windows thermal zones; N/A if this PC exposes none' },
  { type: 'choice', key: 'tempUnit', label: 'Unit', choices: [{ value: 'c', label: '°C' }, { value: 'f', label: '°F' }] },
  { type: 'choice', key: 'tempWarn', label: 'Warn at', choices: [{ value: '70', label: '70°C' }, { value: '80', label: '80°C' }, { value: '90', label: '90°C' }] },
  { type: 'toggle', key: 'gpu', label: 'GPU graph', hint: 'Shown below; uses Windows performance counters' },
  { type: 'toggle', key: 'recycle', label: 'Recycle Bin size on the bar', hint: 'Checked once a minute' },
]} />

<div class="grid">
  <div class="card c"><span class="dim h"><Cpu size={13} /> CPU</span><b>{Math.round(last?.cpu ?? 0)}%{#if settings.temp}<span class="t" class:hot={isHot(last?.cpuTemp, +settings.tempWarn)}> · {fmtTemp(last?.cpuTemp, settings.tempUnit)}</span>{/if}</b><Sparkline values={s.list.map((x) => x.cpu)} width={130} height={40} /></div>
  <div class="card c"><span class="dim h"><MemoryStick size={13} /> Memory</span><b>{Math.round(last?.mem ?? 0)}%</b><Sparkline values={s.list.map((x) => x.mem)} width={130} height={40} color="#bf5af2" /></div>
  {#if settings.gpu}
    <div class="card c"><span class="dim h"><Gpu size={13} /> GPU</span><b>{last?.gpu == null ? '–' : `${Math.round(last.gpu)}%`}</b><Sparkline values={s.list.map((x) => x.gpu ?? 0)} width={130} height={40} color="#30d158" /></div>
  {/if}
  <div class="card c"><span class="dim h"><ArrowDownUp size={13} /> Network</span><b class="small">↓ {rate(last?.rxBps ?? 0)}<br />↑ {rate(last?.txBps ?? 0)}</b>
    <Sparkline values={s.list.map((x) => x.rxBps)} max={netMax} width={130} height={40} color="#64d2ff" /></div>
</div>

<Boost compact />

<h4 class="dim h"><HardDrive size={13} /> Storage</h4>
{#each drives.list as d (d.mount)}
  {@const p = usedPct(d)}
  {@const lv = level(p, +settings.diskHigh, +settings.diskCritical)}
  <div class="card drive">
    <div class="row">
      <b>{letter(d.mount)}</b><span class="dim ellipsis">{d.label}{d.removable ? ' · removable' : ''}</span>
      <label class="dim on" title="Show this drive on the bar"><input type="checkbox" checked={settings.disk && onBar.has(d.mount)}
        onchange={() => (settings.disk ? toggleDrive(d.mount) : updateWidgetSettings(instance, { disk: true, drives: [d.mount] }))} /> bar</label>
    </div>
    <div class="track"><span class="fill" style:width="{p}%" style:background={levelColor[lv]}></span></div>
    <div class="row dim small"><span>{bytes(d.total - d.available)} of {bytes(d.total)} used</span><span>{p}% · {bytes(d.available)} free</span></div>
  </div>
{:else}
  <p class="dim small">{drives.loaded ? 'Windows reported no drives.' : 'Loading…'}</p>
{/each}

<RecycleCard />

<style>
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-top: 10px; }
  .c { display: flex; flex-direction: column; gap: 4px; padding: 10px 12px; }
  .c b { font-size: 20px; font-variant-numeric: tabular-nums; }
  .t { font-size: 13px; font-weight: 500; opacity: 0.8; }
  .hot { color: #ff453a; opacity: 1; }
  .h { display: inline-flex; align-items: center; gap: 5px; }
  h4 { margin: 14px 2px 6px; font-size: 12px; font-weight: 500; }
  .drive { padding: 9px 12px; margin-bottom: 6px; display: flex; flex-direction: column; gap: 5px; }
  .row { display: flex; align-items: center; gap: 8px; justify-content: space-between; }
  .row .ellipsis { flex: 1; font-size: 12px; }
  .on { display: inline-flex; gap: 4px; align-items: center; font-size: 11px; }
  .track { height: 7px; border-radius: calc(4px * var(--rs)); background: var(--track); overflow: hidden; }
  .fill { display: block; height: 100%; border-radius: calc(4px * var(--rs)); }
  .small { font-size: 11px; }
  .c b.small { font-size: 12px; line-height: 1.4; }
</style>
