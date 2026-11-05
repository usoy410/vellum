import subprocess,time,os,json,pathlib
x=subprocess.Popen(['Xvfb',':98','-screen','0','1920x1080x24','-nolisten','tcp'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
time.sleep(1)
os.environ['DISPLAY']=':98'
results=[]
try:
 for name,args in [('native-static',[]),('native-30hz',['--animate']),('web-static',['file:///workspace/vellum/experiments/scene.html'])]:
  rows=[]
  for repeat in range(3):
   with open('/tmp/'+name+'.log','w') as log:
    p=subprocess.Popen(['/workspace/.vellum-env/'+('web-poc' if name.startswith('web') else 'native-poc')]+args,stdout=log,stderr=log)
    time.sleep(1)
    if p.poll() is not None:
     rows.append({'exit':p.returncode,'error':pathlib.Path(log.name).read_text()});continue
    def cpu():
     a=pathlib.Path(f'/proc/{p.pid}/stat').read_text().rsplit(')',1)[1].split();return (int(a[11])+int(a[12]))/os.sysconf('SC_CLK_TCK')
    start=cpu();t=time.monotonic();time.sleep(3)
    mem={}
    for line in pathlib.Path(f'/proc/{p.pid}/smaps_rollup').read_text().splitlines():
     if line.startswith(('Rss:','Pss:')): k,v,*_=line.split();mem[k[:-1]]=int(v)
    rows.append({'cpu_one_core_percent':100*(cpu()-start)/(time.monotonic()-t),'memory_kib':mem,'scope':'parent only'})
    p.terminate();p.wait(timeout=5)
  results.append({'candidate':name,'runs':rows})
finally:x.terminate();x.wait()
print(json.dumps(results,indent=2))
