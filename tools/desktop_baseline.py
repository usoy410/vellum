#!/usr/bin/env python3
"""Read-only Linux process-tree sampler. Does not launch or stop wallpaper apps."""
import argparse
import datetime
import json
import os
from pathlib import Path
import platform
import time

REQUIRED_METADATA = {'compositor', 'outputs', 'gpu_driver', 'power_mode', 'workload', 'binary_revision'}


def parse_stat(text):
    # comm can contain spaces and parentheses; fields after the last ')' are fixed.
    fields = text[text.rindex(')') + 2:].split()
    return {'ppid': int(fields[1]), 'ticks': int(fields[11]) + int(fields[12]),
            'start': int(fields[19])}


def descendants(stats, root):
    selected = {root}
    while True:
        added = {pid for pid, stat in stats.items() if stat['ppid'] in selected} - selected
        if not added:
            return selected
        selected.update(added)


def memory(text):
    values = {}
    for line in text.splitlines():
        key, _, value = line.partition(':')
        if key in ('Rss', 'Pss'):
            values[key.lower() + '_kib'] = int(value.split()[0])
    if len(values) != 2:
        raise ValueError('smaps_rollup lacks RSS/PSS')
    return values


def snapshot(pid, expected_start, proc=Path('/proc')):
    stats, errors = {}, 0
    for directory in proc.iterdir():
        if not directory.name.isdecimal():
            continue
        try:
            stats[int(directory.name)] = parse_stat((directory / 'stat').read_text())
        except (FileNotFoundError, ProcessLookupError):
            continue
        except (OSError, ValueError, IndexError):
            errors += 1
    if pid not in stats or stats[pid]['start'] != expected_start:
        raise RuntimeError('root process exited or PID was reused; discard this run')
    processes = []
    for child in sorted(descendants(stats, pid)):
        record = {'pid': child, **stats[child], 'rss_kib': None, 'pss_kib': None}
        try:
            record.update(memory((proc / str(child) / 'smaps_rollup').read_text()))
            after = parse_stat((proc / str(child) / 'stat').read_text())
            if after['start'] != record['start']:
                raise ValueError('PID reused during memory sample')
        except (OSError, ValueError, IndexError) as error:
            record.update(rss_kib=None, pss_kib=None, memory_error=type(error).__name__)
        processes.append(record)
    return {'monotonic_seconds': time.monotonic(), 'processes': processes,
            'proc_scan_errors': errors}


def summarize(previous, current, ticks_per_second):
    before = {(p['pid'], p['start']): p for p in previous['processes']}
    after = {(p['pid'], p['start']): p for p in current['processes']}
    changed = before.keys() != after.keys()
    elapsed = current['monotonic_seconds'] - previous['monotonic_seconds']
    cpu = None
    if not changed and not previous['proc_scan_errors'] and not current['proc_scan_errors'] and elapsed > 0:
        delta = sum(after[k]['ticks'] - before[k]['ticks'] for k in after)
        if delta >= 0:
            cpu = 100 * delta / ticks_per_second / elapsed
    result = {'tree_changed': changed, 'cpu_one_core_percent': cpu}
    for metric in ('rss_kib', 'pss_kib'):
        values = [p[metric] for p in current['processes']]
        result[metric] = sum(values) if all(v is not None for v in values) and not current['proc_scan_errors'] else None
    return result


def positive(value):
    number = float(value)
    if not 0 < number < float('inf'):
        raise argparse.ArgumentTypeError('must be finite and positive')
    return number


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--metadata', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--warmup', type=positive, default=10)
    parser.add_argument('--seconds', type=positive, default=60)
    parser.add_argument('--interval', type=positive, default=1)
    args = parser.parse_args()
    metadata = json.loads(args.metadata.read_text())
    if not isinstance(metadata, dict) or not REQUIRED_METADATA <= metadata.keys():
        parser.error('metadata must include: ' + ', '.join(sorted(REQUIRED_METADATA)))
    if args.interval > args.seconds:
        parser.error('interval must not exceed duration')
    start = parse_stat(Path(f'/proc/{args.pid}/stat').read_text())['start']
    report = {'schema_version': 1, 'created_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'kernel': platform.release(), 'machine': platform.machine(),
              'os_release': Path('/etc/os-release').read_text(), 'metadata': metadata,
              'parameters': {'warmup_seconds': args.warmup, 'duration_seconds': args.seconds, 'interval_seconds': args.interval},
              'scope': 'observed root and descendants; not service cgroup or GPU memory',
              'acceptance': 'not assessed; complete desktop checklist separately',
              'status': 'incomplete', 'samples': []}
    # Exclusive creation preserves previous measurements, including failed runs.
    with args.output.open('x') as output:
        try:
            time.sleep(args.warmup)
            previous = snapshot(args.pid, start)
            report['initial_sample'] = previous
            deadline = time.monotonic() + args.seconds
            while time.monotonic() < deadline:
                time.sleep(min(args.interval, max(0, deadline - time.monotonic())))
                current = snapshot(args.pid, start)
                current['aggregate'] = summarize(previous, current, os.sysconf('SC_CLK_TCK'))
                report['samples'].append(current)
                previous = current
            report['status'] = 'sampling_completed'
        except (OSError, RuntimeError, KeyboardInterrupt) as error:
            report['error'] = str(error) or type(error).__name__
            raise
        finally:
            json.dump(report, output, indent=2)
            output.write('\n')


if __name__ == '__main__':
    main()
