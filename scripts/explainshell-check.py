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

def response(command):
    response = subprocess.run([str(root/'.venv/bin/python'), '-I', '-c', bridge, str(root)], input=json.dumps({'command': command}), capture_output=True, text=True, timeout=5, env={})
    assert not response.stderr
    assert response.returncode == 0
    result = json.loads(response.stdout)
    for part in result.get('spans', []):
        assert 0 <= part['start'] < part['end'] <= len(command), part
        part['part'] = command[part['start']:part['end']]
    for d in result.get('diagnostics', []):
        assert 0 <= d['start'] <= d['end'] <= len(command), d
    return result

def explain(command):
    result = response(command)
    assert not result.get('error'), result
    assert not result.get('diagnostics'), result.get('diagnostics')
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
# Complete shell units, delimiter quote removal, comments and preserved coordinates.
for delimiter in ["'PY'", '"PY"', 'P"Y"', r'P\Y', 'PY']:
    command = f"pwd\n# café\npython3 - <<{delimiter}\nprint('hello')\nPY\ngit status --short"
    parts = explain(command)
    assert selected(parts, 'pwd')['known']
    assert selected(parts, '# café')['kind'] == 'comment'
    assert selected(parts, "print('hello')")['kind'] == 'Python body'
    assert not selected(parts, "print('hello')")['known']
    assert selected(parts, '--short')['known']
    body = selected(parts, "print('hello')")
    assert not any(p['known'] and p['start'] < body['end'] and p['end'] > body['start'] for p in parts)
parts = explain("cat <<-'ONE' <<TWO\n\tfirst\n\tONE\nsecond\nTWO\npwd")
assert selected(parts, '\tfirst')['kind'] == 'Input body'
assert selected(parts, 'second')['kind'] == 'Input body'
assert selected(parts, '\tONE')['kind'] == 'delimiter'
assert selected(parts, 'pwd')['known']
parts = explain("cat <<'EMPTY'\nEMPTY\npwd")
assert not any(p['kind'].endswith(' body') for p in parts)
assert selected(parts, 'pwd')['known']
parts = explain("# only café")
assert len(parts) == 1 and parts[0]['kind'] == 'comment'
parts = explain('printf "%s" "# literal" # actual\npwd')
assert selected(parts, '# actual')['kind'] == 'comment'
assert selected(parts, 'pwd')['known']
assert selected(parts, '"# literal"')['kind'] != 'comment'
# Unsupported expansions keep whole words opaque while surrounding syntax survives.
for command in ['pwd; printf "%s" "$((1+2))"; git status --short', 'x=$((1+2))\npwd']:
    result = response(command)
    assert not result.get('error')
    assert [d['code'] for d in result['diagnostics']] == ['unsupported_expansion']
    assert selected(result['spans'], 'pwd')['known']
    assert any(p['kind'] == 'opaque expansion' and not p['known'] for p in result['spans'])
for tail in ['printf "unfinished', "cat <<'EOF'\nunterminated", 'case x in x) pwd;; esac']:
    result = response('pwd\n' + tail)
    assert not result.get('error')
    assert result['diagnostics'][0]['code'] in ('malformed_input', 'unsupported_syntax')
    assert selected(result['spans'], 'pwd')['known']
    assert selected(result['spans'], tail)['kind'] == 'unparsed shell'
# Substitution semantics must not inherit an outer argument's known status.
for command in ['echo "$(printf \'%s\' \"$((1+2))\")"; pwd', "echo \"$(cat <<'EOF'\nhello\nEOF\n)\"; pwd"]:
    parts = explain(command)
    sub = next(p for p in parts if p['kind'] == 'shell substitution')
    assert not sub['known']
    assert not any(p['known'] and p['start'] < sub['end'] and p['end'] > sub['start'] for p in parts)
    assert selected(parts, 'pwd')['known']
parts = explain('uv run --offline --with=pyte python3 -c "print(1)"')
assert 'uv 0.12 reference' in selected(parts, '--with=pyte')['source']
assert selected(parts, 'python3')['known']
assert selected(parts, '"print(1)"')['kind'] == 'Python body'
assert not selected(parts, '"print(1)"')['known']
parts = explain("uv run --with pyte python3 - <<'PY'\nprint(1)\nPY\npwd")
assert selected(parts, 'print(1)')['kind'] == 'Python body'
assert selected(parts, 'pwd')['known']
parts = explain('uv run --linger-unknown python3 -c "print(1)"')
assert not selected(parts, 'python3')['known']
parts = explain('uv run --with')
assert not selected(parts, '--with')['known']
parts = explain('pnpm run verify --silent --jobs 2')
assert selected(parts, 'verify')['kind'] == 'script name'
assert all(not selected(parts, p)['known'] for p in ['--silent', '--jobs', '2'])
parts = explain('node -e "console.log(1)"')
assert selected(parts, '"console.log(1)"')['kind'] == 'JavaScript body'
print(json.dumps({'checks': ['Unicode spans','rg operand roles','pipeline','SSH hostname','BatchMode','numeric sed range','Git subcommand flags','unknown option and uncertain operands','end of options','unknown executable','substitution never executed','Python body remains opaque','multiline shell units','quoted/mixed/unquoted delimiters','multiple and tab-stripped heredocs','empty heredoc','comment boundaries and final newline','partial malformed and unsupported syntax','opaque arithmetic and substitutions','uv wrapper and child body','unknown wrapper boundary','pnpm forwarded arguments','Node body'], 'backend': 'real local explainshell + pinned manpage pack', 'elapsed_seconds': round(time.monotonic()-started, 2), 'provider_requests': 0}))
