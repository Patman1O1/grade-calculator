//! Tests of the operations the front-end calls (through `wasm-bindgen`), and of
//! the JSON shapes they produce.

mod fixtures;

use grade_calculator::api::{NOT_A_COURSE, evaluate, import_courses, summaries};
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
                { "id": "a", "name": "A", "earnedPoints": 60, "totalPoints": 100 },
                { "id": "b", "name": "B", "earnedPoints": null, "totalPoints": 100 }
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
    assert_eq!(items[0]["earnedPoints"], 60.0);
    assert_eq!(items[0]["totalPoints"], 100.0);
    assert_eq!(items[1]["status"], "ungraded");
    assert_eq!(items[1]["earnedPoints"], Value::Null);

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

    assert_eq!(import_courses(json!({ "hello": 1 })), Err(NOT_A_COURSE));
    assert_eq!(import_courses(json!(42)), Err(NOT_A_COURSE));
}

#[test]
fn computed_fields_in_input_are_recomputed() {
    let forged = json!({ "name": "F", "scheme": "points", "scale": [], "percent": 100, "letter": "A",
        "categories": [{ "name": "C", "weight": 0, "earned": 99,
            "items": [
                { "name": "I", "earnedPoints": 0, "totalPoints": 10, "status": "dropped" },
                { "name": "J", "earnedPoints": 5, "totalPoints": 10, "status": "excluded" }
            ] }] });

    let imported = import_courses(forged).unwrap();
    assert_eq!(imported[0].percent, None);
    assert_eq!(imported[0].categories[0].earned, None);
    assert_eq!(imported[0].categories[0].items[0].status, Status::Counted);
    assert_eq!(imported[0].categories[0].items[1].status, Status::Excluded);

    let graded = evaluate(&imported[0], &[]).course;
    assert_eq!(graded.percent, Some(0.0));
    assert_eq!(graded.letter, None);
    assert_eq!(graded.categories[0].items[0].earned_points, Some(0.0));
    assert_eq!(graded.categories[0].items[0].status, Status::Counted);
    // The excluded item is kept out: 0/10, not 5/20.
    assert_eq!(graded.categories[0].items[1].status, Status::Excluded);
    assert_eq!((graded.earned, graded.possible), (Some(0.0), Some(10.0)));
}
