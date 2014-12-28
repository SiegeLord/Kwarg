// This file is released into the public domain.

#![feature(phase)]

#[phase(plugin)]
extern crate kwarg_macros;

kwarg_decl!{foo(a = 1, b = None, c = Some(6))}

fn foo(a: int, b: Option<int>, c: Option<int>) -> (int, Option<int>, Option<int>)
{
	(a, b, c)
}

kwarg_decl!{bar(a)}

fn bar(a: int) -> int
{
	a
}

kwarg_decl!{baz()}

fn baz()
{
}

kwarg_decl!{baz2(a)}

fn baz2(_a: ())
{
}

#[test]
fn test()
{
	let ret = foo!();
	assert_eq!(ret, (1, None, Some(6)));
	let ret = foo!(c = Some(2), b = Some(6));
	assert_eq!(ret, (1, Some(6), Some(2)));
	let ret = foo!(a = 1 + 5);
	assert_eq!(ret, (6, None, Some(6)));
	
	let ret = bar!(a = 1 + 5);
	assert_eq!(ret, (6));

	baz!();

	let mut a;
	baz2!(a = a = 1u);
	let _b = a;
} 
