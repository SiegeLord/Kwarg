// Copyright (c) 2014 by SiegeLord
//
// All rights reserved. Distributed under LGPL 3.0. For full terms see the file LICENSE.

#![crate_name="kwarg_macros"]
#![crate_type="dylib"]
#![feature(quote, plugin_registrar)]
#![feature(rustc_private)]

extern crate syntax;
extern crate rustc;

use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr, NormalTT, IdentTT, DummyResult, TTMacroExpander};
use syntax::parse::token;
use syntax::ast::Ident;
use syntax::parse::token::intern;
use rustc::plugin::Registry;

use std::slice::Iter;
use std::rc::Rc;

#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(registrar: &mut Registry)
{
	registrar.register_syntax_extension(intern("kwarg_decl"), IdentTT(Box::new(kwarg_decl), None))
}

fn get_span_from_tt(tt: &ast::TokenTree) -> Option<Span>
{
	match *tt
	{
		ast::TtToken(sp, _) => Some(sp),
		ast::TtDelimited(sp, _) => Some(sp),
		_ => None
	}
}

#[derive(Clone)]
struct KWargDecl
{
	name: ast::Ident,
	arg_names: Vec<String>,
	initializers: Vec<Option<ast::TokenTree>>
}

fn new_delimited(sp: Span, delim: token::DelimToken, tts: Vec<ast::TokenTree>) -> Rc<ast::Delimited>
{
	Rc::new(ast::Delimited{ delim: delim, open_span: sp, close_span: sp, tts: tts })
}

struct TTLookAhead<'l>
{
	tts: Iter<'l, ast::TokenTree>,
	cur_tt: Option<&'l ast::TokenTree>,
	next_tt: Option<&'l ast::TokenTree>,
}

impl<'l> TTLookAhead<'l>
{
	fn new(tts: Iter<'l, ast::TokenTree>) -> TTLookAhead<'l>
	{
		let mut ret = TTLookAhead
		{
			tts: tts,
			cur_tt: None,
			next_tt: None,
		};
		ret.bump();
		ret.bump();
		ret
	}

	fn bump(&mut self) -> Option<&'l ast::TokenTree>
	{
		self.cur_tt = self.next_tt;
		self.next_tt = self.tts.next();
		self.cur_tt
	}
}

impl TTMacroExpander for KWargDecl
{
	fn expand<'l>(&self, cx: &'l mut ExtCtxt, sp: Span, tts: &[ast::TokenTree]) -> Box<MacResult+'l>
	{
		let mut arg_vals = self.initializers.clone();

		let mut tts = TTLookAhead::new(tts.iter());
		let mut found_kwarg = false;
		let mut pos_arg_idx = 0us;

		'arg_list_loop: loop
		{
			let mut eq_span = sp;
			let arg_idx = match (tts.cur_tt, tts.next_tt)
			{
				(Some(&ast::TtToken(sp1, token::Ident(ref ident, _))), Some(&ast::TtToken(sp2, token::Eq))) =>
				{
					eq_span = sp2;
					let ident_str = ident.as_str();

					match self.arg_names.iter().position(|arg_name| &arg_name[] == ident_str)
					{
						Some(arg_idx) =>
						{
							found_kwarg = true;
							/* Skip argument name and `=` */
							tts.bump();
							tts.bump();

							arg_idx
						}
						None =>
						{
							cx.span_err(sp1, "unknown argument name");
							return DummyResult::any(sp);
						}
					}
				}
				(Some(tt), _) =>
				{
					if found_kwarg
					{
						cx.span_err(get_span_from_tt(tt).unwrap_or(sp), "positional arguments must preceede keyword arguments");
						return DummyResult::any(sp);
					}

					if pos_arg_idx == self.arg_names.len()
					{
						cx.span_err(sp, &format!("too many arguments passed to `{}` (expected {})", self.name.as_str(), self.arg_names.len())[]);
						return DummyResult::any(sp);
					}

					pos_arg_idx += 1;
					pos_arg_idx - 1
				}
				(None, _) => break 'arg_list_loop,
			};

			let mut initializer_tts = vec![];

			/* Collect tts until the next comma */
			let mut found_any = false;
			loop
			{
				match tts.cur_tt
				{
					Some(&ast::TtToken(sp, token::Comma)) =>
					{
						if !found_any
						{
							cx.span_err(sp, "unexpected token: `,`");
							return DummyResult::any(sp);
						}
						break;
					}
					Some(tt) =>
					{
						found_any = true;
						initializer_tts.push(tt.clone());
					}
					None => break,
				}
				tts.bump();
			}
			if !found_any
			{
				cx.span_err(eq_span, "expected argument value after `=`");
				return DummyResult::any(sp);
			}

			arg_vals[arg_idx] = Some(ast::TtDelimited(sp, new_delimited(sp, token::Brace, initializer_tts)));

			tts.bump();
		}

		/* Construct the call */
		let mut arg_tts = vec![];
		for (ii, tt) in arg_vals.into_iter().enumerate()
		{
			match tt
			{
				Some(tt) =>
				{
					arg_tts.push(tt);
					if ii < self.arg_names.len() - 1
					{
						arg_tts.push(ast::TtToken(sp, token::Comma));
					}
				},
				None =>
				{
					cx.span_err(sp, &format!("argument `{}` is required, but not given a value", self.arg_names[ii])[]);
					return DummyResult::any(sp);
				}
			}
		}

		let mut call_tts = vec![];
		call_tts.push(ast::TtToken(sp, token::Ident(self.name.clone(), token::Plain)));
		call_tts.push(ast::TtDelimited(sp, new_delimited(sp, token::Paren, arg_tts)));
		MacExpr::new(quote_expr!(cx, $call_tts))
	}
}

fn kwarg_decl<'l>(cx: &'l mut ExtCtxt, sp: Span, name: Ident, tts: Vec<ast::TokenTree>) -> Box<MacResult+'l>
{
	let mut tts = tts.iter();

	let mut arg_names = vec![];
	let mut initializers = vec![];

	loop
	{
		let arg_name = match tts.next()
		{
			Some(&ast::TtToken(sp, ref tok)) =>
			{
				match *tok
				{
					token::Ident(ref ident, _) => ident.name.as_str().to_string(),
					token::CloseDelim(token::Paren) => break,
					_ =>
					{
						cx.span_err(sp, "expected a sequence of `arg_name` or `arg_name = default_expr`");
						return DummyResult::any(sp);
					}
				}
			}
			Some(tt) =>
			{
				cx.span_err(get_span_from_tt(tt).unwrap_or(sp), "expected a sequence of `arg_name` or `arg_name = default_expr`");
				return DummyResult::any(sp);
			}
			None => break
		};

		let mut done = false;
		let initializer = match tts.next()
		{
			Some(&ast::TtToken(sp, ref tok)) =>
			{
				match *tok
				{
					token::Eq =>
					{
						let mut initializer_tts = vec![];

						loop
						{
							match tts.next()
							{
								Some(&ast::TtToken(_, token::Comma)) => break,
								Some(&ast::TtToken(_, token::CloseDelim(token::Paren))) | None =>
								{
									done = true;
									break
								}
								Some(tt) => initializer_tts.push(tt.clone()),
							}
						}

						Some(ast::TtDelimited(sp, new_delimited(sp, token::Brace, initializer_tts)))
					},
					_ => None
				}
			}
			_ => None
		};

		arg_names.push(arg_name);
		initializers.push(initializer);

		if done
		{
			break;
		}
	}

	cx.syntax_env.insert(intern(name.as_str()),
		NormalTT(Box::new(KWargDecl{ name: name, arg_names: arg_names, initializers: initializers }), None));

	return DummyResult::any(sp);
}
