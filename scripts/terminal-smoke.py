import os,pty,fcntl,termios,struct,subprocess,time,select,json
from pathlib import Path
import pyte
Path("outputs").mkdir(exist_ok=True)
master,slave=pty.openpty()
fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,120,0,0))
env=dict(os.environ,TERM='xterm-256color')
env.pop('INCEPTION_API_KEY',None)
def setup_tty():
    os.setsid()
    fcntl.ioctl(slave,termios.TIOCSCTTY,0)
p=subprocess.Popen(['target/release/linger','examples/repair.jsonl','--follow'],stdin=slave,stdout=slave,stderr=slave,env=env,preexec_fn=setup_tty)
os.close(slave)
screen=pyte.Screen(120,40);stream=pyte.Stream(screen)
def pump(seconds=0.4):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        if select.select([master],[],[],0.05)[0]:
            try: data=os.read(master,65536)
            except OSError: break
            if b'\x1b[6n' in data: os.write(master,b'\x1b[1;1R')
            stream.feed(data.decode('utf-8',errors='replace'))
def keys(data):
    os.write(master,data);pump()
def frame(name):
    text='\n'.join(screen.display)
    Path('outputs/'+name+'.txt').write_text(text)
    return text
try:
    pump(1.5)
    frame("initial")
    keys(b'\r');text=frame('calls')
    assert 'Linger / main / 4 calls' in text,text
    keys(b'k');keys(b'\r');text=frame('python-output')
    assert 'ZeroDivisionError' in text,text
    keys(b'1');text=frame('python-input')
    assert "print(sum([]) / len([]))" in text,text
    keys(b'2');keys(b'.');keys(b',');text=frame('time-navigation')
    keys(b'i');pump();text=frame('mercury-unconfigured')
    assert 'Mercury is not configured' in text,text
    keys(b'\x1b');keys(b'\x1b');keys(b'q')
    p.wait(timeout=3)
    assert p.returncode==0
    print(json.dumps({'terminal':'120x40 PTY','calls':4,'checks':['open inspector','select earlier call','read Python output','read multiline input','event stepping','missing-key path','return and quit'],'exit':p.returncode}))
finally:
    if p.poll() is None: p.terminate();p.wait()
    os.close(master)
