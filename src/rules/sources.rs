//! The bibliography that rule citations point into.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Source {
    pub id: &'static str,
    pub reference: &'static str,
}

pub const TEMPERLEY_2001: &str = "temperley2001";
pub const TEMPERLEY_2007: &str = "temperley2007";
pub const KOSTKA_PAYNE_ALMEN_2023: &str = "kostka2023";

pub const SOURCES: &[Source] = &[
    Source {
        id: TEMPERLEY_2001,
        reference: "Temperley, David. 2001. The Cognition of Basic Musical Structures. Cambridge, MA: MIT Press.",
    },
    Source {
        id: TEMPERLEY_2007,
        reference: "Temperley, David. 2007. Music and Probability. Cambridge, MA: MIT Press.",
    },
    Source {
        id: KOSTKA_PAYNE_ALMEN_2023,
        reference: "Kostka, Stefan, Dorothy Payne, and Byron Almén. 2023. Tonal Harmony, with an Introduction to Post-Tonal Music. 9th ed. New York: McGraw-Hill.",
    },
];
