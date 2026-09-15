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

// --check writes nothing; it only needs a scratch input file, never an
// output path. These tests give each call its own file name (built from the
// test name) so they can't collide when the test binary runs them in
// parallel threads.
fn scratch_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("namegen-convert-test-{}-{}.ngt", std::process::id(), label))
}

fn write_scratch(label: &str, contents: &str) -> std::path::PathBuf {
    let path = scratch_path(label);
    std::fs::write(&path, contents).expect("write scratch fixture");
    path
}

// Same idea as scratch_path, but for an output path a conversion is expected
// to create rather than an input fixture that's written up front.
fn scratch_output_path(label: &str, extension: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("namegen-convert-test-{}-{}.{}", std::process::id(), label, extension))
}

#[test]
fn check_accepts_a_valid_grammar_and_writes_nothing() {
    let path = write_scratch("check-valid", NGT_BASIC);
    let result = crate::run(vec!["--check".to_string(), path.to_string_lossy().into_owned()]);
    assert!(result.is_ok());
    std::fs::remove_file(&path).ok();
}

#[test]
fn check_rejects_an_undefined_placeholder_without_lenient() {
    let path = write_scratch("check-invalid", "name = {missing}\n");
    let result = crate::run(vec!["--check".to_string(), path.to_string_lossy().into_owned()]);
    assert!(result.is_err());
    std::fs::remove_file(&path).ok();
}

#[test]
fn check_with_lenient_accepts_an_undefined_placeholder() {
    let path = write_scratch("check-lenient", "name = {missing}\n");
    let result = crate::run(vec![
        "--check".to_string(),
        "--lenient".to_string(),
        path.to_string_lossy().into_owned(),
    ]);
    assert!(result.is_ok());
    std::fs::remove_file(&path).ok();
}

#[test]
fn check_rejects_a_second_positional_argument() {
    let path = write_scratch("check-extra-arg", NGT_BASIC);
    let result = crate::run(vec![
        "--check".to_string(),
        path.to_string_lossy().into_owned(),
        "extra.ngj".to_string(),
    ]);
    assert!(result.is_err());
    std::fs::remove_file(&path).ok();
}

#[test]
fn sample_accepts_a_numeric_seed() {
    let path = write_scratch("sample-seed-valid", NGT_BASIC);
    let result = crate::run_sample(vec!["--seed".to_string(), "42".to_string(), path.to_string_lossy().into_owned()]);
    assert!(result.is_ok());
    std::fs::remove_file(&path).ok();
}

#[test]
fn sample_rejects_a_non_numeric_seed() {
    let path = write_scratch("sample-seed-invalid", NGT_BASIC);
    let result = crate::run_sample(vec![
        "--seed".to_string(),
        "not-a-number".to_string(),
        path.to_string_lossy().into_owned(),
    ]);
    assert!(result.is_err());
    std::fs::remove_file(&path).ok();
}

// These exercise the actual conversion path (two positional args, real
// output file), which the tests above never touch: they cover --check and
// sample, but nothing that writes a converted file.

#[test]
fn convert_infers_formats_from_extension_and_writes_a_parseable_output() {
    let input = write_scratch("convert-infer", NGT_BASIC);
    let output = scratch_output_path("convert-infer", "ngj");

    let result = crate::run(vec![input.to_string_lossy().into_owned(), output.to_string_lossy().into_owned()]);
    assert!(result.is_ok());

    let written = std::fs::read_to_string(&output).expect("output file was written");
    assert_eq!(parsed_ngj(&written), parsed_ngt(NGT_BASIC));

    std::fs::remove_file(&input).ok();
    std::fs::remove_file(&output).ok();
}

#[test]
fn convert_respects_from_and_to_overrides_for_unrecognized_extensions() {
    let input = scratch_path("convert-override");
    let input = input.with_extension("dat");
    std::fs::write(&input, NGJ_BASIC).expect("write scratch fixture");
    let output = scratch_output_path("convert-override", "out");

    let result = crate::run(vec![
        "--from".to_string(),
        "ngj".to_string(),
        "--to".to_string(),
        "ngt".to_string(),
        input.to_string_lossy().into_owned(),
        output.to_string_lossy().into_owned(),
    ]);
    assert!(result.is_ok());

    let written = std::fs::read_to_string(&output).expect("output file was written");
    assert_eq!(parsed_ngt(&written), parsed_ngj(NGJ_BASIC));

    std::fs::remove_file(&input).ok();
    std::fs::remove_file(&output).ok();
}

#[test]
fn convert_rejects_an_undefined_placeholder_and_writes_nothing() {
    let input = write_scratch("convert-invalid", "name = {missing}\n");
    let output = scratch_output_path("convert-invalid", "ngj");
    std::fs::remove_file(&output).ok();

    let result = crate::run(vec![input.to_string_lossy().into_owned(), output.to_string_lossy().into_owned()]);
    assert!(result.is_err());
    assert!(!output.exists(), "conversion should not write output when validation fails");

    std::fs::remove_file(&input).ok();
}

#[test]
fn convert_with_lenient_merges_duplicate_categories_in_the_output() {
    let input = write_scratch("convert-lenient", "first = Anna\nfirst = Beth\nname = {first}\n");
    let output = scratch_output_path("convert-lenient", "ngj");

    let result = crate::run(vec![
        "--lenient".to_string(),
        input.to_string_lossy().into_owned(),
        output.to_string_lossy().into_owned(),
    ]);
    assert!(result.is_ok());

    let written = std::fs::read_to_string(&output).expect("output file was written");
    let doc = parsed_ngj(&written);
    let first = doc.categories.iter().find(|(name, _)| name == "first").map(|(_, e)| e).unwrap();
    assert_eq!(first.len(), 2);

    std::fs::remove_file(&input).ok();
    std::fs::remove_file(&output).ok();
}

#[test]
fn convert_errors_on_a_missing_input_file() {
    let input = scratch_path("convert-missing");
    std::fs::remove_file(&input).ok();
    let output = scratch_output_path("convert-missing", "ngj");

    let result = crate::run(vec![input.to_string_lossy().into_owned(), output.to_string_lossy().into_owned()]);
    assert!(result.is_err());
    assert!(!output.exists());
}
