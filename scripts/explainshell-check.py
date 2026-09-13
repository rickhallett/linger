"""Read-only integration checks against the installed, pinned manpage pack.
Uses fictional commands, no API requests, and asserts commands never execute.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

root = Path(os.environ.get('LINGER_EXPLAINSHELL_DIR', str(Path(os.environ.get('XDG_DATA_HOME', str(Path.home()/'.local/share')))/'linger/explainshell')))
bridge = Path('src/explainshell_bridge.py').read_text()

def explain(command):
    response = subprocess.run([str(root/'.venv/bin/python'), '-I', '-c', bridge, str(root)], input=json.dumps({'command': command}), capture_output=True, text=True, timeout=5, env={})
    assert not response.stderr
    assert response.returncode == 0
    result = json.loads(response.stdout)
    assert not result.get('error'), result
    for part in result['spans']:
        assert 0 <= part['start'] < part['end'] <= len(command), part
        part['part'] = command[part['start']:part['end']]
    return result['spans']

def selected(parts, text):
    return next(p for p in parts if p['part'] == text)

started = time.monotonic()
parts = explain('rg -n "café" src | head -n 20')
assert 'line numbers' in selected(parts, '-n')['text'].lower()
assert 'regular expression' in selected(parts, '"café"')['text'].lower()
assert selected(parts, 'src')['kind'] == 'argument'
assert '/rg.' in selected(parts, 'src')['source']
assert selected(parts, '|')['kind'] == 'pipe'
parts = explain('ssh batch -o BatchMode=yes')
assert selected(parts, 'batch')['kind'] == 'argument'
assert '/ssh.' in selected(parts, 'batch')['source']
assert 'disables interactive' in selected(parts, '-o BatchMode=yes')['text']
parts = explain("sed -n '20,40p' file")
assert '20 through 40, inclusive' in selected(parts, "'20,40p'")['text']
assert selected(parts, 'file')['kind'] == 'argument'
parts = explain('git status --short --branch')
assert '/git-status.' in selected(parts, '--short')['source']
assert selected(parts, '--branch')['known']
parts = explain('rg --linger-unknown café src')
assert not selected(parts, '--linger-unknown')['known']
assert not selected(parts, 'café')['known']
parts = explain('rg -- -foo src')
assert selected(parts, '-foo')['known']
parts = explain('linger-fictional-command --unknown')
assert not any(p['known'] for p in parts)
with tempfile.TemporaryDirectory(prefix='linger-never-execute-') as temp:
    sentinel = Path(temp)/'executed'
    parts = explain(f"printf '%s' \"$(touch {sentinel})\"")
    assert not sentinel.exists()
    program = f"from pathlib import Path; Path('{sentinel}').touch()"
    parts = explain(f"python3 - <<'PY'\n{program}\nPY")
    assert selected(parts, "<<'PY'")['known']
    assert not selected(parts, program)['known']
    assert selected(parts, program)['kind'] == 'Python body'
    assert not sentinel.exists()
print(json.dumps({'checks': ['Unicode spans','rg operand roles','pipeline','SSH hostname','BatchMode','numeric sed range','Git subcommand flags','unknown option and uncertain operands','end of options','unknown executable','substitution never executed','Python body remains opaque'], 'backend': 'real local explainshell + pinned manpage pack', 'elapsed_seconds': round(time.monotonic()-started, 2), 'provider_requests': 0}))
