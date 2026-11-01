//! Global pitch adaptation only. Source music is borrowed and never edited.
mod arrange;
mod report;
mod search;
pub use arrange::{Arrangement, ArrangementError, arrange};
pub use report::*;
pub use search::{
    Candidate, MelodySelection, SCORING_RULE, TransposeMode, TranspositionSearch,
    search_transposition, sky_15_instrument,
};

#[cfg(test)]
mod arrangement_tests;
#[cfg(test)]
mod search_tests;
