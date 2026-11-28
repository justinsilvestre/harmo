use super::*;
use crate::pitch::tpc_from_name;

fn label(key: &str, root: &str, bass: &str) -> Label {
    let key = Key::parse(key, "major").unwrap();
    let chord = Chord::new(tpc_from_name(root).unwrap(), Quality::Major);
    Label::new(key, None, chord, tpc_from_name(bass).unwrap())
}

fn limits(count: usize, margin: f32) -> LayerLimits {
    LayerLimits { count, key_margin: margin, chord_margin: margin, numeral_margin: margin }
}

fn key(name: &str) -> Key {
    Key::parse(name, "major").unwrap()
}

/// Four hypotheses in C reading G, G6, F and C, then one in G reading G.
fn held() -> Vec<(f32, Label)> {
    vec![
        (10.0, label("C", "G", "G")),
        (9.5, label("C", "G", "B")),
        (9.0, label("C", "F", "F")),
        (8.5, label("C", "C", "C")),
        (8.0, label("G", "G", "G")),
    ]
}

#[test]
fn a_key_is_offered_however_many_chords_lead_within_another() {
    assert_eq!(layer_offers(&held(), 10.0, &limits(4, 4.0)).keys, vec![key("C"), key("G")]);
}

#[test]
fn chords_are_told_apart_by_inversion() {
    assert_eq!(layer_offers(&held(), 10.0, &limits(2, 4.0)).chords.len(), 2);
}

#[test]
fn a_chord_held_in_two_keys_is_offered_once() {
    assert_eq!(layer_offers(&held(), 10.0, &limits(4, 4.0)).chords.len(), 4);
}

#[test]
fn the_margin_limits_each_layer() {
    assert_eq!(layer_offers(&held(), 10.0, &limits(4, 1.0)).keys, vec![key("C")]);
}

#[test]
fn numerals_pair_offered_keys_with_offered_chords() {
    let offers = layer_offers(&held(), 10.0, &limits(8, 4.0));
    assert!(offers.numerals.contains(&label("G", "G", "B")));
}

#[test]
fn numerals_are_ranked_by_combined_shortfall() {
    let offers = layer_offers(&held(), 10.0, &limits(4, 4.0));
    assert_eq!(offers.numerals[..2], [label("C", "G", "G"), label("C", "G", "B")]);
}

#[test]
fn numerals_are_capped_at_the_offer_count() {
    assert_eq!(layer_offers(&held(), 10.0, &limits(3, 4.0)).numerals.len(), 3);
}

#[test]
fn a_numeral_takes_the_tonicization_of_its_best_holder() {
    let applied = Label::new(
        key("C"),
        Some(key("G")),
        Chord::new(tpc_from_name("D").unwrap(), Quality::Major),
        tpc_from_name("D").unwrap(),
    );
    let offers = layer_offers(&[(1.0, applied)], 1.0, &limits(4, 4.0));
    assert_eq!(offers.numerals, vec![applied]);
}

#[test]
fn a_value_without_a_shortfall_is_never_offered() {
    let held = [(f32::NEG_INFINITY, 1), (2.0, 2)];
    assert_eq!(by_shortfall(held.into_iter(), 2.0, 100.0, 4), vec![(2, 0.0)]);
}

#[test]
fn shortfall_counts_the_best_holder() {
    let held = [(1.0, 1), (3.0, 1)];
    assert_eq!(by_shortfall(held.into_iter(), 3.0, 4.0, 4), vec![(1, 0.0)]);
}
