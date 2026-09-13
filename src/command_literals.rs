//! Static extraction from JavaScript call expressions; never evaluates source.
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, CallExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
#[derive(Clone, Debug)]
pub struct Literal {
    pub text: String,
    /// Decoded UTF-8 bytes mapped to the original JavaScript source.
    pub origin: Vec<(usize, usize)>,
    source_start: usize,
}

pub fn is_orchestration(tool: &str) -> bool {
    matches!(tool, "exec" | "functions.exec")
}

fn string(source: &str, start: usize) -> Option<(Literal, usize, bool)> {
    let quote = source.as_bytes()[start] as char;
    let mut at = start + 1;
    let mut out = Literal {
        text: String::new(),
        origin: Vec::new(),
        source_start: start,
    };
    let mut constant = true;
    while at < source.len() {
        let c = source[at..].chars().next()?;
        if c == quote {
            return Some((out, at + 1, constant));
        }
        if quote == '`' && source[at..].starts_with("${") {
            // Only constant templates have a single decodable source map.
            return None;
        }
        let begin = at;
        at += c.len_utf8();
        let decoded = if c == '\\' {
            let escape = source[at..].chars().next()?;
            at += escape.len_utf8();
            match escape {
                'n' => "\n".into(),
                'r' => "\r".into(),
                't' => "\t".into(),
                'b' => "\u{8}".into(),
                'f' => "\u{c}".into(),
                'v' => "\u{b}".into(),
                '\n' => String::new(),
                '\r' => {
                    if source.as_bytes().get(at) == Some(&b'\n') {
                        at += 1;
                    }
                    String::new()
                }
                '\\' | '\'' | '"' | '`' | '$' => escape.to_string(),
                'x' | 'u' => {
                    if escape == 'u' && source.as_bytes().get(at) == Some(&b'{') {
                        let end = at + source[at..].find('}')?;
                        let value = u32::from_str_radix(source.get(at + 1..end)?, 16).ok()?;
                        at = end + 1;
                        char::from_u32(value)?.to_string()
                    } else {
                        let digits = if escape == 'x' { 2 } else { 4 };
                        let hex = source.get(at..at + digits)?;
                        let mut value = u32::from_str_radix(hex, 16).ok()?;
                        at += digits;
                        if escape == 'u' && (0xD800..=0xDBFF).contains(&value) {
                            if source.get(at..at + 2)? != "\\u" {
                                return None;
                            }
                            let low = u32::from_str_radix(source.get(at + 2..at + 6)?, 16).ok()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return None;
                            }
                            at += 6;
                            value = 0x10000 + ((value - 0xD800) << 10) + low - 0xDC00;
                        }
                        char::from_u32(value)?.to_string()
                    }
                }
                _ => {
                    constant = false;
                    escape.to_string()
                }
            }
        } else {
            if quote != '`' && matches!(c, '\n' | '\r') {
                return None;
            }
            c.to_string()
        };
        out.origin
            .extend(std::iter::repeat_n((begin, at), decoded.len()));
        out.text.push_str(&decoded);
    }
    None
}
/// Result distinguishes absent shell calls from commands whose values need evaluation.
#[derive(Default)]
pub struct Extraction {
    pub literals: Vec<Literal>,
    pub calls: usize,
    pub syntax_error: bool,
}

pub fn extract(tool: &str, source: &str) -> Extraction {
    let mut result = Extraction::default();
    if !is_orchestration(tool) {
        return result;
    }
    if source.len() > 128 * 1024 {
        result.syntax_error = true;
        return result;
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    // Syntax recovery can synthesize nodes. Do not present those as recorded calls.
    if parsed.panicked || !parsed.errors.is_empty() {
        result.syntax_error = true;
        return result;
    }
    struct Collector<'s> {
        source: &'s str,
        result: Extraction,
    }
    impl Collector<'_> {
        fn literal(&self, call: &CallExpression<'_>) -> Option<Literal> {
            let [Argument::ObjectExpression(object)] = call.arguments.as_slice() else {
                return None;
            };
            let mut value = None;
            for property in &object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return None;
                };
                if property.computed {
                    return None;
                }
                let key = match &property.key {
                    PropertyKey::StaticIdentifier(key) => key.name.as_str(),
                    PropertyKey::StringLiteral(key) => key.value.as_str(),
                    _ => return None,
                };
                if !matches!(key, "cmd" | "command") {
                    continue;
                }
                if value.is_some()
                    || property.method
                    || property.shorthand
                    || property.kind != PropertyKind::Init
                {
                    return None;
                }
                let span = property.value.span();
                let cooked = match &property.value {
                    Expression::StringLiteral(s) => s.value.as_str(),
                    Expression::TemplateLiteral(t)
                        if t.expressions.is_empty() && t.quasis.len() == 1 =>
                    {
                        t.quasis[0].value.cooked.as_ref()?.as_str()
                    }
                    _ => return None,
                };
                // Retain the exact character-to-source map and cross-check its decoding
                // against the AST. Unsupported escapes stay unextracted, never guessed.
                let (literal, end, constant) = string(self.source, span.start as usize)?;
                if !constant || end != span.end as usize || literal.text != cooked {
                    return None;
                }
                value = Some(literal);
            }
            value
        }
    }
    impl<'a> Visit<'a> for Collector<'_> {
        fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
            if let Expression::StaticMemberExpression(member) = &call.callee
                && !member.optional
                && !call.optional
                && let Expression::Identifier(object) = &member.object
                && object.name == "tools"
                && matches!(
                    member.property.name.as_str(),
                    "exec_command" | "shell_command"
                )
            {
                self.result.calls += 1;
                if let Some(literal) = self.literal(call) {
                    self.result.literals.push(literal);
                }
            }
            walk::walk_call_expression(self, call);
        }
    }
    let mut collector = Collector { source, result };
    collector.visit_program(&parsed.program);
    collector.result.literals.sort_by_key(|l| l.source_start);
    collector.result
}

pub fn commands(tool: &str, source: &str) -> Vec<Literal> {
    extract(tool, source).literals
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regex_division_and_dynamic_templates_do_not_erase_other_literal_calls() {
        let source = r#"const pattern = /tools.exec_command\(\{cmd:'fake'\}\)/;
const ratio = 20 / 4;
const dynamic = `value ${ratio / 2}`;
tools.exec_command({cmd: 'pwd', other: /x/});
tools.exec_command({cmd: dynamic});
tools.shell_command({command: `git status --short`});
const later = /after/;"#;
        let result = extract("exec", source);
        assert!(!result.syntax_error);
        assert_eq!(result.calls, 3);
        assert_eq!(
            result
                .literals
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>(),
            ["pwd", "git status --short"]
        );
    }
    #[test]
    fn ast_literals_preserve_unicode_escape_maps_and_empty_source_order() {
        let source = r#"tools.exec_command({cmd:'echo \uD83D\uDE42'});tools.exec_command({cmd:''});tools.exec_command({cmd:`echo \u{1F642}`});"#;
        let c = commands("exec", source);
        assert_eq!(
            c.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            ["echo 🙂", "", "echo 🙂"]
        );
        assert_eq!(
            &source[c[0].origin[5].0..c[0].origin[8].1],
            "\\uD83D\\uDE42"
        );
    }
    #[test]
    fn extraction_reports_absence_dynamic_values_and_invalid_source_separately() {
        let absent = extract("exec", "const s = /tools.exec_command/;");
        assert_eq!(absent.calls, 0);
        assert!(!absent.syntax_error);
        let dynamic = extract("exec", "tools.exec_command({cmd: value});");
        assert_eq!(dynamic.calls, 1);
        assert!(dynamic.literals.is_empty());
        assert!(extract("exec", "tools.exec_command({cmd:").syntax_error);
        for source in [
            "tools.exec_command({cmd:'pwd',command:'ls'})",
            "tools.exec_command({...x, cmd:'pwd'})",
            "tools.exec_command({cmd:'pwd', get command(){return 'ls'}})",
            "tools.exec_command({cmd:'pwd', [key]:42})",
            "new tools.exec_command({cmd:'pwd'})",
            "tools?.exec_command({cmd:'pwd'})",
        ] {
            assert!(commands("exec", source).is_empty(), "{source}");
        }
    }
    #[test]
    fn literal_calls_preserve_escapes_offsets_and_source_order() {
        let source = r#"text(await tools.exec_command({cmd: 'rg -n "café" src', workdir: dir})); await tools.exec_command({"cmd": `git status --short\nrg -l x .`})"#;
        let c = commands("functions.exec", source);
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].text, "git status --short\nrg -l x .");
        let pos = c[0].text.find("café").unwrap();
        assert_eq!(&source[c[0].origin[pos].0..c[0].origin[pos + 4].1], "café");
    }
    #[test]
    fn dynamic_arguments_comments_and_strings_are_not_shell_evidence() {
        for source in [
            r#"// tools.exec_command({cmd:'fake'})"#,
            r#"const s = "tools.exec_command({cmd:'fake'})""#,
            "tools.exec_command({cmd: command})",
            "tools.exec_command({cmd: 'rg ' + target})",
            "tools.exec_command({cmd: `rg ${target}`})",
            "tools.exec_command({cmd:'rg', ...opts})",
            "tools.exec_command({cmd:'rg', cmd:'git'})",
            "other.tools.exec_command({cmd:'rg'})",
            "tools.exec_command({['cmd']:'rg'})",
            "let re = /tools.exec_command({cmd:'fake'})/",
        ] {
            assert!(commands("exec", source).is_empty(), "{source}");
        }
        assert!(commands("python", "tools.exec_command({cmd:'rg'})").is_empty());
    }
}
