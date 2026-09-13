"""Validate a release tag against its independently versioned component."""
import json, os, re, sys, tomllib
kind, tag = sys.argv[1:]
if kind != 'runtime':
    raise SystemExit('Expected runtime')
match = re.fullmatch(rf'{kind}-v(\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?)', tag)
if not match:
    raise SystemExit('Invalid component release tag')
version = match[1]
with open('Cargo.toml', 'rb') as f:
    runtime = tomllib.load(f)['package']['version']
expected = runtime
if version != expected:
    raise SystemExit(f'Tag version {version} does not match manifest {expected}')
values = {'version': version, 'prerelease': str('-' in version).lower()}
print(json.dumps(values))
if os.environ.get('GITHUB_OUTPUT'):
    with open(os.environ['GITHUB_OUTPUT'], 'a') as f:
        for key, value in values.items():
            f.write(f'{key}={value}\n')
