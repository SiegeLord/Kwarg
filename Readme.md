# Keyword arguments for Rust

[![Build Status](https://travis-ci.org/SiegeLord/Kwarg.svg)](https://travis-ci.org/SiegeLord/Kwarg)

This crate provides a procedural macro that allows you to generate macro 
wrappers around your functions which support keyword and default arguments.

## Example

Within a simple crate, using this macro is as easy as this:

```rust
#![feature(phase)]

#[phase(plugin)]
extern crate kwarg_macros;

kwarg_decl!{foo(a = 1, b = None, c = Some(6))}

fn foo(a: int, b: Option<int>, c: Option<int>) -> (int, Option<int>, Option<int>)
{
	(a, b, c)
}

fn main()
{
	let ret = foo!(c = Some(2), b = Some(6));
	assert_eq!(ret, (1, Some(6), Some(2)));
}
```

It is not possible to export these generated macros, so instead you should 
provide a macro that re-generates them and export it instead:

`library`:

```rust
#![feature(macro_rules)]

#[macro_export]
macro_rules! library_kwargs
{
	() =>
	{
		kwarg_decl!{foo(a = 1, b = None, c = Some(6))}
	}
}

pub fn foo(a: int, b: Option<int>, c: Option<int>) -> (int, Option<int>, Option<int>)
{
	(a, b, c)
}
```

`application`:

```rust
#![feature(phase)]

#[phase(plugin)]
extern crate kwarg_macros;
#[phase(plugin, link)]
extern crate library;

library_kwargs!()

fn main()
{
	use library::foo;
	let ret = foo!(c = Some(2), b = Some(6));
	assert_eq!(ret, (1, Some(6), Some(2)));
}
```

## Syntax

The general syntax is as follows:

```
function_name '(' [required_arg_name | optional_arg_name '=' initializer_expr ],* ')'
```

E.g.

```rust
kwarg_decl!{function_name(req_arg1, req_arg2, opt_arg1 = 1, opt_arg2 = 2)}

// ...

function_name!(1, opt_arg1 = 2, req_arg2 = 2); // `req_arg1` is set positionally, `opt_arg2` remains at default
```

When invoking the generated macro, positional arguments must come before the 
optional arguments.

## Installation

### Via Cargo

The package name is `kwarg`.

## License

LGPL 3.0
