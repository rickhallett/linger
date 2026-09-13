"""Collection requires drill-in; three real terminal runs, fictional inputs only."""
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
        terminal.key(b'G');terminal.expect('0 entries');terminal.expect('Drill into a tool call')
        terminal.capture('field-guide-empty')
        terminal.key(b'G\rkkkG');terminal.expect('0 entries')  # browsing calls is not collection
        terminal.key(b'G\rG');terminal.expect('1 entries')
        terminal.capture('field-guide-shelf')
        terminal.key(b'\r');terminal.expect('field guide / rg');terminal.expect('Specimen 1/1')
        highlighted(terminal,'rg -n TODO src','rg')
        terminal.key(b'n');terminal.key(b'-n adds line numbers. Read the pattern and path separately.\r');terminal.expect('Field note saved')
        terminal.capture('field-guide-entry')
        terminal.key(b'\rG');terminal.expect('Specimen 1/1')  # reopening never increments
    with Session('examples/structure.jsonl',data_dir) as terminal:
        terminal.key(b'G');terminal.expect('1 entries')  # new session does not auto-collect
        terminal.key(b'G');terminal.key(b'b/rg -n\r\r');terminal.expect('Fictional result 0')
        terminal.key(b'b/rg -n\rl\r');terminal.expect('Fictional result 4')
        terminal.key(b'G');terminal.expect('2 entries')  # only rg and its shell wrapper
        terminal.capture('field-guide-grown')
        terminal.key(b'/rg\r\r');terminal.expect('Read the pattern and path separately.');terminal.expect('Specimen 1/3')
        terminal.key(b'l');terminal.expect('Specimen 2/3');terminal.expect('rg -n average lib')
        terminal.key(b'p');terminal.expect('Learning state saved')
        terminal.key(b'\r');terminal.expect('Fictional result 4');terminal.expect('◎')
        terminal.key(b'G');terminal.expect('Specimen 2/3');terminal.capture('field-guide-return')
        terminal.key(b'llll');terminal.expect('Specimen 3/3');terminal.expect('Collected in another recording')
        terminal.key(b'h');terminal.expect('Specimen 2/3')
    with Session('examples/patterns.jsonl',data_dir) as terminal:
        terminal.key(b'G');terminal.expect('2 entries')
        terminal.key(b'/zsh\r\r');terminal.expect('field guide / /bin/zsh')
        terminal.expect('Collected in another recording');highlighted(terminal,'{"command":','/bin/zsh')
        terminal.key(b'\r');terminal.expect('open its original recording');terminal.capture('field-guide-cached')
    db=sqlite3.connect(str(Path(data_dir)/'learning.sqlite3'))
    assert db.execute('SELECT COUNT(*) FROM specimens').fetchone()[0]==3
    assert db.execute('SELECT text FROM notes').fetchall()==[('-n adds line numbers. Read the pattern and path separately.',)]
    db.close()
    db=sqlite3.connect(str(Path(data_dir)/'patterns.sqlite3'))
    assert db.execute('SELECT COUNT(*) FROM occurrences').fetchone()[0]==14
    db.close()
print(json.dumps({'terminal':'120x40 PTY','runs':3,'collected_specimens':3,'recorded_occurrences':14,'checks':['no automatic collection','drill-in and frequency jump collect','reopen deduplication','only collected forms and examples','notes survive restart','cross-session specimen cycling','correct output jump','Practising','original recording boundary'],'provider_requests':0}))
