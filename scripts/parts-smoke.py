"""Constituent frequency and syntax colours, fictional inputs in a real PTY."""
import importlib.util,json,tempfile
from pathlib import Path
spec=importlib.util.spec_from_file_location('pattern_smoke',Path(__file__).with_name('pattern-smoke.py'))
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
with tempfile.TemporaryDirectory(prefix='linger-parts-') as data_dir:
    fixture=Path(data_dir)/'parts.jsonl'
    base=[json.loads(line) for line in Path('examples/repair.jsonl').read_text().splitlines()][:3]
    records=[base[0]]
    for n,command in enumerate([
        ['/bin/zsh','-lc',"rg -n --hidden -g '*.rs' TODO src | sort -u && git diff --stat > report.txt"],
        "rg -n --hidden -g '*.py' FIXME tests",
    ]):
        records.append({'timestamp':f'2026-09-13T10:00:{n*10:02d}Z','type':'response_item','payload':{'type':'function_call','name':'exec_command','call_id':f'parts-{n}','arguments':json.dumps({'command':command})}})
        records.append({'timestamp':f'2026-09-13T10:00:{n*10+4:02d}Z','type':'response_item','payload':{'type':'function_call_output','call_id':f'parts-{n}','output':json.dumps({'output':f'Fictional result {n}','exit_code':0})}})
    fixture.write_text('\n'.join(json.dumps(r) for r in records)+'\n')
    with module.Session(str(fixture),data_dir) as terminal:
        terminal.key(b'b');terminal.expect('Parts');assert 'structured shell input' not in terminal.text()
        terminal.key(b'/rg -n\r');terminal.expect('2 here');terminal.expect('2 all');terminal.expect('2 rows');terminal.capture('frequency-parts')
        terminal.key(b'/shell |\r');terminal.expect('shell |');terminal.expect('1 here')
        terminal.key(b'/git diff\r');terminal.expect('git diff --stat')
        terminal.key(b'/rg -n\r\r');terminal.expect('Fictional result 0')
        terminal.key(b'3');terminal.expect('Part 1/');terminal.expect('unknown')
        def cell_after(text, offset):
            for y,line in enumerate(terminal.screen.display):
                x=line.find(text,36)
                if x>=0:return terminal.screen.buffer[y][x+offset]
            raise AssertionError((text,terminal.text()))
        assert cell_after('rg -n --hidden',3).fg=='e5c736',cell_after('rg -n --hidden',3)
        terminal.key(b'l');terminal.expect('Part 2/')
        assert cell_after('rg -n --hidden',0).fg=='63b07a',cell_after('rg -n --hidden',0)
        assert cell_after('rg -n --hidden',3).bg=='32473b'
        terminal.capture('command-syntax-colours')
    with module.Session(str(fixture),data_dir) as terminal:
        terminal.key(b'b/rg -n\r');terminal.expect('2 all');terminal.expect('2 here')
print(json.dumps({'terminal':'120x40 PTY','checks':['Parts default','flags across changed operands','subcommands and operators','source highlights','occurrence jump','syntax colours and selection','reopen deduplication'],'provider_requests':0}))
