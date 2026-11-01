//! Global and context-aware pitch adaptation. Source music is borrowed, never edited.
mod arrange;
mod context;
mod diagnostics;
mod report;
mod search;
pub use arrange::{Arrangement, ArrangementError, arrange, arrange_with_options};
pub use context::{ArrangementMode, ArrangementOptions};
pub use report::*;
pub use search::{
    Candidate, MelodySelection, SCORING_RULE, TransposeMode, TranspositionSearch,
    search_transposition, sky_15_instrument,
};

#[cfg(test)]
mod arrangement_tests;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod search_tests;
