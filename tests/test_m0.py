import os,pathlib,subprocess,tempfile,unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
class Safety(unittest.TestCase):
 def test_failed_start_never_kills_competitors(self):
  with tempfile.TemporaryDirectory() as d:
   p=pathlib.Path(d); marker=p/'called'; fake=p/'pkill'
   fake.write_text('#!/bin/sh\ntouch "'+str(marker)+'"\n');fake.chmod(0o755)
   env=dict(os.environ,PATH=d+':'+os.environ['PATH'],DISPLAY='',WAYLAND_DISPLAY='',GDK_BACKEND='x11')
   binary=pathlib.Path(os.environ['CARGO_TARGET_DIR'])/'debug/vellum'
   r=subprocess.run([str(binary)],env=env,capture_output=True,timeout=10)
   self.assertNotEqual(r.returncode,0)
   self.assertIn(b'Failed to initialize GTK',r.stderr)
   self.assertFalse(marker.exists())
 def test_help_and_version_succeed_without_display(self):
  env=dict(os.environ,DISPLAY='',WAYLAND_DISPLAY='',GDK_BACKEND='x11')
  binary=pathlib.Path(os.environ.get('CARGO_TARGET_DIR',ROOT/'target'))/'debug/vellum'
  for flag in ['-h','--help','help']:
   r=subprocess.run([str(binary),flag],env=env,capture_output=True,timeout=10,text=True)
   self.assertEqual(r.returncode,0)
   self.assertIn('Vellum',r.stdout)
   self.assertIn('USAGE:',r.stdout)
  for flag in ['-v','-V','--version']:
   r=subprocess.run([str(binary),flag],env=env,capture_output=True,timeout=10,text=True)
   self.assertEqual(r.returncode,0)
   self.assertIn('vellum',r.stdout)
 def test_staged_install_has_one_startup_path(self):
  with tempfile.TemporaryDirectory() as d:
   subprocess.run(['make','install','DESTDIR='+d],cwd=ROOT,check=True,capture_output=True)
   p=pathlib.Path(d)
   self.assertTrue((p/'usr/bin/vellum').is_file())
   self.assertTrue((p/'usr/lib/systemd/user/vellum.service').is_file())
   self.assertFalse((p/'etc/xdg/autostart').exists())
if __name__=='__main__':unittest.main()
