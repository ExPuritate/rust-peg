#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable", feature(error_in_core))]
#![allow(internal_features)]
#![feature(try_trait_v2)]
#![feature(panic_internals)]
#![feature(cold_path)]
#![feature(rustc_allow_const_fn_unstable)]
#![feature(const_precise_live_drops)]
#![feature(const_trait_impl)]
#![feature(const_destruct)]

use core::{
    convert::Infallible,
    hint::cold_path,
    marker::Destruct,
    ops::{ControlFlow, FromResidual, Try},
    panicking::panic,
    pin::Pin,
};
use std::fmt::Display;

pub mod error;
mod slice;
pub mod str;

/// The result type used internally in the parser.
///
/// You'll only need this if implementing the `Parse*` traits for a custom input
/// type, or using the `#{}` syntax to embed a custom Rust snippet within the parser.
///
/// The public API of a parser adapts errors to `std::result::Result` instead of using this type.
#[derive(Clone, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum RuleResult<T> {
    /// Success, with final location
    Matched(usize, T),

    /// Failure (furthest failure location is not yet known)
    Failed,
}

impl<T> FromResidual<RuleResult<Infallible>> for RuleResult<T> {
    fn from_residual(residual: RuleResult<Infallible>) -> Self {
        match residual {
            RuleResult::Failed => RuleResult::Failed,
        }
    }
}

impl<T> Try for RuleResult<T> {
    type Output = (usize, T);
    type Residual = RuleResult<Infallible>;
    fn from_output(output: Self::Output) -> Self {
        RuleResult::Matched(output.0, output.1)
    }
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            RuleResult::Matched(pos, value) => ControlFlow::Continue((pos, value)),
            RuleResult::Failed => ControlFlow::Break(RuleResult::Failed),
        }
    }
}

impl<T> RuleResult<T> {
    pub const fn is_matched(&self) -> bool {
        matches!(*self, RuleResult::Matched(_, _))
    }

    pub const fn is_matched_and<F>(self, f: F) -> bool
    where
        F: [const] FnOnce(usize, T) -> bool + [const] Destruct,
    {
        match self {
            Self::Failed => false,
            Self::Matched(pos, val) => f(pos, val),
        }
    }

    pub const fn is_failed(&self) -> bool {
        !self.is_matched()
    }

    pub const fn is_failed_or<F>(self, f: F) -> bool
    where
        F: [const] FnOnce(usize, T) -> bool + [const] Destruct,
    {
        match self {
            Self::Failed => true,
            Self::Matched(pos, val) => f(pos, val),
        }
    }

    pub const fn as_ref(&self) -> RuleResult<&T> {
        match *self {
            Self::Failed => RuleResult::Failed,
            Self::Matched(pos, ref val) => RuleResult::Matched(pos, val),
        }
    }

    pub const fn as_mut(&mut self) -> RuleResult<&mut T> {
        match *self {
            Self::Failed => RuleResult::Failed,
            Self::Matched(pos, ref mut val) => RuleResult::Matched(pos, val),
        }
    }

    pub const fn unwrap(self) -> (usize, T) {
        match self {
            Self::Matched(pos, val) => (pos, val),
            Self::Failed => panic("called `RuleResult::unwrap()` on a `Failed` value"),
        }
    }

    /// # Safety
    ///
    /// Calling this method on [`Failed`] is *[undefined behavior]*.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    pub const unsafe fn unwrap_unchecked(self) -> (usize, T) {
        match self {
            Self::Matched(pos, val) => (pos, val),
            Self::Failed => {
                cold_path();
                unsafe { core::hint::unreachable_unchecked() }
            }
        }
    }

    pub const fn map<U, F>(self, f: F) -> RuleResult<U>
    where
        F: [const] FnOnce(usize, T) -> (usize, U) + [const] Destruct,
    {
        match self {
            Self::Matched(pos, val) => {
                let (pos, val) = f(pos, val);
                RuleResult::Matched(pos, val)
            }
            Self::Failed => RuleResult::Failed,
        }
    }
}

/// A type that can be used as input to a parser.
#[allow(clippy::needless_lifetimes)]
pub trait Parse {
    type PositionRepr: Display;
    fn start<'input>(&'input self) -> usize;
    fn is_eof<'input>(&'input self, p: usize) -> bool;
    fn position_repr<'input>(&'input self, p: usize) -> Self::PositionRepr;
}

/// A parser input type supporting the `[...]` syntax.
pub trait ParseElem<'input>: Parse {
    /// Type of a single atomic element of the input, for example a character or token
    type Element: Copy;

    /// Get the element at `pos`, or `Failed` if past end of input.
    fn parse_elem(&'input self, pos: usize) -> RuleResult<Self::Element>;
}

/// A parser input type supporting the `"literal"` syntax.
pub trait ParseLiteral: Parse {
    /// Attempt to match the `literal` string at `pos`, returning whether it
    /// matched or failed.
    fn parse_string_literal(&self, pos: usize, literal: &str) -> RuleResult<()>;
}

/// A parser input type supporting the `$()` syntax.
pub trait ParseSlice<'input>: Parse {
    /// Type of a slice of the input.
    type Slice;

    /// Get a slice of input.
    fn parse_slice(&'input self, p1: usize, p2: usize) -> Self::Slice;
}

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
extern crate core as std;

// needed for type inference on the `#{|input, pos| ..}` closure, since there
// are different type inference rules on closures in function args.
#[doc(hidden)]
pub fn call_custom_closure<I, T>(
    f: impl FnOnce(I, usize) -> RuleResult<T>,
    input: I,
    pos: usize,
) -> RuleResult<T> {
    f(input, pos)
}
