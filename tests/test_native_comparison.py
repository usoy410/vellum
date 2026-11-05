import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class Comparison(unittest.TestCase):
    def run_comparison(self, directory, body):
        binary = directory / 'fake-renderer'
        binary.write_text('#!/usr/bin/env python3\n' + body)
        binary.chmod(0o755)
        metadata = directory / 'meta.json'
        metadata.write_text(json.dumps({key: 'test' for key in
            ('compositor', 'outputs', 'gpu_driver', 'power_mode', 'workload', 'binary_revision')}))
        command = [sys.executable, str(ROOT / 'experiments/compare_native.py'),
                   '--gtk3', str(binary), '--gtk4', str(binary), '--metadata', str(metadata),
                   '--output', str(directory / 'results'), '--repeats', '1', '--warmup', '.01', '--seconds', '.01']
        return subprocess.run(command, capture_output=True, timeout=15), command

    def test_completed_runs_retain_data_and_stop_children(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            result, command = self.run_comparison(directory,
                'import os,time\nprint(os.getpid(),flush=True)\ntime.sleep(30)\n')
            self.assertEqual(result.returncode, 0, result.stderr)
            manifest = json.loads((directory / 'results/manifest.json').read_text())
            self.assertEqual(len(manifest['completed']), 4)
            for label in manifest['completed']:
                record = json.loads((directory / f'results/{label}.json').read_text())
                self.assertEqual(record['status'], 'sampling_completed')
                pid = int((directory / f'results/{label}.log').read_text())
                self.assertFalse(Path(f'/proc/{pid}').exists())
            original = (directory / 'results/manifest.json').read_text()
            self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
            self.assertEqual((directory / 'results/manifest.json').read_text(), original)

    def test_renderer_failure_is_not_a_successful_measurement(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            result, _ = self.run_comparison(directory, 'raise SystemExit(2)\n')
            self.assertNotEqual(result.returncode, 0)
            manifest = json.loads((directory / 'results/manifest.json').read_text())
            self.assertEqual(manifest['status'], 'incomplete')
            self.assertEqual(manifest['completed'], [])


if __name__ == '__main__':
    unittest.main()
