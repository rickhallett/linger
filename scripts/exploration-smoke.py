"""120x40 native command exploration. Fictional recordings, no API calls."""
import importlib.util
import json
from pathlib import Path
import tempfile
spec = importlib.util.spec_from_file_location('pattern_smoke', Path(__file__).with_name('pattern-smoke.py'))
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
Session = module.Session
with tempfile.TemporaryDirectory(prefix='linger-explorer-pty-') as data_dir:
    with Session('examples/repair.jsonl', data_dir) as terminal:
        terminal.key(b'\r3'); terminal.expect('Part 1/3')
        terminal.key(b'l'); terminal.expect('Part 2/3'); terminal.expect('short-format')
        terminal.capture('command-git')
        terminal.key(b'\x1b'); terminal.key(b'k3'); terminal.expect('Part 1/5')
        terminal.key(b'll'); terminal.expect('here-document'); terminal.expect('Quoting the'); terminal.expect('delimiter prevents')
        terminal.capture('command-heredoc')
        terminal.key(b'l'); terminal.expect('Python body'); terminal.expect('unknown')
        terminal.key(b'\x1b'); terminal.key(b'kk3'); terminal.expect('Part 1/4')
        terminal.key(b'll'); terminal.expect('regular expression')
        terminal.key(b'l'); terminal.expect('file or directory')
        terminal.capture('command-rg-path')
        terminal.key(b'2'); terminal.expect('TODO handle empty input')
        terminal.key(b'3'); terminal.expect('Part 4/4')
        terminal.key(b'\x1b'); terminal.key(b'j3'); terminal.expect('Part 1/4')
        terminal.key(b'll'); terminal.expect('8 through 16, inclusive')
        terminal.capture('command-sed-range')
print(json.dumps({'terminal':'120x40 PTY', 'checks':['Git flag selection','quoted heredoc','opaque Python body','rg pattern/path distinction','return to output','part selection retained across tabs','numeric sed range'], 'provider_requests':0}))
def highlighted(terminal):
    # Ignore the selected frequency row: only inspect the recorded-example pane.
    return ''.join(terminal.screen.buffer[y][x].data for y in range(3, 32) for x in range(56, 119) if terminal.screen.buffer[y][x].bg == '32473b')

with tempfile.TemporaryDirectory(prefix='linger-structure-pty-') as data_dir:
    with Session('examples/structure.jsonl', data_dir) as terminal:
        terminal.key(b'bf'); terminal.expect('Combinations'); terminal.expect('scripts hidden')
        assert 'unique-script' not in terminal.text()
        terminal.key(b'/rg -n\r'); terminal.expect('2 here')
        terminal.key(b'/\r'); terminal.key(b'f'); terminal.expect('Programs'); terminal.expect('/bin/zsh'); terminal.expect('3 here')
        assert highlighted(terminal) == '"/bin/zsh"', highlighted(terminal)
        terminal.capture('frequency-programs')
        terminal.key(b'f'); terminal.expect('Wrappers'); terminal.expect('/bin/zsh -lc'); terminal.expect('3 here')
        assert highlighted(terminal) == '"/bin/zsh""-lc"', highlighted(terminal)
        terminal.capture('frequency-wrappers')
        terminal.key(b'p'); terminal.expect('Learning state saved')
        terminal.key(b'\r'); terminal.expect('◎'); terminal.key(b'3'); terminal.expect('shell string from argv'); terminal.expect('Part 1/4')
        terminal.capture('command-from-argv')
        terminal.key(b'bf'); terminal.expect('Combinations')
        terminal.key(b'/unique-script\r'); terminal.expect('unique-script'); terminal.expect('1 here')
        terminal.capture('frequency-script-search')
    with Session('examples/structure.jsonl', data_dir) as terminal:
        terminal.key(b'bfff'); terminal.expect('Wrappers'); terminal.expect('Practising'); terminal.expect('3 all')
        terminal.key(b'\t'); terminal.expect('All cached sessions'); terminal.expect('3 all')
    with Session('examples/repair.jsonl', data_dir) as terminal:
        terminal.key(b'bfff\t'); terminal.expect('Wrappers'); terminal.expect('0 here'); terminal.expect('3 all')
        terminal.expect('Cached example')
        assert highlighted(terminal) == '"/bin/zsh""-lc"', highlighted(terminal)
        terminal.capture('frequency-cached-highlight')
print(json.dumps({'checks':['hidden inline scripts','search includes original scripts','program counts','wrapper counts','inner command counts','wrapper Practising survives restart','reopen deduplication','wrapper occurrence jump','explore shell string in argv'], 'provider_requests':0}))
# A fresh install without the optional backend keeps evidence navigation usable.
import os
previous = os.environ.get('LINGER_EXPLAINSHELL_DIR')
with tempfile.TemporaryDirectory(prefix='linger-no-pack-') as data_dir:
    try:
        os.environ['LINGER_EXPLAINSHELL_DIR'] = str(Path(data_dir)/'absent-backend')
        with Session('examples/repair.jsonl', data_dir) as terminal:
            terminal.key(b'\r3'); terminal.expect('optional local explainshell pack')
            terminal.expect('uv run scripts/setup-explainshell.py')
            terminal.key(b'2'); terminal.expect('## main')
    finally:
        if previous is None: os.environ.pop('LINGER_EXPLAINSHELL_DIR', None)
        else: os.environ['LINGER_EXPLAINSHELL_DIR'] = previous
print(json.dumps({'missing_backend':'setup instruction and recorded output remain usable'}))
