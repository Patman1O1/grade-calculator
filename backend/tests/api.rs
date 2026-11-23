//! Tests of the operations the front-end calls (through `wasm-bindgen`), and of
//! the JSON shapes they produce.

mod fixtures;

use grade_calculator::api::{NOT_A_COURSE, evaluate, import_courses, normalize_courses, summaries};
use grade_calculator::course::Course;
use grade_calculator::status::Status;
use serde_json::{Value, json};

fn to_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}

#[test]
fn blank_course_is_ungraded() {
    let course = to_json(&Course::blank());
    assert_eq!(course["name"], "New course");
    assert_eq!(course["scheme"], "weighted");
    assert_eq!(course["categories"].as_array().unwrap().len(), 2);
    assert_eq!(course["scale"][0], json!({ "letter": "A", "min": 90.0 }));
    // Ids belong to the front-end; the engine has none.
    assert!(course.get("id").is_none());
    assert!(course["categories"][0].get("id").is_none());
    assert!(course["categories"][0]["items"][0].get("id").is_none());
    // Ungraded courses carry no computed fields; an item's status is stored.
    assert!(course.get("percent").is_none());
    assert!(course["categories"][0].get("earned").is_none());
    assert_eq!(course["categories"][0]["items"][0]["status"], "ungraded");
}

#[test]
fn evaluates_a_course_and_solves_targets() {
    let course: Course = serde_json::from_value(json!({
        "id": "c", "name": "Test", "scheme": "points",
        "scale": [{ "letter": "A", "min": 90 }, { "letter": "F", "min": 0 }],
        "categories": [{
            "id": "k", "name": "All", "weight": 0,
            "items": [
                { "id": "a", "name": "A", "earned": 60, "possible": 100 },
                { "id": "b", "name": "B", "earned": null, "possible": 100 }
            ]
        }]
    }))
    .unwrap();
    let out = to_json(&evaluate(&course, &[80.0, 95.0, 30.0]));

    let graded = &out["course"];
    assert_eq!(graded["name"], "Test");
    assert_eq!(graded["percent"], 60.0);
    assert_eq!(graded["letter"], "F");
    assert_eq!(graded["earned"], 60.0);
    assert_eq!(graded["categories"][0]["percent"], 60.0);
    let items = &graded["categories"][0]["items"];
    assert_eq!(items[0]["status"], "counted");
    assert_eq!(items[0]["earned"], 60.0);
    assert_eq!(items[0]["possible"], 100.0);
    assert_eq!(items[1]["status"], "ungraded");
    assert_eq!(items[1]["earned"], Value::Null);

    assert_eq!(out["remaining"], 1);
    assert_eq!(out["needs"][0]["target"], 80.0);
    assert_eq!(out["needs"][0]["needed"]["kind"], "needs");
    assert!((out["needs"][0]["needed"]["fraction"].as_f64().unwrap() - 1.0).abs() < 1e-6);
    assert_eq!(out["needs"][1]["needed"]["kind"], "impossible");
    assert_eq!(out["needs"][2]["needed"], json!({ "kind": "secured" }));
}

#[test]
fn summarises_every_course() {
    let c1 = fixtures::cs401();
    let c2 = Course::blank();
    // Summaries come back in the same order as the courses.
    let out = summaries(vec![c1, c2]);
    assert_eq!(out.len(), 2);
    assert!((out[0].percent.unwrap() - 56.95).abs() < 0.005);
    assert_eq!(out[0].letter.as_deref(), Some("D"));
    assert_eq!(out[1].percent, None);
    // The front-end relies on `null` (not a missing key) here.
    assert_eq!(to_json(&out[1])["percent"], Value::Null);
}

#[test]
fn import_validates_courses_and_drops_ids() {
    let mut course = to_json(&fixtures::math218());
    // Exports carry the front-end's ids; the engine ignores them.
    course["id"] = json!("course-1");
    course["categories"][0]["id"] = json!("cat-1");
    course["categories"][0]["items"][0]["id"] = json!("item-1");
    let imported = import_courses(json!([course, { "not": "a course" }])).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].name, course["name"]);
    let out = to_json(&imported[0]);
    assert!(out.get("id").is_none());
    assert!(out["categories"][0].get("id").is_none());
    assert!(out["categories"][0]["items"][0].get("id").is_none());

    // Exports from the old TypeScript-only app have the same shape (ids optional).
    let legacy = json!({ "name": "Old", "scheme": "points", "scale": [], "categories": [
        { "name": "C", "weight": 0, "items": [{ "name": "I", "score": 1, "outOf": 2, "neverDrop": true }] }
    ]});
    let imported = import_courses(legacy).unwrap();
    let item = &imported[0].categories[0].items[0];
    assert!(item.never_drop);
    // ...and are converted to earned / possible.
    assert_eq!((item.earned, item.possible), (Some(1.0), 2.0));
    let out = to_json(&imported[0]);
    assert!(out["categories"][0]["items"][0].get("outOf").is_none());

    assert_eq!(import_courses(json!({ "hello": 1 })), Err(NOT_A_COURSE));
    assert_eq!(import_courses(json!(42)), Err(NOT_A_COURSE));
}

#[test]
fn normalize_converts_saved_courses_in_order() {
    let saved = json!([
        { "id": "c1", "name": "Old", "scheme": "points", "scale": [], "categories": [
            { "id": "k1", "name": "C", "weight": 0, "items": [
                { "id": "i1", "name": "Report", "score": 85, "outOf": 100, "worth": 10 }
            ] }
        ] },
        { "not": "a course" }
    ]);
    let courses = normalize_courses(saved);
    // Same order as the input, with None for the entry that isn't a course,
    // so the front-end can put its ids back by position.
    assert_eq!(courses.len(), 2);
    assert!(courses[1].is_none());
    let course = courses[0].as_ref().unwrap();
    assert_eq!(course.name, "Old");
    let item = &course.categories[0].items[0];
    assert_eq!(item.name, "Report");
    assert_eq!(
        (item.earned, item.possible),
        (Some(85.0 / 100.0 * 10.0), 10.0)
    );
    assert!(normalize_courses(json!([])).is_empty());
}

#[test]
fn computed_fields_in_input_are_recomputed() {
    let forged = json!({ "name": "F", "scheme": "points", "scale": [], "percent": 100, "letter": "A",
        "categories": [{ "name": "C", "weight": 0, "earned": 99,
            "items": [
                { "name": "I", "score": 0, "outOf": 10, "earned": 10, "status": "dropped" },
                { "name": "J", "earned": 5, "possible": 10, "status": "excluded" }
            ] }] });

    let imported = import_courses(forged.clone()).unwrap();
    assert_eq!(imported[0].percent, None);
    assert_eq!(imported[0].categories[0].earned, None);
    // In the older score/outOf form, `earned` comes from the score, not the forged value.
    assert_eq!(imported[0].categories[0].items[0].earned, Some(0.0));
    assert_eq!(imported[0].categories[0].items[0].status, Status::Counted);
    assert_eq!(imported[0].categories[0].items[1].status, Status::Excluded);

    let course: Course = serde_json::from_value(forged).unwrap();
    let graded = evaluate(&course, &[]).course;
    assert_eq!(graded.percent, Some(0.0));
    assert_eq!(graded.letter, None);
    assert_eq!(graded.categories[0].items[0].earned, Some(0.0));
    assert_eq!(graded.categories[0].items[0].status, Status::Counted);
    // The excluded item is kept out: 0/10, not 5/20.
    assert_eq!(graded.categories[0].items[1].status, Status::Excluded);
    assert_eq!((graded.earned, graded.possible), (Some(0.0), Some(10.0)));
}
