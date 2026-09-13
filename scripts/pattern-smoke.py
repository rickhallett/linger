"""Three real terminal runs; two fictional sessions; no provider requests."""
import codecs,fcntl,json,os,pty,select,sqlite3,struct,subprocess,tempfile,termios,time
from pathlib import Path
import pyte

class Session:
    def __init__(self,fixture,data_dir):
        self.master,slave=pty.openpty()
        fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,120,0,0))
        env=dict(os.environ,TERM='xterm-256color',COLORTERM='truecolor',LINGER_DATA_DIR=data_dir)
        for name in ('NO_COLOR','INCEPTION_API_KEY','OPENROUTER_API_KEY'):env.pop(name,None)
        empty=Path(data_dir)/'empty.env';empty.write_text('');env['LINGER_ENV_FILE']=str(empty)
        def setup():
            os.setsid();fcntl.ioctl(slave,termios.TIOCSCTTY,0)
        self.process=subprocess.Popen(['target/release/linger',fixture,'--follow'],stdin=slave,stdout=slave,stderr=slave,env=env,preexec_fn=setup)
        os.close(slave)
        self.screen=pyte.Screen(120,40);self.stream=pyte.Stream(self.screen)
        self.decoder=codecs.getincrementaldecoder('utf-8')(errors='replace')
    def pump(self,seconds=.3):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            if select.select([self.master],[],[],.04)[0]:
                try:data=os.read(self.master,65536)
                except OSError:break
                if b'\x1b[6n' in data:os.write(self.master,b'\x1b[1;1R')
                self.stream.feed(self.decoder.decode(data))
    def key(self,key):os.write(self.master,key);self.pump()
    def text(self):return '\n'.join(self.screen.display)
    def expect(self,needle):
        end=time.monotonic()+4
        while needle not in self.text() and time.monotonic()<end:self.pump(.1)
        assert needle in self.text(),(needle,self.text())
    def capture(self,name):
        Path('outputs').mkdir(exist_ok=True)
        Path('outputs/'+name+'.txt').write_text(self.text())
        Path('outputs/'+name+'-cells.json').write_text(json.dumps([[self.screen.buffer[y][x]._asdict() for x in range(120)] for y in range(40)]))
    def __enter__(self):self.pump(1);return self
    def __exit__(self,*exc):
        if self.process.poll() is None:self.key(b'\x1b\x1bq')
        try:self.process.wait(timeout=5)
        finally:
            if self.process.poll() is None:self.process.kill();self.process.wait()
            os.close(self.master)
        assert self.process.returncode==0

if __name__ == '__main__':
    with tempfile.TemporaryDirectory(prefix='linger-pattern-pty-') as data_dir:
        with Session('examples/repair.jsonl',data_dir) as terminal:
            terminal.key(b'b/rg -n\r');terminal.expect('rg -n <pattern> <path>')
            terminal.key(b'[');terminal.expect('11:00:00');terminal.expect('rg -n <pattern> <path>')
            terminal.key(b'g');terminal.expect('11:00:28');terminal.expect('rg -n <pattern> <path>')
            terminal.key(b'?');terminal.expect('g/End latest');terminal.expect('scope')
            terminal.capture('patterns-help')
            terminal.key(b'\x1b');terminal.expect('Recorded input')
            terminal.key(b'p');terminal.expect('Learning state saved');terminal.expect('Practising')
            terminal.key(b'\r');terminal.expect('TODO handle empty input');terminal.expect('◎')
            terminal.capture('practising-inspector')
        with Session('examples/patterns.jsonl',data_dir) as terminal:
            terminal.key(b'b/rg -n\r');terminal.expect('Practising')
            terminal.expect('3 here');terminal.expect('4 all');terminal.expect('2 sessions')
            terminal.key(b'\t');terminal.expect('All cached sessions');terminal.capture('patterns-library')
            terminal.key(b'l');terminal.expect('Occurrence 2/3');terminal.expect('rg -n empty tests')
            terminal.key(b'L');terminal.expect('No patterns in this view.');terminal.expect('Learning state saved')
        with Session('examples/repair.jsonl',data_dir) as terminal:
            terminal.key(b'b/rg -n\r');terminal.expect('No patterns in this view.')
            terminal.key(b'H');terminal.expect('Learned');terminal.expect('4 all');terminal.expect('1 here')
            terminal.key(b'p');terminal.expect('Practising');terminal.expect('Learning state saved')
            terminal.key(b'\r');terminal.expect('TODO handle empty input');terminal.expect('◎')
            terminal.key(b'b');terminal.expect('4 all');terminal.capture('patterns-reopened')
        conn=sqlite3.connect(str(Path(data_dir)/'patterns.sqlite3'))
        assert conn.execute('SELECT COUNT(*) FROM occurrences').fetchone()[0]==9
        assert conn.execute('SELECT COUNT(DISTINCT session) FROM occurrences').fetchone()[0]==2
        conn.close()
    print(json.dumps({'terminal':'120x40 PTY','runs':3,'distinct_sessions':2,'unique_calls':9,'checks':['cross-session frequency','reopen deduplication','persistent Practising','persistent Learned','hide Learned','occurrence preview','jump to recorded output','Practising inspector marker'],'provider_requests':0}))
