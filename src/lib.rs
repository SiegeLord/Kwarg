// Copyright (c) 2014 by SiegeLord
//
// All rights reserved. Distributed under LGPL 3.0. For full terms see the file LICENSE.

#![crate_name="kwarg_macros"]
#![crate_type="dylib"]
#![feature(quote, plugin_registrar, macro_rules)]

extern crate syntax;
extern crate rustc;

use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr, MacroDef, NormalTT, DummyResult, TTMacroExpander};
use syntax::parse::token;
use syntax::ptr::P;
use rustc::plugin::Registry;

use std::slice::Items;
use std::rc::Rc;

#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(registrar: &mut Registry)
{
	registrar.register_macro("kwarg_decl", kwarg_decl)
}

#[deriving(Clone)]
struct KWargHelper
{
	name: ast::Ident,
	arg_names: Vec<String>,
	initializers: Vec<Option<ast::TokenTree>>
}

impl MacResult for KWargHelper
{
	fn make_def(&mut self) -> Option<MacroDef>
	{
		Some(MacroDef{ name: self.name.as_str().to_string(), ext: NormalTT(box self.clone(), None)})
    }

	fn make_stmt(self: Box<KWargHelper>) -> Option<P<ast::Stmt>>
	{
		None
	}
}

struct TTLookAhead<'l>
{
	tts: Items<'l, ast::TokenTree>,
	cur_tt: Option<&'l ast::TokenTree>,
	next_tt: Option<&'l ast::TokenTree>,
}

impl<'l> TTLookAhead<'l>
{
	fn new(tts: Items<'l, ast::TokenTree>) -> TTLookAhead<'l>
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

impl TTMacroExpander for KWargHelper
{
	fn expand<'l>(&self, cx: &'l mut ExtCtxt, sp: Span, tts: &[ast::TokenTree]) -> Box<MacResult+'l>
	{
		let mut arg_vals = self.initializers.clone();

		let mut tts = TTLookAhead::new(tts.iter());
		let mut found_kwarg = false;
		let mut pos_arg_idx = 0u;

		'arg_list_loop: loop
		{
			let arg_idx = match (tts.cur_tt, tts.next_tt)
			{
				(Some(&ast::TTTok(sp1, token::IDENT(ref ident, _))), Some(&ast::TTTok(_, token::EQ))) =>
				{
					let ident_str = ident.as_str();

					match self.arg_names.iter().position(|arg_name| arg_name.as_slice() == ident_str)
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
				(Some(_), _) =>
				{
					if found_kwarg
					{
						cx.span_err(sp, "positional arguments must preceede keyword arguments");
						return DummyResult::any(sp);
					}

					if pos_arg_idx == self.arg_names.len()
					{
						cx.span_err(sp, format!("too many arguments passed to `{}` (expected {})", self.name.as_str(), self.arg_names.len()).as_slice());
						return DummyResult::any(sp);
					}

					pos_arg_idx += 1;
					pos_arg_idx - 1
				}
				(None, _) => break 'arg_list_loop,
			};

			let mut initializer_tts = vec![];

			initializer_tts.push(ast::TTTok(sp, token::LBRACE));

			/* Collect tts until the next comma */
			'next_arg_loop: loop
			{
				match tts.cur_tt
				{
					Some(&ast::TTTok(_, token::COMMA)) => break,
					Some(tt) => initializer_tts.push(tt.clone()),
					None => break,
				}
				tts.bump();
			}

			initializer_tts.push(ast::TTTok(sp, token::RBRACE));

			*arg_vals.get_mut(arg_idx) = Some(ast::TTDelim(Rc::new(initializer_tts)));

			tts.bump();
		}

		/* Construct the call */
		let mut arg_tts = vec![];
		arg_tts.push(ast::TTTok(sp, token::LPAREN));
		for (ii, tt) in arg_vals.move_iter().enumerate()
		{
			match tt
			{
				Some(tt) =>
				{
					arg_tts.push(tt);
					if ii < self.arg_names.len() - 1
					{
						arg_tts.push(ast::TTTok(sp, token::COMMA));
					}
				},
				None =>
				{
					cx.span_err(sp, format!("argument `{}` is required, but not given a value", self.arg_names[ii]).as_slice());
					return DummyResult::any(sp);
				}
			}
		}
		arg_tts.push(ast::TTTok(sp, token::RPAREN));

		let mut call_tts = vec![];
		call_tts.push(ast::TTTok(sp, token::IDENT(self.name.clone(), false)));
		call_tts.push(ast::TTDelim(Rc::new(arg_tts)));
		MacExpr::new(quote_expr!(cx, $call_tts))
	}
}

fn kwarg_decl(cx: &mut ExtCtxt, sp: Span, tts: &[ast::TokenTree]) -> Box<MacResult+'static>
{
	let mut tts = tts.iter();

	let name = match tts.next()
	{
		Some(&ast::TTTok(sp, ref tok)) =>
		{
			match *tok
			{
				token::IDENT(ref ident, _) => ident.clone(),
				_ =>
				{
					cx.span_err(sp, "expected identifier as an argument");
					return DummyResult::any(sp);
				}
			}
		}
		_ =>
		{
			cx.span_err(sp, "expected identifier as an argument");
			return DummyResult::any(sp);
		}
	};

	let mut arg_names = vec![];
	let mut initializers = vec![];

	match tts.next()
	{
		Some(&ast::TTDelim(ref tts)) =>
		{
			let mut tts = tts.iter();
			/* Skip the opening delim */
			match tts.next()
			{
				Some(&ast::TTTok(sp, ref tok)) =>
				{
					match *tok
					{
						token::LPAREN => (),
						_ =>
						{
							cx.span_err(sp, "expected '('");
							return DummyResult::any(sp);
						}
					}
				}
				_ =>
				{
					cx.span_err(sp, "expected '('");
					return DummyResult::any(sp);
				}
			}

			loop
			{
				let arg_name = match tts.next()
				{
					Some(&ast::TTTok(sp, ref tok)) =>
					{
						match *tok
						{
							token::IDENT(ref ident, _) => ident.name.as_str().to_string(),
							token::RPAREN => break,
							_ =>
							{
								cx.span_err(sp, "expected a sequence of `arg_name` or `arg_name = default_expr`");
								return DummyResult::any(sp);
							}
						}
					}
					Some(_) =>
					{
						cx.span_err(sp, "expected a sequence of `arg_name` or `arg_name = default_expr`");
						return DummyResult::any(sp);
					}
					None => break
				};

				let mut done = false;
				let initializer = match tts.next()
				{
					Some(&ast::TTTok(sp, ref tok)) =>
					{
						match *tok
						{
							token::EQ =>
							{
								let mut initializer_tts = vec![];

								initializer_tts.push(ast::TTTok(sp, token::LBRACE));

								loop
								{
									match tts.next()
									{
										Some(&ast::TTTok(_, token::COMMA)) => break,
										Some(&ast::TTTok(_, token::RPAREN)) | None =>
										{
											done = true;
											break
										}
										Some(tt) => initializer_tts.push(tt.clone()),
									}
								}

								initializer_tts.push(ast::TTTok(sp, token::RBRACE));

								Some(ast::TTDelim(Rc::new(initializer_tts)))
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
		}
		_ =>
		{
			cx.span_err(sp, "expected a set of delimited arguments for `kwarg_decl``");
			return DummyResult::any(sp);
		}
	}

	box KWargHelper{ name: name, arg_names: arg_names, initializers: initializers }
}
