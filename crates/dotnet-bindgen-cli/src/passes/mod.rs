use crate::representations::{level_0, level_1, level_2};
use level_0::ToTokens;
use std::fmt::Debug;

pub trait Pass: Sized + Debug {
    type Input;
    type Output;

    fn perform(&self, input: Self::Input) -> Self::Output;
}

#[derive(Debug)]
pub struct ComposedPass<First: Pass, Second: Pass> {
    first: First,
    second: Second,
}

impl<Input, Intermediate, Output, FirstPass, SecondPass> Pass
    for ComposedPass<FirstPass, SecondPass>
where
    FirstPass: Pass<Input = Input, Output = Intermediate>,
    SecondPass: Pass<Input = Intermediate, Output = Output>,
{
    type Input = Input;
    type Output = Output;

    fn perform(&self, input: Self::Input) -> Self::Output {
        let intermediate = self.first.perform(input);
        self.second.perform(intermediate)
    }
}

/// This trait is the mechanism by which passes can be composed
pub trait AndThen<T: Pass> {
    fn and_then(self, second: T) -> ComposedPass<Self, T>
    where
        Self: Pass;
}

impl<First: Pass, Second: Pass> AndThen<Second> for First {
    fn and_then(self, second: Second) -> ComposedPass<Self, Second> {
        ComposedPass {
            first: self,
            second,
        }
    }
}

/// Inject formatting tokens (newlines, indents, etc..) with a basic heuristic to make the token
/// stream more human-readable.
#[derive(Debug)]
struct Level0Format {}

impl Pass for Level0Format {
    type Input = level_0::LayerEntrypoint;
    type Output = level_0::LayerEntrypoint;

    fn perform(&self, input: Self::Input) -> Self::Output {
        // TODO: Implement this pass
        input
    }
}

#[derive(Debug)]
struct LowerLevel1ToLevel0 {}

impl Pass for LowerLevel1ToLevel0 {
    type Input = level_1::LayerEntrypoint;
    type Output = level_0::LayerEntrypoint;

    fn perform(&self, input: Self::Input) -> Self::Output {
        input.to_token_stream()
    }
}

#[derive(Debug)]
struct LowerLevel2ToLevel1 {}

impl Pass for LowerLevel2ToLevel1 {
    type Input = level_2::BindingMethod;
    type Output = level_1::LayerEntrypoint;

    fn perform(&self, input: Self::Input) -> Self::Output {
        input.thunk_method()
    }
}

#[derive(Debug)]
pub struct Maybe<Repr, Inner>
where
    Repr: Debug,
    Inner: Pass<Input = Repr, Output = Repr>,
{
    enabled: bool,
    inner: Inner,
}

impl<Repr, Inner> Pass for Maybe<Repr, Inner>
where
    Repr: Debug,
    Inner: Pass<Input = Repr, Output = Repr>,
{
    type Input = Repr;
    type Output = Repr;

    fn perform(&self, input: Self::Input) -> Self::Output {
        if self.enabled {
            self.inner.perform(input)
        } else {
            input
        }
    }
}

pub trait OnlyIf<Repr: Debug>: Pass<Input = Repr, Output = Repr> {
    fn only_if(self, enabled: bool) -> Maybe<Repr, Self>;
}

impl<Repr, Inner> OnlyIf<Repr> for Inner
where
    Repr: Debug,
    Inner: Pass<Input = Repr, Output = Repr>,
{
    fn only_if(self, enabled: bool) -> Maybe<Repr, Self> {
        Maybe {
            enabled,
            inner: self,
        }
    }
}

/// Build a balanced tree of ComposedPass structs from a list of individual passes
macro_rules! passes {
    ($pass:expr) => { $pass };
    ($firstPass:expr, $secondPass:expr) => {
        ($firstPass).and_then($secondPass)
    };
    ($firstPass:expr, $secondPass:expr, $($others:expr),+) => {
        $firstPass
            .and_then($secondPass)
            .and_then(passes!($($others),+))
    };
}

pub fn default_passes(
    format_output: bool,
) -> impl Pass<Input = level_2::LayerEntrypoint, Output = level_0::LayerEntrypoint> {
    passes!(
        LowerLevel2ToLevel1 {},
        LowerLevel1ToLevel0 {},
        Level0Format {}.only_if(format_output)
    )
}

#[test]
fn print_default_passes() {
    dbg!(default_passes(true));
    dbg!(default_passes(false));
}