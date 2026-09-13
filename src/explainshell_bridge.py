"""Linger's JSON adapter to a separately installed explainshell backend.
Only parses stdin as data. Never runs a captured command or writes the store.
"""
import html
import json
import logging
import re
import shlex
import sqlite3
from pathlib import Path
import sys

logging.disable(logging.CRITICAL)
root = Path(sys.argv[1])
sys.path.insert(0, str(root / "source"))

SHELL_SOURCE = 'Linger shell structure; https://www.gnu.org/software/bash/manual/html_node/Redirections.html'

def part(start, end, text, kind, known=True, source=SHELL_SOURCE):
    return {'start': start, 'end': end, 'text': text, 'kind': kind,
            'known': known, 'source': source, 'extractor': ''}

def diagnostic(code, message, start, end):
    return {'code': code, 'message': message, 'start': start, 'end': end}

def install_parser_adapters():
    """Process-local adapters for pinned bashlex 0.18. No backend files change."""
    import bashlex.ast, bashlex.errors, bashlex.heredoc, bashlex.tokenizer, bashlex.subst

    def read_heredoc(tokenizer, redirect, _lineno, strip_tabs):
        raw = redirect.output.word
        if "$'" in raw or '$"' in raw:
            raise NotImplementedError('heredoc delimiter dialect')
        decoded = shlex.split(raw)
        if len(decoded) != 1 or '\n' in decoded[0]:
            raise NotImplementedError('heredoc delimiter dialect')
        delimiter = decoded[0]
        quoted = raw != delimiter
        beginning = tokenizer._shell_input_line_index
        while True:
            line_start = tokenizer._shell_input_line_index
            line = tokenizer.readline(not quoted)
            if line is None:
                raise bashlex.errors.ParsingError('Unterminated here-document', tokenizer._shell_input_line, line_start)
            candidate = line[:-1] if line.endswith('\n') else line
            if strip_tabs:
                candidate = candidate.lstrip('\t')
            if candidate == delimiter:
                end = tokenizer._shell_input_line_index
                if tokenizer._shell_input_line[end-1:end] == '\n':
                    end -= 1
                # Keep raw body and terminator coordinates, even when tab stripping
                # or unquoted backslash-newline joining affected delimiter matching.
                redirect.heredoc = bashlex.ast.node(kind='heredoc', value='', pos=(beginning, end))
                redirect.heredoc.body_length = line_start - beginning
                redirect.heredoc.quoted = quoted
                redirect.heredoc.strip_tabs = strip_tabs
                return ''

    bashlex.heredoc.makeheredoc = read_heredoc
    discard = bashlex.tokenizer.tokenizer._discard_until
    def capture_comment(tokenizer, character):
        start = tokenizer._shell_input_line_index - 1
        discard(tokenizer, character)
        if character == '\n' and tokenizer._shell_input_line[start:start+1] == '#':
            comments = getattr(tokenizer, 'linger_comments', [])
            comments.append((start, tokenizer._shell_input_line_index))
            tokenizer.linger_comments = comments
    bashlex.tokenizer.tokenizer._discard_until = capture_comment

    expand = bashlex.subst._expandwordinternal
    def expand_word(parser, token, *args):
        try:
            return expand(parser, token, *args)
        except NotImplementedError:
            # The lexer already delimited the whole shell word. Keep it opaque;
            # do not reinterpret its expansions, but allow following AST nodes.
            marker = bashlex.ast.node(kind='parameter', value='__linger_opaque__', pos=(token.lexpos, token.endlexpos))
            marker.linger_opaque = True
            return [marker], token.value
    bashlex.subst._expandwordinternal = expand_word

def explain(command):
    import bashlex.ast, bashlex.parser, bashlex.errors
    from explainshell.store import Store
    from explainshell.matcher import Matcher
    from explainshell.errors import ProgramDoesNotExist
    install_parser_adapters()

    class ReferenceStore(Store):
        def find_man_page(self, *args, **kwargs):
            pages = super().find_man_page(*args, **kwargs)
            for page in pages:
                page.nested_cmd = False
                for option in page.options:
                    option.nested_cmd = False
            return pages

    class PartMatcher(Matcher):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self.extra = []
            self.commands = []
            self.body_language = []
            self.opaque = []
            self.substitutions = []

        def visitcommand(self, node, parts):
            words = [p for p in parts if p.kind == 'word']
            self.commands.append(words)
            language = interpreter_language(words) or 'Input'
            self.body_language.append(language)
            return super().visitcommand(node, parts)

        def visitnodeend(self, node):
            super().visitnodeend(node)
            if node.kind == 'command':
                self.body_language.pop()

        def visitredirect(self, node, input, r_type, output, heredoc):
            if heredoc is None:
                return super().visitredirect(node, input, r_type, output, heredoc)
            language = self.body_language[-1] if self.body_language else 'Input'
            rule = 'Quoting the delimiter prevents shell parameter, command and arithmetic expansion in the body.' if heredoc.quoted else 'The unquoted body permits shell parameter, command and arithmetic expansion; those expansions are not evaluated here.'
            if heredoc.strip_tabs:
                rule += ' <<- removes leading tabs, not spaces, from body and delimiter lines.'
            self.extra.append(part(node.pos[0], output.pos[1], 'Supply a here-document on the redirected input. ' + rule, 'here-document'))
            start, end = heredoc.pos
            body_end = start + heredoc.body_length
            terminator_start = body_end
            if body_end > start and self.s[body_end-1:body_end] == '\n':
                body_end -= 1
            if start < body_end:
                self.extra.append(part(start, body_end, language + ' body supplied as data. Shell structure is recognized; the body\'s meaning is not explained. Use i for contextual interpretation.', language + ' body', False))
            if terminator_start < end:
                self.extra.append(part(terminator_start, end, 'Ends this here-document; this delimiter line is not supplied as input.', 'delimiter'))
            return False

        def visitcommandsubstitution(self, node, command):
            self.substitutions.append(node.pos)
            return False

        def visitprocesssubstitution(self, node, command):
            self.substitutions.append(node.pos)
            return False

        def visitparameter(self, node, value):
            if getattr(node, 'linger_opaque', False):
                self.opaque.append(node.pos)
                return False
            return super().visitparameter(node, value)

    spans, diagnostics = [], []
    offset = 0
    functions = set()
    store = ReferenceStore(str(root / 'manpages.db'), read_only=True)
    try:
        while offset < len(command):
            if command[offset].isspace():
                offset += 1
                continue
            if command[offset] == '#':
                end = command.find('\n', offset)
                end = len(command) if end < 0 else end
                spans.append(part(offset, end, 'A shell comment, ending at the newline.', 'comment'))
                offset = end
                continue
            remaining = command[offset:]
            padded = remaining if remaining.endswith('\n') else remaining + '\n'
            parser = bashlex.parser._parser(padded, strictmode=True, expansionlimit=1)
            try:
                node = parser.parse()
                if not isinstance(node, bashlex.ast.node):
                    raise ValueError('No shell node')
                consumed = min(parser.tok._shell_input_line_index, len(remaining))
                if consumed <= 0:
                    raise ValueError('No parser progress')
            except (bashlex.errors.ParsingError, NotImplementedError, ValueError):
                code = 'unsupported_syntax' if sys.exc_info()[0] is NotImplementedError else 'malformed_input'
                message = 'This shell region uses unsupported syntax.' if code == 'unsupported_syntax' else 'This shell region is incomplete or malformed for the Bash reference parser.'
                spans.append(part(offset, len(command), message + ' Original input is preserved.', 'unparsed shell', False))
                diagnostics.append(diagnostic(code, message, offset, len(command)))
                break
            matcher = PartMatcher(padded, store, distro='ubuntu', release='26.04')
            matcher.functions = functions
            try:
                matcher.visit(node)
                unit_spans = group_spans(matcher.groups, remaining)
            except ProgramDoesNotExist:
                unit_spans = group_spans(matcher.groups, remaining)
            except Exception:
                unit_spans = []
                diagnostics.append(diagnostic('lookup_failed', 'Documentation matching failed for this shell unit; its syntax remains available.', offset, offset + consumed))
            functions = matcher.functions
            unit_spans += matcher.extra
            for start, end in getattr(parser.tok, 'linger_comments', []):
                end = min(end, len(remaining))
                if start < end:
                    unit_spans.append(part(start, end, 'A shell comment, ending at the newline.', 'comment'))
            # AST words preserve syntactic roles even when no manpage exists.
            for words in matcher.commands:
                for index, word in enumerate(words):
                    existing = [p for p in unit_spans if p['start'] <= word.pos[0] and p['end'] >= word.pos[1]]
                    if not existing:
                        unit_spans.append(part(*word.pos, 'No command documentation is available for this part.', 'command' if index == 0 else 'argument', False))
                    for p in existing:
                        if p['kind'] == 'unknown' and (p['start'], p['end']) == word.pos:
                            p['kind'] = 'command' if index == 0 else 'argument'
            def inner_reference(words):
                inner = PartMatcher(padded, store, distro='ubuntu', release='26.04')
                inner.visit(bashlex.ast.node(kind='command', parts=list(words), pos=(words[0].pos[0], words[-1].pos[1])))
                return group_spans(inner.groups, remaining)
            unit_spans = reference_overlays(unit_spans, matcher.commands, remaining, inner_reference)
            for start, end in matcher.substitutions:
                unit_spans = replace_region(unit_spans, start, end, part(start, end, 'Shell substitution. Its contents and resulting value are not explained or evaluated here.', 'shell substitution', False))
            # No surrounding manual span may claim an opaque body is documented.
            for body in [p for p in unit_spans if p['kind'].endswith(' body')]:
                unit_spans = replace_region(unit_spans, body['start'], body['end'], body)
            for start, end in matcher.opaque:
                # Split overlapping descriptions instead of claiming to explain
                # unsupported word syntax as an ordinary option or operand.
                unit_spans = replace_region(unit_spans, start, end, part(start, end, 'This shell word contains an unsupported expansion. Its contents remain opaque; surrounding commands are still parsed.', 'opaque expansion', False))
                diagnostics.append(diagnostic('unsupported_expansion', 'An expansion remains opaque.', offset + start, offset + end))
            for p in unit_spans:
                if 0 <= p['start'] < p['end'] <= len(remaining):
                    p['start'] += offset
                    p['end'] += offset
                    spans.append(p)
                else:
                    diagnostics.append(diagnostic('invalid_adapter_span', 'An invalid documentation span was discarded.', offset, offset + consumed))
            offset += consumed
    finally:
        store.close()
    return {'spans': sorted(spans, key=lambda p: (p['start'], p['end'])), 'diagnostics': diagnostics}

def replace_region(spans, start, end, replacement):
    kept = []
    for p in spans:
        if p['end'] <= start or p['start'] >= end:
            kept.append(p)
        else:
            if p['start'] < start:
                kept.append({**p, 'end': start})
            if p['end'] > end:
                kept.append({**p, 'start': end})
    if start < end:
        kept.append(replacement)
    return kept

UV_FLAGS = {
    '--with': (True, 'Add the specified package for this invocation.'),
    '--with-requirements': (True, 'Add packages from a requirements file.'),
    '--python': (True, 'Select the Python interpreter.'),
    '-p': (True, 'Select the Python interpreter.'),
    '--offline': (False, 'Disable network access.'),
    '--no-sync': (False, 'Skip environment synchronization.'),
    '--no-project': (False, 'Skip project discovery.'),
    '--locked': (False, 'Require the lockfile to remain unchanged.'),
    '--frozen': (False, 'Use the lockfile without checking whether it is current.'),
    '--help': (False, 'Show help.'),
    '-h': (False, 'Show help.'),
}

def interpreter_language(words):
    if not words or words[0].parts:
        return None
    name = Path(words[0].word).name
    if name == 'uv' and len(words) > 2 and words[1].word == 'run' and not words[1].parts:
        at = 2
        while at < len(words):
            word = words[at]
            if word.parts:
                return None
            if word.word == '--':
                at += 1
                break
            if not word.word.startswith('-'):
                break
            flag, equal, _ = word.word.partition('=')
            if flag not in UV_FLAGS or (equal and not UV_FLAGS[flag][0]):
                return None
            at += 2 if UV_FLAGS[flag][0] and not equal else 1
        return interpreter_language(words[at:])
    if re.fullmatch(r'python(?:3(?:\.\d+)?)?', name):
        return 'Python'
    return 'JavaScript' if name in ('node', 'nodejs') else None

def reference_overlays(spans, commands, source, inner_reference):
    """Small source-labelled references, never host/version detection."""
    uv_source = 'Linger uv 0.12 reference (2026-09-13); https://docs.astral.sh/uv/reference/cli/#uv-run'
    pnpm_source = 'Linger pnpm 10 reference; https://pnpm.io/10.x/cli/run'
    interpreters = list(commands)
    for words in commands:
        if not words:
            continue
        name = Path(words[0].word).name
        if name not in ('uv', 'pnpm') or words[0].parts:
            continue
        reference = uv_source if name == 'uv' else pnpm_source
        def add(start, end, text, kind, known=True):
            nonlocal spans
            spans = replace_region(spans, start, end, part(start, end, text, kind, known, reference))
        add(*words[0].pos, 'Python project and package tool.' if name == 'uv' else 'JavaScript package and script manager.', 'command')
        if len(words) < 2 or words[1].word != 'run' or words[1].parts:
            continue
        add(*words[1].pos, 'Run a command in the project environment.' if name == 'uv' else 'Run a script defined in package.json.', 'subcommand')
        at = 2
        if name == 'pnpm':
            if at < len(words) and not words[at].word.startswith('-'):
                add(*words[at].pos, 'Name of the package script. Its implementation is defined by the project.', 'script name')
                for word in words[at+1:]:
                    add(*word.pos, 'Passed to the package script; its meaning depends on that script.', 'script argument', False)
            continue
        while at < len(words):
            word = words[at]
            literal = word.word
            if word.parts:
                at = len(words)
                break
            if literal == '--':
                add(*word.pos, 'End uv options; the following word selects the command or script.', 'separator')
                at += 1
                break
            if not literal.startswith('-'):
                break
            flag, equal, _ = literal.partition('=')
            if flag not in UV_FLAGS or (equal and not UV_FLAGS[flag][0]):
                # Unknown flags may consume a value. Never guess the child boundary.
                at = len(words)
                break
            takes_value, note = UV_FLAGS[flag]
            end = word.pos[1]
            if takes_value and not equal:
                if at + 1 >= len(words):
                    add(*word.pos, 'This option requires a value that is absent from the recorded command.', 'option', False)
                    at = len(words)
                    break
                at += 1
                end = words[at].pos[1]
            add(word.pos[0], end, note, 'option')
            at += 1
        if at < len(words):
            child = words[at:]
            if child[0].parts:
                add(*child[0].pos, 'The wrapped executable is computed; its documentation cannot be selected statically.', 'wrapped command', False)
            elif child[0].word.endswith('.py'):
                add(*child[0].pos, 'Python script selected by uv; script semantics are not explained here.', 'script name')
            else:
                for p in inner_reference(child):
                    spans = replace_region(spans, p['start'], p['end'], p)
                interpreters.append(child)
    # Inline interpreter code is data, even when a manual matched -c plus its value.
    for words in interpreters:
        if not words or words[0].parts:
            continue
        name = Path(words[0].word).name
        language = 'Python' if re.fullmatch(r'python(?:3(?:\.\d+)?)?', name) else 'JavaScript' if name in ('node', 'nodejs') else None
        if language:
            for index, word in enumerate(words[1:], 1):
                if word.word in (('-c',) if language == 'Python' else ('-e', '--eval')) and not word.parts and index + 1 < len(words):
                    body = words[index + 1]
                    spans = replace_region(spans, *body.pos, part(*body.pos, language + ' program string. Shell quoting is preserved; program semantics are not explained.', language + ' body', False))
                    break
                if not word.word.startswith('-'):
                    break
    return spans


def group_spans(groups, command):
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
    return spans

try:
    command = json.loads(sys.stdin.read(131073))["command"]
    if not isinstance(command, str) or len(command.encode()) > 32768:
        result = {"error": "Command input is invalid or exceeds the 32 KiB limit.", "error_code": "input_limit"}
    else:
        result = explain(command)
except (ImportError, OSError, sqlite3.Error):
    result = {"error": "The local documentation backend is unavailable. Original input is preserved.", "error_code": "backend_unavailable"}
except Exception:
    # Exception messages can contain private command arguments. Never echo them.
    result = {"error": "The local documentation adapter failed. Original input is preserved.", "error_code": "adapter_failure"}
sys.stdout.write(json.dumps(result))
