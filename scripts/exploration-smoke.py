"""120x40 native command exploration. Fictional recordings, no API calls."""
import importlib.util
import json
from pathlib import Path
import tempfile
spec = importlib.util.spec_from_file_location('pattern_smoke', Path(__file__).with_name('pattern-smoke.py'))
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
Session = module.Session

def assert_connection(terminal):
    rows = terminal.screen.display
    bracket_row = next(y for y, row in enumerate(rows) if '└' in row and ('┤' in row or '┬' in row))
    bracket_start = rows[bracket_row].index('└')
    selected = [x for x in range(36,120) if terminal.screen.buffer[bracket_row-1][x].bg == '32473b']
    assert selected and bracket_start == min(selected), (bracket_start, selected)
    rail = rows[bracket_row+1].index('╭')
    heading = next(y for y in range(bracket_row+2,len(rows)) if 'Part ' in rows[y])
    assert rows[heading][rail:rail+3] == '╰─ '
    assert all(rows[y][rail] == '│' for y in range(bracket_row+2,heading))
    assert rows[heading+1][rail+3:118].strip() == ''
    assert '│' not in rows[heading][rail+3:118]  # no prose line-number gutter
    return bracket_start
with tempfile.TemporaryDirectory(prefix='linger-explorer-pty-') as data_dir:
    with Session('examples/repair.jsonl', data_dir) as terminal:
        terminal.key(b'\r3'); terminal.expect('Part 1/3')
        terminal.key(b'l'); terminal.expect('Part 2/3'); terminal.expect('short-format')
        first_anchor = assert_connection(terminal)
        terminal.key(b'l'); terminal.expect('Part 3/3')
        assert assert_connection(terminal) > first_anchor
        terminal.key(b'h'); terminal.expect('Part 2/3')
        terminal.capture('command-git')
        terminal.key(b'\x1b'); terminal.key(b'k3'); terminal.expect('Part 1/5')
        terminal.key(b'll'); terminal.expect('here-document'); terminal.expect('Quoting the'); terminal.expect('prevents shell parameter')
        terminal.capture('command-heredoc')
        terminal.key(b'l'); terminal.expect('Python body'); terminal.expect('unknown')
        assert_connection(terminal)
        terminal.key(b'\x1b'); terminal.key(b'kk3'); terminal.expect('Part 1/4')
        terminal.key(b'll'); terminal.expect('regular expression')
        terminal.key(b'l'); terminal.expect('file or directory')
        terminal.capture('command-rg-path')
        terminal.key(b'2'); terminal.expect('TODO handle empty input')
        terminal.key(b'3'); terminal.expect('Part 4/4')
        terminal.key(b'\x1b'); terminal.key(b'j3'); terminal.expect('Part 1/4')
        terminal.key(b'll'); terminal.expect('8 through 16, inclusive')
        terminal.capture('command-sed-range')
print(json.dumps({'terminal':'120x40 PTY', 'checks':['Git flag selection','span-to-explanation connector','connector follows selection','spaced prose without line numbers','quoted heredoc','opaque Python body','rg pattern/path distinction','return to output','part selection retained across tabs','numeric sed range'], 'provider_requests':0}))
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
# Regression for the command-pane failures: a real native lookup of composed
# shell input recovered from JavaScript containing an unrelated regex literal.
with tempfile.TemporaryDirectory(prefix='linger-parser-pty-') as data_dir:
    command = "pwd\npython3 - <<'PY'\nprint(1)\nPY\ngit status --short\nprintf '%s' \"$((1+2))\""
    source = 'const unrelated = /a[b]/; await tools.exec_command({cmd:' + json.dumps(command) + '});'
    rows = [json.loads(Path('examples/nested.jsonl').read_text().splitlines()[0]),
            {'timestamp':'2026-09-13T13:00:01Z','type':'response_item',
             'payload':{'type':'custom_tool_call','name':'exec','call_id':'parser-fictional', 'input':source}}]
    fixture = Path(data_dir)/'parser.jsonl'
    fixture.write_text(''.join(json.dumps(row)+'\n' for row in rows))
    with Session(str(fixture), data_dir) as terminal:
        terminal.key(b'\r3'); terminal.expect('literal from JavaScript')
        terminal.expect('Coverage:'); terminal.expect('unsupported_expansion:')
        terminal.expect('Part 1/')
        seen = set()
        # Inspect actual selectable parts, not command text repeated in the header.
        for _ in range(24):
            content = terminal.text()
            for kind in ['Python body · unknown', 'opaque expansion · unknown', 'delimiter · matched']:
                if kind in content:
                    seen.add(kind)
            if 'short-format' in content:
                seen.add('following git option')
            terminal.key(b'l')
        assert seen == {'Python body · unknown','opaque expansion · unknown','delimiter · matched','following git option'}, seen
        terminal.capture('command-partial-coverage')
        terminal.key(b'1'); terminal.expect('const unrelated')
        terminal.key(b'3'); terminal.expect('unsupported_expansion:')
print(json.dumps({'checks':['regex-adjacent literal opens in native command pane','coverage and partial diagnostic visible','composed heredoc parts selectable','following command documented','unsupported expansion selectable','original JavaScript retained'], 'provider_requests':0}))
