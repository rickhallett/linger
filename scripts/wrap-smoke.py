"""Default wrapping and opt-out in a real terminal, using fictional evidence."""
import importlib.util
import json
import tempfile
from pathlib import Path

spec = importlib.util.spec_from_file_location('pattern_smoke', Path(__file__).with_name('pattern-smoke.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
with tempfile.TemporaryDirectory(prefix='linger-wrap-pty-') as data_dir:
    fixture = Path(data_dir) / 'wrap.jsonl'
    records = [json.loads(line) for line in Path('examples/repair.jsonl').read_text().splitlines()][:3]
    records[1]['payload']['arguments'] = json.dumps({'cmd': 'rg -n TODO ' + 'fictional_path/' * 12 + 'INPUT_TAIL'})
    records[2]['payload']['output'] = json.dumps({'output': '    ' + 'fictional result ' * 12 + 'OUTPUT_TAIL\n' + '\n'.join(f'row {i}' for i in range(45)) + '\nBOTTOM_TAIL', 'exit_code': 0})
    fixture.write_text('\n'.join(json.dumps(record) for record in records) + '\n')
    with module.Session(str(fixture), data_dir) as terminal:
        terminal.key(b'\r\r2'); terminal.expect('OUTPUT_TAIL')
        terminal.key(b'W'); assert 'OUTPUT_TAIL' not in terminal.text()
        terminal.key(b'W'); terminal.expect('OUTPUT_TAIL')
        terminal.key(b'/BOTTOM_TAIL\r'); terminal.expect('Match on line'); terminal.expect('BOTTOM_TAIL')
        terminal.key(b'1'); terminal.expect('INPUT_TAIL')
        terminal.key(b'v'); terminal.expect('INPUT_TAIL')
        terminal.key(b'W'); assert 'INPUT_TAIL' not in terminal.text()
        terminal.key(b'W'); terminal.expect('INPUT_TAIL')
        terminal.capture('inspector-wrapped-input')
        terminal.key(b'4'); terminal.expect('No interpretation'); terminal.expect('when evidence'); terminal.expect('changes.')
print(json.dumps({'terminal': '120x40 PTY', 'checks': ['input and output wrap', 'raw input wraps', 'unwrap and rewrap', 'search and scroll to wrapped output', 'interpretation wraps'], 'provider_requests': 0}))
