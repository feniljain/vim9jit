use anyhow::Result;
use vim9_lexer::Token;
use vim9_lexer::TokenKind;

use crate::Block;
use crate::Body;
use crate::ExCommand;
use crate::Literal;
use crate::Parser;

#[derive(Debug, PartialEq, Clone)]
pub struct AugroupCommand {
    augroup: Token,
    pub augroup_name: Literal,
    augroup_eol: Token,
    pub body: Body,
    augroup_end: Token,
    augroup_end_name: Token,
    augroup_end_eol: Token,
}

impl AugroupCommand {
    pub fn parse(parser: &mut Parser) -> Result<ExCommand> {
        Ok(ExCommand::Augroup(AugroupCommand {
            augroup: parser.expect_identifier_with_text("augroup")?,
            augroup_name: parser
                .expect_token(TokenKind::Identifier)?
                .try_into()?,
            augroup_eol: parser.expect_eol()?,
            // TODO: This should be until augroup END, unless you can't have nested ones legally
            body: Body::parse_until(parser, "augroup")?,
            augroup_end: parser.expect_identifier_with_text("augroup")?,
            augroup_end_name: parser.expect_identifier_with_text("END")?,
            augroup_end_eol: parser.expect_eol()?,
        }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AutocmdCommand {
    autocmd: Token,
    pub bang: bool,
    pub events: Vec<Literal>,
    pub pattern: Vec<Literal>,
    pub block: AutocmdBlock,
}

impl AutocmdCommand {
    pub fn parse(parser: &mut Parser) -> Result<ExCommand> {
        Ok(ExCommand::Autocmd(AutocmdCommand {
            // TODO: Accept au! for example
            autocmd: parser.expect_identifier_with_text("autocmd")?,
            bang: if parser.current_token.kind == TokenKind::Bang {
                parser.next_token();
                true
            } else {
                false
            },
            events: {
                let mut events = vec![];
                loop {
                    events.push(parser.pop().try_into()?);
                    if parser.current_token.kind != TokenKind::Comma {
                        break;
                    }

                    parser.next_token();
                }

                events
            },
            pattern: {
                let mut pattern = vec![];
                loop {
                    pattern.push(parser.pop().try_into()?);
                    if parser.current_token.kind != TokenKind::Comma {
                        break;
                    }

                    parser.next_token();
                }

                pattern
            },
            block: AutocmdBlock::parse(parser)?,
        }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AutocmdBlock {
    Command(Box<ExCommand>),
    Block(Block),
}

impl AutocmdBlock {
    pub fn parse(parser: &mut Parser) -> Result<AutocmdBlock> {
        Ok(match parser.current_token.kind {
            TokenKind::LeftBrace => AutocmdBlock::Block(Block::parse(parser)?),
            _ => AutocmdBlock::Command(parser.parse_command()?.into()),
        })
    }
}