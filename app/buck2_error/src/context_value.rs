/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::fmt;
use std::sync::Arc;

use smallvec::SmallVec;
use starlark_syntax::codemap::FileSpan;
use starlark_syntax::span_display::span_display;

#[derive(allocative::Allocative, Clone)]
pub enum ContextValue {
    Dyn(Arc<str>),
    Tier(Tier),
    Tags(SmallVec<[crate::ErrorTag; 1]>),
    Typed(Arc<dyn TypedContext>),
    // Stable value for category key
    Key(Arc<str>),
    StarlarkError(StarlarkContext),
}

impl ContextValue {
    /// Returns whether the context should be included in the error message
    pub(crate) fn should_display(&self) -> bool {
        match self {
            Self::Dyn(..) => true,
            Self::Typed(e) => e.should_display(),
            // Displaying the category in the middle of an error message doesn't seem useful
            Self::Tier(_) => false,
            Self::Tags(_) => false,
            Self::Key(..) => false,
            Self::StarlarkError(..) => false,
        }
    }

    #[cfg(test)]
    pub(crate) fn assert_eq(&self, other: &Self) {
        match (self, other) {
            (ContextValue::Dyn(a), ContextValue::Dyn(b)) => {
                assert_eq!(a, b);
            }
            (ContextValue::Tier(a), ContextValue::Tier(b)) => {
                assert_eq!(a, b);
            }
            (ContextValue::Tags(a), ContextValue::Tags(b)) => {
                assert_eq!(a, b);
            }
            (ContextValue::Typed(left), ContextValue::Typed(right)) => {
                assert!(left.eq(&**right))
            }
            (ContextValue::Key(a), ContextValue::Key(b)) => {
                assert_eq!(a, b);
            }
            (ContextValue::StarlarkError(a), ContextValue::StarlarkError(b)) => {
                assert_eq!(a, b);
            }
            (_, _) => panic!("context variants don't match!"),
        }
    }
}

impl std::fmt::Display for ContextValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dyn(v) => f.write_str(v),
            Self::Tier(category) => write!(f, "{:?}", category),
            Self::Tags(tags) => write!(f, "{:?}", tags),
            Self::Typed(v) => std::fmt::Display::fmt(v, f),
            Self::Key(v) => f.write_str(v),
            Self::StarlarkError(v) => write!(f, "{}", v),
        }
    }
}

impl From<String> for ContextValue {
    fn from(value: String) -> Self {
        ContextValue::Dyn(value.into())
    }
}

impl<'a> From<&'a str> for ContextValue {
    fn from(value: &str) -> Self {
        ContextValue::Dyn(value.into())
    }
}

#[derive(
    allocative::Allocative,
    PartialEq,
    Eq,
    Copy,
    Clone,
    Debug,
    PartialOrd,
    Ord
)]
pub enum Tier {
    Input,
    Environment,
    Tier0,
}

impl Tier {
    pub fn combine(self, other: Option<Tier>) -> Tier {
        let Some(other) = other else { return self };
        std::cmp::max(self, other)
    }
}

impl From<Tier> for ContextValue {
    fn from(value: Tier) -> Self {
        ContextValue::Tier(value)
    }
}

pub trait TypedContext:
    allocative::Allocative + Send + Sync + std::fmt::Display + std::any::Any + 'static
{
    fn eq(&self, other: &dyn TypedContext) -> bool;

    fn should_display(&self) -> bool {
        true
    }
}

impl<T: TypedContext> From<T> for ContextValue {
    fn from(value: T) -> Self {
        ContextValue::Typed(Arc::new(value))
    }
}

#[derive(Clone, allocative::Allocative, Debug, PartialEq, Eq, Hash)]
pub struct StarlarkContext {
    // TODO(minglunli): We could take in the CallStack type and do some magic to make it look nicer
    // but I think just concatenating the string form and it's accurate and look good enough
    pub call_stack: String,
    pub error_msg: String,
    pub span: Option<FileSpan>,
}

impl StarlarkContext {
    pub fn concat(&self, other: Option<Self>) -> Self {
        if let Some(ctx) = other {
            let trimmed = ctx
                .call_stack
                .trim_start_matches(starlark_syntax::call_stack::CALL_STACK_TRACEBACK_PREFIX);
            let call_stack = format!("{}{}", self.call_stack, trimmed);
            Self {
                call_stack,
                error_msg: ctx.error_msg.clone(),
                span: ctx.span.clone(),
            }
        } else {
            self.clone()
        }
    }
}

impl std::fmt::Display for StarlarkContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let span = span_display(
            self.span.as_ref().map(|s| s.as_ref()),
            self.error_msg.as_str(),
            false,
        );
        write!(f, "{}\n{}", self.call_stack, span)
    }
}

#[cfg(test)]
mod tests {
    use crate::Tier;
    use crate::{self as buck2_error};

    #[derive(buck2_error_derive::Error, Debug)]
    #[error("test error")]
    struct TestError;

    #[test]
    fn test_category_not_in_formatting() {
        let e: crate::Error = TestError.into();
        let e = e.context("foo");
        let e2 = e.clone().context(crate::Tier::Input);
        assert_eq!(format!("{:#}", e), format!("{:#}", e2));
    }

    #[test]
    fn test_category_infra_preferred() {
        let e: crate::Error = TestError.into();
        let e = e
            .clone()
            .context(crate::Tier::Tier0)
            .context(crate::Tier::Input);
        assert_eq!(e.get_tier(), Some(crate::Tier::Tier0));
    }

    #[test]
    fn test_combine() {
        assert_eq!(Tier::Input.combine(None), Tier::Input);
        assert_eq!(Tier::Input.combine(Some(Tier::Input)), Tier::Input);
        assert_eq!(
            Tier::Input.combine(Some(Tier::Environment)),
            Tier::Environment
        );
        assert_eq!(Tier::Input.combine(Some(Tier::Tier0)), Tier::Tier0);
        assert_eq!(Tier::Environment.combine(Some(Tier::Tier0)), Tier::Tier0);
        assert_eq!(Tier::Tier0.combine(Some(Tier::Input)), Tier::Tier0);
    }
}
