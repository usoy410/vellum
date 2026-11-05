import importlib.util
import json
import tempfile
import time
import pathlib
import subprocess
import sys
import unittest

spec = importlib.util.spec_from_file_location('baseline', pathlib.Path(__file__).resolve().parents[1] / 'tools/desktop_baseline.py')
b = importlib.util.module_from_spec(spec)
spec.loader.exec_module(b)


class Baseline(unittest.TestCase):
    def test_stat_with_parentheses_and_spaces(self):
        fields = ['S', '41'] + ['0'] * 18
        fields[11], fields[12], fields[19] = '3', '5', '123'
        self.assertEqual(b.parse_stat('9 (odd ) name) ' + ' '.join(fields)), {'ppid': 41, 'ticks': 8, 'start': 123})

    def test_descendants_include_grandchildren_only(self):
        self.assertEqual(b.descendants({1: {'ppid': 0}, 2: {'ppid': 1}, 3: {'ppid': 2}, 4: {'ppid': 0}}, 1), {1, 2, 3})

    def test_memory_requires_both_metrics(self):
        self.assertEqual(b.memory('Rss: 10 kB\nPss: 7 kB'), {'rss_kib': 10, 'pss_kib': 7})
        with self.assertRaises(ValueError):
            b.memory('Rss: 10 kB')

    def sample(self, ticks, start=10, pss=7, scan_errors=0):
        return {'monotonic_seconds': ticks / 100, 'proc_scan_errors': scan_errors,
                'processes': [{'pid': 1, 'start': start, 'ticks': ticks, 'rss_kib': 10, 'pss_kib': pss}]}

    def test_cpu_and_memory(self):
        self.assertEqual(b.summarize(self.sample(100), self.sample(200), 100),
                         {'tree_changed': False, 'cpu_one_core_percent': 100, 'rss_kib': 10, 'pss_kib': 7})

    def test_pid_reuse_or_membership_change_invalidates_cpu(self):
        self.assertIsNone(b.summarize(self.sample(100), self.sample(200, start=11), 100)['cpu_one_core_percent'])

    def test_missing_data_is_not_zero(self):
        self.assertIsNone(b.summarize(self.sample(100), self.sample(200, pss=None), 100)['pss_kib'])
        result = b.summarize(self.sample(100), self.sample(200, scan_errors=1), 100)
        self.assertIsNone(result['cpu_one_core_percent'])
        self.assertIsNone(result['rss_kib'])

    def test_live_child_tree_and_root_exit(self):
        program = 'import subprocess,sys; p=subprocess.Popen([sys.executable,"-c","import sys; sys.stdin.read()"],stdin=subprocess.PIPE); print(p.pid,flush=True); sys.stdin.read(); p.stdin.close(); p.wait()'
        with subprocess.Popen([sys.executable, '-c', program], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True) as root:
            child = int(root.stdout.readline())
            start = b.parse_stat(pathlib.Path(f'/proc/{root.pid}/stat').read_text())['start']
            sample = b.snapshot(root.pid, start)
            self.assertEqual({p['pid'] for p in sample['processes']}, {root.pid, child})
            root.stdin.close()
            root.wait(timeout=5)
            with self.assertRaises(RuntimeError):
                b.snapshot(root.pid, start)

    def test_cli_preserves_reports_and_records_incomplete_run(self):
        script = pathlib.Path(b.__file__)
        with tempfile.TemporaryDirectory() as directory:
            directory = pathlib.Path(directory)
            metadata = directory / 'metadata.json'
            metadata.write_text(json.dumps({k: 'test' for k in b.REQUIRED_METADATA}))
            root = subprocess.Popen([sys.executable, '-c', 'import sys; sys.stdin.read()'], stdin=subprocess.PIPE)
            try:
                report = directory / 'report.json'
                command = [sys.executable, str(script), '--pid', str(root.pid), '--metadata', str(metadata),
                           '--output', str(report), '--warmup', '.01', '--seconds', '.03', '--interval', '.01']
                subprocess.run(command, check=True, capture_output=True)
                original = report.read_text()
                self.assertEqual(json.loads(original)['status'], 'sampling_completed')
                self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
                self.assertEqual(report.read_text(), original)
                failed = directory / 'incomplete.json'
                command[command.index('--output') + 1] = str(failed)
                command[command.index('--warmup') + 1] = '1'
                with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as collector:
                    deadline = time.monotonic() + 5
                    while not failed.exists() and time.monotonic() < deadline and collector.poll() is None:
                        time.sleep(.01)
                    self.assertTrue(failed.exists())
                    root.stdin.close()
                    root.wait(timeout=5)
                    collector.communicate(timeout=5)
                    self.assertNotEqual(collector.returncode, 0)
                self.assertEqual(json.loads(failed.read_text())['status'], 'incomplete')
            finally:
                if root.poll() is None:
                    root.stdin.close()
                    root.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
