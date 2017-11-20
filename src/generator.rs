use std::ops::{Generator, GeneratorState};

// // Wrapper type to use a generator as an iterator.
// // Taken from https://play.rust-lang.org/?gist=7f8d2960361237403fe4d19161c00731&version=nightly
// pub struct GenToIter<G>(G);

// impl<G> GenToIter<G> where G: Generator<Return=()> {
//     pub fn new(gen: G) -> GenToIter<G> {
//         GenToIter{ 0: gen }
//     }
// }

// impl<G> Iterator for GenToIter<G> where G: Generator<Return=()> {
//     type Item = G::Yield;
//     fn next(&mut self) -> Option<Self::Item> {
//         match self.0.resume() {
//             GeneratorState::Yielded(x) => Some(x),
//             GeneratorState::Complete(()) => None,
//         }
//     }
// }

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
