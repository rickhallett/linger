"""Linger's JSON adapter to a separately installed explainshell backend.
Only parses stdin as data. Never runs a captured command or writes the store.
"""
import html
import json
import logging
import re
import shlex
from pathlib import Path
import sys

logging.disable(logging.CRITICAL)
root = Path(sys.argv[1])
sys.path.insert(0, str(root / "source"))

def python_heredoc(command):
    # A bounded adapter for bashlex 0.18's quoted-heredoc parser limitation.
    # Accept exactly one Python stdin command and a quoted, literal delimiter.
    match = re.fullmatch(r"(?P<prefix>python(?:3(?:\.\d+)?)?[ \t]+-[ \t]*)(?P<redirect><<[ \t]*(?P<quote>['\"])(?P<delimiter>[A-Za-z_][A-Za-z0-9_]*)(?P=quote))[ \t]*\n(?P<body>[\s\S]*?)\n(?P<end>(?P=delimiter))[ \t]*\n?", command)
    if not match or match['delimiter'] in match['body'].splitlines():
        return None
    result = explain(match['prefix'].rstrip())
    def part(name, text, kind, known):
        start, end = match.span(name)
        if start != end:
            result['spans'].append({'start': start, 'end': end, 'text': text, 'kind': kind, 'known': known, 'source': 'Linger bounded heredoc rule; https://www.gnu.org/software/bash/manual/html_node/Redirections.html', 'extractor': ''})
    part('redirect', 'Feed the following lines to Python on standard input. Quoting the delimiter prevents shell parameter, command and arithmetic expansion in that body. The body is Python source, not a shell command.', 'here-document', True)
    part('body', 'Python source supplied on standard input. This local shell matcher does not explain Python semantics. Press i for a contextual explanation of the recorded program and output.', 'Python body', False)
    part('end', 'This literal delimiter ends the here-document; it is not passed to Python as program text.', 'delimiter', True)
    return result

def explain(command):
    heredoc = python_heredoc(command)
    if heredoc is not None:
        return heredoc
    from explainshell.store import Store
    from explainshell.matcher import Matcher
    from explainshell.errors import ProgramDoesNotExist
    class ReferenceStore(Store):
        def find_man_page(self, *args, **kwargs):
            pages = super().find_man_page(*args, **kwargs)
            for page in pages:
                # The pack incorrectly labels e.g. rg PATH and ssh destination
                # as nested commands. Shell AST nodes still get full matching;
                # command-looking operands stay operands in this adapter.
                page.nested_cmd = False
                for option in page.options:
                    option.nested_cmd = False
            return pages

    class PartMatcher(Matcher):
        def _merge_adjacent(self, matches):
            # Keep distinct operands selectable even if their manual text is
            # identical (sed script and file, for example). Coalesce only the
            # parser's character-by-character unknown remainder.
            result = []
            from dataclasses import replace
            for match in matches:
                if result and match.unknown and result[-1].unknown and result[-1].end == match.start:
                    result[-1] = replace(result[-1], end=match.end)
                else:
                    result.append(match)
            return result

    store = ReferenceStore(str(root / "manpages.db"), read_only=True)
    try:
        matcher = PartMatcher(command + "\n", store, distro="ubuntu", release="26.04")
        try:
            groups = matcher.match()
        except ProgramDoesNotExist:
            groups = matcher.groups
        spans = []
        for group in groups:
            page = group.manpage
            source = page.source if page else "Bash syntax via explainshell" if group.name == "shell" else "No matching manpage"
            extractor = (page.extractor or "unspecified") if page else ""
            explicit_sed_script = any(set((r.debug_info or {}).get("short", [])) & {"-e", "-f"} for r in group.results)
            end_options = False
            uncertain_operands = False
            for result in group.results:
                note = html.unescape(result.text) if result.text else None
                portion = command[result.start:result.end]
                debug = result.debug_info or {}
                matched = result.text is not None
                try:
                    words = shlex.split(portion)
                except ValueError:
                    words = []
                literal = words[0] if len(words) == 1 else ''
                if literal == '--':
                    end_options = True
                elif not end_options and literal.startswith('-') and literal != '-' and debug.get('kind') in ('argument', 'unknown'):
                    uncertain_operands = True
                    matched = False
                    note = 'No option documentation matched this flag. Its argument requirements and the roles of later operands are unknown.'
                elif uncertain_operands and debug.get('kind') == 'argument':
                    matched = False
                    note = 'An earlier option was not recognized. This operand cannot be assigned a reliable role by the local matcher.'
                if matched and not explicit_sed_script and page and page.name == 'sed' and debug.get('positional') == 'script-if-no-other-script':
                    literal = portion[1:-1] if len(portion) > 1 and portion[0] in "'\"" and portion[-1] == portion[0] else portion
                    numeric = re.fullmatch(r"([1-9][0-9]*)(?:,([1-9][0-9]*))?p", literal)
                    if numeric and (numeric[2] is None or int(numeric[2]) >= int(numeric[1])):
                        selected = f"lines {numeric[1]} through {numeric[2]}, inclusive" if numeric[2] else f"line {numeric[1]}"
                        note = f"Linger numeric-print rule: p prints {selected}. With -n, normal automatic printing is suppressed.\nReference: https://www.gnu.org/software/sed/manual/html_node/Addresses.html\n\nManpage excerpt:\n{note}"
                if page and page.name == 'ssh' and re.fullmatch(r"-o[ \t]+['\"]?BatchMode=yes['\"]?", portion):
                    note = f"Linger option rule: BatchMode=yes disables interactive password and host-key confirmation prompts.\nReference: https://man.openbsd.org/ssh_config#BatchMode\n\nManpage excerpt:\n{note}"
                spans.append({"start": result.start, "end": result.end,
                              "text": note if note else "No documentation matched this part. Use i for contextual interpretation.",
                              "source": source, "extractor": extractor,
                              "kind": (result.debug_info or {}).get("kind", group.name),
                              "known": matched})
        return {"spans": spans}
    finally:
        store.close()

try:
    command = json.loads(sys.stdin.read(131073))["command"]
    if not isinstance(command, str) or len(command.encode()) > 32768:
        raise ValueError()
    result = explain(command)
except Exception:
    # Parser errors can embed private arguments. Never echo them or tracebacks.
    result = {"error": "The local matcher could not parse or look up this command. Input remains available; use i for context."}
sys.stdout.write(json.dumps(result))
