#!/usr/bin/env python3
"""Compare two experimental binaries on an already configured isolated display."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import random
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--gtk3', type=Path, required=True)
    parser.add_argument('--gtk4', type=Path, required=True)
    parser.add_argument('--metadata', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repeats', type=int, default=5)
    parser.add_argument('--warmup', type=float, default=10)
    parser.add_argument('--seconds', type=float, default=60)
    args = parser.parse_args()
    if args.repeats < 1 or not 0 < args.warmup < float('inf') or not 0 < args.seconds < float('inf'):
        parser.error('repeats, warmup and seconds must be positive and finite')
    binaries = {'gtk3': args.gtk3.resolve(strict=True), 'gtk4': args.gtk4.resolve(strict=True)}
    metadata = json.loads(args.metadata.read_text())
    args.output.mkdir(parents=True, exist_ok=False)
    schedule = [(name, mode, repeat) for name in binaries for mode in ('static', 'animated')
                for repeat in range(args.repeats)]
    random.Random(20261105).shuffle(schedule)
    manifest = {'status': 'incomplete', 'seed': 20261105, 'schedule': schedule,
                'binary_sha256': {k: hashlib.sha256(v.read_bytes()).hexdigest() for k, v in binaries.items()},
                'display_settings': {k: os.environ.get(k) for k in ('GDK_BACKEND', 'GSK_RENDERER', 'GTK_A11Y')},
                'completed': []}
    try:
        for name, mode, repeat in schedule:
            label = f'{name}-{mode}-{repeat+1}'
            details = dict(metadata, workload=f'{name} {mode}; 1920x1080 window, preloaded 128x128 RGBA texture')
            meta = args.output / f'{label}-metadata.json'
            meta.write_text(json.dumps(details, indent=2))
            with (args.output / f'{label}.log').open('w') as log:
                command = [str(binaries[name])] + (['--animate'] if mode == 'animated' else [])
                process = subprocess.Popen(command, stdout=log, stderr=log)
                try:
                    subprocess.run([sys.executable, str(ROOT / 'tools/desktop_baseline.py'),
                                    '--pid', str(process.pid), '--metadata', str(meta),
                                    '--output', str(args.output / f'{label}.json'),
                                    '--warmup', str(args.warmup), '--seconds', str(args.seconds),
                                    '--interval', str(min(1, args.seconds))], check=True)
                    if process.poll() is not None:
                        raise RuntimeError(f'{label} exited during collection')
                finally:
                    if process.poll() is None:
                        process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
            manifest['completed'].append(label)
            print(f'completed {label}', flush=True)
        manifest['status'] = 'sampling_completed_not_desktop_acceptance'
    finally:
        (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


if __name__ == '__main__':
    main()
