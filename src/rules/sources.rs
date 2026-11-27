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
pub const SCHULTZ_ET_AL_2018: &str = "schultz2018";
pub const BURSTEIN_STRAUS_2016: &str = "burstein2016";
pub const CAPLIN_2024: &str = "caplin2024";

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
    Source {
        id: SCHULTZ_ET_AL_2018,
        reference: "Condit-Schultz, Nathaniel, Yaolong Ju, and Ichiro Fujinaga. 2018. \"A Flexible Approach to Automated Harmonic Analysis: Multiple Annotations of Chorales by Bach and Prætorius.\" In Proceedings of the 19th International Society for Music Information Retrieval Conference, 66–73. Paris.",
    },
    Source {
        id: BURSTEIN_STRAUS_2016,
        reference: "Burstein, L. Poundie, and Joseph N. Straus. 2016. Concise Introduction to Tonal Harmony. New York: W. W. Norton.",
    },
    Source {
        id: CAPLIN_2024,
        reference: "Caplin, William E. 2024. Cadence: A Study of Closure in Tonal Music. New York: Oxford University Press.",
    },
];
