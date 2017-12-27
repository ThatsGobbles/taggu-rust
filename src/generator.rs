use std::ops::{Generator, GeneratorState};

pub fn gen_to_iter<G>(g: G) -> impl Iterator<Item = G::Yield>
where
    G: Generator<Return = ()>
{
    struct I<G>(G);

    impl<G> Iterator for I<G>
    where
        G: Generator<Return = ()>
    {
        type Item = G::Yield;
        fn next(&mut self) -> Option<Self::Item> {
            match self.0.resume() {
                GeneratorState::Yielded(y) => Some(y),
                GeneratorState::Complete(()) => None,
            }
        }
    }

    I(g)
}
