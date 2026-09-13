"""Code-mode extraction and shortcut feedback in a real 120x40 PTY; synthetic input."""
import importlib.util, tempfile, json
from pathlib import Path
spec=importlib.util.spec_from_file_location('patterns', 'scripts/pattern-smoke.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)

def lit(terminal, letter):
    return [(x,y) for y in range(40) for x in range(120)
            if terminal.screen.buffer[y][x].data == letter
            and terminal.screen.buffer[y][x].bg == 'e5c736']

with tempfile.TemporaryDirectory(prefix='linger-nested-pty-') as data:
    with module.Session('examples/nested.jsonl', data) as t:
        t.key(b'b');t.expect('Parts')
        assert 'Counts are calls containing' not in t.text()
        t.key(b'f');t.expect('Combinations');assert lit(t, 'f'),t.text()
        t.capture('shortcut-feedback')
        t.pump(.7);assert not lit(t, 'f')
        t.key(b'c');t.expect('Parts')
        t.key(b'/rg -n\r');t.expect('1 here');t.expect('Recorded input')
        # Only the two actual cmd values, never the misleading label literal.
        highlighted=''.join(t.screen.buffer[y][x].data for y in range(40) for x in range(55,120)
                            if t.screen.buffer[y][x].bg == '32473b')
        assert highlighted.count('rg') == 2, (highlighted,t.text())
        t.capture('nested-pattern-source')
        t.key(b'\r');t.expect('Reading');t.expect('Fictional nested output 0');assert lit(t,'E')
        t.key(b'3');t.expect('shell 1/2');t.expect('Part 1/4')
        assert lit(t,'3');t.capture('nested-shell-one')
        t.key(b'l');t.expect('Part 2/4');t.expect('line number')
        t.key(b'}');t.expect('shell 2/2');t.expect('git diff --stat');t.expect('Part 1/')
        t.capture('nested-shell-two')
        t.key(b'{');t.expect('shell 1/2');t.expect('Part 1/4')
        t.key(b'1');t.expect('const label');t.expect('tools.exec_command')
        t.key(b'\x1b');t.expect('Preview')
        t.key(b'g');t.key(b'j');t.expect("const command")
        t.key(b'3');t.expect('No deterministic guide')
        assert 'literal from JavaScript' not in t.text()
        t.key(b'b');t.key(b'/git diff\r');t.expect('git diff')
        t.key(b'\r3');t.expect('shell 2/2');t.expect('git diff --stat')
    # Restart reconstructs the nested projection from retained code inputs.
    with module.Session('examples/nested.jsonl', data) as t:
        t.key(b'b/rg -n\r');t.expect('1 here');t.expect('1 cached')
    # A long call list should place the selected instance near the middle.
    rows = [json.loads(Path('examples/nested.jsonl').read_text().splitlines()[0])]
    for n in range(20):
        rows.append({'timestamp': f'2026-09-13T13:00:{n:02}Z', 'type':'response_item',
                     'payload':{'type':'function_call','name':'exec_command','call_id':f'long-{n}',
                                'arguments':json.dumps({'cmd':f'rg -n item{n} src'})}})
    fixture=Path(data)/'long.jsonl';fixture.write_text(''.join(json.dumps(row)+'\n' for row in rows))
    with module.Session(str(fixture), data) as t:
        t.key(b'\r');t.expect('Preview · call 20/20')
        y=next(y for y,row in enumerate(t.screen.display) if '▌' in row[:35])
        assert 9 <= y <= 20,(y,t.text())
        t.key(b'k');t.expect('Preview · call 19/20')
        t.key(b'\r');t.expect('Reading · call 19/20')
        t.capture('selected-call-focus')
print(json.dumps({'checks':['literal commands from real custom_tool_call input','nested frequency deduplication','source highlights','switch embedded command','matching nested command on pattern drill-in','dynamic command stays opaque','cache restart','key pulse and expiry','Reading/Preview focus'], 'provider_requests':0}))
