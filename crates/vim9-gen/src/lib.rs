use lexer::new_lexer;
use parser;
use parser::new_parser;
use parser::ArrayLiteral;
use parser::AssignStatement;
use parser::AugroupCommand;
use parser::AutocmdCommand;
use parser::Body;
use parser::CallExpression;
use parser::DefCommand;
use parser::DictLiteral;
use parser::EchoCommand;
use parser::ExCommand;
use parser::Expression;
use parser::GroupedExpression;
use parser::Identifier;
use parser::IfCommand;
use parser::InfixExpression;
use parser::Literal;
use parser::Operator;
use parser::RawIdentifier;
use parser::ReturnCommand;
use parser::ScopedIdentifier;
use parser::StatementCommand;
use parser::VarCommand;
use parser::Vim9ScriptCommand;
use parser::VimBoolean;
use parser::VimKey;
use parser::VimNumber;
use parser::VimOption;
use parser::VimString;

mod test_harness;

pub struct State {
    pub augroup: Option<Literal>,
}

pub trait Generate {
    fn gen(&self, state: &mut State) -> String;
}

impl Generate for ExCommand {
    fn gen(&self, state: &mut State) -> String {
        match self {
            ExCommand::Vim9Script(cmd) => cmd.gen(state),
            ExCommand::Var(cmd) => cmd.gen(state),
            ExCommand::Echo(cmd) => cmd.gen(state),
            ExCommand::Statement(cmd) => cmd.gen(state),
            ExCommand::Return(cmd) => cmd.gen(state),
            ExCommand::Def(cmd) => cmd.gen(state),
            ExCommand::If(cmd) => cmd.gen(state),
            ExCommand::Augroup(cmd) => cmd.gen(state),
            ExCommand::Autocmd(cmd) => cmd.gen(state),
            // ExCommand::Call(_) => todo!(),
            // ExCommand::Finish(_) => todo!(),
            // ExCommand::Skip => todo!(),
            // ExCommand::EndOfFile => todo!(),
            ExCommand::Comment(token) => format!("-- {}", token.text),
            ExCommand::NoOp(token) => format!("-- {:?}", token),
            _ => todo!("Have not yet handled: {:?}", self),
        }
    }
}

impl Generate for AugroupCommand {
    fn gen(&self, state: &mut State) -> String {
        state.augroup = Some(self.augroup_name.clone());

        let group = self.augroup_name.token.text.clone();
        let result = format!(
            r#"
    vim.api.nvim_create_augroup("{}", {{ clear = false }})

    {}
"#,
            group,
            self.body.gen(state)
        );

        state.augroup = None;

        result
    }
}

impl Generate for AutocmdCommand {
    fn gen(&self, state: &mut State) -> String {
        let group = match &state.augroup {
            Some(group) => format!("group = '{}',", group.token.text),
            None => "".to_string(),
        };

        let event_list = self
            .events
            .iter()
            .map(|e| format!("'{}'", e.token.text))
            .collect::<Vec<String>>()
            .join(", ");

        let callback = match &self.block {
            parser::AutocmdBlock::Command(cmd) => format!("function()\n{}\nend", cmd.gen(state)),
            parser::AutocmdBlock::Block(block) => format!("function()\n{}\nend", block.body.gen(state)),
        };

        format!(
            r#"
vim.api.nvim_create_autocmd({{ {} }}, {{
    {}
    callback = {},
}})
"#,
            event_list, group, callback
        )
    }
}

impl Generate for ReturnCommand {
    fn gen(&self, state: &mut State) -> String {
        match &self.expr {
            Some(expr) => format!("return {}", expr.gen(state)),
            None => "return".to_string(),
        }
    }
}

impl Generate for DefCommand {
    fn gen(&self, state: &mut State) -> String {
        // TODO: If this command follows certain patterns,
        // we will also need to define a vimscript function,
        // so that this function is available.
        //
        // this could be something just like:
        // function <NAME>(...)
        //   return luaeval('...', ...)
        // endfunction
        //
        // but we haven't done this part yet.
        // This is a "must-have" aspect of what we're doing.
        format!(
            r#"
local {} = function()
  {}
end"#,
            self.name.gen(state),
            self.body.gen(state)
        )
    }
}

impl Generate for StatementCommand {
    fn gen(&self, state: &mut State) -> String {
        match self {
            StatementCommand::Assign(assign) => assign.gen(state),
        }
    }
}

impl Generate for AssignStatement {
    fn gen(&self, state: &mut State) -> String {
        format!("{} = {}", self.left.gen(state), self.right.gen(state))
    }
}

impl Generate for IfCommand {
    fn gen(&self, state: &mut State) -> String {
        format!(
            r#"
if {} then
  {}
end"#,
            self.condition.gen(state),
            self.body.gen(state)
        )
        .trim()
        .to_string()
    }
}

impl Generate for Body {
    fn gen(&self, state: &mut State) -> String {
        self.commands
            .iter()
            .map(|cmd| cmd.gen(state))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl Generate for EchoCommand {
    fn gen(&self, state: &mut State) -> String {
        // TODO: Probably should add some function that
        // pretty prints these the way they would get printed in vim
        // or maybe just call vim.cmd [[echo <...>]] ?
        //  Not sure.
        //  Maybe have to expose something to get exactly the same
        //  results
        //
        // format!("vim.api.nvim_echo({}, false, {{}})", chunks)
        format!("print({})", self.expr.gen(state))
    }
}

impl Generate for Vim9ScriptCommand {
    fn gen(&self, _: &mut State) -> String {
        // TODO: Actually connect this
        // format!("require('vim9script').new_script {{ noclear = {} }}", self.noclear)
        format!("-- vim9script")
    }
}

impl Generate for VarCommand {
    fn gen(&self, state: &mut State) -> String {
        format!("local {} = {}", self.name.gen(state), self.expr.gen(state))
    }
}

impl Generate for Identifier {
    fn gen(&self, state: &mut State) -> String {
        match self {
            Identifier::Raw(raw) => raw.gen(state),
            Identifier::Scope(scoped) => scoped.gen(state),
        }
    }
}

impl Generate for ScopedIdentifier {
    fn gen(&self, state: &mut State) -> String {
        format!(
            "{}.{}",
            match self.scope {
                parser::VimScope::Global => "vim.g",
                _ => todo!(),
            },
            self.accessor.gen(state)
        )
    }
}

impl Generate for RawIdentifier {
    fn gen(&self, _: &mut State) -> String {
        self.name.clone()
    }
}

impl Generate for Expression {
    fn gen(&self, state: &mut State) -> String {
        match self {
            Expression::Identifier(identifier) => identifier.gen(state),
            Expression::Number(num) => num.gen(state),
            Expression::String(str) => str.gen(state),
            Expression::Boolean(bool) => bool.gen(state),
            Expression::Grouped(grouped) => grouped.gen(state),
            Expression::Call(call) => call.gen(state),
            Expression::Prefix(_) => todo!(),
            Expression::Infix(infix) => infix.gen(state),
            Expression::Array(array) => array.gen(state),
            Expression::Dict(dict) => dict.gen(state),
            Expression::VimOption(opt) => opt.gen(state),
            Expression::Empty => "".to_string(),
        }
    }
}

impl Generate for VimOption {
    fn gen(&self, state: &mut State) -> String {
        // TODO: not sure if i need to do something smarter than this
        format!("vim.api.nvim_get_option_value('{}', {{}})", self.option.gen(state))
    }
}

impl Generate for Literal {
    fn gen(&self, _: &mut State) -> String {
        self.token.text.clone()
    }
}

impl Generate for VimKey {
    fn gen(&self, _: &mut State) -> String {
        match self {
            VimKey::Literal(literal) => literal.token.text.clone(),
        }
    }
}

impl Generate for DictLiteral {
    fn gen(&self, state: &mut State) -> String {
        format!(
            "{{ {} }}",
            self.elements
                .iter()
                .map(|x| format!("{} = {}", x.key.gen(state), x.value.gen(state)))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Generate for ArrayLiteral {
    fn gen(&self, state: &mut State) -> String {
        format!(
            "{{ {} }}",
            self.elements
                .iter()
                .map(|x| x.gen(state))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Generate for VimString {
    fn gen(&self, _: &mut State) -> String {
        match self {
            VimString::SingleQuote(s) => format!("'{}'", s),
            VimString::DoubleQuote(s) => format!("\"{}\"", s),
        }
    }
}

impl Generate for CallExpression {
    fn gen(&self, state: &mut State) -> String {
        let func = match &(*self.expr) {
            Expression::Identifier(ident) => match ident {
                Identifier::Raw(raw) => {
                    if raw.name.to_lowercase() == raw.name {
                        if raw.name.starts_with("nvim_") {
                            format!("vim.api.{}", raw.name)
                        } else {
                            format!("vim.fn.{}", raw.name)
                        }
                    } else {
                        raw.name.clone()
                    }
                }
                Identifier::Scope(_) => todo!(),
            },
            // Expression::String(_) => todo!(),
            // Expression::Grouped(_) => todo!(),
            // Expression::Call(_) => todo!(),
            // Expression::Prefix(_) => todo!(),
            // Expression::Infix(_) => todo!(),
            _ => unimplemented!(),
        };

        format!(
            "{}({})",
            func,
            self.args
                .iter()
                .map(|e| e.gen(state))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Generate for GroupedExpression {
    fn gen(&self, state: &mut State) -> String {
        format!("({})", self.expr.gen(state))
    }
}

impl Generate for VimBoolean {
    fn gen(&self, _: &mut State) -> String {
        format!("{}", self.value)
    }
}

impl Generate for VimNumber {
    fn gen(&self, _: &mut State) -> String {
        self.value.clone()
    }
}

impl Generate for InfixExpression {
    fn gen(&self, state: &mut State) -> String {
        format!(
            "({} {} {})",
            self.left.gen(state),
            self.operator.gen(state),
            self.right.gen(state)
        )
    }
}

impl Generate for Operator {
    fn gen(&self, _: &mut State) -> String {
        match self {
            Operator::Plus => "+",
            Operator::Minus => "-",
            Operator::Bang => "not",
            Operator::Or => "or",
            Operator::And => "and",
            Operator::LessThan => "<",
            Operator::GreaterThan => ">",
            Operator::LessThanOrEqual => "<=",
            Operator::GreaterThanOrEqual => ">=",
            // _ => todo!("{:?}", self),
        }
        .to_string()
    }
}

pub fn eval(program: parser::Program) -> String {
    let mut state = State { augroup: None };
    let mut output = String::new();
    for command in program.commands.iter() {
        output += &command.gen(&mut state);
        output += "\n";
    }

    output
}

pub fn generate(contents: &str) -> String {
    let lexer = new_lexer(contents);
    let mut parser = new_parser(lexer);
    let program = parser.parse_program();

    stylua_lib::format_code(
        &eval(program),
        stylua_lib::Config::default(),
        None,
        stylua_lib::OutputVerification::None,
    )
    .unwrap()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_harness::exec_lua;

    macro_rules! snapshot {
        ($name:tt, $path:tt) => {
            #[test]
            fn $name() {
                let contents = include_str!($path);
                let mut settings = insta::Settings::clone_current();
                settings.set_snapshot_path("../testdata/output/");
                settings.bind(|| {
                    insta::assert_snapshot!(generate(contents));
                });
            }
        };
    }

    snapshot!(test_expr, "../testdata/snapshots/expr.vim");
    snapshot!(test_if, "../testdata/snapshots/if.vim");
    snapshot!(test_assign, "../testdata/snapshots/assign.vim");
    snapshot!(test_call, "../testdata/snapshots/call.vim");
    snapshot!(test_autocmd, "../testdata/snapshots/autocmd.vim");
    snapshot!(test_matchparen, "../testdata/snapshots/matchparen.vim");

    #[test]
    fn test_simple_def() {
        let contents = r#"
vim9script

def MyCoolFunc(): number
  return 5
enddef

var x = MyCoolFunc() + 1
"#;

        let generated = generate(contents);
        let eval = exec_lua(&generated, "x").unwrap();
        assert_eq!(eval, 6.into());
    }

    #[test]
    fn test_builtin_func() {
        let contents = r#"
vim9script

var x = len("hello")
"#;

        let generated = generate(contents);
        let eval = exec_lua(&generated, "x").unwrap();
        assert_eq!(eval, 5.into());
    }

    #[test]
    fn test_augroup_1() {
        let contents = r#"
vim9script

augroup matchparen
  # Replace all matchparen autocommands
  autocmd! CursorMoved,CursorMovedI,WinEnter * {
      echo "Block"
    }

  autocmd WinLeave * echo "Command"
augroup END

var x = len(nvim_get_autocmds({group: "matchparen"}))

"#;

        let generated = generate(contents);
        let eval = exec_lua(&generated, "x").unwrap();
        assert_eq!(eval, 4.into());
    }
}