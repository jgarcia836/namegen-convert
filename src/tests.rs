// Round-trip tests: an .ngt fixture and its .ngj equivalent should parse to
// the same document, and converting either format to the other and back
// should not lose or reorder anything.

use crate::model::{finalize, Document};
use crate::{ngj, ngt};

fn parsed_ngt(src: &str) -> Document {
    let (mut doc, mut issues) = ngt::parse(src).expect("valid ngt fixture");
    finalize(&mut doc, &mut issues);
    doc
}

fn parsed_ngj(src: &str) -> Document {
    let (mut doc, mut issues) = ngj::parse(src).expect("valid ngj fixture");
    finalize(&mut doc, &mut issues);
    doc
}

const NGT_BASIC: &str =
    "!start name\n\nfirst = Anna, Beth:2, Clara\nlast = Smith, Jones\nname = {first} {last}\n";

const NGJ_BASIC: &str = r#"{
  "start": "name",
  "categories": {
    "first": ["Anna", { "text": "Beth", "weight": 2 }, "Clara"],
    "last": ["Smith", "Jones"],
    "name": ["{first} {last}"]
  }
}"#;

#[test]
fn ngt_and_ngj_fixtures_parse_to_the_same_document() {
    assert_eq!(parsed_ngt(NGT_BASIC), parsed_ngj(NGJ_BASIC));
}

#[test]
fn ngt_survives_a_round_trip_through_ngj() {
    let original = parsed_ngt(NGT_BASIC);
    let as_ngj = ngj::write(&original);
    assert_eq!(original, parsed_ngj(&as_ngj));
}

#[test]
fn ngj_survives_a_round_trip_through_ngt() {
    let original = parsed_ngj(NGJ_BASIC);
    let as_ngt = ngt::write(&original);
    assert_eq!(original, parsed_ngt(&as_ngt));
}

#[test]
fn fallback_start_category_survives_a_round_trip() {
    let original = parsed_ngt("name = solo\n");
    assert_eq!(original.start.as_deref(), Some("name"));

    assert_eq!(original, parsed_ngj(&ngj::write(&original)));
    assert_eq!(original, parsed_ngt(&ngt::write(&original)));
}
