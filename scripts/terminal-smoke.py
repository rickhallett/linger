import argparse,tempfile,codecs,os,pty,fcntl,termios,struct,subprocess,time,select,json
from pathlib import Path
import pyte
parser=argparse.ArgumentParser(description="Exercise Linger in a real terminal using fictional evidence.")
parser.add_argument('--live-mercury',action='store_true',help='Send the fictional Python call to the configured provider (one paid request).')
args=parser.parse_args()
Path("outputs").mkdir(exist_ok=True)
master,slave=pty.openpty()
fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,120,0,0))
env=dict(os.environ,TERM='xterm-256color',COLORTERM='truecolor')
# The agent runner sets NO_COLOR; this fixture explicitly checks colour rendering.
env.pop('NO_COLOR',None)
library_dir=tempfile.TemporaryDirectory(prefix='linger-smoke-')
env['LINGER_DATA_DIR']=library_dir.name
if not args.live_mercury:
    env.pop('INCEPTION_API_KEY',None)
    env.pop('OPENROUTER_API_KEY',None)
    empty=Path('outputs/empty.env').resolve()
    empty.write_text('')
    env['LINGER_ENV_FILE']=str(empty)
def setup_tty():
    os.setsid()
    fcntl.ioctl(slave,termios.TIOCSCTTY,0)
p=subprocess.Popen(['target/release/linger','examples/repair.jsonl','--follow'],stdin=slave,stdout=slave,stderr=slave,env=env,preexec_fn=setup_tty)
os.close(slave)
screen=pyte.Screen(120,40);stream=pyte.Stream(screen)
decoder=codecs.getincrementaldecoder('utf-8')(errors='replace')
def pump(seconds=0.4):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        if select.select([master],[],[],0.05)[0]:
            try: data=os.read(master,65536)
            except OSError: break
            if b'\x1b[6n' in data: os.write(master,b'\x1b[1;1R')
            stream.feed(decoder.decode(data))
def keys(data):
    os.write(master,data);pump()
def frame(name):
    text='\n'.join(screen.display)
    Path('outputs/'+name+'.txt').write_text(text)
    Path('outputs/'+name+'-cells.json').write_text(json.dumps([[screen.buffer[y][x]._asdict() for x in range(120)] for y in range(40)]))
    return text
try:
    pump(1.5)
    frame("initial")
    keys(b'\r');text=frame('calls')
    assert 'linger / main / 4 calls' in text,text
    keys(b'k');keys(b'\r');text=frame('python-output')
    assert 'ZeroDivisionError' in text,text
    keys(b'1');text=frame('python-input')
    assert "print(sum([]) / len([]))" in text,text
    keys(b'2');keys(b'.');keys(b',');text=frame('time-navigation')
    started=time.monotonic()
    keys(b'i')
    if args.live_mercury:
        # The UI must remain usable while the request is running.
        keys(b'1')
        assert "print(sum([]) / len([]))" in frame('pending-input')
        keys(b'4')
        while time.monotonic()-started < 35:
            pump(0.1)
            text='\n'.join(screen.display)
            if 'Model interpretation' in text or 'HTTP ' in text or 'timed out' in text:
                break
        elapsed=round(time.monotonic()-started,2)
        text=frame('live-mercury')
        assert 'Model interpretation' in text,text
        assert 'mercury-2.5' in text,text
        assert any(word in text.lower() for word in ['zero','division','empty']),text
        Path('outputs/live-mercury-receipt.json').write_text(json.dumps({'fixture':'examples/repair.jsonl','elapsed_seconds':elapsed,'model':'mercury-2.5','terminal':'120x40 PTY','navigation_during_request':True}))
        print(json.dumps({'live_mercury':'passed','elapsed_seconds':elapsed,'fixture':'fictional Python division by zero'}))
    else:
        pump();text=frame('mercury-unconfigured')
        assert 'Mercury is not configured' in text,text
    keys(b'\x1b');keys(b'\x1b');keys(b'q')
    p.wait(timeout=3)
    assert p.returncode==0
    print(json.dumps({'terminal':'120x40 PTY','calls':4,'checks':['open inspector','select earlier call','read Python output','read multiline input','event stepping','live Mercury' if args.live_mercury else 'missing-key path','return and quit'],'exit':p.returncode}))
finally:
    if p.poll() is None: p.terminate();p.wait()
    os.close(master)
    library_dir.cleanup()
