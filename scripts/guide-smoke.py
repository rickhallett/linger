"""A growing field guide across three real terminal runs, fictional inputs only."""
import importlib.util,json,sqlite3,tempfile
from pathlib import Path
spec=importlib.util.spec_from_file_location('pattern_smoke',Path(__file__).with_name('pattern-smoke.py'))
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
Session=module.Session
def highlighted(terminal, line, word):
    y=next(y for y,text in enumerate(terminal.screen.display) if line in text)
    x=terminal.screen.display[y].index(word,terminal.screen.display[y].index(line))
    assert all(terminal.screen.buffer[y][i].bg=='32473b' for i in range(x,x+len(word)))

with tempfile.TemporaryDirectory(prefix='linger-guide-pty-') as data_dir:
    with Session('examples/repair.jsonl',data_dir) as terminal:
        terminal.key(b'G');terminal.expect('4 entries');terminal.expect('field guide')
        terminal.capture('field-guide-shelf')
        terminal.key(b'/rg\r\r');terminal.expect('field guide / rg');terminal.expect('Collected forms');terminal.expect('TODO src')
        highlighted(terminal,'rg -n TODO src','rg')
        terminal.key(b'n');terminal.key(b'-n adds line numbers. Read the pattern and path separately.\r');terminal.expect('Field note saved')
        terminal.capture('field-guide-entry')
    with Session('examples/structure.jsonl',data_dir) as terminal:
        terminal.key(b'G');terminal.expect('5 entries');terminal.expect('field note')
        terminal.capture('field-guide-grown')
        terminal.key(b'/rg\r\r');terminal.expect('Read the pattern and path separately.');terminal.expect('Sighting 1/2')
        terminal.key(b'llll');terminal.expect('Sighting 2/2');terminal.expect('rg -n average lib')
        terminal.key(b'h');terminal.expect('Sighting 1/2');terminal.key(b'l')
        terminal.key(b'p');terminal.expect('Learning state saved')
        terminal.key(b'\r');terminal.expect('Fictional result 4');terminal.expect('◎')
        terminal.key(b'G');terminal.expect('field guide / rg')
        terminal.capture('field-guide-return')
    with Session('examples/patterns.jsonl',data_dir) as terminal:
        terminal.key(b'G');terminal.expect('5 entries')
        terminal.key(b'/zsh\r');terminal.key(b'hhhh');terminal.key(b'\r');terminal.expect('field guide / /bin/zsh')
        terminal.expect('Cached specimen');highlighted(terminal,'{"command":','/bin/zsh');terminal.key(b'\r');terminal.expect('open its original recording')
        terminal.capture('field-guide-cached')
    db=sqlite3.connect(str(Path(data_dir)/'learning.sqlite3'))
    notes=db.execute('SELECT text FROM notes').fetchall();db.close()
    assert notes==[('-n adds line numbers. Read the pattern and path separately.',)]
print(json.dumps({'terminal':'120x40 PTY','runs':3,'checks':['gallery growth','related forms','highlighted specimens','saved note after restart','cycle occurrences','inspect recorded output','return to entry','Practising propagation','cached-only boundary'],'provider_requests':0}))
