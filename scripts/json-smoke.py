"""Structured JSON presentation in the native terminal; fictional, local input."""
import importlib.util, json, tempfile
from pathlib import Path
spec = importlib.util.spec_from_file_location('terminal_helpers', Path(__file__).with_name('pattern-smoke.py'))
module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)

with tempfile.TemporaryDirectory(prefix='linger-json-pty-') as data:
    records = [json.loads(Path('examples/repair.jsonl').read_text().splitlines()[0])]
    records += [
        {'timestamp':'2026-09-13T13:00:01Z','type':'response_item','payload':{
            'type':'function_call','name':'exec_command','call_id':'json-fictional',
            'arguments':json.dumps({'cmd':'printf fictional','workdir':'/fictional/example'})}},
        {'timestamp':'2026-09-13T13:00:02Z','type':'response_item','payload':{
            'type':'function_call_output','call_id':'json-fictional','output':[
                {'type':'text','text':'Fictional result'},
                {'type':'resource','resource':{'uri':'fixture://example',
                    'metadata':{'ready':True,'count':2,'empty':None},
                    'text':'café 界 ' + 'a readable JSON value ' * 7 + 'JSON_TAIL'}}]}}]
    fixture = Path(data)/'structured.jsonl'
    fixture.write_text(''.join(json.dumps(row)+'\n' for row in records))
    with module.Session(str(fixture), data) as t:
        t.key(b'\r\r2');t.expect('[Structured content]');t.expect('"metadata": {');t.expect('JSON_TAIL')
        def color_for(needle):
            y = next(y for y,row in enumerate(t.screen.display) if needle in row)
            x = t.screen.display[y].index(needle)
            return t.screen.buffer[y][x]
        assert color_for('"metadata"').fg == 'b6e3bd'
        assert color_for('true').fg == 'a4d7be'
        assert color_for('JSON_TAIL').fg == '8bc8a3'  # continuation of a wrapped string
        assert color_for('"metadata"').bg == '14231b'
        code_row = next(y for y,row in enumerate(t.screen.display) if '"metadata"' in row)
        assert t.screen.buffer[code_row][117].bg == '14231b'
        t.capture('structured-json-readable')
        t.key(b'v');t.expect('"type": "resource"')
        assert '[Structured content]' not in t.text()
        assert color_for('"metadata"').fg == 'b6e3bd'
        t.key(b'v');t.expect('[Structured content]')
        t.key(b'/JSON_TAIL\r');t.expect('Match on line');t.expect('JSON_TAIL')
        t.key(b'1');t.expect('Invocation arguments');t.expect('"cmd":')
        assert color_for('"cmd"').fg == 'b6e3bd'
        t.capture('structured-json-input')
print(json.dumps({'checks':['indented structured JSON','mint keys and green values','wrapped Unicode string keeps colour','quiet code surface','raw view preserves payload','search works','input JSON uses same palette'],'provider_requests':0}))
