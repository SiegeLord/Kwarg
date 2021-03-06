# Keyword arguments for Rust

[![Build Status](https://travis-ci.org/SiegeLord/Kwarg.svg)](https://travis-ci.org/SiegeLord/Kwarg)

This crate provides a procedural macro that allows you to generate macro 
wrappers around your functions which support keyword and default arguments.

## Example

Within a single crate, using this macro is as easy as this:

```rust
#![feature(plugin)]

#![plugin(kwarg_macros)]

kwarg_decl! foo(a = 1, b = None, c = Some(6));

fn foo(a: i32, b: Option<i32>, c: Option<i32>) -> (i32, Option<i32>, Option<i32>)
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
#[macro_export]
macro_rules! library_kwargs
{
	() =>
	{
		kwarg_decl! foo(a = 1, b = None, c = Some(6));
	}
}

pub fn foo(a: i32, b: Option<i32>, c: Option<i32>) -> (i32, Option<i32>, Option<i32>)
{
	(a, b, c)
}
```

`application`:

```rust
#![feature(plugin)]

#![plugin(kwarg_macros)]
#[macro_use]
extern crate library;

library_kwargs!();

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
kwarg_decl! function_name(req_arg1, req_arg2, opt_arg1 = 1, opt_arg2 = 2);

// ...

function_name!(1, opt_arg1 = 2, req_arg2 = 2); // `req_arg1` is set positionally, `opt_arg2` remains at default
```

When invoking the generated macro, positional arguments must come before the 
optional arguments.

## Installation

### Via Cargo

* [kwarg_macros](https://crates.io/crates/kwarg_macros)

## License

LGPL 3.0
