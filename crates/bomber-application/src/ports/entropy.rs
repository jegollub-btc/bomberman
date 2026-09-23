/// Where a match seed comes from when the config does not pin one.
///
/// A port rather than a call to the system RNG, because "every match is
/// reproducible" is a property worth being able to hold in a test: swap in
/// [`FixedEntropy`] and the whole session becomes deterministic end to end.
pub trait Entropy: Send + Sync {
    fn next_seed(&self) -> u64;
}

/// Always hands out the same seed. For tests and for pinned tournament runs.
#[derive(Debug, Clone, Copy)]
pub struct FixedEntropy(pub u64);

impl Entropy for FixedEntropy {
    fn next_seed(&self) -> u64 {
        self.0
    }
}
