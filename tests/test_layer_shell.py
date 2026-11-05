"""Opt-in integration tests: create a private headless Sway, never use the user's compositor."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time
import unittest

REQUIRED = ('VELLUM_LAYER_BINARY', 'VELLUM_REFERENCE_BINARY', 'VELLUM_POINTER_SEAT')


@unittest.skipUnless(all(os.environ.get(k) for k in REQUIRED), 'set the three VELLUM_* test binaries')
class LayerShell(unittest.TestCase):
    def setUp(self):
        from PIL import Image
        self.Image = Image
        self.directory = tempfile.TemporaryDirectory(prefix='vellum-wl-')
        self.root = Path(self.directory.name)
        self.runtime = self.root / 'runtime'
        self.runtime.mkdir(mode=0o700)
        self.processes, self.files = [], []
        self.addCleanup(self.cleanup)
        config = self.root / 'sway.conf'
        config.write_text('xwayland disable\noutput HEADLESS-1 resolution 1920x1080 position 0 0\n'
                          'output HEADLESS-2 resolution 1280x720 position 1920 0\nseat seat0 fallback true\n')
        self.env = dict(os.environ, XDG_RUNTIME_DIR=str(self.runtime), WLR_BACKENDS='headless',
                        WLR_RENDERER='pixman', WLR_HEADLESS_OUTPUTS='2', GDK_BACKEND='wayland')
        for key in ('SWAYSOCK', 'WAYLAND_DISPLAY', 'DISPLAY', 'WAYLAND_DEBUG'):
            self.env.pop(key, None)
        self.compositor = self.launch(['sway', '-c', str(config)], 'sway')
        self.wait(lambda: list(self.runtime.glob('sway-ipc.*.sock')), 'private compositor socket')
        self.env['SWAYSOCK'] = str(next(self.runtime.glob('sway-ipc.*.sock')))
        self.wait(lambda: [p for p in self.runtime.glob('wayland-*') if not p.name.endswith('.lock')], 'Wayland socket')
        self.env['WAYLAND_DISPLAY'] = next(p.name for p in self.runtime.glob('wayland-*') if not p.name.endswith('.lock'))
        self.wait(lambda: len(self.ipc('-t', 'get_outputs')) == 2, 'two outputs')
        self.pointer = self.launch([os.environ['VELLUM_POINTER_SEAT']], 'pointer', stdin=subprocess.PIPE)
        self.wait(lambda: 'ready' in self.log('pointer'), 'virtual pointer')

    def cleanup(self):
        for process in reversed(self.processes):
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait()
            if process.stdin:
                process.stdin.close()
        for file in self.files:
            file.close()
        # Retain evidence only when explicitly requested; never overwrite prior runs.
        if os.environ.get('VELLUM_TEST_ARTIFACTS'):
            destination = Path(os.environ['VELLUM_TEST_ARTIFACTS']) / self._testMethodName
            shutil.copytree(self.root, destination, ignore=shutil.ignore_patterns('runtime'))
        self.directory.cleanup()

    def launch(self, command, name, **kwargs):
        output = (self.root / f'{name}.log').open('w')
        self.files.append(output)
        process = subprocess.Popen(command, env=self.env, stdout=output, stderr=output, **kwargs)
        self.processes.append(process)
        return process

    def log(self, name):
        return (self.root / f'{name}.log').read_text()

    def wait(self, predicate, label):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if predicate():
                return
            time.sleep(.03)
        self.fail('timed out: ' + label + '\n' + '\n'.join(p.read_text()[-2000:] for p in self.root.glob('*.log')))

    def ipc(self, *args):
        result = subprocess.run(['swaymsg', '-r', *args], env=self.env, capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(result.stdout)
        if args[0] != '-t':
            self.assertTrue(all(item['success'] for item in data), data)
        return data

    def start_wallpaper(self, animated=False):
        self.env['WAYLAND_DEBUG'] = 'client'
        process = self.launch([os.environ['VELLUM_LAYER_BINARY']] + (['--animate'] if animated else []), 'wallpaper')
        self.env.pop('WAYLAND_DEBUG')
        self.wait(lambda: 'logical=1920x1080' in self.log('wallpaper') and 'logical=1280x720' in self.log('wallpaper'), 'configured surfaces')
        return process

    def click(self, x, y):
        self.ipc(f'seat seat0 cursor set {x} {y}')
        self.ipc('seat seat0 cursor press button1')
        self.ipc('seat seat0 cursor release button1')

    def screenshot(self, output, filename):
        destination = self.root / filename
        subprocess.run(['grim', '-o', output, str(destination)], env=self.env, check=True, capture_output=True, timeout=5)
        with self.Image.open(destination) as image:
            return image.convert('RGB')

    def test_static_protocol_input_outputs_and_focus(self):
        process = self.start_wallpaper()
        protocol = self.log('wallpaper')
        self.assertEqual(len(re.findall(r'get_layer_surface\(.*?, 0, "vellum-native-experiment"', protocol)), 2)
        for request in ('set_keyboard_interactivity(0)', 'set_exclusive_zone(-1)', 'set_anchor(15)'):
            self.assertEqual(protocol.count(request), 2)
        self.wait(lambda: self.screenshot('HEADLESS-1', 'initial.png').getpixel((320, 320)) == (44, 150, 195), 'initial frame presented')
        first = self.screenshot('HEADLESS-1', 'initial.png')
        self.assertEqual(first.size, (1920, 1080))
        self.assertEqual(first.getpixel((100, 100)), (20, 36, 56))
        self.assertEqual(first.getpixel((320, 320)), (44, 150, 195))
        self.click(300, 300)
        self.wait(lambda: 'click output=' in self.log('wallpaper'), 'background click')
        self.wait(lambda: self.screenshot('HEADLESS-1', 'clicked.png').getpixel((255, 255)) == (255, 179, 51), 'click frame presented')
        self.ipc('focus output HEADLESS-1')
        leaves = len(re.findall(r'hover output=\d+ inside=0', self.log('wallpaper')))
        reference = self.launch([os.environ['VELLUM_REFERENCE_BINARY']], 'reference')
        def view(node):
            if node.get('pid') == reference.pid:
                return node
            for child in node.get('nodes', []) + node.get('floating_nodes', []):
                found = view(child)
                if found: return found
            return None
        self.wait(lambda: view(self.ipc('-t', 'get_tree')), 'covering application')
        self.assertEqual(view(self.ipc('-t', 'get_tree'))['rect']['width'], 1920)
        self.wait(lambda: len(re.findall(r'hover output=\d+ inside=0', self.log('wallpaper'))) > leaves, 'pointer handoff to covering application')
        self.wait(lambda: self.screenshot('HEADLESS-1', 'covered.png').getpixel((1, 1)) != (20, 36, 56), 'covering application presented')
        before = self.log('wallpaper').count('click output=')
        self.click(300, 300)
        self.wait(lambda: 'click delivered' in self.log('reference'), 'click delivered to covering application')
        self.assertEqual(self.log('wallpaper').count('click output='), before)
        for command in ('floating enable', 'resize set 400 300', 'move position 1000 600'):
            self.ipc(f'[pid={reference.pid}] {command}')
        self.wait(lambda: self.screenshot('HEADLESS-1', 'exposed.png').getpixel((1, 1)) == (20, 36, 56), 'floating application move presented')
        focused = self.ipc('-t', 'get_seats')[0]['focus']
        self.assertEqual(focused, view(self.ipc('-t', 'get_tree'))['id'])
        self.click(300, 300)
        self.wait(lambda: self.log('wallpaper').count('click output=') > before, 'exposed background click')
        self.assertEqual(self.ipc('-t', 'get_seats')[0]['focus'], focused)
        reference.terminate(); reference.wait(timeout=5)
        self.ipc('output HEADLESS-1 scale 2')
        self.wait(lambda: re.search(r'logical=960x540 scale=2', self.log('wallpaper')), 'HiDPI configure')
        before = self.log('wallpaper').count('click output=')
        self.click(150, 150)
        self.wait(lambda: self.log('wallpaper').count('click output=') > before, 'scaled hit testing')
        self.assertEqual(self.screenshot('HEADLESS-1', 'scaled.png').size, (1920, 1080))
        self.ipc('output HEADLESS-2 disable')
        self.wait(lambda: 'removed output=' in self.log('wallpaper'), 'output removal')
        self.ipc('output HEADLESS-2 enable')
        self.wait(lambda: self.log('wallpaper').count('added output=') == 3, 'output re-addition')
        self.wait(lambda: self.log('wallpaper').count('logical=1280x720') >= 2, 're-added output configured')
        self.wait(lambda: self.screenshot('HEADLESS-2', 'readded.png').getpixel((210, 210)) == (44, 150, 195), 're-added output rendered')
        second = self.screenshot('HEADLESS-2', 'readded.png')
        self.assertEqual(second.size, (1280, 720))
        self.assertEqual(second.getpixel((210, 210)), (44, 150, 195))
        self.ipc('seat seat0 cursor set 10 10')
        time.sleep(.3)
        frames = self.log('wallpaper').count('.damage')
        time.sleep(.3)
        self.assertEqual(self.log('wallpaper').count('.damage'), frames, 'static scene periodically repainted')
        maps = Path(f'/proc/{process.pid}/maps').read_text().lower()
        self.assertNotIn('webkit', maps)
        process.terminate(); self.assertEqual(process.wait(timeout=5), 0)
        self.assertEqual(self.log('wallpaper').count('removed output='), 3)

    def test_missing_wayland_display_does_not_create_fallback_window(self):
        environment = dict(self.env, WAYLAND_DISPLAY='missing-test-display', DISPLAY=':99999')
        result = subprocess.run([os.environ['VELLUM_LAYER_BINARY']], env=environment,
                                capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 2)
        self.assertIn('Requires a Wayland compositor', result.stderr)
        self.assertNotIn('added output=', result.stdout)

    def test_animation_survives_output_removal_and_stops_cleanly(self):
        process = self.start_wallpaper(animated=True)
        frames = self.log('wallpaper').count('.damage')
        self.wait(lambda: self.log('wallpaper').count('.damage') > frames, 'animation frames')
        self.ipc('output HEADLESS-2 disable')
        self.wait(lambda: 'removed output=' in self.log('wallpaper'), 'animated output removal')
        time.sleep(.1)
        self.assertIsNone(process.poll())
        process.terminate(); self.assertEqual(process.wait(timeout=5), 0)
        self.assertNotIn('CRITICAL', self.log('wallpaper'))


if __name__ == '__main__':
    unittest.main()
